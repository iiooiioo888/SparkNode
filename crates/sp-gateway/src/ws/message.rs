//! 多态 WebSocket 消息协议
//!
//! 在单条 WebSocket 连接上实现多路复用:
//! - DAG 协作事件 (PulseStream)
//! - AI 推理 Token 流
//! - AI 推理请求
//!
//! 使用 serde tag 机制实现零拷贝类型分发。

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::pulse_stream::PulseEvent;

/// SparkNode 多态 WebSocket 消息
///
/// 通过 `type` 字段进行标签分发，`data` 字段承载载荷。
/// 前端发送与后端推送使用同一枚举，实现全双工多路复用。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SparkWsMessage {
    // ── DAG 协作事件（复用 PulseStream 协议）──
    /// 节点/边的增量同步、协作光标等
    #[serde(rename = "pulse")]
    Pulse(PulseEvent),

    // ── AI 推理请求（前端 → 后端）──
    /// 前端发起 AI 推理请求
    #[serde(rename = "ai_request")]
    AiRequest {
        /// 请求唯一标识（用于关联响应流）
        request_id: Uuid,
        /// 推理提示词
        prompt: String,
        /// 可选的模型指定
        model: Option<String>,
        /// 最大生成 Token 数
        max_tokens: Option<u32>,
    },

    // ── AI 推理 Token 流（后端 → 前端）──
    /// 后端推送流式 Token
    #[serde(rename = "ai_chunk")]
    AiStreamChunk {
        /// 关联的请求 ID
        request_id: Uuid,
        /// 本次推送的 Token 文本片段
        token: String,
        /// 是否为最后一个 Token
        done: bool,
    },

    // ── AI 推理错误（后端 → 前端）──
    /// AI 推理过程中的错误通知
    #[serde(rename = "ai_error")]
    AiError {
        /// 关联的请求 ID
        request_id: Uuid,
        /// 错误描述
        error: String,
    },

    // ── 系统控制消息 ──
    /// Ping 心跳检测
    #[serde(rename = "ping")]
    Ping,

    /// Pong 心跳响应
    #[serde(rename = "pong")]
    Pong,
}

impl SparkWsMessage {
    /// 判断是否为 DAG 协作事件
    pub fn is_pulse(&self) -> bool {
        matches!(self, SparkWsMessage::Pulse(_))
    }

    /// 判断是否为 AI 相关消息
    pub fn is_ai(&self) -> bool {
        matches!(
            self,
            SparkWsMessage::AiRequest { .. }
                | SparkWsMessage::AiStreamChunk { .. }
                | SparkWsMessage::AiError { .. }
        )
    }

    /// 判断是否为心跳消息
    pub fn is_heartbeat(&self) -> bool {
        matches!(self, SparkWsMessage::Ping | SparkWsMessage::Pong)
    }

    /// 提取请求 ID（若存在）
    pub fn request_id(&self) -> Option<Uuid> {
        match self {
            SparkWsMessage::AiRequest { request_id, .. }
            | SparkWsMessage::AiStreamChunk { request_id, .. }
            | SparkWsMessage::AiError { request_id, .. } => Some(*request_id),
            _ => None,
        }
    }
}