use cnat::{At, AtPhase, AtSpec, AtStatus};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{ListParams, WatchEvent},
    Api,
};

mod helpers;
use helpers::k3d::TestEnv;

#[tokio::test]
async fn cnat_controller() {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_owned()))
        .init();

    tracing::info!("creating test env");
    let test_env = TestEnv::builder().servers(1).agents(0).build();
    let client = test_env.client().await;
    helpers::cluster_ready(client.clone(), 30)
        .await
        .expect("cluster becomes ready");
    helpers::create_crd(client.clone(), 45).await;

    {
        tracing::info!("starting controller");
        // note that a panic in spawned task doesn't terminate the process
        let controller_client = client.clone();
        tokio::spawn(async move {
            cnat::run(controller_client).await;
        });
    }

    tracing::info!("testing At controller");
    const NAME: &str = "example";
    let api: Api<At> = Api::default_namespaced(client.clone());
    {
        tracing::info!("creating a new At scheduled to run at 3s from now");
        let schedule = chrono::Utc::now() + chrono::Duration::seconds(3);
        let command = vec!["sh", "-c", "sleep 2 && echo foo"]
            .into_iter()
            .map(Into::into)
            .collect();
        let spec = AtSpec { schedule, command };
        let created = api
            .create(&Default::default(), &At::new(NAME, spec))
            .await
            .unwrap();
        assert_eq!(created.spec.schedule, schedule);
        tracing::info!("[OK] created without error");
        assert_eq!(created.status, None);
        tracing::info!("[OK] status is None");
    }

    {
        tracing::info!("checking that the phase transitions to Running");
        let modified = on_next_modified(api.clone(), NAME, 5)
            .await
            .expect("modified event");
        assert_eq!(
            modified.status,
            Some(AtStatus {
                phase: AtPhase::Running
            })
        );
        tracing::info!("[OK] transitioned to Running");
    }

    {
        tracing::info!("checking that the phase transitions to Done");
        let modified = on_next_modified(api, NAME, 30)
            .await
            .expect("modified event");
        assert_eq!(
            modified.status,
            Some(AtStatus {
                phase: AtPhase::Done
            })
        );
        tracing::info!("[OK] transitioned to Done");
    }

    {
        tracing::info!("checking that a pod was created");
        let pods: Api<Pod> = Api::default_namespaced(client);
        let logs = pods
            .logs(NAME, &Default::default())
            .await
            .expect("pod exists");
        tracing::info!("[OK] pod with the same name exists");
        assert_eq!(logs, "foo\n");
        tracing::info!("[OK] logs matches the expected");
    }
}

async fn on_next_modified(api: Api<At>, name: &str, timeout: u32) -> Option<At> {
    let lp = ListParams::default()
        .fields(&format!("metadata.name={}", name))
        .timeout(timeout);
    let mut stream = api.watch(&lp, "0").await.unwrap().boxed_local();
    while let Some(event) = stream.try_next().await.unwrap() {
        if let WatchEvent::Modified(at) = event {
            return Some(at);
        }
    }
    None
}
