//! WebSocket 连接处理器
//!
//! 处理前端星轨编织器的实时协作连接，
//! 基于 PulseStream 协议推送 DAG 变更事件。
//!
//! **升级**: 支持多态消息分发，在单条 WebSocket 连接上
//! 同时处理 DAG 协作事件与 AI 推理 Token 流推送。

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::AppState;
use super::message::SparkWsMessage;
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
///
/// 实现多态消息分发:
/// - `SparkWsMessage::Pulse` → PulseStream 协作处理
/// - `SparkWsMessage::AiRequest` → gRPC AI 推理流
/// - `SparkWsMessage::Ping` → 心跳响应
/// - 旧版 PulseEvent JSON → 向后兼容
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
            // ── 从 Redis PubSub 接收协作广播（其他 peer 的事件）──
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
            // ── 接收前端 WebSocket 消息（多态分发）──
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        dispatch_message(
                            &text,
                            &mut pulse,
                            &state,
                            &channel,
                            &mut sender,
                        ).await;
                    }
                    Some(Ok(Message::Binary(data))) => {
                        // 尝试从二进制帧反序列化 Protobuf/MsgPack
                        tracing::debug!("收到二进制帧: {} bytes", data.len());
                        // TODO: 当 Protobuf 契约就绪后，在此处解码并分发
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

/// 多态消息分发
///
/// 优先尝试解析为 `SparkWsMessage`（新协议），
/// 若失败则回退到 `PulseEvent`（向后兼容旧版 PulseStream 客户端）。
async fn dispatch_message(
    text: &str,
    pulse: &mut PulseStream,
    state: &AppState,
    channel: &str,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
) {
    // 优先尝试新版多态协议
    if let Ok(ws_msg) = serde_json::from_str::<SparkWsMessage>(text) {
        match ws_msg {
            // ── DAG 协作事件 ──
            SparkWsMessage::Pulse(event) => {
                tracing::debug!("收到 PulseEvent (多态): {:?}", event.event_type);
                let response = pulse.handle_event(event).await;
                broadcast_to_redis(state, channel, &response).await;
            }

            // ── AI 推理请求 ──
            SparkWsMessage::AiRequest { request_id, prompt, model, max_tokens } => {
                tracing::info!(
                    "收到 AI 推理请求: request_id={}, model={:?}, max_tokens={:?}",
                    request_id, model, max_tokens
                );
                // 启动异步 AI 推理任务，将 Token 流推送回前端
                handle_ai_request(
                    request_id, prompt, model, max_tokens,
                    state, sender,
                ).await;
            }

            // ── 心跳 ──
            SparkWsMessage::Ping => {
                let pong = serde_json::to_string(&SparkWsMessage::Pong)
                    .unwrap_or_else(|_| r#"{"type":"pong"}"#.to_string());
                let _ = sender.send(Message::Text(pong.into())).await;
            }

            // 其他消息类型（服务端推送类，客户端不应发送）
            SparkWsMessage::AiStreamChunk { .. }
            | SparkWsMessage::AiError { .. }
            | SparkWsMessage::Pong => {
                tracing::warn!("收到不应由客户端发送的消息类型: {:?}", ws_msg);
            }
        }
        return;
    }

    // 回退: 尝试旧版 PulseEvent 协议（向后兼容）
    if let Ok(event) = serde_json::from_str::<PulseEvent>(text) {
        tracing::debug!("收到 PulseEvent (旧协议): {:?}", event.event_type);
        let response = pulse.handle_event(event).await;
        broadcast_to_redis(state, channel, &response).await;
        return;
    }

    tracing::warn!("无法解析 WebSocket 消息: {}", &text[..text.len().min(100)]);
}

/// 将事件广播到 Redis PubSub 频道
async fn broadcast_to_redis(state: &AppState, channel: &str, event: &PulseEvent) {
    if let Ok(serialized) = serde_json::to_string(event) {
        let _: Result<(), _> = state
            .redis
            .clone()
            .publish::<_, _, ()>(channel, &serialized)
            .await;
    }
}

/// 处理 AI 推理请求
///
/// 通过 gRPC 流式调用向 Python AI 层发起推理，
/// 将返回的 Token 流实时推送为 `SparkWsMessage::AiStreamChunk`。
///
/// **gRPC 对接点**: 当 `GrpcPool` 就绪后，替换为真实的
/// `NarrativeServiceClient::stream_generate()` 调用。
async fn handle_ai_request(
    request_id: Uuid,
    prompt: String,
    model: Option<String>,
    max_tokens: Option<u32>,
    state: &AppState,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
) {
    // ── 骨架实现（gRPC 对接点）──
    // 完整实现:
    //
    // let grpc = &state.grpc_pool;
    // let request = tonic::Request::new(GenerateRequest {
    //     prompt, model: model.unwrap_or_default(), max_tokens: max_tokens.unwrap_or(512),
    // });
    // let mut stream = grpc.narrative_client().stream_generate(request).await?;
    //
    // while let Some(chunk) = stream.message().await? {
    //     let msg = SparkWsMessage::AiStreamChunk {
    //         request_id,
    //         token: chunk.text,
    //         done: chunk.finished,
    //     };
    //     sender.send(Message::Text(serde_json::to_string(&msg)?.into())).await?;
    // }

    // 暂时推送模拟响应
    tracing::debug!(
        "AI 推理骨架实现: request_id={}, prompt_len={}",
        request_id,
        prompt.len()
    );

    let start_msg = SparkWsMessage::AiStreamChunk {
        request_id,
        token: format!("[AI推理已排队] request_id={}, model={:?}", request_id, model),
        done: false,
    };
    if let Ok(json) = serde_json::to_string(&start_msg) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    let done_msg = SparkWsMessage::AiStreamChunk {
        request_id,
        token: String::new(),
        done: true,
    };
    if let Ok(json) = serde_json::to_string(&done_msg) {
        let _ = sender.send(Message::Text(json.into())).await;
    }
}

/// 客户端发送的消息类型（旧版兼容）
#[derive(Debug, Deserialize)]
pub struct WsMessage {
    pub event_type: String,
    pub payload: Value,
}