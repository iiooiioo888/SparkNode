//! 图结构 CRDT (Graph CRDT)
//!
//! 为星轨编织器的 DAG 图提供无冲突复制。
//! 支持节点/边的并发增删改查，保证所有副本最终收敛到同一状态。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// 图操作类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GraphOp {
    /// 添加节点
    AddNode { node_id: Uuid },
    /// 删除节点 (及其关联边)
    RemoveNode { node_id: Uuid },
    /// 更新节点属性
    UpdateNode { node_id: Uuid, field: String },
    /// 添加边
    AddEdge { edge_id: Uuid, source: Uuid, target: Uuid },
    /// 删除边
    RemoveEdge { edge_id: Uuid },
    /// 更新边属性
    UpdateEdge { edge_id: Uuid, field: String },
}

/// 图操作记录 (带向量时钟)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphOperation {
    pub op: GraphOp,
    pub writer_id: Uuid,
    pub timestamp: DateTime<Utc>,
    /// 向量时钟 {peer_id: counter}
    pub vector_clock: HashMap<Uuid, u64>,
    /// 该操作是否已被墓碑标记 (tombstone)
    pub tombstone: bool,
}

/// 图 CRDT
///
/// 基于「添加-移除-wins」策略的 DAG CRDT。
/// 节点和边的删除使用墓碑 (tombstone) 标记，
/// 确保并发删除与添加的最终一致性。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphCrdt {
    /// 故事 ID
    pub story_id: Uuid,
    /// 已知的节点集合 (存活节点)
    pub nodes: HashSet<Uuid>,
    /// 已墓碑标记的节点
    pub tombstoned_nodes: HashSet<Uuid>,
    /// 边集合 {edge_id: (source, target)}
    pub edges: HashMap<Uuid, (Uuid, Uuid)>,
    /// 已墓碑标记的边
    pub tombstoned_edges: HashSet<Uuid>,
    /// 操作日志 (用于同步与冲突检测)
    pub operations: Vec<GraphOperation>,
    /// 本节点的向量时钟
    pub vector_clock: HashMap<Uuid, u64>,
    /// 本节点的 Peer ID
    pub peer_id: Uuid,
}

impl GraphCrdt {
    /// 创建新的图 CRDT 实例
    pub fn new(story_id: Uuid, peer_id: Uuid) -> Self {
        Self {
            story_id,
            nodes: HashSet::new(),
            tombstoned_nodes: HashSet::new(),
            edges: HashMap::new(),
            tombstoned_edges: HashSet::new(),
            operations: Vec::new(),
            vector_clock: HashMap::new(),
            peer_id,
        }
    }

    /// 递增本节点的逻辑时钟
    fn tick(&mut self) {
        let counter = self.vector_clock.entry(self.peer_id).or_insert(0);
        *counter += 1;
    }

    /// 添加节点
    pub fn add_node(&mut self, node_id: Uuid) {
        self.tick();
        self.nodes.insert(node_id);
        // 如果之前被墓碑了，移除墓碑 (add-wins 语义)
        self.tombstoned_nodes.remove(&node_id);

        self.operations.push(GraphOperation {
            op: GraphOp::AddNode { node_id },
            writer_id: self.peer_id,
            timestamp: Utc::now(),
            vector_clock: self.vector_clock.clone(),
            tombstone: false,
        });
    }

    /// 删除节点 (墓碑标记)
    pub fn remove_node(&mut self, node_id: Uuid) {
        self.tick();
        self.nodes.remove(&node_id);
        self.tombstoned_nodes.insert(node_id);

        // 级联删除关联边
        let edges_to_remove: Vec<Uuid> = self
            .edges
            .iter()
            .filter(|(_, &(s, t))| s == node_id || t == node_id)
            .map(|(&id, _)| id)
            .collect();

        for edge_id in edges_to_remove {
            self.edges.remove(&edge_id);
            self.tombstoned_edges.insert(edge_id);
        }

        self.operations.push(GraphOperation {
            op: GraphOp::RemoveNode { node_id },
            writer_id: self.peer_id,
            timestamp: Utc::now(),
            vector_clock: self.vector_clock.clone(),
            tombstone: false,
        });
    }

    /// 添加边 (需两端节点均存活)
    pub fn add_edge(&mut self, edge_id: Uuid, source: Uuid, target: Uuid) -> bool {
        // 验证两端节点存在且未被墓碑
        if !self.nodes.contains(&source)
            || !self.nodes.contains(&target)
            || self.tombstoned_nodes.contains(&source)
            || self.tombstoned_nodes.contains(&target)
        {
            return false;
        }

        self.tick();
        self.edges.insert(edge_id, (source, target));
        self.tombstoned_edges.remove(&edge_id);

        self.operations.push(GraphOperation {
            op: GraphOp::AddEdge { edge_id, source, target },
            writer_id: self.peer_id,
            timestamp: Utc::now(),
            vector_clock: self.vector_clock.clone(),
            tombstone: false,
        });

        true
    }

    /// 删除边 (墓碑标记)
    pub fn remove_edge(&mut self, edge_id: Uuid) {
        self.tick();
        self.edges.remove(&edge_id);
        self.tombstoned_edges.insert(edge_id);

        self.operations.push(GraphOperation {
            op: GraphOp::RemoveEdge { edge_id },
            writer_id: self.peer_id,
            timestamp: Utc::now(),
            vector_clock: self.vector_clock.clone(),
            tombstone: false,
        });
    }

    /// 检测添加边是否会形成环路 (DAG 约束)
    pub fn would_create_cycle(&self, source: Uuid, target: Uuid) -> bool {
        // BFS: 从 target 出发，看能否到达 source
        let mut visited = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(target);

        while let Some(current) = queue.pop_front() {
            if current == source {
                return true;
            }
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            for (_, &(s, t)) in self.edges.iter() {
                if s == current && !visited.contains(&t) {
                    queue.push_back(t);
                }
            }
        }

        false
    }

    /// 获取节点的出边
    pub fn out_edges(&self, node_id: Uuid) -> Vec<(Uuid, Uuid)> {
        self.edges
            .iter()
            .filter(|(_, &(s, _))| s == node_id)
            .map(|(&edge_id, &(_, target))| (edge_id, target))
            .collect()
    }

    /// 获取节点的入边
    pub fn in_edges(&self, node_id: Uuid) -> Vec<(Uuid, Uuid)> {
        self.edges
            .iter()
            .filter(|(_, &(_, t))| t == node_id)
            .map(|(&edge_id, &(source, _))| (edge_id, source))
            .collect()
    }

    /// 合并远程操作 (CRDT 合并)
    pub fn merge_remote(&mut self, remote_ops: &[GraphOperation]) {
        for op in remote_ops {
            // 跳过已见过的操作 (基于向量时钟比较)
            if self.has_seen(&op.vector_clock) {
                continue;
            }

            match &op.op {
                GraphOp::AddNode { node_id } => {
                    self.nodes.insert(*node_id);
                    self.tombstoned_nodes.remove(node_id);
                }
                GraphOp::RemoveNode { node_id } => {
                    self.nodes.remove(node_id);
                    self.tombstoned_nodes.insert(*node_id);
                }
                GraphOp::AddEdge { edge_id, source, target } => {
                    if self.nodes.contains(source) && self.nodes.contains(target) {
                        self.edges.insert(*edge_id, (*source, *target));
                        self.tombstoned_edges.remove(edge_id);
                    }
                }
                GraphOp::RemoveEdge { edge_id } => {
                    self.edges.remove(edge_id);
                    self.tombstoned_edges.insert(*edge_id);
                }
                _ => {} // UpdateNode/UpdateEdge 由 LWW Register 处理
            }

            // 更新向量时钟
            for (&peer, &counter) in &op.vector_clock {
                let local = self.vector_clock.entry(peer).or_insert(0);
                *local = (*local).max(counter);
            }

            self.operations.push(op.clone());
        }
    }

    /// 检查是否已经见过某个向量时钟状态
    fn has_seen(&self, remote_clock: &HashMap<Uuid, u64>) -> bool {
        for (&peer, &remote_counter) in remote_clock {
            let local_counter = self.vector_clock.get(&peer).copied().unwrap_or(0);
            if remote_counter > local_counter {
                return false;
            }
        }
        true
    }

    /// 获取存活节点数量
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 获取存活边数量
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}