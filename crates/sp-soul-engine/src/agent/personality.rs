//! 人格特质向量 (OCEAN 模型)
//!
//! 影响 NPC 的行为偏好与决策风格。

/// 人格对行为的影响权重
pub fn personality_bias(openness: f64, conscientiousness: f64, extraversion: f64, agreeableness: f64, neuroticism: f64) -> f64 {
    // 综合人格指数: 影响 NPC 的冒险倾向
    openness * 0.3 + (1.0 - conscientiousness) * 0.2 + extraversion * 0.2 + (1.0 - agreeableness) * 0.1 + neuroticism * 0.2
}