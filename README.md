# lineark

Unofficial Linear CLI and Rust SDK — for humans and LLMs.

[![CI](https://github.com/flipbit03/lineark/actions/workflows/ci.yml/badge.svg)](https://github.com/flipbit03/lineark/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/lineark?label=lineark)](https://crates.io/crates/lineark)
[![crates.io](https://img.shields.io/crates/v/lineark-sdk?label=lineark-sdk)](https://crates.io/crates/lineark-sdk)
[![docs.rs](https://img.shields.io/docsrs/lineark-sdk?label=docs.rs)](https://docs.rs/lineark-sdk)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Why lineark?

MCP tools are the standard way to connect AI agents to external services. But they come at a cost: **the Linear MCP server alone consumes ~13,000 tokens** of context just to describe its tools and schemas to the model — before your agent does any actual work.

For context-constrained tools like Claude Code, that's a huge tax. Claude Code's own system prompt and tools already use ~20K tokens, leaving a tight budget for your code, your conversation, and the agent's reasoning.

lineark takes a different approach: it's a **CLI that your agent calls via Bash**. There's no tool schema to inject. When your agent needs the command reference, it runs `lineark usage` and gets everything in **under 1,000 tokens**. That's a **~13x reduction** in context overhead compared to the MCP approach — context your agent can spend on actually understanding your codebase and solving problems.

It's also a standalone **Rust SDK** ([lineark-sdk](https://crates.io/crates/lineark-sdk)) for building your own Linear integrations.

## Quick start

### Install

```sh
curl -fsSL https://raw.githubusercontent.com/flipbit03/lineark/main/install.sh | sh
```

Or via cargo: `cargo install lineark`

To update to the latest version:

```sh
lineark self update
```

### Authenticate

Create a [Linear Personal API key](https://linear.app/settings/account/security) and save it:

```sh
echo "lin_api_..." > ~/.linear_api_token
```

### Use it

```sh
lineark whoami                              # check your identity
lineark issues list --team ENG --mine       # my issues on the ENG team
lineark issues search "auth bug" -l 5       # full-text search
lineark issues create "Fix login" --team ENG -p 2 --assignee "Jane"
lineark issues update ENG-42 -s "In Progress"
```

Output auto-detects format — human-readable tables in a terminal, JSON when piped. Override with `--format json`.

## Set up your AI agent

Add three lines to your LLM's context (`CLAUDE.md`, `.cursorrules`, system prompt, etc.):

```
We track our tickets and projects in Linear (https://linear.app), a project management tool.
We use the `lineark` CLI tool for communicating with Linear. Use your Bash tool to call the
`lineark` executable. Run `lineark usage` to see usage information.
```

That's it. Your agent discovers all commands at runtime via `lineark usage` — no tool schemas, no function definitions, no context bloat.

## What it can do

| Area | Commands |
|------|----------|
| **Issues** | `list`, `read`, `search`, `create`, `update`, `archive`, `delete` |
| **Comments** | `create` on any issue |
| **Projects** | `list` |
| **Milestones** | `list`, `read`, `create`, `update`, `delete` |
| **Cycles** | `list`, `read` |
| **Documents** | `list`, `read`, `create`, `update`, `delete` |
| **Teams / Users / Labels** | `list` |
| **File embeds** | `upload`, `download` |

Every command supports `--help` for full details. Most flags accept human-readable names — `--team ENG`, `--assignee "Jane Doe"`, `--labels "Bug,P0"` — no UUIDs required.

Run `lineark usage` for the complete command reference.

## SDK: lineark-sdk

Use lineark-sdk as a library in your own Rust projects:

```sh
cargo add lineark-sdk
```

```rust
use lineark_sdk::Client;
use lineark_sdk::generated::types::{User, Team, IssueSearchResult};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::auto()?;

    let me = client.whoami::<User>().await?;
    println!("{:?}", me);

    let teams = client.teams::<Team>().first(10).send().await?;
    for team in &teams.nodes {
        println!("{}: {}", team.key.as_deref().unwrap_or("?"), team.name.as_deref().unwrap_or("?"));
    }

    let results = client.search_issues::<IssueSearchResult>("bug").first(5).send().await?;
    println!("Found {} issues", results.nodes.len());

    Ok(())
}
```

All query methods are generic over `T: DeserializeOwned + GraphQLFields` — define custom lean structs with `#[derive(GraphQLFields)]` to fetch only the fields you need.

## Architecture

lineark is four crates:

- **lineark-codegen** — reads Linear's GraphQL schema and generates typed Rust code
- **lineark-sdk** — generated types + hand-written core (client, auth, pagination)
- **lineark-derive** — `#[derive(GraphQLFields)]` for custom lean types with zero overfetching
- **lineark** — the CLI, a pure SDK consumer with no raw GraphQL

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for diagrams and detailed walkthrough.

## License

MIT
