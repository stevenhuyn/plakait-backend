use std::sync::Arc;

use ::serde::{Deserialize, Serialize};
use anyhow::anyhow;
use async_openai::types::{ChatCompletionResponseMessage, Role};
use axum::{
    extract::{Path, State},
    Json,
};
use axum_macros::debug_handler;
use uuid::Uuid;

use crate::{
    app_error::AppError,
    routes::{send_user_message, Message},
    Context,
};

pub const RETRY_COUNT: usize = 8;

#[derive(Clone, Deserialize, Debug)]
pub struct PostChatRequest {
    name: String,
    content: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct JsonAiResponse {
    pub name: Option<String>,
    pub dialogue: Option<String>,
    pub expression: Option<String>,
    #[serde(rename = "endMessage")]
    pub end_message: Option<String>,
}

impl From<Message> for ChatCompletionResponseMessage {
    fn from(message: Message) -> Self {
        match message {
            Message::User { name, content } => ChatCompletionResponseMessage {
                content: Some(format!(
                    "{}: {}",
                    name.unwrap_or_else(|| "Admin".to_string()),
                    content
                )),
                role: Role::User,
                function_call: None,
            },
            Message::Bot {
                name,
                expression,
                content,
                end_message,
            } => {
                let json_content = serde_json::json!(JsonAiResponse {
                    name: Some(name),
                    expression,
                    dialogue: content,
                    end_message
                })
                .to_string();

                tracing::debug!("message into request: {}", &json_content);

                ChatCompletionResponseMessage {
                    content: Some(json_content),
                    role: Role::Assistant,
                    function_call: None,
                }
            }
        }
    }
}

#[debug_handler]
pub async fn post_chat(
    Path(game_id): Path<Uuid>,
    State(context): State<Arc<Context>>,
    Json(payload): Json<PostChatRequest>,
) -> Result<Json<Vec<Message>>, AppError> {
    tracing::debug!("cm {} - {}: {}", game_id, payload.name, payload.content);
    let game_states = context.game_state.read().await;
    let mut game_state = game_states
        .get(&game_id)
        .ok_or_else(|| anyhow!("Game not found"))?
        .lock()
        .await;

    let messages = send_user_message(
        &context,
        &mut game_state,
        Message::User {
            name: Some(payload.name.clone()),
            content: payload.content,
        },
    )
    .await;

    match messages {
        Ok(messages) => Ok(Json(messages)),
        Err(e) => Err(e),
    }
}
