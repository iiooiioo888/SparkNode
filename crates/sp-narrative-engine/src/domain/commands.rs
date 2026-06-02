//! CQRS 写入端命令 (Command)
//!
//! 所有对叙事图的修改操作都封装为 Command。
//! Command 由聚合根（Aggregate Root）验证后产出 Event。
//!
//! 设计原则：
//! - 命令是意图的表达，不是结果
//! - 命令必须包含足够的上下文用于业务验证
//! - 命令可以被拒绝（返回错误），但 Event 一旦产生则不可变

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 叙事图命令枚举
///
/// 所有修改 `NarrativeGraph` 状态的操作必须通过此枚举下达。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command_type", content = "payload")]
pub enum NarrativeCommand {
    // ── 节点操作 ──
    /// 创建叙事节点
    #[serde(rename = "create_node")]
    CreateNode(CreateNodeCommand),

    /// 更新叙事节点
    #[serde(rename = "update_node")]
    UpdateNode(UpdateNodeCommand),

    /// 删除叙事节点（级联删除关联边）
    #[serde(rename = "delete_node")]
    DeleteNode(DeleteNodeCommand),

    // ── 边操作 ──
    /// 连接叙事边（创建两个节点间的转移关系）
    #[serde(rename = "link_edge")]
    LinkEdge(LinkEdgeCommand),

    /// 更新叙事边概率
    #[serde(rename = "update_edge")]
    UpdateEdge(UpdateEdgeCommand),

    /// 删除叙事边
    #[serde(rename = "delete_edge")]
    DeleteEdge(DeleteEdgeCommand),

    // ── 观察者坍缩操作 ──
    /// 触发观察者坍缩（注入情感权重，重塑概率分布）
    #[serde(rename = "collapse_branch")]
    CollapseBranch(CollapseBranchCommand),

    // ── Chronos 时间轴操作 ──
    /// 创建世界状态快照
    #[serde(rename = "create_checkpoint")]
    CreateCheckpoint(CreateCheckpointCommand),

    /// 回溯到指定快照
    #[serde(rename = "rollback")]
    Rollback(RollbackCommand),
}

/// 创建叙事节点命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNodeCommand {
    pub story_id: Uuid,
    pub node_type: String,
    pub title: Option<String>,
    pub content: Option<String>,
    pub position_x: f64,
    pub position_y: f64,
    pub metadata: Option<serde_json::Value>,
}

/// 更新叙事节点命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNodeCommand {
    pub story_id: Uuid,
    pub node_id: Uuid,
    pub title: Option<String>,
    pub content: Option<String>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub metadata: Option<serde_json::Value>,
}

/// 删除叙事节点命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteNodeCommand {
    pub story_id: Uuid,
    pub node_id: Uuid,
}

/// 连接叙事边命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkEdgeCommand {
    pub story_id: Uuid,
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub edge_type: String,
    pub probability: f64,
    pub reward_signal: Option<f64>,
    pub conditions: Option<serde_json::Value>,
}

/// 更新叙事边命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEdgeCommand {
    pub story_id: Uuid,
    pub edge_id: Uuid,
    pub probability: Option<f64>,
    pub reward_signal: Option<f64>,
    pub observer_weight: Option<f64>,
}

/// 删除叙事边命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteEdgeCommand {
    pub story_id: Uuid,
    pub edge_id: Uuid,
}

/// 观察者坍缩命令
///
/// 当读者（观察者）对叙事产生情感反馈时，
/// 通过此命令将观察者信号注入 MDP 概率分布，
/// 触发波函数坍缩，重塑分支概率。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollapseBranchCommand {
    pub story_id: Uuid,
    pub node_id: Uuid,
    pub reader_id: Uuid,
    /// 观察者情感信号
    pub signal: ObserverSignalPayload,
}

/// 观察者信号载荷（简化版，与 proto ObserverSignal 对应）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverSignalPayload {
    /// 情感效价 [-1, 1]
    pub valence: f64,
    /// 情感唤醒度 [0, 1]
    pub arousal: f64,
    /// 支配度 [0, 1]
    pub dominance: f64,
    /// 视线停留时长 (ms)
    pub gaze_duration_ms: Option<f64>,
    /// 聚焦的实体 ID
    pub focused_entity_id: Option<String>,
}

/// 创建世界状态快照命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCheckpointCommand {
    pub story_id: Uuid,
    pub node_id: Uuid,
    pub checkpoint_type: String,
    pub world_state: serde_json::Value,
}

/// 回溯到指定快照命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackCommand {
    pub story_id: Uuid,
    pub checkpoint_id: Uuid,
}