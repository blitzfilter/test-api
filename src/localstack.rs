use aws_config::{BehaviorVersion, SdkConfig};
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::config::Credentials;
use std::collections::HashMap;
use std::process::Command;
use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::localstack::LocalStack;
use tokio::sync::OnceCell;

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

static CONFIG: OnceCell<SdkConfig> = OnceCell::const_new();
/// Lazily initializes and returns a shared AWS-Config.
pub async fn get_aws_config() -> &'static SdkConfig {
    CONFIG
        .get_or_init(|| async {
            aws_config::defaults(BehaviorVersion::latest())
                .credentials_provider(Credentials::for_tests())
                .region("eu-central-1")
                .endpoint_url("http://localhost:4566")
                .load()
                .await
        })
        .await
}

static DYNAMODB_CLIENT: OnceCell<Client> = OnceCell::const_new();
/// Lazily initializes and returns a shared DynamoDB client.
pub async fn get_dynamodb_client() -> &'static Client {
    DYNAMODB_CLIENT
        .get_or_init(|| async { Client::new(get_aws_config().await) })
        .await
}

static SQS_CLIENT: OnceCell<aws_sdk_sqs::Client> = OnceCell::const_new();
/// Lazily initializes and returns a shared SQS client.
pub async fn get_sqs_client() -> &'static aws_sdk_sqs::Client {
    SQS_CLIENT
        .get_or_init(|| async {
            let config = get_aws_config().await;
            aws_sdk_sqs::Client::new(&config)
        })
        .await
}

static LAMBDA_CLIENT: OnceCell<aws_sdk_lambda::Client> = OnceCell::const_new();
/// Lazily initializes and returns a shared Lambda client.
pub async fn get_lambda_client() -> &'static aws_sdk_lambda::Client {
    LAMBDA_CLIENT
        .get_or_init(|| async {
            let config = get_aws_config().await;
            aws_sdk_lambda::Client::new(&config)
        })
        .await
}
