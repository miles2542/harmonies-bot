use std::{fs, net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::Context;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use harmonies_core::{advise, AdvisorRequestV1, CardCatalog};
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    catalog: CardCatalog,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ServiceConfig::from_env();
    let catalog = load_catalog(&config.catalog_path)?;
    let state = Arc::new(AppState { catalog });
    let app = Router::new()
        .route("/health", get(health))
        .route("/advise", post(advise_http))
        .route("/ws", get(advise_ws))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.addr)
        .await
        .with_context(|| format!("failed to bind {}", config.addr))?;
    eprintln!("harmonies-service listening on {}", config.addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("service stopped with error")?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn advise_http(
    State(state): State<Arc<AppState>>,
    Json(mut request): Json<AdvisorRequestV1>,
) -> Json<harmonies_core::AdvisorResponseV1> {
    request.catalog = state.catalog.clone();
    Json(advise(request))
}

async fn advise_ws(
    State(state): State<Arc<AppState>>,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    upgrade.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    while let Some(Ok(message)) = socket.recv().await {
        let Message::Text(text) = message else {
            continue;
        };
        let response = match serde_json::from_str::<AdvisorRequestV1>(&text) {
            Ok(mut request) => {
                request.catalog = state.catalog.clone();
                serde_json::to_string(&advise(request))
            }
            Err(error) => serde_json::to_string(&serde_json::json!({
                "status": "invalidSnapshot",
                "elapsedMs": 0,
                "bestMoves": [],
                "progress": {
                    "depthCompleted": 0,
                    "nodesEvaluated": 0,
                    "stoppedEarly": false
                },
                "warnings": [format!("request parse error: {error}")]
            })),
        };
        let Ok(payload) = response else {
            let _ = socket
                .send(Message::Text(String::from(
                    "{\"status\":\"invalidSnapshot\"}",
                )))
                .await;
            continue;
        };
        if socket.send(Message::Text(payload)).await.is_err() {
            return;
        }
    }
}

fn load_catalog(path: &PathBuf) -> anyhow::Result<CardCatalog> {
    let input =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    CardCatalog::from_cards_database_json(&input).context("failed to parse card catalog")
}

#[derive(Clone, Debug)]
struct ServiceConfig {
    addr: SocketAddr,
    catalog_path: PathBuf,
}

impl ServiceConfig {
    fn from_env() -> Self {
        let host =
            std::env::var("HARMONIES_SERVICE_HOST").unwrap_or_else(|_| String::from("127.0.0.1"));
        let port = std::env::var("HARMONIES_SERVICE_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(17848);
        let addr = format!("{host}:{port}")
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 17848)));
        let catalog_path = std::env::var("HARMONIES_CATALOG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("docs/cards_database.json"));
        Self { addr, catalog_path }
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
