.PHONY: codegen update-schema check test

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

# Run tests only.
test:
	cargo test --workspace
