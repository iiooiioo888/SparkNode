//! CQRS 领域事件 (Domain Event)
//!
//! Event 是不可变的事实记录，代表已经发生的状态变更。
//! 所有 Event 存储在 PostgreSQL `generation_events` 表中，
//! 并由 Graph Projection Worker 同步到 Memgraph Read Model。
//!
//! 设计原则：
//! - Event 一旦产生不可修改、不可删除
//! - Event 携带完整的载荷信息，支持从零重建状态
//! - Event 与 `generation_events` 表 schema 精确对应

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 领域事件元数据（所有事件共享的头部信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// 事件唯一标识
    pub event_id: Uuid,
    /// 所属故事 ID
    pub story_id: Uuid,
    /// 触发事件的 Actor ID（用户或系统）
    pub actor_id: Uuid,
    /// 事件发生时间
    pub timestamp: DateTime<Utc>,
    /// 向量时钟（用于 CRDT 多端同步）
    pub vector_clock: serde_json::Value,
}

impl EventMetadata {
    pub fn new(story_id: Uuid, actor_id: Uuid) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            story_id,
            actor_id,
            timestamp: Utc::now(),
            vector_clock: serde_json::json!({}),
        }
    }
}

/// 叙事领域事件
///
/// 对应 `generation_events.event_type` 枚举值。
/// 每个变体的 `payload` 字段序列化后写入 `generation_events.payload` (JSONB)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "payload")]
pub enum NarrativeEvent {
    // ── 节点事件 ──
    #[serde(rename = "node_created")]
    NodeCreated {
        meta: EventMetadata,
        node_id: Uuid,
        node_type: String,
        title: Option<String>,
        content: Option<String>,
        position_x: f64,
        position_y: f64,
        metadata: serde_json::Value,
    },

    #[serde(rename = "node_updated")]
    NodeUpdated {
        meta: EventMetadata,
        node_id: Uuid,
        title: Option<String>,
        content: Option<String>,
        position_x: Option<f64>,
        position_y: Option<f64>,
        metadata: Option<serde_json::Value>,
    },

    #[serde(rename = "node_deleted")]
    NodeDeleted {
        meta: EventMetadata,
        node_id: Uuid,
        /// 删除时级联删除的边 ID 列表
        cascaded_edge_ids: Vec<Uuid>,
    },

    // ── 边事件 ──
    #[serde(rename = "edge_linked")]
    EdgeLinked {
        meta: EventMetadata,
        edge_id: Uuid,
        source_node_id: Uuid,
        target_node_id: Uuid,
        edge_type: String,
        probability: f64,
        reward_signal: f64,
        conditions: serde_json::Value,
    },

    #[serde(rename = "edge_updated")]
    EdgeUpdated {
        meta: EventMetadata,
        edge_id: Uuid,
        probability: Option<f64>,
        reward_signal: Option<f64>,
        observer_weight: Option<f64>,
    },

    #[serde(rename = "edge_deleted")]
    EdgeDeleted {
        meta: EventMetadata,
        edge_id: Uuid,
    },

    // ── 观察者坍缩事件 ──
    #[serde(rename = "branch_collapsed")]
    BranchCollapsed {
        meta: EventMetadata,
        node_id: Uuid,
        reader_id: Uuid,
        signal: ObserverSignalEvent,
        /// 坍缩前后的概率分布变化
        shifts: Vec<ProbabilityShift>,
        /// 坍缩后选中的路径 ID
        collapsed_path_id: Uuid,
    },

    // ── Chronos 时间轴事件 ──
    #[serde(rename = "checkpoint_created")]
    CheckpointCreated {
        meta: EventMetadata,
        checkpoint_id: Uuid,
        node_id: Uuid,
        checkpoint_type: String,
        world_state: serde_json::Value,
    },

    #[serde(rename = "branch_rolled_back")]
    BranchRolledBack {
        meta: EventMetadata,
        checkpoint_id: Uuid,
        /// 回溯前的状态快照 ID
        rollback_from: Uuid,
        /// 回溯深度（距根节点的跳数）
        timeline_depth: i32,
    },
}

impl NarrativeEvent {
    /// 获取事件的元数据引用
    pub fn meta(&self) -> &EventMetadata {
        match self {
            NarrativeEvent::NodeCreated { meta, .. }
            | NarrativeEvent::NodeUpdated { meta, .. }
            | NarrativeEvent::NodeDeleted { meta, .. }
            | NarrativeEvent::EdgeLinked { meta, .. }
            | NarrativeEvent::EdgeUpdated { meta, .. }
            | NarrativeEvent::EdgeDeleted { meta, .. }
            | NarrativeEvent::BranchCollapsed { meta, .. }
            | NarrativeEvent::CheckpointCreated { meta, .. }
            | NarrativeEvent::BranchRolledBack { meta, .. } => meta,
        }
    }

    /// 获取事件类型标签（与 `generation_events.event_type` 对应）
    pub fn event_type_label(&self) -> &'static str {
        match self {
            NarrativeEvent::NodeCreated { .. } => "node_created",
            NarrativeEvent::NodeUpdated { .. } => "node_updated",
            NarrativeEvent::NodeDeleted { .. } => "node_deleted",
            NarrativeEvent::EdgeLinked { .. } => "edge_linked",
            NarrativeEvent::EdgeUpdated { .. } => "edge_updated",
            NarrativeEvent::EdgeDeleted { .. } => "edge_deleted",
            NarrativeEvent::BranchCollapsed { .. } => "branch_collapsed",
            NarrativeEvent::CheckpointCreated { .. } => "checkpoint_created",
            NarrativeEvent::BranchRolledBack { .. } => "branch_rolled_back",
        }
    }

    /// 序列化事件载荷为 JSON（用于写入 `generation_events.payload`）
    pub fn to_payload_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// 观察者信号事件载荷（存储在 Event 中的版本）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverSignalEvent {
    pub valence: f64,
    pub arousal: f64,
    pub dominance: f64,
    pub gaze_duration_ms: Option<f64>,
    pub focused_entity_id: Option<String>,
}

/// MDP 概率偏移记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityShift {
    pub edge_id: Uuid,
    pub probability_before: f64,
    pub probability_after: f64,
    pub reason: String,
}