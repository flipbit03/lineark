//! Offline CLI tests for lineark.
//!
//! These tests don't need any API token — they test usage, help, and error handling.
//! Online tests that hit the real Linear API live in `cli_online.rs`.

use assert_cmd::Command;
use predicates::prelude::*;

fn lineark() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("lineark").unwrap()
}

// ── Usage command ───────────────────────────────────────────────────────────

#[test]
fn usage_prints_command_reference() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("lineark"))
        .stdout(predicate::str::contains("COMMANDS"))
        .stdout(predicate::str::contains("whoami"))
        .stdout(predicate::str::contains("teams"))
        .stdout(predicate::str::contains("issues"))
        .stdout(predicate::str::contains("AUTH"));
}

#[test]
fn usage_mentions_global_options() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("--api-token"))
        .stdout(predicate::str::contains("--format"));
}

// ── Help flags ──────────────────────────────────────────────────────────────

#[test]
fn help_flag_shows_help() {
    lineark()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("lineark"))
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn version_flag_shows_version() {
    lineark()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("lineark"));
}

#[test]
fn teams_help_shows_subcommands() {
    lineark()
        .args(["teams", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"));
}

#[test]
fn issues_help_shows_subcommands() {
    lineark()
        .args(["issues", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("read"))
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"));
}

#[test]
fn issues_create_help_shows_flags() {
    lineark()
        .args(["issues", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--team"))
        .stdout(predicate::str::contains("--priority"))
        .stdout(predicate::str::contains("--description"))
        .stdout(predicate::str::contains("--assignee"))
        .stdout(predicate::str::contains("--labels"));
}

#[test]
fn issues_update_help_shows_flags() {
    lineark()
        .args(["issues", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--status"))
        .stdout(predicate::str::contains("--priority"))
        .stdout(predicate::str::contains("--labels"))
        .stdout(predicate::str::contains("--label-by"))
        .stdout(predicate::str::contains("--clear-labels"))
        .stdout(predicate::str::contains("--assignee"))
        .stdout(predicate::str::contains("--parent"));
}

#[test]
fn comments_help_shows_subcommands() {
    lineark()
        .args(["comments", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("create"));
}

#[test]
fn comments_create_help_shows_flags() {
    lineark()
        .args(["comments", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--body"))
        .stdout(predicate::str::contains("<ISSUE>"));
}

#[test]
fn usage_includes_write_commands() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("issues create"))
        .stdout(predicate::str::contains("issues update"))
        .stdout(predicate::str::contains("comments create"));
}

// ── Auth error handling ─────────────────────────────────────────────────────

#[test]
fn invalid_token_prints_error_and_exits_nonzero() {
    lineark()
        .args(["--api-token", "invalid_token_abc", "whoami"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn empty_token_prints_error() {
    lineark()
        .args(["--api-token", "", "whoami"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}
