use chrono::{DateTime, Utc};
use futures::StreamExt;
use k8s_openapi::{
    api::core::v1::{Container, Pod, PodSpec},
    apimachinery::pkg::apis::meta::v1::OwnerReference,
};
use kube::{
    api::{ListParams, Meta, ObjectMeta, Patch, PatchParams, PostParams},
    error::ErrorResponse,
    Api, Client, Error as KubeError,
};
use kube_runtime::controller::{Context, Controller, ReconcilerAction};
use snafu::{ResultExt, Snafu};
use tracing::{debug, trace, warn};

use crate::resource::{At, AtPhase, AtStatus};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Failed to get Pod: {}", source))]
    GetPod { source: kube::Error },
    #[snafu(display("Failed to create Pod: {}", source))]
    CreatePod { source: kube::Error },
    #[snafu(display("Failed to patch status: {}", source))]
    PatchStatus { source: kube::Error },
    #[snafu(display("Missing object key: {}", key))]
    MissingObjectKey { key: &'static str },
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
async fn reconciler(at: At, ctx: Context<ContextData>) -> Result<ReconcilerAction> {
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
                    pods.create(&PostParams::default(), &pod)
                        .await
                        .context(CreatePod)?;
                    Ok(ReconcilerAction {
                        requeue_after: None,
                    })
                }

                // Unexpected errors.
                Err(err) => Err(Error::GetPod { source: err }),
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
    at.metadata.name.as_ref().ok_or(Error::MissingObjectKey {
        key: ".metadata.name",
    })
}

fn get_namespace_ref(at: &At) -> Result<&String> {
    at.metadata
        .namespace
        .as_ref()
        .ok_or(Error::MissingObjectKey {
            key: ".metadata.namespace",
        })
}

#[tracing::instrument(skip(client, at), level = "debug")]
async fn to_next_phase(client: Client, at: &At, phase: AtPhase) -> Result<()> {
    let ats = Api::<At>::namespaced(client, get_namespace_ref(at)?);
    let status = serde_json::json!({
        "status": AtStatus { phase }
    });

    ats.patch_status(
        &get_name_ref(at)?,
        &PatchParams::default(),
        &Patch::Merge(&status),
    )
    .await
    .context(PatchStatus)?;
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
        name: meta.name.ok_or(Error::MissingObjectKey {
            key: ".metadata.name",
        })?,
        uid: meta.uid.ok_or(Error::MissingObjectKey {
            key: ".metadata.uid",
        })?,
        ..OwnerReference::default()
    })
}
