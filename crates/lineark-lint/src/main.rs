//! Column alignment checker for command reference sections.
//!
//! Checks that description columns are consistently aligned in:
//! - `crates/lineark/src/commands/usage.rs` (COMMANDS: → GLOBAL OPTIONS:)
//! - `crates/lineark/README.md` (bare code block after ## Usage)

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process;
use tabled::settings::Style;
use tabled::{Table, Tabled};

struct Diagnostic {
    file: String,
    line_number: usize,
    line: String,
    found_column: usize,
    expected_column: usize,
}

/// Find the 1-indexed column where the description starts.
///
/// Scans left-to-right for the **last** gap of 2+ consecutive spaces
/// between a non-whitespace char and an `[A-Z0-9]` char.
fn find_description_column(line: &str) -> Option<usize> {
    let chars: Vec<char> = line.chars().collect();
    let mut result = None;

    for i in 0..chars.len() {
        if !chars[i].is_whitespace() {
            let gap_start = i + 1;
            if gap_start < chars.len() && chars[gap_start] == ' ' {
                let mut gap_end = gap_start;
                while gap_end < chars.len() && chars[gap_end] == ' ' {
                    gap_end += 1;
                }
                if gap_end - gap_start >= 2 && gap_end < chars.len() {
                    let c = chars[gap_end];
                    if c.is_ascii_uppercase() || c.is_ascii_digit() {
                        result = Some(gap_end + 1); // 1-indexed
                    }
                }
            }
        }
    }

    result
}

/// Most frequent value in a slice.
fn compute_mode(values: &[usize]) -> usize {
    let mut counts: HashMap<usize, usize> = HashMap::new();
    for &v in values {
        *counts.entry(v).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(val, _)| val)
        .unwrap_or(0)
}

/// Check alignment of description columns. Returns diagnostics for outliers.
#[cfg(test)]
fn check_alignment(lines: &[(usize, &str)], file: &str) -> Vec<Diagnostic> {
    check_alignment_detailed(lines, file).2
}

/// Like check_alignment but also returns (lines_checked, majority_column, diagnostics).
fn check_alignment_detailed(
    lines: &[(usize, &str)],
    file: &str,
) -> (usize, Option<usize>, Vec<Diagnostic>) {
    let entries: Vec<(usize, &str, usize)> = lines
        .iter()
        .filter_map(|&(ln, content)| find_description_column(content).map(|col| (ln, content, col)))
        .collect();

    if entries.is_empty() {
        return (0, None, vec![]);
    }

    let columns: Vec<usize> = entries.iter().map(|e| e.2).collect();
    let mode = compute_mode(&columns);

    let diagnostics = entries
        .iter()
        .filter(|e| e.2 != mode)
        .map(|&(ln, content, col)| Diagnostic {
            file: file.to_string(),
            line_number: ln,
            line: content.to_string(),
            found_column: col,
            expected_column: mode,
        })
        .collect();

    (entries.len(), Some(mode), diagnostics)
}

struct CheckResult {
    file: &'static str,
    section: &'static str,
    lines_checked: usize,
    majority_column: Option<usize>,
    diagnostics: Vec<Diagnostic>,
}

/// Extract and check lines between COMMANDS: and GLOBAL OPTIONS: in usage.rs.
fn check_usage_rs(root: &Path) -> CheckResult {
    let rel = "crates/lineark/src/commands/usage.rs";
    let content = read_file(root, rel);

    let mut in_section = false;
    let mut section_lines = Vec::new();

    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed == "COMMANDS:" {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with("GLOBAL OPTIONS:") {
            break;
        }
        if in_section {
            section_lines.push((i + 1, line));
        }
    }

    let (lines_checked, majority_column, diagnostics) =
        check_alignment_detailed(&section_lines, rel);
    CheckResult {
        file: rel,
        section: "COMMANDS: → GLOBAL OPTIONS:",
        lines_checked,
        majority_column,
        diagnostics,
    }
}

/// Extract and check lines in bare code block after ## Usage in README.md.
fn check_readme_md(root: &Path) -> CheckResult {
    let rel = "crates/lineark/README.md";
    let content = read_file(root, rel);

    let mut found_usage = false;
    let mut in_code_block = false;
    let mut section_lines = Vec::new();

    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed == "## Usage" {
            found_usage = true;
            continue;
        }
        if found_usage && !in_code_block && trimmed == "```" {
            in_code_block = true;
            continue;
        }
        if in_code_block && trimmed == "```" {
            break;
        }
        if in_code_block {
            section_lines.push((i + 1, line));
        }
    }

    let (lines_checked, majority_column, diagnostics) =
        check_alignment_detailed(&section_lines, rel);
    CheckResult {
        file: rel,
        section: "code block after ## Usage",
        lines_checked,
        majority_column,
        diagnostics,
    }
}

fn read_file(root: &Path, rel: &str) -> String {
    let path = root.join(rel);
    match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: cannot read {rel}: {e}");
            process::exit(2);
        }
    }
}

fn print_diagnostic(d: &Diagnostic) {
    eprintln!(
        "error: description column misaligned in {}:{}",
        d.file, d.line_number
    );
    eprintln!(
        "  found column {}, expected {} (majority)",
        d.found_column, d.expected_column
    );
    eprintln!("  {}", d.line);
    eprintln!("{}^ expected here", " ".repeat(d.expected_column + 1));
}

fn find_workspace_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return Some(dir);
                }
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn main() {
    let root = find_workspace_root().unwrap_or_else(|| {
        eprintln!("error: cannot find workspace root (no Cargo.toml with [workspace])");
        process::exit(2);
    });

    let results = [check_usage_rs(&root), check_readme_md(&root)];

    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "File")]
        file: &'static str,
        #[tabled(rename = "Section")]
        section: &'static str,
        #[tabled(rename = "Lines")]
        lines: String,
        #[tabled(rename = "Column")]
        column: String,
        #[tabled(rename = "Status")]
        status: &'static str,
    }

    let mut rows = Vec::new();
    let mut total_diagnostics = 0;
    for r in &results {
        rows.push(Row {
            file: r.file,
            section: r.section,
            lines: r.lines_checked.to_string(),
            column: r.majority_column.map_or("-".into(), |c| c.to_string()),
            status: if r.diagnostics.is_empty() {
                "ok"
            } else {
                "FAIL"
            },
        });
        total_diagnostics += r.diagnostics.len();
    }

    println!("Column alignment:");
    println!("{}", Table::new(&rows).with(Style::rounded()));

    if total_diagnostics == 0 {
        // already printed per-file "ok" status
    } else {
        eprintln!();
        for r in &results {
            for d in &r.diagnostics {
                print_diagnostic(d);
                eprintln!();
            }
        }
        eprintln!("error: {} alignment issue(s) found", total_diagnostics);
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_desc_col_basic() {
        // abc(3) + 7 spaces + Desc → D at char index 10 → column 11
        let line = "abc       Desc";
        assert_eq!(find_description_column(line), Some(11));
    }

    #[test]
    fn find_desc_col_digit_start() {
        let line = "abc       0=none";
        assert_eq!(find_description_column(line), Some(11));
    }

    #[test]
    fn find_desc_col_no_desc() {
        let line = "    [--cycle NAME-OR-ID]";
        assert_eq!(find_description_column(line), None);
    }

    #[test]
    fn find_desc_col_continuation_line() {
        // Leading spaces only — no non-ws char before the gap
        let line = "                         Embed as markdown";
        assert_eq!(find_description_column(line), None);
    }

    #[test]
    fn find_desc_col_multi_byte() {
        // em dash (U+2014) is 3 bytes in UTF-8 but 1 char
        // a(1) \u{2014}(2) b(3) + 7 spaces + D at column 11
        let line = "a\u{2014}b       Desc";
        assert_eq!(find_description_column(line), Some(11));
    }

    #[test]
    fn find_desc_col_empty() {
        assert_eq!(find_description_column(""), None);
    }

    #[test]
    fn find_desc_col_last_gap_wins() {
        // Two qualifying gaps — the last one wins
        // A(1)A(2) 2×' ' B(5)B(6) 2×' ' C(9)C(10)
        let line = "AA  BB  CC";
        assert_eq!(find_description_column(line), Some(9));
    }

    #[test]
    fn mode_basic() {
        assert_eq!(compute_mode(&[50, 50, 50, 51, 49]), 50);
    }

    #[test]
    fn mode_single() {
        assert_eq!(compute_mode(&[42]), 42);
    }

    #[test]
    fn alignment_outlier() {
        let lines = vec![
            (1, "abc       Desc A"),  // col 11
            (2, "defgh     Desc B"),  // col 11
            (3, "ij         Desc C"), // col 12 (outlier)
        ];
        let diags = check_alignment(&lines, "test.txt");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line_number, 3);
        assert_eq!(diags[0].found_column, 12);
        assert_eq!(diags[0].expected_column, 11);
    }

    #[test]
    fn alignment_all_ok() {
        let lines = vec![
            (1, "abc       Desc A"), // col 11
            (2, "defgh     Desc B"), // col 11
        ];
        let diags = check_alignment(&lines, "test.txt");
        assert!(diags.is_empty());
    }

    #[test]
    fn alignment_skips_no_desc_lines() {
        let lines = vec![
            (1, "abc       Desc A"),  // col 11
            (2, "    [--some-flag]"), // no desc, skipped
            (3, "defgh     Desc B"),  // col 11
        ];
        let diags = check_alignment(&lines, "test.txt");
        assert!(diags.is_empty());
    }
}
