//! 情感权重重构
//!
//! 高情感强度的记忆更容易被回忆，
//! 情感权重影响记忆的检索优先级。

use uuid::Uuid;

/// 情感权重计算
pub fn compute_emotion_boost(valence: f64, arousal: f64) -> f64 {
    // 杏仁核激活度: 高唤醒 + 极端效价 → 更强的记忆编码
    let amygdala_activation = arousal * (1.0 + valence.abs());
    amygdala_activation.min(2.0) // 上限 2.0
}

/// 根据情感权重调整检索分数
pub fn apply_emotion_weight(base_score: f64, emotion_boost: f64) -> f64 {
    base_score * (1.0 + emotion_boost * 0.3)
}