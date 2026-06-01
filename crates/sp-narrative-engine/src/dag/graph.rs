//! DAG 图结构
//!
//! 基于邻接表的有向无环图实现，专为叙事引擎优化。
//! 每个节点代表一个叙事场景，每条边携带 MDP 转移概率。

use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

/// DAG 邻接表节点
#[derive(Debug, Clone)]
pub struct DagNode {
    pub id: Uuid,
    pub node_type: String,
    pub title: Option<String>,
}

/// DAG 边
#[derive(Debug, Clone)]
pub struct DagEdge {
    pub id: Uuid,
    pub source: Uuid,
    pub target: Uuid,
    pub edge_type: String,
    pub probability: f64,
    pub observer_weight: f64,
}

/// 有向无环图 (DAG)
///
/// 为星轨编织器提供高性能的图操作。
/// 使用邻接表 + 反向索引实现 O(1) 的边查询。
#[derive(Debug, Clone)]
pub struct DirectedAcyclicGraph {
    /// 所有节点
    pub nodes: HashMap<Uuid, DagNode>,
    /// 出边索引: source_id → [edge_ids]
    pub out_edges: HashMap<Uuid, Vec<Uuid>>,
    /// 入边索引: target_id → [edge_ids]
    pub in_edges: HashMap<Uuid, Vec<Uuid>>,
    /// 所有边
    pub edges: HashMap<Uuid, DagEdge>,
    /// 入度表 (用于拓扑排序)
    pub in_degree: HashMap<Uuid, usize>,
}

impl DirectedAcyclicGraph {
    /// 创建空 DAG
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            out_edges: HashMap::new(),
            in_edges: HashMap::new(),
            edges: HashMap::new(),
            in_degree: HashMap::new(),
        }
    }

    /// 添加节点
    pub fn add_node(&mut self, node: DagNode) {
        self.in_degree.entry(node.id).or_insert(0);
        self.nodes.insert(node.id, node);
    }

    /// 添加边 (自动检测环路)
    pub fn add_edge(&mut self, edge: DagEdge) -> Result<(), sp_common::error::SpError> {
        // 验证两端节点存在
        if !self.nodes.contains_key(&edge.source) || !self.nodes.contains_key(&edge.target) {
            return Err(sp_common::error::SpError::Internal(
                "边的源节点或目标节点不存在".to_string(),
            ));
        }

        // 环路检测: 如果添加这条边会形成环，则拒绝
        if self.would_create_cycle(edge.source, edge.target) {
            return Err(sp_common::error::SpError::DagCycleDetected {
                source_id: edge.source,
                target_id: edge.target,
            });
        }

        // 更新索引
        self.out_edges
            .entry(edge.source)
            .or_default()
            .push(edge.id);
        self.in_edges
            .entry(edge.target)
            .or_default()
            .push(edge.id);

        // 更新入度
        *self.in_degree.entry(edge.target).or_insert(0) += 1;

        self.edges.insert(edge.id, edge);
        Ok(())
    }

    /// 检测添加 source→target 的边是否会形成环路
    /// BFS: 从 target 出发，看能否到达 source
    pub fn would_create_cycle(&self, source: Uuid, target: Uuid) -> bool {
        if source == target {
            return true;
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(target);

        while let Some(current) = queue.pop_front() {
            if current == source {
                return true;
            }
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            // 遍历 current 的所有出边
            if let Some(edge_ids) = self.out_edges.get(&current) {
                for edge_id in edge_ids {
                    if let Some(edge) = self.edges.get(edge_id) {
                        if !visited.contains(&edge.target) {
                            queue.push_back(edge.target);
                        }
                    }
                }
            }
        }

        false
    }

    /// 拓扑排序 (Kahn 算法)
    pub fn topological_sort(&self) -> Result<Vec<Uuid>, sp_common::error::SpError> {
        let mut in_degree = self.in_degree.clone();
        let mut queue: VecDeque<Uuid> = VecDeque::new();
        let mut result = Vec::new();

        // 将所有入度为 0 的节点加入队列
        for (&node_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node_id);
            }
        }

        while let Some(current) = queue.pop_front() {
            result.push(current);

            // 减少后继节点的入度
            if let Some(edge_ids) = self.out_edges.get(&current) {
                for edge_id in edge_ids {
                    if let Some(edge) = self.edges.get(edge_id) {
                        let deg = in_degree.get_mut(&edge.target).unwrap();
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(edge.target);
                        }
                    }
                }
            }
        }

        if result.len() == self.nodes.len() {
            Ok(result)
        } else {
            // 存在环路 (理论上不应到达此处，因为 add_edge 已做检测)
            Err(sp_common::error::SpError::Internal(
                "拓扑排序失败: 图中存在环路".to_string(),
            ))
        }
    }

    /// 获取节点数量
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 获取边数量
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// 获取所有根节点 (入度为 0)
    pub fn root_nodes(&self) -> Vec<Uuid> {
        self.in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect()
    }

    /// 获取所有叶节点 (出度为 0)
    pub fn leaf_nodes(&self) -> Vec<Uuid> {
        self.nodes
            .keys()
            .filter(|id| {
                self.out_edges
                    .get(id)
                    .map_or(true, |edges| edges.is_empty())
            })
            .copied()
            .collect()
    }

    /// 删除节点 (级联删除关联边)
    pub fn remove_node(&mut self, node_id: Uuid) {
        // 删除所有出边
        if let Some(out_edge_ids) = self.out_edges.remove(&node_id) {
            for edge_id in out_edge_ids {
                if let Some(edge) = self.edges.remove(&edge_id) {
                    if let Some(in_edges) = self.in_edges.get_mut(&edge.target) {
                        in_edges.retain(|id| *id != edge_id);
                    }
                    if let Some(deg) = self.in_degree.get_mut(&edge.target) {
                        *deg = deg.saturating_sub(1);
                    }
                }
            }
        }

        // 删除所有入边
        if let Some(in_edge_ids) = self.in_edges.remove(&node_id) {
            for edge_id in in_edge_ids {
                if let Some(edge) = self.edges.remove(&edge_id) {
                    if let Some(out_edges) = self.out_edges.get_mut(&edge.source) {
                        out_edges.retain(|id| *id != edge_id);
                    }
                }
            }
        }

        self.nodes.remove(&node_id);
        self.in_degree.remove(&node_id);
    }
}

impl Default for DirectedAcyclicGraph {
    fn default() -> Self {
        Self::new()
    }
}