use std::future::Future;

/// Wait for the Linear API to propagate recently created resources.
/// Linear is eventually consistent — created resources may not be queryable immediately.
pub async fn settle() {
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
}

/// Retry a create operation persistently on Linear's transient "conflict on
/// insert" / "already exists" errors.
///
/// Linear's API has a known cold-start failure mode where freshly-generated
/// UUIDs spuriously collide for the first several attempts of a fresh test
/// process, then abruptly start working. Per-test persistence (rather than
/// a suite-level retry that would waste cycles re-running passing tests)
/// is the right answer.
///
/// 15 attempts with backoffs `[0, 2, 5, 10, 20, 30, 60, 60, 60, 60, 60, 60,
/// 60, 60, 60]` seconds (~9 min worst case). Mirrors the CLI helper
/// `run_lineark_with_retry` in `crates/lineark/tests/online.rs`.
///
/// Note: unlike the CLI helper, this can't mutate the request body between
/// attempts (the closure captures input by reference). Callers that
/// generate input bodies inside the closure can vary the body themselves
/// for finer control; otherwise we're relying on persistence alone.
pub async fn retry_create<T, F, Fut>(mut f: F) -> T
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, lineark_sdk::LinearError>>,
{
    const BACKOFFS: &[u64] = &[0, 2, 5, 10, 20, 30, 60, 60, 60, 60, 60, 60, 60, 60, 60];
    let mut last_msg = String::new();
    for (attempt, &wait) in BACKOFFS.iter().enumerate() {
        if wait > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
        }
        match f().await {
            Ok(val) => return val,
            Err(e) => {
                let msg = e.to_string();
                if !msg.contains("conflict on insert") && !msg.contains("already exists") {
                    panic!("create failed with non-transient error: {msg}");
                }
                last_msg = msg;
                eprintln!("retry_create: attempt {attempt} failed with transient error, retrying");
            }
        }
    }
    panic!("create failed after {} retries: {last_msg}", BACKOFFS.len());
}

/// Retry a search operation with generous backoff for Linear's eventually-consistent search index.
/// Returns `Some(result)` on the first attempt where `predicate` returns true, or `None` after exhausting retries.
pub async fn retry_search<T, F, Fut, P>(mut f: F, mut predicate: P) -> Option<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, lineark_sdk::LinearError>>,
    P: FnMut(&T) -> bool,
{
    for i in 0..12 {
        tokio::time::sleep(std::time::Duration::from_secs(if i < 3 { 2 } else { 5 })).await;
        let result = match f().await {
            Ok(v) => v,
            Err(_) => continue, // rate-limited or transient error — retry
        };
        if predicate(&result) {
            return Some(result);
        }
    }
    None
}

/// Retry a closure up to `max_attempts` times with exponential backoff.
/// Delays: 0s, 1s, 2s, 4s, 8s, 10s, 10s, ... (capped at 10s).
/// Returns `Ok(T)` on the first successful attempt, or `Err(last_error_message)`.
pub fn retry_with_backoff<T, F>(max_attempts: u32, mut f: F) -> Result<T, String>
where
    F: FnMut() -> Result<T, String>,
{
    let mut last_err = String::new();
    for attempt in 0..max_attempts {
        let delay = if attempt == 0 {
            0
        } else {
            std::cmp::min(1u64 << (attempt - 1), 10)
        };
        if delay > 0 {
            std::thread::sleep(std::time::Duration::from_secs(delay));
        }
        match f() {
            Ok(val) => return Ok(val),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}
