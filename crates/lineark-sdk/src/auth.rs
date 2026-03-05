//! API token resolution.
//!
//! Supports multiple sources (in precedence order):
//! 1. Explicit token (CLI flag)
//! 2. `LINEAR_API_TOKEN` environment variable
//! 3. Named profile from `~/.config/lineark/config.toml`
//! 4. `default` profile from `~/.config/lineark/config.toml`
//! 5. Legacy `~/.linear_api_token` file

use crate::error::LinearError;
use std::collections::HashMap;
use std::path::PathBuf;

/// A named profile from the config file.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct Profile {
    pub api_token: String,
}

/// Top-level config file structure.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct Config {
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

/// Resolve a Linear API token from the legacy `~/.linear_api_token` file.
pub fn token_from_file() -> Result<String, LinearError> {
    let path = legacy_token_path()?;
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
    match std::env::var("LINEAR_API_TOKEN") {
        Ok(val) if !val.trim().is_empty() => Ok(val.trim().to_string()),
        _ => Err(LinearError::AuthConfig(
            "LINEAR_API_TOKEN environment variable not set".to_string(),
        )),
    }
}

/// Resolve a Linear API token from a named profile in `~/.config/lineark/config.toml`.
///
/// If `profile` is `None`, looks up the `default` profile.
pub fn token_from_config(profile: Option<&str>) -> Result<String, LinearError> {
    let path = config_file_path()?;
    let contents = std::fs::read_to_string(&path).map_err(|e| {
        LinearError::AuthConfig(format!(
            "Could not read config file {}: {}",
            path.display(),
            e
        ))
    })?;

    let config: Config = toml::from_str(&contents).map_err(|e| {
        LinearError::AuthConfig(format!("Invalid config file {}: {}", path.display(), e))
    })?;

    let name = profile.unwrap_or("default");
    let p = config.profiles.get(name).ok_or_else(|| {
        LinearError::AuthConfig(format!(
            "Profile '{}' not found in {}",
            name,
            path.display()
        ))
    })?;

    let token = p.api_token.trim().to_string();
    if token.is_empty() {
        return Err(LinearError::AuthConfig(format!(
            "Profile '{}' has an empty api_token",
            name
        )));
    }
    Ok(token)
}

/// Resolve a Linear API token with precedence:
/// env var -> profile config -> legacy file.
///
/// If `profile` is `Some`, the config file lookup uses that profile name.
/// If `profile` is `None`, falls through: env -> config `default` -> legacy file.
pub fn auto_token(profile: Option<&str>) -> Result<String, LinearError> {
    // If an explicit profile was requested, skip env and legacy — go straight to config.
    if profile.is_some() {
        return token_from_config(profile);
    }

    // Check $LINEAR_PROFILE env var for profile selection.
    if let Ok(env_profile) = std::env::var("LINEAR_PROFILE") {
        let env_profile = env_profile.trim().to_string();
        if !env_profile.is_empty() {
            return token_from_config(Some(&env_profile));
        }
    }

    token_from_env()
        .or_else(|_| token_from_config(None))
        .or_else(|_| token_from_file())
}

/// Path to the config file: `~/.config/lineark/config.toml`.
pub fn config_file_path() -> Result<PathBuf, LinearError> {
    let home = home::home_dir()
        .ok_or_else(|| LinearError::AuthConfig("Could not determine home directory".to_string()))?;
    Ok(home.join(".config").join("lineark").join("config.toml"))
}

fn legacy_token_path() -> Result<PathBuf, LinearError> {
    let home = home::home_dir()
        .ok_or_else(|| LinearError::AuthConfig("Could not determine home directory".to_string()))?;
    Ok(home.join(".linear_api_token"))
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
    fn auto_token_prefers_env() {
        with_env_token(Some("env-token-auto"), || {
            assert_eq!(auto_token(None).unwrap(), "env-token-auto");
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
    fn legacy_token_path_is_home_based() {
        let path = legacy_token_path().unwrap();
        assert!(path.to_str().unwrap().contains(".linear_api_token"));
        assert!(path.to_str().unwrap().starts_with("/"));
    }

    #[test]
    fn config_file_path_is_xdg_based() {
        let path = config_file_path().unwrap();
        let s = path.to_str().unwrap();
        assert!(s.contains(".config/lineark/config.toml"));
        assert!(s.starts_with("/"));
    }

    #[test]
    fn token_from_config_parses_valid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".config").join("lineark");
        std::fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");
        std::fs::write(
            &config_path,
            r#"
[profiles.default]
api_token = "lin_default_123"

[profiles.work]
api_token = "lin_work_456"
"#,
        )
        .unwrap();

        // Parse directly to test the TOML structure
        let contents = std::fs::read_to_string(&config_path).unwrap();
        let config: Config = toml::from_str(&contents).unwrap();
        assert_eq!(config.profiles["default"].api_token, "lin_default_123");
        assert_eq!(config.profiles["work"].api_token, "lin_work_456");
        assert_eq!(config.profiles.len(), 2);
    }

    #[test]
    fn config_missing_profile_errors() {
        let contents = r#"
[profiles.default]
api_token = "lin_abc"
"#;
        let config: Config = toml::from_str(contents).unwrap();
        assert!(config.profiles.get("nonexistent").is_none());
    }

    #[test]
    fn config_empty_token_detected() {
        let contents = r#"
[profiles.default]
api_token = "   "
"#;
        let config: Config = toml::from_str(contents).unwrap();
        assert!(config.profiles["default"].api_token.trim().is_empty());
    }

    #[test]
    fn explicit_profile_skips_env() {
        // When --profile is specified, env var should be ignored.
        // We can't test the full auto_token flow without a real config file,
        // but we verify the logic: explicit profile goes straight to config lookup.
        with_env_token(Some("env-token"), || {
            let result = auto_token(Some("nonexistent"));
            // Should fail looking for "nonexistent" in config, NOT succeed with env token
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("nonexistent") || err.contains("config"),
                "Expected config-related error, got: {}",
                err
            );
        });
    }
}
