//! WebSocket 连接处理器
//!
//! 处理前端星轨编织器的实时协作连接，
//! 基于 PulseStream 协议推送 DAG 变更事件。

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::AppState;
use super::pulse_stream::{PulseEvent, PulseStream};

/// WebSocket 升级处理
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(story_id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, story_id, state))
}

/// 处理单个 WebSocket 连接
async fn handle_socket(socket: WebSocket, story_id: Uuid, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let peer_id = Uuid::new_v4();

    tracing::info!(
        "WebSocket 连接已建立: peer={}, story={}",
        peer_id,
        story_id
    );

    let mut pulse = PulseStream::new(story_id, peer_id);
    let channel = format!("sparknode:pulse:{}", story_id);

    // 獨立 Redis 連線用於 Pub/Sub 訂閱
    let redis_client = match redis::Client::open(state.config.redis_url.clone()) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Redis 客戶端建立失敗: {}", e);
            return;
        }
    };

    let mut pubsub = match redis_client.get_async_pubsub().await {
        Ok(ps) => ps,
        Err(e) => {
            tracing::error!("Redis PubSub 連線失敗: {}", e);
            return;
        }
    };

    if let Err(e) = pubsub.subscribe(&channel).await {
        tracing::error!("Redis 訂閱失敗 {}: {}", channel, e);
        return;
    }

    let mut redis_stream = pubsub.on_message();

    loop {
        tokio::select! {
            incoming = redis_stream.next() => {
                match incoming {
                    Some(msg) => {
                        if let Ok(payload) = msg.get_payload::<String>() {
                            if sender.send(Message::Text(payload.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    None => break,
                }
            }
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(event) = serde_json::from_str::<PulseEvent>(&text) {
                            tracing::debug!("收到 PulseEvent: {:?}", event.event_type);
                            let response = pulse.handle_event(event).await;
                            if let Ok(serialized) = serde_json::to_string(&response) {
                                let _: Result<(), _> = state.redis.clone()
                                    .publish::<_, _, ()>(&channel, &serialized)
                                    .await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!("WebSocket 连接关闭: peer={}", peer_id);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// 客户端发送的消息类型
#[derive(Debug, Deserialize)]
pub struct WsMessage {
    pub event_type: String,
    pub payload: Value,
}
