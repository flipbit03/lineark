use super::*;

// ── TeamsQueryBuilder ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn teams_first_sets_variable() {
    let (server, client) = setup("teams").await;
    let _ = client.teams::<Team>().first(42).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 42);
    assert_eq!(vars["last"], Value::Null);
}

#[tokio::test]
async fn teams_last_sets_variable() {
    let (server, client) = setup("teams").await;
    let _ = client.teams::<Team>().last(7).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["last"], 7);
    assert_eq!(vars["first"], Value::Null);
}

#[tokio::test]
async fn teams_before_after_set_variables() {
    let (server, client) = setup("teams").await;
    let _ = client
        .teams::<Team>()
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
    let _ = client.teams::<Team>().include_archived(true).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["includeArchived"], true);
}

#[tokio::test]
async fn teams_all_params_chain() {
    let (server, client) = setup("teams").await;
    let _ = client
        .teams::<Team>()
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
    let _ = client.teams::<Team>().send().await;
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
    let _ = client
        .users::<User>()
        .include_disabled(true)
        .last(5)
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["includeDisabled"], true);
    assert_eq!(vars["last"], 5);
}

// ── SearchIssuesQueryBuilder (has required `term` arg) ──────────────────────────────

#[tokio::test]
async fn search_issues_term_is_required() {
    let (server, client) = setup("searchIssues").await;
    let _ = client
        .search_issues::<IssueSearchResult>("my query")
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["term"], "my query");
}

#[tokio::test]
async fn search_issues_all_optional_params() {
    let (server, client) = setup("searchIssues").await;
    let _ = client
        .search_issues::<IssueSearchResult>("bug")
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
        .search_issues::<IssueSearchResult>("term")
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
        .issues::<Issue>()
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
    let _ = client.cycles::<Cycle>().first(50).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 50);
}

// ── IssueLabelsQueryBuilder ─────────────────────────────────────────────────────────

#[tokio::test]
async fn issue_labels_first_sets_variable() {
    let (server, client) = setup("issueLabels").await;
    let _ = client.issue_labels::<IssueLabel>().first(250).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 250);
}

// ── ProjectsQueryBuilder ────────────────────────────────────────────────────────────

#[tokio::test]
async fn projects_last_and_before() {
    let (server, client) = setup("projects").await;
    let _ = client
        .projects::<Project>()
        .last(25)
        .before("cur-end")
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["last"], 25);
    assert_eq!(vars["before"], "cur-end");
}

// ── WorkflowStatesQueryBuilder ──────────────────────────────────────────────────────

#[tokio::test]
async fn workflow_states_first_sets_variable() {
    let (server, client) = setup("workflowStates").await;
    let _ = client
        .workflow_states::<WorkflowState>()
        .first(50)
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 50);
}

// ── DocumentsQueryBuilder ──────────────────────────────────────────────────────────

#[tokio::test]
async fn documents_first_sets_variable() {
    let (server, client) = setup("documents").await;
    let _ = client.documents::<Document>().first(20).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 20);
}

#[tokio::test]
async fn documents_last_sets_variable() {
    let (server, client) = setup("documents").await;
    let _ = client.documents::<Document>().last(8).send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["last"], 8);
    assert_eq!(vars["first"], Value::Null);
}

#[tokio::test]
async fn documents_after_sets_variable() {
    let (server, client) = setup("documents").await;
    let _ = client
        .documents::<Document>()
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
    let _ = client
        .documents::<Document>()
        .before("cursor-end")
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["before"], "cursor-end");
}

#[tokio::test]
async fn documents_include_archived_sets_variable() {
    let (server, client) = setup("documents").await;
    let _ = client
        .documents::<Document>()
        .include_archived(true)
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["includeArchived"], true);
}

#[tokio::test]
async fn documents_all_params_chain() {
    let (server, client) = setup("documents").await;
    let _ = client
        .documents::<Document>()
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
    let _ = client.documents::<Document>().send().await;
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
    let _ = client
        .issue_relations::<IssueRelation>()
        .first(25)
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], 25);
}

#[tokio::test]
async fn issue_relations_last_sets_variable() {
    let (server, client) = setup("issueRelations").await;
    let _ = client
        .issue_relations::<IssueRelation>()
        .last(3)
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["last"], 3);
    assert_eq!(vars["first"], Value::Null);
}

#[tokio::test]
async fn issue_relations_before_after_set_variables() {
    let (server, client) = setup("issueRelations").await;
    let _ = client
        .issue_relations::<IssueRelation>()
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
    let _ = client
        .issue_relations::<IssueRelation>()
        .include_archived(true)
        .send()
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["includeArchived"], true);
}

#[tokio::test]
async fn issue_relations_all_params_chain() {
    let (server, client) = setup("issueRelations").await;
    let _ = client
        .issue_relations::<IssueRelation>()
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
    let _ = client.issue_relations::<IssueRelation>().send().await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["first"], Value::Null);
    assert_eq!(vars["last"], Value::Null);
    assert_eq!(vars["before"], Value::Null);
    assert_eq!(vars["after"], Value::Null);
    assert_eq!(vars["includeArchived"], Value::Null);
}
