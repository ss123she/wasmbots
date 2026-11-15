use std::{
    net::{Ipv6Addr, SocketAddr},
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
    eprintln!("Booting arena...");
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
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);

    let v6 = SocketAddr::from((Ipv6Addr::UNSPECIFIED, port));
    let v4 = SocketAddr::from(([0, 0, 0, 0], port));

    let l6 = tokio::net::TcpListener::bind(v6).await.ok();
    let l4 = tokio::net::TcpListener::bind(v4).await.ok();

    match (l6, l4) {
        (Some(l6), Some(l4)) => {
            eprintln!("Arena running on http://{v4} and http://{v6}");
            tokio::try_join!(axum::serve(l6, app.clone()), axum::serve(l4, app))?;
        }
        (Some(l6), None) => {
            eprintln!("Arena running on http://{v6} (dual-stack)");
            axum::serve(l6, app).await?;
        }
        (None, Some(l4)) => {
            eprintln!("Arena running on http://{v4}");
            axum::serve(l4, app).await?;
        }
        (None, None) => anyhow::bail!("Failed to bind port {port}"),
    }

    Ok(())
}
