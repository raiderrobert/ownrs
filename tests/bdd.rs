use std::path::PathBuf;

use cucumber::{given, then, when, World};
use tempfile::TempDir;

#[derive(Debug, Default, World)]
pub struct OwnrsWorld {
    _temp_dir: Option<TempDir>,
    cache_dir: Option<PathBuf>,
    valid_teams: Option<Vec<String>>,
    repos: Vec<RepoRow>,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Clone)]
struct RepoRow {
    repo_name: String,
    catalog_owner: Option<String>,
    codeowners_teams: Vec<String>,
    admin_teams: Vec<String>,
    pushed_at: String,
}

const ORG: &str = "testorg";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ownrs_binary() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove binary name
    if path.ends_with("deps") {
        path.pop(); // remove deps/
    }
    path.push("ownrs");
    path
}

fn parse_list(s: &str) -> Vec<String> {
    let s = s.trim();
    if s.is_empty() || s == "-" {
        return Vec::new();
    }
    s.split(',').map(|t| t.trim().to_string()).collect()
}

fn parse_optional(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() || s == "-" {
        None
    } else {
        Some(s.to_string())
    }
}

fn write_cache_file(cache_dir: &PathBuf, key: &str, json: &str) {
    let safe_key = key.replace('/', "__");
    let path = cache_dir.join(format!("{safe_key}.json"));
    std::fs::write(&path, json).unwrap_or_else(|e| {
        panic!("Failed to write cache file {}: {}", path.display(), e);
    });
}

fn build_catalog_yaml(owner: &str) -> String {
    format!(
        "apiVersion: backstage.io/v1alpha1\nkind: Component\nspec:\n  owner: group:{}\n",
        owner
    )
}

fn build_codeowners_content(teams: &[String]) -> String {
    let owners: Vec<String> = teams.iter().map(|t| format!("@{}/{}", ORG, t)).collect();
    format!("* {}\n", owners.join(" "))
}

fn write_fixtures(world: &mut OwnrsWorld) {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let cache_dir = temp_dir.path().to_path_buf();

    // Determine valid teams
    let valid_teams: Vec<String> = if let Some(ref explicit) = world.valid_teams {
        explicit.clone()
    } else {
        // Auto-derive from all team names in repos
        let mut teams = std::collections::HashSet::new();
        for repo in &world.repos {
            if let Some(ref owner) = repo.catalog_owner {
                teams.insert(owner.clone());
            }
            for t in &repo.codeowners_teams {
                teams.insert(t.clone());
            }
            for t in &repo.admin_teams {
                teams.insert(t.clone());
            }
        }
        teams.into_iter().collect()
    };

    // Write teams cache
    let teams_json = serde_json::to_string(&valid_teams).unwrap();
    write_cache_file(&cache_dir, &format!("teams_{ORG}"), &teams_json);

    // Build repos list
    let repos_json: Vec<serde_json::Value> = world
        .repos
        .iter()
        .map(|r| {
            let pushed = if r.pushed_at.is_empty() || r.pushed_at == "-" {
                serde_json::Value::Null
            } else {
                // Accept YYYY-MM-DD and expand to full datetime
                let dt = if r.pushed_at.len() == 10 {
                    format!("{}T00:00:00Z", r.pushed_at)
                } else {
                    r.pushed_at.clone()
                };
                serde_json::Value::String(dt)
            };
            serde_json::json!({
                "name": r.repo_name,
                "pushed_at": pushed,
            })
        })
        .collect();
    write_cache_file(
        &cache_dir,
        &format!("repos_{ORG}"),
        &serde_json::to_string(&repos_json).unwrap(),
    );

    // Per-repo cache files
    for repo in &world.repos {
        let name = &repo.repo_name;

        // CODEOWNERS
        let co_key = format!("content_{ORG}_{name}_codeowners");
        let co_value: serde_json::Value = if repo.codeowners_teams.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(build_codeowners_content(&repo.codeowners_teams))
        };
        write_cache_file(
            &cache_dir,
            &co_key,
            &serde_json::to_string(&co_value).unwrap(),
        );

        // catalog-info.yaml
        let cat_key = format!("content_{ORG}_{name}_catalog");
        let cat_value: serde_json::Value = match &repo.catalog_owner {
            Some(owner) => serde_json::Value::String(build_catalog_yaml(owner)),
            None => serde_json::Value::Null,
        };
        write_cache_file(
            &cache_dir,
            &cat_key,
            &serde_json::to_string(&cat_value).unwrap(),
        );

        // admin teams
        let admin_key = format!("admin_teams_{ORG}_{name}");
        write_cache_file(
            &cache_dir,
            &admin_key,
            &serde_json::to_string(&repo.admin_teams).unwrap(),
        );
    }

    world._temp_dir = Some(temp_dir);
    world.cache_dir = Some(cache_dir);
}

// ---------------------------------------------------------------------------
// Given steps
// ---------------------------------------------------------------------------

#[given(expr = "the valid teams are {string}")]
fn given_valid_teams(world: &mut OwnrsWorld, teams_csv: String) {
    world.valid_teams = Some(parse_list(&teams_csv));
}

#[given("a test org with the following repos:")]
fn given_repos(world: &mut OwnrsWorld, step: &cucumber::gherkin::Step) {
    let table = step.table.as_ref().expect("expected a data table");
    let headers: Vec<&str> = table.rows[0].iter().map(|s| s.as_str()).collect();

    let col = |name: &str| -> Option<usize> { headers.iter().position(|h| *h == name) };

    let repo_name_idx = col("repo_name").expect("missing repo_name column");
    let catalog_owner_idx = col("catalog_owner").expect("missing catalog_owner column");
    let codeowners_teams_idx = col("codeowners_teams").expect("missing codeowners_teams column");
    let admin_teams_idx = col("admin_teams").expect("missing admin_teams column");
    let pushed_at_idx = col("pushed_at").expect("missing pushed_at column");

    for row in table.rows.iter().skip(1) {
        world.repos.push(RepoRow {
            repo_name: row[repo_name_idx].trim().to_string(),
            catalog_owner: parse_optional(&row[catalog_owner_idx]),
            codeowners_teams: parse_list(&row[codeowners_teams_idx]),
            admin_teams: parse_list(&row[admin_teams_idx]),
            pushed_at: row[pushed_at_idx].trim().to_string(),
        });
    }

    // Write fixtures immediately
    write_fixtures(world);
}

// ---------------------------------------------------------------------------
// When steps
// ---------------------------------------------------------------------------

#[when(expr = "I run ownrs {string}")]
fn run_ownrs(world: &mut OwnrsWorld, args_str: String) {
    let cache_dir = world
        .cache_dir
        .as_ref()
        .expect("cache_dir not set — did you forget the Given step?");

    let binary = ownrs_binary();
    let args: Vec<&str> = args_str.split_whitespace().collect();

    let output = std::process::Command::new(&binary)
        .args(&args)
        .arg("--cache-dir")
        .arg(cache_dir)
        .env("GITHUB_TOKEN", "fake-token-for-testing")
        .output()
        .unwrap_or_else(|e| {
            panic!("Failed to execute {}: {}", binary.display(), e);
        });

    world.exit_code = Some(output.status.code().unwrap_or(-1));
    world.stdout = String::from_utf8_lossy(&output.stdout).to_string();
    world.stderr = String::from_utf8_lossy(&output.stderr).to_string();
}

// ---------------------------------------------------------------------------
// Then steps
// ---------------------------------------------------------------------------

#[then("the command should succeed")]
fn command_should_succeed(world: &mut OwnrsWorld) {
    let code = world.exit_code.expect("no command was run");
    assert_eq!(
        code, 0,
        "Expected exit code 0, got {}.\nstdout:\n{}\nstderr:\n{}",
        code, world.stdout, world.stderr,
    );
}

#[then("the command should fail")]
fn command_should_fail(world: &mut OwnrsWorld) {
    let code = world.exit_code.expect("no command was run");
    assert_ne!(
        code, 0,
        "Expected non-zero exit code, got 0.\nstdout:\n{}\nstderr:\n{}",
        world.stdout, world.stderr,
    );
}

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

#[then(expr = "stderr should contain {string}")]
fn stderr_contains(world: &mut OwnrsWorld, expected: String) {
    assert!(
        world.stderr.contains(&expected),
        "Expected stderr to contain '{expected}', but got:\n{}",
        world.stderr,
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

#[then("stdout should be:")]
fn stdout_should_be(world: &mut OwnrsWorld, step: &cucumber::gherkin::Step) {
    let expected = step.docstring.as_ref().expect("expected docstring").trim();
    let actual = world.stdout.trim();
    assert_eq!(
        actual, expected,
        "stdout mismatch\ngot:\n{actual}\nexpected:\n{expected}"
    );
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    futures::executor::block_on(OwnrsWorld::run("tests/features"));
}
