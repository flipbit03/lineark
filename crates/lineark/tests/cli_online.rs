//! Online CLI tests for lineark — these hit the real Linear API.
//!
//! Requires a valid Linear API token at `~/.linear_api_token_test`.
//! When the token file is missing, tests are automatically skipped with a message.

use assert_cmd::Command;
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
        .block_on(async { client.issue_delete(Some(true), id).await.unwrap() });
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
        assert!(json.is_array(), "issues list JSON should be an array");
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
        assert!(json.is_array(), "issues search JSON should be an array");
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
        let identifier = created["identifier"]
            .as_str()
            .expect("created issue should have identifier");
        assert!(
            identifier.contains('-'),
            "identifier should be like ABC-123, got: {identifier}"
        );
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
        let updated: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(
            updated["title"].as_str(),
            Some("[test] CLI create+update — updated")
        );

        // Clean up: permanently delete the issue.
        delete_issue(issue_id);
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
