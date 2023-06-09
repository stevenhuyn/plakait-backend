use axum::{
    extract::Host,
    handler::HandlerWithoutStateExt,
    http::{StatusCode, Uri},
    response::Redirect,
    routing::{get, post},
    BoxError, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use config::{Config, Value};
use reqwest::{header::CONTENT_TYPE, Client, Method};
use routes::GameStates;
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt, Layer};

use crate::routes::{get_history, get_root, post_chat, post_game};

mod app_error;
mod gpt;
mod prompt;
mod routes;

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

pub struct Context {
    open_ai_key: String,
    client: Client,
    game_state: GameStates,
}

const OPEN_AI_KEY_CONFIG: &str = "OPENAI_SECRET_KEY";
const ENVIRONMENT_CONFIG: &str = "ENVIRONMENT";
const CERT_LOCATION_CONFIG: &str = "CERT_LOCATION";

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

    let mut config = Config::builder()
        .add_source(config::File::with_name("./config.json"))
        .build()
        .unwrap()
        .try_deserialize::<HashMap<String, Value>>()
        .unwrap();

    let environment = config
        .remove(ENVIRONMENT_CONFIG)
        .unwrap_or_else(|| panic!("{} not found in config.json", ENVIRONMENT_CONFIG))
        .to_string();

    let open_ai_key = config
        .remove(OPEN_AI_KEY_CONFIG)
        .unwrap_or_else(|| panic!("{} not found in config.json", OPEN_AI_KEY_CONFIG))
        .to_string();

    let context = Arc::new(Context {
        open_ai_key,
        client: reqwest::Client::new(),
        game_state: RwLock::new(HashMap::new()),
    });

    let ports = Ports {
        http: 7878,
        https: 3000,
    };

    let origins = match environment.as_str() {
        "prod" => [
            "http://plakait.com".parse().unwrap(),
            "https://plakait.com".parse().unwrap(),
        ],
        "dev" => [
            "http://localhost:5173".parse().unwrap(),
            "https://localhost:5173".parse().unwrap(),
        ],
        _ => panic!("config.json envrionment value must be `prod` or `dev`"),
    };

    tracing::debug!("{}", environment);
    tracing::debug!("{:?}", origins);
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE]);

    let app = app(&context).layer(cors);

    if environment.as_str() == "prod" {
        // optional: spawn a second server to redirect http requests to this server
        tokio::spawn(redirect_http_to_https(ports));
        let addr = SocketAddr::from(([0, 0, 0, 0], ports.https));

        let cert_location = config
            .remove(CERT_LOCATION_CONFIG)
            .unwrap_or_else(|| panic!("{} not found in config.json", CERT_LOCATION_CONFIG))
            .to_string();

        // configure certificate and private key used by https
        let tls_config = RustlsConfig::from_pem_file(
            PathBuf::from(&cert_location).join("cert.pem"),
            PathBuf::from(&cert_location).join("privkey.pem"),
        )
        .await
        .unwrap();

        // run https server
        tracing::debug!("listening on {}", addr);
        axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service())
            .await
            .unwrap();
    } else {
        // environment == "dev"
        // run http server
        let addr = SocketAddr::from(([127, 0, 0, 1], ports.http));
        tracing::debug!("listening on {}", addr);
        axum_server::bind(addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
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

async fn redirect_http_to_https(ports: Ports) {
    fn make_https(host: String, uri: Uri, ports: Ports) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace(&ports.http.to_string(), &ports.https.to_string());
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, ports) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], ports.http));
    tracing::debug!("http redirect listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(redirect.into_make_service())
        .await
        .unwrap();
}
