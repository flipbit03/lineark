use colored::Colorize;
use serde::Serialize;
use std::io::IsTerminal;
use tabled::settings::Style;
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
            let table = Table::new(items).with(Style::blank()).to_string();
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
            let value = serde_json::to_value(item).unwrap();
            print_value_human(&value, 0);
        }
    }
}

fn print_value_human(value: &serde_json::Value, indent: usize) {
    let pad = "  ".repeat(indent);
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                match val {
                    serde_json::Value::Null => {}
                    serde_json::Value::Object(_) => {
                        println!("{}{}:", pad, key.bold().cyan());
                        print_value_human(val, indent + 1);
                    }
                    serde_json::Value::Array(arr) => {
                        println!("{}{}:", pad, key.bold().cyan());
                        for item in arr {
                            print_value_human(item, indent + 1);
                            println!("{}  ---", pad);
                        }
                    }
                    _ => {
                        let display = match val {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        println!("{}{}: {}", pad, key.bold().cyan(), display);
                    }
                }
            }
        }
        serde_json::Value::String(s) => println!("{}{}", pad, s),
        other => println!("{}{}", pad, other),
    }
}
