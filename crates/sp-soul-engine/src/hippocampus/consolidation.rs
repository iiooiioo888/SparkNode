//! 记忆巩固 (短期→长期)

/// 记忆巩固过程
/// 将高频访问的短期记忆转化为长期记忆，
/// 提升其稳定性 S 值。
pub fn consolidate(stability: f64, access_frequency: f64, emotional_intensity: f64) -> f64 {
    // 巩固因子: 频率越高、情感越强，巩固效果越好
    let consolidation_factor = 1.0 + access_frequency * 0.3 + emotional_intensity * 0.5;
    stability * consolidation_factor
}