use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const GITHUB_REPO: &str = "flipbit03/lineark";
const CACHE_TTL_SECS: u64 = 24 * 60 * 60; // 24 hours

/// Cached version check result.
#[derive(Debug, Serialize, Deserialize)]
struct VersionCache {
    checked_at: u64,
    latest_version: String,
}

/// Returns the path to the version cache file (~/.config/lineark/latest_version_check.json).
fn cache_path() -> Option<PathBuf> {
    // Use XDG_CONFIG_HOME if set, otherwise ~/.config
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| home::home_dir().map(|h| h.join(".config")));
    config_dir.map(|d| d.join("lineark").join("latest_version_check.json"))
}

/// Read the cached version check, if it exists and is valid JSON.
fn read_cache() -> Option<VersionCache> {
    let path = cache_path()?;
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Write the version cache to disk, creating parent directories as needed.
fn write_cache(cache: &VersionCache) {
    if let Some(path) = cache_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(
            &path,
            serde_json::to_string_pretty(cache).unwrap_or_default(),
        );
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Fetch the latest release version from GitHub.
async fn fetch_latest_version() -> Result<String> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .get(&url)
        .header("User-Agent", "lineark-self-update")
        .send()
        .await
        .context("failed to reach GitHub API")?
        .error_for_status()
        .context("GitHub API returned an error")?
        .json()
        .await
        .context("failed to parse GitHub API response")?;

    let tag = resp["tag_name"]
        .as_str()
        .context("no tag_name in GitHub release")?;
    Ok(tag.strip_prefix('v').unwrap_or(tag).to_string())
}

/// Get the latest version, using cache unless `force` is true.
///
/// - `force = false`: returns cached value if fresh (< 24h), otherwise fetches.
/// - `force = true`: always fetches from GitHub.
///
/// Returns `None` only if both the network request and cache are unavailable.
pub async fn get_latest_version(force: bool) -> Option<String> {
    if !force {
        if let Some(cache) = read_cache() {
            if now_secs().saturating_sub(cache.checked_at) < CACHE_TTL_SECS {
                return Some(cache.latest_version);
            }
        }
    }

    match fetch_latest_version().await {
        Ok(version) => {
            write_cache(&VersionCache {
                checked_at: now_secs(),
                latest_version: version.clone(),
            });
            Some(version)
        }
        Err(_) => {
            // Network failed — return stale cache if available.
            read_cache().map(|c| c.latest_version)
        }
    }
}

/// Read the cached latest version synchronously (no network, no async).
/// Returns `Some` only if the cache exists and is fresh (< 24h).
pub fn get_cached_version() -> Option<String> {
    let cache = read_cache()?;
    if now_secs().saturating_sub(cache.checked_at) < CACHE_TTL_SECS {
        Some(cache.latest_version)
    } else {
        None
    }
}

/// Returns the current compiled-in version of lineark.
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Returns true if this is a dev build (version 0.0.0).
pub fn is_dev_build() -> bool {
    current_version() == "0.0.0"
}

/// Returns true if `latest` is newer than `current` using semver comparison.
pub fn is_newer(current: &str, latest: &str) -> bool {
    match (
        semver::Version::parse(current),
        semver::Version::parse(latest),
    ) {
        (Ok(c), Ok(l)) => l > c,
        _ => latest != current,
    }
}

/// Returns the download URL for a GitHub release asset for the current platform.
pub fn release_asset_url(tag: &str) -> Result<String> {
    let os_tag = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        _ => anyhow::bail!("unsupported OS: {}", std::env::consts::OS),
    };
    let arch_tag = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => anyhow::bail!("unsupported architecture: {}", std::env::consts::ARCH),
    };

    if os_tag == "macos" && arch_tag == "x86_64" {
        anyhow::bail!("macOS x86_64 binaries are not provided — use `cargo install lineark`");
    }

    Ok(format!(
        "https://github.com/{GITHUB_REPO}/releases/download/v{tag}/lineark_{os_tag}_{arch_tag}"
    ))
}
