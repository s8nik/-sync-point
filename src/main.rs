use std::net::SocketAddr;

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    sync_point::logger::init()?;

    let addr: SocketAddr = std::env::var("SYNC_POINT_ADDR")?
        .parse()
        .with_context(|| "invalid sync point addr")?;

    sync_point::serve(addr).await
}
