// https://github.com/tokio-rs/axum/blob/main/examples/anyhow-error-response/src/main.rs
// Tradeoffs? Reveals ur code potentially

use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;

// Make our own error that wraps `anyhow::Error`.
#[derive(Debug)]
pub struct AppError(pub anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("Error returned: {}", self.0);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
