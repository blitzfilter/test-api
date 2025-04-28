use serial_test::serial;
use test_api::localstack::{get_dynamodb_client, spin_up_localstack_with_services};

#[serial]
#[tokio::test]
async fn should_expose_test_host_and_port() {
    let container = spin_up_localstack_with_services(&[]).await;

    let host_ip = container.get_host().await.ok();
    let host_port = container.get_host_port_ipv4(4566).await.ok();

    assert_eq!(host_ip.unwrap().to_string(), "localhost");
    assert_eq!(host_port.unwrap(), 4566);

    drop(container);
}

#[serial]
#[tokio::test]
async fn should_spin_up_localstack() {
    let container = spin_up_localstack_with_services(&["dynamodb"]).await;

    match get_dynamodb_client().await.list_tables().send().await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{:?}", e);
            assert!(false);
        }
    }

    drop(container);
}
