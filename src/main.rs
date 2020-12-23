use argh::FromArgs;
use kube::Client;
use tracing::{error, info};

mod controller;
mod crd;
mod error;

use crd::At;
use error::{Error, Result};

#[derive(FromArgs)]
/// Cloud native `at` command.
struct Options {
    /// output CustomResourceDefinition to stdout and exit
    #[argh(switch, short = 'g')]
    gen_crd: bool,
    /// automatically install the current CRD if missing, assuming permission to modify CRDs
    #[argh(switch, short = 'a')]
    auto_install: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let opts: Options = argh::from_env();
    if opts.gen_crd {
        println!("{}", serde_yaml::to_string(&At::crd()).unwrap());
        return Ok(());
    }

    let client = Client::try_default().await?;
    if !crd::exists(client.clone()).await {
        if !opts.auto_install {
            error!("CRD is not installed. Generate and apply before running or use auto-install option.");
            return Err(Error::MissingCRD);
        }

        info!("Creating At CRD");
        crd::create(client.clone()).await?;
        crd::wait_for_ready(client.clone(), 15).await?;
        info!("Successfully created At CRD");
    }
    info!("Running controller");
    controller::run(client.clone()).await;
    Ok(())
}
