//! 平行宇宙 Diff 算法 (Chronos 基础)
//!
//! 计算两个世界状态快照之间的差异，
//! 用于蝴蝶效应可视化和时间回溯对比。

use std::collections::HashMap;
use uuid::Uuid;

/// 差异类型
#[derive(Debug, Clone)]
pub enum DiffKind {
    /// 节点新增
    NodeAdded(Uuid),
    /// 节点删除
    NodeRemoved(Uuid),
    /// 节点属性变更
    NodeChanged { node_id: Uuid, field: String, old: String, new: String },
    /// 边新增
    EdgeAdded(Uuid),
    /// 边删除
    EdgeRemoved(Uuid),
    /// 边概率变更
    EdgeProbabilityChanged { edge_id: Uuid, old: f64, new: f64 },
}

/// 两个世界状态之间的 Diff 结果
#[derive(Debug, Clone)]
pub struct WorldDiff {
    pub diffs: Vec<DiffKind>,
    pub total_changes: usize,
    pub impact_radius: usize, // 受影响的节点数 (蝴蝶效应半径)
}

impl WorldDiff {
    pub fn new() -> Self {
        Self {
            diffs: Vec::new(),
            total_changes: 0,
            impact_radius: 0,
        }
    }

    /// 计算蝴蝶效应半径 (受影响的下游节点数)
    pub fn compute_impact_radius(
        &self,
        dag: &super::graph::DirectedAcyclicGraph,
    ) -> usize {
        let mut affected = std::collections::HashSet::new();
        for diff in &self.diffs {
            match diff {
                DiffKind::NodeChanged { node_id, .. } => {
                    affected.insert(*node_id);
                    // 遍历所有下游节点
                    let traversal = dag.traverse(*node_id, super::traversal::TraversalMode::BreadthFirst);
                    for id in traversal.visited {
                        affected.insert(id);
                    }
                }
                _ => {}
            }
        }
        self.impact_radius
    }
}

impl Default for WorldDiff {
    fn default() -> Self {
        Self::new()
    }
}