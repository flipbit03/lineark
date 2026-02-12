# LINEARK MASTERPLAN

## Vision

**lineark** is an unofficial Linear ecosystem for Rust, consisting of two crates:

- **`lineark-sdk`** — A typed, async-first Rust SDK for the Linear GraphQL API
- **`lineark`** — A CLI that serves both humans and LLMs, powered by `lineark-sdk`

The CLI is designed to be the primary interface for LLM agents interacting with Linear — compact, self-documenting via `--help`, JSON output by default in non-interactive contexts, and zero-config if `~/.linear_api_token` exists.

The SDK and its generated types are maintained via a Rust codegen tool that reads Linear's public GraphQL schema. Schema updates are handled by a CI cron job that fetches the latest schema, regenerates code, and opens a PR. A developer (or an LLM session) reviews and merges.

---

## Repository Structure

Single monorepo, Cargo workspace:

```
lineark/
├── Cargo.toml                    # [workspace] root
├── docs/
│   └── MASTERPLAN.md             # This file
├── crates/
│   ├── lineark-sdk/              # Library crate -> published to crates.io
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # Public API surface
│   │       ├── client.rs         # LinearClient (async + blocking)
│   │       ├── error.rs          # Error types (mirroring Linear's error taxonomy)
│   │       ├── pagination.rs     # Connection<T>, PageInfo, cursor helpers
│   │       ├── auth.rs           # Token resolution (~/.linear_api_token, env, flag)
│   │       └── generated/        # ALL codegen output lives here
│   │           ├── mod.rs
│   │           ├── types.rs      # Object types (Team, Issue, User, Project, etc.)
│   │           ├── inputs.rs     # Input types (IssueCreateInput, IssueUpdateInput, etc.)
│   │           ├── enums.rs      # Enums (IssueRelationType, PriorityLevel, etc.)
│   │           ├── scalars.rs    # Custom scalar mappings (DateTime->chrono, JSON->Value)
│   │           ├── queries.rs    # Query functions (teams, issues, viewer, etc.)
│   │           └── mutations.rs  # Mutation functions (create_issue, update_issue, etc.)
│   │
│   ├── lineark/                  # Binary crate -> published to crates.io + binary releases
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/         # One module per command group
│   │       │   ├── mod.rs
│   │       │   ├── issues.rs     # issues list|read|search|create|update
│   │       │   ├── comments.rs   # comments create
│   │       │   ├── teams.rs      # teams list
│   │       │   ├── users.rs      # users list
│   │       │   ├── projects.rs   # projects list
│   │       │   ├── cycles.rs     # cycles list|read
│   │       │   ├── labels.rs     # labels list
│   │       │   ├── embeds.rs     # embeds download|upload
│   │       │   ├── documents.rs  # documents list|read|create|update|delete
│   │       │   └── usage.rs      # Compact LLM-friendly command reference
│   │       └── output.rs         # Human (tables/color) vs JSON formatting
│   │
│   └── lineark-codegen/          # Binary crate, NOT published (internal tool)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs           # CLI: reads schema.graphql, writes generated/*.rs
│           ├── parser.rs         # Wraps apollo-parser, extracts type info
│           ├── emit_types.rs     # Generates types.rs
│           ├── emit_inputs.rs    # Generates inputs.rs
│           ├── emit_enums.rs     # Generates enums.rs
│           ├── emit_scalars.rs   # Generates scalars.rs
│           ├── emit_queries.rs   # Generates queries.rs
│           └── emit_mutations.rs # Generates mutations.rs
│
├── schema/
│   ├── schema.graphql            # Checked-in copy of Linear's public schema
│   └── operations.toml           # Allowlist of which queries/mutations to generate
│
└── .github/
    └── workflows/
        ├── ci.yml                # Build + test on every push/PR
        ├── release.yml           # cargo-dist generated: build binaries + publish
        └── schema-update.yml     # Weekly cron: fetch schema, codegen, open PR
```

---

## Crate Details

### lineark-sdk

**Published as:** `lineark-sdk` on crates.io
**Purpose:** Rust library for interacting with the Linear API

**Dependencies:**
```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }

[features]
default = []
blocking = ["reqwest/blocking"]
```

**Async by default, blocking opt-in:**
- `lineark_sdk::Client` — async (requires tokio runtime)
- `lineark_sdk::blocking::Client` — sync (behind `blocking` feature flag)
- Both expose identical APIs, differing only in async/sync

**Client API shape (async):**
```rust
use lineark_sdk::Client;

let client = Client::from_token("lin_api_...")?;
// or
let client = Client::from_env()?;      // LINEAR_API_TOKEN env var
// or
let client = Client::from_file()?;     // ~/.linear_api_token
// or
let client = Client::auto()?;          // tries file -> env (same precedence as CLI)

let me = client.viewer().await?;
let teams = client.teams().await?;
let issue = client.issue("ENG-123").await?;
let issues = client.issues()
    .team("Engineering")
    .status("In Progress")
    .limit(25)
    .await?;
let created = client.create_issue(IssueCreateInput {
    title: "Fix the thing".into(),
    team_id: team.id.clone(),
    ..Default::default()
}).await?;
```

**Error types** mirror Linear's taxonomy:
- `LinearError::Authentication`
- `LinearError::RateLimited { retry_after, .. }`
- `LinearError::InvalidInput { message, .. }`
- `LinearError::Forbidden`
- `LinearError::Network { .. }`
- `LinearError::GraphQL { errors: Vec<GraphQLError> }`

**Pagination** follows Relay cursor spec:
```rust
// Manual pagination
let page1 = client.issues().limit(50).await?;
let page2 = client.issues().limit(50).after(page1.page_info.end_cursor).await?;

// Auto-pagination (collects all pages)
let all_issues = client.issues().all().await?;
```

### lineark (CLI)

**Published as:** `lineark` on crates.io + binary releases on GitHub
**Purpose:** Human + LLM friendly Linear CLI

**Dependencies:**
```toml
[dependencies]
lineark-sdk = { path = "../lineark-sdk", version = "0.1" }
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde_json = "1"
tabled = "0.17"       # Human-readable table output
colored = "2"          # Terminal colors
```

**Output format auto-detection:**
```
stdout is a terminal  -> human-readable (tables, colors)
stdout is piped/file  -> JSON (one object per result)
--format human        -> force human output
--format json         -> force JSON output
```

Detection uses `std::io::IsTerminal` from Rust stdlib (no external crate).

**Authentication precedence:**
```
1. --api-token <token>     (CLI flag, highest priority)
2. $LINEAR_API_TOKEN        (environment variable)
3. ~/.linear_api_token      (file, linearis-compatible)
```

**Command structure** (linearis-inspired, clap-powered):

```
lineark issues list [--team NAME] [--status NAME] [--assignee NAME] [--limit N]
lineark issues read <IDENTIFIER>        # e.g. ENG-123
lineark issues search <QUERY> [--team NAME] [--project NAME]
lineark issues create <TITLE> --team NAME [--assignee ID] [--labels L1,L2] [--priority 0-3] [--description TEXT]
lineark issues update <IDENTIFIER> [--status NAME] [--priority 0-3] [--labels L1,L2] [--assignee ID] [--parent ID]

lineark comments create <ISSUE-ID> --body <TEXT>

lineark embeds download <URL> [--output PATH] [--overwrite]
lineark embeds upload <FILE>

lineark documents list [--project NAME] [--issue ID]
lineark documents read <ID>
lineark documents create --title TEXT --content TEXT [--project NAME] [--attach-to ISSUE-ID]
lineark documents update <ID> [--title TEXT] [--content TEXT]
lineark documents delete <ID>

lineark teams list
lineark users list [--active]
lineark projects list
lineark labels list [--team NAME]
lineark cycles list [--team NAME] [--active] [--limit N]
lineark cycles read <ID-OR-NAME> [--team NAME]

lineark viewer                          # Who am I? (token info)
lineark usage                           # Compact command reference (LLM-optimized, <1000 tokens)
```

Every command supports `--help` with full descriptions, argument docs, and examples. This is the LLM's entry point — it reads `lineark usage` or `lineark issues --help` and knows exactly what to do.

### lineark-codegen (internal)

**NOT published.** Lives in the workspace for development convenience.

**Purpose:** Read `schema/schema.graphql` -> emit `crates/lineark-sdk/src/generated/*.rs`

**Approach:**
1. Parse schema using the `apollo-parser` crate (modern, error-resilient GraphQL parser that produces a CST)
2. Walk all `ObjectTypeDefinition` -> emit Rust structs with `#[derive(Debug, Clone, Serialize, Deserialize)]`
3. Walk all `InputObjectTypeDefinition` -> emit input structs with `Default` impl
4. Walk all `EnumTypeDefinition` -> emit Rust enums with serde rename
5. Walk all `ScalarTypeDefinition` -> emit type aliases (DateTime->chrono::DateTime, JSON->serde_json::Value, etc.)
6. Walk `Query` type fields -> emit async query functions with builder pattern
7. Walk `Mutation` type fields -> emit async mutation functions
8. Format output with `prettyplease` (Rust code formatter crate, no need for external rustfmt)

**Key dependency:** `apollo-parser` — modern, actively maintained GraphQL parser from Apollo. Produces a lossless CST with error recovery. Replaces `apollo-parser` which is less maintained.

**Run as:**
```bash
cargo run -p lineark-codegen
# Reads schema/schema.graphql
# Writes crates/lineark-sdk/src/generated/*.rs
```

---

## Codegen Strategy

### What gets generated

The Linear GraphQL schema contains approximately:
- 485 object types -> Rust structs
- 337 input types -> Rust structs with `Default`
- 72 enums -> Rust enums
- 16 custom scalars -> type aliases
- ~292 root query fields -> query functions
- ~285 root mutation fields -> mutation functions

### Incremental operation support

**Not all operations are generated from day one.** The codegen emits all types/enums/inputs (they're needed for type completeness), but query and mutation functions are gated by an allowlist in the codegen config.

```
schema/operations.toml    # Controls which queries/mutations to generate
```

```toml
[queries]
# Phase 1
viewer = true
teams = true
team = true
users = true
issues = true
issue = true
projects = true
project = true
cycles = true
cycle = true
labels = true
# Phase 2+: add more as needed

[mutations]
# Phase 2
issueCreate = true
issueUpdate = true
commentCreate = true
# Phase 3+: add more as needed
```

Types and enums are always fully generated (they're cheap and needed for completeness). Operations are added incrementally as the CLI needs them.

### Scalar mapping

| GraphQL Scalar | Rust Type |
|---|---|
| `String` | `String` |
| `Int` | `i64` |
| `Float` | `f64` |
| `Boolean` | `bool` |
| `ID` | `String` |
| `DateTime` | `chrono::DateTime<chrono::Utc>` |
| `TimelessDate` | `chrono::NaiveDate` |
| `JSON` | `serde_json::Value` |
| `JSONObject` | `serde_json::Map<String, serde_json::Value>` |
| `UUID` | `String` (keep it simple) |

### GraphQL query generation

Each query/mutation function embeds its GraphQL query string as a constant. The codegen determines which fields to request based on the object type's scalar and enum fields (not nested objects — those require separate queries, keeping responses lean).

```rust
// Generated example
impl Client {
    pub async fn teams(&self) -> Result<Connection<Team>, LinearError> {
        const QUERY: &str = r#"
            query Teams($first: Int, $after: String) {
                teams(first: $first, after: $after) {
                    nodes { id name key description color }
                    pageInfo { hasNextPage endCursor }
                }
            }
        "#;
        // ... execute and deserialize
    }
}
```

---

## Distribution

### crates.io

Both `lineark-sdk` and `lineark` are published to crates.io:
```bash
cargo install lineark           # CLI
cargo add lineark-sdk            # Library
```

### Binary releases via cargo-dist

Targets:
| Target | GHA Runner | Method |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` | Native build |
| `aarch64-unknown-linux-gnu` | `ubuntu-24.04-arm` | Native build (ARM runner) |
| `aarch64-apple-darwin` | `macos-latest` | Native build (Apple Silicon runner) |

Installers generated by cargo-dist:
- **Shell script** (`curl | sh`) for Linux/macOS
- **Homebrew formula** via a `cadu/tap` repository

```toml
# Root Cargo.toml or dist-workspace.toml
[workspace.metadata.dist]
cargo-dist-version = "0.27.0"
ci = "github"
installers = ["shell", "homebrew"]
targets = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "aarch64-apple-darwin",
]

[workspace.metadata.dist.github-custom-runners]
aarch64-unknown-linux-gnu = "ubuntu-24.04-arm"
```

### Release workflow

Using `release-plz` + `cargo-dist`:
1. `release-plz` monitors main branch, bumps versions based on conventional commits, opens a Release PR
2. Merging the PR publishes `lineark-sdk` then `lineark` to crates.io, creates git tags
3. Tags trigger `cargo-dist` workflow: builds binaries for all 3 targets, creates GitHub Release with artifacts, updates Homebrew formula

---

## CI Workflows

### ci.yml — Every push/PR
```
- cargo fmt --check
- cargo clippy -- -D warnings
- cargo test --workspace
- cargo build --workspace
```

### release.yml — Generated by cargo-dist
Triggered by version tags. Builds binaries, publishes crates, creates GitHub Release.

### schema-update.yml — Weekly cron
```
1. Fetch schema: curl https://api.linear.app/graphql (introspection query)
2. Compare with schema/schema.graphql
3. If changed:
   a. Update schema/schema.graphql
   b. Run: cargo run -p lineark-codegen
   c. Run: cargo build --workspace (verify it compiles)
   d. Run: cargo test --workspace
   e. Open PR with title "chore: update Linear schema [automated]"
4. If unchanged: exit 0
```

PR review can be done by a human or an LLM session:
```
"Review the schema update PR, fix any compilation errors in hand-written code,
and ensure all tests pass."
```

---

## Roadmap

### Phase 1 — Foundation + Core Reads

**Goal:** Working codegen, SDK with read operations, CLI that can list/read the most important entities.

**Workspace setup (#1):**
- [x] Create root `Cargo.toml` with `[workspace]` and `members = ["crates/*"]` (#1)
- [x] Create `crates/lineark-sdk/Cargo.toml` with dependencies (reqwest, tokio, serde, serde_json, chrono) (#1)
- [x] Create `crates/lineark/Cargo.toml` with dependencies (lineark-sdk, clap, tokio, serde_json, tabled, colored) (#1)
- [x] Create `crates/lineark-codegen/Cargo.toml` with dependencies (apollo-parser, prettyplease, toml) (#1)
- [x] Verify `cargo build --workspace` compiles with empty lib.rs/main.rs stubs (#1)

**Schema acquisition (#2):**
- [x] Fetch Linear's public GraphQL schema via introspection query (#2)
- [x] Save as `schema/schema.graphql` (#2)
- [x] Create initial `schema/operations.toml` with Phase 1 query allowlist (viewer, teams, team, users, issues, issue, projects, project, cycles, cycle, labels) (#2)

**Codegen — type generation (#3):**
- [x] Implement `parser.rs`: parse `schema.graphql` with `apollo-parser`, extract all type definitions into a structured intermediate representation (#3)
- [x] Implement `emit_scalars.rs`: map GraphQL custom scalars to Rust types (DateTime->chrono, JSON->serde_json::Value, etc.) (#3)
- [x] Implement `emit_enums.rs`: generate Rust enums with `#[derive(Debug, Clone, Serialize, Deserialize)]` and serde rename for all 72 GraphQL enums (#3)
- [x] Implement `emit_types.rs`: generate Rust structs for all ~485 object types (scalar + enum fields only, skip nested objects) (#3)
- [x] Implement `emit_inputs.rs`: generate Rust input structs for all ~337 input types with `Default` impl and `Option<T>` for optional fields (#3)
- [x] Implement `main.rs` for codegen: wire up parser + emitters, read schema file, write `crates/lineark-sdk/src/generated/*.rs` (#3)
- [x] Format generated output with `prettyplease` (#3)
- [x] Run codegen and verify generated code compiles: `cargo run -p lineark-codegen && cargo build -p lineark-sdk` (#3)

**Codegen — query generation (#4):**
- [x] Implement `emit_queries.rs`: for each allowed query in `operations.toml`, generate an async function on `Client` that embeds the GraphQL query string and deserializes the response (#4)
- [x] Implement `emit_mutations.rs`: same pattern for mutations (empty for Phase 1, but the infrastructure must exist) (#4)
- [x] Generate GraphQL query strings that select scalar + enum fields of the return type, plus `pageInfo` for connection types (#4)
- [x] Re-run codegen, verify everything compiles (#4)

**SDK core — hand-written (#5):**
- [x] Implement `auth.rs`: token resolution — read `~/.linear_api_token` file, `$LINEAR_API_TOKEN` env var, or accept token directly (#5)
- [x] Implement `client.rs`: `Client` struct wrapping `reqwest::Client`, with `from_token()`, `from_env()`, `from_file()`, `auto()` constructors (#5)
- [x] Implement HTTP transport: POST to `https://api.linear.app/graphql` with JSON body `{ query, variables }`, parse response `{ data, errors }` (#5)
- [x] Implement `error.rs`: `LinearError` enum with variants for Authentication, RateLimited, InvalidInput, Forbidden, Network, GraphQL (#5)
- [x] Implement rate limit handling: parse `retry-after`, `x-ratelimit-*` headers from error responses (#5)
- [x] Implement `pagination.rs`: `Connection<T>` struct with `nodes: Vec<T>`, `PageInfo { has_next_page, end_cursor }` (#5) — note: `.all()` auto-paginator deferred to Phase 2
- [x] Implement `lib.rs`: public re-exports of Client, error types, generated types/enums/inputs, pagination types (#5)
- [x] Verify SDK compiles and public API surface is clean (#5)

**CLI skeleton (#6):**
- [x] Implement `main.rs`: tokio async main, clap derive for top-level args (`--api-token`, `--format`) (#6)
- [x] Implement `output.rs`: detect `std::io::stdout().is_terminal()`, format as human tables or JSON accordingly; support `--format human|json` override (#6)
- [x] Implement auth resolution in CLI: `--api-token` flag > `$LINEAR_API_TOKEN` env > `~/.linear_api_token` file (#6)

**CLI commands:**
- [x] Implement `commands/teams.rs`: `lineark teams list` (#7)
- [x] Implement `commands/users.rs`: `lineark users list [--active]` (#8)
- [x] Implement `commands/projects.rs`: `lineark projects list` (#9)
- [x] Implement `commands/labels.rs`: `lineark labels list` (#10)
- [x] Implement `commands/cycles.rs`: `lineark cycles list [--limit N]` and `lineark cycles read <ID>` (#11)
- [x] Implement `commands/issues.rs`: `lineark issues list [--team KEY] [--mine] [--limit N] [--show-done]` (#12)
- [x] Implement `commands/issues.rs`: `lineark issues read <IDENTIFIER>` (supports ABC-123 smart identifier resolution) (#12)
- [x] Implement `commands/issues.rs`: `lineark issues search <QUERY> [--limit N] [--show-done]` (#12)
- [x] Implement viewer command: `lineark whoami` (who am I) (#13)
- [x] Implement `commands/usage.rs`: compact LLM-friendly command reference (<1000 tokens) (#14)
- [x] Ensure every command and subcommand has comprehensive `--help` text via clap doc comments (#15)

**Testing (#16):**
- [x] Unit tests for codegen: verify generated Rust code for a small test schema matches expected output (#16)
- [x] Unit tests for auth: token file reading, env var reading, precedence (#16)
- [x] Unit tests for error parsing: verify LinearError is correctly constructed from various API error shapes (#16)
- [x] Integration tests for SDK: mock HTTP responses, verify deserialization of teams/issues/users/etc. (#16)
- [x] CLI output tests: verify JSON output structure, verify human output is reasonable (#16)

**Phase 1 acceptance criteria (#17):**
- [x] `cargo install lineark` works (or `cargo run -p lineark --`) (#17)
- [x] `lineark whoami` returns current user info (#17) — renamed from `lineark viewer`
- [x] `lineark teams list` returns all teams (#17)
- [x] `lineark issues list --team X` returns issues (#17)
- [x] `lineark issues read E-931` returns issue details (#17)
- [x] JSON output when piped (`lineark teams list | jq .`) (#17)
- [x] Human table output when interactive (#17)
- [x] Auth from `~/.linear_api_token` with no flags needed (#17)

---

### Phase 2 — Core Writes

**Goal:** Create and update the most important entities.

**Codegen updates (#18):**
- [x] Add mutation operations to `operations.toml`: `issueCreate`, `issueUpdate`, `issueArchive`, `commentCreate` (#18)
- [x] Re-run codegen, verify mutation functions are generated and compile (#18)

**CLI write commands:**
- [x] Implement `lineark issues create <TITLE> --team NAME [--assignee ID] [--labels L1,L2] [--priority 0-4] [--description TEXT]` (#19)
- [x] Implement `lineark issues update <IDENTIFIER> [--status NAME] [--priority 0-4] [--labels L1,L2] [--assignee ID] [--parent ID]` (#20)
- [x] Implement label management: `--labels` with `--label-by adding|replacing|removing` and `--clear-labels` (#20)
- [x] Implement priority support: `--priority 0-4` (0=no priority, 1=urgent, 2=high, 3=medium, 4=low) (#20)
- [x] Implement status updates: `--status "Status Name"` (resolve status name against team's workflow states) (#20)
- [x] Implement parent-child linking: `--parent IDENTIFIER` (#20)
- [x] Implement `lineark comments create <ISSUE-ID> --body <TEXT>` (#21)

**Testing (#22):**
- [x] Integration tests for mutations: mock API, verify correct GraphQL mutation is sent with expected variables (#22)
- [x] CLI tests for create/update: verify output format and error handling (#22)

**Phase 2 acceptance criteria (#23):**
- [x] Can create an issue: `lineark issues create "Fix bug" --team Engineering --priority 2` (#23)
- [x] Can update an issue: `lineark issues update ENG-123 --status "In Progress" --assignee user-id` (#23)
- [x] Can comment on an issue: `lineark comments create ENG-123 --body "Working on it"` (#23)
- [x] Write operations return the created/updated entity in the same JSON/human format as reads (#23)

---

### Phase 3 — Rich Features

**Goal:** File handling, documents, broader entity support.

**Embeds (#24):**
- [ ] Implement `lineark embeds download <URL> [--output PATH] [--overwrite]` (handle Linear's signed/expiring URLs) (#24)
- [ ] Implement `lineark embeds upload <FILE>` (multipart upload, return asset URL in JSON) (#24)
- [ ] Add embed info to issue read output (list of attachments with URLs) (#24)

**Documents (#25):**
- [ ] Add document query/mutation operations to `operations.toml` (#25)
- [ ] Re-run codegen (#25)
- [ ] Implement `lineark documents list [--project NAME] [--issue ID]` (#25)
- [ ] Implement `lineark documents read <ID>` (#25)
- [ ] Implement `lineark documents create --title TEXT --content TEXT [--project NAME] [--attach-to ISSUE-ID]` (#25)
- [ ] Implement `lineark documents update <ID> [--title TEXT] [--content TEXT]` (#25)
- [ ] Implement `lineark documents delete <ID>` (#25)

**SDK blocking API (#26):**
- [ ] Implement `lineark_sdk::blocking::Client` behind `blocking` feature flag (#26)
- [ ] Mirror all async methods as blocking equivalents (#26)
- [ ] Test blocking API independently (#26)

**Additional operations — as needed (#27):**
- [ ] Issue relations (blocking, related, duplicate) (#27)
- [ ] Issue attachments listing (#27)
- [ ] Any other operations that surface as needed during real usage (#27)

**Phase 3 acceptance criteria (#28):**
- [ ] Can download issue attachments to local files (#28)
- [ ] Can upload files and reference them in comments (#28)
- [ ] Full document CRUD works (#28)
- [ ] `lineark-sdk` usable with `features = ["blocking"]` for sync consumers (#28)
- [ ] Feature parity with linearis (#28)

---

### Phase 4 — Polish + Distribution

**Goal:** Production-ready distribution and developer experience.

**cargo-dist setup (#29):**
- [ ] Run `cargo dist init` in workspace root (#29)
- [ ] Configure targets: x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, aarch64-apple-darwin (#29)
- [ ] Configure installers: shell, homebrew (#29)
- [ ] Configure custom runner for aarch64 linux: `ubuntu-24.04-arm` (#29)
- [ ] Verify generated `release.yml` workflow builds all targets (#29)
- [ ] Test a release end-to-end (tag, build, GitHub Release with artifacts) (#29)

**release-plz setup (#30):**
- [ ] Add `release-plz` GitHub Action workflow (#30)
- [ ] Configure to publish `lineark-sdk` before `lineark` (dependency ordering) (#30)
- [ ] Configure to create git tags that trigger cargo-dist (#30)
- [ ] Test automated version bump and release PR flow (#30)

**Schema update automation (#31):**
- [ ] Write `schema-update.yml` cron workflow (weekly) (#31)
- [ ] Implement GraphQL introspection query fetch step (#31)
- [ ] Diff against checked-in `schema/schema.graphql` (#31)
- [ ] If changed: run codegen, build, test, open PR (#31)
- [ ] Test the workflow end-to-end (#31)

**Homebrew (#32):**
- [ ] Create `cadu/homebrew-tap` repository (#32)
- [ ] Configure cargo-dist to publish formula there (#32)
- [ ] Verify `brew install cadu/tap/lineark` works (#32)

**Shell completions (#33):**
- [ ] Enable clap shell completion generation (bash, zsh, fish) (#33)
- [ ] Include completions in binary releases or document `lineark completions <shell>` command (#33)

**Documentation (#34):**
- [ ] Write comprehensive README.md with: project overview, installation methods, quick start, SDK usage examples, CLI usage examples (#34)
- [ ] Ensure all CLI commands have thorough `--help` text (#34)

**CI workflow (#35):**
- [x] Create `.github/workflows/ci.yml` with fmt, clippy, test, build (#35)

**Publish (#36):**
- [ ] Publish `lineark-sdk` to crates.io (#36)
- [ ] Publish `lineark` to crates.io (#36)
- [ ] Create first GitHub Release with binaries (#36)

**Phase 4 acceptance criteria (#37):**
- [ ] `brew install cadu/tap/lineark` works on macOS (#37)
- [ ] `curl | sh` installer works on Linux (#37)
- [ ] `cargo install lineark` works (#37)
- [ ] Weekly schema update cron opens PRs when Linear's schema changes (#37)
- [ ] Shell completions available for bash, zsh, fish (#37)

---

## Design Principles

1. **LLM-first, human-friendly.** JSON by default when piped. Human tables when interactive. `--help` is the documentation. `usage` fits in one LLM context.

2. **Codegen is king.** Hand-written code is kept under 2000 lines across the entire SDK. Everything else is generated. This makes schema updates a codegen run, not a rewrite.

3. **Incremental by design.** Types are always complete. Operations are added via an allowlist. The CLI grows command by command, not all at once.

4. **LLM-maintainable.** The codegen tool, the SDK runtime, and the CLI are each simple enough for a single Claude session to fully understand and modify. No clever abstractions, no deep inheritance hierarchies, no macro magic.

5. **Zero config for existing Linear users.** If `~/.linear_api_token` exists (from linearis or manual setup), lineark works immediately.

6. **Async-first, sync-optional.** The SDK defaults to async (tokio + reqwest). The `blocking` feature flag enables sync for consumers who need it. The CLI uses async internally.

---

## License

MIT

## Author

Built by **[cadu](https://github.com/flipbit03)**. Unofficial — not affiliated with Linear.
