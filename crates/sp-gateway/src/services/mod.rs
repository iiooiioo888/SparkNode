//! 領域服務層（橋接 narrative-engine 與持久化）

pub mod dag;
pub mod mdp;

pub use dag::*;
pub use mdp::*;
