use axum::{
    Router,
    body::Bytes,
    extract::{
        Path, Query, State, WebSocketUpgrade,
        ws::{self, WebSocket},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

use arena::world::World;

mod arena;
mod wasm;

#[derive(Clone)]
struct AppState {
    world: Arc<Mutex<World>>,
    tx: broadcast::Sender<String>,
}

#[derive(Serialize, Deserialize)]
struct UploadQuery {
    name: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (tx, _) = broadcast::channel::<String>(32);
    let world = Arc::new(Mutex::new(World::new()?));

    let state = AppState {
        world: world.clone(),
        tx: tx.clone(),
    };

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(33));
        loop {
            interval.tick().await;
            let json = {
                let mut w = world.lock().unwrap();
                let snap = w.step();
                serde_json::to_string(&snap).unwrap()
            };
            let _ = tx.send(json);
        }
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/bots", post(upload_bot))
        .route("/api/presets/{kind}", post(spawn_preset))
        .route("/api/reset", post(reset_world))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    println!("Arena running on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
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
