//! Tests that builder query parameters are correctly serialized into GraphQL variables.
//!
//! Uses wiremock to intercept HTTP requests and inspect the actual JSON body sent
//! to verify that each builder setter method produces the expected variable values.

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

// ── TeamsQuery ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn teams_first_sets_variable() {
    let (server, client) = setup("teams").await;
    let _ = client.teams().first(42).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 42);
    assert_eq!(vars["last"], Value::Null);
}

#[tokio::test]
async fn teams_last_sets_variable() {
    let (server, client) = setup("teams").await;
    let _ = client.teams().last(7).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["last"], 7);
    assert_eq!(vars["first"], Value::Null);
}

#[tokio::test]
async fn teams_before_after_set_variables() {
    let (server, client) = setup("teams").await;
    let _ = client
        .teams()
        .before("cursor-abc")
        .after("cursor-xyz")
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["before"], "cursor-abc");
    assert_eq!(vars["after"], "cursor-xyz");
}

#[tokio::test]
async fn teams_include_archived_sets_variable() {
    let (server, client) = setup("teams").await;
    let _ = client.teams().include_archived(true).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["includeArchived"], true);
}

#[tokio::test]
async fn teams_all_params_chain() {
    let (server, client) = setup("teams").await;
    let _ = client
        .teams()
        .first(10)
        .after("cur")
        .include_archived(false)
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 10);
    assert_eq!(vars["after"], "cur");
    assert_eq!(vars["includeArchived"], false);
    assert_eq!(vars["before"], Value::Null);
    assert_eq!(vars["last"], Value::Null);
}

#[tokio::test]
async fn teams_no_params_sends_all_null() {
    let (server, client) = setup("teams").await;
    let _ = client.teams().send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], Value::Null);
    assert_eq!(vars["last"], Value::Null);
    assert_eq!(vars["before"], Value::Null);
    assert_eq!(vars["after"], Value::Null);
    assert_eq!(vars["includeArchived"], Value::Null);
}

// ── UsersQuery ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn users_include_disabled_sets_variable() {
    let (server, client) = setup("users").await;
    let _ = client.users().include_disabled(true).last(5).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["includeDisabled"], true);
    assert_eq!(vars["last"], 5);
}

// ── SearchIssuesQuery (has required `term` arg) ──────────────────────────────

#[tokio::test]
async fn search_issues_term_is_required() {
    let (server, client) = setup("searchIssues").await;
    let _ = client.search_issues("my query").send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["term"], "my query");
}

#[tokio::test]
async fn search_issues_all_optional_params() {
    let (server, client) = setup("searchIssues").await;
    let _ = client
        .search_issues("bug")
        .first(20)
        .include_comments(true)
        .team_id("team-uuid-123")
        .include_archived(true)
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["term"], "bug");
    assert_eq!(vars["first"], 20);
    assert_eq!(vars["includeComments"], true);
    assert_eq!(vars["teamId"], "team-uuid-123");
    assert_eq!(vars["includeArchived"], true);
    // Unset optionals are null.
    assert_eq!(vars["last"], Value::Null);
    assert_eq!(vars["before"], Value::Null);
    assert_eq!(vars["after"], Value::Null);
}

#[tokio::test]
async fn search_issues_string_setters_accept_str_ref() {
    let (server, client) = setup("searchIssues").await;
    // All string setters should accept &str (via impl Into<String>).
    let _ = client
        .search_issues("term")
        .before("b")
        .after("a")
        .team_id("t")
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["term"], "term");
    assert_eq!(vars["before"], "b");
    assert_eq!(vars["after"], "a");
    assert_eq!(vars["teamId"], "t");
}

// ── IssuesQuery ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn issues_first_and_include_archived() {
    let (server, client) = setup("issues").await;
    let _ = client
        .issues()
        .first(100)
        .include_archived(true)
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 100);
    assert_eq!(vars["includeArchived"], true);
}

// ── CyclesQuery ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn cycles_first_sets_variable() {
    let (server, client) = setup("cycles").await;
    let _ = client.cycles().first(50).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 50);
}

// ── IssueLabelsQuery ─────────────────────────────────────────────────────────

#[tokio::test]
async fn issue_labels_first_sets_variable() {
    let (server, client) = setup("issueLabels").await;
    let _ = client.issue_labels().first(250).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 250);
}

// ── ProjectsQuery ────────────────────────────────────────────────────────────

#[tokio::test]
async fn projects_last_and_before() {
    let (server, client) = setup("projects").await;
    let _ = client.projects().last(25).before("cur-end").send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["last"], 25);
    assert_eq!(vars["before"], "cur-end");
}

// ── WorkflowStatesQuery ──────────────────────────────────────────────────────

#[tokio::test]
async fn workflow_states_first_sets_variable() {
    let (server, client) = setup("workflowStates").await;
    let _ = client.workflow_states().first(50).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 50);
}
