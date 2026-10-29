use crate::engine::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crate::engine::stamina::{StaminaConfig, StaminaController};
use crate::game::types::{AccountConfig, GameStateData, SnapshotMessage};
use crate::network::socket::SigmaSocketClient;
use crate::subsystem::dailies::DailyManager;
use crate::subsystem::inventory::InventoryManager;
use crate::subsystem::potions::PotionManager;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

pub struct BotInstance {
    pub config: AccountConfig,
    pub circuit_breaker: Arc<CircuitBreaker>,
    pub stamina: Arc<StaminaController>,
    pub inventory: Arc<InventoryManager>,
    pub dailies: Arc<DailyManager>,
    pub potions: Arc<PotionManager>,
    pub socket: Arc<SigmaSocketClient>,

    pub state: Arc<Mutex<GameStateData>>,
    pub kills: AtomicU64,
    pub gold: AtomicU64,
    pub exp: AtomicU64,
    pub current_level: AtomicU32,
    pub in_hunt: AtomicBool,
    pub running: AtomicBool,
}

impl BotInstance {
    pub fn new(config: AccountConfig) -> Self {
        let cb_cfg = CircuitBreakerConfig::default();
        let cb = Arc::new(CircuitBreaker::new(&config.id, cb_cfg));
        let stam = Arc::new(StaminaController::new(StaminaConfig::default()));

        let ws_url = if config.credentials.origin.is_empty() {
            "wss://api.poke.sigmaproject.online/ws".to_string()
        } else {
            let orig = &config.credentials.origin;
            let scheme = if orig.starts_with("https") { "wss" } else { "ws" };
            let host = orig.trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/');
            format!("{}://{}/ws", scheme, host)
        };

        let socket = Arc::new(SigmaSocketClient::new(&ws_url));

        Self {
            config,
            circuit_breaker: cb,
            stamina: stam,
            inventory: Arc::new(InventoryManager::default()),
            dailies: Arc::new(DailyManager::default()),
            potions: Arc::new(PotionManager::default()),
            socket,
            state: Arc::new(Mutex::new(GameStateData::default())),
            kills: AtomicU64::new(0),
            gold: AtomicU64::new(0),
            exp: AtomicU64::new(0),
            current_level: AtomicU32::new(1),
            in_hunt: AtomicBool::new(false),
            running: AtomicBool::new(false),
        }
    }

    pub async fn handle_message(&self, raw: &str) {
        let msg: SnapshotMessage = match serde_json::from_str(raw) {
            Ok(m) => m,
            Err(_) => return,
        };

        if let Some(snap) = msg.snapshot {
            self.current_level.store(snap.trainer.level, Ordering::Relaxed);
            self.gold.store(snap.trainer.gold as u64, Ordering::Relaxed);
            self.exp.store(snap.trainer.xp, Ordering::Relaxed);
            self.in_hunt.store(snap.auto_hunt, Ordering::Relaxed);

            // Auto-Líder: se houver membros na equipe, promove o de maior nível
            if snap.team.len() > 1 {
                if let Some(best) = snap.team.iter().max_by_key(|p| p.level) {
                    if let Some(leader) = &snap.leader {
                        if best.pokemon_id != leader.pokemon_id && best.level > leader.level {
                            info!("🚀 [AUTO-LÍDER] Promovendo Pokémon #{} (Nv. {}) a Líder!", best.pokemon_id, best.level);
                            let _ = self.socket.send_intent(&serde_json::json!({
                                "type": "setLeader",
                                "pokemonId": best.pokemon_id
                            })).await;
                        }
                    }
                }
            }

            *self.state.lock().await = snap;
        }

        if let Some(notice) = msg.message {
            info!("📢 [Sigma Notice]: {notice}");
        }
    }

    pub async fn perform_automation_tick(&self) {
        if !self.socket.is_connected().await {
            return;
        }

        let st = self.state.lock().await.clone();

        // 1. Auto-Heal: Centro Pokémon se HP < 30%
        if self.config.settings.auto_heal && st.trainer.max_hp > 0 {
            if (st.trainer.hp as f64) < (st.trainer.max_hp as f64 * 0.3) {
                info!("🚑 HP crítico! Curando no Centro Pokémon...");
                let _ = self.socket.send_intent(&serde_json::json!({ "type": "heal" })).await;
            }
        }

        // 2. Auto-Hunt
        if self.config.settings.auto_hunt && !self.in_hunt.load(Ordering::Relaxed) {
            let target = if self.config.settings.hunt_slug.is_empty() {
                "viridian_forest"
            } else {
                &self.config.settings.hunt_slug
            };
            info!("⚔️ Iniciando caçada em '{target}'...");
            let _ = self.socket.send_intent(&serde_json::json!({
                "type": "startHunt",
                "mapSlug": target
            })).await;
            self.in_hunt.store(true, Ordering::Relaxed);
        }

        // 3. Auto-Spin: Roda da sorte
        if self.config.settings.auto_spin {
            let _ = self.socket.send_intent(&serde_json::json!({ "type": "spin" })).await;
        }

        // 4. Auto-Daily: Reivindicar diários
        if self.config.settings.auto_daily {
            let _ = self.socket.send_intent(&serde_json::json!({ "type": "claimDaily" })).await;
        }
    }

    pub async fn run_loop(self: Arc<Self>) {
        self.running.store(true, Ordering::SeqCst);
        info!("🚀 Bot [{}] (Sigma Project Online) iniciado com protocolo de rede nativo.", self.config.name);

        let this = Arc::clone(&self);
        let _ = self.socket.connect(move |raw| {
            let this = Arc::clone(&this);
            tokio::spawn(async move {
                this.handle_message(&raw).await;
            });
        }).await;

        let mut ping_timer = tokio::time::interval(Duration::from_secs(20));
        let mut action_timer = tokio::time::interval(Duration::from_secs(5));

        while self.running.load(Ordering::Relaxed) {
            tokio::select! {
                _ = ping_timer.tick() => {
                    if self.socket.is_connected().await {
                        let _ = self.socket.send_intent(&serde_json::json!({ "type": "ping" })).await;
                    }
                }
                _ = action_timer.tick() => {
                    self.perform_automation_tick().await;
                }
            }
        }
    }
}
