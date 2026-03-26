.PHONY: codegen update-schema check test test-online

# Fetch the latest Linear GraphQL schema + regenerate SDK types.
# No API key required — Linear's introspection endpoint is public.
update-schema:
	cargo run -p lineark-codegen -- --fetch

# Run codegen from the local schema (no fetch).
codegen:
	cargo run -p lineark-codegen

# Lint, doc, and build checks (no tests).
check:
	cargo fmt --check
	cargo run -q -p lineark-lint
	cargo clippy --workspace -- -D warnings
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
	cargo build --workspace
	cargo test --workspace --no-run

# Run offline tests (unit + integration). Safe, fast, no API token needed.
test:
	cargo test --workspace --lib
	cargo test --workspace --test offline

# Run online tests against the live Linear API. Requires ~/.linear_api_token_test.
# Cleans the test workspace before running to avoid stale resource conflicts.
# test_with's custom harness runs tests sequentially and aborts on first panic.
test-online:
	cargo run -p lineark-test-utils --bin cleanup-test-workspace
	cargo test --workspace --test online -- --test-threads=1
