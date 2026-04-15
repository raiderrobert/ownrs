use std::str::FromStr;

use chrono::{NaiveDate, TimeZone, Utc};
use cucumber::{given, then, when, World};

use ownrs::output::table::{render_table, TableOptions};
use ownrs::reconcile::types::{AlignmentStatus, RepoOwnership};

#[derive(Debug, Default, World)]
pub struct OwnrsWorld {
    repos: Vec<RepoOwnership>,
    team_filter: Option<String>,
    stdout: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_status(s: &str) -> AlignmentStatus {
    match s.to_lowercase().as_str() {
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
    if s.is_empty() || s == "-" {
        return None;
    }
    let nd = NaiveDate::from_str(s).unwrap_or_else(|e| panic!("bad date '{s}': {e}"));
    Some(Utc.from_utc_datetime(&nd.and_hms_opt(0, 0, 0).unwrap()))
}

fn parse_list(s: &str) -> Vec<String> {
    if s.is_empty() || s == "-" {
        return Vec::new();
    }
    s.split(',').map(|t| t.trim().to_string()).collect()
}

fn parse_optional(s: &str) -> Option<String> {
    if s.is_empty() || s == "-" {
        None
    } else {
        Some(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// Given steps
// ---------------------------------------------------------------------------

#[given("the following repos:")]
fn given_repos(world: &mut OwnrsWorld, step: &cucumber::gherkin::Step) {
    let table = step.table.as_ref().expect("expected a data table");
    for row in table.rows.iter().skip(1) {
        // columns: repo_name | status | catalog_owner | codeowners_teams | admin_teams | pushed_at
        let repo_name = row[0].clone();
        let alignment = parse_status(&row[1]);
        let catalog_owner = parse_optional(&row[2]);
        let codeowners_teams = parse_list(&row[3]);
        let admin_teams = parse_list(&row[4]);
        let pushed_at = parse_date(&row[5]);

        world.repos.push(RepoOwnership {
            repo_name,
            pushed_at,
            catalog_owner,
            codeowners_teams: codeowners_teams.clone(),
            catalog_team_exists: None,
            codeowners_teams_exist: codeowners_teams.iter().map(|t| (t.clone(), true)).collect(),
            admin_teams,
            alignment,
            notes: Vec::new(),
            suggested_owners: None,
        });
    }
}

#[given(expr = "the team filter is {string}")]
fn given_team_filter(world: &mut OwnrsWorld, team: String) {
    world.team_filter = Some(team);
}

// ---------------------------------------------------------------------------
// When steps
// ---------------------------------------------------------------------------

#[when("I render the table")]
fn render_default(world: &mut OwnrsWorld) {
    let opts = TableOptions::default();
    world.stdout = render_table(&world.repos, &opts, world.team_filter.as_deref());
}

#[when(expr = "I render the table with {string}")]
fn render_with_flags(world: &mut OwnrsWorld, flags: String) {
    let mut opts = TableOptions::default();
    for flag in flags.split_whitespace() {
        match flag {
            "--wide" => opts.wide = true,
            f if f.starts_with("--sort=") => {
                opts.sort = Some(f.trim_start_matches("--sort=").to_string());
            }
            other => panic!("unknown flag: {other}"),
        }
    }
    world.stdout = render_table(&world.repos, &opts, world.team_filter.as_deref());
}

#[when(expr = "I render the table with format {string}")]
fn render_format(world: &mut OwnrsWorld, format: String) {
    let mut opts = TableOptions::default();
    opts.format = Some(format);
    world.stdout = render_table(&world.repos, &opts, world.team_filter.as_deref());
}

// ---------------------------------------------------------------------------
// Then steps
// ---------------------------------------------------------------------------

#[then(expr = "stdout should contain {string}")]
fn stdout_contains(world: &mut OwnrsWorld, expected: String) {
    assert!(
        world.stdout.contains(&expected),
        "Expected stdout to contain '{expected}', but got:\n{}",
        world.stdout,
    );
}

#[then(expr = "stdout should not contain {string}")]
fn stdout_not_contains(world: &mut OwnrsWorld, unexpected: String) {
    assert!(
        !world.stdout.contains(&unexpected),
        "Expected stdout NOT to contain '{unexpected}', but got:\n{}",
        world.stdout,
    );
}

fn data_rows(stdout: &str) -> Vec<&str> {
    // Skip header lines (lines containing column headers) and blank/separator lines.
    // Data rows are lines that don't start with common table-chrome characters
    // and aren't the header row.
    let lines: Vec<&str> = stdout.lines().collect();
    let mut data = Vec::new();
    let mut past_header = false;
    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Once we see a line containing "REPO" we know it's the header
        if trimmed.contains("REPO") {
            past_header = true;
            continue;
        }
        if !past_header {
            continue;
        }
        // Skip separator lines (e.g. "---" or "===")
        if trimmed.chars().all(|c| c == '-' || c == '=' || c == '+' || c == '|' || c == ' ') {
            continue;
        }
        // Skip tally/footer lines
        if trimmed.contains("aligned") && trimmed.contains('%') {
            continue;
        }
        if trimmed.starts_with("repos(") || trimmed.starts_with("repos[") {
            continue;
        }
        data.push(trimmed);
    }
    data
}

#[then(expr = "the first data row should start with {string}")]
fn first_data_row(world: &mut OwnrsWorld, expected: String) {
    let rows = data_rows(&world.stdout);
    assert!(
        !rows.is_empty(),
        "No data rows found in stdout:\n{}",
        world.stdout
    );
    assert!(
        rows[0].starts_with(&expected),
        "Expected first data row to start with '{expected}', but got: '{}'",
        rows[0],
    );
}

#[then(expr = "the second data row should start with {string}")]
fn second_data_row(world: &mut OwnrsWorld, expected: String) {
    let rows = data_rows(&world.stdout);
    assert!(
        rows.len() >= 2,
        "Expected at least 2 data rows, got {}:\n{}",
        rows.len(),
        world.stdout,
    );
    assert!(
        rows[1].starts_with(&expected),
        "Expected second data row to start with '{expected}', but got: '{}'",
        rows[1],
    );
}

#[then(expr = "the third data row should start with {string}")]
fn third_data_row(world: &mut OwnrsWorld, expected: String) {
    let rows = data_rows(&world.stdout);
    assert!(
        rows.len() >= 3,
        "Expected at least 3 data rows, got {}:\n{}",
        rows.len(),
        world.stdout,
    );
    assert!(
        rows[2].starts_with(&expected),
        "Expected third data row to start with '{expected}', but got: '{}'",
        rows[2],
    );
}

#[then(expr = "the sort indicator should be on {string}")]
fn sort_indicator_on(world: &mut OwnrsWorld, column: String) {
    // Look for a sort arrow (e.g. "▲" or "▼") near the column name in the header
    let header_line = world
        .stdout
        .lines()
        .find(|l| l.contains(&column))
        .unwrap_or_else(|| panic!("Column '{column}' not found in stdout:\n{}", world.stdout));
    assert!(
        header_line.contains('▲') || header_line.contains('▼'),
        "Expected sort indicator on column '{column}', but header line is: '{header_line}'",
    );
}

#[then(expr = "stdout should equal:")]
fn stdout_equals(world: &mut OwnrsWorld, step: &cucumber::gherkin::Step) {
    let expected = step.docstring.as_ref().expect("expected a docstring");
    assert_eq!(
        world.stdout.trim(),
        expected.trim(),
        "stdout did not match expected:\n--- got ---\n{}\n--- expected ---\n{}",
        world.stdout.trim(),
        expected.trim(),
    );
}

#[then(expr = "no line should exceed {int} characters")]
fn no_line_exceeds_width(world: &mut OwnrsWorld, max_width: usize) {
    for (i, line) in world.stdout.lines().enumerate() {
        assert!(
            line.len() <= max_width,
            "Line {} exceeds {} characters (len={}): '{}'",
            i + 1,
            max_width,
            line.len(),
            line,
        );
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    futures::executor::block_on(OwnrsWorld::run("tests/features"));
}
