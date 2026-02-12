//! Online integration tests for the lineark SDK against a real Linear workspace.
//!
//! These tests require a valid Linear API token at `~/.linear_api_token_test`.
//! When the token file is missing, tests are automatically skipped with a message.
//!
//! The token should be connected to a test workspace — never use production tokens here.

use lineark_sdk::Client;

fn no_online_test_token() -> Option<String> {
    let path = home::home_dir()?.join(".linear_api_token_test");
    if path.exists() {
        None
    } else {
        Some("~/.linear_api_token_test not found".to_string())
    }
}

fn test_client() -> Client {
    let path = home::home_dir()
        .expect("could not determine home directory")
        .join(".linear_api_token_test");
    let token = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {}", path.display(), e))
        .trim()
        .to_string();
    Client::from_token(token).expect("failed to create test client")
}

test_with::tokio_runner!(online);

#[test_with::module]
mod online {
    // ── Viewer ──────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn viewer_returns_authenticated_user() {
        let client = test_client();
        let user = client.viewer().await.unwrap();
        assert!(user.id.is_some(), "viewer should have an id");
        assert!(user.email.is_some(), "viewer should have an email");
        assert!(user.active.is_some(), "viewer should have active status");
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn viewer_fields_deserialize_correctly() {
        let client = test_client();
        let user = client.viewer().await.unwrap();
        assert!(user.id.is_some());
        assert!(user.name.is_some());
        assert!(user.email.is_some());
        let active = user.active.expect("viewer should have active field");
        assert!(active, "test user should be active");
    }

    // ── Teams ───────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn teams_returns_at_least_one_team() {
        let client = test_client();
        let conn = client
            .teams(None, None, Some(10), None, None)
            .await
            .unwrap();
        assert!(
            !conn.nodes.is_empty(),
            "workspace should have at least one team"
        );
        let team = &conn.nodes[0];
        assert!(team.id.is_some());
        assert!(team.name.is_some());
        assert!(team.key.is_some());
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn team_by_id() {
        let client = test_client();
        let conn = client.teams(None, None, Some(1), None, None).await.unwrap();
        assert!(!conn.nodes.is_empty());
        let team_id = conn.nodes[0].id.clone().unwrap();
        let team = client.team(team_id.clone()).await.unwrap();
        assert_eq!(team.id, Some(team_id));
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn team_fields_deserialize_correctly() {
        let client = test_client();
        let conn = client.teams(None, None, Some(1), None, None).await.unwrap();
        assert!(!conn.nodes.is_empty());
        let team = &conn.nodes[0];
        assert!(team.id.is_some());
        assert!(team.key.is_some());
        assert!(team.name.is_some());
        let key = team.key.as_ref().unwrap();
        assert!(!key.is_empty());
    }

    // ── Users ───────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn users_returns_at_least_one_user() {
        let client = test_client();
        let conn = client
            .users(None, None, None, Some(10), None, None)
            .await
            .unwrap();
        assert!(
            !conn.nodes.is_empty(),
            "workspace should have at least one user"
        );
        let user = &conn.nodes[0];
        assert!(user.id.is_some());
        assert!(user.name.is_some());
    }

    // ── Projects ────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn projects_returns_connection() {
        let client = test_client();
        let conn = client
            .projects(None, None, Some(10), None, None)
            .await
            .unwrap();
        assert!(conn.page_info.has_next_page || !conn.page_info.has_next_page);
    }

    // ── Issues ──────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issues_returns_connection() {
        let client = test_client();
        let conn = client
            .issues(None, None, Some(5), None, None)
            .await
            .unwrap();
        for issue in &conn.nodes {
            assert!(issue.id.is_some());
        }
    }

    // ── Issue Labels ────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issue_labels_returns_connection() {
        let client = test_client();
        let conn = client
            .issue_labels(None, None, Some(10), None, None)
            .await
            .unwrap();
        for label in &conn.nodes {
            assert!(label.id.is_some());
            assert!(label.name.is_some());
        }
    }

    // ── Cycles ──────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn cycles_returns_connection() {
        let client = test_client();
        let conn = client
            .cycles(None, None, Some(10), None, None)
            .await
            .unwrap();
        for cycle in &conn.nodes {
            assert!(cycle.id.is_some());
        }
    }

    // ── Workflow States ─────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn workflow_states_returns_connection() {
        let client = test_client();
        let conn = client
            .workflow_states(None, None, Some(50), None, None)
            .await
            .unwrap();
        assert!(
            !conn.nodes.is_empty(),
            "workspace should have workflow states"
        );
        let state = &conn.nodes[0];
        assert!(state.id.is_some());
        assert!(state.name.is_some());
    }

    // ── Search ──────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn search_issues_returns_connection() {
        let client = test_client();
        let conn = client
            .search_issues(
                None,
                None,
                Some(5),
                None,
                None,
                "test".to_string(),
                None,
                None,
            )
            .await
            .unwrap();
        for issue in &conn.nodes {
            assert!(issue.id.is_some());
        }
    }

    // ── Pagination ──────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn pagination_respects_first_limit() {
        let client = test_client();
        let conn = client.teams(None, None, Some(1), None, None).await.unwrap();
        assert!(
            conn.nodes.len() <= 1,
            "first=1 should return at most 1 team"
        );
    }

    // ── Mutations ────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issue_create_and_delete() {
        use lineark_sdk::generated::inputs::IssueCreateInput;

        let client = test_client();

        // Get the first team to create an issue in.
        let teams = client.teams(None, None, Some(1), None, None).await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        // Create an issue.
        let input = IssueCreateInput {
            title: Some("[test] SDK issue_create_and_delete".to_string()),
            team_id: Some(team_id),
            description: Some("Automated test — will be deleted immediately.".to_string()),
            priority: Some(4), // Low
            ..Default::default()
        };
        let payload = client.issue_create(input).await.unwrap();
        assert_eq!(payload.get("success").and_then(|v| v.as_bool()), Some(true));

        let issue = payload.get("issue").unwrap();
        let issue_id = issue.get("id").and_then(|v| v.as_str()).unwrap();
        assert!(!issue_id.is_empty());
        let identifier = issue
            .get("identifier")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            identifier.contains('-'),
            "identifier should be like ABC-123"
        );

        // Permanently delete the issue to keep the workspace clean.
        let delete_payload = client
            .issue_delete(Some(true), issue_id.to_string())
            .await
            .unwrap();
        assert_eq!(
            delete_payload.get("success").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issue_update() {
        use lineark_sdk::generated::inputs::{IssueCreateInput, IssueUpdateInput};

        let client = test_client();

        // Create an issue to update.
        let teams = client.teams(None, None, Some(1), None, None).await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        let input = IssueCreateInput {
            title: Some("[test] SDK issue_update".to_string()),
            team_id: Some(team_id),
            priority: Some(4),
            ..Default::default()
        };
        let create_payload = client.issue_create(input).await.unwrap();
        let issue = create_payload.get("issue").unwrap();
        let issue_id = issue
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        // Update the issue.
        let update_input = IssueUpdateInput {
            title: Some("[test] SDK issue_update — updated".to_string()),
            priority: Some(3), // Medium
            ..Default::default()
        };
        let update_payload = client
            .issue_update(update_input, issue_id.clone())
            .await
            .unwrap();
        assert_eq!(
            update_payload.get("success").and_then(|v| v.as_bool()),
            Some(true)
        );
        let updated_issue = update_payload.get("issue").unwrap();
        assert_eq!(
            updated_issue.get("title").and_then(|v| v.as_str()),
            Some("[test] SDK issue_update — updated")
        );

        // Clean up: permanently delete.
        client.issue_delete(Some(true), issue_id).await.unwrap();
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn comment_create() {
        use lineark_sdk::generated::inputs::{CommentCreateInput, IssueCreateInput};

        let client = test_client();

        // Create an issue to comment on.
        let teams = client.teams(None, None, Some(1), None, None).await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        let issue_input = IssueCreateInput {
            title: Some("[test] SDK comment_create".to_string()),
            team_id: Some(team_id),
            priority: Some(4),
            ..Default::default()
        };
        let create_payload = client.issue_create(issue_input).await.unwrap();
        let issue = create_payload.get("issue").unwrap();
        let issue_id = issue
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        // Create a comment.
        let comment_input = CommentCreateInput {
            body: Some("Automated test comment from lineark SDK.".to_string()),
            issue_id: Some(issue_id.clone()),
            ..Default::default()
        };
        let comment_payload = client.comment_create(comment_input).await.unwrap();
        assert_eq!(
            comment_payload.get("success").and_then(|v| v.as_bool()),
            Some(true)
        );
        let comment = comment_payload.get("comment").unwrap();
        assert!(comment.get("id").and_then(|v| v.as_str()).is_some());

        // Clean up: permanently delete the issue.
        client.issue_delete(Some(true), issue_id).await.unwrap();
    }

    // ── Error handling ──────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn invalid_token_returns_auth_error() {
        let client = Client::from_token("lin_api_invalid_token_12345").unwrap();
        let result = client.viewer().await;
        assert!(result.is_err(), "invalid token should produce an error");
    }
}
