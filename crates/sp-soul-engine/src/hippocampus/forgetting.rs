//! 艾宾浩斯遗忘曲线
//!
//! R(t) = e^(-t/S)
//! R = 记忆强度 (Retrievability)
//! t = 距上次访问的时间间隔 (小时)
//! S = 稳定性 (Stability)

use chrono::{DateTime, Utc, Duration};

/// 计算当前记忆强度
/// R(t) = e^(-t/S)
pub fn compute_strength(stability: f64, last_accessed: DateTime<Utc>) -> f64 {
    let t_hours = (Utc::now() - last_accessed).num_seconds() as f64 / 3600.0;
    if stability <= 0.0 {
        return 0.0;
    }
    (-t_hours / stability).exp()
}

/// 成功回忆后更新稳定性
/// S' = S * (1 + k / (1 + e^(-n)))
/// n = 复述次数, k = 学习速率常数
pub fn update_stability_after_success(current_stability: f64, rehearsal_count: u32) -> f64 {
    let k = 1.5;
    let n = rehearsal_count as f64;
    current_stability * (1.0 + k / (1.0 + (-n).exp()))
}

/// 失败回忆后衰减稳定性
pub fn update_stability_after_failure(current_stability: f64) -> f64 {
    current_stability * 0.5
}

/// 判断记忆是否已被遗忘 (强度低于阈值)
pub fn is_forgotten(stability: f64, last_accessed: DateTime<Utc>, threshold: f64) -> bool {
    compute_strength(stability, last_accessed) < threshold
}