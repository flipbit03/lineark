/// Returns `Some(reason)` if the test token file is missing, `None` if present.
/// Used with `test_with::runtime_ignore_if` to skip online tests gracefully.
pub fn no_online_test_token() -> Option<String> {
    let path = home::home_dir()?.join(".linear_api_token_test");
    if path.exists() {
        None
    } else {
        Some("~/.linear_api_token_test not found".to_string())
    }
}

/// Read the test API token from `~/.linear_api_token_test`.
pub fn test_token() -> String {
    let path = home::home_dir()
        .expect("could not determine home directory")
        .join(".linear_api_token_test");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {}", path.display(), e))
        .trim()
        .to_string()
}
