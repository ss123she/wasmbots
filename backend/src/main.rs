use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::broadcast;

use crate::{api::AppState, arena::world::World};

mod api;
mod arena;
mod wasm;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (tx, _) = broadcast::channel::<String>(32);
    let world = Arc::new(Mutex::new(World::new()?));

    let loop_world = world.clone();
    let loop_tx = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(33));
        loop {
            interval.tick().await;
            let json = {
                let mut w = loop_world.lock().unwrap();
                serde_json::to_string(&w.step()).unwrap()
            };
            let _ = loop_tx.send(json);
        }
    });

    let app = api::router(AppState { world, tx });
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    println!("Arena running on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
