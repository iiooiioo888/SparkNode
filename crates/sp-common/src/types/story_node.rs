//! 叙事节点 (StoryNode)
//!
//! DAG 图中的核心实体，代表故事中的一个场景、对话、决策点或战斗。
//! 每个节点携带完整的世界状态快照，支持 Chronos 引擎的 O(1) 回溯。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 叙事节点类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeType {
    /// 场景描写节点
    Scene,
    /// 对话节点 (NPC/角色间对话)
    Dialogue,
    /// 决策分支节点 (读者/玩家选择)
    Decision,
    /// 战斗/冲突节点
    Combat,
    /// 过渡/转场节点
    Transition,
    /// 系统事件节点 (自动触发)
    SystemEvent,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeType::Scene => "scene",
            NodeType::Dialogue => "dialogue",
            NodeType::Decision => "decision",
            NodeType::Combat => "combat",
            NodeType::Transition => "transition",
            NodeType::SystemEvent => "system_event",
        }
    }
}

/// 叙事节点 - DAG 图的核心实体
///
/// 每个节点在「星轨编织器」中以可视化的形式呈现，
/// 在 Memgraph 中作为图节点存在，在 PostgreSQL 中持久化完整状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryNode {
    pub id: Uuid,
    pub story_id: Uuid,
    pub node_type: NodeType,
    pub title: Option<String>,
    pub content: Option<String>,

    // ── 星轨编织器画布坐标 ──
    pub position_x: f64,
    pub position_y: f64,

    // ── 扩展元数据 ──
    pub metadata: serde_json::Value,

    // ── 世界状态快照 (嵌入当前节点的世界切面) ──
    pub world_snapshot: serde_json::Value,

    // ── LLM 生成元数据 ──
    pub llm_provider: Option<String>,
    pub llm_prompt: Option<String>,
    pub llm_tokens_used: i32,

    // ── CRDT 版本控制 ──
    pub crdt_vector: serde_json::Value,
    pub version: i64,

    // ── 时间戳 ──
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl StoryNode {
    /// 创建新的叙事节点
    pub fn new(story_id: Uuid, node_type: NodeType, title: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            story_id,
            node_type,
            title,
            content: None,
            position_x: 0.0,
            position_y: 0.0,
            metadata: serde_json::json!({}),
            world_snapshot: serde_json::json!({}),
            llm_provider: None,
            llm_prompt: None,
            llm_tokens_used: 0,
            crdt_vector: serde_json::json!({}),
            version: 1,
            created_at: now,
            updated_at: now,
        }
    }

    /// 在画布上定位节点
    pub fn with_position(mut self, x: f64, y: f64) -> Self {
        self.position_x = x;
        self.position_y = y;
        self
    }

    /// 设置节点内容
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// 记录 LLM 生成信息
    pub fn with_llm_info(mut self, provider: &str, prompt: &str, tokens: i32) -> Self {
        self.llm_provider = Some(provider.to_string());
        self.llm_prompt = Some(prompt.to_string());
        self.llm_tokens_used = tokens;
        self
    }
}

/// 创建节点的请求 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNodeRequest {
    pub story_id: Uuid,
    pub node_type: NodeType,
    pub title: Option<String>,
    pub content: Option<String>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub metadata: Option<serde_json::Value>,
}

/// 更新节点的请求 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNodeRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub metadata: Option<serde_json::Value>,
    pub world_snapshot: Option<serde_json::Value>,
}