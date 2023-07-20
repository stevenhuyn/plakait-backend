use std::{collections::HashMap, sync::Arc};

use anyhow::anyhow;
use async_openai::types::{ChatCompletionRequestMessage, Role};
use serde::Serialize;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::{
    app_error::AppError,
    gpt::gpt_chat_retry,
    prompt::{Scenario, PROMPT_DATA},
    routes::chat::{JsonAiResponse, RETRY_COUNT},
    Context,
};

mod chat;
mod game;
mod history;
mod root;

pub type GameStates = RwLock<HashMap<Uuid, Arc<Mutex<GameState>>>>;

pub use chat::post_chat;
pub use game::post_game;
pub use history::get_history;
pub use root::get_root;

#[derive(Serialize, Debug)]
pub struct GameState {
    messages: Vec<Message>,
    scenario: Scenario,
}

#[derive(Clone, Serialize, Debug)]
#[serde(tag = "type")] // https://serde.rs/enum-representations.html#internally-tagged
pub enum Message {
    User {
        name: Option<String>,
        content: String,
    },
    Bot {
        name: String,
        expression: Option<String>,
        content: Option<String>,
        #[serde(rename = "endMessage")]
        end_message: Option<String>,
    },
}

impl From<Message> for ChatCompletionRequestMessage {
    fn from(message: Message) -> Self {
        match message {
            Message::User { name, content } => ChatCompletionRequestMessage {
                content: Some(format!(
                    "{}: {}",
                    name.unwrap_or_else(|| "Admin".to_string()),
                    content
                )),
                role: Role::User,
                function_call: None,
                name: None,
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

                ChatCompletionRequestMessage {
                    content: Some(json_content),
                    role: Role::Assistant,
                    function_call: None,
                    name: None,
                }
            }
        }
    }
}

/// Send a user message to ChatGPT and add both user and bot messages to the game state
pub async fn send_user_message(
    context: &Context,
    game_state: &mut GameState,
    user_message: Message,
) -> Result<Vec<Message>, AppError> {
    let messages = &mut game_state.messages;
    match user_message {
        user_message @ Message::User { .. } => {
            messages.push(user_message);
        }
        _ => return Err(anyhow!("Invalid message type").into()),
    }

    let bot_name = PROMPT_DATA
        .get(&game_state.scenario)
        .unwrap()
        .bot_name
        .to_owned();

    let request_messages: Vec<ChatCompletionRequestMessage> = messages
        .iter()
        .map(|message: &Message| message.to_owned().into())
        .collect();

    let json = serde_json::json!({
        "model": "gpt-4",
        "messages": request_messages
    })
    .to_string();

    let json_ai_response: JsonAiResponse =
        match gpt_chat_retry(&context.client, &context.open_ai_key, &json, RETRY_COUNT).await {
            Ok(response) => response,
            Err(_err) => return Err(anyhow!("Failed to get valid response from OpenAI").into()),
        };

    let dialogue = json_ai_response.dialogue;

    tracing::debug!("success - msg: {:?}", dialogue);
    messages.push(Message::Bot {
        name: bot_name.to_string(),
        expression: json_ai_response.expression,
        content: dialogue,
        end_message: json_ai_response.end_message,
    });

    Ok(messages[1..].to_vec())
}
