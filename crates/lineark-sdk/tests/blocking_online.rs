//! Tests for the blocking (synchronous) API client.
//!
//! These tests require the `blocking` feature to be enabled and a valid
//! Linear API token at `~/.linear_api_token_test` for online tests.
//! When the token file is missing, online tests are automatically skipped.

use lineark_sdk::blocking_client::Client;
use lineark_sdk::generated::types::{Document, Issue};

fn no_online_test_token() -> Option<String> {
    let path = home::home_dir()?.join(".linear_api_token_test");
    if path.exists() {
        None
    } else {
        Some("~/.linear_api_token_test not found".to_string())
    }
}

fn test_token() -> String {
    let path = home::home_dir()
        .expect("could not determine home directory")
        .join(".linear_api_token_test");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {}", path.display(), e))
        .trim()
        .to_string()
}

fn test_client() -> Client {
    Client::from_token(test_token()).expect("failed to create blocking test client")
}

/// Returns the team ID to use for blocking tests.
/// If `LINEAR_TEST_TEAM_KEY` is set, looks up the team by key.
/// Otherwise falls back to `teams().first(1)` (alphabetical first).
fn test_team_id_blocking(client: &Client) -> String {
    use lineark_sdk::generated::types::Team;

    if let Ok(key) = std::env::var("LINEAR_TEST_TEAM_KEY") {
        let conn = client.teams::<Team>().first(50).send().unwrap();
        conn.nodes
            .iter()
            .find(|t| t.key.as_deref() == Some(&key))
            .unwrap_or_else(|| panic!("LINEAR_TEST_TEAM_KEY={key} not found in workspace"))
            .id
            .clone()
            .unwrap()
    } else {
        let conn = client.teams::<Team>().first(1).send().unwrap();
        conn.nodes[0].id.clone().unwrap()
    }
}

/// RAII guard — permanently deletes an issue on drop.
struct IssueGuard {
    token: String,
    id: String,
}

impl Drop for IssueGuard {
    fn drop(&mut self) {
        let Ok(client) = Client::from_token(self.token.clone()) else {
            return;
        };
        let _ = client.issue_delete::<Issue>(Some(true), self.id.clone());
    }
}

/// RAII guard — permanently deletes a document on drop.
struct DocumentGuard {
    token: String,
    id: String,
}

impl Drop for DocumentGuard {
    fn drop(&mut self) {
        let Ok(client) = Client::from_token(self.token.clone()) else {
            return;
        };
        let _ = client.document_delete::<Document>(self.id.clone());
    }
}

test_with::runner!(blocking_online);

#[test_with::module]
mod blocking_online {
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
        let team_id = test_team_id_blocking(&client);

        // Create.
        let input = DocumentCreateInput {
            title: Some("[test] blocking document CRUD".to_string()),
            content: Some("Blocking API test.".to_string()),
            team_id: Some(team_id),
            ..Default::default()
        };
        let entity = client.document_create::<Document>(input).unwrap();
        let doc_id = entity.id.clone().unwrap();
        let _doc_guard = DocumentGuard {
            token: test_token(),
            id: doc_id.clone(),
        };

        // Read.
        let doc = client.document(doc_id.clone()).unwrap();
        assert_eq!(doc.id, Some(doc_id.clone()));

        // Delete.
        let _del = client.document_delete::<Document>(doc_id).unwrap();
    }

    // ── Issue Relations ─────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn blocking_issue_relations_list() {
        let client = test_client();
        let conn = client.issue_relations().first(5).send().unwrap();
        // Just verify deserialization works — may be empty.
        assert!(conn.page_info.has_next_page || !conn.page_info.has_next_page);
    }

    // ── Search ────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn blocking_search_issues() {
        let client = test_client();
        let conn = client.search_issues("test").first(5).send().unwrap();
        // Just verify deserialization works — results may be empty.
        for result in &conn.nodes {
            assert!(result.id.is_some());
        }
    }

    // ── Mutations ───────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn blocking_issue_create_and_delete() {
        use lineark_sdk::generated::inputs::IssueCreateInput;

        let client = test_client();
        let team_id = test_team_id_blocking(&client);

        let input = IssueCreateInput {
            title: Some("[test] blocking issue_create".to_string()),
            team_id: Some(team_id),
            priority: Some(4),
            ..Default::default()
        };
        let entity = client.issue_create::<Issue>(input).unwrap();
        let issue_id = entity.id.clone().unwrap();
        let _issue_guard = IssueGuard {
            token: test_token(),
            id: issue_id.clone(),
        };

        let _del = client.issue_delete::<Issue>(Some(true), issue_id).unwrap();
    }

    // ── Archive / Unarchive ────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn blocking_issue_archive_and_unarchive() {
        use lineark_sdk::generated::inputs::IssueCreateInput;

        let client = test_client();
        let team_id = test_team_id_blocking(&client);

        let input = IssueCreateInput {
            title: Some("[test] blocking archive/unarchive".to_string()),
            team_id: Some(team_id),
            priority: Some(4),
            ..Default::default()
        };
        let entity = client.issue_create::<Issue>(input).unwrap();
        let issue_id = entity.id.clone().unwrap();
        let _issue_guard = IssueGuard {
            token: test_token(),
            id: issue_id.clone(),
        };

        // Archive.
        let arch = client
            .issue_archive::<Issue>(None, issue_id.clone())
            .unwrap();
        assert!(
            arch.archived_at.is_some(),
            "archivedAt should be set after archiving"
        );

        // Unarchive.
        let unarch = client.issue_unarchive::<Issue>(issue_id.clone()).unwrap();
        assert!(
            unarch.archived_at.is_none(),
            "archivedAt should be null after unarchiving"
        );

        // Clean up.
        let _ = client.issue_delete::<Issue>(Some(true), issue_id).unwrap();
    }

    // ── File Upload ─────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn blocking_file_upload_returns_signed_url() {
        let client = test_client();
        let entity = client
            .file_upload(
                None,
                None,
                50,
                "text/plain".to_string(),
                "blocking-test.txt".to_string(),
            )
            .unwrap();
        // Non-generic mutation returns the full payload; entity is under "uploadFile".
        assert!(
            entity["uploadFile"]["uploadUrl"].as_str().is_some(),
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
