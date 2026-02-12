use crate::error::LinearError;
use std::path::PathBuf;

/// Resolve a Linear API token from the filesystem.
/// Reads `~/.linear_api_token` (linearis-compatible).
pub fn token_from_file() -> Result<String, LinearError> {
    let path = token_file_path();
    std::fs::read_to_string(&path)
        .map(|s| s.trim().to_string())
        .map_err(|e| {
            LinearError::AuthConfig(format!(
                "Could not read token file {}: {}",
                path.display(),
                e
            ))
        })
}

/// Resolve a Linear API token from the environment variable `LINEAR_API_TOKEN`.
pub fn token_from_env() -> Result<String, LinearError> {
    std::env::var("LINEAR_API_TOKEN").map_err(|_| {
        LinearError::AuthConfig("LINEAR_API_TOKEN environment variable not set".to_string())
    })
}

/// Resolve a Linear API token with precedence: env var -> file.
/// (CLI flag takes highest precedence but is handled at the CLI layer.)
pub fn auto_token() -> Result<String, LinearError> {
    token_from_env().or_else(|_| token_from_file())
}

fn token_file_path() -> PathBuf {
    dirs_next().join(".linear_api_token")
}

fn dirs_next() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("~"))
}
