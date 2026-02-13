//! Tests for the blocking (synchronous) API client.
//!
//! These tests require the `blocking` feature to be enabled and a valid
//! Linear API token at `~/.linear_api_token_test` for online tests.
//! When the token file is missing, online tests are automatically skipped.

use lineark_sdk::blocking::Client;

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
    Client::from_token(token).expect("failed to create blocking test client")
}

test_with::runner!(blocking);

#[test_with::module]
mod blocking {
    // ── Viewer ──────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn blocking_whoami() {
        let client = test_client();
        let user = client.whoami().unwrap();
        assert!(user.id.is_some(), "viewer should have an id");
        assert!(user.email.is_some(), "viewer should have an email");
    }

    // ── Teams ───────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn blocking_teams_list() {
        let client = test_client();
        let conn = client.teams().first(10).send().unwrap();
        assert!(
            !conn.nodes.is_empty(),
            "workspace should have at least one team"
        );
        let team = &conn.nodes[0];
        assert!(team.id.is_some());
        assert!(team.name.is_some());
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn blocking_team_by_id() {
        let client = test_client();
        let conn = client.teams().first(1).send().unwrap();
        let team_id = conn.nodes[0].id.clone().unwrap();
        let team = client.team(team_id.clone()).unwrap();
        assert_eq!(team.id, Some(team_id));
    }

    // ── Documents ───────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn blocking_documents_list() {
        let client = test_client();
        let conn = client.documents().first(10).send().unwrap();
        // Just verify deserialization works.
        for doc in &conn.nodes {
            assert!(doc.id.is_some());
        }
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn blocking_document_create_and_delete() {
        use lineark_sdk::generated::inputs::DocumentCreateInput;

        let client = test_client();

        // Get a team (documents require at least one parent).
        let teams = client.teams().first(1).send().unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        // Create.
        let input = DocumentCreateInput {
            title: Some("[test] blocking document CRUD".to_string()),
            content: Some("Blocking API test.".to_string()),
            team_id: Some(team_id),
            ..Default::default()
        };
        let payload = client.document_create(input).unwrap();
        assert_eq!(payload.get("success").and_then(|v| v.as_bool()), Some(true));
        let doc_id = payload["document"]["id"].as_str().unwrap().to_string();

        // Read.
        let doc = client.document(doc_id.clone()).unwrap();
        assert_eq!(doc.id, Some(doc_id.clone()));

        // Delete.
        let del = client.document_delete(doc_id).unwrap();
        assert_eq!(del.get("success").and_then(|v| v.as_bool()), Some(true));
    }

    // ── Issue Relations ─────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn blocking_issue_relations_list() {
        let client = test_client();
        let conn = client.issue_relations().first(5).send().unwrap();
        // Just verify deserialization works — may be empty.
        assert!(conn.page_info.has_next_page || !conn.page_info.has_next_page);
    }

    // ── Mutations ───────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn blocking_issue_create_and_delete() {
        use lineark_sdk::generated::inputs::IssueCreateInput;

        let client = test_client();
        let teams = client.teams().first(1).send().unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        let input = IssueCreateInput {
            title: Some("[test] blocking issue_create".to_string()),
            team_id: Some(team_id),
            priority: Some(4),
            ..Default::default()
        };
        let payload = client.issue_create(input).unwrap();
        assert_eq!(payload.get("success").and_then(|v| v.as_bool()), Some(true));
        let issue_id = payload["issue"]["id"].as_str().unwrap().to_string();

        let del = client.issue_delete(Some(true), issue_id).unwrap();
        assert_eq!(del.get("success").and_then(|v| v.as_bool()), Some(true));
    }

    // ── File Upload ─────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn blocking_file_upload_returns_signed_url() {
        let client = test_client();
        let payload = client
            .file_upload(
                None,
                None,
                50,
                "text/plain".to_string(),
                "blocking-test.txt".to_string(),
            )
            .unwrap();
        assert_eq!(payload.get("success").and_then(|v| v.as_bool()), Some(true));
        assert!(
            payload["uploadFile"]["uploadUrl"].as_str().is_some(),
            "should have uploadUrl"
        );
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn blocking_upload_file_end_to_end() {
        let client = test_client();
        let content = b"blocking upload test content".to_vec();
        let result = client
            .upload_file("blocking-test.txt", "text/plain", content, false)
            .unwrap();
        assert!(
            !result.asset_url.is_empty(),
            "asset_url should be non-empty"
        );
        assert!(
            result.asset_url.starts_with("https://"),
            "asset_url should be HTTPS"
        );
    }
}
