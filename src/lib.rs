#![warn(
    clippy::perf,
    clippy::semicolon_if_nothing_returned,
    clippy::missing_const_for_fn,
    clippy::use_self
)]

mod api;

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::Router;
use once_cell::sync::Lazy;
use tokio::sync::{Mutex, Notify};

type UniqueId = u64;
type SyncState = Arc<Mutex<HashMap<UniqueId, Arc<Notify>>>>;

static SYNC_TIMEOUT: Lazy<u64> = Lazy::new(|| {
    std::env::var("SYNC_POINT_TIMEOUT_SEC")
        .ok()
        .and_then(|duration| duration.parse::<u64>().ok())
        .unwrap_or(10)
});

pub async fn serve(addr: SocketAddr) -> anyhow::Result<()> {
    let state = SyncState::default();

    let app = router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await?;

    Ok(())
}

fn router(state: SyncState) -> Router {
    Router::new()
        .route(
            "/wait-for-second-party/:unique-id",
            axum::routing::post(api::sync_two_parties),
        )
        .with_state(state)
}

pub mod logger {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    pub fn init() -> anyhow::Result<()> {
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(EnvFilter::from_default_env())
            .try_init()?;

        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            tracing::error!("{info}");
            hook(info);
        }));

        Ok(())
    }
}
