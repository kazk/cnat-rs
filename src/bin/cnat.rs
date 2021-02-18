use kube::Client;

use cnat::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let client = Client::try_default().await?;
    cnat::run(client).await;
    Ok(())
}
