use crate::localstack::{get_lambda_client, get_sqs_client, spin_up_localstack_with_services};
use aws_sdk_lambda::client::Waiters;
use aws_sdk_lambda::types::Runtime;
use aws_sdk_sqs::types::QueueAttributeName::QueueArn;
use std::fs::File;
use std::io::Read;
use std::process::Command;
use std::time::Duration;
use testcontainers::ContainerAsync;
use testcontainers_modules::localstack::LocalStack;
use tokio::sync::OnceCell;

static LOCALSTACK_SQS_LAMBDA_DYNAMODB: OnceCell<ContainerAsync<LocalStack>> = OnceCell::const_new();

/// Lazily initializes and returns a shared Localstack container running:
/// - SQS collecting items to write
/// - Lambda consuming items from the SQS and writing them to
/// - DynamoDB
pub async fn get_localstack_sqs_lambda_dynamodb() -> &'static ContainerAsync<LocalStack> {
    LOCALSTACK_SQS_LAMBDA_DYNAMODB
        .get_or_init(|| async {
            let ls = spin_up_localstack_with_services(&["sqs", "lambda", "dynamodb"]).await;
            init().await;
            ls
        })
        .await
}

pub const LAMBDA_NAME: &str = "item_write_lambda";
const LAMBDA_BOOTSRAP_ZIP_PATH: &str = "/tmp/item_write_lambda_bootstrap.zip";

pub async fn init() {
    let lambda_client = get_lambda_client().await;
    let sqs_client = get_sqs_client().await;
    set_up_queues(sqs_client)
        .await
        .expect(&format!("shouldn't fail creating queue '{QUEUE_NAME}'"));
    set_up_lambda(lambda_client)
        .await
        .expect(&format!("shouldn't fail setting up lambda '{LAMBDA_NAME}'"));
    lambda_client
        .wait_until_function_active_v2()
        .function_name(LAMBDA_NAME)
        .wait(Duration::from_secs(30))
        .await
        .expect("shouldn't fail waiting until lambda is active");
    setup_sqs_lambda_config(sqs_client, lambda_client)
        .await
        .expect("shouldn't fail setting up sqs lambda config");
    crate::dynamodb::init().await;
}

async fn set_up_lambda(client: &aws_sdk_lambda::Client) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("wget")
        .args([
            "--no-check-certificate",
            "https://raw.githubusercontent.com/blitzfilter/item-write-lambda/main/bootstrap.zip",
            "-O",
            LAMBDA_BOOTSRAP_ZIP_PATH,
        ])
        .output()
        .expect(&format!(
            "shouldn't fail downloading bootstrap-zip for '{LAMBDA_NAME}'"
        ));

    let mut file = File::open(LAMBDA_BOOTSRAP_ZIP_PATH).expect(&format!(
        "shouldn't fail opening '{LAMBDA_BOOTSRAP_ZIP_PATH}'"
    ));
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect(&format!(
        "shouldn't fail reading '{LAMBDA_BOOTSRAP_ZIP_PATH}'"
    ));

    client
        .create_function()
        .function_name(LAMBDA_NAME)
        .runtime(Runtime::Providedal2023)
        .handler("lib.function_handler")
        .role("arn:aws:iam::000000000000:role/service-role/dummy")
        .code(
            aws_sdk_lambda::types::FunctionCode::builder()
                .zip_file(buffer.into())
                .build(),
        )
        .send()
        .await
        .expect("shouldn't fail creating lambda");

    Ok(())
}

async fn setup_sqs_lambda_config(
    sqs_client: &aws_sdk_sqs::Client,
    lambda_client: &aws_sdk_lambda::Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let q_arn = sqs_client
        .get_queue_attributes()
        .queue_url(QUEUE_URL)
        .attribute_names(QueueArn)
        .send()
        .await
        .expect(&format!(
            "shouldn't fail retrieving ARN for queue '{QUEUE_NAME}' with url '{QUEUE_URL}'"
        ))
        .attributes
        .expect("shouldn't fail getting queue attributes because we explicitly requested some")
        .get(&QueueArn)
        .expect(&format!(
            "shouldn't fail getting queue attribute '{QueueArn}' because we explicitly requested it"
        ))
        .to_string();

    lambda_client
        .create_event_source_mapping()
        .event_source_arn(q_arn)
        .function_name(LAMBDA_NAME)
        .batch_size(1000)
        .maximum_batching_window_in_seconds(5)
        .send()
        .await
        .expect("shouldn't fail creating event source mapping");

    Ok(())
}

pub const QUEUE_NAME: &str = "write_lambda_queue";
pub const QUEUE_URL: &str =
    "http://sqs.eu-central-1.localhost.localstack.cloud:4566/000000000000/write_lambda_queue";

pub async fn setup(dynamodb_client: &aws_sdk_dynamodb::Client) {
    crate::dynamodb::setup(dynamodb_client).await;
}

async fn set_up_queues(sqs_client: &aws_sdk_sqs::Client) -> Result<(), aws_sdk_sqs::Error> {
    sqs_client
        .create_queue()
        .queue_name(QUEUE_NAME)
        .send()
        .await?;
    Ok(())
}

pub async fn reset(sqs_client: &aws_sdk_sqs::Client, dynamodb_client: &aws_sdk_dynamodb::Client) {
    crate::dynamodb::reset(dynamodb_client).await;
    sqs_client
        .purge_queue()
        .queue_url(QUEUE_URL)
        .send()
        .await
        .expect(&format!("shouldn't fail purging queue '{QUEUE_NAME}'"));
}
