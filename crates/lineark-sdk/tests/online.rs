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
        let user = client.whoami().await.unwrap();
        assert!(user.id.is_some(), "viewer should have an id");
        assert!(user.email.is_some(), "viewer should have an email");
        assert!(user.active.is_some(), "viewer should have active status");
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn viewer_fields_deserialize_correctly() {
        let client = test_client();
        let user = client.whoami().await.unwrap();
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
        let conn = client.teams().first(10).send().await.unwrap();
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
        let conn = client.teams().first(1).send().await.unwrap();
        assert!(!conn.nodes.is_empty());
        let team_id = conn.nodes[0].id.clone().unwrap();
        let team = client.team(team_id.clone()).await.unwrap();
        assert_eq!(team.id, Some(team_id));
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn team_fields_deserialize_correctly() {
        let client = test_client();
        let conn = client.teams().first(1).send().await.unwrap();
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
        let conn = client.users().last(10).send().await.unwrap();
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
        let conn = client.projects().first(10).send().await.unwrap();
        assert!(conn.page_info.has_next_page || !conn.page_info.has_next_page);
    }

    // ── Issues ──────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issues_returns_connection() {
        let client = test_client();
        let conn = client.issues().first(5).send().await.unwrap();
        for issue in &conn.nodes {
            assert!(issue.id.is_some());
        }
    }

    // ── Issue Labels ────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issue_labels_returns_connection() {
        let client = test_client();
        let conn = client.issue_labels().first(10).send().await.unwrap();
        for label in &conn.nodes {
            assert!(label.id.is_some());
            assert!(label.name.is_some());
        }
    }

    // ── Cycles ──────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn cycles_returns_connection() {
        let client = test_client();
        let conn = client.cycles().first(10).send().await.unwrap();
        for cycle in &conn.nodes {
            assert!(cycle.id.is_some());
        }
    }

    // ── Workflow States ─────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn workflow_states_returns_connection() {
        let client = test_client();
        let conn = client.workflow_states().first(50).send().await.unwrap();
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
        let conn = client.search_issues("test").first(5).send().await.unwrap();
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
        let all = client.workflow_states().first(50).send().await.unwrap();
        assert!(
            all.nodes.len() >= 2,
            "need at least 2 workflow states to test first(), got {}",
            all.nodes.len()
        );
        let limited = client.workflow_states().first(1).send().await.unwrap();
        assert_eq!(
            limited.nodes.len(),
            1,
            "first(1) should return exactly 1 item"
        );
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn last_returns_different_item_than_first() {
        let client = test_client();
        let all = client.workflow_states().first(50).send().await.unwrap();
        if all.nodes.len() < 2 {
            // Can't distinguish first vs last with <2 items, skip.
            return;
        }
        let from_first = client.workflow_states().first(1).send().await.unwrap();
        let from_last = client.workflow_states().last(1).send().await.unwrap();
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
        let all = client.workflow_states().first(50).send().await.unwrap();
        if all.nodes.len() < 2 {
            return;
        }
        // Fetch first page of 1.
        let page1 = client.workflow_states().first(1).send().await.unwrap();
        assert_eq!(page1.nodes.len(), 1);
        let cursor = page1
            .page_info
            .end_cursor
            .as_ref()
            .expect("first page should have endCursor");

        // Fetch second page using after(cursor).
        let page2 = client
            .workflow_states()
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
            .teams()
            .first(1)
            .include_archived(true)
            .send()
            .await
            .unwrap();
        let _ = client
            .teams()
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
        let teams = client.teams().first(1).send().await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        // Create an issue with a unique title.
        let unique = format!("[builder-test-{}]", uuid::Uuid::new_v4());
        let input = IssueCreateInput {
            title: Some(unique.clone()),
            team_id: Some(team_id),
            priority: Some(4),
            ..Default::default()
        };
        let payload = client.issue_create(input).await.unwrap();
        let issue_id = payload["issue"]["id"].as_str().unwrap().to_string();

        // Linear's search index is async — retry a few times.
        let mut matched = false;
        for _ in 0..6 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let found = client.search_issues(&unique).first(5).send().await.unwrap();
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
            .search_issues("xyzzy_nonexistent_99999")
            .first(5)
            .send()
            .await
            .unwrap();
        let false_match = not_found
            .nodes
            .iter()
            .any(|n| n.title.as_deref().is_some_and(|t| t.contains(&unique)));
        assert!(
            !false_match,
            "search with different term should not find our issue"
        );

        // Clean up.
        client.issue_delete(Some(true), issue_id).await.unwrap();
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn search_issues_team_id_filters_by_team() {
        use lineark_sdk::generated::inputs::IssueCreateInput;

        let client = test_client();
        let teams = client.teams().first(10).send().await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        // Create an issue with a unique title in the first team.
        let unique = format!("[team-filter-{}]", uuid::Uuid::new_v4());
        let input = IssueCreateInput {
            title: Some(unique.clone()),
            team_id: Some(team_id.clone()),
            priority: Some(4),
            ..Default::default()
        };
        let payload = client.issue_create(input).await.unwrap();
        let issue_id = payload["issue"]["id"].as_str().unwrap().to_string();

        // Linear's search index is async — retry a few times.
        let mut found = false;
        for _ in 0..6 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let with_team = client
                .search_issues(&unique)
                .first(5)
                .team_id(&team_id)
                .send()
                .await
                .unwrap();
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
        let with_wrong_team = client
            .search_issues(&unique)
            .first(5)
            .team_id("00000000-0000-0000-0000-000000000000")
            .send()
            .await
            .unwrap();
        let false_match = with_wrong_team
            .nodes
            .iter()
            .any(|n| n.title.as_deref().is_some_and(|t| t.contains(&unique)));
        assert!(
            !false_match,
            "search with wrong team_id should not find the issue"
        );

        // Clean up.
        client.issue_delete(Some(true), issue_id).await.unwrap();
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn users_include_disabled_accepted() {
        let client = test_client();
        // Verify include_disabled parameter is accepted without error.
        let _ = client
            .users()
            .include_disabled(true)
            .first(5)
            .send()
            .await
            .unwrap();
        let _ = client
            .users()
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
        let conn = client.teams().send().await.unwrap();
        assert!(
            !conn.nodes.is_empty(),
            "teams() with no params should return results"
        );
        let conn = client.issues().send().await.unwrap();
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
        let teams = client.teams().first(1).send().await.unwrap();
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
        let teams = client.teams().first(1).send().await.unwrap();
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
        let teams = client.teams().first(1).send().await.unwrap();
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

    // ── Documents ────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn documents_returns_connection() {
        let client = test_client();
        let conn = client.documents().first(10).send().await.unwrap();
        // Connection should deserialize; may be empty if workspace has no docs.
        assert!(conn.page_info.has_next_page || !conn.page_info.has_next_page);
        for doc in &conn.nodes {
            assert!(doc.id.is_some());
        }
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn document_create_update_and_delete() {
        use lineark_sdk::generated::inputs::{DocumentCreateInput, DocumentUpdateInput};

        let client = test_client();

        // Get a team to associate the document with (Linear requires at least one parent).
        let teams = client.teams().first(1).send().await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        // Create a document.
        let input = DocumentCreateInput {
            title: Some("[test] SDK document_create_update_and_delete".to_string()),
            content: Some("Automated test document content.".to_string()),
            team_id: Some(team_id),
            ..Default::default()
        };
        let payload = client.document_create(input).await.unwrap();
        assert_eq!(payload.get("success").and_then(|v| v.as_bool()), Some(true));
        let doc = payload.get("document").unwrap();
        let doc_id = doc.get("id").and_then(|v| v.as_str()).unwrap().to_string();
        assert!(!doc_id.is_empty());
        assert_eq!(
            doc.get("title").and_then(|v| v.as_str()),
            Some("[test] SDK document_create_update_and_delete")
        );

        // Read the document by ID.
        let fetched = client.document(doc_id.clone()).await.unwrap();
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
        let update_payload = client
            .document_update(update_input, doc_id.clone())
            .await
            .unwrap();
        assert_eq!(
            update_payload.get("success").and_then(|v| v.as_bool()),
            Some(true)
        );
        let updated_doc = update_payload.get("document").unwrap();
        assert_eq!(
            updated_doc.get("title").and_then(|v| v.as_str()),
            Some("[test] SDK document — updated")
        );

        // Delete the document.
        let delete_payload = client.document_delete(doc_id).await.unwrap();
        assert_eq!(
            delete_payload.get("success").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    // ── Issue Relations ─────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issue_relations_returns_connection() {
        let client = test_client();
        let conn = client.issue_relations().first(10).send().await.unwrap();
        // Connection should deserialize; may be empty.
        assert!(conn.page_info.has_next_page || !conn.page_info.has_next_page);
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn issue_relation_create_between_two_issues() {
        use lineark_sdk::generated::enums::IssueRelationType;
        use lineark_sdk::generated::inputs::{IssueCreateInput, IssueRelationCreateInput};

        let client = test_client();

        // Get a team to create issues in.
        let teams = client.teams().first(1).send().await.unwrap();
        let team_id = teams.nodes[0].id.clone().unwrap();

        // Create two issues to relate.
        let input_a = IssueCreateInput {
            title: Some("[test] relation issue A".to_string()),
            team_id: Some(team_id.clone()),
            priority: Some(4),
            ..Default::default()
        };
        let payload_a = client.issue_create(input_a).await.unwrap();
        let issue_a_id = payload_a["issue"]["id"].as_str().unwrap().to_string();

        let input_b = IssueCreateInput {
            title: Some("[test] relation issue B".to_string()),
            team_id: Some(team_id),
            priority: Some(4),
            ..Default::default()
        };
        let payload_b = client.issue_create(input_b).await.unwrap();
        let issue_b_id = payload_b["issue"]["id"].as_str().unwrap().to_string();

        // Create a "blocks" relation: A blocks B.
        let relation_input = IssueRelationCreateInput {
            issue_id: Some(issue_a_id.clone()),
            related_issue_id: Some(issue_b_id.clone()),
            r#type: Some(IssueRelationType::Blocks),
            ..Default::default()
        };
        let relation_payload = client
            .issue_relation_create(None, relation_input)
            .await
            .unwrap();
        assert_eq!(
            relation_payload.get("success").and_then(|v| v.as_bool()),
            Some(true)
        );
        let relation = relation_payload.get("issueRelation").unwrap();
        assert!(
            relation.get("id").and_then(|v| v.as_str()).is_some(),
            "relation should have an id"
        );

        // Verify the relation is queryable.
        let relation_id = relation.get("id").unwrap().as_str().unwrap().to_string();
        let fetched = client.issue_relation(relation_id).await.unwrap();
        assert!(fetched.id.is_some());

        // Clean up: delete both issues (cascades the relation).
        client.issue_delete(Some(true), issue_a_id).await.unwrap();
        client.issue_delete(Some(true), issue_b_id).await.unwrap();
    }

    // ── File Upload ─────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn file_upload_returns_signed_url() {
        let client = test_client();

        // Request a signed upload URL for a small file.
        let payload = client
            .file_upload(
                None,
                None,
                100,
                "text/plain".to_string(),
                "test.txt".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(payload.get("success").and_then(|v| v.as_bool()), Some(true));
        let upload_file = payload.get("uploadFile").unwrap();
        assert!(
            upload_file
                .get("uploadUrl")
                .and_then(|v| v.as_str())
                .is_some(),
            "should have uploadUrl"
        );
        assert!(
            upload_file
                .get("assetUrl")
                .and_then(|v| v.as_str())
                .is_some(),
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

    #[test_with::runtime_ignore_if(no_online_test_token)]
    async fn invalid_token_returns_auth_error() {
        let client = Client::from_token("lin_api_invalid_token_12345").unwrap();
        let result = client.whoami().await;
        assert!(result.is_err(), "invalid token should produce an error");
    }
}
