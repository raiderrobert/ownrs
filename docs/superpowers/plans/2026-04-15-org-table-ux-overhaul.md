# Org Table UX Overhaul Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the `ownrs org` table output with a kubectl-style padded-column layout, make the detail view the default, add flexible sorting, a names export format, and BDD test coverage.

**Architecture:** The CLI args (`cli.rs`) change to replace `--detail`/`SortOrder` with `--summary`/`--wide`/column-based `--sort`. The table renderer (`output/table.rs`) is rewritten to use custom padded-column formatting instead of comfy-table. `main.rs` and `config.rs` are updated to wire the new flags. BDD tests via cucumber-rs validate the output layer with canned data.

**Tech Stack:** Rust, clap (CLI), cucumber-rs + futures (BDD), unicode-width (column alignment)

**Spec:** `docs/superpowers/specs/2026-04-15-org-table-ux-overhaul.md`

---

### Task 1: Set up BDD infrastructure

**Files:**
- Create: `tests/bdd.rs`
- Create: `tests/features/default_table.feature`
- Modify: `Cargo.toml` (add dev-dependencies)

- [ ] **Step 1: Add dev-dependencies to Cargo.toml**

Add to the end of `Cargo.toml`:

```toml
[dev-dependencies]
cucumber = "0.21"
futures = "0.3"
```

- [ ] **Step 2: Create the BDD test runner**

Create `tests/bdd.rs`:

```rust
use cucumber::{given, then, when, World};

use ownrs::output::table::{render_table, TableOptions};
use ownrs::reconcile::types::{AlignmentStatus, RepoOwnership};

use chrono::{NaiveDate, TimeZone, Utc};

#[derive(Debug, Default, World)]
pub struct OwnrsWorld {
    repos: Vec<RepoOwnership>,
    team_filter: Option<String>,
    stdout: String,
}

fn parse_status(s: &str) -> AlignmentStatus {
    match s {
        "aligned" => AlignmentStatus::Aligned,
        "mismatched" => AlignmentStatus::Mismatched,
        "catalog-only" => AlignmentStatus::CatalogOnly,
        "codeowners-only" => AlignmentStatus::CodeownersOnly,
        "admin-only" => AlignmentStatus::AdminOnly,
        "stale" => AlignmentStatus::Stale,
        "missing" => AlignmentStatus::Missing,
        other => panic!("unknown status: {other}"),
    }
}

fn parse_date(s: &str) -> Option<chrono::DateTime<Utc>> {
    if s == "-" || s.is_empty() {
        return None;
    }
    let nd = NaiveDate::parse_from_str(s, "%Y-%m-%d").expect("bad date");
    Some(Utc.from_utc_datetime(&nd.and_hms_opt(0, 0, 0).unwrap()))
}

fn parse_list(s: &str) -> Vec<String> {
    if s == "-" || s.is_empty() {
        return vec![];
    }
    s.split(", ").map(|t| t.trim().to_string()).collect()
}

fn parse_optional(s: &str) -> Option<String> {
    if s == "-" || s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

#[given(regex = "the following repos:")]
fn given_repos(world: &mut OwnrsWorld, step: &cucumber::Step) {
    let table = step.table.as_ref().expect("step must have a table");
    let headers: Vec<&str> = table.rows[0].iter().map(|s| s.as_str()).collect();

    let col = |name: &str| -> usize {
        headers.iter().position(|h| *h == name).unwrap_or_else(|| panic!("missing column: {name}"))
    };

    for row in &table.rows[1..] {
        let status = parse_status(&row[col("status")]);
        let catalog_owner = parse_optional(&row[col("catalog_owner")]);
        let codeowners_teams = parse_list(&row[col("codeowners_teams")]);
        let admin_teams = if headers.contains(&"admin_teams") {
            parse_list(&row[col("admin_teams")])
        } else {
            vec![]
        };
        let pushed_at = parse_date(&row[col("pushed_at")]);
        let notes = if headers.contains(&"notes") {
            let n = row[col("notes")].trim();
            if n.is_empty() || n == "-" {
                vec![]
            } else {
                vec![n.to_string()]
            }
        } else {
            vec![]
        };

        world.repos.push(RepoOwnership {
            repo_name: row[col("repo_name")].clone(),
            pushed_at,
            catalog_owner: catalog_owner.clone(),
            codeowners_teams: codeowners_teams.clone(),
            catalog_team_exists: catalog_owner.as_ref().map(|_| true),
            codeowners_teams_exist: codeowners_teams.iter().map(|t| (t.clone(), true)).collect(),
            admin_teams,
            alignment: status,
            notes,
            suggested_owners: None,
        });
    }
}

#[given(expr = "the team filter is {string}")]
fn given_team_filter(world: &mut OwnrsWorld, filter: String) {
    world.team_filter = Some(filter);
}

#[when("I render the table")]
fn render_default(world: &mut OwnrsWorld) {
    let opts = TableOptions {
        wide: false,
        sort_columns: vec!["repo".to_string()],
        team_filter: world.team_filter.clone(),
    };
    world.stdout = render_table(&world.repos, &opts);
}

#[when(expr = "I render the table with {string}")]
fn render_with_flags(world: &mut OwnrsWorld, flags: String) {
    let mut wide = false;
    let mut sort_columns = vec!["repo".to_string()];

    let parts: Vec<&str> = flags.split_whitespace().collect();
    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            "--wide" => wide = true,
            "--sort" => {
                i += 1;
                sort_columns = parts[i].split(',').map(|s| s.to_string()).collect();
            }
            "--summary" => {
                // summary rendering tested separately
            }
            _ => {}
        }
        i += 1;
    }

    let opts = TableOptions {
        wide,
        sort_columns,
        team_filter: world.team_filter.clone(),
    };
    world.stdout = render_table(&world.repos, &opts);
}

#[when(expr = "I render with {string}")]
fn render_format(world: &mut OwnrsWorld, flags: String) {
    if flags.contains("--format names") {
        let mut names: Vec<String> = world.repos.iter().map(|r| r.repo_name.clone()).collect();
        names.sort();
        world.stdout = names.join("\n") + "\n";
    }
}

#[then(expr = "stdout should contain {string}")]
fn stdout_contains(world: &mut OwnrsWorld, expected: String) {
    assert!(
        world.stdout.contains(&expected),
        "expected stdout to contain {expected:?}, got:\n{}",
        world.stdout
    );
}

#[then(expr = "stdout should not contain {string}")]
fn stdout_not_contains(world: &mut OwnrsWorld, unexpected: String) {
    assert!(
        !world.stdout.contains(&unexpected),
        "expected stdout NOT to contain {unexpected:?}, but it did:\n{}",
        world.stdout
    );
}

#[then(expr = "the first data row should start with {string}")]
fn first_data_row(world: &mut OwnrsWorld, expected: String) {
    let data_lines: Vec<&str> = world
        .stdout
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with("repos(") && !l.chars().next().unwrap_or(' ').is_uppercase() && !l.contains(" · "))
        .collect();
    assert!(
        !data_lines.is_empty(),
        "no data rows found in:\n{}",
        world.stdout
    );
    let first = data_lines[0].trim();
    assert!(
        first.starts_with(&expected),
        "expected first row to start with {expected:?}, got: {first:?}"
    );
}

#[then(expr = "the second data row should start with {string}")]
fn second_data_row(world: &mut OwnrsWorld, expected: String) {
    let data_lines: Vec<&str> = world
        .stdout
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with("repos(") && !l.chars().next().unwrap_or(' ').is_uppercase() && !l.contains(" · "))
        .collect();
    assert!(data_lines.len() >= 2, "fewer than 2 data rows");
    let second = data_lines[1].trim();
    assert!(
        second.starts_with(&expected),
        "expected second row to start with {expected:?}, got: {second:?}"
    );
}

#[then(expr = "the third data row should start with {string}")]
fn third_data_row(world: &mut OwnrsWorld, expected: String) {
    let data_lines: Vec<&str> = world
        .stdout
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with("repos(") && !l.chars().next().unwrap_or(' ').is_uppercase() && !l.contains(" · "))
        .collect();
    assert!(data_lines.len() >= 3, "fewer than 3 data rows");
    let third = data_lines[2].trim();
    assert!(
        third.starts_with(&expected),
        "expected third row to start with {expected:?}, got: {third:?}"
    );
}

#[then(expr = "the sort indicator should be on {string}")]
fn sort_indicator_on(world: &mut OwnrsWorld, column: String) {
    let header_line = world
        .stdout
        .lines()
        .find(|l| l.contains(&column))
        .expect("no header line found");
    let marker = format!("{column}↑");
    assert!(
        header_line.contains(&marker),
        "expected sort indicator on {column}, header was: {header_line}"
    );
}

#[then("stdout should be:")]
fn stdout_equals(world: &mut OwnrsWorld, step: &cucumber::Step) {
    let expected = step.docstring.as_ref().expect("step must have docstring").trim();
    let actual = world.stdout.trim();
    assert_eq!(actual, expected, "stdout mismatch\ngot:\n{actual}");
}

#[then(expr = "no output line should exceed the terminal width")]
fn no_line_exceeds_width(world: &mut OwnrsWorld) {
    let max_width = 120; // reasonable default
    for line in world.stdout.lines() {
        assert!(
            line.len() <= max_width,
            "line exceeds {max_width} chars ({} chars): {line}",
            line.len()
        );
    }
}

fn main() {
    futures::executor::block_on(OwnrsWorld::run("tests/features"));
}
```

- [ ] **Step 3: Create the first feature file**

Create `tests/features/default_table.feature`:

```gherkin
Feature: Default table output

  Background:
    Given the following repos:
      | repo_name    | status  | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | beta-service | stale   | usermgmt      | usermgmt         |             | 2026-04-10 |
      | alpha-repo   | aligned | my-team       | my-team          | my-team     | 2026-04-14 |
      | gamma-tool   | missing | -             | -                |             | 2026-03-01 |

  Scenario: Default output shows detail table sorted by repo name
    When I render the table
    Then stdout should contain "REPO"
    And stdout should contain "STATUS"
    And stdout should contain "CATALOG OWNER"
    And stdout should contain "CODEOWNERS TEAMS"
    And stdout should contain "LAST PUSH"
    And the first data row should start with "alpha-repo"
    And the second data row should start with "beta-service"
    And the third data row should start with "gamma-tool"

  Scenario: Default output includes tally footer with percentages
    When I render the table
    Then stdout should contain "1 aligned (33%)"
    And stdout should contain "1 stale (33%)"
    And stdout should contain "1 missing (33%)"

  Scenario: Default output includes title line with count
    Given the team filter is "my-team"
    When I render the table
    Then stdout should contain "repos(my-team)["

  Scenario: Default output does not show Admin Teams or Notes columns
    When I render the table
    Then stdout should not contain "ADMIN TEAMS"
    And stdout should not contain "NOTES"
```

- [ ] **Step 4: Run the BDD tests to verify they fail**

Run: `cargo test --test bdd`
Expected: Compilation failure because `render_table` and `TableOptions` don't exist yet. This confirms the test infrastructure is wired up correctly and we need to implement the rendering.

- [ ] **Step 5: Commit**

```bash
git add tests/bdd.rs tests/features/default_table.feature Cargo.toml
git commit -m "test: add BDD infrastructure and default table feature"
```

---

### Task 2: Implement the new table renderer

**Files:**
- Modify: `src/output/table.rs`
- Modify: `src/output/mod.rs`
- Modify: `Cargo.toml` (add unicode-width)

This task creates the `render_table` function and `TableOptions` struct that the BDD tests call. It replaces the comfy-table rendering with padded-column output.

- [ ] **Step 1: Add unicode-width dependency**

Add to `[dependencies]` in `Cargo.toml`:

```toml
unicode-width = "0.2"
```

- [ ] **Step 2: Make the output module public**

Change `src/main.rs` line 5 from:

```rust
mod output;
```

to:

```rust
pub mod output;
```

Also make `reconcile` public — change line 6 from:

```rust
mod reconcile;
```

to:

```rust
pub mod reconcile;
```

And make the other modules that `reconcile::types` depends on public. Change line 2 from:

```rust
mod cli;
```

to:

```rust
pub mod cli;
```

And change line 7 from:

```rust
mod suggest;
```

to:

```rust
pub mod suggest;
```

- [ ] **Step 3: Rewrite `src/output/table.rs`**

Replace the entire contents of `src/output/table.rs` with:

```rust
use unicode_width::UnicodeWidthStr;

use crate::reconcile::types::{AlignmentStatus, AuditSummary, RepoOwnership};

/// Options controlling table output.
pub struct TableOptions {
    pub wide: bool,
    pub sort_columns: Vec<String>,
    pub team_filter: Option<String>,
}

/// Column definition for the padded-column renderer.
struct Column {
    header: &'static str,
    key: &'static str,
}

const DEFAULT_COLUMNS: &[Column] = &[
    Column { header: "REPO", key: "repo" },
    Column { header: "STATUS", key: "status" },
    Column { header: "CATALOG OWNER", key: "catalog-owner" },
    Column { header: "CODEOWNERS TEAMS", key: "codeowners-teams" },
    Column { header: "LAST PUSH", key: "last-push" },
];

const WIDE_COLUMNS: &[Column] = &[
    Column { header: "REPO", key: "repo" },
    Column { header: "STATUS", key: "status" },
    Column { header: "CATALOG OWNER", key: "catalog-owner" },
    Column { header: "CODEOWNERS TEAMS", key: "codeowners-teams" },
    Column { header: "ADMIN TEAMS", key: "admin-teams" },
    Column { header: "LAST PUSH", key: "last-push" },
    Column { header: "NOTES", key: "notes" },
];

const MAX_COL_WIDTH: usize = 30;

/// Render repos as a padded-column table string. Public for BDD tests.
pub fn render_table(repos: &[RepoOwnership], opts: &TableOptions) -> String {
    let mut repos = repos.to_vec();
    sort_repos(&mut repos, &opts.sort_columns);

    let columns = if opts.wide { WIDE_COLUMNS } else { DEFAULT_COLUMNS };

    // Build rows as Vec<Vec<String>>
    let rows: Vec<Vec<String>> = repos.iter().map(|r| row_values(r, columns)).collect();

    // Calculate column widths (max of header and data, capped at MAX_COL_WIDTH)
    let col_widths: Vec<usize> = columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let data_max = rows.iter().map(|r| UnicodeWidthStr::width(r[i].as_str())).max().unwrap_or(0);
            let header_width = UnicodeWidthStr::width(col.header) + 1; // +1 for possible sort arrow
            data_max.max(header_width).min(MAX_COL_WIDTH)
        })
        .collect();

    let mut out = String::new();

    // Title line
    let filter_label = opts.team_filter.as_deref().unwrap_or("all");
    out.push_str(&format!("repos({})[{}]\n\n", filter_label, repos.len()));

    // Header
    let primary_sort = opts.sort_columns.first().map(|s| s.as_str()).unwrap_or("repo");
    for (i, col) in columns.iter().enumerate() {
        let header = if col.key == primary_sort {
            format!("{}↑", col.header)
        } else {
            col.header.to_string()
        };
        if i < columns.len() - 1 {
            out.push_str(&pad_right(&header, col_widths[i] + 1));
            out.push(' ');
        } else {
            out.push_str(&header);
        }
    }
    out.push('\n');

    // Data rows
    for row in &rows {
        for (i, val) in row.iter().enumerate() {
            let truncated = truncate(val, col_widths[i]);
            if i < columns.len() - 1 {
                out.push_str(&pad_right(&truncated, col_widths[i] + 1));
                out.push(' ');
            } else {
                out.push_str(&truncated);
            }
        }
        out.push('\n');
    }

    // Tally footer
    out.push('\n');
    out.push_str(&tally_footer(&repos));
    out.push('\n');

    out
}

/// Render the legacy summary table (for --summary flag).
pub fn render_summary(summary: &AuditSummary) -> String {
    use comfy_table::{ContentArrangement, Table};

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Status", "Count", "%"]);

    let total = summary.total as f64;
    let pct = |n: usize| {
        if total == 0.0 {
            "0.0".to_string()
        } else {
            format!("{:.1}", n as f64 / total * 100.0)
        }
    };

    table.add_row(vec!["Aligned", &summary.aligned.to_string(), &pct(summary.aligned)]);
    table.add_row(vec!["Mismatched", &summary.mismatched.to_string(), &pct(summary.mismatched)]);
    table.add_row(vec!["Catalog Only", &summary.catalog_only.to_string(), &pct(summary.catalog_only)]);
    table.add_row(vec!["Codeowners Only", &summary.codeowners_only.to_string(), &pct(summary.codeowners_only)]);
    table.add_row(vec!["Admin Only", &summary.admin_only.to_string(), &pct(summary.admin_only)]);
    table.add_row(vec!["Stale", &summary.stale.to_string(), &pct(summary.stale)]);
    table.add_row(vec!["Missing", &summary.missing.to_string(), &pct(summary.missing)]);
    table.add_row(vec!["Total", &summary.total.to_string(), ""]);

    format!("{table}")
}

/// Render repo names one per line (for --format names).
pub fn render_names(repos: &[RepoOwnership]) -> String {
    let mut names: Vec<&str> = repos.iter().map(|r| r.repo_name.as_str()).collect();
    names.sort();
    let mut out = String::new();
    for name in names {
        out.push_str(name);
        out.push('\n');
    }
    out
}

/// Print the single-repo detail view (unchanged from before).
pub fn print_single_repo(repo: &RepoOwnership) {
    println!("Repository: {}", repo.repo_name);
    println!("Status:     {}", repo.alignment);
    println!(
        "Catalog:    {}{}",
        repo.catalog_owner.as_deref().unwrap_or("(none)"),
        match repo.catalog_team_exists {
            Some(true) => " (team exists)",
            Some(false) => " (team NOT found)",
            None => "",
        }
    );

    if repo.codeowners_teams.is_empty() {
        println!("CODEOWNERS: (none)");
    } else {
        for (team, exists) in &repo.codeowners_teams_exist {
            let status = if *exists { "(team exists)" } else { "(team NOT found)" };
            println!("CODEOWNERS: {} {}", team, status);
        }
    }

    if repo.admin_teams.is_empty() {
        println!("Admin:      (none)");
    } else {
        println!("Admin:      {}", repo.admin_teams.join(", "));
    }

    if let Some(pushed) = repo.pushed_at {
        println!("Last Push:  {}", pushed.format("%Y-%m-%d %H:%M UTC"));
    }
    if !repo.notes.is_empty() {
        println!("Notes:      {}", repo.notes.join("; "));
    }

    if let Some(ref suggestion) = repo.suggested_owners {
        if suggestion.suggestions.is_empty() {
            println!(
                "\nSuggested:  No activity found in last {} days",
                suggestion.lookback_days
            );
        } else {
            println!(
                "\nSuggested owners (based on last {} days of activity):",
                suggestion.lookback_days
            );
            for s in &suggestion.suggestions {
                let commits_label = if s.commits == 1 { "commit" } else { "commits" };
                let reviews_label = if s.reviews == 1 { "review" } else { "reviews" };
                let members_str = s.members.join(", ");
                println!(
                    "  {:<16} {} {}, {} {} ({})",
                    s.team, s.commits, commits_label, s.reviews, reviews_label, members_str
                );
            }
        }
        if !suggestion.unresolved.is_empty() {
            println!("\nUnresolved:        {}", suggestion.unresolved.join(", "));
        }
    }
}

// --- Internal helpers ---

fn row_values(repo: &RepoOwnership, columns: &[Column]) -> Vec<String> {
    columns
        .iter()
        .map(|col| match col.key {
            "repo" => repo.repo_name.clone(),
            "status" => repo.alignment.to_string(),
            "catalog-owner" => repo.catalog_owner.as_deref().unwrap_or("-").to_string(),
            "codeowners-teams" => {
                if repo.codeowners_teams.is_empty() {
                    "-".to_string()
                } else {
                    repo.codeowners_teams.join(", ")
                }
            }
            "admin-teams" => {
                if repo.admin_teams.is_empty() {
                    "-".to_string()
                } else {
                    repo.admin_teams.join(", ")
                }
            }
            "last-push" => repo
                .pushed_at
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "-".to_string()),
            "notes" => {
                if repo.notes.is_empty() {
                    String::new()
                } else {
                    repo.notes.join("; ")
                }
            }
            _ => String::new(),
        })
        .collect()
}

fn sort_repos(repos: &mut [RepoOwnership], sort_columns: &[String]) {
    repos.sort_by(|a, b| {
        for col in sort_columns {
            let ord = match col.as_str() {
                "repo" => a.repo_name.cmp(&b.repo_name),
                "status" => a.alignment.to_string().cmp(&b.alignment.to_string()),
                "catalog-owner" => {
                    let av = a.catalog_owner.as_deref().unwrap_or("");
                    let bv = b.catalog_owner.as_deref().unwrap_or("");
                    av.cmp(bv)
                }
                "codeowners-teams" => {
                    let av = a.codeowners_teams.join(", ");
                    let bv = b.codeowners_teams.join(", ");
                    av.cmp(&bv)
                }
                "admin-teams" => {
                    let av = a.admin_teams.join(", ");
                    let bv = b.admin_teams.join(", ");
                    av.cmp(&bv)
                }
                "last-push" => a.pushed_at.cmp(&b.pushed_at),
                "notes" => a.notes.join("; ").cmp(&b.notes.join("; ")),
                _ => std::cmp::Ordering::Equal,
            };
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    });
}

fn truncate(s: &str, max: usize) -> String {
    if UnicodeWidthStr::width(s) <= max {
        return s.to_string();
    }
    let mut result = String::new();
    let mut width = 0;
    for ch in s.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width + 1 > max {
            result.push('…');
            return result;
        }
        result.push(ch);
        width += ch_width;
    }
    result
}

fn pad_right(s: &str, width: usize) -> String {
    let s_width = UnicodeWidthStr::width(s);
    if s_width >= width {
        return s.to_string();
    }
    format!("{}{}", s, " ".repeat(width - s_width))
}

fn tally_footer(repos: &[RepoOwnership]) -> String {
    let total = repos.len();
    if total == 0 {
        return String::new();
    }

    let counts = [
        ("aligned", AlignmentStatus::Aligned),
        ("mismatched", AlignmentStatus::Mismatched),
        ("catalog-only", AlignmentStatus::CatalogOnly),
        ("codeowners-only", AlignmentStatus::CodeownersOnly),
        ("admin-only", AlignmentStatus::AdminOnly),
        ("stale", AlignmentStatus::Stale),
        ("missing", AlignmentStatus::Missing),
    ];

    let parts: Vec<String> = counts
        .iter()
        .filter_map(|(label, status)| {
            let count = repos.iter().filter(|r| r.alignment == *status).count();
            if count == 0 {
                None
            } else {
                let pct = (count as f64 / total as f64 * 100.0).round() as usize;
                Some(format!("{count} {label} ({pct}%)"))
            }
        })
        .collect();

    parts.join(" · ")
}
```

- [ ] **Step 4: Run the BDD tests**

Run: `cargo test --test bdd`
Expected: All scenarios in `default_table.feature` pass.

- [ ] **Step 5: Commit**

```bash
git add src/output/table.rs src/output/mod.rs src/main.rs Cargo.toml
git commit -m "feat: implement padded-column table renderer with tally footer"
```

---

### Task 3: Add remaining BDD feature files

**Files:**
- Create: `tests/features/wide.feature`
- Create: `tests/features/summary.feature`
- Create: `tests/features/sorting.feature`
- Create: `tests/features/format_names.feature`
- Create: `tests/features/truncation.feature`

- [ ] **Step 1: Create `tests/features/wide.feature`**

```gherkin
Feature: Wide output

  Background:
    Given the following repos:
      | repo_name  | status  | catalog_owner | codeowners_teams | admin_teams       | pushed_at  | notes                 |
      | alpha-repo | aligned | my-team       | my-team          | my-team, sec-eng  | 2026-04-14 |                       |
      | beta-svc   | stale   | usermgmt      | usermgmt         | my-team           | 2026-04-10 | references stale team |

  Scenario: Wide flag adds Admin Teams and Notes columns
    When I render the table with "--wide"
    Then stdout should contain "ADMIN TEAMS"
    And stdout should contain "NOTES"
    And stdout should contain "my-team, sec-eng"
    And stdout should contain "references stale team"
```

- [ ] **Step 2: Create `tests/features/summary.feature`**

Add a `--summary` step to `tests/bdd.rs`. In the `render_with_flags` function, after the `--summary` match arm, add handling that calls `render_summary`. For now, add this step to `tests/bdd.rs`:

```rust
#[when("I render the summary")]
fn render_summary_step(world: &mut OwnrsWorld) {
    use ownrs::reconcile::types::AuditSummary;
    let summary = AuditSummary::from_repos(world.repos.clone());
    world.stdout = ownrs::output::table::render_summary(&summary);
}
```

Then create `tests/features/summary.feature`:

```gherkin
Feature: Summary flag

  Background:
    Given the following repos:
      | repo_name  | status  | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | alpha-repo | aligned | my-team       | my-team          | my-team     | 2026-04-14 |
      | beta-svc   | stale   | usermgmt      | usermgmt         |             | 2026-04-10 |

  Scenario: Summary flag shows status count table
    When I render the summary
    Then stdout should contain "Status"
    And stdout should contain "Count"
    And stdout should contain "Aligned"
    And stdout should contain "1"
```

- [ ] **Step 3: Create `tests/features/sorting.feature`**

```gherkin
Feature: Sorting

  Background:
    Given the following repos:
      | repo_name    | status  | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | charlie-svc  | aligned | team-a        | team-a           |             | 2026-04-10 |
      | alpha-repo   | stale   | usermgmt      | usermgmt         |             | 2026-04-14 |
      | beta-tool    | missing | -             | -                |             | 2026-03-01 |

  Scenario: Sort by status
    When I render the table with "--sort status"
    Then the first data row should start with "charlie-svc"
    And the sort indicator should be on "STATUS"

  Scenario: Sort by last-push
    When I render the table with "--sort last-push"
    Then the first data row should start with "beta-tool"

  Scenario: Multi-column sort
    When I render the table with "--sort status,repo"
    Then the sort indicator should be on "STATUS"

  Scenario: Sort indicator arrow on sorted column
    When I render the table with "--sort repo"
    Then stdout should contain "REPO↑"
    And stdout should not contain "STATUS↑"
```

- [ ] **Step 4: Create `tests/features/format_names.feature`**

```gherkin
Feature: Format names

  Background:
    Given the following repos:
      | repo_name    | status  | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | beta-service | stale   | usermgmt      | usermgmt         |             | 2026-04-10 |
      | alpha-repo   | aligned | my-team       | my-team          |             | 2026-04-14 |

  Scenario: Names format outputs one repo per line alphabetically
    When I render with "--format names"
    Then stdout should be:
      """
      alpha-repo
      beta-service
      """

  Scenario: Names format has no headers
    When I render with "--format names"
    Then stdout should not contain "REPO"
    And stdout should not contain "STATUS"
```

- [ ] **Step 5: Create `tests/features/truncation.feature`**

```gherkin
Feature: Long value truncation

  Background:
    Given the following repos:
      | repo_name  | status  | catalog_owner | codeowners_teams                                | admin_teams | pushed_at  |
      | alpha-repo | aligned | my-team       | team-a, team-b, team-c, team-d, team-e, team-f |             | 2026-04-14 |

  Scenario: Long values are truncated with ellipsis
    When I render the table
    Then stdout should contain "…"
```

- [ ] **Step 6: Run all BDD tests**

Run: `cargo test --test bdd`
Expected: All scenarios across all feature files pass.

- [ ] **Step 7: Commit**

```bash
git add tests/features/ tests/bdd.rs
git commit -m "test: add BDD features for wide, summary, sorting, names, truncation"
```

---

### Task 4: Update CLI args and config

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/config.rs`

- [ ] **Step 1: Update `src/cli.rs`**

Replace the `SortOrder` enum and update the `Org` command and `OutputFormat`:

```rust
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "ownrs",
    version,
    about = "Three-way ownership reconciliation CLI"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Force re-fetch cached data
    #[arg(long, global = true)]
    pub refresh: bool,

    /// Cache directory
    #[arg(long, global = true, default_value = None)]
    pub cache_dir: Option<String>,

    /// Cache TTL in seconds
    #[arg(long, global = true, default_value_t = 86400)]
    pub cache_ttl: u64,

    /// Lookback window in days for ownership suggestions
    #[arg(
        long,
        global = true,
        default_value_t = 90,
        help_heading = "Suggestion Options"
    )]
    pub lookback_days: u64,

    /// Max team size to consider for suggestions (filters out org-wide teams)
    #[arg(
        long,
        global = true,
        default_value_t = 20,
        help_heading = "Suggestion Options"
    )]
    pub max_team_size: usize,

    /// Teams to exclude from suggestions (comma-separated)
    #[arg(
        long,
        global = true,
        value_delimiter = ',',
        help_heading = "Suggestion Options"
    )]
    pub exclude_team: Vec<String>,

    /// GitHub token (defaults to GITHUB_TOKEN env var)
    #[arg(long, global = true, env = "GITHUB_TOKEN", hide_env_values = true)]
    pub token: Option<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Audit repos across a GitHub org
    Org {
        /// GitHub organization name
        org: String,

        /// Audit only the first N repos
        #[arg(long)]
        limit: Option<usize>,

        /// Sort by columns (comma-separated: repo, status, catalog-owner, codeowners-teams, last-push, admin-teams, notes)
        #[arg(long, default_value = "repo", value_delimiter = ',')]
        sort: Vec<String>,

        /// Filter to repos referencing this team (comma-separated)
        #[arg(long, value_delimiter = ',')]
        team: Vec<String>,

        /// Filter by alignment status (comma-separated)
        #[arg(long, value_delimiter = ',')]
        status: Vec<StatusFilter>,

        /// Output format: table (default), csv, json, names
        #[arg(long, default_value = "table")]
        format: OutputFormat,

        /// Show summary statistics table
        #[arg(long)]
        summary: bool,

        /// Show all columns (Admin Teams, Notes)
        #[arg(long)]
        wide: bool,

        /// Require exact team set match across all sources (default: intersection)
        #[arg(long)]
        strict: bool,
    },

    /// Audit a single repo
    Repo {
        /// org/repo (auto-detected from git remote if omitted)
        repo: Option<String>,

        /// Filter by alignment status (comma-separated)
        #[arg(long, value_delimiter = ',')]
        status: Vec<StatusFilter>,

        /// Output format: table (default), json
        #[arg(long, default_value = "table")]
        format: OutputFormat,

        /// Require exact team set match across all sources (default: intersection)
        #[arg(long)]
        strict: bool,

        /// Run ownership suggestion heuristic (comma-separated: missing, stale, mismatched, partial)
        #[arg(long, value_delimiter = ',', help_heading = "Suggestion Options")]
        suggest: Vec<SuggestMode>,
    },
}

#[derive(Clone, ValueEnum, PartialEq, Eq)]
pub enum StatusFilter {
    Aligned,
    Mismatched,
    Stale,
    Missing,
    CatalogOnly,
    CodeownersOnly,
    AdminOnly,
}

#[derive(Clone, ValueEnum)]
pub enum SuggestMode {
    Missing,
    Stale,
    Mismatched,
    Partial,
}

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Table,
    Csv,
    Json,
    Names,
}
```

- [ ] **Step 2: Update `src/config.rs`**

Replace `SortOrder` with `Vec<String>` sort columns, replace `detail: bool` with `summary: bool` and `wide: bool`:

```rust
use std::path::PathBuf;

use crate::cli::{Cli, Command, OutputFormat, StatusFilter, SuggestMode};

pub struct Config {
    pub scope: Scope,
    pub token: String,
    pub refresh: bool,
    pub cache_dir: PathBuf,
    pub cache_ttl: u64,
    pub lookback_days: u64,
    pub max_team_size: usize,
    pub exclude_team: Vec<String>,
}

pub enum Scope {
    Org {
        org: String,
        limit: Option<usize>,
        sort: Vec<String>,
        team_filter: Vec<String>,
        status_filter: Vec<StatusFilter>,
        format: OutputFormat,
        summary: bool,
        wide: bool,
        strict: bool,
    },
    Repo {
        org: String,
        repo: String,
        status_filter: Vec<StatusFilter>,
        format: OutputFormat,
        strict: bool,
        suggest: Vec<SuggestMode>,
    },
}

impl Config {
    pub fn from_cli(cli: Cli) -> anyhow::Result<Self> {
        let token = match cli.token {
            Some(t) => t,
            None => token_from_gh_cli()?,
        };

        let cache_dir = match cli.cache_dir {
            Some(dir) => PathBuf::from(dir),
            None => default_cache_dir()?,
        };

        let scope = match cli.command {
            Command::Org {
                org,
                limit,
                sort,
                team,
                status,
                format,
                summary,
                wide,
                strict,
            } => Scope::Org {
                org,
                limit,
                sort,
                team_filter: team,
                status_filter: status,
                format,
                summary,
                wide,
                strict,
            },
            Command::Repo {
                repo,
                status,
                format,
                strict,
                suggest,
            } => {
                let (org, repo_name) = parse_repo_arg(repo)?;
                Scope::Repo {
                    org,
                    repo: repo_name,
                    status_filter: status,
                    format,
                    strict,
                    suggest,
                }
            }
        };

        Ok(Config {
            scope,
            token,
            refresh: cli.refresh,
            cache_dir,
            cache_ttl: cli.cache_ttl,
            lookback_days: cli.lookback_days,
            max_team_size: cli.max_team_size,
            exclude_team: cli.exclude_team,
        })
    }
}

fn token_from_gh_cli() -> anyhow::Result<String> {
    let output = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let token = String::from_utf8(out.stdout)?.trim().to_string();
            if token.is_empty() {
                anyhow::bail!("GitHub token required. Run `gh auth login` or set GITHUB_TOKEN");
            }
            Ok(token)
        }
        _ => anyhow::bail!("GitHub token required. Run `gh auth login` or set GITHUB_TOKEN"),
    }
}

fn default_cache_dir() -> anyhow::Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("", "", "ownrs")
        .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?;
    Ok(proj_dirs.cache_dir().to_path_buf())
}

fn parse_repo_arg(repo: Option<String>) -> anyhow::Result<(String, String)> {
    match repo {
        Some(slug) => {
            let parts: Vec<&str> = slug.splitn(2, '/').collect();
            if parts.len() != 2 {
                anyhow::bail!("Repo must be in org/repo format, got: {slug}");
            }
            Ok((parts[0].to_string(), parts[1].to_string()))
        }
        None => detect_from_git_remote(),
    }
}

fn detect_from_git_remote() -> anyhow::Result<(String, String)> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("No repo specified and could not detect from git remote");
    }

    let url = String::from_utf8(output.stdout)?.trim().to_string();

    // Handle SSH: git@github.com:org/repo.git
    if let Some(path) = url.strip_prefix("git@github.com:") {
        let path = path.strip_suffix(".git").unwrap_or(path);
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    // Handle HTTPS: https://github.com/org/repo.git
    if let Some(path) = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
    {
        let path = path.strip_suffix(".git").unwrap_or(path);
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    anyhow::bail!("Could not parse org/repo from git remote: {url}")
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compilation error in `main.rs` because it still references the old `SortOrder`, `detail`, etc. That's expected — we fix `main.rs` in the next task.

- [ ] **Step 4: Commit**

```bash
git add src/cli.rs src/config.rs
git commit -m "feat: update CLI args for new table UX (sort columns, summary, wide, names)"
```

---

### Task 5: Wire up main.rs to new rendering

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update `run_org` in `src/main.rs`**

Replace the `run_org` function signature and body. Key changes:
- `sort: &SortOrder` becomes `sort: &[String]`
- `detail: bool` becomes `summary: bool` and `wide: bool`
- Remove the old sort match block (sorting is now handled by the renderer)
- Replace the output match to use `render_table`, `render_summary`, `render_names`

Replace `run_org` (lines 93–233) with:

```rust
async fn run_org(
    client: &GitHubClient,
    cache: &FileCache,
    config: &Config,
    org: &str,
    limit: Option<usize>,
    sort: &[String],
    team_filter: &[String],
    status_filter: &[cli::StatusFilter],
    format: &OutputFormat,
    summary: bool,
    wide: bool,
    strict: bool,
) -> anyhow::Result<()> {
    // Fetch teams
    let sp = ProgressBar::new_spinner();
    sp.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap(),
    );
    sp.set_message("Fetching teams...");
    sp.enable_steady_tick(std::time::Duration::from_millis(100));
    let valid_teams = fetch_team_slugs(client, org, cache, config.refresh).await?;
    sp.finish_with_message(format!("Fetched {} teams", valid_teams.len()));

    // Fetch repos
    let sp = ProgressBar::new_spinner();
    sp.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap(),
    );
    sp.set_message("Fetching repos...");
    sp.enable_steady_tick(std::time::Duration::from_millis(100));
    let mut repos = list_repos(client, org, cache, config.refresh, |count| {
        sp.set_message(format!("Fetching repos... {count} so far"));
    })
    .await?;
    sp.finish_with_message(format!("Fetched {} repos", repos.len()));

    // Limit (applied before source fetching for efficiency)
    if let Some(n) = limit {
        repos.truncate(n);
    }

    let repo_names: Vec<String> = repos.iter().map(|r| r.name.clone()).collect();

    // Fetch source files
    let pb = ProgressBar::new(repo_names.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40}] {pos}/{len}")
            .unwrap()
            .progress_chars("=> "),
    );
    pb.set_message("Fetching source files");

    let all_sources = fetch_all(client, org, &repo_names, cache, config.refresh).await;
    pb.finish_and_clear();

    // Reconcile
    let mut ownership_results = Vec::new();
    for source in &all_sources {
        let repo_info = repos.iter().find(|r| r.name == source.repo_name);
        let pushed_at = repo_info.and_then(|r| r.pushed_at);

        let catalog_owner = source
            .catalog_info
            .as_deref()
            .and_then(sources::catalog::extract_owner);
        let codeowners_teams = source
            .codeowners
            .as_deref()
            .map(sources::codeowners::extract_teams)
            .unwrap_or_default();

        let result = reconcile(
            &source.repo_name,
            pushed_at,
            catalog_owner.as_deref(),
            &codeowners_teams,
            &source.admin_teams,
            &valid_teams,
            strict,
        );

        ownership_results.push(result);
    }

    // Apply team filter
    if !team_filter.is_empty() {
        ownership_results.retain(|r| {
            let cat_match = r
                .catalog_owner
                .as_ref()
                .is_some_and(|o| team_filter.iter().any(|t| o.eq_ignore_ascii_case(t)));
            let co_match = r
                .codeowners_teams
                .iter()
                .any(|o| team_filter.iter().any(|t| o.eq_ignore_ascii_case(t)));
            let admin_match = r
                .admin_teams
                .iter()
                .any(|o| team_filter.iter().any(|t| o.eq_ignore_ascii_case(t)));
            cat_match || co_match || admin_match
        });
    }

    // Apply status filter
    if !status_filter.is_empty() {
        ownership_results.retain(|r| r.alignment.matches_filter(status_filter));
    }

    let audit = AuditSummary::from_repos(ownership_results);

    match format {
        OutputFormat::Json => output::json::print_json(&audit),
        OutputFormat::Csv => output::csv::print_csv(&audit.repos),
        OutputFormat::Names => {
            print!("{}", output::table::render_names(&audit.repos));
        }
        OutputFormat::Table => {
            if summary {
                println!("{}", output::table::render_summary(&audit));
            }
            let team_label = if team_filter.is_empty() {
                None
            } else {
                Some(team_filter.join(","))
            };
            let opts = output::table::TableOptions {
                wide,
                sort_columns: sort.to_vec(),
                team_filter: team_label,
            };
            print!("{}", output::table::render_table(&audit.repos, &opts));
        }
    }

    // Exit code
    if !status_filter.is_empty() && !audit.repos.is_empty() {
        process::exit(1);
    }

    Ok(())
}
```

- [ ] **Step 2: Update the `run` function's `Scope::Org` match arm**

Replace the `Scope::Org` match arm (lines 43–67) with:

```rust
        Scope::Org {
            ref org,
            limit,
            ref sort,
            ref team_filter,
            ref status_filter,
            ref format,
            summary,
            wide,
            strict,
        } => {
            run_org(
                &client,
                &cache,
                &config,
                org,
                limit,
                sort,
                team_filter,
                status_filter,
                format,
                summary,
                wide,
                strict,
            )
            .await
        }
```

- [ ] **Step 3: Remove unused imports**

In `src/main.rs`, remove the `SortOrder` import. The `use cli::{OutputFormat, SortOrder, SuggestMode};` line should become:

```rust
use cli::{OutputFormat, SuggestMode};
```

- [ ] **Step 4: Verify compilation and run all tests**

Run: `cargo check && cargo test`
Expected: Compiles cleanly. Unit tests and BDD tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire main.rs to new table renderer and CLI flags"
```

---

### Task 6: Clean up and verify

**Files:**
- Modify: `Cargo.toml` (optional: remove comfy-table if no longer used elsewhere)

- [ ] **Step 1: Check if comfy-table is still needed**

`render_summary` still uses comfy-table for the `--summary` flag output. Keep it in dependencies.

- [ ] **Step 2: Run the full test suite**

Run: `cargo test`
Expected: All unit tests and BDD tests pass.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

- [ ] **Step 4: Build a release binary and smoke test**

Run: `cargo build --release`

Then manually test (requires GitHub token):
```bash
# Default: detail table, alphabetical, tally footer
./target/release/ownrs org pantheon-systems --team workspace-management

# With summary
./target/release/ownrs org pantheon-systems --team workspace-management --summary

# Wide
./target/release/ownrs org pantheon-systems --team workspace-management --wide

# Sort by status
./target/release/ownrs org pantheon-systems --team workspace-management --sort status

# Multi-column sort
./target/release/ownrs org pantheon-systems --team workspace-management --sort status,repo

# Names export
./target/release/ownrs org pantheon-systems --team workspace-management --format names
```

- [ ] **Step 5: Commit any final fixes**

```bash
git add -A
git commit -m "chore: clean up and verify org table UX overhaul"
```
