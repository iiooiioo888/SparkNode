//! MDP 转移概率矩阵
//!
//! 维护叙事引擎的核心概率矩阵 T[s][a][s']。
//! 每一行代表从一个叙事节点出发的所有可能转移。

use std::collections::HashMap;
use uuid::Uuid;
use sp_common::types::MdpConfig;

/// 转移矩阵条目
#[derive(Debug, Clone)]
pub struct TransitionEntry {
    pub edge_id: Uuid,
    pub target_node_id: Uuid,
    pub probability: f64,
    pub reward: f64,
    pub observer_weight: f64,
}

/// 叙事 MDP 转移矩阵
///
/// 行索引 = 源节点 ID, 列 = 转移到的目标节点集合
/// 每行的概率之和必须归一化为 1.0
#[derive(Debug, Clone)]
pub struct TransitionMatrix {
    /// 转移矩阵: source_node_id → [TransitionEntry]
    pub matrix: HashMap<Uuid, Vec<TransitionEntry>>,
    /// MDP 配置参数
    pub config: MdpConfig,
}

impl TransitionMatrix {
    pub fn new(config: MdpConfig) -> Self {
        Self {
            matrix: HashMap::new(),
            config,
        }
    }

    /// 添加转移条目
    pub fn add_transition(&mut self, source: Uuid, entry: TransitionEntry) {
        self.matrix.entry(source).or_default().push(entry);
    }

    /// 获取从 source 出发的所有转移
    pub fn get_transitions(&self, source: &Uuid) -> Option<&Vec<TransitionEntry>> {
        self.matrix.get(source)
    }

    /// 归一化某一行的概率分布
    pub fn normalize_row(&mut self, source: &Uuid) {
        if let Some(entries) = self.matrix.get_mut(source) {
            let sum: f64 = entries.iter().map(|e| e.probability).sum();
            if sum > 0.0 {
                for entry in entries.iter_mut() {
                    entry.probability /= sum;
                }
            }
        }
    }

    /// 归一化所有行
    pub fn normalize_all(&mut self) {
        let sources: Vec<Uuid> = self.matrix.keys().copied().collect();
        for source in sources {
            self.normalize_row(&source);
        }
    }

    /// 应用观察者坍缩 (修改概率分布)
    /// P'(s'|s,a) = softmax(log(P(s'|s,a)) + α·O(s') / τ)
    pub fn apply_observer_collapse(
        &mut self,
        source: &Uuid,
        observer_weights: &HashMap<Uuid, f64>,
    ) {
        let alpha = self.config.observer_alpha;
        let tau = self.config.temperature;

        if let Some(entries) = self.matrix.get_mut(source) {
            // 计算 logits
            let logits: Vec<f64> = entries
                .iter()
                .map(|e| {
                    let log_p = if e.probability > 0.0 {
                        e.probability.ln()
                    } else {
                        -1e10
                    };
                    let observer = observer_weights.get(&e.target_node_id).copied().unwrap_or(0.0);
                    (log_p + alpha * observer) / tau
                })
                .collect();

            // 数值稳定的 softmax
            let max_logit = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let exp_sum: f64 = logits.iter().map(|l| (l - max_logit).exp()).sum();

            for (i, entry) in entries.iter_mut().enumerate() {
                entry.observer_weight = observer_weights
                    .get(&entry.target_node_id)
                    .copied()
                    .unwrap_or(0.0);
                entry.probability = (logits[i] - max_logit).exp() / exp_sum;
            }
        }
    }

    /// 获取矩阵大小 (状态数)
    pub fn state_count(&self) -> usize {
        self.matrix.len()
    }
}