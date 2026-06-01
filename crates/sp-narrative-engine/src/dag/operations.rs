//! DAG 操作层
//!
//! 封装对 DAG 的高级操作，包括节点/边的增删改查、
//! 批量操作、以及与数据库的同步逻辑。

use uuid::Uuid;
use super::graph::{DirectedAcyclicGraph, DagNode, DagEdge};

/// DAG 高级操作 trait
pub trait DagOperations {
    /// 批量添加节点
    fn add_nodes(&mut self, nodes: Vec<DagNode>);
    /// 批量添加边 (带环路检测)
    fn add_edges(&mut self, edges: Vec<DagEdge>) -> Result<(), sp_common::error::SpError>;
    /// 获取从 source 到 target 的所有路径
    fn find_all_paths(&self, source: Uuid, target: Uuid) -> Vec<Vec<Uuid>>;
    /// 计算两个节点之间的最短路径 (按边数)
    fn shortest_path(&self, source: Uuid, target: Uuid) -> Option<Vec<Uuid>>;
}

impl DagOperations for DirectedAcyclicGraph {
    fn add_nodes(&mut self, nodes: Vec<DagNode>) {
        for node in nodes {
            self.add_node(node);
        }
    }

    fn add_edges(&mut self, edges: Vec<DagEdge>) -> Result<(), sp_common::error::SpError> {
        for edge in edges {
            self.add_edge(edge)?;
        }
        Ok(())
    }

    /// DFS 回溯法查找所有路径
    fn find_all_paths(&self, source: Uuid, target: Uuid) -> Vec<Vec<Uuid>> {
        let mut paths = Vec::new();
        let mut current_path = vec![source];
        let mut visited = std::collections::HashSet::new();
        visited.insert(source);

        fn dfs(
            graph: &DirectedAcyclicGraph,
            current: Uuid,
            target: Uuid,
            path: &mut Vec<Uuid>,
            visited: &mut std::collections::HashSet<Uuid>,
            paths: &mut Vec<Vec<Uuid>>,
        ) {
            if current == target {
                paths.push(path.clone());
                return;
            }

            if let Some(edge_ids) = graph.out_edges.get(&current) {
                for edge_id in edge_ids {
                    if let Some(edge) = graph.edges.get(edge_id) {
                        if !visited.contains(&edge.target) {
                            visited.insert(edge.target);
                            path.push(edge.target);
                            dfs(graph, edge.target, target, path, visited, paths);
                            path.pop();
                            visited.remove(&edge.target);
                        }
                    }
                }
            }
        }

        dfs(self, source, target, &mut current_path, &mut visited, &mut paths);
        paths
    }

    /// BFS 最短路径
    fn shortest_path(&self, source: Uuid, target: Uuid) -> Option<Vec<Uuid>> {
        use std::collections::VecDeque;

        let mut visited = std::collections::HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(vec![source]);
        visited.insert(source);

        while let Some(path) = queue.pop_front() {
            let current = *path.last()?;
            if current == target {
                return Some(path);
            }

            if let Some(edge_ids) = self.out_edges.get(&current) {
                for edge_id in edge_ids {
                    if let Some(edge) = self.edges.get(edge_id) {
                        if !visited.contains(&edge.target) {
                            visited.insert(edge.target);
                            let mut new_path = path.clone();
                            new_path.push(edge.target);
                            queue.push_back(new_path);
                        }
                    }
                }
            }
        }

        None
    }
}