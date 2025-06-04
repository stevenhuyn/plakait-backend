use anyhow::Result;
use async_openai::{
    types::{
        ChatCompletionRequestMessage,
        CreateChatCompletionRequestArgs, ResponseFormat,
    },
    Client,
};
use serde::de::DeserializeOwned;

use crate::app_error::AppError;

pub async fn gpt_chat<T>(messages: &[ChatCompletionRequestMessage]) -> Result<T, AppError>
where
    T: DeserializeOwned + Clone,
{
    let client = Client::new();

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u16)
        .model("chatgpt-4o-latest")
        .response_format(ResponseFormat::JsonObject)
        .temperature(1.8)
        .messages(messages)
        .build()
        .unwrap();

    let openai_response = client.chat().create(request).await.unwrap();

    let response_content = openai_response
        .choices
        .first()
        .unwrap()
        .message
        .content
        .clone()
        .unwrap();

    tracing::debug!(response_content);

    let response_content = response_content.replace("```json", "");
    let response_content = response_content.replace("```", "");

    Ok(serde_json::from_str::<T>(&response_content)?)
}

