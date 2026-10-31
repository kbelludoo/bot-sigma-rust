use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    tungstenite::{client::IntoClientRequest, http::HeaderValue, Message},
    MaybeTlsStream, WebSocketStream,
};
use tracing::{error, warn};

pub type WsWriter = futures_util::stream::SplitSink<
    WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;

pub struct SigmaSocketClient {
    pub bind_ip: Option<String>,
    pub seq: AtomicI64,
    pub connected: AtomicBool,
    pub writer: Arc<Mutex<Option<WsWriter>>>,
    disconnect_tx: Arc<Mutex<Option<mpsc::Sender<()>>>>,
}

impl SigmaSocketClient {
    pub fn new(bind_ip: Option<&str>) -> Self {
        Self {
            bind_ip: bind_ip.map(|s| s.to_string()),
            seq: AtomicI64::new(0),
            connected: AtomicBool::new(false),
            writer: Arc::new(Mutex::new(None)),
            disconnect_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect<F>(
        &self,
        ticket: &str,
        server_ws_url: Option<&str>,
        disconnect_tx: mpsc::Sender<()>,
        mut on_message: F,
    ) -> Result<(), String>
    where
        F: FnMut(String) + Send + 'static,
    {
        let base_ws = server_ws_url.unwrap_or("wss://api.poke.sigmaproject.online");
        let encoded_ticket: String = url::form_urlencoded::byte_serialize(ticket.as_bytes()).collect();
        let ws_url = format!("{}/ws?ticket={}", base_ws.trim_end_matches('/'), encoded_ticket);

        let mut req = ws_url
            .as_str()
            .into_client_request()
            .map_err(|e| format!("URL WebSocket inválida: {e}"))?;

        let headers = req.headers_mut();
        headers.insert(
            "User-Agent",
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(
            "Origin",
            HeaderValue::from_static("https://poke.sigmaproject.online"),
        );

        let (ws_stream, _) = tokio_tungstenite::connect_async(req)
            .await
            .map_err(|e| format!("Falha ao discar WebSocket: {e}"))?;

        let (writer, mut reader) = ws_stream.split();
        *self.writer.lock().await = Some(writer);
        *self.disconnect_tx.lock().await = Some(disconnect_tx.clone());
        self.connected.store(true, Ordering::SeqCst);
        self.seq.store(0, Ordering::SeqCst);

        let connected_flag = Arc::new(AtomicBool::new(true));
        let cf_clone = connected_flag.clone();

        tokio::spawn(async move {
            while let Some(msg_result) = reader.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        on_message(text);
                    }
                    Ok(Message::Close(_)) => {
                        warn!("[Sigma WS] Conexão encerrada pelo servidor.");
                        break;
                    }
                    Err(e) => {
                        error!("[Sigma WS] Erro no stream: {e}");
                        break;
                    }
                    _ => {}
                }
            }
            cf_clone.store(false, Ordering::SeqCst);
            let _ = disconnect_tx.send(()).await;
        });

        Ok(())
    }

    pub async fn send_intent(&self, intent: Value) -> Result<(), String> {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
        let payload = serde_json::json!({
            "seq": seq,
            "intent": intent,
        });
        self.send_json(&payload).await
    }

    pub async fn send_json(&self, payload: &Value) -> Result<(), String> {
        let text = serde_json::to_string(payload).map_err(|e| e.to_string())?;
        self.send_text(text).await
    }

    pub async fn send_text(&self, text: String) -> Result<(), String> {
        let mut writer_guard = self.writer.lock().await;
        if let Some(w) = writer_guard.as_mut() {
            w.send(Message::Text(text))
                .await
                .map_err(|e| format!("Falha de envio WS: {e}"))
        } else {
            Err("WebSocket desconectado".to_string())
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub async fn close(&self) {
        self.connected.store(false, Ordering::SeqCst);
        let mut writer_guard = self.writer.lock().await;
        if let Some(mut w) = writer_guard.take() {
            let _ = w.close().await;
        }
    }
}
