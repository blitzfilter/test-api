use test_api::localstack::spin_up_localstack_with_services;

#[tokio::test]
async fn should_spin_up_localstack() {
    let container = spin_up_localstack_with_services(&["dynamodb"]).await;
    let host_ip = container.get_host().await.ok();
    let host_port = container.get_host_port_ipv4(4566).await.ok();

    assert_eq!(host_ip.unwrap().to_string(), "localhost");
    assert_eq!(host_port.unwrap(), 4566);

    let client = &test_api::dynamodb::get_client().await;

    // health ping
    match client.list_tables().send().await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{:?}", e);
            assert!(false);
        }
    }
}
