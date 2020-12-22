use chrono::{DateTime, Utc};
use futures::StreamExt;
use k8s_openapi::{
    api::core::v1::{Container, Pod, PodSpec},
    apimachinery::pkg::apis::meta::v1::OwnerReference,
};
use kube::{
    api::{ListParams, Meta, ObjectMeta, PatchParams, PostParams},
    error::ErrorResponse,
    Api, Client, Error as KubeError,
};
use kube_runtime::controller::{Context, Controller, ReconcilerAction};

use crate::crd::{At, AtPhase, AtStatus};
use crate::error::{Error, Result};

pub async fn run(client: Client) {
    let ats = Api::<At>::all(client.clone());
    let pods = Api::<Pod>::all(client.clone());
    Controller::new(ats, ListParams::default())
        .owns(pods, ListParams::default())
        .run(
            reconcile,
            error_policy,
            Context::new(ContextData {
                client: client.clone(),
            }),
        )
        .for_each(|res| async move {
            match res {
                Ok(o) => log::debug!("reconciled {:?}", o),
                Err(e) => log::error!("reconcile failed: {}", e),
            }
        })
        .await;
}

// Data to store in context
struct ContextData {
    client: Client,
}

/// The reconciler called when `At` or `Pod` change.
async fn reconcile(at: At, ctx: Context<ContextData>) -> Result<ReconcilerAction> {
    match at.status.as_ref().map(|s| s.phase) {
        None => {
            log::debug!("status.phase: none");
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
            log::debug!("status.phase: running");
            let client = ctx.get_ref().client.clone();
            let pods = Api::<Pod>::namespaced(client.clone(), &get_namespace_ref(&at)?);
            match pods.get(&get_name_ref(&at)?).await {
                // Found pod.
                Ok(pod) => match pod.status.and_then(|x| x.phase).as_ref() {
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

                // Expected error Not Found.
                Err(KubeError::Api(ErrorResponse { code: 404, .. })) => {
                    let pod = build_owned_pod(&at)?;
                    pods.create(&PostParams::default(), &pod).await?;
                    Ok(ReconcilerAction {
                        requeue_after: None,
                    })
                }

                // Unexpected errors.
                Err(err) => Err(Error::KubeError(err)),
            }
        }

        Some(AtPhase::Done) => {
            log::debug!("status.phase: done");
            Ok(ReconcilerAction {
                requeue_after: None,
            })
        }
    }
}

/// An error handler called when the reconciler fails.
fn error_policy(_error: &Error, _ctx: Context<ContextData>) -> ReconcilerAction {
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

async fn to_next_phase(client: Client, at: &At, phase: AtPhase) -> Result<()> {
    let ats = Api::<At>::namespaced(client, get_namespace_ref(at)?);
    let status = serde_json::json!({
        "status": AtStatus { phase }
    });
    ats.patch_status(
        &get_name_ref(at)?,
        &PatchParams::default(),
        serde_json::to_vec(&status)?,
    )
    .await?;
    Ok(())
}

fn build_owned_pod(at: &At) -> Result<Pod> {
    Ok(Pod {
        metadata: ObjectMeta {
            name: at.metadata.name.clone(),
            owner_references: Some(vec![OwnerReference {
                controller: Some(true),
                ..object_to_owner_reference::<At>(at.metadata.clone())?
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

fn object_to_owner_reference<K: Meta>(meta: ObjectMeta) -> Result<OwnerReference> {
    Ok(OwnerReference {
        api_version: K::API_VERSION.to_string(),
        kind: K::KIND.to_string(),
        name: meta.name.ok_or(Error::MissingObjectKey(".metadata.name"))?,
        uid: meta.uid.ok_or(Error::MissingObjectKey(".metadata.uid"))?,
        ..OwnerReference::default()
    })
}
