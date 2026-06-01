//! DAG 遍历算法
//!
//! BFS/DFS 遍历、以及面向叙事引擎的专用遍历模式。

use uuid::Uuid;
use super::graph::DirectedAcyclicGraph;

/// 遍历模式
#[derive(Debug, Clone, Copy)]
pub enum TraversalMode {
    /// 广度优先 (适合获取同层所有剧情分支)
    BreadthFirst,
    /// 深度优先 (适合追踪单一剧情线的深度)
    DepthFirst,
}

/// 遍历结果
#[derive(Debug, Clone)]
pub struct TraversalResult {
    pub visited: Vec<Uuid>,
    pub depth_map: std::collections::HashMap<Uuid, usize>,
}

impl DirectedAcyclicGraph {
    /// 从指定节点开始遍历
    pub fn traverse(&self, start: Uuid, mode: TraversalMode) -> TraversalResult {
        match mode {
            TraversalMode::BreadthFirst => self.bfs_traverse(start),
            TraversalMode::DepthFirst => self.dfs_traverse(start),
        }
    }

    fn bfs_traverse(&self, start: Uuid) -> TraversalResult {
        use std::collections::VecDeque;
        let mut visited = Vec::new();
        let mut depth_map = std::collections::HashMap::new();
        let mut seen = std::collections::HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back((start, 0));
        seen.insert(start);

        while let Some((current, depth)) = queue.pop_front() {
            visited.push(current);
            depth_map.insert(current, depth);

            if let Some(edge_ids) = self.out_edges.get(&current) {
                for edge_id in edge_ids {
                    if let Some(edge) = self.edges.get(edge_id) {
                        if seen.insert(edge.target) {
                            queue.push_back((edge.target, depth + 1));
                        }
                    }
                }
            }
        }

        TraversalResult { visited, depth_map }
    }

    fn dfs_traverse(&self, start: Uuid) -> TraversalResult {
        let mut visited = Vec::new();
        let mut depth_map = std::collections::HashMap::new();
        let mut seen = std::collections::HashSet::new();

        fn dfs(
            graph: &DirectedAcyclicGraph,
            current: Uuid,
            depth: usize,
            visited: &mut Vec<Uuid>,
            depth_map: &mut std::collections::HashMap<Uuid, usize>,
            seen: &mut std::collections::HashSet<Uuid>,
        ) {
            if !seen.insert(current) {
                return;
            }
            visited.push(current);
            depth_map.insert(current, depth);

            if let Some(edge_ids) = graph.out_edges.get(&current) {
                for edge_id in edge_ids {
                    if let Some(edge) = graph.edges.get(edge_id) {
                        dfs(graph, edge.target, depth + 1, visited, depth_map, seen);
                    }
                }
            }
        }

        dfs(self, start, 0, &mut visited, &mut depth_map, &mut seen);
        TraversalResult { visited, depth_map }
    }
}