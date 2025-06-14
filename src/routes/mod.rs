use std::{collections::HashMap, sync::Arc};

use anyhow::anyhow;
use async_openai::types::{
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
    ChatCompletionRequestMessage, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent,
};
use serde::Serialize;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::{
    app_error::AppError,
    gpt::gpt_chat,
    prompt::{Scenario, PROMPT_DATA},
    routes::chat::{JsonAiResponse},
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
            Message::User { name, content } => {
                let content = ChatCompletionRequestUserMessageContent::Text(format!(
                    "{}: {}",
                    name.unwrap_or_else(|| "Admin".to_string()),
                    content
                ));
                ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                    content,
                    name: None,
                })
            }
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

                let content = ChatCompletionRequestAssistantMessageContent::Text(json_content);
                ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                    content: Some(content),
                    name: None,
                    tool_calls: None,
                    ..Default::default()
                })
            }
        }
    }
}

/// Send a user message to ChatGPT and add both user and bot messages to the game state
pub async fn send_user_message(
    _context: &Context,
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

    let json_ai_response: JsonAiResponse = match gpt_chat(&request_messages).await {
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
