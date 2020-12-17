use kube::Client;
use simple_logger::SimpleLogger;

mod controller;
mod crd;
mod error;

use crate::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::from_env().init().unwrap();

    let client = Client::try_default().await?;
    if !crd::exists(client.clone()).await {
        log::info!("Creating At CRD");
        crd::create(client.clone()).await?;
        crd::wait_for_ready(client.clone(), 5).await?;
        log::info!("Successfully created At CRD");
    }
    log::info!("Running controller");
    controller::run(client.clone()).await;
    Ok(())
}
