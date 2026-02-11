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
cargo build --workspace                  # build everything
cargo test --workspace                   # run all tests
cargo run -p lineark-codegen             # regenerate SDK types from schema
cargo run -p lineark -- <args>           # run the CLI
cargo clippy --workspace -- -D warnings  # lint
cargo fmt --check                        # format check
```

## Conventions

- **All generated code** lives in `crates/lineark-sdk/src/generated/`. Never hand-edit these files — they are overwritten by codegen.
- **Hand-written SDK code** (client, auth, error, pagination) should stay under 2000 lines total. Keep it simple and LLM-readable.
- **Codegen uses `graphql-parser`** crate to parse the schema SDL into a typed Rust AST, then emits `.rs` files formatted with `prettyplease`.
- **Operations are incremental.** Types/enums/inputs are always fully generated. Query and mutation functions are gated by `schema/operations.toml`.
- **Auth precedence:** `--api-token` flag > `$LINEAR_API_TOKEN` env var > `~/.linear_api_token` file.
- **Output format:** auto-detect with `std::io::IsTerminal` — human tables when interactive, JSON when piped. Override with `--format human|json`.
- **Async by default.** The SDK uses tokio + reqwest async. A `blocking` feature flag exposes a sync API.
- **No macro magic.** No proc macros in the SDK itself. Codegen emits plain Rust structs and functions.

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
| Schema parsing | `graphql-parser` (in codegen only) |
| Code formatting | `prettyplease` (in codegen only) |
