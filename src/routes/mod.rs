use std::{collections::HashMap, sync::Arc};

use anyhow::anyhow;
use openai::chat::{ChatCompletionMessage, ChatCompletionMessageRole};
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

pub mod chat;
pub mod game;
pub mod history;
pub mod root;

type MessageRole = ChatCompletionMessageRole;
pub type GameStates = RwLock<HashMap<Uuid, Arc<Mutex<GameState>>>>;

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

    let request_messages: Vec<ChatCompletionMessage> = messages
        .iter()
        .map(|message: &Message| message.to_owned().into())
        .collect();

    let json = serde_json::json!({
        "model": "gpt-3.5-turbo",
        "messages": request_messages
    })
    .to_string();

    let json_ai_response: JsonAiResponse = match gpt_chat_retry(
        &context.client,
        &context.completion_url,
        &context.open_ai_key,
        &json,
        RETRY_COUNT,
    )
    .await
    {
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
