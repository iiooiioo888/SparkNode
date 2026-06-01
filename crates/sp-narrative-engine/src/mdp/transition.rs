//! 状态转移函数
//!
//! 封装 MDP 的核心转移逻辑: T(s, a) → s'

use uuid::Uuid;

/// 采样结果
#[derive(Debug, Clone)]
pub struct TransitionResult {
    pub selected_edge_id: Uuid,
    pub selected_target: Uuid,
    pub probability: f64,
    pub random_value: f64,
}

/// 按概率分布采样一次转移 (轮盘赌算法)
pub fn sample_transition(
    transitions: &[(Uuid, Uuid, f64)], // (edge_id, target_id, probability)
    random_value: f64,
) -> Option<TransitionResult> {
    let mut cumulative = 0.0;
    for &(edge_id, target_id, prob) in transitions {
        cumulative += prob;
        if random_value <= cumulative {
            return Some(TransitionResult {
                selected_edge_id: edge_id,
                selected_target: target_id,
                probability: prob,
                random_value,
            });
        }
    }
    transitions.last().map(|&(edge_id, target_id, prob)| TransitionResult {
        selected_edge_id: edge_id,
        selected_target: target_id,
        probability: prob,
        random_value,
    })
}