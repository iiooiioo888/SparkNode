//! SparkNode 核心数据类型
//!
//! 包含叙事节点、叙事边、世界状态、NPC 实体、概率分布等
//! 跨引擎共享的底层数据结构。

pub mod story_node;
pub mod edge;
pub mod world_state;
pub mod npc;
pub mod probability;
pub mod validation;

pub use story_node::*;
pub use validation::*;
pub use edge::*;
pub use world_state::*;
pub use npc::{Npc, PersonalityVector, EmotionalState, NpcRelationship, NpcMemory, MemoryType, AutonomyLevel};
pub use probability::*;