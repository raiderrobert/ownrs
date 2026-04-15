use std::str::FromStr;

use chrono::{NaiveDate, TimeZone, Utc};
use cucumber::{given, then, when, World};

use ownrs::output::table::{render_names, render_summary, render_table, TableOptions};
use ownrs::reconcile::types::{AlignmentStatus, AuditSummary, RepoOwnership};

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
    let headers: Vec<&str> = table.rows[0].iter().map(|s| s.as_str()).collect();
    let notes_idx = headers.iter().position(|h| *h == "notes");

    for row in table.rows.iter().skip(1) {
        // columns: repo_name | status | catalog_owner | codeowners_teams | admin_teams | pushed_at [| notes]
        let repo_name = row[0].clone();
        let alignment = parse_status(&row[1]);
        let catalog_owner = parse_optional(&row[2]);
        let codeowners_teams = parse_list(&row[3]);
        let admin_teams = parse_list(&row[4]);
        let pushed_at = parse_date(&row[5]);

        let notes = if let Some(idx) = notes_idx {
            let val = row[idx].trim();
            if val.is_empty() || val == "-" {
                Vec::new()
            } else {
                vec![val.to_string()]
            }
        } else {
            Vec::new()
        };

        world.repos.push(RepoOwnership {
            repo_name,
            pushed_at,
            catalog_owner,
            codeowners_teams: codeowners_teams.clone(),
            catalog_team_exists: None,
            codeowners_teams_exist: codeowners_teams.iter().map(|t| (t.clone(), true)).collect(),
            admin_teams,
            alignment,
            notes,
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
    let opts = TableOptions {
        wide: false,
        sort_columns: vec![],
        team_filter: world.team_filter.clone(),
    };
    world.stdout = render_table(&world.repos, &opts);
}

#[when(expr = "I render the table with {string}")]
fn render_with_flags(world: &mut OwnrsWorld, flags: String) {
    let mut wide = false;
    let mut sort_columns: Vec<String> = vec![];

    let parts: Vec<&str> = flags.split_whitespace().collect();
    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            "--wide" => wide = true,
            "--sort" => {
                // next token is comma-separated sort columns
                i += 1;
                if i < parts.len() {
                    for col in parts[i].split(',') {
                        sort_columns.push(col.trim().to_string());
                    }
                }
            }
            f if f.starts_with("--sort=") => {
                for col in f.trim_start_matches("--sort=").split(',') {
                    sort_columns.push(col.trim().to_string());
                }
            }
            other => panic!("unknown flag: {other}"),
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

#[when("I render the summary")]
fn render_summary_step(world: &mut OwnrsWorld) {
    let summary = AuditSummary::from_repos(world.repos.clone());
    world.stdout = render_summary(&summary);
}

#[when(expr = "I render with format {string}")]
fn render_with_format(world: &mut OwnrsWorld, format: String) {
    match format.as_str() {
        "names" => {
            world.stdout = render_names(&world.repos);
        }
        _ => panic!("unknown format: {format}"),
    }
}

#[when(expr = "I render with {string}")]
fn render_with_flags_format(world: &mut OwnrsWorld, flags: String) {
    let parts: Vec<&str> = flags.split_whitespace().collect();
    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            "--format" => {
                i += 1;
                if i < parts.len() {
                    match parts[i] {
                        "names" => {
                            world.stdout = render_names(&world.repos);
                        }
                        other => panic!("unknown format: {other}"),
                    }
                }
            }
            other => panic!("unknown flag: {other}"),
        }
        i += 1;
    }
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
        // Skip separator lines
        if trimmed
            .chars()
            .all(|c| c == '-' || c == '=' || c == '+' || c == '|' || c == ' ')
        {
            continue;
        }
        // Skip tally/footer lines (contain count + status + percentage)
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
    let header_line = world
        .stdout
        .lines()
        .find(|l| l.contains(&column))
        .unwrap_or_else(|| panic!("Column '{column}' not found in stdout:\n{}", world.stdout));
    assert!(
        header_line.contains('\u{2191}'), // ↑
        "Expected sort indicator (↑) on column '{column}', but header line is: '{header_line}'",
    );
}

#[then("stdout should be:")]
fn stdout_should_be(world: &mut OwnrsWorld, step: &cucumber::gherkin::Step) {
    let expected = step.docstring.as_ref().expect("expected docstring").trim();
    let actual = world.stdout.trim();
    assert_eq!(
        actual, expected,
        "stdout mismatch\ngot:\n{actual}\nexpected:\n{expected}"
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
