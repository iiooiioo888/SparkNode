//! 海马体记忆网络 (Hippocampus)
//!
//! NPC 的持久化向量记忆系统。
//! 基于 Ebbinghaus 遗忘曲线管理记忆强度，
//! 支持语义检索、记忆巩固、情感权重注入。

pub mod memory;
pub mod qdrant;
pub mod forgetting;
pub mod consolidation;
pub mod emotion_weight;

pub use memory::*;
pub use forgetting::*;
pub use consolidation::*;
pub use emotion_weight::*;