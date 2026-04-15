use std::path::PathBuf;

use cucumber::{given, then, when, World};
use tempfile::TempDir;

#[derive(Debug, Default, World)]
pub struct OwnrsWorld {
    _temp_dir: Option<TempDir>,
    cache_dir: Option<PathBuf>,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

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

// ---------------------------------------------------------------------------
// Given steps
// ---------------------------------------------------------------------------

#[given(expr = "the fixtures from {string}")]
fn given_fixtures(world: &mut OwnrsWorld, fixture_name: String) {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(&fixture_name);

    assert!(
        fixture_dir.exists(),
        "Fixture directory not found: {}",
        fixture_dir.display()
    );

    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let cache_dir = temp_dir.path().to_path_buf();

    // Copy all fixture files to the temp cache dir
    for entry in std::fs::read_dir(&fixture_dir).unwrap() {
        let entry = entry.unwrap();
        let dest = cache_dir.join(entry.file_name());
        std::fs::copy(entry.path(), &dest).unwrap();
    }

    world._temp_dir = Some(temp_dir);
    world.cache_dir = Some(cache_dir);
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
