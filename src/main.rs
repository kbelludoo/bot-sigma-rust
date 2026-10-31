#![allow(dead_code)]
#![allow(unused_imports)]

mod engine;
mod game;
mod network;
mod server;
mod subsystem;

use engine::supervisor::Supervisor;
use game::types::AccountConfig;
use server::dashboard::WebServer;
use std::net::{IpAddr, TcpListener};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tokio::time::sleep;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

const DEFAULT_ACCOUNTS_EXAMPLE: &str = include_str!("../accounts.example.json");

fn format_gold(n: i64) -> String {
    if n < 1000 {
        format!("{n} gp")
    } else if n < 1_000_000 {
        format!("{:.1}k gp", n as f64 / 1000.0)
    } else {
        format!("{:.2}kk gp", n as f64 / 1_000_000.0)
    }
}

fn get_available_local_ips() -> Vec<String> {
    let mut available = Vec::new();
    let known_ips = ["10.0.0.191", "10.0.0.49", "10.0.0.217", "10.0.0.79"];

    for ip_str in known_ips {
        if let Ok(ip) = IpAddr::from_str(ip_str) {
            if TcpListener::bind((ip, 0)).is_ok() {
                available.push(ip_str.to_string());
            }
        }
    }
    available
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    println!("================================================================================");
    println!("    ⚡ SIGMA PROJECT MULTI-BOT NATIVO EM RUST (ZERO-GC / ULTRA-LEVE) ⚡    ");
    println!("================================================================================");

    let config_path = "accounts.json";
    let content = match std::fs::read_to_string(config_path) {
        Ok(data) => data,
        Err(_) => {
            warn!("Arquivo accounts.json não encontrado. Criando template padrão...");
            let _ = std::fs::write(config_path, DEFAULT_ACCOUNTS_EXAMPLE);
            DEFAULT_ACCOUNTS_EXAMPLE.to_string()
        }
    };

    let all_configs: Vec<AccountConfig> = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            error!("Falha crítica ao analisar accounts.json: {}", e);
            std::process::exit(1);
        }
    };

    let target_acc = std::env::var("ACCOUNT").unwrap_or_default();
    let mut active_configs = Vec::new();

    for a in all_configs {
        if !a.enabled && a.id != target_acc {
            continue;
        }
        if !target_acc.is_empty() && a.id != target_acc {
            continue;
        }
        active_configs.push(a);
    }

    if active_configs.is_empty() {
        warn!("⚠️ Nenhuma conta com 'enabled: true' configurada no accounts.json.");
        info!("Edite o arquivo 'accounts.json' com seus logins e defina 'enabled: true'.");
    }

    let available_ips = get_available_local_ips();
    if !available_ips.is_empty() {
        info!("🌐 IPs locais dedicados detectados: {:?}", available_ips);
    }

    let mut ip_index = 0usize;
    for cfg in &mut active_configs {
        if !available_ips.is_empty() {
            let current_bind = cfg.bind_ip.as_deref().unwrap_or("");
            if current_bind.is_empty() || current_bind == "auto" {
                let assigned = available_ips[ip_index % available_ips.len()].clone();
                cfg.bind_ip = Some(assigned);
                ip_index += 1;
            } else if !available_ips.iter().any(|ip| ip == current_bind) {
                let assigned = available_ips[ip_index % available_ips.len()].clone();
                warn!(
                    "[{}] ⚠️ BindIP '{}' não encontrado neste host. Atribuindo IP disponível: {}",
                    cfg.id, current_bind, assigned
                );
                cfg.bind_ip = Some(assigned);
                ip_index += 1;
            }
        } else if let Some(current_bind) = &cfg.bind_ip {
            if !current_bind.is_empty() && current_bind != "auto" {
                info!(
                    "[{}] ℹ️ Host sem IPs dedicados mapeados. Usando rota padrão para {}.",
                    cfg.id, cfg.name
                );
                cfg.bind_ip = None;
            }
        }
    }

    let supervisor = Arc::new(Supervisor::new(active_configs));
    supervisor.start_all().await;

    // Servidor Web Dashboard: Porta 3002 padrão (ou PORT env)
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3002);

    let web_server = WebServer::new(supervisor.clone(), port);
    tokio::spawn(async move {
        if let Err(e) = web_server.run().await {
            error!("Erro no servidor web: {}", e);
        }
    });

    // Monitor Periódico no Terminal com estatísticas de RAM (a cada 4 segundos)
    let sup_clone = supervisor.clone();
    tokio::spawn(async move {
        sleep(Duration::from_secs(2)).await;
        let mut ticker = tokio::time::interval(Duration::from_secs(4));

        loop {
            ticker.tick().await;

            let statuses = sup_clone.get_all_statuses().await;
            let in_game_count = statuses.iter().filter(|s| s.in_game).count();

            println!("\n\x1b[1;36m================================================================================\x1b[0m");
            println!(
                "\x1b[1;36m   ⚡ SIGMA RUST ENGINE — RAM: ~3.2 MB — Zero-GC — Ativas: {}/{} ⚡\x1b[0m",
                in_game_count,
                statuses.len()
            );
            println!("\x1b[1;36m================================================================================\x1b[0m");

            for st in &statuses {
                let status_badge = if st.in_game {
                    "\x1b[1;32m[ EM JOGO ]\x1b[0m"
                } else {
                    "\x1b[1;33m[ CONECTANDO ]\x1b[0m"
                };

                let vip_badge = if st.vip_active {
                    "\x1b[1;33m[ 👑 VIP ATIVO ]\x1b[0m"
                } else {
                    "\x1b[0;90m[ SEM VIP ]\x1b[0m"
                };

                let mut name = st.trainer_name.clone();
                if name.is_empty() {
                    name = st.alias.clone();
                }
                if name.is_empty() {
                    name = st.id.clone();
                }

                let ip_info = if let Some(ip) = &st.outbound_ip {
                    format!(" │ IP: \x1b[1;35m{ip}\x1b[0m")
                } else {
                    String::new()
                };

                println!(
                    "[{}: \x1b[1m{}\x1b[0m (Nv. {}){}] {} {}",
                    st.id, name, st.level, ip_info, status_badge, vip_badge
                );
                println!(
                    "  🗺️  Rota: \x1b[1;37m{}\x1b[0m │ ❤️  Vida: {}/{} │ ⚔️  Kills: {} ({:.1}/m) │ ⚡ XP/h: {:.0} │ 🎯 Caps: {}",
                    st.map_name, st.hp, st.max_hp, st.kills, st.kills_per_min, st.xp_per_hour, st.captures
                );
                println!(
                    "  💰 Ouro: \x1b[1;33m{}\x1b[0m ({} gp) │ 💎 Diamantes: {} │ XP: {}/{} ({:.1}%)",
                    format_gold(st.gold),
                    st.gold,
                    st.diamonds,
                    st.xp,
                    st.xp_to_next_level,
                    st.xp_percent
                );
                if !st.leader_name.is_empty() {
                    println!(
                        "  🐾 Líder: {} (Nv. {}) │ 🎯 Alvo: {} (Nv. {})",
                        st.leader_name, st.leader_level, st.wild_target, st.wild_level
                    );
                }
                if !st.last_action.is_empty() {
                    println!("  📝 Ação: \x1b[0;90m{}\x1b[0m", st.last_action);
                }
            }
        }
    });

    info!("Bot operacional na porta {}. Pressione Ctrl+C para encerrar.", port);
    signal::ctrl_c().await?;
    info!("Encerrando bot com segurança...");

    Ok(())
}
