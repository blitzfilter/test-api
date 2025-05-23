use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn blitzfilter_dynamodb_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let fn_block = &input.block;

    let result = quote! {
        #[tokio::test]
        #[test_api::serial_test::serial]
        async fn #fn_name() {
            let container = test_api::dynamodb::get_localstack_dynamodb().await;
            let client = test_api::localstack::get_dynamodb_client().await;

            test_api::dynamodb::setup(client).await;

            let test_fn = async #fn_block;
            test_fn.await;

            test_api::dynamodb::reset(client).await;
        }
    };

    result.into()
}

#[proc_macro_attribute]
pub fn blitzfilter_data_ingestion_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let fn_block = &input.block;

    let result = quote! {
        #[tokio::test]
        #[test_api::serial_test::serial]
        async fn #fn_name() {
            let container = test_api::sqs_lambda_dynamodb::get_localstack_sqs_lambda_dynamodb().await;
            let dynamodb_client = test_api::localstack::get_dynamodb_client().await;
            let sqs_client = test_api::localstack::get_sqs_client().await;

            test_api::sqs_lambda_dynamodb::setup(dynamodb_client).await;

            let test_fn = async #fn_block;
            test_fn.await;

            test_api::sqs_lambda_dynamodb::reset(sqs_client, dynamodb_client).await;
        }
    };

    result.into()
}
