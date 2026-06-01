//! NPC Agent 核心循环
//!
//! 每个 NPC 的行为决策引擎:
//! 1. 感知环境 → 2. 检索记忆 → 3. 情感评估 → 4. 行为决策 → 5. 执行动作

use uuid::Uuid;
use super::super::hippocampus::Hippocampus;
use super::super::amygdala::Amygdala;

/// NPC Agent
pub struct NpcAgent {
    pub npc_id: Uuid,
    pub hippocampus: Hippocampus,
    pub amygdala: Amygdala,
    pub temperature: f64,
}

/// 行为决策结果
#[derive(Debug, Clone)]
pub struct BehaviorDecision {
    pub action: String,
    pub dialogue: Option<String>,
    pub internal_monologue: String,
    pub confidence: f64,
    pub triggered_memory_ids: Vec<Uuid>,
}

impl NpcAgent {
    pub fn new(npc_id: Uuid, temperature: f64) -> Self {
        Self {
            npc_id,
            hippocampus: Hippocampus::new(npc_id),
            amygdala: Amygdala::new(),
            temperature,
        }
    }

    /// 核心行为循环: 感知 → 记忆 → 情感 → 决策
    pub async fn decide(&mut self, situation: &str) -> BehaviorDecision {
        // 1. 检索相关记忆
        let memories = self.hippocampus.recall(situation, 5).await;
        let memory_ids: Vec<Uuid> = memories.iter().map(|m| m.memory_id).collect();

        // 2. 情感状态影响决策温度
        let emotional_bias = self.amygdala.current_state.valence;

        // 3. 生成决策 (委托给 LLM)
        BehaviorDecision {
            action: "observe".to_string(),
            dialogue: None,
            internal_monologue: format!(
                "当前情感: {} (强度: {:.2}), 检索到 {} 条相关记忆",
                self.amygdala.current_state.primary_emotion,
                self.amygdala.current_state.intensity,
                memories.len()
            ),
            confidence: 0.5 + emotional_bias * 0.1,
            triggered_memory_ids: memory_ids,
        }
    }
}