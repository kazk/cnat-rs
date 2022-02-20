use std::sync::Arc;

use chrono::{DateTime, Utc};
use futures::StreamExt;
use k8s_openapi::{
    api::core::v1::{Container, Pod, PodSpec},
    apimachinery::pkg::apis::meta::v1::OwnerReference,
};
use kube::{
    api::{ListParams, ObjectMeta, Patch, PatchParams, PostParams, ResourceExt},
    runtime::controller::{Context, Controller, ReconcilerAction},
    Api, Client, Resource,
};
use thiserror::Error;
use tracing::{debug, trace, warn};

use crate::resource::{At, AtPhase, AtStatus};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get Pod: {0}")]
    GetPod(#[source] kube::Error),
    #[error("Failed to create Pod: {0}")]
    CreatePod(#[source] kube::Error),
    #[error("Failed to patch status: {0}")]
    PatchStatus(#[source] kube::Error),
    #[error("Missing object key: {0}")]
    MissingObjectKey(&'static str),
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub async fn run(client: Client) {
    let context = Context::new(ContextData {
        client: client.clone(),
    });
    let lp = ListParams::default();
    Controller::<At>::new(Api::all(client.clone()), lp.clone())
        .owns::<Pod>(Api::all(client.clone()), lp)
        .run(reconciler, error_policy, context)
        .filter_map(|x| async move { x.ok() })
        .for_each(|(_, action)| async move {
            trace!("Reconciled: requeue after {:?}", action.requeue_after);
        })
        .await;
}

// Data to store in context
struct ContextData {
    client: Client,
}

/// The reconciler called when `At` or `Pod` change.
#[tracing::instrument(skip(at, ctx), level = "debug")]
async fn reconciler(at: Arc<At>, ctx: Context<ContextData>) -> Result<ReconcilerAction> {
    match at.status.as_ref().map(|s| s.phase) {
        None => {
            debug!("status.phase: none");
            let schedule = at.spec.schedule;
            let now: DateTime<Utc> = Utc::now();
            if schedule <= now {
                // Ready to execute
                to_next_phase(ctx.get_ref().client.clone(), &at, AtPhase::Running).await?;
                Ok(ReconcilerAction {
                    requeue_after: None,
                })
            } else {
                // Not yet
                Ok(ReconcilerAction {
                    requeue_after: Some((schedule - now).to_std().unwrap()),
                })
            }
        }

        Some(AtPhase::Running) => {
            debug!("status.phase: running");
            let client = ctx.get_ref().client.clone();
            let pods = Api::<Pod>::namespaced(client.clone(), get_namespace_ref(&at)?);
            match pods
                .get_opt(get_name_ref(&at)?)
                .await
                .map_err(Error::GetPod)?
            {
                Some(pod) => match pod.status.and_then(|x| x.phase).as_ref() {
                    Some(pod_phase) if pod_phase == "Succeeded" || pod_phase == "Failed" => {
                        to_next_phase(client.clone(), &at, AtPhase::Done).await?;
                        Ok(ReconcilerAction {
                            requeue_after: None,
                        })
                    }

                    _ => Ok(ReconcilerAction {
                        requeue_after: None,
                    }),
                },
                None => {
                    let pod = build_owned_pod(&at)?;
                    pods.create(&PostParams::default(), &pod)
                        .await
                        .map_err(Error::CreatePod)?;
                    Ok(ReconcilerAction {
                        requeue_after: None,
                    })
                }
            }
        }

        Some(AtPhase::Done) => {
            debug!("status.phase: done");
            Ok(ReconcilerAction {
                requeue_after: None,
            })
        }
    }
}

/// An error handler called when the reconciler fails.
fn error_policy(error: &Error, _ctx: Context<ContextData>) -> ReconcilerAction {
    warn!("reconcile failed: {}", error);
    ReconcilerAction {
        requeue_after: None,
    }
}

fn get_name_ref(at: &At) -> Result<&String> {
    at.metadata
        .name
        .as_ref()
        .ok_or(Error::MissingObjectKey(".metadata.name"))
}

fn get_namespace_ref(at: &At) -> Result<&String> {
    at.metadata
        .namespace
        .as_ref()
        .ok_or(Error::MissingObjectKey(".metadata.namespace"))
}

#[tracing::instrument(skip(client, at), level = "debug")]
async fn to_next_phase(client: Client, at: &At, phase: AtPhase) -> Result<()> {
    let ats = Api::<At>::namespaced(client, get_namespace_ref(at)?);
    let status = serde_json::json!({
        "status": AtStatus { phase }
    });

    ats.patch_status(
        get_name_ref(at)?,
        &PatchParams::default(),
        &Patch::Merge(&status),
    )
    .await
    .map_err(Error::PatchStatus)?;
    Ok(())
}

fn build_owned_pod(at: &At) -> Result<Pod> {
    Ok(Pod {
        metadata: ObjectMeta {
            name: at.metadata.name.clone(),
            owner_references: Some(vec![OwnerReference {
                controller: Some(true),
                api_version: At::api_version(&()).into_owned(),
                kind: At::kind(&()).into_owned(),
                name: at.name(),
                uid: at.uid().expect("has uid"),
                ..Default::default()
            }]),
            ..ObjectMeta::default()
        },
        spec: Some(PodSpec {
            containers: vec![Container {
                name: "busybox".into(),
                image: Some("busybox".into()),
                command: Some(at.spec.command.clone()),
                ..Container::default()
            }],
            restart_policy: Some("Never".into()),
            ..PodSpec::default()
        }),
        ..Pod::default()
    })
}
