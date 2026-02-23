use super::*;

#[test]
fn documents_help_shows_subcommands() {
    lineark()
        .args(["documents", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("read"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"));
}

#[test]
fn documents_create_help_shows_flags() {
    lineark()
        .args(["documents", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--content"))
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--issue"));
}

#[test]
fn documents_update_help_shows_flags() {
    lineark()
        .args(["documents", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--content"));
}

#[test]
fn documents_update_no_flags_prints_error() {
    lineark()
        .args([
            "--api-token",
            "fake-token",
            "documents",
            "update",
            "doc-uuid",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No update fields provided"));
}

#[test]
fn documents_list_help_shows_filter_flags() {
    lineark()
        .args(["documents", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--issue"));
}
