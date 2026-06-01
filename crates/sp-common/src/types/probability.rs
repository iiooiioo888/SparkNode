//! 概率分布类型 (MDP 框架)
//!
//! 为「量子叙事叠加态」提供数学基础。
//! 包含马尔可夫决策过程 (MDP) 的核心概率类型，
//! 支持观察者坍缩前后的概率分布变换。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 概率分布
///
/// 表示从一个叙事节点出发，到所有可能后续节点的转移概率分布。
/// 所有概率之和必须归一化为 1.0 (量子叠加态的约束条件)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityDistribution {
    /// 源节点 ID
    pub source_node_id: Uuid,
    /// 转移概率 {edge_id: probability}
    pub transitions: HashMap<Uuid, f64>,
    /// 分布熵 H = -Σ p·log(p)，衡量不确定性
    pub entropy: f64,
    /// 是否已被观察者坍缩
    pub collapsed: bool,
}

impl ProbabilityDistribution {
    /// 创建均匀分布 (最大不确定性)
    pub fn uniform(edge_ids: &[Uuid]) -> Self {
        let n = edge_ids.len() as f64;
        let p = if n > 0.0 { 1.0 / n } else { 0.0 };
        let transitions: HashMap<Uuid, f64> = edge_ids.iter().map(|&id| (id, p)).collect();
        let entropy = if n > 0.0 { n.ln() } else { 0.0 };

        Self {
            source_node_id: Uuid::nil(),
            transitions,
            entropy,
            collapsed: false,
        }
    }

    /// 验证概率分布是否已归一化
    pub fn is_normalized(&self) -> bool {
        let sum: f64 = self.transitions.values().sum();
        (sum - 1.0).abs() < 1e-10
    }

    /// 重新归一化概率分布
    pub fn normalize(&mut self) {
        let sum: f64 = self.transitions.values().sum();
        if sum > 0.0 {
            for p in self.transitions.values_mut() {
                *p /= sum;
            }
        }
        self.entropy = self.compute_entropy();
    }

    /// 计算分布熵 H = -Σ p·log(p)
    pub fn compute_entropy(&self) -> f64 {
        -self
            .transitions
            .values()
            .filter(|&&p| p > 0.0)
            .map(|&p| p * p.ln())
            .sum::<f64>()
    }

    /// 应用 softmax 变换 (用于观察者坍缩)
    /// P'(i) = exp(log(P(i)) + α·O(i)) / Σ exp(...)
    pub fn softmax_with_observer(
        &self,
        observer_weights: &HashMap<Uuid, f64>,
        alpha: f64,
        temperature: f64,
    ) -> Self {
        let mut logits: Vec<(Uuid, f64)> = self
            .transitions
            .iter()
            .map(|(&edge_id, &prob)| {
                let logit = if prob > 0.0 { prob.ln() } else { -1e10 };
                let observer = observer_weights.get(&edge_id).copied().unwrap_or(0.0);
                (edge_id, (logit + alpha * observer) / temperature)
            })
            .collect();

        // 数值稳定的 softmax
        let max_logit = logits.iter().map(|(_, l)| *l).fold(f64::NEG_INFINITY, f64::max);
        let exp_sum: f64 = logits
            .iter()
            .map(|(_, l)| (l - max_logit).exp())
            .sum();

        let transitions: HashMap<Uuid, f64> = logits
            .iter()
            .map(|&(id, l)| (id, (l - max_logit).exp() / exp_sum))
            .collect();

        let mut result = Self {
            source_node_id: self.source_node_id,
            transitions,
            entropy: 0.0,
            collapsed: true,
        };
        result.entropy = result.compute_entropy();
        result
    }

    /// 按概率采样一条边 (轮盘赌算法)
    pub fn sample(&self, random_value: f64) -> Option<Uuid> {
        let mut cumulative = 0.0;
        let mut sorted: Vec<(&Uuid, &f64)> = self.transitions.iter().collect();
        sorted.sort_by_key(|(id, _)| id.to_string()); // 确保确定性

        for (&edge_id, &prob) in &sorted {
            cumulative += prob;
            if random_value <= cumulative {
                return Some(edge_id);
            }
        }
        sorted.last().map(|(&id, _)| id)
    }
}

/// 马尔可夫决策过程 (MDP) 配置
///
/// 控制叙事引擎的概率决策行为。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdpConfig {
    /// 折扣因子 γ ∈ [0, 1]，控制未来奖励的重要性
    pub gamma: f64,
    /// 探索温度 τ，控制概率分布的平坦程度
    pub temperature: f64,
    /// 观察者效应强度 α
    pub observer_alpha: f64,
    /// 情感共鸣系数 β
    pub emotion_beta: f64,
    /// 学习率 (用于策略迭代)
    pub learning_rate: f64,
    /// 最大迭代次数 (策略迭代收敛)
    pub max_iterations: u32,
}

impl Default for MdpConfig {
    fn default() -> Self {
        Self {
            gamma: 0.95,
            temperature: 1.0,
            observer_alpha: 0.3,
            emotion_beta: 0.2,
            learning_rate: 0.01,
            max_iterations: 100,
        }
    }
}

/// 观察者信号 (来自前端情感计算)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverSignal {
    /// 读者 ID
    pub reader_id: Uuid,
    /// 情感效价 [-1, 1]
    pub valence: f64,
    /// 情感唤醒度 [0, 1]
    pub arousal: f64,
    /// 情感支配度 [0, 1]
    pub dominance: f64,
    /// 视线停留时长 (毫秒)
    pub gaze_duration_ms: f64,
    /// 瞳孔放大系数
    pub pupil_dilation: f64,
    /// 视线聚焦的实体 ID
    pub focused_entity_id: Option<Uuid>,
    /// 微表情特征向量
    pub facial_embedding: Vec<f32>,
}

impl ObserverSignal {
    /// 将观察者信号转换为权重向量
    /// 用于注入 MDP 转移矩阵
    pub fn to_weight_vector(&self) -> f64 {
        // 综合情感强度 × 注意力集中度
        let emotion_intensity =
            (self.valence.powi(2) + self.arousal.powi(2) + self.dominance.powi(2)).sqrt();
        let attention_factor = (self.gaze_duration_ms / 2000.0).min(1.0); // 归一化到 [0, 1]
        let pupil_factor = (self.pupil_dilation - 1.0).max(0.0); // 瞳孔放大 > 1.0 为正信号

        emotion_intensity * 0.4 + attention_factor * 0.4 + pupil_factor * 0.2
    }
}