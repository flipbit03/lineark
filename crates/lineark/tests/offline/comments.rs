use super::*;

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
