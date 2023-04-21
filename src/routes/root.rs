use std::sync::Arc;

use axum::extract::State;
use axum_macros::debug_handler;

use crate::Context;

#[debug_handler]
pub async fn get_root(State(context): State<Arc<Context>>) -> String {
    tracing::debug!("get request received!");
    let game_states = context.game_state.read().await;

    game_states.keys().len().to_string()
}
