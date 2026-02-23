use super::*;

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
