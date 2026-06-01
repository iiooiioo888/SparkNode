//! 叙事边 (NarrativeEdge)
//!
//! DAG 图中的有向边，连接两个叙事节点。
//! 携带 MDP 转移概率、奖励信号与观察者坍缩权重，
//! 是「量子叙事叠加态」的核心数据载体。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 叙事边类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EdgeType {
    /// 因果关系 (A 导致 B)
    Causal,
    /// 分支选择 (读者在 A 处选择走向 B)
    Branch,
    /// 平行叙事 (A 和 B 同时发生)
    Parallel,
    /// 时间跳跃 (A 之后经过一段时间到 B)
    Temporal,
    /// 条件触发 (满足条件时从 A 跳转到 B)
    Conditional,
}

impl EdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeType::Causal => "causal",
            EdgeType::Branch => "branch",
            EdgeType::Parallel => "parallel",
            EdgeType::Temporal => "temporal",
            EdgeType::Conditional => "conditional",
        }
    }
}

/// 叙事边 - 连接两个叙事节点的有向边
///
/// 在 MDP 框架下，每条边代表一个状态转移:
/// - `probability`: P(s'|s,a) 转移概率
/// - `reward_signal`: R(s,a,s') 奖励信号
/// - `observer_weight`: 观察者效应权重 (被情感计算实时修改)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEdge {
    pub id: Uuid,
    pub story_id: Uuid,
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub edge_type: EdgeType,

    // ── MDP 概率权重 ──
    /// 转移概率 P(s'|s,a)，所有从同一节点出发的边概率之和应为 1.0
    pub probability: f64,
    /// 奖励信号 R(s,a,s')，用于强化学习驱动的剧情优化
    pub reward_signal: f64,

    // ── 观察者坍缩权重 ──
    /// 被前端情感计算注入的权重，影响概率分布的坍缩方向
    pub observer_weight: f64,
    /// 被观察坍缩的累计次数
    pub collapse_count: i32,

    // ── 条件触发器 ──
    /// JSON 数组，定义触发条件，如: [信任度>80, HP>50]
    pub conditions: serde_json::Value,

    // ── 扩展元数据 ──
    pub metadata: serde_json::Value,

    pub created_at: DateTime<Utc>,
}

impl NarrativeEdge {
    /// 创建新的叙事边
    pub fn new(
        story_id: Uuid,
        source_node_id: Uuid,
        target_node_id: Uuid,
        edge_type: EdgeType,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            story_id,
            source_node_id,
            target_node_id,
            edge_type,
            probability: 1.0,
            reward_signal: 0.0,
            observer_weight: 0.0,
            collapse_count: 0,
            conditions: serde_json::json!([]),
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }

    /// 设置转移概率
    pub fn with_probability(mut self, p: f64) -> Self {
        self.probability = p;
        self
    }

    /// 设置奖励信号
    pub fn with_reward(mut self, r: f64) -> Self {
        self.reward_signal = r;
        self
    }

    /// 计算观察者坍缩后的有效概率
    /// P'(s'|s,a) = softmax(P(s'|s,a) + α·O(s'))
    /// 其中 α 为观察者效应强度系数
    pub fn collapsed_probability(&self, alpha: f64) -> f64 {
        self.probability + alpha * self.observer_weight
    }
}

/// 创建边的请求 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEdgeRequest {
    pub story_id: Uuid,
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub edge_type: EdgeType,
    pub probability: Option<f64>,
    pub reward_signal: Option<f64>,
    pub conditions: Option<serde_json::Value>,
}

/// 更新边的请求 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEdgeRequest {
    pub probability: Option<f64>,
    pub reward_signal: Option<f64>,
    pub observer_weight: Option<f64>,
    pub conditions: Option<serde_json::Value>,
}