use axum::{
    http::{header::CONTENT_TYPE, Method, HeaderValue},
    routing::{get, post},
    Router,
};
use reqwest::Client;
use routes::GameStates;
use std::{env, net::SocketAddr, sync::Arc, collections::HashMap};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt, Layer};

use crate::routes::{get_history, get_root, post_chat, post_game};

mod app_error;
mod gpt;
mod prompt;
mod routes;

pub struct Context {
    open_ai_key: String,
    client: Client,
    game_state: GameStates,
}

const OPEN_AI_KEY_CONFIG: &str = "OPENAI_API_KEY";
const ENVIRONMENT_CONFIG: &str = "ENV";

#[tokio::main]
async fn main() {
    // initialize tracing
    let stdout_log = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(stdout_log.with_filter(filter::filter_fn(|metadata| {
            // only log events from this crate
            metadata.target().starts_with("plakait")
        })))
        .init();

    let _ = dotenvy::dotenv();

    let open_ai_key = env::var(OPEN_AI_KEY_CONFIG).expect("No OPENAI_API_KEY env var found");
    let environment =
        env::var(ENVIRONMENT_CONFIG).expect("No ENV=prod|dev environment variable found");

    if (environment != "prod") && (environment != "dev") {
        panic!("ENV must be either prod or dev");
    }

    let context = Arc::new(Context {
        open_ai_key,
        client: reqwest::Client::new(),
        game_state: RwLock::new(HashMap::new()),
    });

    let origins: Vec<HeaderValue> = match environment.as_str() {
        "prod" => vec![
            "https://plakait.com".parse().unwrap(),
            "https://plakait.stevenhuyn.com".parse().unwrap(),
        ],
        "dev" => vec!["https://localhost:5173".parse().unwrap()],
        _ => unreachable!(),
    };

    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE]);

    let host = match environment.as_str() {
        "prod" => [127, 0, 0, 1],
        "dev" => [127, 0, 0, 1],
        _ => unreachable!(),
    };

    let port_string = env::var("PORT").unwrap_or_else(|_| String::from("8000"));
    let port = port_string.parse::<u16>().unwrap_or(8000);
    let addr = SocketAddr::from((host, port));

    let app = app(&context).layer(cors);
    tracing::debug!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Having a function that produces our app makes it easy to call it from tests
/// without having to create an HTTP server.
pub fn app(context: &Arc<Context>) -> Router {
    Router::new()
        .route("/", get(get_root).with_state(context.clone()))
        .route("/history/:id", get(get_history).with_state(context.clone()))
        .route("/game", post(post_game).with_state(context.clone()))
        .route("/chat/:id", post(post_chat).with_state(context.clone()))
}
