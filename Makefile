.PHONY: fetch-schema codegen update-schema check

# Fetch the latest Linear GraphQL schema + regenerate SDK types.
# No API key required — Linear's introspection endpoint is public.
update-schema:
	cargo run -p lineark-codegen -- --fetch

# Run codegen from the local schema (no fetch).
codegen:
	cargo run -p lineark-codegen

# Run all CI checks locally.
check:
	cargo fmt --check
	cargo clippy --workspace -- -D warnings
	cargo build --workspace
	cargo test --workspace
