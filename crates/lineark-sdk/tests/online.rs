//! Online integration tests for the lineark SDK against a real Linear workspace.
//!
//! These tests require a valid Linear API token at `~/.linear_api_token_test`.
//! When the token file is missing, tests are automatically skipped with a message.
//!
//! The token should be connected to a test workspace — never use production tokens here.

use lineark_sdk::generated::types::*;
use lineark_sdk::Client;

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
    Client::from_token(test_token()).expect("failed to create test client")
}

/// RAII guard — permanently deletes a team on drop.
/// Uses a dedicated thread+runtime since Drop can't be async.
struct TeamGuard {
    token: String,
    id: String,
}

impl Drop for TeamGuard {
    fn drop(&mut self) {
        let token = self.token.clone();
        let id = self.id.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                if let Ok(client) = Client::from_token(token) {
                    let _ = client.team_delete(id).await;
                }
            });
        })
        .join();
    }
}

/// RAII guard — permanently deletes an issue on drop.
struct IssueGuard {
    token: String,
    id: String,
}

impl Drop for IssueGuard {
    fn drop(&mut self) {
        let token = self.token.clone();
        let id = self.id.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                if let Ok(client) = Client::from_token(token) {
                    let _ = client.issue_delete::<Issue>(Some(true), id).await;
                }
            });
        })
        .join();
    }
}

/// RAII guard — permanently deletes a document on drop.
struct DocumentGuard {
    token: String,
    id: String,
}

impl Drop for DocumentGuard {
    fn drop(&mut self) {
        let token = self.token.clone();
        let id = self.id.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                if let Ok(client) = Client::from_token(token) {
                    let _ = client.document_delete::<Document>(id).await;
                }
            });
        })
        .join();
    }
}

test_with::tokio_runner!(online);

#[test_with::module]
mod online {
    // ── Viewer ──────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn viewer_returns_authenticated_user() {
        let client = test_client();
        let user = client.whoami::<User>().await.unwrap();
        assert!(user.id.is_some(), "viewer should have an id");
        assert!(user.email.is_some(), "viewer should have an email");
        assert!(user.active.is_some(), "viewer should have active status");
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn viewer_fields_deserialize_correctly() {
        let client = test_client();
        let user = client.whoami::<User>().await.unwrap();
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
        let conn = client.teams::<Team>().first(10).send().await.unwrap();
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
        let conn = client.teams::<Team>().first(1).send().await.unwrap();
        assert!(!conn.nodes.is_empty());
        let team_id = conn.nodes[0].id.clone().unwrap();
        let team = client.team::<Team>(team_id.clone()).await.unwrap();
        assert_eq!(team.id, Some(team_id));
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn team_fields_deserialize_correctly() {
        let client = test_client();
        let conn = client.teams::<Team>().first(1).send().await.unwrap();
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
        let conn = client.users::<User>().last(10).send().await.unwrap();
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
        let conn = client.projects::<Project>().first(10).send().await.unwrap();
        // Connection should deserialize; verify page_info is present.
        let _ = conn.page_info;
    }

    // ── Issues ──────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issues_returns_connection() {
        let client = test_client();
        let conn = client.issues::<Issue>().first(5).send().await.unwrap();
        for issue in &conn.nodes {
            assert!(issue.id.is_some());
        }
    }

    // ── Issue Labels ────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issue_labels_returns_connection() {
        let client = test_client();
        let conn = client
            .issue_labels::<IssueLabel>()
            .first(10)
            .send()
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
        let conn = client.cycles::<Cycle>().first(10).send().await.unwrap();
        for cycle in &conn.nodes {
            assert!(cycle.id.is_some());
        }
    }

    // ── Workflow States ─────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn workflow_states_returns_connection() {
        let client = test_client();
        let conn = client
            .workflow_states::<WorkflowState>()
            .first(50)
            .send()
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
            .search_issues::<IssueSearchResult>("test")
            .first(5)
            .send()
            .await
            .unwrap();
        for issue in &conn.nodes {
            assert!(issue.id.is_some());
        }
    }

    // ── Builder parameter stress tests ─────────────────────────────────────
    // These verify that each builder setter actually affects the API response.

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn first_limits_result_count() {
        let client = test_client();
        // Workflow states typically have 5+ (Triage, Backlog, Todo, In Progress, Done, Canceled).
        let all = client
            .workflow_states::<WorkflowState>()
            .first(50)
            .send()
            .await
            .unwrap();
        assert!(
            all.nodes.len() >= 2,
            "need at least 2 workflow states to test first(), got {}",
            all.nodes.len()
        );
        let limited = client
            .workflow_states::<WorkflowState>()
            .first(1)
            .send()
            .await
            .unwrap();
        assert_eq!(
            limited.nodes.len(),
            1,
            "first(1) should return exactly 1 item"
        );
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn last_returns_different_item_than_first() {
        let client = test_client();
        let all = client
            .workflow_states::<WorkflowState>()
            .first(50)
            .send()
            .await
            .unwrap();
        if all.nodes.len() < 2 {
            // Can't distinguish first vs last with <2 items, skip.
            return;
        }
        let from_first = client
            .workflow_states::<WorkflowState>()
            .first(1)
            .send()
            .await
            .unwrap();
        let from_last = client
            .workflow_states::<WorkflowState>()
            .last(1)
            .send()
            .await
            .unwrap();
        assert_eq!(from_first.nodes.len(), 1);
        assert_eq!(from_last.nodes.len(), 1);
        // first(1) and last(1) should be different items (first vs last of the list).
        assert_ne!(
            from_first.nodes[0].id, from_last.nodes[0].id,
            "first(1) and last(1) should return different workflow states"
        );
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn after_cursor_paginates_to_next_page() {
        let client = test_client();
        let all = client
            .workflow_states::<WorkflowState>()
            .first(50)
            .send()
            .await
            .unwrap();
        if all.nodes.len() < 2 {
            return;
        }
        // Fetch first page of 1.
        let page1 = client
            .workflow_states::<WorkflowState>()
            .first(1)
            .send()
            .await
            .unwrap();
        assert_eq!(page1.nodes.len(), 1);
        let cursor = page1
            .page_info
            .end_cursor
            .as_ref()
            .expect("first page should have endCursor");

        // Fetch second page using after(cursor).
        let page2 = client
            .workflow_states::<WorkflowState>()
            .first(1)
            .after(cursor)
            .send()
            .await
            .unwrap();
        assert_eq!(page2.nodes.len(), 1);
        assert_ne!(
            page1.nodes[0].id, page2.nodes[0].id,
            "after(cursor) should return a different item than page 1"
        );
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn include_archived_does_not_error() {
        let client = test_client();
        // Just verify the parameter is accepted by the API without error.
        let _ = client
            .teams::<Team>()
            .first(1)
            .include_archived(true)
            .send()
            .await
            .unwrap();
        let _ = client
            .teams::<Team>()
            .first(1)
            .include_archived(false)
            .send()
            .await
            .unwrap();
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn search_issues_term_filters_results() {
        use lineark_sdk::generated::inputs::IssueCreateInput;

        let client = test_client();
        let teams = client.teams::<Team>().first(1).send().await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        // Create an issue with a unique title.
        let unique = format!("[builder-test-{}]", uuid::Uuid::new_v4());
        let input = IssueCreateInput {
            title: Some(unique.clone()),
            team_id: Some(team_id),
            priority: Some(4),
            ..Default::default()
        };
        let entity = client.issue_create::<Issue>(input).await.unwrap();
        let issue_id = entity.id.clone().unwrap();
        let _issue_guard = IssueGuard {
            token: test_token(),
            id: issue_id.clone(),
        };

        // Linear's search index is async — retry with backoff.
        let mut matched = false;
        for i in 0..8 {
            tokio::time::sleep(std::time::Duration::from_secs(if i < 3 { 1 } else { 3 })).await;
            let found = match client
                .search_issues::<IssueSearchResult>(&unique)
                .first(5)
                .send()
                .await
            {
                Ok(v) => v,
                Err(_) => continue, // rate-limited or transient error — retry
            };
            matched = found
                .nodes
                .iter()
                .any(|n| n.title.as_deref().is_some_and(|t| t.contains(&unique)));
            if matched {
                break;
            }
        }
        assert!(matched, "search_issues(term) should find the created issue");

        // Search for nonsense — should NOT find it.
        let not_found = client
            .search_issues::<IssueSearchResult>("xyzzy_nonexistent_99999")
            .first(5)
            .send()
            .await
            .expect("nonsense search should not be rate-limited");
        let false_match = not_found
            .nodes
            .iter()
            .any(|n| n.title.as_deref().is_some_and(|t| t.contains(&unique)));
        assert!(
            !false_match,
            "search with different term should not find our issue"
        );

        // Clean up.
        client
            .issue_delete::<Issue>(Some(true), issue_id)
            .await
            .unwrap();
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn search_issues_team_id_filters_by_team() {
        use lineark_sdk::generated::inputs::IssueCreateInput;

        let client = test_client();
        let teams = client.teams::<Team>().first(10).send().await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        // Create an issue with a unique title in the first team.
        let unique = format!("[team-filter-{}]", uuid::Uuid::new_v4());
        let input = IssueCreateInput {
            title: Some(unique.clone()),
            team_id: Some(team_id.clone()),
            priority: Some(4),
            ..Default::default()
        };
        let entity = client.issue_create::<Issue>(input).await.unwrap();
        let issue_id = entity.id.clone().unwrap();
        let _issue_guard = IssueGuard {
            token: test_token(),
            id: issue_id.clone(),
        };

        // Linear's search index is async — retry a few times.
        let mut found = false;
        for _ in 0..8 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let with_team = match client
                .search_issues::<IssueSearchResult>(&unique)
                .first(5)
                .team_id(&team_id)
                .send()
                .await
            {
                Ok(v) => v,
                Err(_) => continue, // rate-limited or transient error — retry
            };
            found = with_team
                .nodes
                .iter()
                .any(|n| n.title.as_deref().is_some_and(|t| t.contains(&unique)));
            if found {
                break;
            }
        }
        assert!(found, "search with correct team_id should find the issue");

        // Search with a fake team_id — should NOT find it.
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let with_wrong_team = client
            .search_issues::<IssueSearchResult>(&unique)
            .first(5)
            .team_id("00000000-0000-0000-0000-000000000000")
            .send()
            .await
            .expect("wrong-team search should not be rate-limited");
        let false_match = with_wrong_team
            .nodes
            .iter()
            .any(|n| n.title.as_deref().is_some_and(|t| t.contains(&unique)));
        assert!(
            !false_match,
            "search with wrong team_id should not find the issue"
        );

        // Clean up.
        client
            .issue_delete::<Issue>(Some(true), issue_id)
            .await
            .unwrap();
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn users_include_disabled_accepted() {
        let client = test_client();
        // Verify include_disabled parameter is accepted without error.
        let _ = client
            .users::<User>()
            .include_disabled(true)
            .first(5)
            .send()
            .await
            .unwrap();
        let _ = client
            .users::<User>()
            .include_disabled(false)
            .first(5)
            .send()
            .await
            .unwrap();
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn no_params_returns_defaults() {
        let client = test_client();
        // Calling send() with no setters should work (all params null = API defaults).
        let conn = client.teams::<Team>().send().await.unwrap();
        assert!(
            !conn.nodes.is_empty(),
            "teams() with no params should return results"
        );
        let conn = client.issues::<Issue>().send().await.unwrap();
        // issues() with no filter returns all non-archived issues.
        // Just verify it doesn't error.
        for issue in &conn.nodes {
            assert!(issue.id.is_some());
        }
    }

    // ── Mutations ────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issue_create_and_delete() {
        use lineark_sdk::generated::inputs::IssueCreateInput;

        let client = test_client();

        // Get the first team to create an issue in.
        let teams = client.teams::<Team>().first(1).send().await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        // Create an issue.
        let input = IssueCreateInput {
            title: Some("[test] SDK issue_create_and_delete".to_string()),
            team_id: Some(team_id),
            description: Some("Automated test — will be deleted immediately.".to_string()),
            priority: Some(4), // Low
            ..Default::default()
        };
        let entity = client.issue_create::<Issue>(input).await.unwrap();
        let issue_id = entity.id.clone().unwrap();
        let _issue_guard = IssueGuard {
            token: test_token(),
            id: issue_id.clone(),
        };
        assert!(!issue_id.is_empty());

        // Permanently delete the issue to keep the workspace clean.
        client
            .issue_delete::<Issue>(Some(true), issue_id)
            .await
            .unwrap();
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issue_update() {
        use lineark_sdk::generated::inputs::{IssueCreateInput, IssueUpdateInput};

        let client = test_client();

        // Create an issue to update.
        let teams = client.teams::<Team>().first(1).send().await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        let input = IssueCreateInput {
            title: Some("[test] SDK issue_update".to_string()),
            team_id: Some(team_id),
            priority: Some(4),
            ..Default::default()
        };
        let entity = client.issue_create::<Issue>(input).await.unwrap();
        let issue_id = entity.id.clone().unwrap();
        let _issue_guard = IssueGuard {
            token: test_token(),
            id: issue_id.clone(),
        };

        // Update the issue.
        let update_input = IssueUpdateInput {
            title: Some("[test] SDK issue_update — updated".to_string()),
            priority: Some(3), // Medium
            ..Default::default()
        };
        let updated_entity = client
            .issue_update::<Issue>(update_input, issue_id.clone())
            .await
            .unwrap();
        // Verify the returned entity has an id.
        assert!(updated_entity.id.is_some());

        // Clean up: permanently delete.
        client
            .issue_delete::<Issue>(Some(true), issue_id)
            .await
            .unwrap();
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issue_archive_and_unarchive() {
        use lineark_sdk::generated::inputs::IssueCreateInput;

        let client = test_client();

        // Create an issue to archive.
        let teams = client.teams::<Team>().first(1).send().await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        let input = IssueCreateInput {
            title: Some("[test] SDK issue_archive_and_unarchive".to_string()),
            team_id: Some(team_id),
            priority: Some(4),
            ..Default::default()
        };
        let entity = client.issue_create::<Issue>(input).await.unwrap();
        let issue_id = entity.id.clone().unwrap();
        let _issue_guard = IssueGuard {
            token: test_token(),
            id: issue_id.clone(),
        };

        // Archive the issue — success is verified by not returning an error.
        client
            .issue_archive::<Issue>(None, issue_id.clone())
            .await
            .unwrap();

        // Unarchive the issue.
        client
            .issue_unarchive::<Issue>(issue_id.clone())
            .await
            .unwrap();

        // Clean up: permanently delete.
        client
            .issue_delete::<Issue>(Some(true), issue_id)
            .await
            .unwrap();
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn comment_create() {
        use lineark_sdk::generated::inputs::{CommentCreateInput, IssueCreateInput};

        let client = test_client();

        // Create an issue to comment on.
        let teams = client.teams::<Team>().first(1).send().await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        let issue_input = IssueCreateInput {
            title: Some("[test] SDK comment_create".to_string()),
            team_id: Some(team_id),
            priority: Some(4),
            ..Default::default()
        };
        let issue_entity = client.issue_create::<Issue>(issue_input).await.unwrap();
        let issue_id = issue_entity.id.clone().unwrap();
        let _issue_guard = IssueGuard {
            token: test_token(),
            id: issue_id.clone(),
        };

        // Create a comment.
        let comment_input = CommentCreateInput {
            body: Some("Automated test comment from lineark SDK.".to_string()),
            issue_id: Some(issue_id.clone()),
            ..Default::default()
        };
        let comment_entity = client
            .comment_create::<Comment>(comment_input)
            .await
            .unwrap();
        assert!(comment_entity.id.is_some());

        // Clean up: permanently delete the issue.
        client
            .issue_delete::<Issue>(Some(true), issue_id)
            .await
            .unwrap();
    }

    // ── Documents ────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn documents_returns_connection() {
        let client = test_client();
        let conn = client
            .documents::<Document>()
            .first(10)
            .send()
            .await
            .unwrap();
        // Connection should deserialize; may be empty if workspace has no docs.
        let _ = conn.page_info;
        for doc in &conn.nodes {
            assert!(doc.id.is_some());
        }
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn document_create_update_and_delete() {
        use lineark_sdk::generated::inputs::{DocumentCreateInput, DocumentUpdateInput};

        let client = test_client();

        // Get a team to associate the document with (Linear requires at least one parent).
        let teams = client.teams::<Team>().first(1).send().await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        // Create a document.
        let input = DocumentCreateInput {
            title: Some("[test] SDK document_create_update_and_delete".to_string()),
            content: Some("Automated test document content.".to_string()),
            team_id: Some(team_id),
            ..Default::default()
        };
        let doc_entity = client.document_create::<Document>(input).await.unwrap();
        let doc_id = doc_entity.id.clone().unwrap();
        let _doc_guard = DocumentGuard {
            token: test_token(),
            id: doc_id.clone(),
        };
        assert!(!doc_id.is_empty());

        // Read the document by ID.
        let fetched = client.document::<Document>(doc_id.clone()).await.unwrap();
        assert_eq!(fetched.id, Some(doc_id.clone()));
        assert_eq!(
            fetched.title,
            Some("[test] SDK document_create_update_and_delete".to_string())
        );

        // Update the document.
        let update_input = DocumentUpdateInput {
            title: Some("[test] SDK document — updated".to_string()),
            content: Some("Updated content.".to_string()),
            ..Default::default()
        };
        // Just verify the update succeeded.
        client
            .document_update::<Document>(update_input, doc_id.clone())
            .await
            .unwrap();

        // Delete the document.
        client.document_delete::<Document>(doc_id).await.unwrap();
    }

    // ── Issue Relations ─────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issue_relations_returns_connection() {
        let client = test_client();
        let conn = client
            .issue_relations::<IssueRelation>()
            .first(10)
            .send()
            .await
            .unwrap();
        // Connection should deserialize; may be empty.
        let _ = conn.page_info;
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issue_relation_create_between_two_issues() {
        use lineark_sdk::generated::enums::IssueRelationType;
        use lineark_sdk::generated::inputs::{IssueCreateInput, IssueRelationCreateInput};

        let client = test_client();

        // Get a team to create issues in.
        let teams = client.teams::<Team>().first(1).send().await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        // Create two issues to relate.
        let input_a = IssueCreateInput {
            title: Some("[test] relation issue A".to_string()),
            team_id: Some(team_id.clone()),
            priority: Some(4),
            ..Default::default()
        };
        let entity_a = client.issue_create::<Issue>(input_a).await.unwrap();
        let issue_a_id = entity_a.id.clone().unwrap();
        let _issue_a_guard = IssueGuard {
            token: test_token(),
            id: issue_a_id.clone(),
        };

        let input_b = IssueCreateInput {
            title: Some("[test] relation issue B".to_string()),
            team_id: Some(team_id),
            priority: Some(4),
            ..Default::default()
        };
        let entity_b = client.issue_create::<Issue>(input_b).await.unwrap();
        let issue_b_id = entity_b.id.clone().unwrap();
        let _issue_b_guard = IssueGuard {
            token: test_token(),
            id: issue_b_id.clone(),
        };

        // Create a "blocks" relation: A blocks B.
        let relation_input = IssueRelationCreateInput {
            issue_id: Some(issue_a_id.clone()),
            related_issue_id: Some(issue_b_id.clone()),
            r#type: Some(IssueRelationType::Blocks),
            ..Default::default()
        };
        let relation_entity = client
            .issue_relation_create::<IssueRelation>(None, relation_input)
            .await
            .unwrap();
        assert!(relation_entity.id.is_some(), "relation should have an id");

        // Verify the relation is queryable.
        let relation_id = relation_entity.id.clone().unwrap();
        let fetched = client
            .issue_relation::<IssueRelation>(relation_id)
            .await
            .unwrap();
        assert!(fetched.id.is_some());

        // Clean up: delete both issues (cascades the relation).
        client
            .issue_delete::<Issue>(Some(true), issue_a_id)
            .await
            .unwrap();
        client
            .issue_delete::<Issue>(Some(true), issue_b_id)
            .await
            .unwrap();
    }

    // ── File Upload ─────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn file_upload_returns_signed_url() {
        let client = test_client();

        // Request a signed upload URL for a small file.
        let entity = client
            .file_upload(
                None,
                None,
                100,
                "text/plain".to_string(),
                "test.txt".to_string(),
            )
            .await
            .unwrap();

        // Non-generic mutation returns the full payload; entity is under "uploadFile".
        let upload_file = entity.get("uploadFile").expect("should have uploadFile");
        assert!(
            upload_file.get("uploadUrl").is_some(),
            "should have uploadUrl"
        );
        assert!(
            upload_file.get("assetUrl").is_some(),
            "should have assetUrl"
        );
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn upload_file_end_to_end() {
        let client = test_client();

        // Upload a small text file.
        let content = b"lineark SDK test upload content".to_vec();
        let result = client
            .upload_file("test-upload.txt", "text/plain", content, false)
            .await
            .unwrap();

        assert!(
            !result.asset_url.is_empty(),
            "asset_url should be non-empty"
        );
        assert!(
            result.asset_url.starts_with("https://"),
            "asset_url should be an HTTPS URL, got: {}",
            result.asset_url
        );
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn upload_then_download_round_trip() {
        let client = test_client();

        // Upload a file with known content.
        let content = b"SDK round-trip test content 12345".to_vec();
        let upload_result = client
            .upload_file("round-trip.txt", "text/plain", content.clone(), false)
            .await
            .unwrap();
        assert!(!upload_result.asset_url.is_empty());

        // Download it back via download_url.
        let download_result = client.download_url(&upload_result.asset_url).await.unwrap();
        assert_eq!(
            download_result.bytes, content,
            "downloaded bytes should match uploaded content"
        );
        assert!(
            download_result
                .content_type
                .as_deref()
                .is_some_and(|ct| ct.contains("text/plain")),
            "content type should be text/plain, got: {:?}",
            download_result.content_type
        );
    }

    // ── Error handling ──────────────────────────────────────────────────────

    // ── Team CRUD ────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn team_create_update_and_delete() {
        use lineark_sdk::generated::inputs::{TeamCreateInput, TeamUpdateInput};

        let client = test_client();

        // Create a team with a unique name.
        let unique = format!("[test] sdk-team {}", &uuid::Uuid::new_v4().to_string()[..8]);
        let input = TeamCreateInput {
            name: Some(unique.clone()),
            ..Default::default()
        };
        let team = client.team_create::<Team>(None, input).await.unwrap();
        let team_id = team.id.clone().unwrap();
        let _team_guard = TeamGuard {
            token: test_token(),
            id: team_id.clone(),
        };
        assert!(!team_id.is_empty());
        assert_eq!(team.name, Some(unique));

        // Update the team's description.
        let update_input = TeamUpdateInput {
            description: Some("Updated by SDK test.".to_string()),
            ..Default::default()
        };
        let updated = client
            .team_update::<Team>(None, update_input, team_id.clone())
            .await
            .unwrap();
        assert!(updated.id.is_some());

        // Verify the update by reading the team.
        let fetched = client.team::<Team>(team_id.clone()).await.unwrap();
        assert_eq!(
            fetched.description,
            Some("Updated by SDK test.".to_string())
        );

        // Delete the team.
        client.team_delete(team_id).await.unwrap();
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn team_membership_create_and_delete() {
        use lineark_sdk::generated::inputs::{TeamCreateInput, TeamMembershipCreateInput};

        let client = test_client();

        // Create a team (the authenticated user becomes creator + auto-member).
        let unique = format!(
            "[test] sdk-member {}",
            &uuid::Uuid::new_v4().to_string()[..8]
        );
        let input = TeamCreateInput {
            name: Some(unique),
            ..Default::default()
        };
        let team = client.team_create::<Team>(None, input).await.unwrap();
        let team_id = team.id.clone().unwrap();
        let _team_guard = TeamGuard {
            token: test_token(),
            id: team_id.clone(),
        };

        // Discover a different user to add as a member.
        let viewer = client.whoami::<User>().await.unwrap();
        let my_id = viewer.id.clone().unwrap();
        let all_users = client.users::<User>().last(250).send().await.unwrap();
        let other_user = all_users
            .nodes
            .iter()
            .find(|u| u.id.as_deref() != Some(&my_id))
            .expect("workspace must have at least two users to run this test");
        let other_user_id = other_user.id.clone().unwrap();

        // Add the other user as a member — must succeed cleanly.
        let membership_input = TeamMembershipCreateInput {
            team_id: Some(team_id.clone()),
            user_id: Some(other_user_id),
            ..Default::default()
        };
        let membership = client
            .team_membership_create::<TeamMembership>(membership_input)
            .await
            .unwrap();
        let membership_id = membership.id.clone().unwrap();
        assert!(!membership_id.is_empty());

        // Delete the membership.
        client
            .team_membership_delete(None, membership_id)
            .await
            .unwrap();

        // Clean up: delete the team.
        client.team_delete(team_id).await.unwrap();
    }

    // ── Error handling ──────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn invalid_token_returns_auth_error() {
        let client = Client::from_token("lin_api_invalid_token_12345").unwrap();
        let result = client.whoami::<User>().await;
        assert!(result.is_err(), "invalid token should produce an error");
    }
}
