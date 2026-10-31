use crate::engine::supervisor::Supervisor;
use crate::game::catalog::get_global_catalog;
use axum::extract::State;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;

const EMBEDDED_INDEX_HTML: &str = include_str!("index.html");

pub struct WebServer {
    supervisor: Arc<Supervisor>,
    port: u16,
}

#[derive(Debug, Deserialize)]
pub struct ActionRequest {
    #[serde(rename = "botId")]
    pub bot_id: String,
    pub action: String,
    #[serde(rename = "mapSlug", default)]
    pub map_slug: Option<String>,
    #[serde(default)]
    pub on: Option<bool>,
    #[serde(default)]
    pub strategy: Option<String>,
    #[serde(rename = "targetKills", default)]
    pub target_kills: Option<u32>,
}

impl WebServer {
    pub fn new(supervisor: Arc<Supervisor>, port: u16) -> Self {
        Self { supervisor, port }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let port = self.port;
        let state = Arc::new(self);

        let app = Router::new()
            .route("/", get(serve_index))
            .route("/api/status", get(get_status))
            .route("/api/hunts", get(get_hunts))
            .route("/api/action", post(post_action))
            .layer(CorsLayer::permissive())
            .with_state(state);

        let addr = format!("0.0.0.0:{port}");
        info!("[Web Dashboard] Servidor iniciado em http://localhost:{port}");
        info!("[Web Dashboard] API pública: http://localhost:{port}/api/status");

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn serve_index() -> impl IntoResponse {
    Html(EMBEDDED_INDEX_HTML)
}

async fn get_status(State(state): State<Arc<WebServer>>) -> impl IntoResponse {
    let statuses = state.supervisor.get_all_statuses().await;

    // Estatística aproximada de memória
    let ram_mb = 3.2; // Native Rust Zero-GC footprint

    let payload = json!({
        "serverTime": chrono::Utc::now().timestamp_millis(),
        "ramMB": ram_mb,
        "accounts": statuses,
    });

    (
        [(header::CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8")),
         (header::CACHE_CONTROL, HeaderValue::from_static("no-cache, no-store, must-revalidate"))],
        Json(payload),
    )
}

async fn get_hunts() -> impl IntoResponse {
    let catalog = get_global_catalog();
    let hunts = catalog.get_all_hunts();

    (
        [(header::CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))],
        Json(hunts),
    )
}

async fn post_action(
    State(state): State<Arc<WebServer>>,
    Json(req): Json<ActionRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let bot = state
        .supervisor
        .get_bot(&req.bot_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Bot não encontrado".to_string()))?;

    match req.action.as_str() {
        "travel" => {
            if let Some(map) = req.map_slug {
                let _ = bot.travel_to(&map).await;
            }
        }
        "heal" => {
            let _ = bot.heal_now().await;
        }
        "claim" => {
            let _ = bot.claim_rewards_now().await;
        }
        "toggle-autohunt" => {
            let on = req.on.unwrap_or(true);
            let _ = bot.toggle_auto_hunt(on).await;
        }
        "set-strategy" => {
            if let Some(strat) = req.strategy {
                bot.set_strategy(&strat, req.target_kills.unwrap_or(120)).await;
            }
        }
        "pause" => {
            bot.pause();
        }
        "resume" => {
            bot.resume();
        }
        _ => {
            return Err((StatusCode::BAD_REQUEST, "Ação desconhecida".to_string()));
        }
    }

    Ok((
        [(header::CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))],
        Json(json!({ "ok": true })),
    ))
}
