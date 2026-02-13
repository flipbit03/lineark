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

// ── TeamsQueryBuilder ───────────────────────────────────────────────────────────────

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

// ── UsersQueryBuilder ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn users_include_disabled_sets_variable() {
    let (server, client) = setup("users").await;
    let _ = client.users().include_disabled(true).last(5).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["includeDisabled"], true);
    assert_eq!(vars["last"], 5);
}

// ── SearchIssuesQueryBuilder (has required `term` arg) ──────────────────────────────

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

// ── IssuesQueryBuilder ──────────────────────────────────────────────────────────────

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

// ── CyclesQueryBuilder ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn cycles_first_sets_variable() {
    let (server, client) = setup("cycles").await;
    let _ = client.cycles().first(50).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 50);
}

// ── IssueLabelsQueryBuilder ─────────────────────────────────────────────────────────

#[tokio::test]
async fn issue_labels_first_sets_variable() {
    let (server, client) = setup("issueLabels").await;
    let _ = client.issue_labels().first(250).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 250);
}

// ── ProjectsQueryBuilder ────────────────────────────────────────────────────────────

#[tokio::test]
async fn projects_last_and_before() {
    let (server, client) = setup("projects").await;
    let _ = client.projects().last(25).before("cur-end").send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["last"], 25);
    assert_eq!(vars["before"], "cur-end");
}

// ── WorkflowStatesQueryBuilder ──────────────────────────────────────────────────────

#[tokio::test]
async fn workflow_states_first_sets_variable() {
    let (server, client) = setup("workflowStates").await;
    let _ = client.workflow_states().first(50).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 50);
}

// ── DocumentsQueryBuilder ──────────────────────────────────────────────────────────

#[tokio::test]
async fn documents_first_sets_variable() {
    let (server, client) = setup("documents").await;
    let _ = client.documents().first(20).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 20);
}

#[tokio::test]
async fn documents_last_sets_variable() {
    let (server, client) = setup("documents").await;
    let _ = client.documents().last(8).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["last"], 8);
    assert_eq!(vars["first"], Value::Null);
}

#[tokio::test]
async fn documents_after_sets_variable() {
    let (server, client) = setup("documents").await;
    let _ = client
        .documents()
        .first(10)
        .after("cursor-abc")
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["after"], "cursor-abc");
}

#[tokio::test]
async fn documents_before_sets_variable() {
    let (server, client) = setup("documents").await;
    let _ = client.documents().before("cursor-end").send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["before"], "cursor-end");
}

#[tokio::test]
async fn documents_include_archived_sets_variable() {
    let (server, client) = setup("documents").await;
    let _ = client.documents().include_archived(true).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["includeArchived"], true);
}

#[tokio::test]
async fn documents_all_params_chain() {
    let (server, client) = setup("documents").await;
    let _ = client
        .documents()
        .first(15)
        .after("cur-start")
        .include_archived(true)
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 15);
    assert_eq!(vars["after"], "cur-start");
    assert_eq!(vars["includeArchived"], true);
    assert_eq!(vars["before"], Value::Null);
    assert_eq!(vars["last"], Value::Null);
}

#[tokio::test]
async fn documents_no_params_sends_all_null() {
    let (server, client) = setup("documents").await;
    let _ = client.documents().send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], Value::Null);
    assert_eq!(vars["last"], Value::Null);
    assert_eq!(vars["before"], Value::Null);
    assert_eq!(vars["after"], Value::Null);
    assert_eq!(vars["includeArchived"], Value::Null);
}

// ── IssueRelationsQueryBuilder ─────────────────────────────────────────────────────

#[tokio::test]
async fn issue_relations_first_sets_variable() {
    let (server, client) = setup("issueRelations").await;
    let _ = client.issue_relations().first(25).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 25);
}

#[tokio::test]
async fn issue_relations_last_sets_variable() {
    let (server, client) = setup("issueRelations").await;
    let _ = client.issue_relations().last(3).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["last"], 3);
    assert_eq!(vars["first"], Value::Null);
}

#[tokio::test]
async fn issue_relations_before_after_set_variables() {
    let (server, client) = setup("issueRelations").await;
    let _ = client
        .issue_relations()
        .before("cursor-b")
        .after("cursor-a")
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["before"], "cursor-b");
    assert_eq!(vars["after"], "cursor-a");
}

#[tokio::test]
async fn issue_relations_include_archived_sets_variable() {
    let (server, client) = setup("issueRelations").await;
    let _ = client.issue_relations().include_archived(true).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["includeArchived"], true);
}

#[tokio::test]
async fn issue_relations_all_params_chain() {
    let (server, client) = setup("issueRelations").await;
    let _ = client
        .issue_relations()
        .first(30)
        .after("rel-cursor")
        .include_archived(false)
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 30);
    assert_eq!(vars["after"], "rel-cursor");
    assert_eq!(vars["includeArchived"], false);
    assert_eq!(vars["before"], Value::Null);
    assert_eq!(vars["last"], Value::Null);
}

#[tokio::test]
async fn issue_relations_no_params_sends_all_null() {
    let (server, client) = setup("issueRelations").await;
    let _ = client.issue_relations().send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], Value::Null);
    assert_eq!(vars["last"], Value::Null);
    assert_eq!(vars["before"], Value::Null);
    assert_eq!(vars["after"], Value::Null);
    assert_eq!(vars["includeArchived"], Value::Null);
}

// ── Mutation variable tests ─────────────────────────────────────────────────

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

#[tokio::test]
async fn document_create_sends_input_variable() {
    use lineark_sdk::generated::inputs::DocumentCreateInput;

    let (server, client) = setup_mutation("documentCreate").await;
    let input = DocumentCreateInput {
        title: Some("Test Document".to_string()),
        content: Some("# Hello".to_string()),
        ..Default::default()
    };
    let _ = client.document_create(input).await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["input"]["title"], "Test Document");
    assert_eq!(vars["input"]["content"], "# Hello");
}

#[tokio::test]
async fn document_update_sends_input_and_id() {
    use lineark_sdk::generated::inputs::DocumentUpdateInput;

    let (server, client) = setup_mutation("documentUpdate").await;
    let input = DocumentUpdateInput {
        title: Some("Updated Title".to_string()),
        ..Default::default()
    };
    let _ = client
        .document_update(input, "doc-uuid-123".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["input"]["title"], "Updated Title");
    assert_eq!(vars["id"], "doc-uuid-123");
}

#[tokio::test]
async fn document_delete_sends_id() {
    let (server, client) = setup_mutation("documentDelete").await;
    let _ = client.document_delete("doc-uuid-456".to_string()).await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["id"], "doc-uuid-456");
}

#[tokio::test]
async fn issue_relation_create_sends_input() {
    use lineark_sdk::generated::enums::IssueRelationType;
    use lineark_sdk::generated::inputs::IssueRelationCreateInput;

    let (server, client) = setup_mutation("issueRelationCreate").await;
    let input = IssueRelationCreateInput {
        issue_id: Some("issue-a".to_string()),
        related_issue_id: Some("issue-b".to_string()),
        r#type: Some(IssueRelationType::Blocks),
        ..Default::default()
    };
    let _ = client.issue_relation_create(None, input).await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["input"]["issueId"], "issue-a");
    assert_eq!(vars["input"]["relatedIssueId"], "issue-b");
    assert_eq!(vars["input"]["type"], "blocks");
    assert_eq!(vars["overrideCreatedAt"], Value::Null);
}

#[tokio::test]
async fn file_upload_sends_required_params() {
    let (server, client) = setup_mutation("fileUpload").await;
    let _ = client
        .file_upload(
            None,
            Some(true),
            1024,
            "image/png".to_string(),
            "screenshot.png".to_string(),
        )
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["size"], 1024);
    assert_eq!(vars["contentType"], "image/png");
    assert_eq!(vars["filename"], "screenshot.png");
    assert_eq!(vars["makePublic"], true);
    assert_eq!(vars["metaData"], Value::Null);
}

#[tokio::test]
async fn image_upload_from_url_sends_url() {
    let (server, client) = setup_mutation("imageUploadFromUrl").await;
    let _ = client
        .image_upload_from_url("https://example.com/image.png".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["url"], "https://example.com/image.png");
}

#[tokio::test]
async fn issue_archive_sends_id_and_trash() {
    let (server, client) = setup_mutation("issueArchive").await;
    let _ = client
        .issue_archive(Some(true), "issue-uuid-arch".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["id"], "issue-uuid-arch");
    assert_eq!(vars["trash"], true);
}

#[tokio::test]
async fn issue_archive_without_trash_sends_null() {
    let (server, client) = setup_mutation("issueArchive").await;
    let _ = client
        .issue_archive(None, "issue-uuid-arch2".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["id"], "issue-uuid-arch2");
    assert_eq!(vars["trash"], Value::Null);
}

#[tokio::test]
async fn issue_unarchive_sends_id() {
    let (server, client) = setup_mutation("issueUnarchive").await;
    let _ = client
        .issue_unarchive("issue-uuid-unarch".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["id"], "issue-uuid-unarch");
}

#[tokio::test]
async fn issue_delete_sends_id_and_permanently_delete() {
    let (server, client) = setup_mutation("issueDelete").await;
    let _ = client
        .issue_delete(Some(true), "issue-uuid-123".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["id"], "issue-uuid-123");
    assert_eq!(vars["permanentlyDelete"], true);
}

#[tokio::test]
async fn issue_delete_without_permanently_sends_null() {
    let (server, client) = setup_mutation("issueDelete").await;
    let _ = client
        .issue_delete(None, "issue-uuid-456".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["id"], "issue-uuid-456");
    assert_eq!(vars["permanentlyDelete"], Value::Null);
}
