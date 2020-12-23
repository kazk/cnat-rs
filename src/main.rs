use argh::FromArgs;
use kube::Client;
use simple_logger::SimpleLogger;

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
    let opts: Options = argh::from_env();
    if opts.gen_crd {
        println!("{}", serde_yaml::to_string(&At::crd()).unwrap());
        return Ok(());
    }

    SimpleLogger::from_env().init().unwrap();

    let client = Client::try_default().await?;
    if !crd::exists(client.clone()).await {
        if !opts.auto_install {
            eprintln!("CRD is not installed. Generate and apply before running or use auto-install option.");
            return Err(Error::MissingCRD);
        }

        log::info!("Creating At CRD");
        crd::create(client.clone()).await?;
        crd::wait_for_ready(client.clone(), 5).await?;
        log::info!("Successfully created At CRD");
    }
    log::info!("Running controller");
    controller::run(client.clone()).await;
    Ok(())
}
