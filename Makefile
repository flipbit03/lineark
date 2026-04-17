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
#
# Linear's API has a known transient failure mode on `*Create` mutations
# (returns "conflict on insert" with a UUID it just generated, with no
# matching record server-side — confirmed by `read` returning "not found").
# The per-call helper in tests/online.rs retries with body mutation, but the
# cold-start window is sometimes longer than the 8-attempt budget there.
# Wrap the whole suite in a 3x retry so a single unlucky test doesn't sink CI.
test-online:
	cargo run -p lineark-test-utils --bin cleanup-test-workspace
	@for attempt in 1 2 3; do \
		echo ">>> test-online attempt $$attempt/3"; \
		if cargo test --workspace --test online -- --test-threads=1; then \
			echo ">>> test-online passed on attempt $$attempt"; exit 0; \
		fi; \
		echo ">>> test-online attempt $$attempt failed; cleaning up before retry"; \
		cargo run -p lineark-test-utils --bin cleanup-test-workspace; \
		sleep 30; \
	done; \
	echo ">>> test-online failed after 3 attempts"; exit 1
