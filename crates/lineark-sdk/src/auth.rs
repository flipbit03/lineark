//! API token resolution.
//!
//! Supports three sources (in precedence order): explicit token, the
//! `LINEAR_API_TOKEN` environment variable, and a token file at any path.

use crate::error::LinearError;
use std::path::Path;

/// Resolve a Linear API token from a file at the given path.
pub fn token_from_file(path: &Path) -> Result<String, LinearError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        LinearError::AuthConfig(format!(
            "Could not read token file {}: {}",
            path.display(),
            e
        ))
    })?;
    let token = content.trim().to_string();
    if token.is_empty() {
        return Err(LinearError::AuthConfig(format!(
            "Token file {} is empty",
            path.display()
        )));
    }
    Ok(token)
}

/// Resolve a Linear API token from the environment variable `LINEAR_API_TOKEN`.
pub fn token_from_env() -> Result<String, LinearError> {
    match std::env::var("LINEAR_API_TOKEN") {
        Ok(val) if !val.trim().is_empty() => Ok(val.trim().to_string()),
        _ => Err(LinearError::AuthConfig(
            "LINEAR_API_TOKEN environment variable not set".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Guards all tests that manipulate the `LINEAR_API_TOKEN` env var.
    /// Tests run in parallel by default — without this, one test's `remove_var`
    /// races with another test's `set_var`, causing spurious failures.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Run a closure with `LINEAR_API_TOKEN` set to `value`, restoring the
    /// original value (or removing it) when done — even on panic.
    fn with_env_token<F: FnOnce()>(value: Option<&str>, f: F) {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original = std::env::var("LINEAR_API_TOKEN").ok();
        match value {
            Some(v) => std::env::set_var("LINEAR_API_TOKEN", v),
            None => std::env::remove_var("LINEAR_API_TOKEN"),
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match &original {
            Some(v) => std::env::set_var("LINEAR_API_TOKEN", v),
            None => std::env::remove_var("LINEAR_API_TOKEN"),
        }
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    #[test]
    fn token_from_env_success() {
        with_env_token(Some("test-token-12345"), || {
            assert_eq!(token_from_env().unwrap(), "test-token-12345");
        });
    }

    #[test]
    fn token_from_env_missing() {
        with_env_token(None, || {
            let result = token_from_env();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("LINEAR_API_TOKEN"));
        });
    }

    #[test]
    fn token_from_env_empty_string_is_treated_as_absent() {
        with_env_token(Some(""), || {
            assert!(token_from_env().is_err());
        });
    }

    #[test]
    fn token_from_env_whitespace_only_is_treated_as_absent() {
        with_env_token(Some("   "), || {
            assert!(token_from_env().is_err());
        });
    }

    #[test]
    fn token_from_env_trims_whitespace() {
        with_env_token(Some("  my-token  "), || {
            assert_eq!(token_from_env().unwrap(), "my-token");
        });
    }

    #[test]
    fn token_from_file_reads_and_trims() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".linear_api_token");
        std::fs::write(&path, "  my-token-123  \n").unwrap();
        assert_eq!(token_from_file(&path).unwrap(), "my-token-123");
    }

    #[test]
    fn token_from_file_missing_file() {
        let path = std::path::PathBuf::from("/tmp/nonexistent_token_file_xyz");
        let err = token_from_file(&path).unwrap_err();
        assert!(err.to_string().contains("nonexistent_token_file_xyz"));
    }

    #[test]
    fn token_from_file_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".linear_api_token");
        std::fs::write(&path, "  \n").unwrap();
        let err = token_from_file(&path).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }
}
