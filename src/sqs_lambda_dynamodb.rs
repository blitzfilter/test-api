use crate::localstack::{get_aws_config, spin_up_localstack_with_services};
use aws_sdk_lambda::types::Runtime;
use aws_sdk_sqs::types::QueueAttributeName::QueueArn;
use std::fs::File;
use std::io::Read;
use std::process::Command;
use testcontainers::ContainerAsync;
use testcontainers_modules::localstack::LocalStack;
use tokio::sync::OnceCell;

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

static LOCALSTACK_SQS_LAMBDA_DYNAMODB: OnceCell<ContainerAsync<LocalStack>> = OnceCell::const_new();

/// Lazily initializes and returns a shared Localstack container running:
/// - SQS collecting items to write
/// - Lambda consuming items from the SQS and writing them to
/// - DynamoDB
pub async fn get_localstack_sqs_lambda_dynamodb() -> &'static ContainerAsync<LocalStack> {
    LOCALSTACK_SQS_LAMBDA_DYNAMODB
        .get_or_init(|| async {
            spin_up_localstack_with_services(&["sqs", "lambda", "dynamodb"]).await
        })
        .await
}

pub const LAMBDA_NAME: &str = "item_write_lambda";
const LAMBDA_BOOTSRAP_ZIP_PATH: &str = "/tmp/item_write_lambda_bootstrap.zip";

async fn upload_lambda() {
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

    // Read the Lambda function's zip file into memory
    let mut file = File::open(LAMBDA_BOOTSRAP_ZIP_PATH).expect(&format!(
        "shouldn't fail opening '{LAMBDA_BOOTSRAP_ZIP_PATH}'"
    ));
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect(&format!(
        "shouldn't fail reading '{LAMBDA_BOOTSRAP_ZIP_PATH}'"
    ));

    let client = aws_sdk_lambda::Client::new(get_aws_config().await);
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
}

async fn configure_lambda_with_sqs() {
    let q_arn = get_sqs_client()
        .await
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

    get_lambda_client()
        .await
        .create_event_source_mapping()
        .event_source_arn(q_arn)
        .function_name(LAMBDA_NAME)
        .batch_size(1000)
        .maximum_batching_window_in_seconds(5)
        .send()
        .await
        .expect("shouldn't fail creating event source mapping");
}

pub const QUEUE_NAME: &str = "write_lambda_queue";
pub const QUEUE_URL: &str =
    "http://sqs.eu-central-1.localhost.localstack.cloud:4566/000000000000/write_lambda_queue";

pub async fn setup(sqs_client: &aws_sdk_sqs::Client, dynamodb_client: &aws_sdk_dynamodb::Client) {
    crate::dynamodb::setup(dynamodb_client).await;
    tear_down_queues(sqs_client)
        .await
        .expect("shouldn't fail tearing down existing queues");
    set_up_queues(sqs_client)
        .await
        .expect(&format!("shouldn't fail creating queue '{QUEUE_NAME}'"));
    upload_lambda().await;
    configure_lambda_with_sqs().await;
}

async fn tear_down_queues(sqs_client: &aws_sdk_sqs::Client) -> Result<(), aws_sdk_sqs::Error> {
    let qs = sqs_client.list_queues().send().await?;
    for q in qs.queue_urls.unwrap_or_default() {
        sqs_client.delete_queue().queue_url(&q).send().await?;
    }
    Ok(())
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
