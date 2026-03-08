//! Profile utilities for multi-token auth.
//!
//! Profiles are named token files: `~/.linear_api_token_{name}`.
//! The "default" profile maps to `~/.linear_api_token` (no suffix).

use std::path::{Path, PathBuf};

/// Resolve the token file path for a profile name.
/// "default" maps to `~/.linear_api_token`, others to `~/.linear_api_token_{name}`.
pub fn token_path(home: &Path, name: &str) -> PathBuf {
    if name == "default" {
        home.join(".linear_api_token")
    } else {
        home.join(format!(".linear_api_token_{name}"))
    }
}

/// Display path for a profile (tilde-prefixed, for user-facing output).
pub fn display_path(name: &str) -> String {
    if name == "default" {
        "~/.linear_api_token".to_string()
    } else {
        format!("~/.linear_api_token_{name}")
    }
}

/// Discover available profiles by scanning `~/.linear_api_token_*`.
/// Returns profile names (the suffix after `_`), sorted, excluding "test".
pub fn discover(home: &Path) -> Vec<String> {
    let prefix = ".linear_api_token_";
    let Ok(entries) = std::fs::read_dir(home) else {
        return Vec::new();
    };
    let mut profiles: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let suffix = name.strip_prefix(prefix)?;
            if suffix.is_empty() || suffix == "test" {
                return None;
            }
            Some(suffix.to_string())
        })
        .collect();
    profiles.sort();
    profiles
}

/// Format the error message when a profile file is not found.
pub fn not_found_error(profile: &str, home: &Path) -> String {
    let profiles = discover(home);
    let default_exists = home.join(".linear_api_token").exists();

    let mut available: Vec<String> = Vec::new();
    if default_exists {
        available.push("\"default\"".to_string());
    }
    for p in &profiles {
        available.push(format!("\"{p}\""));
    }

    let mut msg = format!("Profile \"{profile}\" not found.");
    if available.is_empty() {
        msg.push_str(" No profiles found.");
    } else {
        msg.push_str(&format!(" Available profiles: {}.", available.join(", ")));
    }
    msg.push_str(&format!(
        "\nCreate it with:\n  echo \"lin_api_...\" > {}",
        display_path(profile)
    ));
    msg
}
