use aws_sdk_lambda::Client;
use test_api::localstack::get_aws_config;
use test_api::sqs_lambda_dynamodb::{QUEUE_URL, get_sqs_client, LAMBDA_NAME};
use test_api_macros::blitzfilter_data_ingestion_test;

#[blitzfilter_data_ingestion_test]
async fn should_send_sqs_message_and_consume_it() {
    let sqs_client = get_sqs_client().await;

    let send_res = sqs_client
        .send_message()
        .queue_url(QUEUE_URL)
        .message_body("Test message body")
        .send()
        .await;
    assert!(send_res.is_ok());

    let receive_res = sqs_client
        .receive_message()
        .queue_url(QUEUE_URL)
        .send()
        .await;
    assert!(receive_res.is_ok());

    let received_msgs_opt = receive_res.unwrap().messages;
    assert!(received_msgs_opt.is_some());
    let received_msgs = received_msgs_opt.unwrap();
    assert_eq!(received_msgs.len(), 1);
    assert_eq!(
        received_msgs.get(0).unwrap().body.clone().unwrap(),
        "Test message body"
    );
}

#[blitzfilter_data_ingestion_test]
async fn should_enable_lambda_service_and_upload_lambda() {
    let client = Client::new(get_aws_config().await);
    let list_functions_res = client.list_functions().send().await;
    assert!(list_functions_res.is_ok());
    
    let list_functions_opt = list_functions_res.unwrap().functions;
    let list_functions = list_functions_opt.unwrap();
    assert_eq!(list_functions.len(), 1);
    assert_eq!(list_functions[0].function_name, Some(LAMBDA_NAME.to_string()));
}
