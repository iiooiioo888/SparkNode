//! MDP 策略迭代
//!
//! 基于 Bellman 方程的策略迭代算法，
//! 用于优化叙事剧情走向，最大化读者参与度奖励。

use uuid::Uuid;
use super::matrix::TransitionMatrix;

/// 叙事策略 (每个状态的最优动作)
pub type Policy = std::collections::HashMap<Uuid, Uuid>; // source → best_target

/// 策略迭代求解器
pub struct PolicyIterator {
    pub gamma: f64,
    pub max_iterations: u32,
    pub tolerance: f64,
}

impl PolicyIterator {
    pub fn new(gamma: f64, max_iterations: u32, tolerance: f64) -> Self {
        Self { gamma, max_iterations, tolerance }
    }

    /// 策略迭代求解最优叙事路径
    ///
    /// V(s) = max_a [R(s,a) + γ · Σ P(s'|s,a) · V(s')]
    pub fn solve(&self, matrix: &TransitionMatrix) -> (Policy, std::collections::HashMap<Uuid, f64>) {
        let mut v: std::collections::HashMap<Uuid, f64> = std::collections::HashMap::new();
        let mut policy: Policy = std::collections::HashMap::new();

        // 初始化
        for &source in matrix.matrix.keys() {
            v.insert(source, 0.0);
        }

        // 迭代
        for _ in 0..self.max_iterations {
            let mut delta: f64 = 0.0;

            // 策略评估: 更新 V(s)
            for (source, entries) in &matrix.matrix {
                let mut best_value = f64::NEG_INFINITY;
                let mut best_target = None;

                for entry in entries {
                    let future_value = v.get(&entry.target_node_id).copied().unwrap_or(0.0);
                    let q_value = entry.reward + self.gamma * future_value;

                    if q_value > best_value {
                        best_value = q_value;
                        best_target = Some(entry.target_node_id);
                    }
                }

                let old_v = v.get(source).copied().unwrap_or(0.0);
                v.insert(*source, best_value);
                delta = delta.max((best_value - old_v).abs());

                if let Some(target) = best_target {
                    policy.insert(*source, target);
                }
            }

            if delta < self.tolerance {
                break;
            }
        }

        (policy, v)
    }
}