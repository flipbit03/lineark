use lineark_sdk::Client;
use lineark_test_utils::{cleanup_workspace, test_token};

#[tokio::main]
async fn main() {
    let client = Client::from_token(test_token()).expect("failed to create test client");
    cleanup_workspace(&client).await;
    eprintln!("cleanup: done");
}
