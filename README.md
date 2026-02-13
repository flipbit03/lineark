# lineark / lineark-sdk

Unofficial [Linear](https://linear.app) CLI and Rust SDK — for humans and LLMs.

[![CI](https://github.com/flipbit03/lineark/actions/workflows/ci.yml/badge.svg)](https://github.com/flipbit03/lineark/actions/workflows/ci.yml)
[![crates.io lineark](https://img.shields.io/crates/v/lineark?label=lineark)](https://crates.io/crates/lineark)
[![crates.io lineark-sdk](https://img.shields.io/crates/v/lineark-sdk?label=lineark-sdk)](https://crates.io/crates/lineark-sdk)
[![CLI downloads](https://img.shields.io/crates/d/lineark?label=CLI%20downloads)](https://crates.io/crates/lineark)
[![SDK downloads](https://img.shields.io/crates/d/lineark-sdk?label=SDK%20downloads)](https://crates.io/crates/lineark-sdk)
[![docs.rs](https://img.shields.io/docsrs/lineark-sdk?label=docs.rs)](https://docs.rs/lineark-sdk)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Getting Started

Create a [Linear API token](https://linear.app/settings/account/security) (Settings > Security & Access > Personal API Keys) and save it:

```sh
echo "lin_api_..." > ~/.linear_api_token
```

Or use an environment variable:

```sh
export LINEAR_API_TOKEN="lin_api_..."
```

Then proceed to install the [CLI](#cli-lineark) or [SDK](#sdk-lineark-sdk).

## CLI: `lineark`

### Install

#### Pre-built binary (fastest)

```sh
curl -fsSL https://raw.githubusercontent.com/flipbit03/lineark/main/install.sh | sh
```

Run it again to update to the latest version.

#### Via cargo

```sh
cargo install lineark
```

Run it again to upgrade to the latest version.

#### Download binary manually

Grab a binary from the [latest release](https://github.com/flipbit03/lineark/releases/latest).

### Usage

```
lineark whoami                                   Show authenticated user
lineark teams list                               List all teams
lineark users list [--active]                    List users
lineark projects list                            List all projects
lineark labels list                              List issue labels
lineark cycles list [--limit N] [--team KEY]     List cycles
  [--active]                                     Only the active cycle
  [--around-active N]                            Active ± N neighbors
lineark cycles read <ID> [--team KEY]            Read cycle (UUID, name, or number)
lineark issues list [--limit N] [--team KEY]     Active issues, newest first
  [--mine]                                       Only issues assigned to me
  [--show-done]                                  Include done/canceled issues
lineark issues read <IDENTIFIER>                 Full issue detail (e.g., E-929)
lineark issues search <QUERY> [--limit N]        Full-text search
  [--show-done]                                  Include done/canceled results
lineark issues create <TITLE> --team KEY         Create an issue
  [--priority 0-4] [--assignee ID]               0=none 1=urgent 2=high 3=medium 4=low
  [--labels ID,...] [--description TEXT]          Comma-separated label UUIDs
  [--status NAME] [--parent ID]                  Status resolved against team states
lineark issues update <IDENTIFIER>               Update an issue
  [--status NAME] [--priority 0-4]               Status resolved against team states
  [--assignee ID] [--parent ID]                  User UUID or issue identifier
  [--labels ID,...] [--label-by adding|replacing|removing]
  [--clear-labels] [--title TEXT] [--description TEXT]
lineark issues archive <IDENTIFIER>              Archive an issue
lineark issues unarchive <IDENTIFIER>            Unarchive a previously archived issue
lineark issues delete <IDENTIFIER>               Delete (trash) an issue
  [--permanently]                                Permanently delete instead of trashing
lineark comments create <ISSUE-ID> --body TEXT   Comment on an issue
lineark documents list [--limit N]               List documents
lineark documents read <ID>                      Read document (includes content)
lineark documents create --title TEXT             Create a document
  [--content TEXT] [--project ID] [--issue ID]
lineark documents update <ID>                    Update a document
  [--title TEXT] [--content TEXT]
lineark documents delete <ID>                    Delete (trash) a document
lineark embeds upload <FILE> [--public]          Upload file, returns asset URL
lineark embeds download <URL>                    Download a file by URL
  [--output PATH] [--overwrite]
lineark usage                                    Compact command reference
```

Every command supports `--help` for full details.

Output auto-detects format (tables in terminal, JSON when piped) — override with `--format {human,json}`.

### LLM / AI Agent Setup

Add this to your LLM's context (e.g. `CLAUDE.md`, `.cursorrules`, system prompt):

```
We track our tickets and projects in Linear (https://linear.app), a project management tool.
We use the `lineark` CLI tool for communicating with Linear. Use your Bash tool to call the
`lineark` executable. Run `lineark usage` to see usage information.
```

`lineark usage` gives your agent a complete command reference in under 1,000 tokens.

## SDK: `lineark-sdk`

Use `lineark-sdk` as a library in your own Rust projects:

```sh
cargo add lineark-sdk
```

```rust
use lineark_sdk::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // auto() tries all token methods (env var, ~/.linear_api_token file)
    let client = Client::auto()?;

    let me = client.whoami().await?;
    println!("{:?}", me);

    // Queries with optional args use a builder pattern:
    let teams = client.teams().first(10).include_archived(false).send().await?;
    for team in &teams.nodes {
        println!("{}: {}",
            team.key.as_deref().unwrap_or("?"),
            team.name.as_deref().unwrap_or("?"),
        );
    }

    // Search uses a required `term` arg + optional builder params:
    let results = client.search_issues("bug").first(5).send().await?;
    println!("Found {} issues", results.nodes.len());

    Ok(())
}
```

## License

MIT
