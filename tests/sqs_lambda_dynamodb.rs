use test_api::sqs_lambda_dynamodb::{QUEUE_URL, get_sqs_client};
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
