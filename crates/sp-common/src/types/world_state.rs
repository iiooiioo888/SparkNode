//! 世界状态 (WorldState)
//!
//! 完整的世界切面快照，用于 Chronos 引擎的时间轴管理。
//! 每个叙事节点在生成时会自动捕获当前世界状态，
//! 支持 O(1) 复杂度的回溯与平行宇宙 Diff 对比。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 世界状态快照
///
/// 代表某一时刻整个故事世界的完整状态切面。
/// 存储在 `world_checkpoints` 表中，支持树状回溯。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    /// 快照 ID
    pub id: Uuid,
    /// 所属故事 ID
    pub story_id: Uuid,
    /// 触发快照的叙事节点 ID
    pub node_id: Option<Uuid>,
    /// 快照类型
    pub checkpoint_type: CheckpointType,

    // ── NPC 状态 ──
    /// 所有 NPC 的当前状态 {npc_id: NpcWorldState}
    pub npc_states: HashMap<Uuid, NpcWorldState>,

    // ── NPC 关系网络 ──
    /// NPC 间关系矩阵 {(npc_a, npc_b): RelationshipState}
    pub relationships: HashMap<(Uuid, Uuid), RelationshipState>,

    // ── 环境状态 ──
    /// 当前场景的环境参数
    pub environment: EnvironmentState,

    // ── 世界标志位 (Flags) ──
    /// 全局标志位，用于条件触发器判断
    pub flags: HashMap<String, serde_json::Value>,

    // ── 时间轴元数据 ──
    /// 时间轴深度 (回溯层级)
    pub timeline_depth: i32,
    /// 父快照 ID (时间树结构)
    pub parent_id: Option<Uuid>,
    /// 与父快照的增量差异 (仅存储 diff 以节省空间)
    pub diff_from_parent: Option<serde_json::Value>,

    pub created_at: DateTime<Utc>,
}

/// 快照类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CheckpointType {
    /// 自动快照 (每次节点创建时)
    Auto,
    /// 手动快照 (作者手动保存)
    Manual,
    /// 观察者坍缩触发的快照
    Collapse,
    /// 时间回溯创建的分支快照
    Branch,
}

/// NPC 在世界快照中的状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcWorldState {
    pub npc_id: Uuid,
    pub is_alive: bool,
    pub current_location: String,
    pub health: f64,
    pub emotional_state: EmotionalState,
    pub active_effects: Vec<StatusEffect>,
    pub inventory: Vec<InventoryItem>,
}

/// 情感状态 (简化版，用于世界快照)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalState {
    pub valence: f64,      // [-1, 1]
    pub arousal: f64,      // [0, 1]
    pub dominance: f64,    // [0, 1]
    pub primary_emotion: String,
    pub intensity: f64,
}

impl Default for EmotionalState {
    fn default() -> Self {
        Self {
            valence: 0.0,
            arousal: 0.3,
            dominance: 0.5,
            primary_emotion: "neutral".to_string(),
            intensity: 0.1,
        }
    }
}

/// 状态效果 (如中毒、加速、隐身等)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEffect {
    pub effect_type: String,
    pub duration_remaining: i32,
    pub magnitude: f64,
}

/// 背包物品
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub item_id: String,
    pub name: String,
    pub quantity: i32,
    pub properties: serde_json::Value,
}

/// NPC 间关系状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipState {
    pub trust: f64,        // [0, 1] 信任度
    pub affection: f64,    // [0, 1] 好感度
    pub fear: f64,         // [0, 1] 恐惧度
    pub respect: f64,      // [0, 1] 尊重度
    pub history_depth: i32, // 交互历史深度
}

impl Default for RelationshipState {
    fn default() -> Self {
        Self {
            trust: 0.5,
            affection: 0.5,
            fear: 0.0,
            respect: 0.5,
            history_depth: 0,
        }
    }
}

/// 环境状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentState {
    pub location_name: String,
    pub time_of_day: String,       // "dawn", "morning", "noon", "afternoon", "dusk", "night"
    pub weather: String,           // "clear", "rain", "storm", "fog", "snow"
    pub temperature_celsius: f64,
    pub ambient_light: f64,        // [0, 1] 环境光照强度
    pub danger_level: f64,         // [0, 1] 区域危险等级
    pub custom_properties: serde_json::Value,
}

impl Default for EnvironmentState {
    fn default() -> Self {
        Self {
            location_name: "未知区域".to_string(),
            time_of_day: "noon".to_string(),
            weather: "clear".to_string(),
            temperature_celsius: 22.0,
            ambient_light: 0.8,
            danger_level: 0.0,
            custom_properties: serde_json::json!({}),
        }
    }
}