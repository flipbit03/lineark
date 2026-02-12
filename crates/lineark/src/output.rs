use serde::Serialize;
use std::io::IsTerminal;
use tabled::{Table, Tabled};

/// Output format selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    Human,
    Json,
}

/// Determine the output format based on the user's choice and terminal detection.
pub fn resolve_format(format: Option<Format>) -> Format {
    match format {
        Some(f) => f,
        None => {
            if std::io::stdout().is_terminal() {
                Format::Human
            } else {
                Format::Json
            }
        }
    }
}

/// Print data in the resolved format.
/// `T` must implement both `Serialize` (for JSON) and `Tabled` (for human output).
pub fn print_table<T: Serialize + Tabled>(items: &[T], format: Format) {
    match format {
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(items).unwrap());
        }
        Format::Human => {
            if items.is_empty() {
                println!("No results.");
                return;
            }
            let table = Table::new(items).to_string();
            println!("{}", table);
        }
    }
}

/// Print a single item in the resolved format.
pub fn print_one<T: Serialize>(item: &T, format: Format) {
    match format {
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(item).unwrap());
        }
        Format::Human => {
            // For single items, pretty-print as JSON even in human mode.
            // This is the most readable format for detailed views.
            println!("{}", serde_json::to_string_pretty(item).unwrap());
        }
    }
}
