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
        .stdout(predicate::str::contains("search"));
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
