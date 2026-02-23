//! Tests that builder query parameters are correctly serialized into GraphQL variables.
//!
//! Uses wiremock to intercept HTTP requests and inspect the actual JSON body sent
//! to verify that each builder setter method produces the expected variable values.

use lineark_sdk::generated::types::*;
use lineark_sdk::Client;
use serde_json::Value;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup(data_path: &str) -> (MockServer, Client) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                data_path: {
                    "nodes": [],
                    "pageInfo": { "hasNextPage": false, "endCursor": null }
                }
            }
        })))
        .mount(&server)
        .await;

    let mut client = Client::from_token("test-token").unwrap();
    client.set_base_url(server.uri());
    (server, client)
}

fn extract_variables(server_requests: &[wiremock::Request]) -> Value {
    assert_eq!(server_requests.len(), 1, "expected exactly one request");
    let body: Value = serde_json::from_slice(&server_requests[0].body).unwrap();
    body["variables"].clone()
}

async fn setup_mutation(data_path: &str) -> (MockServer, Client) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                data_path: {
                    "success": true
                }
            }
        })))
        .mount(&server)
        .await;

    let mut client = Client::from_token("test-token").unwrap();
    client.set_base_url(server.uri());
    (server, client)
}

#[path = "offline/mutations.rs"]
mod mutations;
#[path = "offline/queries.rs"]
mod queries;
