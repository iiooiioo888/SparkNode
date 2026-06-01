//! 從資料庫載入 DAG 並執行敘事引擎校驗

use sp_common::error::SpError;
use sp_narrative_engine::dag::graph::{DagEdge, DagNode, DirectedAcyclicGraph};
use sqlx::PgPool;
use uuid::Uuid;

/// 從 DB 載入故事 DAG
pub async fn load_story_dag(pool: &PgPool, story_id: Uuid) -> Result<DirectedAcyclicGraph, SpError> {
    let story_exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM stories WHERE id = $1)"#,
        story_id
    )
    .fetch_one(pool)
    .await
    .map_err(SpError::Database)?;

    if !story_exists.unwrap_or(false) {
        return Err(SpError::StoryNotFound(story_id));
    }

    let nodes = sqlx::query!(
        r#"SELECT id, node_type, title FROM story_nodes WHERE story_id = $1"#,
        story_id
    )
    .fetch_all(pool)
    .await
    .map_err(SpError::Database)?;

    let edges = sqlx::query!(
        r#"SELECT id, source_node_id, target_node_id, edge_type, probability, observer_weight
           FROM narrative_edges WHERE story_id = $1"#,
        story_id
    )
    .fetch_all(pool)
    .await
    .map_err(SpError::Database)?;

    let mut dag = DirectedAcyclicGraph::new();
    for n in nodes {
        dag.add_node(DagNode {
            id: n.id,
            node_type: n.node_type,
            title: n.title,
        });
    }

    for e in edges {
        dag.add_edge(DagEdge {
            id: e.id,
            source: e.source_node_id,
            target: e.target_node_id,
            edge_type: e.edge_type,
            probability: e.probability.unwrap_or(0.0) as f64,
            observer_weight: e.observer_weight.unwrap_or(0.0) as f64,
        })?;
    }

    Ok(dag)
}

/// 檢查新增邊是否會形成環
pub async fn validate_new_edge(
    pool: &PgPool,
    story_id: Uuid,
    source: Uuid,
    target: Uuid,
) -> Result<(), SpError> {
    let mut dag = load_story_dag(pool, story_id).await?;

    if !dag.nodes.contains_key(&source) || !dag.nodes.contains_key(&target) {
        return Err(SpError::Internal(
            "邊的源節點或目標節點不存在於此故事".to_string(),
        ));
    }

    if dag.would_create_cycle(source, target) {
        return Err(SpError::DagCycleDetected {
            source_id: source,
            target_id: target,
        });
    }

    Ok(())
}

/// 同一源節點出邊概率歸一化（含新邊）
pub fn normalized_probability_for_source(
    existing_probs: &[f64],
    new_prob: f64,
) -> (Vec<f64>, f64) {
    let mut all: Vec<f64> = existing_probs
        .iter()
        .map(|p| if *p > 0.0 { *p } else { 0.0 })
        .collect();
    all.push(new_prob.max(0.0));
    let sum: f64 = all.iter().sum();
    if sum <= 0.0 {
        let uniform = 1.0 / all.len() as f64;
        return (vec![uniform; existing_probs.len()], uniform);
    }
    let normalized: Vec<f64> = all.iter().map(|p| p / sum).collect();
    let new_normalized = *normalized.last().unwrap_or(&new_prob);
    let old_normalized = normalized[..existing_probs.len()].to_vec();
    (old_normalized, new_normalized)
}
