use aws_sdk_dynamodb::types::AttributeValue::S;
use std::collections::HashMap;
use test_api::localstack::get_dynamodb_client;
use test_api_macros::blitzfilter_dynamodb_test;

#[blitzfilter_dynamodb_test]
async fn should_set_up_tables_for_setup() {
    let list_tables_output = get_dynamodb_client()
        .await
        .list_tables()
        .send()
        .await
        .ok()
        .unwrap();
    let tables = list_tables_output.table_names();

    assert_eq!(tables.len(), 3);
    assert!(tables.contains(&"parties".to_string()));
    assert!(tables.contains(&"items".to_string()));
    assert!(tables.contains(&"filters".to_string()));
}

#[blitzfilter_dynamodb_test]
async fn should_insert_test_items_for_setup() {
    let scan_output = get_dynamodb_client()
        .await
        .scan()
        .table_name("items")
        .send()
        .await
        .ok()
        .unwrap();
    assert_eq!(scan_output.count, 25);
}

#[blitzfilter_dynamodb_test]
async fn should_drop_test_items_for_reset() {
    let client = get_dynamodb_client().await;
    client
        .put_item()
        .table_name("items")
        .set_item(Some(HashMap::from([
            ("pk".to_string(), S("item#123456".to_string())),
            ("sk".to_string(), S("item#abcdef".to_string())),
        ])))
        .send()
        .await
        .ok();

    let scan_output_pre_reset = client.scan().table_name("items").send().await.ok().unwrap();
    assert_eq!(scan_output_pre_reset.count, 26);

    test_api::dynamodb::reset(client).await;

    let scan_output_post_reset = client.scan().table_name("items").send().await.ok().unwrap();
    assert_eq!(scan_output_post_reset.count, 0);
}
