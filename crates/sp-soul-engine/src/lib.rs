//! # 灵犀节点 灵魂引擎 (Soul Engine)
//!
//! 从「NPC 对话」到「具备潜意识与梦境的矽基生命」的核心引擎。
//!
//! ## 核心模块
//! - `hippocampus`: 海马体记忆网络 (持久化向量记忆 + 遗忘曲线)
//! - `amygdala`: 杏仁核情感模块 (情感状态机)
//! - `agent`: NPC Agent 核心循环 (行为模拟 + 决策推理)

pub mod hippocampus;
pub mod amygdala;
pub mod agent;