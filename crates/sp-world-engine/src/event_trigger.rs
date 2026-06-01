//! 物理事件触发器
//!
//! LLM 输出的文本经解析后生成事件触发器，
//! 驱动世界引擎的物理模拟与 NPC 行为。

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 事件触发器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTrigger {
    pub id: Uuid,
    pub event_type: String,    // "fire", "explosion", "flood", "earthquake"
    pub source_entity: Option<Uuid>,
    pub position: [f64; 3],    // [x, y, z]
    pub magnitude: f64,        // 影响范围/强度
    pub duration_secs: f64,
    pub affected_entities: Vec<Uuid>,
}