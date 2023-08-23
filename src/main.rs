use axum::{
    routing::{get, post},
    Router,
};
use reqwest::{header::CONTENT_TYPE, Client, Method};
use routes::{GameState, GameStates};
use std::{
    collections::HashMap,
    env,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt, Layer};
use uuid::Uuid;

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

const OPEN_AI_KEY_CONFIG: &str = "OPENAI_SECRET_KEY";
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

    let open_ai_key = env::var(OPEN_AI_KEY_CONFIG).expect("No OPENAI_SECRET_KEY env var found");
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

    let origins = match environment.as_str() {
        "prod" => ["https://plakait.com".parse().unwrap()],
        "dev" => ["https://localhost:5173".parse().unwrap()],
        _ => unreachable!(),
    };

    tracing::debug!("{}", environment);
    tracing::debug!("{:?}", origins);
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE]);

    let host = match environment.as_str() {
        "prod" => [0, 0, 0, 0],
        "dev" => [127, 0, 0, 1],
        _ => unreachable!(),
    };

    let port_string = env::var("PORT").unwrap_or_else(|_| String::from("8000"));
    let port = port_string.parse::<u16>().unwrap_or(8000);
    let addr = SocketAddr::from((host, port));

    let app = app(&context).layer(cors);
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    tokio::spawn(async move {
        remove_old_gamestates(&context.clone()).await;
    });
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

async fn remove_old_gamestates(context: &Context) {
    let one_day = Duration::from_secs(24 * 3600);
    let game_states = &context.game_state;

    loop {
        tokio::time::sleep(one_day).await;

        let mut ids_to_remove: Vec<&Uuid> = vec![];
        let read_game_states = game_states.read().await;
        for (id, game_state) in read_game_states.iter() {
            let game_state = game_state.lock().await;
            let elapsed_time = SystemTime::now().duration_since(game_state.get_created_at());
            let should_remove = match elapsed_time {
                Ok(time) => time < one_day,
                Err(_) => true, // Keep the game state if there's an error in time comparison
            };

            if should_remove {
                ids_to_remove.push(id);
            }
        }

        let write_game_states = &mut game_states.write().await;
        for id in ids_to_remove {
            write_game_states.remove(id);
        }
    }
}
