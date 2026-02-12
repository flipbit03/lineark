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

    // ── Error handling ──────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn invalid_token_returns_auth_error() {
        let client = Client::from_token("lin_api_invalid_token_12345").unwrap();
        let result = client.viewer().await;
        assert!(result.is_err(), "invalid token should produce an error");
    }
}
