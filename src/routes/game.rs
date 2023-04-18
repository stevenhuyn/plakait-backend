use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    app_error::AppError,
    prompt::{Scenario, PROMPT_DATA},
    routes::{send_user_message, Message},
    Context,
};

use super::GameState;

#[derive(Deserialize, Debug)]
pub struct PostRoomRequest {
    scenario: Scenario,
}

#[derive(Serialize, Debug)]
pub struct PostRoomResponse {
    #[serde(rename = "gameId")]
    game_id: Uuid,
    messages: Vec<Message>,
}

// TODO: Refactor messaging behaviour from post_game and post_chat
pub async fn post_game(
    State(context): State<Arc<Context>>,
    Json(payload): Json<PostRoomRequest>,
) -> Result<Json<PostRoomResponse>, AppError> {
    let prompt_data = PROMPT_DATA.get(&payload.scenario).unwrap();

    let initial_message: Message = Message::User {
        name: None,
        content: prompt_data.prompt.clone(),
    };

    let game_id = Uuid::new_v4();

    let mut game_states = context.game_state.write().await;
    let mut game_state = game_states
        .entry(game_id)
        .or_insert_with(|| {
            Arc::new(Mutex::new(GameState {
                scenario: payload.scenario,
                messages: vec![],
            }))
        })
        .lock()
        .await;

    let messages = send_user_message(&context, &mut game_state, initial_message).await;
    tracing::debug!("Post Game: {:#?} - {}", payload.scenario, game_id);

    match messages {
        Ok(messages) => Ok(Json(PostRoomResponse { game_id, messages })),
        Err(e) => Err(e),
    }
}
