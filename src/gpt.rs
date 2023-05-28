use anyhow::{anyhow, Result};
use async_openai::types::CreateChatCompletionResponse;
use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::app_error::AppError;

pub async fn gpt_chat(
    client: &Client,
    open_ai_key: &str,
    body: &str,
) -> Result<CreateChatCompletionResponse, AppError> {
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", open_ai_key))
        .body(body.to_owned())
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    // TODO: tracing error handle?

    let chat_completion: CreateChatCompletionResponse = serde_json::from_str(&response)?;

    Ok(chat_completion)
}

pub async fn gpt_chat_retry<T>(
    client: &Client,
    open_ai_key: &str,
    body: &str,
    retry: usize,
) -> Result<T, AppError>
where
    T: DeserializeOwned + Clone,
{
    for i in 0..retry {
        let chat_completion = gpt_chat(client, open_ai_key, body).await;

        if let Err(err) = chat_completion {
            tracing::error!("failure with OpenAI Chat endpoint: {:?}", err);
            continue;
        };

        let content = &chat_completion.unwrap().choices[0].message.content;

        match serde_json::from_str::<T>(content) {
            Ok(json_content) => return Ok(json_content),
            Err(err) => {
                tracing::debug!("failure {}: {} msg: {}", i, err, content);
                continue;
            }
        };
    }

    Err(anyhow!("Failed to get valid response from OpenAI").into())
}
