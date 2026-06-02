//! MDP 矩陣與觀察者坍縮（橋接 DB 與 narrative-engine）

use std::collections::HashMap;

use sp_common::error::SpError;
use sp_common::types::{MdpConfig, ObserverSignal};
use sp_narrative_engine::mdp::matrix::{TransitionEntry, TransitionMatrix};
use sp_narrative_engine::mdp::observer::ObserverCollapse;
use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;

/// 從 DB 載入轉移矩陣
pub async fn load_transition_matrix(
    pool: &PgPool,
    story_id: Uuid,
) -> Result<TransitionMatrix, SpError> {
    let edges = sqlx::query(
        r#"SELECT id, source_node_id, target_node_id, probability, reward_signal, observer_weight
           FROM narrative_edges WHERE story_id = $1"#
    )
    .bind(story_id)
    .fetch_all(pool)
    .await
    .map_err(SpError::Database)?;

    let mut matrix = TransitionMatrix::new(MdpConfig::default());
    for e in edges {
        let id: Uuid = e.get("id");
        let source_node_id: Uuid = e.get("source_node_id");
        let target_node_id: Uuid = e.get("target_node_id");
        let probability: Option<f64> = e.get("probability");
        let reward_signal: Option<f64> = e.get("reward_signal");
        let observer_weight: Option<f64> = e.get("observer_weight");
        matrix.add_transition(
            source_node_id,
            TransitionEntry {
                edge_id: id,
                target_node_id,
                probability: probability.unwrap_or(0.0),
                reward: reward_signal.unwrap_or(0.0),
                observer_weight: observer_weight.unwrap_or(0.0),
            },
        );
    }
    matrix.normalize_all();
    Ok(matrix)
}

/// 應用觀察者坍縮並寫回 DB
pub async fn apply_collapse(
    pool: &PgPool,
    story_id: Uuid,
    source_node_id: Uuid,
    signal: &ObserverSignal,
) -> Result<Vec<(Uuid, f64)>, SpError> {
    let mut matrix = load_transition_matrix(pool, story_id).await?;
    let collapse = ObserverCollapse::new(matrix.config.observer_alpha);

    let entity_map: HashMap<Uuid, Vec<Uuid>> = if let Some(focused) = signal.focused_entity_id {
        let rows = sqlx::query(
            r#"SELECT id FROM narrative_edges
               WHERE story_id = $1 AND (source_node_id = $2 OR target_node_id = $2)"#
        )
        .bind(story_id)
        .bind(focused)
        .fetch_all(pool)
        .await
        .map_err(SpError::Database)?;
        let edge_ids: Vec<Uuid> = rows.iter().map(|r| r.get("id")).collect();
        let mut map = HashMap::new();
        map.insert(focused, edge_ids);
        map
    } else {
        HashMap::new()
    };

    let edge_weights = collapse.compute_weights(signal, &entity_map);

    let mut observer_by_target = HashMap::new();
    if let Some(entries) = matrix.matrix.get(&source_node_id) {
        for entry in entries {
            if let Some(w) = edge_weights.get(&entry.edge_id) {
                observer_by_target.insert(entry.target_node_id, *w);
            }
        }
    }

    matrix.apply_observer_collapse(&source_node_id, &observer_by_target);

    let mut updates = Vec::new();
    if let Some(entries) = matrix.matrix.get(&source_node_id) {
        for entry in entries {
            sqlx::query(
                r#"UPDATE narrative_edges
                   SET probability = $2, observer_weight = $3, collapse_count = collapse_count + 1
                   WHERE id = $1"#
            )
            .bind(entry.edge_id)
            .bind(entry.probability)
            .bind(entry.observer_weight)
            .execute(pool)
            .await
            .map_err(SpError::Database)?;
            updates.push((entry.edge_id, entry.probability));
        }
    }

    Ok(updates)
}
