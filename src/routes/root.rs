use std::sync::Arc;

use axum::extract::State;
use axum_macros::debug_handler;

use crate::Context;

#[debug_handler]
pub async fn get_root(State(context): State<Arc<Context>>) -> String {
    let game_states = context.game_state.read().await;

    let game_count = game_states.keys().len().to_string();
    tracing::debug!("Root get gotten - Game count: {}", game_count);
    game_count
}

#[cfg(test)]
mod tests {
    use crate::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn get_root() {
        let context = Arc::new(Context {
            open_ai_key: "OpenAI Key".to_string(),
            client: reqwest::Client::new(),
            game_state: RwLock::new(HashMap::new()),
        });

        let app = app(&context);

        // `Router` implements `tower::Service<Request<Body>>` so we can
        // call it like any tower service, no need to run an HTTP server.
        let response = app
            // .borrow_mut()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(&body[..], b"0");
    }
}
