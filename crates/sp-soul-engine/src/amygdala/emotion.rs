//! 杏仁核情感状态机
//!
//! 基于 VAD (Valence-Arousal-Dominance) 模型的情感引擎。
//! 接收外部事件刺激，更新 NPC 的情感状态。

use serde::{Deserialize, Serialize};

/// 情感状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionState {
    pub valence: f64,      // [-1, 1]
    pub arousal: f64,      // [0, 1]
    pub dominance: f64,    // [0, 1]
    pub primary_emotion: String,
    pub intensity: f64,
}

impl Default for EmotionState {
    fn default() -> Self {
        Self {
            valence: 0.0,
            arousal: 0.3,
            dominance: 0.5,
            primary_emotion: "neutral".to_string(),
            intensity: 0.1,
        }
    }
}

/// 杏仁核处理器
pub struct Amygdala {
    pub current_state: EmotionState,
    pub decay_rate: f64, // 情感衰减速率
}

impl Amygdala {
    pub fn new() -> Self {
        Self {
            current_state: EmotionState::default(),
            decay_rate: 0.05,
        }
    }

    /// 外部刺激 → 更新情感状态
    pub fn process_stimulus(&mut self, external_v: f64, external_a: f64) {
        // 加权平均: 保留 70% 当前状态 + 30% 外部刺激
        self.current_state.valence = self.current_state.valence * 0.7 + external_v * 0.3;
        self.current_state.arousal = self.current_state.arousal * 0.7 + external_a * 0.3;
        self.current_state.intensity = (self.current_state.arousal * 0.6
            + self.current_state.valence.abs() * 0.4)
            .min(1.0);
        self.current_state.primary_emotion = self.classify_emotion();
    }

    /// 情感自然衰减 (趋向中性)
    pub fn decay(&mut self) {
        self.current_state.valence *= 1.0 - self.decay_rate;
        self.current_state.arousal = self.current_state.arousal * (1.0 - self.decay_rate) + 0.3 * self.decay_rate;
        self.current_state.intensity *= 1.0 - self.decay_rate;
    }

    /// 基于 VAD 值分类情绪标签
    fn classify_emotion(&self) -> String {
        let v = self.current_state.valence;
        let a = self.current_state.arousal;
        if v > 0.3 && a > 0.5 { "joy".to_string() }
        else if v > 0.3 && a <= 0.5 { "serenity".to_string() }
        else if v < -0.3 && a > 0.5 { "anger".to_string() }
        else if v < -0.3 && a <= 0.5 { "sadness".to_string() }
        else if v.abs() <= 0.3 && a > 0.6 { "surprise".to_string() }
        else if v.abs() <= 0.3 && a < 0.3 { "calm".to_string() }
        else { "neutral".to_string() }
    }
}

impl Default for Amygdala {
    fn default() -> Self {
        Self::new()
    }
}