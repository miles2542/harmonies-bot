use std::{
    fs,
    net::SocketAddr,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

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
use harmonies_core::{
    advise, advise_with_progress_and_cancel, AdvisorRequestV1, CardCatalog, EvalWeights,
};
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    catalog: CardCatalog,
    weights: EvalWeights,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ServiceConfig::from_env();
    let catalog = load_catalog(&config.catalog_path)?;
    let weights = load_weights(&config.weights_path).unwrap_or_default();
    let state = Arc::new(AppState { catalog, weights });
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
    request.weights = state.weights.clone();
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
        if stream_advice(&mut socket, text, state.clone())
            .await
            .is_err()
        {
            return;
        }
    }
}

async fn stream_advice(
    socket: &mut WebSocket,
    text: String,
    state: Arc<AppState>,
) -> Result<(), axum::Error> {
    let Ok(mut request) = serde_json::from_str::<AdvisorRequestV1>(&text) else {
        return socket
            .send(Message::Text(advisor_event(
                true,
                serde_json::json!({
                    "status": "invalidSnapshot",
                    "elapsedMs": 0,
                    "bestMoves": [],
                    "progress": {
                        "depthCompleted": 0,
                        "nodesEvaluated": 0,
                        "stoppedEarly": false
                    },
                    "warnings": ["request parse error"]
                }),
            )))
            .await;
    };
    request.catalog = state.catalog.clone();
    request.weights = state.weights.clone();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let progress_tx = tx.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    tokio::task::spawn_blocking(move || {
        let final_response = advise_with_progress_and_cancel(
            request,
            |response| {
                let _ = progress_tx.send(advisor_event(false, response));
            },
            || worker_cancel.load(Ordering::Relaxed),
        );
        let _ = tx.send(advisor_event(true, final_response));
    });

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) if is_stop_command(&text) => {
                        cancel.store(true, Ordering::Relaxed);
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        cancel.store(true, Ordering::Relaxed);
                        return Ok(());
                    }
                    Some(Ok(_)) => {}
                    Some(Err(error)) => return Err(error),
                }
            }
            payload = rx.recv() => {
                let Some(payload) = payload else {
                    return Ok(());
                };
                socket.send(Message::Text(payload)).await?;
            }
        }
    }
}

fn advisor_event(final_event: bool, response: impl serde::Serialize) -> String {
    serde_json::to_string(&serde_json::json!({
        "event": "advisorResponse",
        "final": final_event,
        "response": response
    }))
    .unwrap_or_else(|_| String::from("{\"event\":\"advisorResponse\",\"final\":true}"))
}

fn is_stop_command(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|value| {
            value
                .get("command")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .map(|command| command == "stop")
        .unwrap_or(false)
}

fn load_catalog(path: &PathBuf) -> anyhow::Result<CardCatalog> {
    let input =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    CardCatalog::from_cards_database_json(&input).context("failed to parse card catalog")
}

fn load_weights(path: &PathBuf) -> anyhow::Result<EvalWeights> {
    let input =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&input).context("failed to parse weights")
}

#[derive(Clone, Debug)]
struct ServiceConfig {
    addr: SocketAddr,
    catalog_path: PathBuf,
    weights_path: PathBuf,
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
        let weights_path = std::env::var("HARMONIES_WEIGHTS")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("docs/weights.baseline.json"));
        Self {
            addr,
            catalog_path,
            weights_path,
        }
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
