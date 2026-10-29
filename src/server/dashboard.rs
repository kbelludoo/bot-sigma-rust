use crate::engine::supervisor::Supervisor;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::CorsLayer;

pub struct WebServer {
    supervisor: Arc<Supervisor>,
    port: u16,
    started_at: Instant,
}

impl WebServer {
    pub fn new(supervisor: Arc<Supervisor>, port: u16) -> Self {
        Self {
            supervisor,
            port,
            started_at: Instant::now(),
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let state = Arc::new(self);

        let app = Router::new()
            .route("/api/status", get(get_status))
            .layer(CorsLayer::permissive())
            .with_state(state.clone());

        let addr = format!("0.0.0.0:{}", state.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn get_status(State(state): State<Arc<WebServer>>) -> impl IntoResponse {
    let mut accounts_info = Vec::new();

    for bot in state.supervisor.bots() {
        accounts_info.push(json!({
            "id": bot.config.id,
            "name": bot.config.name,
            "running": bot.running.load(Ordering::Relaxed),
            "kills": bot.kills.load(Ordering::Relaxed),
            "gold": bot.gold.load(Ordering::Relaxed),
            "exp": bot.exp.load(Ordering::Relaxed),
            "cb_state": format!("{:?}", bot.circuit_breaker.state().await),
            "fatigue": bot.stamina.current_fatigue().await,
        }));
    }

    let payload = json!({
        "game": "Sigma Idle",
        "engine": "Native Rust Zero-GC",
        "uptime_sec": state.started_at.elapsed().as_secs(),
        "total_accounts": state.supervisor.bots().len(),
        "port": state.port,
        "accounts": accounts_info
    });

    (StatusCode::OK, Json(payload))
}
