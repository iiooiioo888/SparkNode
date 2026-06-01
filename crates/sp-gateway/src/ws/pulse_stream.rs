//! PulseStream 协议
//!
//! 灵犀节点的实时事件推送协议。
//! 基于 WebSocket 传输，支持 DAG 图谱的增量同步、
//! LLM 生成进度推送、以及观察者坍缩事件广播。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// PulseStream 事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseEvent {
    /// 事件类型
    pub event_type: PulseEventType,
    /// 事件载荷
    pub payload: serde_json::Value,
    /// 发送者 Peer ID
    pub peer_id: Uuid,
    /// 事件时间戳
    pub timestamp: DateTime<Utc>,
    /// 向量时钟 (用于 CRDT 同步)
    pub vector_clock: std::collections::HashMap<Uuid, u64>,
}

/// PulseStream 事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PulseEventType {
    // ── DAG 图谱事件 ──
    /// 节点已创建
    NodeCreated,
    /// 节点已更新
    NodeUpdated,
    /// 节点已删除
    NodeDeleted,
    /// 节点位置变更 (画布拖拽)
    NodeMoved,
    /// 边已创建
    EdgeCreated,
    /// 边已更新
    EdgeUpdated,
    /// 边已删除
    EdgeDeleted,

    // ── 生成事件 ──
    /// LLM 开始生成
    GenerateStarted,
    /// LLM 生成进度 (流式 Token)
    GenerateProgress,
    /// LLM 生成完成
    GenerateCompleted,
    /// LLM 生成失败
    GenerateFailed,

    // ── 观察者事件 ──
    /// 观察者坍缩已触发
    ObserverCollapse,
    /// MDP 概率分布已更新
    MdpMatrixUpdated,

    // ── 协作事件 ──
    /// 协作者光标位置
    CursorPosition,
    /// 协作者选区
    SelectionChanged,
    /// 协作者上线
    PeerJoined,
    /// 协作者离线
    PeerLeft,
}

/// PulseStream 管理器
///
/// 管理单个故事的实时协作会话。
pub struct PulseStream {
    pub story_id: Uuid,
    pub peer_id: Uuid,
    /// 本节点的向量时钟
    pub vector_clock: std::collections::HashMap<Uuid, u64>,
}

impl PulseStream {
    pub fn new(story_id: Uuid, peer_id: Uuid) -> Self {
        Self {
            story_id,
            peer_id,
            vector_clock: std::collections::HashMap::new(),
        }
    }

    /// 处理收到的 PulseEvent，返回需要广播的事件
    pub async fn handle_event(&mut self, event: PulseEvent) -> PulseEvent {
        // 更新向量时钟
        for (&peer, &counter) in &event.vector_clock {
            let local = self.vector_clock.entry(peer).or_insert(0);
            *local = (*local).max(counter);
        }

        // 递增本节点时钟
        let counter = self.vector_clock.entry(self.peer_id).or_insert(0);
        *counter += 1;

        // 返回带更新时钟的事件用于广播
        PulseEvent {
            event_type: event.event_type,
            payload: event.payload,
            peer_id: self.peer_id,
            timestamp: Utc::now(),
            vector_clock: self.vector_clock.clone(),
        }
    }

    /// 构造节点创建事件
    pub fn node_created_event(&mut self, node_id: Uuid, payload: serde_json::Value) -> PulseEvent {
        let counter = self.vector_clock.entry(self.peer_id).or_insert(0);
        *counter += 1;

        PulseEvent {
            event_type: PulseEventType::NodeCreated,
            payload,
            peer_id: self.peer_id,
            timestamp: Utc::now(),
            vector_clock: self.vector_clock.clone(),
        }
    }

    /// 构造观察者坍缩事件
    pub fn collapse_event(&mut self, payload: serde_json::Value) -> PulseEvent {
        let counter = self.vector_clock.entry(self.peer_id).or_insert(0);
        *counter += 1;

        PulseEvent {
            event_type: PulseEventType::ObserverCollapse,
            payload,
            peer_id: self.peer_id,
            timestamp: Utc::now(),
            vector_clock: self.vector_clock.clone(),
        }
    }
}