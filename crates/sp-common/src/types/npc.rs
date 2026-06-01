//! NPC 实体 (Npc)
//!
//! 矽基灵魂的核心数据结构。每个 NPC 拥有独立的:
//! - 海马体 (Hippocampus): 持久化向量记忆网络
//! - 杏仁核 (Amygdala): 基于情感权重的状态机
//! - 人格特质向量: 大五人格模型 (OCEAN)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// NPC 自治等级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AutonomyLevel {
    /// 脚本驱动 (传统对话树)
    Scripted,
    /// 自主行为 (LLM 驱动的多轮推理)
    Autonomous,
    /// 超越级 (具备自我意识与进化能力)
    Transcendent,
}

impl AutonomyLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            AutonomyLevel::Scripted => "scripted",
            AutonomyLevel::Autonomous => "autonomous",
            AutonomyLevel::Transcendent => "transcendent",
        }
    }
}

/// NPC 实体 - 具备数字灵魂的自治体
///
/// 每个核心 NPC 配备独立的海马体记忆网络与杏仁核情感模块，
/// 能够记住交互历史、产生情感反应，并在离峰时段生成梦境。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Npc {
    pub id: Uuid,
    pub story_id: Uuid,
    pub name: String,
    pub avatar_url: Option<String>,

    // ── 灵魂架构 ──
    /// 大五人格向量 (OCEAN 模型)
    /// - openness: 开放性 [0, 1]
    /// - conscientiousness: 尽责性 [0, 1]
    /// - extraversion: 外向性 [0, 1]
    /// - agreeableness: 宜人性 [0, 1]
    /// - neuroticism: 神经质 [0, 1]
    pub personality: PersonalityVector,

    /// 当前情感状态 (杏仁核输出)
    pub emotional_state: EmotionalState,

    /// 核心动机描述 (驱动行为的根本动力)
    pub motivation: Option<String>,

    /// 背景故事 (注入 LLM System Prompt)
    pub backstory: Option<String>,

    // ── 行为参数 ──
    /// 逻辑约束温度 (梦境时调高至 1.5+)
    pub temperature: f64,

    /// 自治等级
    pub autonomy_level: AutonomyLevel,

    // ── 运行时状态 ──
    pub is_alive: bool,
    pub current_location: Option<String>,

    /// NPC 间关系 {npc_id: Relationship}
    pub relationships: HashMap<Uuid, NpcRelationship>,

    // ── DAO 扩展预留 (Phase 4) ──
    pub dao_address: Option<String>,
    pub treasury_balance: f64,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 大五人格向量 (OCEAN 模型)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityVector {
    pub openness: f64,           // 开放性: 好奇心、创造力
    pub conscientiousness: f64,  // 尽责性: 自律、责任感
    pub extraversion: f64,       // 外向性: 社交能力、活力
    pub agreeableness: f64,      // 宜人性: 同理心、合作性
    pub neuroticism: f64,        // 神经质: 情绪波动性
}

impl Default for PersonalityVector {
    fn default() -> Self {
        Self {
            openness: 0.5,
            conscientiousness: 0.5,
            extraversion: 0.5,
            agreeableness: 0.5,
            neuroticism: 0.3,
        }
    }
}

/// 情感状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalState {
    pub valence: f64,            // [-1, 1] 负面↔正面
    pub arousal: f64,            // [0, 1] 低唤醒↔高唤醒
    pub dominance: f64,          // [0, 1] 低支配↔高支配
    pub primary_emotion: String, // 主要情绪标签
    pub intensity: f64,          // 情绪强度 [0, 1]
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

/// NPC 间关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcRelationship {
    pub target_npc_id: Uuid,
    pub trust: f64,          // [0, 1] 信任度
    pub affection: f64,      // [0, 1] 好感度
    pub fear: f64,           // [0, 1] 恐惧度
    pub respect: f64,        // [0, 1] 尊重度
    pub history_depth: i32,  // 交互历史深度
}

/// NPC 记忆条目 (海马体存储单元)
///
/// 遵循 Ebbinghaus 遗忘曲线: R(t) = e^(-t/S)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMemory {
    pub id: Uuid,
    pub npc_id: Uuid,
    pub memory_type: MemoryType,

    /// 记忆内容 (自然语言描述)
    pub content: String,

    /// Qdrant 中的向量 ID
    pub vector_id: Option<String>,

    // ── 遗忘曲线参数 ──
    /// 记忆强度 R(t) = e^(-t/S)，随时间衰减
    pub strength: f64,
    /// 稳定性 S，每次成功回忆后增长
    pub stability: f64,
    /// 复述次数
    pub rehearsal_count: i32,
    /// 最后访问时间
    pub last_accessed: DateTime<Utc>,

    // ── 情感权重 ──
    pub emotional_valence: f64,  // [-1, 1]
    pub emotional_arousal: f64,  // [0, 1]

    // ── 来源追溯 ──
    pub source_node_id: Option<Uuid>,
    pub involved_npcs: Vec<Uuid>,

    // ── 梦境关联 (Phase 3) ──
    pub dream_generated: bool,

    pub created_at: DateTime<Utc>,
}

/// 记忆类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryType {
    /// 情景记忆 (具体事件)
    Episodic,
    /// 语义记忆 (知识与事实)
    Semantic,
    /// 程序性记忆 (技能与习惯)
    Procedural,
    /// 情感记忆 (强烈情绪事件)
    Emotional,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Episodic => "episodic",
            MemoryType::Semantic => "semantic",
            MemoryType::Procedural => "procedural",
            MemoryType::Emotional => "emotional",
        }
    }
}