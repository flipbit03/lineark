//! Housekeeping binary: clean every pooled test workspace.
//!
//! Iterates every token from `~/.linear_api_token_test` (the same `;`/newline
//! pool consumed by `test_token()`), creating a fresh client per workspace
//! and running `cleanup_workspace` against each. Failures on one workspace
//! don't abort the others — we want best-effort cleanup of the entire pool
//! before any test process picks one at random.
//!
//! This is what the CI `Clean test workspace` step calls. Without it, only
//! the workspace the test process happens to draw gets cleaned (via
//! `cleanup_zombies()` inside the test binary), and the others silently
//! accumulate trash over time.

use lineark_sdk::Client;
use lineark_test_utils::{all_test_tokens, cleanup_workspace};

#[tokio::main]
async fn main() {
    let tokens = all_test_tokens();
    let total = tokens.len();
    eprintln!("cleanup: starting sweep of {total} pooled workspace(s)");

    for (idx, token) in tokens.into_iter().enumerate() {
        let n = idx + 1;
        // Last 6 chars only — same triage tag as `test_token` logs, so a
        // failure here lines up visually with what the test process used.
        let tag = token.get(token.len().saturating_sub(6)..).unwrap_or("?");
        eprintln!("cleanup: workspace {n}/{total} (token …{tag})");
        match Client::from_token(token) {
            Ok(client) => cleanup_workspace(&client).await,
            Err(e) => eprintln!("cleanup: workspace {n}/{total} client init failed: {e}"),
        }
    }

    eprintln!("cleanup: done ({total} workspace(s))");
}
