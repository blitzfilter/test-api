use aws_sdk_dynamodb::Client;
use testcontainers::ContainerAsync;
use testcontainers_modules::localstack::LocalStack;
use test_api::localstack::spin_up_localstack_with_services;

#[tokio::test]
async fn test() {
    let container = &spin_up_localstack_with_services(&["dynamodb"]).await;
    let client = &test_api::dynamodb::get_client().await;

    should_expose_test_host_and_port(container).await;
    should_spin_up_localstack(client).await;

    test_api::dynamodb::setup(client).await;
    should_set_up_tables_for_setup(client).await;
    should_insert_test_items_for_setup(client).await;
}

async fn should_expose_test_host_and_port(container: &ContainerAsync<LocalStack>) {
    let host_ip = container.get_host().await.ok();
    let host_port = container.get_host_port_ipv4(4566).await.ok();

    assert_eq!(host_ip.unwrap().to_string(), "localhost");
    assert_eq!(host_port.unwrap(), 4566);
}

async fn should_spin_up_localstack(client: &Client) {
    match client.list_tables().send().await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{:?}", e);
            assert!(false);
        }
    }
}

async fn should_set_up_tables_for_setup(client: &Client) {
    let list_tables_output = client.list_tables().send().await.ok().unwrap();
    let tables = list_tables_output.table_names();

    assert_eq!(tables.len(), 3);
    assert!(tables.contains(&"parties".to_string()));
    assert!(tables.contains(&"items".to_string()));
    assert!(tables.contains(&"filters".to_string()));
}

async fn should_insert_test_items_for_setup(client: &Client) {
    let scan_output = client.scan().table_name("items").send().await.ok().unwrap();
    assert_eq!(scan_output.count, 19);
}
