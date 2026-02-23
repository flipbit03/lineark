use super::*;

#[test]
fn labels_list_help_shows_team_flag() {
    lineark()
        .args(["labels", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--team"));
}
