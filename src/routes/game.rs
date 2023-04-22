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

#[derive(Serialize, Deserialize, Debug)]
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

#[cfg(test)]
mod tests {
    use crate::*;

    use super::*;
    use axum::{
        body::Body,
        extract::connect_info::MockConnectInfo,
        http::{self, Request, StatusCode},
    };
    use openai::chat::ChatCompletion;
    use serde_json::{json, Value};
    use std::{borrow::BorrowMut, net::SocketAddr};
    use tokio::net::TcpListener;
    use tower::Service; // for `call`
    use tower::ServiceExt; // for `oneshot` and `ready`

    #[tokio::test]
    async fn post_game() {
        // Request a new server from the pool
        let mut server = mockito::Server::new();

        // Use one of these addresses to configure your client
        let url = server.url();

        println!("DA URL - {}", url);

        let context = Arc::new(Context {
            completion_url: url,
            open_ai_key: "OpenAI Key".to_string(),
            client: reqwest::Client::new(),
            game_state: RwLock::new(HashMap::new()),
        });

        let app = app(&context);

        let body = r#"
        {
            "id": "chatcmpl-78EC9PW9vg75GvTcdNqYh1O0Gg6G4",
            "object": "chat.completion",
            "created": 1682195545,
            "model": "gpt-3.5-turbo-0301",
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 10,
                "total_tokens": 20
            },
            "choices": [
                {
                    "message": {
                    "role":"assistant",
                    "content": "{\n\"name\": \"Pamela\",\n\"expression\": \"\\ud83d\\udc48\",\n\"dialogue\": \"Hi Jane, I hope you're not here to take Jack away. He's having so much fun with his grandma.\",\n\"endMessage\": null\n}"},
                    "finish_reason": "stop",
                    "index":0
                }
            ]
        }"#;

        // Create a mock
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create();

        let request = serde_json::json!(PostRoomRequest {
            scenario: Scenario::BadMil,
        })
        .to_string();

        let response = app
            // .borrow_mut()
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/game")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(request))
                    .unwrap(),
            )
            .await
            .unwrap();

        // mock.assert();

        assert_eq!(response.status(), StatusCode::OK);

        let body = String::from_utf8(
            hyper::body::to_bytes(response.into_body())
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();

        let expected = "{\"gameId\":\"d3c15659-e25c-4216-9adb-5210c28b0ffe\",\"messages\":[{\"type\":\"Bot\",\"name\":\"Pamela\",\"expression\":\"👈\",\"content\":\"Hi Jane, I hope you're not here to take Jack away. He's having so much fun with his grandma.\",\"endMessage\":null}]}";
        assert_eq!(body, "bruh");
    }
}
