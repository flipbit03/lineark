use super::*;

#[test]
fn teams_help_shows_subcommands() {
    lineark()
        .args(["teams", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"));
}
