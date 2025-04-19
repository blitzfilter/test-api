use std::collections::HashMap;
use std::process::Command;
use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::localstack::LocalStack;

const LOCALSTACK_CONTAINER_NAME: &str = "localstack-test-api";

pub async fn spin_up_localstack(env_vars: HashMap<&str, &str>) -> ContainerAsync<LocalStack> {
    cleanup_existing_container(LOCALSTACK_CONTAINER_NAME);
    let request = env_vars
        .iter()
        .fold(
            LocalStack::default()
                .with_tag("latest")
                .with_container_name(LOCALSTACK_CONTAINER_NAME),
            |ls, (k, v)| ls.with_env_var(*k, *v),
        )
        .with_mapped_port(4566, 4566.tcp());

    request
        .start()
        .await
        .map_err(|e| {
            eprintln!("Failed to start LocalStack: {e:?}");
            e
        })
        .unwrap()
}

pub async fn spin_up_localstack_with_services(services: &[&str]) -> ContainerAsync<LocalStack> {
    spin_up_localstack(HashMap::from([("SERVICES", services.join(",").as_str())])).await
}

fn cleanup_existing_container(name: &str) {
    let _ = Command::new("docker").args(["rm", "-f", name]).output();
}
