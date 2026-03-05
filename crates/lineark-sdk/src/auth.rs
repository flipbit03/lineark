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
use std::path::{Path, PathBuf};

/// A named profile from the config file.
#[derive(Debug, serde::Deserialize)]
struct Profile {
    api_token: String,
}

/// Top-level config file structure.
#[derive(Debug, serde::Deserialize)]
struct Config {
    #[serde(default)]
    profiles: HashMap<String, Profile>,
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
    token_from_config_at(&path, profile)
}

/// Resolve a Linear API token from a named profile at a specific config file path.
///
/// This is the testable core — `token_from_config` is a thin wrapper that
/// provides the default path.
fn token_from_config_at(path: &Path, profile: Option<&str>) -> Result<String, LinearError> {
    let contents = std::fs::read_to_string(path).map_err(|e| {
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

    /// Guards all tests that manipulate env vars (`LINEAR_API_TOKEN`, `LINEAR_PROFILE`).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Run a closure with specific env vars set, restoring originals when done.
    fn with_env<F: FnOnce()>(vars: &[(&str, Option<&str>)], f: F) {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let originals: Vec<(&str, Option<String>)> = vars
            .iter()
            .map(|(key, _)| (*key, std::env::var(key).ok()))
            .collect();
        for (key, value) in vars {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        for (key, original) in &originals {
            match original {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    /// Helper to create a temp config file with the given TOML content.
    fn write_temp_config(content: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".config").join("lineark");
        std::fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");
        std::fs::write(&config_path, content).unwrap();
        (dir, config_path)
    }

    // ── token_from_env tests ─────────────────────────────────────────────

    #[test]
    fn token_from_env_success() {
        with_env(&[("LINEAR_API_TOKEN", Some("test-token-12345"))], || {
            assert_eq!(token_from_env().unwrap(), "test-token-12345");
        });
    }

    #[test]
    fn token_from_env_missing() {
        with_env(&[("LINEAR_API_TOKEN", None)], || {
            let result = token_from_env();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("LINEAR_API_TOKEN"));
        });
    }

    #[test]
    fn token_from_env_empty_string_is_treated_as_absent() {
        with_env(&[("LINEAR_API_TOKEN", Some(""))], || {
            assert!(token_from_env().is_err());
        });
    }

    #[test]
    fn token_from_env_whitespace_only_is_treated_as_absent() {
        with_env(&[("LINEAR_API_TOKEN", Some("   "))], || {
            assert!(token_from_env().is_err());
        });
    }

    #[test]
    fn token_from_env_trims_whitespace() {
        with_env(&[("LINEAR_API_TOKEN", Some("  my-token  "))], || {
            assert_eq!(token_from_env().unwrap(), "my-token");
        });
    }

    // ── path tests ───────────────────────────────────────────────────────

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

    // ── token_from_config_at tests ───────────────────────────────────────

    #[test]
    fn config_reads_default_profile() {
        let (_dir, path) = write_temp_config(
            r#"
[profiles.default]
api_token = "lin_default_123"

[profiles.work]
api_token = "lin_work_456"
"#,
        );
        assert_eq!(token_from_config_at(&path, None).unwrap(), "lin_default_123");
    }

    #[test]
    fn config_reads_named_profile() {
        let (_dir, path) = write_temp_config(
            r#"
[profiles.default]
api_token = "lin_default_123"

[profiles.work]
api_token = "lin_work_456"
"#,
        );
        assert_eq!(
            token_from_config_at(&path, Some("work")).unwrap(),
            "lin_work_456"
        );
    }

    #[test]
    fn config_missing_profile_returns_error() {
        let (_dir, path) = write_temp_config(
            r#"
[profiles.default]
api_token = "lin_abc"
"#,
        );
        let err = token_from_config_at(&path, Some("nonexistent")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("nonexistent"), "Error should name the profile: {msg}");
        assert!(msg.contains("not found"), "Error should say not found: {msg}");
    }

    #[test]
    fn config_empty_token_returns_error() {
        let (_dir, path) = write_temp_config(
            r#"
[profiles.default]
api_token = "   "
"#,
        );
        let err = token_from_config_at(&path, None).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("empty"), "Error should mention empty token: {msg}");
    }

    #[test]
    fn config_trims_token_whitespace() {
        let (_dir, path) = write_temp_config(
            r#"
[profiles.default]
api_token = "  lin_trimmed  "
"#,
        );
        assert_eq!(token_from_config_at(&path, None).unwrap(), "lin_trimmed");
    }

    #[test]
    fn config_missing_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.toml");
        let err = token_from_config_at(&path, None).unwrap_err();
        assert!(
            err.to_string().contains("Could not read"),
            "Error should mention reading: {}",
            err
        );
    }

    #[test]
    fn config_malformed_toml_returns_error() {
        let (_dir, path) = write_temp_config("this is not [valid toml }{");
        let err = token_from_config_at(&path, None).unwrap_err();
        assert!(
            err.to_string().contains("Invalid config"),
            "Error should mention invalid config: {}",
            err
        );
    }

    #[test]
    fn config_no_profiles_section_returns_error() {
        let (_dir, path) = write_temp_config(
            r#"
[something_else]
key = "value"
"#,
        );
        let err = token_from_config_at(&path, None).unwrap_err();
        assert!(
            err.to_string().contains("not found"),
            "Error should say profile not found: {}",
            err
        );
    }

    #[test]
    fn config_empty_profiles_section_returns_error() {
        let (_dir, path) = write_temp_config("");
        let err = token_from_config_at(&path, None).unwrap_err();
        assert!(
            err.to_string().contains("not found"),
            "Error should say profile not found: {}",
            err
        );
    }

    #[test]
    fn config_multiple_profiles_are_independent() {
        let (_dir, path) = write_temp_config(
            r#"
[profiles.default]
api_token = "tok_default"

[profiles.staging]
api_token = "tok_staging"

[profiles.production]
api_token = "tok_production"
"#,
        );
        assert_eq!(token_from_config_at(&path, None).unwrap(), "tok_default");
        assert_eq!(
            token_from_config_at(&path, Some("staging")).unwrap(),
            "tok_staging"
        );
        assert_eq!(
            token_from_config_at(&path, Some("production")).unwrap(),
            "tok_production"
        );
    }

    // ── auto_token precedence tests ──────────────────────────────────────

    #[test]
    fn auto_token_prefers_env_over_config() {
        with_env(
            &[
                ("LINEAR_API_TOKEN", Some("env-token")),
                ("LINEAR_PROFILE", None),
            ],
            || {
                // Even though config file doesn't exist, env token should win
                assert_eq!(auto_token(None).unwrap(), "env-token");
            },
        );
    }

    #[test]
    fn auto_token_explicit_profile_skips_env() {
        with_env(
            &[
                ("LINEAR_API_TOKEN", Some("env-token")),
                ("LINEAR_PROFILE", None),
            ],
            || {
                // Explicit profile should NOT fall back to env
                let result = auto_token(Some("nonexistent"));
                assert!(result.is_err());
                let err = result.unwrap_err().to_string();
                assert!(
                    err.contains("nonexistent") || err.contains("config"),
                    "Expected config-related error, got: {err}"
                );
            },
        );
    }

    #[test]
    fn auto_token_linear_profile_env_var_skips_default() {
        // LINEAR_PROFILE should direct to the named profile, not "default".
        // Since we can't inject the config path into auto_token, we verify
        // that it attempts to read the config file (and fails) rather than
        // falling through to env token.
        with_env(
            &[
                ("LINEAR_API_TOKEN", None),
                ("LINEAR_PROFILE", Some("staging")),
            ],
            || {
                let result = auto_token(None);
                assert!(result.is_err());
                let err = result.unwrap_err().to_string();
                // Should fail trying to read config, not complain about env var
                assert!(
                    err.contains("config") || err.contains("staging"),
                    "Expected config-related error for LINEAR_PROFILE, got: {err}"
                );
            },
        );
    }

    #[test]
    fn auto_token_linear_profile_empty_is_ignored() {
        with_env(
            &[
                ("LINEAR_API_TOKEN", Some("env-token")),
                ("LINEAR_PROFILE", Some("")),
            ],
            || {
                // Empty LINEAR_PROFILE should be ignored, fall through to env
                assert_eq!(auto_token(None).unwrap(), "env-token");
            },
        );
    }

    #[test]
    fn auto_token_linear_profile_whitespace_is_ignored() {
        with_env(
            &[
                ("LINEAR_API_TOKEN", Some("env-token")),
                ("LINEAR_PROFILE", Some("   ")),
            ],
            || {
                // Whitespace-only LINEAR_PROFILE should be ignored
                assert_eq!(auto_token(None).unwrap(), "env-token");
            },
        );
    }
}
