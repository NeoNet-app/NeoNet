use std::collections::HashSet;

use actix_web::{HttpRequest, HttpResponse, web};
use actix_ws::Message;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::auth::SessionToken;
use crate::state::{AppState, WsEvent};

/// `GET /v1/ws` — WebSocket endpoint for real-time event streaming.
pub async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    token: web::Data<SessionToken>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Verify Bearer token on the upgrade request.
    let authorized = (|| {
        let header = req.headers().get("Authorization")?.to_str().ok()?;
        let bearer = header.strip_prefix("Bearer ")?;
        if bearer == token.0.as_str() {
            Some(())
        } else {
            None
        }
    })();

    if authorized.is_none() {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "unauthorized",
            "message": "Missing or invalid session token"
        })));
    }

    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    // Subscribe to the broadcast channel.
    let mut ws_rx: broadcast::Receiver<WsEvent> = state.ws_tx.subscribe();

    // Spawn a task to handle incoming WS messages + push broadcast events.
    actix_web::rt::spawn(async move {
        let mut subscribed_rooms: HashSet<String> = HashSet::new();

        loop {
            tokio::select! {
                // Client → Server messages.
                client_msg = msg_stream.recv() => {
                    match client_msg {
                        Some(Ok(Message::Text(text))) => {
                            match serde_json::from_str::<WsClientMessage>(&text) {
                                Ok(WsClientMessage::Subscribe { room_ids }) => {
                                    log::info!("WS subscribe: {} rooms", room_ids.len());
                                    for rid in &room_ids {
                                        subscribed_rooms.insert(rid.clone());
                                    }
                                    let _ = session
                                        .text(serde_json::to_string(&WsAck {
                                            r#type: "ack".into(),
                                            action: "subscribe".into(),
                                            room_count: subscribed_rooms.len(),
                                        })
                                        .unwrap_or_default())
                                        .await;
                                }
                                Ok(WsClientMessage::Unsubscribe { room_ids }) => {
                                    log::info!("WS unsubscribe: {} rooms", room_ids.len());
                                    for rid in &room_ids {
                                        subscribed_rooms.remove(rid);
                                    }
                                    let _ = session
                                        .text(serde_json::to_string(&WsAck {
                                            r#type: "ack".into(),
                                            action: "unsubscribe".into(),
                                            room_count: subscribed_rooms.len(),
                                        })
                                        .unwrap_or_default())
                                        .await;
                                }
                                Err(_) => {
                                    let _ = session
                                        .text(r#"{"error":"bad_request","message":"Invalid WS message"}"#)
                                        .await;
                                }
                            }
                        }
                        Some(Ok(Message::Ping(bytes))) => {
                            let _ = session.pong(&bytes).await;
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => {}
                    }
                }
                // Broadcast → Client push.
                event = ws_rx.recv() => {
                    match event {
                        Ok(ws_event) => {
                            if subscribed_rooms.contains(&ws_event.room_id) {
                                if session.text(ws_event.payload).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            log::warn!("WS client lagged by {n} messages");
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });

    Ok(response)
}

// ── WS message types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsClientMessage {
    Subscribe { room_ids: Vec<String> },
    Unsubscribe { room_ids: Vec<String> },
}

#[derive(Debug, Serialize)]
struct WsAck {
    r#type: String,
    action: String,
    room_count: usize,
}
