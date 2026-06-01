//! 蝴蝶效应差异计算
//!
//! 可视化「如果当初选择另一条路」的平行宇宙差异。

use uuid::Uuid;

/// 蝴蝶效应对比结果
#[derive(Debug, Clone)]
pub struct ButterflyDiff {
    pub branch_a: Uuid,
    pub branch_b: Uuid,
    pub divergent_node: Uuid,
    pub affected_downstream_nodes: Vec<Uuid>,
    pub npc_relationship_changes: Vec<NpcRelationshipChange>,
    pub environment_changes: Vec<EnvironmentChange>,
}

#[derive(Debug, Clone)]
pub struct NpcRelationshipChange {
    pub npc_a: Uuid,
    pub npc_b: Uuid,
    pub field: String,
    pub value_before: f64,
    pub value_after: f64,
}

#[derive(Debug, Clone)]
pub struct EnvironmentChange {
    pub field: String,
    pub before: String,
    pub after: String,
}

/// 计算两个平行分支之间的蝴蝶效应差异
pub fn compute_butterfly_effect(
    _branch_a_state: &serde_json::Value,
    _branch_b_state: &serde_json::Value,
    divergent_node: Uuid,
) -> ButterflyDiff {
    ButterflyDiff {
        branch_a: Uuid::nil(),
        branch_b: Uuid::nil(),
        divergent_node,
        affected_downstream_nodes: Vec::new(),
        npc_relationship_changes: Vec::new(),
        environment_changes: Vec::new(),
    }
}