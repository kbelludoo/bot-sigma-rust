use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    tungstenite::{client::IntoClientRequest, http::HeaderValue, Message},
    MaybeTlsStream, WebSocketStream,
};
use tracing::{error, info, warn};

pub type WsWriter = futures_util::stream::SplitSink<
    WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;

pub struct SigmaSocketClient {
    pub url: String,
    pub writer: Arc<Mutex<Option<WsWriter>>>,
}

impl SigmaSocketClient {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            writer: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect<F>(&self, mut on_message: F) -> Result<(), String>
    where
        F: FnMut(String) + Send + 'static,
    {
        let mut req = self
            .url
            .as_str()
            .into_client_request()
            .map_err(|e| format!("Invalid URL: {e}"))?;

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
            .map_err(|e| format!("WebSocket dial failed: {e}"))?;

        let (writer, mut reader) = ws_stream.split();
        *self.writer.lock().await = Some(writer);

        tokio::spawn(async move {
            while let Some(msg_result) = reader.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        on_message(text);
                    }
                    Ok(Message::Close(_)) => {
                        warn!("[Sigma WS] Fechado pelo servidor");
                        break;
                    }
                    Err(e) => {
                        error!("[Sigma WS] Erro de leitura: {e}");
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    pub async fn send_intent(&self, intent: &Value) -> Result<(), String> {
        let text = serde_json::to_string(intent).map_err(|e| e.to_string())?;
        self.send_text(text).await
    }

    pub async fn send_text(&self, text: String) -> Result<(), String> {
        let mut writer_lock = self.writer.lock().await;
        if let Some(w) = writer_lock.as_mut() {
            w.send(Message::Text(text))
                .await
                .map_err(|e| format!("Falha de envio WS: {e}"))
        } else {
            Err("WebSocket desconectado".to_string())
        }
    }

    pub async fn is_connected(&self) -> bool {
        self.writer.lock().await.is_some()
    }
}
