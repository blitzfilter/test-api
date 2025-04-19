use crate::localstack::spin_up_localstack_with_services;
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::config::Credentials;
use aws_sdk_dynamodb::types::ScalarAttributeType::S;
use aws_sdk_dynamodb::types::{
    AttributeDefinition, AttributeValue, BillingMode, DeleteRequest, GlobalSecondaryIndex,
    KeySchemaElement, KeyType, Projection, ProjectionType, PutRequest, TableClass, WriteRequest,
};
use aws_sdk_dynamodb::{Client, Error};
use item_core::item_model::ItemModel;
use serde_dynamo::aws_sdk_dynamodb_1::to_item;
use std::collections::HashMap;
use std::fs;
use testcontainers::ContainerAsync;
use testcontainers_modules::localstack::LocalStack;
use tokio::sync::OnceCell;

static CLIENT: OnceCell<Client> = OnceCell::const_new();

/// Lazily initializes and returns a shared DynamoDB client.
pub async fn get_client() -> &'static Client {
    CLIENT
        .get_or_init(|| async {
            let config = aws_config::defaults(BehaviorVersion::latest())
                .credentials_provider(Credentials::for_tests())
                .region("eu-central-1")
                .endpoint_url("http://localhost:4566")
                .load()
                .await;

            Client::new(&config)
        })
        .await
}

static LOCALSTACK_DYNAMODB: OnceCell<ContainerAsync<LocalStack>> = OnceCell::const_new();

/// Lazily initializes and returns a shared Localstack container running DynamoDB.
/// 
/// This also [`sets up`](setup) the data.
pub async fn get_localstack_dynamodb() -> &'static ContainerAsync<LocalStack> {
    LOCALSTACK_DYNAMODB
        .get_or_init(|| async {
            let container = spin_up_localstack_with_services(&["dynamodb"]).await;
            setup(get_client().await).await;
            container
        })
        .await
}

/// Sets up all tables and populates them with test data.
///
/// The test data resides in `src/data/`.
pub async fn setup(client: &Client) {
    set_up_tables(client).await.ok();
    populate_tables(client).await.ok();
}

async fn set_up_tables(client: &Client) -> Result<(), Error> {
    client
        .create_table()
        .table_name("parties")
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("pk")
                .attribute_type(S)
                .build()?,
        )
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("pk")
                .key_type(KeyType::Hash)
                .build()?,
        )
        .billing_mode(BillingMode::PayPerRequest)
        .table_class(TableClass::Standard)
        .send()
        .await?;

    client
        .create_table()
        .table_name("items")
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("pk")
                .attribute_type(S)
                .build()?,
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("sk")
                .attribute_type(S)
                .build()?,
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("party_id")
                .attribute_type(S)
                .build()?,
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("event_id")
                .attribute_type(S)
                .build()?,
        )
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("pk")
                .key_type(KeyType::Hash)
                .build()?,
        )
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("sk")
                .key_type(KeyType::Range)
                .build()?,
        )
        .global_secondary_indexes(
            GlobalSecondaryIndex::builder()
                .index_name("gsi_1_hash_index")
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name("party_id")
                        .key_type(KeyType::Hash)
                        .build()?,
                )
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name("event_id")
                        .key_type(KeyType::Range)
                        .build()?,
                )
                .projection(
                    Projection::builder()
                        .projection_type(ProjectionType::Include)
                        .non_key_attributes("hash")
                        .build(),
                )
                .build()?,
        )
        .billing_mode(BillingMode::PayPerRequest)
        .table_class(TableClass::Standard)
        .send()
        .await?;

    client
        .create_table()
        .table_name("filters")
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("pk")
                .attribute_type(S)
                .build()?,
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("sk")
                .attribute_type(S)
                .build()?,
        )
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("pk")
                .key_type(KeyType::Hash)
                .build()?,
        )
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("sk")
                .key_type(KeyType::Range)
                .build()?,
        )
        .global_secondary_indexes(
            GlobalSecondaryIndex::builder()
                .index_name("gsi_1_inverted_keys")
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name("sk")
                        .key_type(KeyType::Hash)
                        .build()?,
                )
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name("pk")
                        .key_type(KeyType::Range)
                        .build()?,
                )
                .projection(
                    Projection::builder()
                        .projection_type(ProjectionType::KeysOnly)
                        .build(),
                )
                .build()?,
        )
        .billing_mode(BillingMode::PayPerRequest)
        .table_class(TableClass::Standard)
        .send()
        .await?;

    Ok(())
}

async fn populate_tables(client: &Client) -> Result<(), Error> {
    populate_items(client).await
}

async fn populate_items(client: &Client) -> Result<(), Error> {
    let all_items: Vec<ItemModel> = serde_json::from_str(
        fs::read_to_string("src/data/items.json")
            .ok()
            .unwrap()
            .as_str(),
    )
    .ok()
    .unwrap();

    for items in all_items.chunks(25) {
        let reqs = items
            .iter()
            .map(|item_diff| to_item(item_diff).ok())
            .map(|payload| PutRequest::builder().set_item(payload).build().ok())
            .map(|req| WriteRequest::builder().set_put_request(req).build())
            .collect();

        client
            .batch_write_item()
            .request_items("items", reqs)
            .send()
            .await
            .ok();
    }

    Ok(())
}

/// Resets the DynamoDB to it's [`initial`](setup) state.
///
/// Deletes all entries from all tables and repopulates with test data.
///
/// The test data resides in `src/data/`.
pub async fn reset(client: &Client) {
    depopulate_tables(client).await.ok();
    populate_tables(client).await.ok();
}

async fn depopulate_tables(client: &Client) -> Result<(), Error> {
    let list_tables_output = client.list_tables().send().await?;
    let tables = list_tables_output.table_names();

    for table in tables {
        let mut last_evaluated_key = None;

        loop {
            let scan_output = client
                .scan()
                .table_name(table)
                .set_exclusive_start_key(last_evaluated_key)
                .send()
                .await?;

            if let Some(items) = scan_output.items {
                for chunk in items.chunks(25) {
                    let delete_requests: Vec<WriteRequest> = chunk
                        .iter()
                        .map(|item| {
                            let key = extract_primary_key(item);
                            WriteRequest::builder()
                                .delete_request(
                                    DeleteRequest::builder()
                                        .set_key(Some(key))
                                        .build()
                                        .ok()
                                        .unwrap(),
                                )
                                .build()
                        })
                        .collect();

                    let mut request_items = HashMap::new();
                    request_items.insert(table.clone(), delete_requests);

                    client
                        .batch_write_item()
                        .set_request_items(Some(request_items))
                        .send()
                        .await?;
                }
            }

            match scan_output.last_evaluated_key {
                Some(key) => last_evaluated_key = Some(key),
                None => break,
            }
        }
    }

    Ok(())
}

fn extract_primary_key(item: &HashMap<String, AttributeValue>) -> HashMap<String, AttributeValue> {
    let mut key = HashMap::new();
    if let Some(pk) = item.get("pk") {
        key.insert("pk".to_string(), pk.clone());
    }
    if let Some(sk) = item.get("sk") {
        key.insert("sk".to_string(), sk.clone());
    }
    key
}
