// Defines CRD At.
// Heavily based on https://github.com/h2oai/h2o-kubernetes/blob/c41f261fe8d323fa217ef9390c8adb9b8e89b9fd/deployment/src/crd.rs
use chrono::{DateTime, Utc};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{ListParams, PostParams, WatchEvent},
    Api, Client,
};
use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

const NAME: &str = "ats.example.kazk.dev";

/// Spec for custom resource At.
#[derive(CustomResource, Deserialize, Serialize, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
    group = "example.kazk.dev",
    version = "v1alpha1",
    kind = "At",
    status = "AtStatus",
    namespaced
)]
pub struct AtSpec {
    // The date and time to execute the command in UTC.
    // See https://www.utctime.net
    pub schedule: DateTime<Utc>,
    // Command to run.
    pub command: Vec<String>,
}

/// Status for custom resource At.
#[derive(Deserialize, Serialize, Debug, Clone, JsonSchema)]
pub struct AtStatus {
    // The phase is set to Running when it's time to execute the command.
    // When the command finishes, it's set to Done.
    pub phase: AtPhase,
}

/// Describes the status of the scheduled command.
#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, Copy, JsonSchema)]
pub enum AtPhase {
    /// The command is currently running.
    Running,
    /// The command was executed.
    Done,
}

/// Create At CRD.
pub async fn create(client: Client) -> Result<CustomResourceDefinition> {
    let api = Api::<CustomResourceDefinition>::all(client);
    Ok(api.create(&PostParams::default(), &At::crd()).await?)
}

/// Check if At CRD is installed.
pub async fn exists(client: Client) -> bool {
    get_current(client).await.is_ok()
}

/// Get the installed At CRD.
pub async fn get_current(client: Client) -> Result<CustomResourceDefinition> {
    let api = Api::<CustomResourceDefinition>::all(client);
    Ok(api.get(NAME).await?)
}

/// Wait until the NamesAccepted condition is True.
pub async fn wait_for_ready(client: Client, timeout_secs: u32) -> Result<CustomResourceDefinition> {
    let lp = ListParams::default()
        .fields(&format!("metadata.name={}", NAME))
        .timeout(timeout_secs);
    let api = Api::<CustomResourceDefinition>::all(client);
    let mut stream = api.watch(&lp, "0").await?.boxed_local();

    while let Some(status) = stream.try_next().await? {
        if let WatchEvent::Modified(crd) = status {
            let accepted = crd
                .status
                .as_ref()
                .and_then(|s| s.conditions.as_ref())
                .map(|cs| {
                    cs.iter()
                        .any(|c| c.type_ == "NamesAccepted" && c.status == "True")
                })
                .unwrap_or(false);
            if accepted {
                return Ok(crd);
            }
        }
    }

    Err(Error::TimedOut(format!(
        "CR not ready after {} seconds",
        timeout_secs
    )))
}
