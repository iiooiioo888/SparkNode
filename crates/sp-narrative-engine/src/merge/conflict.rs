//! 合并冲突解决
//!
//! 当平行叙事分支需要合并时，处理节点和边的冲突。

use uuid::Uuid;

/// 冲突类型
#[derive(Debug, Clone)]
pub enum MergeConflict {
    /// 同一节点被两个分支修改
    NodeContentConflict {
        node_id: Uuid,
        branch_a_content: String,
        branch_b_content: String,
    },
    /// 同一边的概率被不同修改
    EdgeProbabilityConflict {
        edge_id: Uuid,
        branch_a_prob: f64,
        branch_b_prob: f64,
    },
    /// NPC 状态冲突
    NpcStateConflict {
        npc_id: Uuid,
        field: String,
        branch_a_value: serde_json::Value,
        branch_b_value: serde_json::Value,
    },
}

/// 合并策略
#[derive(Debug, Clone, Copy)]
pub enum MergeStrategy {
    /// 优先分支 A
    PreferA,
    /// 优先分支 B
    PreferB,
    /// 取平均值
    Average,
    /// LWW (最后写入胜出)
    LastWriteWins,
}

/// 合并结果
#[derive(Debug, Clone)]
pub struct MergeResult {
    pub conflicts: Vec<MergeConflict>,
    pub resolved: bool,
    pub strategy_used: MergeStrategy,
}

/// 执行分支合并
pub fn merge_branches(
    conflicts: &[MergeConflict],
    strategy: MergeStrategy,
) -> MergeResult {
    MergeResult {
        conflicts: conflicts.to_vec(),
        resolved: true,
        strategy_used: strategy,
    }
}