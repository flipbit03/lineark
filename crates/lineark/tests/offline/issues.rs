use super::*;

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
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("archive"))
        .stdout(predicate::str::contains("unarchive"))
        .stdout(predicate::str::contains("delete"));
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
fn issues_archive_help_shows_identifier() {
    lineark()
        .args(["issues", "archive", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<IDENTIFIER>"));
}

#[test]
fn issues_unarchive_help_shows_identifier() {
    lineark()
        .args(["issues", "unarchive", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<IDENTIFIER>"));
}

#[test]
fn issues_delete_help_shows_flags() {
    lineark()
        .args(["issues", "delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--permanently"))
        .stdout(predicate::str::contains("<IDENTIFIER>"));
}

#[test]
fn issues_update_no_flags_prints_error() {
    lineark()
        .args(["--api-token", "fake-token", "issues", "update", "ENG-123"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No update fields provided"));
}

#[test]
fn issues_create_help_shows_project_and_cycle() {
    lineark()
        .args(["issues", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--cycle"))
        .stdout(predicate::str::contains("--status"));
}

#[test]
fn issues_update_help_shows_clear_parent_and_project() {
    lineark()
        .args(["issues", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--clear-parent"))
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--cycle"));
}

#[test]
fn issues_search_help_shows_filter_flags() {
    lineark()
        .args(["issues", "search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--team"))
        .stdout(predicate::str::contains("--assignee"))
        .stdout(predicate::str::contains("--status"));
}

#[test]
fn issues_update_clear_parent_conflicts_with_parent() {
    lineark()
        .args([
            "--api-token",
            "fake",
            "issues",
            "update",
            "ENG-123",
            "--parent",
            "ENG-456",
            "--clear-parent",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}
