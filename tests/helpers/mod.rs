use std::time::Duration;

use k8s_openapi::{
    api::core::v1::ServiceAccount,
    apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition,
};
use kube::{
    runtime::wait::{await_condition, conditions},
    Api, Client, CustomResourceExt, Resource,
};
use tokio::time;

pub mod k3d;

// REVIEW Better to panic as a test helper?
/// Wait until the cluster is actually usable by making sure the default SA exists.
pub async fn cluster_ready(client: Client, timeout: u64) -> Result<(), time::error::Elapsed> {
    time::timeout(Duration::from_secs(timeout), async move {
        tracing::info!("cluster: waiting for readiness");
        let mut interval = time::interval(Duration::from_secs(1));
        let sas: Api<ServiceAccount> = Api::default_namespaced(client);
        loop {
            interval.tick().await;
            if sas.get("default").await.is_ok() {
                break;
            }
        }
        tracing::info!("cluster: ready");
    })
    .await
}

/// Create CRD `K` and wait for `Established` condition.
pub async fn create_crd<K>(client: Client, timeout_secs: u64)
where
    K: Resource<DynamicType = ()> + CustomResourceExt,
{
    tracing::info!("CRD: adding and waiting for Established condition");
    tracing::debug!("CRD: creating");
    let crds = Api::<CustomResourceDefinition>::all(client);
    crds.create(&Default::default(), &<K as CustomResourceExt>::crd())
        .await
        .unwrap();
    tracing::debug!("CRD: created");
    tracing::info!("CRD: waiting for Established condition");
    let name = format!(
        "{}.{}",
        <K as Resource>::plural(&()),
        <K as Resource>::group(&())
    );
    let establish = await_condition(crds, &name, conditions::is_crd_established());
    match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), establish).await {
        Ok(_) => {
            tracing::info!("CRD: condition met");
        }
        Err(_) => {
            panic!(
                "CRD: condition 'Established' not met after {} seconds",
                timeout_secs
            );
        }
    }
}
