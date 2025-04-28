use item_core::item_data::ItemData;
use item_core::item_model::ItemModel;
use std::time::Duration;
use test_api::generator::Generator;
use test_api::localstack::{get_lambda_client, get_sqs_client};
use test_api::sqs_lambda_dynamodb::{LAMBDA_NAME, QUEUE_URL};
use test_api_macros::blitzfilter_data_ingestion_test;
use tokio::time::sleep;

#[blitzfilter_data_ingestion_test]
async fn should_enable_lambda_service_and_upload_lambda() {
    let list_functions_res = get_lambda_client().await.list_functions().send().await;
    assert!(list_functions_res.is_ok());

    let list_functions_opt = list_functions_res.unwrap().functions;
    let list_functions = list_functions_opt.unwrap();
    assert_eq!(list_functions.len(), 1);
    assert_eq!(
        list_functions[0].function_name,
        Some(LAMBDA_NAME.to_string())
    );
}

#[blitzfilter_data_ingestion_test]
async fn should_insert_msg_in_q_then_trigger_lambda() {
    let item: ItemData = ItemModel::generate().into();
    let sqs_client = get_sqs_client().await;
    sqs_client
        .send_message()
        .queue_url(QUEUE_URL)
        .message_body(serde_json::to_string(&item).unwrap())
        .send()
        .await
        .expect("shouldn't fail sending message to queue");

    // Wait for lambda to poll event
    sleep(Duration::from_secs(10)).await;

    // Check if queue is empty - someone else (Lambda) polled the event we previously sent
    let receive_res = sqs_client
        .receive_message()
        .queue_url(QUEUE_URL)
        .send()
        .await;
    assert!(receive_res.is_ok());
    let received_msgs_opt = receive_res.unwrap().messages;
    assert!(received_msgs_opt.is_none());
}
