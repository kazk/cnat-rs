// https://github.com/GREsau/schemars/pull/65
#![allow(clippy::field_reassign_with_default)]
// Defines CRD At.
use chrono::{DateTime, Utc};
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
