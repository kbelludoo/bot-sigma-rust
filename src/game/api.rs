use crate::game::types::{BestiaryResponse, TrainerData};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

pub const BASE_URL: &str = "https://api.poke.sigmaproject.online";
pub const ORIGIN_URL: &str = "https://poke.sigmaproject.online";
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    #[serde(rename = "accessToken", default)]
    pub access_token: String,
    #[serde(rename = "expiresIn", default)]
    pub expires_in: Option<u64>,
    #[serde(rename = "hasTrainer", default)]
    pub has_trainer: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketResponse {
    #[serde(default)]
    pub ticket: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainerMeResponse {
    pub trainer: TrainerData,
}

#[derive(Clone)]
pub struct GameApiClient {
    client: Client,
    pub base_url: String,
}

impl GameApiClient {
    pub fn new(bind_ip: Option<&str>) -> Self {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(15));

        if let Some(ip_str) = bind_ip {
            if let Ok(ip) = IpAddr::from_str(ip_str) {
                builder = builder.local_address(ip);
            }
        }

        let client = builder.build().unwrap_or_else(|_| Client::new());
        Self {
            client,
            base_url: BASE_URL.to_string(),
        }
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<LoginResponse, String> {
        let req_body = LoginRequest {
            email: email.to_string(),
            password: password.to_string(),
        };

        let url = format!("{}/auth/login", self.base_url);
        let resp = self
            .client
            .post(&url)
            .header("User-Agent", USER_AGENT)
            .header("Origin", ORIGIN_URL)
            .header("Referer", format!("{ORIGIN_URL}/"))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/plain, */*")
            .json(&req_body)
            .send()
            .await
            .map_err(|e| format!("Falha na requisição de login: {e}"))?;

        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Falha ao ler resposta de login: {e}"))?;

        if status != StatusCode::OK {
            if let Ok(err_resp) = serde_json::from_slice::<LoginResponse>(&bytes) {
                if let Some(err) = err_resp.error {
                    return Err(format!(
                        "Login falhou ({}): {} (código: {:?})",
                        status.as_u16(),
                        err,
                        err_resp.code
                    ));
                }
            }
            return Err(format!(
                "Login falhou com status {}: {}",
                status.as_u16(),
                String::from_utf8_lossy(&bytes)
            ));
        }

        let login_resp: LoginResponse = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Erro ao decodificar JSON de login: {e}"))?;

        if login_resp.access_token.is_empty() {
            return Err("Token de acesso vazio retornado pelo servidor".to_string());
        }

        Ok(login_resp)
    }

    pub async fn get_ticket(&self, access_token: &str) -> Result<TicketResponse, String> {
        let url = format!("{}/ws/ticket", self.base_url);
        let resp = self
            .client
            .post(&url)
            .header("User-Agent", USER_AGENT)
            .header("Origin", ORIGIN_URL)
            .header("Referer", format!("{ORIGIN_URL}/"))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {access_token}"))
            .body("{}")
            .send()
            .await
            .map_err(|e| format!("Falha na requisição de ticket: {e}"))?;

        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Falha ao ler resposta de ticket: {e}"))?;

        if status != StatusCode::OK {
            let body_str = String::from_utf8_lossy(&bytes);
            if status == StatusCode::CONFLICT && body_str.contains("legal-pending") {
                let _ = self.accept_terms(access_token).await;
                let retry_resp = self
                    .client
                    .post(&url)
                    .header("User-Agent", USER_AGENT)
                    .header("Origin", ORIGIN_URL)
                    .header("Referer", format!("{ORIGIN_URL}/"))
                    .header("Content-Type", "application/json")
                    .header("Authorization", format!("Bearer {access_token}"))
                    .body("{}")
                    .send()
                    .await;
                if let Ok(retry) = retry_resp {
                    if retry.status() == StatusCode::OK {
                        if let Ok(retry_bytes) = retry.bytes().await {
                            if let Ok(ticket_resp) = serde_json::from_slice::<TicketResponse>(&retry_bytes) {
                                if !ticket_resp.ticket.is_empty() {
                                    return Ok(ticket_resp);
                                }
                            }
                        }
                    }
                }
            }
            return Err(format!(
                "Ticket falhou com status {}: {}",
                status.as_u16(),
                body_str
            ));
        }

        let ticket_resp: TicketResponse = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Erro ao decodificar ticket JSON: {e}"))?;

        if ticket_resp.ticket.is_empty() {
            return Err("Ticket vazio retornado pelo servidor".to_string());
        }

        Ok(ticket_resp)
    }

    pub async fn accept_terms(&self, access_token: &str) -> Result<(), String> {
        let url = format!("{}/legal/accept", self.base_url);
        let payload = serde_json::json!({
            "termsVersion": "2026-10-13",
            "privacyVersion": "2026-10-14",
            "isAdult": true
        });

        let _ = self
            .client
            .post(&url)
            .header("User-Agent", USER_AGENT)
            .header("Origin", ORIGIN_URL)
            .header("Referer", format!("{ORIGIN_URL}/"))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&payload)
            .send()
            .await;

        Ok(())
    }

    pub async fn get_me(&self, access_token: &str) -> Result<TrainerData, String> {
        let url = format!("{}/me", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .header("Origin", ORIGIN_URL)
            .header("Referer", format!("{ORIGIN_URL}/"))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| format!("Falha na requisição /me: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Falha em /me ({})", resp.status().as_u16()));
        }

        let me: TrainerMeResponse = resp
            .json()
            .await
            .map_err(|e| format!("Erro ao decodificar /me: {e}"))?;
        Ok(me.trainer)
    }

    pub async fn get_bestiary(&self, access_token: &str) -> Result<BestiaryResponse, String> {
        let url = format!("{}/bestiary", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .header("Origin", ORIGIN_URL)
            .header("Referer", format!("{ORIGIN_URL}/"))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| format!("Falha na requisição /bestiary: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Falha em /bestiary ({})", resp.status().as_u16()));
        }

        let bestiary: BestiaryResponse = resp
            .json()
            .await
            .map_err(|e| format!("Erro ao decodificar /bestiary: {e}"))?;
        Ok(bestiary)
    }
}
