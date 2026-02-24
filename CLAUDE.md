# CLAUDE.md — lineark

## What is this

lineark is an unofficial Linear (issue tracker) ecosystem for Rust:
- **lineark-sdk** (`crates/lineark-sdk/`) — Async-first Rust SDK for the Linear GraphQL API
- **lineark-derive** (`crates/lineark-derive/`) — Proc macro crate providing `#[derive(GraphQLFields)]` for type-driven field selection
- **lineark** (`crates/lineark/`) — CLI for humans and LLMs, powered by lineark-sdk
- **lineark-codegen** (`crates/lineark-codegen/`) — Internal tool that reads `schema/schema.graphql` and generates typed Rust code into `crates/lineark-sdk/src/generated/`

See `docs/MASTERPLAN.md` for the full architecture, roadmap, and design decisions (huge file, only read if needed, to keep token consumption down in day-to-day work).

## Workspace layout

```
Cargo.toml              # workspace root
crates/
  lineark-sdk/          # library (published to crates.io)
  lineark-derive/       # proc macro for #[derive(GraphQLFields)]
  lineark/              # CLI binary (published to crates.io + binary releases)
  lineark-codegen/      # codegen tool (not published)
schema/
  schema.graphql        # Linear's GraphQL schema (checked in, fetched from API)
  operations.toml       # allowlist of which queries/mutations to generate
docs/
  MASTERPLAN.md         # full project plan and roadmap
```

## Key commands

```bash
rustup update stable                     # sync local toolchain with CI (do this before developing)
cargo build --workspace                  # build everything
cargo test --workspace                   # run all tests
cargo run -p lineark-codegen             # regenerate SDK types from schema
cargo run -p lineark -- <args>           # run the CLI
cargo clippy --workspace -- -D warnings  # lint
cargo fmt --check                        # format check
make check                               # lint + doc + build (no tests)
make test                                # run tests only
```

**Online tests must run serially.** They hit the live Linear API, which has plan-level limits (e.g. max teams). Running them in parallel causes spurious failures from resource exhaustion. Always use `-- --test-threads=1` when running online tests locally:

```bash
cargo test -p lineark-sdk --test online -- --test-threads=1
cargo test -p lineark --test online -- --test-threads=1
```

## Updating the schema

`schema/schema.graphql` is a vendored copy of Linear's public GraphQL schema (SDL). It's checked in for reproducible builds and reviewable diffs. To update it:

```bash
make update-schema    # fetch latest schema + regenerate SDK types
# or equivalently:
cargo run -p lineark-codegen -- --fetch
```

This fetches the schema via introspection (no API key needed — Linear's endpoint is public), writes `schema/schema.graphql`, then runs codegen to regenerate `crates/lineark-sdk/src/generated/`. Pure Rust, no external tools.

To regenerate without fetching (e.g. after editing `operations.toml`):

```bash
make codegen
```

After updating, fix any compilation errors caused by schema changes, then `make check`.

There's a Claude Code command for the full workflow (fetch + codegen + fix breakage + lint):

```
/update-linear-schema
```

## Conventions

- **All generated code** lives in `crates/lineark-sdk/src/generated/`. Never hand-edit these files — they are overwritten by codegen.
- **Hand-written SDK code** (client, auth, error, pagination) should stay under 2000 lines total. Keep it simple and LLM-readable.
- **Codegen uses `apollo-parser`** crate to parse the schema SDL into a CST, then emits `.rs` files formatted with `prettyplease`.
- **Operations are incremental.** Types/enums/inputs are always fully generated. Query and mutation functions are gated by `schema/operations.toml`.
- **Auth precedence:** `--api-token` flag > `$LINEAR_API_TOKEN` env var > `~/.linear_api_token` file.
- **Output format:** auto-detect with `std::io::IsTerminal` — human tables when interactive, JSON when piped. Override with `--format human|json`.
- **Async by default.** The SDK uses tokio + reqwest async. A `blocking` feature flag exposes a sync API via the `blocking_client` module.
- **Generic queries.** All query and mutation functions are generic over `T: DeserializeOwned + GraphQLFields`. Generated types have auto-generated `impl GraphQLFields` from codegen. Consumers can define custom lean types with `#[derive(GraphQLFields)]` to avoid overfetching.
- **Minimal proc macros.** The only proc macro is `#[derive(GraphQLFields)]` in `lineark-derive` — it enables consumer-defined lean types for field selection. Codegen emits plain Rust structs and `impl GraphQLFields` blocks.

## CLI discoverability

- **`lineark usage`** is the LLM entry point — a compact (<1000 tokens) command reference. Keep it small for token efficiency. Only add flags/commands here when they're important enough that an LLM should know about them upfront.
- **`--help`** on every command/subcommand is the detailed reference. All flags, descriptions, defaults, and examples go here via clap doc comments. Commands should be fully self-discoverable via `--help`.
- Rule of thumb: `usage` = "what can I do?", `--help` = "how exactly does this work?"

## PR guidelines

Every PR must pass CI before merge. The checks are:
1. `cargo fmt --check` — formatting
2. `cargo clippy --workspace -- -D warnings` — linting
3. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` — doc lints
4. `cargo build --workspace` — compilation
5. `cargo test --workspace` — tests

Locally: `make check && make test` runs all of the above.

These run on five targets: `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-gnu`, `aarch64-unknown-linux-musl`, `aarch64-apple-darwin`.

When opening a PR, include a summary of changes and a test plan. If codegen was modified, verify with `cargo run -p lineark-codegen` that the generated output is clean (codegen runs `cargo fmt` as a post-step).

Before merging, run `/update-docs` to review and update all documentation (top-level README, CLI README, SDK README) so they reflect the current codebase. Documentation must stay in sync with code.

## Commit style

- Use conventional commits (`feat:`, `fix:`, `chore:`, `docs:`, etc.)
- No `Co-Authored-By` lines
- No "Generated with Claude Code" footers
- Keep messages concise, focused on "why" not "what"

## Versioning

All crates use `version = "0.0.0"` in their Cargo.toml — this is intentional. It means "current dev". Actual versions are set dynamically at release time by the GitHub Actions release workflow (`.github/workflows/release.yml`), which is triggered by publishing a GitHub Release. The workflow:

1. Extracts the version from the release tag (`v1.2.3` → `1.2.3`)
2. Patches all `0.0.0` placeholders via `sed` (workspace version + inter-crate dep versions)
3. Publishes in dependency order: `lineark-derive` → `lineark-sdk` → `lineark`
4. Builds platform binaries and uploads them to the release

Never manually set version numbers in Cargo.toml files. To release, create a GitHub Release with a semver tag like `v0.1.0` (via the GitHub UI or `gh release create v0.1.0 --generate-notes`).

## Issue tracking

Issue tracking for lineark development happens in GitHub Issues, not in Linear. Use `gh issue` commands to view, create, and manage issues.

## What NOT to do

- Don't hand-edit files in `crates/lineark-sdk/src/generated/` — run codegen instead
- Don't add proc macro dependencies beyond `lineark-derive`
- Don't add webhook support, MCP server, or raw GraphQL escape hatches — these are out of scope
- Don't generate operations not listed in `schema/operations.toml` — operations are added incrementally
- Don't remove `~/.linear_api_token` auth support

## Dependencies (key choices, don't change without good reason)

| Purpose | Crate |
|---|---|
| HTTP client | `reqwest` (async + blocking feature) |
| Async runtime | `tokio` |
| Serialization | `serde` + `serde_json` |
| Date/time | `chrono` |
| CLI framework | `clap` (derive) |
| Table output | `tabled` |
| Terminal colors | `colored` |
| Derive macro | `lineark-derive` (workspace crate) |
| Schema parsing | `apollo-parser` (in codegen only) |
| Code formatting | `prettyplease` (in codegen only) |
