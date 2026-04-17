//! Test API token loading with optional multi-workspace pooling.
//!
//! `~/.linear_api_token_test` (and the `LINEAR_TEST_TOKEN` GitHub secret)
//! holds one or more Linear API tokens, separated by `;` (any whitespace
//! around separators is trimmed; an empty trailing token is ignored, so a
//! trailing `;` is harmless). One token per line is also accepted, which
//! makes a single-token file (the original format) work unchanged.
//!
//! When multiple tokens are present, [`test_token`] picks one at random
//! per test process and returns the same one for the lifetime of that
//! process (resources created by one test reference the same workspace
//! the next test runs against). Across many test runs the load is
//! distributed roughly uniformly, which spreads pressure on Linear's
//! free-plan resource limits and trash-retention quirks across N
//! workspaces instead of hammering one.

use std::sync::OnceLock;

const TOKEN_FILE: &str = ".linear_api_token_test";

fn token_path() -> Option<std::path::PathBuf> {
    home::home_dir().map(|h| h.join(TOKEN_FILE))
}

/// Returns `Some(reason)` if the test token file is missing, `None` if present.
/// Used with `test_with::runtime_ignore_if` to skip online tests gracefully.
pub fn no_online_test_token() -> Option<String> {
    let path = token_path()?;
    if path.exists() {
        None
    } else {
        Some(format!("~/{TOKEN_FILE} not found"))
    }
}

/// Read the test API token from `~/.linear_api_token_test`. When the file
/// holds multiple `;`-separated tokens, picks one at random per process.
/// The choice is cached so every test in the same process uses the same
/// workspace.
pub fn test_token() -> String {
    static CHOSEN: OnceLock<String> = OnceLock::new();
    CHOSEN.get_or_init(load_and_pick).clone()
}

fn load_and_pick() -> String {
    let path = token_path().expect("could not determine home directory");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));
    let tokens = parse_tokens(&raw);
    assert!(
        !tokens.is_empty(),
        "no API tokens found in {} — file should contain one or more `;`-separated tokens",
        path.display()
    );
    let idx = pseudo_random_index(tokens.len());
    let chosen = tokens[idx].to_string();
    eprintln!(
        "test_token: using workspace {}/{} (token ending …{})",
        idx + 1,
        tokens.len(),
        // Last 6 chars of the chosen token — enough to identify which
        // workspace was picked when triaging a failure, without dumping
        // the whole secret.
        chosen.get(chosen.len().saturating_sub(6)..).unwrap_or("?")
    );
    chosen
}

/// Parse a `;`- and/or newline-separated list of API tokens. Whitespace and
/// blank entries are dropped; lines whose first non-whitespace character is
/// `#` are treated as comments and dropped *before* `;`-splitting (so a
/// `;` inside a comment line doesn't accidentally produce fake tokens).
/// A trailing `;` produces an empty entry which is dropped.
fn parse_tokens(raw: &str) -> Vec<&str> {
    raw.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .flat_map(|line| line.split(';'))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect()
}

/// Cheap, dependency-free per-process pseudo-random index. Doesn't need to
/// be cryptographically random — just needs to vary across test processes
/// so different runs hit different tokens. Uses the process ID XORed with
/// the current nanosecond.
fn pseudo_random_index(len: usize) -> usize {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let pid = std::process::id() as u64;
    ((nanos ^ pid).wrapping_mul(6364136223846793005)) as usize % len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_token() {
        assert_eq!(parse_tokens("lin_api_one"), vec!["lin_api_one"]);
        assert_eq!(parse_tokens("lin_api_one\n"), vec!["lin_api_one"]);
        assert_eq!(parse_tokens("  lin_api_one  "), vec!["lin_api_one"]);
    }

    #[test]
    fn parse_multi_token_semicolon() {
        assert_eq!(
            parse_tokens("lin_api_one;lin_api_two;lin_api_three"),
            vec!["lin_api_one", "lin_api_two", "lin_api_three"]
        );
    }

    #[test]
    fn trailing_semicolon_is_harmless() {
        assert_eq!(
            parse_tokens("lin_api_one;"),
            vec!["lin_api_one"],
            "single token with trailing ; should parse as one"
        );
        assert_eq!(
            parse_tokens("lin_api_one;lin_api_two;"),
            vec!["lin_api_one", "lin_api_two"]
        );
    }

    #[test]
    fn newlines_also_separate() {
        assert_eq!(
            parse_tokens("lin_api_one\nlin_api_two"),
            vec!["lin_api_one", "lin_api_two"]
        );
    }

    #[test]
    fn comments_and_blanks_are_ignored() {
        let raw = "# workspace A\nlin_api_one;\n\n# workspace B\nlin_api_two\n";
        assert_eq!(parse_tokens(raw), vec!["lin_api_one", "lin_api_two"]);
    }

    #[test]
    fn empty_input_yields_empty() {
        assert!(parse_tokens("").is_empty());
        assert!(parse_tokens(";;;").is_empty());
        assert!(parse_tokens("# only comments\n").is_empty());
    }

    #[test]
    fn pseudo_random_index_in_range() {
        for len in 1..10 {
            let idx = pseudo_random_index(len);
            assert!(idx < len, "idx={idx} out of range for len={len}");
        }
    }
}
