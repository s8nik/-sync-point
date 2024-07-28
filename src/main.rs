use std::net::SocketAddr;

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    sync_point::logger::init()?;

    let addr: SocketAddr = std::env::var("SYNC_POINT_ADDR")?
        .parse()
        .with_context(|| "invalid sync point addr")?;

    let sync_timeout = std::env::var("SYNC_POINT_TIMEOUT_SEC")
        .ok()
        .and_then(|duration| duration.parse::<u64>().ok())
        .unwrap_or(10);

    sync_point::serve(addr, sync_timeout).await
}
