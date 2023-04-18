use std::sync::Arc;

use anyhow::anyhow;
use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{app_error::AppError, Context};

use super::Message;

pub async fn get_history(
    Path(game_id): Path<Uuid>,
    State(context): State<Arc<Context>>,
) -> Result<Json<Vec<Message>>, AppError> {
    tracing::debug!("Get History: {}", game_id);

    let game_states = context.game_state.read().await;
    let game_state = game_states
        .get(&game_id)
        .ok_or_else(|| anyhow!("Game not found"))?
        .lock()
        .await;

    Ok(Json(game_state.messages[1..].to_vec()))
}
