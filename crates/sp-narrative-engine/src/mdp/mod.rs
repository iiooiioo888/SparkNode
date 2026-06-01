//! 马尔可夫决策过程 (MDP) 模块
//!
//! 「量子叙事叠加态」的数学核心。
//! 维护状态转移概率矩阵，支持观察者坍缩、
//! 策略迭代优化、以及情感权重注入。

pub mod matrix;
pub mod policy;
pub mod observer;
pub mod transition;

pub use matrix::*;
pub use policy::*;
pub use observer::*;
pub use transition::*;