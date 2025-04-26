use crate::localstack::{get_aws_config, spin_up_localstack_with_services};
use testcontainers::ContainerAsync;
use testcontainers_modules::localstack::LocalStack;
use tokio::sync::OnceCell;

static CLIENT: OnceCell<aws_sdk_sqs::Client> = OnceCell::const_new();

/// Lazily initializes and returns a shared SQS client.
pub async fn get_sqs_client() -> &'static aws_sdk_sqs::Client {
    CLIENT
        .get_or_init(|| async {
            let config = get_aws_config().await;
            aws_sdk_sqs::Client::new(&config)
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
            // TODO: Upload lambda
        })
        .await
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
