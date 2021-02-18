use kube::Client;
use tracing_subscriber::fmt::format::FmtSpan;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "cnat=trace".to_owned());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let client = Client::try_default().await?;
    cnat::run(client).await;
    Ok(())
}
