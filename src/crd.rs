// Defines CRD At.
// Heavily based on https://github.com/h2oai/h2o-kubernetes/blob/c41f261fe8d323fa217ef9390c8adb9b8e89b9fd/deployment/src/crd.rs
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{ListParams, PostParams, WatchEvent},
    Api, Client,
};
use kube_derive::CustomResource;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

const NAME: &str = "ats.example.kazk.dev";

/// Spec for custom resource At.
#[derive(CustomResource, Deserialize, Serialize, Debug, PartialEq, Clone)]
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
    pub schedule: String,
    // Command to run.
    pub command: Vec<String>,
}

/// Status for custom resource At.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AtStatus {
    // The phase is set to Running when it's time to execute the command.
    // When the command finishes, it's set to Done.
    pub phase: AtPhase,
}

/// Describes the status of the scheduled command.
#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, Copy)]
pub enum AtPhase {
    /// The command is currently running.
    Running,
    /// The command was executed.
    Done,
}

// Instead of `At::crd()`, we build from a string template for now as
// a workaround for kube not being able to generate "schema" required for `v1`.
// https://github.com/clux/kube-rs/issues/264
// https://github.com/clux/kube-rs/issues/264#issuecomment-716481152
/// Get CustomResourceDefinition for At.
pub fn crd() -> Result<CustomResourceDefinition> {
    Ok(serde_yaml::from_str(AT_CRD)?)
}

/// Create At CRD.
pub async fn create(client: Client) -> Result<CustomResourceDefinition> {
    let api = Api::<CustomResourceDefinition>::all(client);
    Ok(api.create(&PostParams::default(), &crd()?).await?)
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

const AT_CRD: &str = r#"
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: ats.example.kazk.dev
spec:
  group: example.kazk.dev
  names:
    kind: At
    singular: at
    plural: ats
  scope: Namespaced
  versions:
    - name: v1alpha1
      served: true
      storage: true
      schema:
        openAPIV3Schema:
          type: object
          properties:
            spec:
              type: object
              properties:
                schedule:
                  type: string
                command:
                  type: array
                  items:
                    type: string
              required: [schedule, command]
            status:
              type: object
              properties:
                phase:
                  type: string
      subresources:
        status: {}
"#;
