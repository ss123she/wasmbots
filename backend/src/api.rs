use axum::{
    Router,
    body::Bytes,
    extract::{
        DefaultBodyLimit, Path, Query, State, WebSocketUpgrade,
        ws::{self, WebSocket},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

use crate::arena::world::World;

const MAX_WASM_BYTES: usize = 512 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub world: Arc<Mutex<World>>,
    pub tx: broadcast::Sender<String>,
}

#[derive(Deserialize)]
struct UploadQuery {
    name: Option<String>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .route(
            "/api/bots",
            post(upload_bot).layer(DefaultBodyLimit::max(MAX_WASM_BYTES)),
        )
        .route("/api/presets/{kind}", post(spawn_preset))
        .route("/api/reset", post(reset_world))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.tx.subscribe();
    while let Ok(msg) = rx.recv().await {
        if socket.send(ws::Message::Text(msg.into())).await.is_err() {
            break;
        }
    }
}

async fn upload_bot(
    State(state): State<AppState>,
    Query(q): Query<UploadQuery>,
    body: Bytes,
) -> Result<&'static str, (StatusCode, String)> {
    let name = q.name.unwrap_or_else(|| "AnonBot".to_string());
    let mut w = state.world.lock().unwrap();
    w.spawn_bot(&name, &body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid WASM: {e}")))?;
    Ok("Spawned")
}

async fn spawn_preset(
    State(state): State<AppState>,
    Path(kind): Path<String>,
) -> Result<&'static str, (StatusCode, String)> {
    let path = match kind.as_str() {
        "hunter" => "presets/hunter.wasm",
        "sniper" => "presets/sniper.wasm",
        _ => return Err((StatusCode::NOT_FOUND, "Unknown preset".into())),
    };
    let bytes = std::fs::read(path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Missing preset {path}: {e}"),
        )
    })?;

    let mut w = state.world.lock().unwrap();
    w.spawn_bot(&kind, &bytes)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok("Spawned preset")
}

async fn reset_world(State(state): State<AppState>) -> Result<&'static str, (StatusCode, String)> {
    let mut w = state.world.lock().unwrap();
    *w = World::new().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok("Reset")
}
