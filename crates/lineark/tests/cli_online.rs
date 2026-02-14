//! Online CLI tests for lineark — these hit the real Linear API.
//!
//! Requires a valid Linear API token at `~/.linear_api_token_test`.
//! When the token file is missing, tests are automatically skipped with a message.

use assert_cmd::Command;
use lineark_sdk::generated::types::{Issue, IssueRelation};
use lineark_sdk::Client;
use predicates::prelude::*;

fn no_online_test_token() -> Option<String> {
    let path = home::home_dir()?.join(".linear_api_token_test");
    if path.exists() {
        None
    } else {
        Some("~/.linear_api_token_test not found".to_string())
    }
}

fn api_token() -> String {
    let path = home::home_dir()
        .expect("could not determine home directory")
        .join(".linear_api_token_test");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {}", path.display(), e))
        .trim()
        .to_string()
}

fn lineark() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("lineark").unwrap()
}

/// Permanently delete an issue by its UUID to keep the workspace clean.
fn delete_issue(issue_id: &str) {
    let client = Client::from_token(api_token()).unwrap();
    let id = issue_id.to_string();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { client.issue_delete::<Issue>(Some(true), id).await.unwrap() });
}

test_with::runner!(cli_online);

#[test_with::module]
mod cli_online {
    // ── Whoami ────────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn whoami_json_returns_valid_json() {
        let token = api_token();
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "whoami"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "whoami should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        let arr = json.as_array().expect("whoami JSON should be an array");
        assert!(!arr.is_empty(), "whoami array should not be empty");
        assert!(arr[0].get("id").is_some(), "whoami entry should contain id");
        assert!(
            arr[0].get("email").is_some(),
            "whoami entry should contain email"
        );
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn whoami_human_shows_user_info() {
        let token = api_token();
        lineark()
            .args(["--api-token", &token, "--format", "human", "whoami"])
            .assert()
            .success()
            .stdout(predicate::str::contains("name"));
    }

    // ── Teams ─────────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn teams_list_json_returns_array() {
        let token = api_token();
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "teams list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        assert!(json.is_array(), "teams list JSON should be an array");
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn teams_list_human_shows_table() {
        let token = api_token();
        lineark()
            .args(["--api-token", &token, "--format", "human", "teams", "list"])
            .assert()
            .success()
            .stdout(predicate::str::contains("name"));
    }

    // ── Users ─────────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn users_list_json_returns_array() {
        let token = api_token();
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "users", "list"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "users list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        assert!(json.is_array(), "users list JSON should be an array");
    }

    // ── Labels ────────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn labels_list_json_returns_array() {
        let token = api_token();
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "labels", "list"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "labels list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        assert!(json.is_array(), "labels list JSON should be an array");
    }

    /// Regression: labels list must include team field (#65)
    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn labels_list_json_includes_team_field() {
        let token = api_token();
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "labels", "list"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "labels list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        let arr = json.as_array().expect("should be an array");

        if let Some(label) = arr.first() {
            assert!(
                label.get("team").is_some(),
                "each label should include a 'team' field"
            );
            // team should be a string (team key or empty for workspace-wide)
            assert!(
                label["team"].is_string(),
                "team should be a string, got: {}",
                label["team"]
            );
        }
    }

    // ── Issues ────────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_list_json_returns_array() {
        let token = api_token();
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "issues", "list"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "issues list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        let arr = json
            .as_array()
            .expect("issues list JSON should be an array");

        // Regression: relation fields must be populated, not null (#63)
        if let Some(issue) = arr.first() {
            assert!(
                issue.get("state").is_some_and(|v| !v.is_null()),
                "state should be populated"
            );
            assert!(
                issue.get("team").is_some_and(|v| !v.is_null()),
                "team should be populated"
            );
            // assignee can legitimately be null (unassigned issues), so just check the field exists
            assert!(
                issue.get("assignee").is_some(),
                "assignee field should be present"
            );
        }
    }

    /// Regression: issues list JSON must be flat — no nested objects (#65)
    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_list_json_is_flat() {
        let token = api_token();
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "list",
                "--limit",
                "1",
            ])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "issues list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        let arr = json.as_array().expect("should be an array");

        if let Some(issue) = arr.first() {
            // state, assignee, team must be flat strings, not nested objects
            assert!(
                issue["state"].is_string(),
                "state should be a flat string, got: {}",
                issue["state"]
            );
            assert!(
                issue["team"].is_string(),
                "team should be a flat string, got: {}",
                issue["team"]
            );
            // assignee can be "" for unassigned
            assert!(
                issue["assignee"].is_string(),
                "assignee should be a flat string, got: {}",
                issue["assignee"]
            );
            // id and priority (numeric) must be absent
            assert!(
                issue.get("id").is_none(),
                "id (UUID) should not be in list output"
            );
            assert!(
                issue.get("priority").is_none(),
                "priority (numeric) should not be in list output"
            );
            // url and priorityLabel must be present
            assert!(issue.get("url").is_some(), "url should be in list output");
            assert!(
                issue.get("priorityLabel").is_some(),
                "priorityLabel should be in list output"
            );
        }
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_search_json_returns_array() {
        let token = api_token();
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "search",
                "test",
            ])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "issues search should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        let arr = json
            .as_array()
            .expect("issues search JSON should be an array");

        // Regression: relation fields must be populated, not null (#63)
        if let Some(issue) = arr.first() {
            assert!(
                issue.get("state").is_some_and(|v| !v.is_null()),
                "state should be populated"
            );
            assert!(
                issue.get("team").is_some_and(|v| !v.is_null()),
                "team should be populated"
            );
            assert!(
                issue.get("assignee").is_some(),
                "assignee field should be present"
            );
        }
    }

    /// Regression: issues search JSON must be flat — same as list (#65)
    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_search_json_is_flat() {
        let token = api_token();
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "search",
                "test",
                "--limit",
                "1",
            ])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "issues search should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        let arr = json.as_array().expect("should be an array");

        if let Some(issue) = arr.first() {
            assert!(
                issue["state"].is_string(),
                "state should be a flat string in search output"
            );
            assert!(
                issue["team"].is_string(),
                "team should be a flat string in search output"
            );
            assert!(
                issue.get("id").is_none(),
                "id (UUID) should not be in search output"
            );
            assert!(
                issue.get("priority").is_none(),
                "priority (numeric) should not be in search output"
            );
        }
    }

    // ── Issues create + update + archive ─────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_create_update_and_archive() {
        let token = api_token();

        // Get the first team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .expect("failed to execute lineark");
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create an issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] CLI create+update",
                "--team",
                &team_key,
                "--priority",
                "4",
                "--description",
                "Automated CLI test — will be archived.",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let created: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let issue_id = created["id"]
            .as_str()
            .expect("created issue should have id (UUID)");

        // Update the issue (use UUID to avoid search index lag).
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "update",
                issue_id,
                "--priority",
                "2",
                "--title",
                "[test] CLI create+update — updated",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues update should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        // Mutation now returns only the entity with `id` (serde_json::Value
        // selects only `id`), so we just verify the command succeeded and
        // the output is valid JSON with an id.
        let updated: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            updated.get("id").is_some(),
            "update response should contain id"
        );

        // Clean up: permanently delete the issue.
        delete_issue(issue_id);
    }

    // ── Issues archive / unarchive ─────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_archive_and_unarchive_cycle() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create an issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] CLI archive/unarchive",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "issue creation should succeed");
        let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_id = created["id"]
            .as_str()
            .expect("created issue should have id (UUID)")
            .to_string();

        // Archive the issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "archive",
                &issue_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues archive should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        // Mutation now returns the entity directly (no payload wrapper).
        // Success is checked internally by the SDK; the CLI just outputs the entity.
        let archived: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            archived.get("id").is_some(),
            "archive response should contain id"
        );

        // Read the issue and verify archivedAt is set.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "read",
                &issue_id,
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let detail: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(
            detail.get("archivedAt").and_then(|v| v.as_str()).is_some(),
            "archivedAt should be set after archiving"
        );

        // Unarchive the issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "unarchive",
                &issue_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues unarchive should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let unarchived: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            unarchived.get("id").is_some(),
            "unarchive response should contain id"
        );

        // Read again and verify archivedAt is cleared.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "read",
                &issue_id,
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let detail: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(
            detail.get("archivedAt").unwrap().is_null(),
            "archivedAt should be null after unarchiving"
        );

        // Clean up: permanently delete.
        delete_issue(&issue_id);
    }

    // ── Issues unarchive by human identifier ───────────────────────────────────
    // Regression test: resolve_issue_id must include archived issues in search,
    // otherwise `lineark issues unarchive CAD-XXXX` fails with "not found".

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_unarchive_by_human_identifier() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create an issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] unarchive by identifier",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "issue creation should succeed");
        let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_id = created["id"].as_str().unwrap().to_string();
        let identifier = created["identifier"]
            .as_str()
            .expect("created issue should have identifier (e.g. CAD-1234)")
            .to_string();

        // Archive the issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "archive",
                &issue_id,
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "archive should succeed");

        // Unarchive using the HUMAN identifier (e.g. CAD-1234), not the UUID.
        // This is the regression case: search_issues must include_archived(true)
        // for resolve_issue_id to find archived issues.
        //
        // Linear's search index is async — the newly created+archived issue may
        // not be searchable immediately. Retry with backoff to avoid flakiness.
        let mut last_stdout = String::new();
        let mut last_stderr = String::new();
        let mut succeeded = false;
        for attempt in 0..8 {
            let delay = if attempt < 3 { 1 } else { 3 };
            std::thread::sleep(std::time::Duration::from_secs(delay));

            let output = lineark()
                .args([
                    "--api-token",
                    &token,
                    "--format",
                    "json",
                    "issues",
                    "unarchive",
                    &identifier,
                ])
                .output()
                .expect("failed to execute lineark");
            last_stdout = String::from_utf8_lossy(&output.stdout).to_string();
            last_stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if output.status.success() {
                succeeded = true;
                break;
            }
        }
        assert!(
            succeeded,
            "unarchive by human identifier should succeed (after retries).\nstdout: {last_stdout}\nstderr: {last_stderr}"
        );
        let unarchived: serde_json::Value = serde_json::from_str(&last_stdout).unwrap();
        assert!(
            unarchived.get("id").is_some(),
            "unarchive response should contain id"
        );

        // Clean up.
        delete_issue(&issue_id);
    }

    // ── Issues delete ──────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_delete_permanently() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create an issue to delete.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] CLI issues delete",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "issue creation should succeed");
        let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_id = created["id"]
            .as_str()
            .expect("created issue should have id (UUID)");

        // Delete the issue permanently via CLI.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "delete",
                issue_id,
                "--permanently",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues delete --permanently should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        // Delete mutations may not return an entity (just success checked
        // internally). Verify the command succeeded above is sufficient.
        // If output is present, just verify it's valid JSON.
        if !stdout.trim().is_empty() {
            let _deleted: serde_json::Value = serde_json::from_str(&stdout)
                .expect("delete output should be valid JSON if present");
        }
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_delete_trash_and_verify() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create an issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] CLI issues trash",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_id = created["id"]
            .as_str()
            .expect("created issue should have id (UUID)")
            .to_string();

        // Delete without --permanently (trash).
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "delete",
                &issue_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues delete (trash) should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        // Trash mutation returns the entity directly (no payload wrapper).
        if !stdout.trim().is_empty() {
            let _trashed: serde_json::Value = serde_json::from_str(&stdout)
                .expect("trash output should be valid JSON if present");
        }

        // Clean up: permanently delete the trashed issue.
        delete_issue(&issue_id);
    }

    // ── Documents ─────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn documents_list_json_returns_array() {
        let token = api_token();
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "documents",
                "list",
            ])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "documents list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        assert!(json.is_array(), "documents list JSON should be an array");
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn documents_create_read_update_and_delete() {
        let token = api_token();

        // Get a team key first (documents require a parent like project/issue/team).
        // Create an issue to associate the document with.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] doc parent issue",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "issue creation should succeed");
        let issue: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_id = issue["id"].as_str().unwrap().to_string();

        // Create a document associated with the issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "documents",
                "create",
                "--title",
                "[test] CLI documents CRUD",
                "--content",
                "Automated CLI test document.",
                "--issue",
                &issue_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "documents create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let created: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let doc_id = created["id"]
            .as_str()
            .expect("created document should have id");

        // Read the document.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "documents",
                "read",
                doc_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "documents read should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let read_doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(
            read_doc["title"].as_str(),
            Some("[test] CLI documents CRUD")
        );

        // Update the document.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "documents",
                "update",
                doc_id,
                "--title",
                "[test] CLI documents CRUD — updated",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "documents update should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        // Mutation returns the entity with only `id` selected (serde_json::Value),
        // so we just verify the command succeeded and output has an id.
        let updated: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            updated.get("id").is_some(),
            "document update response should contain id"
        );

        // Delete the document.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "documents",
                "delete",
                doc_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "documents delete should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );

        // Clean up the parent issue.
        delete_issue(&issue_id);
    }

    // ── Cycles ───────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn cycles_list_json_returns_array() {
        let token = api_token();
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "cycles", "list"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "cycles list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        assert!(json.is_array(), "cycles list JSON should be an array");
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn cycles_list_active_json() {
        let token = api_token();
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "cycles",
                "list",
                "--active",
            ])
            .output()
            .expect("failed to execute lineark");
        assert!(
            output.status.success(),
            "cycles list --active should succeed"
        );
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        assert!(
            json.is_array(),
            "cycles list --active JSON should be an array"
        );
        // Active filter should return 0 or 1 cycle.
        let arr = json.as_array().unwrap();
        assert!(
            arr.len() <= 1,
            "cycles list --active should return at most 1 cycle, got {}",
            arr.len()
        );
    }

    // ── Cycles --team and --around-active ────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn cycles_list_with_team_filter() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "cycles",
                "list",
                "--team",
                &team_key,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "cycles list --team should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(json.is_array(), "cycles list --team should return an array");
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn cycles_list_around_active() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "cycles",
                "list",
                "--team",
                &team_key,
                "--around-active",
                "1",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "cycles list --around-active should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let arr = json.as_array().expect("should be an array");
        // --around-active 1 returns at most 3 cycles (active ± 1).
        assert!(
            arr.len() <= 3,
            "cycles list --around-active 1 should return at most 3 cycles, got {}",
            arr.len()
        );
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn cycles_read_by_uuid() {
        let token = api_token();

        // Get a cycle UUID from the list.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "cycles", "list"])
            .output()
            .unwrap();
        let cycles: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let arr = cycles.as_array().unwrap();
        if arr.is_empty() {
            // No cycles in workspace — skip.
            return;
        }
        let cycle_id = arr[0]["id"].as_str().unwrap();

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "cycles",
                "read",
                cycle_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "cycles read by UUID should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let read_cycle: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(read_cycle.get("id").is_some(), "cycle should have an id");
    }

    // ── Embeds upload + download ─────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn embeds_upload_and_download_round_trip() {
        let token = api_token();
        let dir = tempfile::tempdir().unwrap();

        // Create a temp file to upload.
        let upload_path = dir.path().join("test-upload.txt");
        let content = "lineark CLI embeds round-trip test content";
        std::fs::write(&upload_path, content).unwrap();

        // Upload.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "embeds",
                "upload",
                upload_path.to_str().unwrap(),
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "embeds upload should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let upload_result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let asset_url = upload_result["assetUrl"]
            .as_str()
            .expect("upload result should have assetUrl");
        assert!(
            asset_url.starts_with("https://"),
            "asset URL should be HTTPS, got: {asset_url}"
        );

        // Download the uploaded file.
        let download_path = dir.path().join("downloaded.txt");
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "embeds",
                "download",
                asset_url,
                "--output",
                download_path.to_str().unwrap(),
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "embeds download should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );

        // Verify the downloaded file exists and has content.
        assert!(download_path.exists(), "downloaded file should exist");
        let downloaded = std::fs::read_to_string(&download_path).unwrap();
        assert_eq!(
            downloaded, content,
            "downloaded content should match uploaded content"
        );
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn embeds_download_overwrite_flag() {
        let token = api_token();
        let dir = tempfile::tempdir().unwrap();

        // Create and upload a file.
        let upload_path = dir.path().join("overwrite-test.txt");
        std::fs::write(&upload_path, "overwrite test").unwrap();

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "embeds",
                "upload",
                upload_path.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let upload_result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let asset_url = upload_result["assetUrl"].as_str().unwrap();

        // Download to a path.
        let download_path = dir.path().join("target.txt");
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "embeds",
                "download",
                asset_url,
                "--output",
                download_path.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "first download should succeed");

        // Try downloading again without --overwrite — should fail.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "embeds",
                "download",
                asset_url,
                "--output",
                download_path.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "download without --overwrite should fail when file exists"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("already exists"),
            "error should mention file already exists"
        );

        // Try with --overwrite — should succeed.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "embeds",
                "download",
                asset_url,
                "--output",
                download_path.to_str().unwrap(),
                "--overwrite",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "download with --overwrite should succeed"
        );
    }

    // ── Issues read with relations ──────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_read_shows_relations() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create two issues.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] relation parent",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let issue_a: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_a_id = issue_a["id"].as_str().unwrap().to_string();

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] relation child",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let issue_b: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_b_id = issue_b["id"].as_str().unwrap().to_string();

        // Create a relation between them via the SDK.
        {
            use lineark_sdk::generated::enums::IssueRelationType;
            use lineark_sdk::generated::inputs::IssueRelationCreateInput;
            let client = Client::from_token(api_token()).unwrap();
            let input = IssueRelationCreateInput {
                issue_id: Some(issue_a_id.clone()),
                related_issue_id: Some(issue_b_id.clone()),
                r#type: Some(IssueRelationType::Blocks),
                ..Default::default()
            };
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                client
                    .issue_relation_create::<IssueRelation>(None, input)
                    .await
                    .unwrap()
            });
        }

        // Read issue A via CLI — should show the relation.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "read",
                &issue_a_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues read should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let detail: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        // Verify the relations field is present and contains our relation.
        let relations = detail
            .get("relations")
            .and_then(|r| r.get("nodes"))
            .and_then(|n| n.as_array());
        assert!(
            relations.is_some(),
            "issues read should include relations field"
        );
        let relations = relations.unwrap();
        assert!(
            !relations.is_empty(),
            "relations should contain at least one entry"
        );
        let has_our_relation = relations.iter().any(|r| {
            r.get("relatedIssue")
                .and_then(|ri| ri.get("id"))
                .and_then(|id| id.as_str())
                == Some(&issue_b_id)
        });
        assert!(
            has_our_relation,
            "relations should contain the relation to issue B"
        );

        // Clean up.
        delete_issue(&issue_a_id);
        delete_issue(&issue_b_id);
    }

    // ── Comments create ──────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn comments_create_on_issue() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create an issue to comment on.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] CLI comments_create",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "issue creation should succeed");
        let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_id = created["id"]
            .as_str()
            .expect("created issue should have id (UUID)");

        // Create a comment (use UUID to avoid search index lag).
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "comments",
                "create",
                issue_id,
                "--body",
                "Automated CLI test comment.",
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "comment create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let comment: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            comment.get("id").is_some(),
            "comment should have an id field"
        );

        // Clean up: permanently delete the issue.
        delete_issue(issue_id);
    }
}
