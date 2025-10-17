use std::sync::Arc;
use axum::extract::{State, WebSocketUpgrade};
use axum::extract::ws::{Message, WebSocket};
use axum::response::IntoResponse;
use log::debug;
use crate::hmr::HmrMessage;
use crate::server::Server;

pub async fn ws_handler(ws: WebSocketUpgrade, State(server): State<Arc<Server>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, server))
}

pub async fn handle_socket(mut socket: WebSocket, server: Arc<Server>) {
    debug!("HMR client connected");

    if let Ok(msg) = serde_json::to_string(&HmrMessage::Connected) {
        let _ = socket.send(Message::Text(msg.into())).await;
    }

    let mut rx = server.hmr_tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&msg) {
            if socket.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    }

    debug!("HMR client disconnected");
}
