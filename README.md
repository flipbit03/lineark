# lineark / lineark-sdk

Unofficial [Linear](https://linear.app) CLI and Rust SDK — for humans and LLMs.

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

#### Via cargo

```sh
cargo install lineark
```

#### Download binary manually

Grab a binary from the [latest release](https://github.com/flipbit03/lineark/releases/latest).

### Usage

```
lineark whoami                                   Show authenticated user
lineark teams list                               List all teams
lineark users list [--active]                    List users
lineark projects list                            List all projects
lineark labels list                              List issue labels
lineark cycles list [--limit N]                  List cycles
lineark cycles read <ID>                         Read a specific cycle
lineark issues list [--limit N] [--team KEY]     Active issues, newest first
  [--mine]                                       Only issues assigned to me
  [--show-done]                                  Include done/canceled issues
lineark issues read <IDENTIFIER>                 Full issue detail (e.g., E-929)
lineark issues search <QUERY> [--limit N]        Full-text search
  [--show-done]                                  Include done/canceled results
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

    let me = client.viewer().await?;
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
