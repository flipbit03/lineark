# CLAUDE.md — lineark

## What is this

lineark is an unofficial Linear (issue tracker) ecosystem for Rust:
- **lineark-sdk** (`crates/lineark-sdk/`) — Async-first Rust SDK for the Linear GraphQL API
- **lineark** (`crates/lineark/`) — CLI for humans and LLMs, powered by lineark-sdk
- **lineark-codegen** (`crates/lineark-codegen/`) — Internal tool that reads `schema/schema.graphql` and generates typed Rust code into `crates/lineark-sdk/src/generated/`

See `docs/MASTERPLAN.md` for the full architecture, roadmap, and design decisions (huge file, only read if needed, to keep token consumption down in day-to-day work).

## Workspace layout

```
Cargo.toml              # workspace root
crates/
  lineark-sdk/          # library (published to crates.io)
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
make check                               # run all CI checks locally
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
- **Async by default.** The SDK uses tokio + reqwest async. A `blocking` feature flag exposes a sync API.
- **No macro magic.** No proc macros in the SDK itself. Codegen emits plain Rust structs and functions.

## CLI discoverability

- **`lineark usage`** is the LLM entry point — a compact (<1000 tokens) command reference. Keep it small for token efficiency. Only add flags/commands here when they're important enough that an LLM should know about them upfront.
- **`--help`** on every command/subcommand is the detailed reference. All flags, descriptions, defaults, and examples go here via clap doc comments. Commands should be fully self-discoverable via `--help`.
- Rule of thumb: `usage` = "what can I do?", `--help` = "how exactly does this work?"

## PR guidelines

Every PR must pass CI before merge. The checks are:
1. `cargo fmt --check` — formatting
2. `cargo clippy --workspace -- -D warnings` — linting
3. `cargo build --workspace` — compilation
4. `cargo test --workspace` — tests

These run on five targets: `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-gnu`, `aarch64-unknown-linux-musl`, `aarch64-apple-darwin`.

When opening a PR, include a summary of changes and a test plan. If codegen was modified, verify with `cargo run -p lineark-codegen` that the generated output is clean (codegen runs `cargo fmt` as a post-step).

## Commit style

- Use conventional commits (`feat:`, `fix:`, `chore:`, `docs:`, etc.)
- No `Co-Authored-By` lines
- No "Generated with Claude Code" footers
- Keep messages concise, focused on "why" not "what"

## What NOT to do

- Don't hand-edit files in `crates/lineark-sdk/src/generated/` — run codegen instead
- Don't add proc macro dependencies to the SDK crate
- Don't add webhook support, MCP server, or raw GraphQL escape hatches — these are out of scope
- Don't generate operations not listed in `schema/operations.toml` — operations are added incrementally
- Don't break linearis compatibility for auth (keep `~/.linear_api_token` support)

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
| Schema parsing | `apollo-parser` (in codegen only) |
| Code formatting | `prettyplease` (in codegen only) |
