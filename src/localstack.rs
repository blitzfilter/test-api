use std::collections::HashMap;
use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::localstack::LocalStack;

pub async fn spin_up_localstack(env_vars: HashMap<&str, &str>) -> ContainerAsync<LocalStack> {
    let request = env_vars
        .iter()
        .fold(
            LocalStack::default()
                .with_tag("latest")
                .with_container_name("localstack-test-api"),
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
