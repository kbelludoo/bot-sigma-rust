use crate::engine::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crate::engine::stamina::{StaminaConfig, StaminaController};
use crate::engine::stealth::{StealthConfig, StealthEngine};
use crate::game::api::GameApiClient;
use crate::game::catalog::{get_global_catalog, Catalog};
use crate::game::types::{
    AccountConfig, AccountStatus, EventData, GameStateData, InventoryItem, SnapshotMessage,
    TeamPokemonData,
};
use crate::network::socket::SigmaSocketClient;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

pub fn get_outbound_ip(bind_ip: &str) -> String {
    match bind_ip {
        "10.0.0.191" => "SEU_IP_DA_VPS".to_string(),
        "10.0.0.49" => "141.148.34.23".to_string(),
        "10.0.0.217" => "193.122.133.1".to_string(),
        "10.0.0.79" => "129.80.45.1".to_string(),
        other if !other.is_empty() => other.to_string(),
        _ => "Padrão".to_string(),
    }
}

pub struct BotInstance {
    pub config: AccountConfig,
    pub status: Arc<RwLock<AccountStatus>>,
    pub api_client: Arc<GameApiClient>,
    pub socket: Arc<SigmaSocketClient>,
    pub catalog: &'static Catalog,

    pub circuit_breaker: Arc<CircuitBreaker>,
    pub stamina: Arc<StaminaController>,
    pub stealth: Arc<StealthEngine>,

    target_map_slug: Arc<RwLock<String>>,
    paused: Arc<AtomicBool>,
    running: Arc<AtomicBool>,

    last_heal_time: Arc<RwLock<Instant>>,
    last_claim_time: Arc<RwLock<Instant>>,
    last_travel_time: Arc<RwLock<Instant>>,
    last_ball_buy_time: Arc<RwLock<Instant>>,
    last_sell_time: Arc<RwLock<Instant>>,
    last_leader_set_time: Arc<RwLock<Instant>>,
    last_bestiary_sync: Arc<RwLock<Instant>>,

    inventory_loot: Arc<RwLock<Vec<InventoryItem>>>,
    total_balls: Arc<AtomicU32>,
    access_token: Arc<RwLock<String>>,
    kills_by_species: Arc<RwLock<HashMap<u32, u32>>>,

    initial_gold: Arc<AtomicI64>,
    initial_xp: Arc<AtomicU64>,
    has_initial: Arc<AtomicBool>,
}

impl BotInstance {
    pub fn new(cfg: AccountConfig) -> Self {
        let cat = get_global_catalog();
        let bind_ip_str = cfg.bind_ip.as_deref().unwrap_or("");
        let outbound = get_outbound_ip(bind_ip_str);

        let initial_status = AccountStatus {
            id: cfg.id.clone(),
            alias: cfg.name.clone(),
            trainer_name: cfg.name.clone(),
            bind_ip: cfg.bind_ip.clone(),
            outbound_ip: Some(outbound),
            strategy: Some(cfg.settings.strategy.clone()),
            bestiary_target_kills: cfg.settings.bestiary_target_kills,
            map_slug: cfg.settings.hunt_slug.clone(),
            map_name: cat.get_hunt_name(&cfg.settings.hunt_slug),
            last_action: "Inicializado".to_string(),
            updated_at: chrono::Utc::now().timestamp_millis(),
            ..Default::default()
        };

        let cb_cfg = CircuitBreakerConfig::default();
        let cb = Arc::new(CircuitBreaker::new(&cfg.id, cb_cfg));
        let stam = Arc::new(StaminaController::new(StaminaConfig::default()));

        let api_client = Arc::new(GameApiClient::new(cfg.bind_ip.as_deref()));
        let socket = Arc::new(SigmaSocketClient::new(cfg.bind_ip.as_deref()));

        let past = Instant::now() - Duration::from_secs(3600);

        Self {
            config: cfg.clone(),
            status: Arc::new(RwLock::new(initial_status)),
            api_client,
            socket,
            catalog: cat,
            circuit_breaker: cb,
            stamina: stam,
            stealth: Arc::new(StealthEngine::new(StealthConfig::default())),
            target_map_slug: Arc::new(RwLock::new(cfg.settings.hunt_slug)),
            paused: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
            last_heal_time: Arc::new(RwLock::new(past)),
            last_claim_time: Arc::new(RwLock::new(past)),
            last_travel_time: Arc::new(RwLock::new(past)),
            last_ball_buy_time: Arc::new(RwLock::new(past)),
            last_sell_time: Arc::new(RwLock::new(past)),
            last_leader_set_time: Arc::new(RwLock::new(past)),
            last_bestiary_sync: Arc::new(RwLock::new(past)),
            inventory_loot: Arc::new(RwLock::new(Vec::new())),
            total_balls: Arc::new(AtomicU32::new(0)),
            access_token: Arc::new(RwLock::new(cfg.credentials.token)),
            kills_by_species: Arc::new(RwLock::new(HashMap::new())),
            initial_gold: Arc::new(AtomicI64::new(0)),
            initial_xp: Arc::new(AtomicU64::new(0)),
            has_initial: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn get_status(&self) -> AccountStatus {
        let mut st = self.status.read().await.clone();
        if st.max_hp > 0 {
            st.hp_percent = (st.hp as f64 / st.max_hp as f64) * 100.0;
        }
        if st.xp_to_next_level > 0 {
            let mut pct = (st.xp_into_level as f64 / st.xp_to_next_level as f64) * 100.0;
            pct = pct.clamp(0.0, 100.0);
            st.xp_percent = pct;
        }
        st
    }

    pub async fn persist_status(&self) {
        let st = self.get_status().await;
        let _ = tokio::fs::create_dir_all("data").await;
        let filename = format!("data/status_{}.json", self.config.id);
        if let Ok(data) = serde_json::to_string_pretty(&st) {
            let _ = tokio::fs::write(filename, data).await;
        }
    }

    pub async fn run_loop(self: Arc<Self>) {
        self.running.store(true, Ordering::SeqCst);

        // Persistência periódica do status em disco (3s)
        let this_persist = Arc::clone(&self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(3));
            while this_persist.running.load(Ordering::Relaxed) {
                ticker.tick().await;
                this_persist.persist_status().await;
            }
        });

        let mut backoff = Duration::from_secs(5);

        while self.running.load(Ordering::Relaxed) {
            let start_time = Instant::now();
            let err = self.run_session().await;

            if let Err(e) = err {
                {
                    let mut st = self.status.write().await;
                    st.in_game = false;
                    st.last_error = Some(e.clone());
                    st.last_action = format!("Erro: {e} (tentando em {backoff:?})");
                }
                warn!("[{}] Sessão encerrou: {e}. Próxima tentativa em {backoff:?}…", self.config.id);
            }

            if start_time.elapsed() > Duration::from_secs(60) {
                backoff = Duration::from_secs(5);
            } else {
                backoff += Duration::from_secs(5);
                if backoff > Duration::from_secs(30) {
                    backoff = Duration::from_secs(30);
                }
            }

            sleep(backoff).await;
        }
    }

    pub async fn run_session(&self) -> Result<(), String> {
        {
            let mut st = self.status.write().await;
            st.in_game = false;
            st.last_error = None;
            st.last_action = "Autenticando...".to_string();
        }

        info!(
            "[{}] Fazendo login para {} ({})...",
            self.config.id, self.config.name, self.config.credentials.email
        );

        let login_resp = self
            .api_client
            .login(&self.config.credentials.email, &self.config.credentials.password)
            .await?;

        let token = login_resp.access_token.clone();
        *self.access_token.write().await = token.clone();

        {
            let mut st = self.status.write().await;
            st.last_action = "Obtendo ticket do servidor...".to_string();
        }

        // Sincroniza dados do bestiário logo após autenticação
        let this_sync = self.clone_ref();
        tokio::spawn(async move {
            this_sync.sync_bestiary().await;
        });

        let ticket_resp = self.api_client.get_ticket(&token).await?;

        let (disconnect_tx, mut disconnect_rx) = mpsc::channel(1);

        {
            let mut st = self.status.write().await;
            st.last_action = "Conectando WebSocket...".to_string();
        }

        let this_dispatch = self.clone_ref();
        self.socket
            .connect(
                &ticket_resp.ticket,
                ticket_resp.url.as_deref(),
                disconnect_tx,
                move |text| {
                    let td = this_dispatch.clone_ref();
                    tokio::spawn(async move {
                        td.handle_incoming_text(&text).await;
                    });
                },
            )
            .await?;

        {
            let mut st = self.status.write().await;
            st.in_game = true;
            st.connected_at = chrono::Utc::now().timestamp_millis();
            st.last_action = "Conectado. Aguardando mundo...".to_string();
        }

        info!("[{}: {}] Conectado ao jogo com sucesso! ✓", self.config.id, self.config.name);

        // Automation loop
        let this_auto = self.clone_ref();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(2));
            let mut ping_ticker = tokio::time::interval(Duration::from_secs(20));

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if !this_auto.socket.is_connected() {
                            break;
                        }
                        this_auto.tick_automation().await;
                    }
                    _ = ping_ticker.tick() => {
                        if this_auto.socket.is_connected() {
                            let _ = this_auto.socket.send_intent(serde_json::json!({ "type": "ping" })).await;
                        }
                    }
                }
            }
        });

        // Aguarda evento de desconexão
        let _ = disconnect_rx.recv().await;
        Err("Conexão WebSocket encerrada".to_string())
    }

    async fn handle_incoming_text(&self, text: &str) {
        let msg: SnapshotMessage = match serde_json::from_str(text) {
            Ok(m) => m,
            Err(_) => return,
        };

        if let Some(snap) = msg.snapshot {
            self.update_snapshot(snap).await;
        }

        if let Some(events) = msg.events {
            self.process_events(events).await;
        }

        if let Some(notice) = msg.message {
            let mut st = self.status.write().await;
            st.last_action = format!("Aviso: {notice}");
        }
    }

    async fn update_snapshot(&self, snap: GameStateData) {
        let mut st = self.status.write().await;

        st.trainer_name = snap.trainer.name.clone();
        st.level = snap.trainer.level;
        st.xp = snap.trainer.xp;
        st.xp_into_level = snap.trainer.xp_into_level;
        st.xp_to_next_level = snap.trainer.xp_to_next_level;
        st.hp = snap.trainer.hp;
        st.max_hp = snap.trainer.max_hp;
        st.gold = snap.trainer.gold;
        st.diamonds = snap.trainer.diamonds;

        let now_ms = chrono::Utc::now().timestamp_millis();
        if let Some(until) = snap.trainer.vip_until {
            if until > now_ms {
                st.vip_active = true;
                st.vip_until = Some(until);
            } else {
                st.vip_active = false;
                st.vip_until = None;
            }
        } else {
            st.vip_active = false;
            st.vip_until = None;
        }
        st.vip_expiring_soon = snap.trainer.vip_expiring_soon;
        st.auto_hunt_active = snap.auto_hunt;

        if !self.has_initial.load(Ordering::Relaxed) {
            self.initial_gold.store(snap.trainer.gold, Ordering::Relaxed);
            self.initial_xp.store(snap.trainer.xp, Ordering::Relaxed);
            self.has_initial.store(true, Ordering::Relaxed);
        } else {
            let init_g = self.initial_gold.load(Ordering::Relaxed);
            let init_x = self.initial_xp.load(Ordering::Relaxed);
            if snap.trainer.gold >= init_g {
                st.gold_found = snap.trainer.gold - init_g;
            }
            if snap.trainer.xp >= init_x {
                st.trainer_xp_gained = snap.trainer.xp - init_x;
            }
        }

        // Estatísticas do Analisador interno
        if let Some(an) = &snap.analyzer {
            st.kills = an.kills;
            st.captures = an.captures;
            st.shinies = an.shiny_captures;
            if an.gold_found > 0 {
                st.gold_found = an.gold_found;
            }
            if an.trainer_xp > 0 {
                st.trainer_xp_gained = an.trainer_xp;
            }
            if an.elapsed_ms >= 3000 {
                let mins = an.elapsed_ms as f64 / 60000.0;
                st.kills_per_min = an.kills as f64 / mins;
                st.xp_per_hour = (an.trainer_xp as f64 / mins) * 60.0;
            } else {
                let elapsed_mins = (now_ms - st.connected_at) as f64 / 60000.0;
                if elapsed_mins > 0.1 {
                    st.kills_per_min = st.kills as f64 / elapsed_mins;
                    st.xp_per_hour = (st.trainer_xp_gained as f64 / elapsed_mins) * 60.0;
                }
            }
        }

        // Pokémon Líder da equipe
        if let Some(leader) = &snap.leader {
            st.leader_species_id = Some(leader.species_id);
            st.leader_name = self.catalog.get_species_name(leader.species_id);
            st.leader_level = leader.level;
            st.leader_hp = leader.hp;
            st.leader_max_hp = leader.max_hp;
            st.leader_shiny = leader.shiny;
        } else if !snap.team.is_empty() {
            let leader = &snap.team[0];
            st.leader_species_id = Some(leader.species_id);
            st.leader_name = self.catalog.get_species_name(leader.species_id);
            st.leader_level = leader.level;
            st.leader_hp = leader.hp;
            st.leader_max_hp = leader.max_hp;
            st.leader_shiny = leader.shiny;
        }

        st.team_size = snap.team.len();

        // Garante que o Pokémon de maior nível da equipe seja sempre o Líder ativo
        if snap.team.len() > 1 && self.socket.is_connected() {
            let last_set = *self.last_leader_set_time.read().await;
            if last_set.elapsed() > Duration::from_secs(4) {
                let mut best: Option<&TeamPokemonData> = None;
                for p in &snap.team {
                    if p.hp > 0 && match best { None => true, Some(b) => p.level > b.level } {
                        best = Some(p);
                    }
                }
                if let (Some(b), Some(leader)) = (best, &snap.leader) {
                    if b.pokemon_id != leader.pokemon_id && b.level > leader.level {
                        *self.last_leader_set_time.write().await = Instant::now();
                        info!(
                            "[{}] 🚀 [AUTO-LÍDER] Promovendo {} (Nv. {}) para Líder!",
                            self.config.id,
                            self.catalog.get_species_name(b.species_id),
                            b.level
                        );
                        let _ = self
                            .socket
                            .send_intent(serde_json::json!({
                                "type": "setLeader",
                                "pokemonId": b.pokemon_id
                            }))
                            .await;
                    }
                }
            }
        }

        // Dados de caça e mapa
        if let Some(hunt) = &snap.hunt {
            if !hunt.map_slug.is_empty() {
                st.in_hunt = true;
                st.map_slug = hunt.map_slug.clone();
                st.map_name = self.catalog.get_hunt_name(&hunt.map_slug);

                if hunt.wild_species_id > 0 {
                    st.wild_target = self.catalog.get_species_name(hunt.wild_species_id);
                    st.wild_hp = hunt.wild_hp;
                    st.wild_max_hp = hunt.wild_max_hp;
                    st.wild_level = hunt.wild_level;
                    st.wild_shiny = hunt.wild_shiny;
                } else if !hunt.wilds.is_empty() {
                    let w = &hunt.wilds[0];
                    st.wild_target = self.catalog.get_species_name(w.species_id);
                    st.wild_hp = w.hp;
                    st.wild_max_hp = w.max_hp;
                    st.wild_level = w.level;
                    st.wild_shiny = w.shiny;
                }
            } else {
                st.in_hunt = false;
                st.map_slug.clear();
                st.map_name = "Na Cidade / PokéCenter".to_string();
                st.wild_target.clear();
                st.wild_hp = 0;
                st.wild_max_hp = 0;
                st.wild_level = 0;
                st.wild_shiny = false;
            }
        } else {
            st.in_hunt = false;
            st.map_slug.clear();
            st.map_name = "Na Cidade / PokéCenter".to_string();
            st.wild_target.clear();
            st.wild_hp = 0;
            st.wild_max_hp = 0;
            st.wild_level = 0;
            st.wild_shiny = false;
        }

        // Guarda itens de saque (loot)
        if !snap.inventory.is_empty() {
            let loots: Vec<InventoryItem> = snap
                .inventory
                .into_iter()
                .filter(|it| it.category == "loot" && it.count > 0 && !it.locked)
                .collect();
            *self.inventory_loot.write().await = loots;
        }

        // Contagem de Pokébolas
        let mut total_b = 0u32;
        for cnt in snap.balls.values() {
            total_b += cnt;
        }
        for cnt in snap.bound_balls.values() {
            total_b += cnt;
        }
        self.total_balls.store(total_b, Ordering::Relaxed);

        st.updated_at = now_ms;
    }

    async fn process_events(&self, events: Vec<EventData>) {
        let mut st = self.status.write().await;
        for ev in events {
            match ev.event_type.as_str() {
                "kill" => {
                    st.kills += 1;
                    if ev.species_id > 0 {
                        let mut map = self.kills_by_species.write().await;
                        *map.entry(ev.species_id).or_insert(0) += 1;
                        st.bestiary_total_kills += 1;
                    }
                    let mon = self.catalog.get_species_name(ev.species_id);
                    st.last_action = format!("Derrotou {mon} (+{} XP, +{} Ouro)", ev.xp, ev.gold);
                }
                "captured" => {
                    st.captures += 1;
                    let mon = self.catalog.get_species_name(ev.species_id);
                    if ev.shiny {
                        st.shinies += 1;
                        info!("✨ [{}] POKÉMON SHINY CAPTURADO: {}! ✨", self.config.id, mon);
                        st.last_action = format!("✨ Capturou SHINY {mon}!");
                    } else {
                        st.last_action = format!("Capturou {mon} com sucesso!");
                    }
                }
                "shinyAppeared" => {
                    let mon = self.catalog.get_species_name(ev.species_id);
                    info!("⭐ [{}] SHINY AVISTADO NA ROTA: {}! ⭐", self.config.id, mon);
                    st.last_action = format!("⭐ Shiny avistado: {mon}!");
                }
                "healed" => {
                    st.last_action = "Curado no Centro Pokémon".to_string();
                }
                "levelUp" => {
                    st.level = ev.level;
                    st.last_action = format!("Subiu para o Nível {}!", ev.level);
                    info!("🎉 [{}] Treinador subiu para o Nível {}! 🎉", self.config.id, ev.level);
                }
                _ => {}
            }
        }
    }

    async fn tick_automation(&self) {
        if self.paused.load(Ordering::Relaxed) {
            let mut st = self.status.write().await;
            st.last_action = "Pausado pelo usuário".to_string();
            return;
        }

        let target_map = self.target_map_slug.read().await.clone();
        let (current_map, in_hunt, hp, max_hp, leader_hp, leader_species_id, level, gold) = {
            let st = self.status.read().await;
            (
                st.map_slug.clone(),
                st.in_hunt,
                st.hp,
                st.max_hp,
                st.leader_hp,
                st.leader_species_id.unwrap_or(0),
                st.level,
                st.gold,
            )
        };

        // 0. Sincronização periódica do bestiário a cada 3 minutos
        {
            let last_sync = *self.last_bestiary_sync.read().await;
            if last_sync.elapsed() > Duration::from_secs(180) {
                let this_sync = self.clone_ref();
                tokio::spawn(async move {
                    this_sync.sync_bestiary().await;
                });
            }
        }

        // 2. Viagem inteligente: se hunt_slug for "auto" ou estratégia for "bestiary"
        let mut dest_map = target_map.clone();
        let mut tactical_reason = String::new();

        if self.config.settings.strategy == "bestiary" {
            let kills_copy = self.kills_by_species.read().await.clone();
            let (next_slug, reason, pending) = self.catalog.get_next_bestiary_hunt(
                level,
                &kills_copy,
                self.config.settings.bestiary_target_kills,
                leader_species_id,
            );

            {
                let mut st = self.status.write().await;
                st.bestiary_pending_count = pending;
            }

            if !next_slug.is_empty() {
                dest_map = next_slug;
                tactical_reason = reason;
            } else {
                let (opt_slug, opt_reason) = self.catalog.get_optimal_hunt_for_leader(level, leader_species_id);
                dest_map = opt_slug;
                tactical_reason = format!("🏆 [Bestiário Completo!] {opt_reason}");
            }
        } else if dest_map.is_empty() || dest_map == "auto" {
            let (opt_slug, opt_reason) = self.catalog.get_optimal_hunt_for_leader(level, leader_species_id);
            dest_map = opt_slug;
            tactical_reason = opt_reason;
        }

        let mut needs_travel = false;
        if !dest_map.is_empty() && (!in_hunt || current_map.is_empty() || current_map != dest_map) {
            needs_travel = true;
        }

        if needs_travel {
            let last_travel = *self.last_travel_time.read().await;
            if last_travel.elapsed() > Duration::from_secs(8) {
                *self.last_travel_time.write().await = Instant::now();
                if self.config.settings.strategy == "bestiary" {
                    let _ = self.socket.send_intent(serde_json::json!({ "type": "claimBestiaryAll" })).await;
                }

                info!(
                    "[{}] 🚀 [ROTA TÁTICA {}] Saindo de '{}' para: '{}' ({})...",
                    self.config.id,
                    self.config.settings.strategy.to_uppercase(),
                    current_map,
                    dest_map,
                    tactical_reason
                );

                let _ = self
                    .socket
                    .send_intent(serde_json::json!({
                        "type": "travel",
                        "mapSlug": dest_map
                    }))
                    .await;

                {
                    let mut st = self.status.write().await;
                    st.last_action = format!(
                        "Viajando para {} ({})...",
                        self.catalog.get_hunt_name(&dest_map),
                        tactical_reason
                    );
                }
                return;
            }
        }

        // 3. Auto-Heal (Curar no Centro Pokémon se HP estiver baixo ou desmaiado)
        if self.config.settings.auto_heal {
            let last_heal = *self.last_heal_time.read().await;
            if last_heal.elapsed() > Duration::from_secs(10) {
                let low_health = (max_hp > 0 && (hp as f64 / max_hp as f64) < 0.25)
                    || (leader_hp == 0 && max_hp > 0);
                if low_health {
                    *self.last_heal_time.write().await = Instant::now();
                    info!("[{}] Vida baixa detectada ({hp}/{max_hp}). Enviando healAtCenter...", self.config.id);
                    let _ = self.socket.send_intent(serde_json::json!({ "type": "healAtCenter" })).await;
                    {
                        let mut st = self.status.write().await;
                        st.last_action = "Curando no Centro Pokémon...".to_string();
                    }
                    return;
                }
            }
        }

        // 4. Auto-Hunt ativo
        if self.config.settings.auto_hunt {
            let auto_hunt_active = self.status.read().await.auto_hunt_active;
            if !auto_hunt_active {
                let _ = self
                    .socket
                    .send_intent(serde_json::json!({
                        "type": "setAutoHunt",
                        "on": true
                    }))
                    .await;
                let mut st = self.status.write().await;
                st.auto_hunt_active = true;
                st.last_action = "Ativando Auto-Hunt...".to_string();
            }
        }

        // 5. Coleta de diários (com jitter orgânico e pacing humano anti-burst)
        if self.config.settings.auto_daily {
            let last_claim = *self.last_claim_time.read().await;
            let claim_interval = self.stealth.jitter_delay(Duration::from_secs(45));
            if last_claim.elapsed() > claim_interval {
                *self.last_claim_time.write().await = Instant::now();
                let _ = self.socket.send_intent(serde_json::json!({ "type": "claimDailyLogin" })).await;
                self.stealth.pacing_step().await;
                let _ = self.socket.send_intent(serde_json::json!({ "type": "claimQuestReward" })).await;
                self.stealth.pacing_step().await;
                let _ = self.socket.send_intent(serde_json::json!({ "type": "claimBestiaryAll" })).await;
                self.stealth.pacing_step().await;
                let _ = self.socket.send_intent(serde_json::json!({ "type": "claimDailyTasks" })).await;
                self.stealth.pacing_step().await;
                let _ = self.socket.send_intent(serde_json::json!({ "type": "claimWeeklyTasks" })).await;
                let mut st = self.status.write().await;
                st.last_action = "Coletando recompensas diárias...".to_string();
            }
        }

        // 7. Auto-compra de Pokébolas se < 10 e gold >= 200 (a cada 30s)
        let total_b = self.total_balls.load(Ordering::Relaxed);
        if total_b < 10 && gold >= 200 {
            let last_buy = *self.last_ball_buy_time.read().await;
            if last_buy.elapsed() > Duration::from_secs(30) {
                *self.last_ball_buy_time.write().await = Instant::now();
                info!(
                    "[{}] 🛒 [AUTO-COMPRA] Pokébolas acabando ({total_b} restantes). Comprando 20 Pokébolas no NPC...",
                    self.config.id
                );
                let _ = self
                    .socket
                    .send_intent(serde_json::json!({
                        "type": "buyBall",
                        "ballId": 1,
                        "count": 20
                    }))
                    .await;
            }
        }

        // 8. Auto-venda de itens de loot no inventário (a cada 20s)
        let loots = self.inventory_loot.read().await.clone();
        if !loots.is_empty() {
            let last_sell = *self.last_sell_time.read().await;
            let sell_interval = self.stealth.jitter_delay(Duration::from_secs(20));
            if last_sell.elapsed() > sell_interval {
                *self.last_sell_time.write().await = Instant::now();
                let mut sold_count = 0u32;
                for it in &loots {
                    if it.count > 0 {
                        let _ = self
                            .socket
                            .send_intent(serde_json::json!({
                                "type": "sellItem",
                                "itemId": it.item_id,
                                "count": it.count
                            }))
                            .await;
                        sold_count += it.count;
                        self.stealth.pacing_step().await;
                    }
                }
                if sold_count > 0 {
                    info!("[{}] 💰 [AUTO-VENDA] Vendidos {sold_count} itens de loot no NPC!", self.config.id);
                }
                self.inventory_loot.write().await.clear();
            }
        }
    }

    pub async fn sync_bestiary(&self) {
        let token = self.access_token.read().await.clone();
        if token.is_empty() {
            return;
        }

        let target_kills = if self.config.settings.bestiary_target_kills == 0 {
            120
        } else {
            self.config.settings.bestiary_target_kills
        };

        let bestiary = match self.api_client.get_bestiary(&token).await {
            Ok(b) => b,
            Err(e) => {
                warn!("[{}] Falha ao sincronizar bestiário: {e}", self.config.id);
                return;
            }
        };

        *self.last_bestiary_sync.write().await = Instant::now();

        let mut total_kills = 0u64;
        let mut completed = 0usize;
        {
            let mut map = self.kills_by_species.write().await;
            for sp in &bestiary.species {
                map.insert(sp.id, sp.kills);
                total_kills += sp.kills as u64;
                if sp.kills >= target_kills {
                    completed += 1;
                }
            }
        }

        let pending = bestiary.species.len().saturating_sub(completed);

        {
            let mut st = self.status.write().await;
            st.bestiary_level = Some(bestiary.level);
            st.bestiary_points = Some(bestiary.points);
            st.bestiary_next_points = bestiary.next_points;
            st.bestiary_total_kills = total_kills;
            st.bestiary_completed_count = completed;
            st.bestiary_pending_count = pending;
        }

        let next_pts_str = bestiary
            .next_points
            .map(|p| format!("{p} pts"))
            .unwrap_or_else(|| "MAX".to_string());

        info!(
            "[{}] 📖 [BESTIÁRIO] Nível {} ({} pts, prox: {}) │ Abates: {} │ Concluídos: {}/{} (Meta: {})",
            self.config.id,
            bestiary.level,
            bestiary.points,
            next_pts_str,
            total_kills,
            completed,
            bestiary.species.len(),
            target_kills
        );
    }

    // Controles remotos via Dashboard
    pub async fn travel_to(&self, map_slug: &str) -> Result<(), String> {
        *self.target_map_slug.write().await = map_slug.to_string();
        if self.socket.is_connected() {
            self.socket
                .send_intent(serde_json::json!({
                    "type": "travel",
                    "mapSlug": map_slug
                }))
                .await
        } else {
            Ok(())
        }
    }

    pub async fn toggle_auto_hunt(&self, on: bool) -> Result<(), String> {
        if self.socket.is_connected() {
            self.socket
                .send_intent(serde_json::json!({
                    "type": "setAutoHunt",
                    "on": on
                }))
                .await
        } else {
            Ok(())
        }
    }

    pub async fn heal_now(&self) -> Result<(), String> {
        if self.socket.is_connected() {
            self.socket
                .send_intent(serde_json::json!({
                    "type": "healAtCenter"
                }))
                .await
        } else {
            Ok(())
        }
    }

    pub async fn claim_rewards_now(&self) -> Result<(), String> {
        if self.socket.is_connected() {
            let _ = self.socket.send_intent(serde_json::json!({ "type": "claimDailyLogin" })).await;
            let _ = self.socket.send_intent(serde_json::json!({ "type": "claimQuestReward" })).await;
            let _ = self.socket.send_intent(serde_json::json!({ "type": "claimBestiaryAll" })).await;
            let _ = self.socket.send_intent(serde_json::json!({ "type": "claimDailyTasks" })).await;
            let _ = self.socket.send_intent(serde_json::json!({ "type": "claimWeeklyTasks" })).await;
        }
        Ok(())
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }

    pub async fn set_strategy(&self, strategy: &str, target_kills: u32) {
        {
            let mut st = self.status.write().await;
            st.strategy = Some(strategy.to_string());
            if target_kills > 0 {
                st.bestiary_target_kills = target_kills;
            }
        }
        info!(
            "[{}] 🎯 Estratégia alterada para '{}' (Meta Bestiário: {})",
            self.config.id, strategy, target_kills
        );
        if strategy == "bestiary" {
            let this_sync = self.clone_ref();
            tokio::spawn(async move {
                this_sync.sync_bestiary().await;
            });
        }
    }

    pub fn clone_ref(&self) -> Arc<Self> {
        Arc::new(Self {
            config: self.config.clone(),
            status: self.status.clone(),
            api_client: self.api_client.clone(),
            socket: self.socket.clone(),
            catalog: self.catalog,
            circuit_breaker: self.circuit_breaker.clone(),
            stamina: self.stamina.clone(),
            stealth: self.stealth.clone(),
            target_map_slug: self.target_map_slug.clone(),
            paused: self.paused.clone(),
            running: self.running.clone(),
            last_heal_time: self.last_heal_time.clone(),
            last_claim_time: self.last_claim_time.clone(),
            last_travel_time: self.last_travel_time.clone(),
            last_ball_buy_time: self.last_ball_buy_time.clone(),
            last_sell_time: self.last_sell_time.clone(),
            last_leader_set_time: self.last_leader_set_time.clone(),
            last_bestiary_sync: self.last_bestiary_sync.clone(),
            inventory_loot: self.inventory_loot.clone(),
            total_balls: self.total_balls.clone(),
            access_token: self.access_token.clone(),
            kills_by_species: self.kills_by_species.clone(),
            initial_gold: self.initial_gold.clone(),
            initial_xp: self.initial_xp.clone(),
            has_initial: self.has_initial.clone(),
        })
    }
}
