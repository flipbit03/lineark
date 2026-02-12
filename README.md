# lineark

Unofficial [Linear](https://linear.app) CLI and Rust SDK — for humans and LLMs.

## Install

### Pre-built binary (fastest)

```sh
curl -fsSL https://raw.githubusercontent.com/flipbit03/lineark/main/install.sh | sh
```

Installs to `~/.local/bin`. Override with `LINEARK_INSTALL_DIR`:

```sh
curl -fsSL https://raw.githubusercontent.com/flipbit03/lineark/main/install.sh | LINEARK_INSTALL_DIR=/usr/local/bin sh
```

### Via cargo

```sh
cargo install lineark
```

### Download binary manually

Grab a binary from the [latest release](https://github.com/flipbit03/lineark/releases/latest):

| Platform | Asset |
|---|---|
| Linux x86_64 | `lineark_linux_x86_64` |
| Linux aarch64 | `lineark_linux_aarch64` |
| macOS aarch64 (Apple Silicon) | `lineark_macos_aarch64` |

## Auth

Create a [Linear API token](https://linear.app/settings/api) and save it:

```sh
echo "lin_api_..." > ~/.linear_api_token
```

Or use an environment variable:

```sh
export LINEAR_API_TOKEN="lin_api_..."
```

Or pass it directly:

```sh
lineark --api-token "lin_api_..." teams list
```

## Usage

```
lineark whoami                                    Show authenticated user
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

## Output

- **Terminal** → human-readable tables
- **Piped** → JSON

Override with `--format human` or `--format json`.

## SDK

Use `lineark-sdk` as a library in your own Rust projects:

```sh
cargo add lineark-sdk
```

```rust
use lineark_sdk::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::auto()?;

    let me = client.viewer().await?;
    println!("{:?}", me);

    let teams = client.teams(None, None, None, None, None).await?;
    for team in &teams.nodes {
        println!("{}: {}",
            team.key.as_deref().unwrap_or("?"),
            team.name.as_deref().unwrap_or("?"),
        );
    }

    Ok(())
}
```

## License

MIT
