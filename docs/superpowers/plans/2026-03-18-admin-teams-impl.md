# Admin Teams Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add per-repo GitHub admin teams as a third ownership signal, multi-team CODEOWNERS parsing, and `--strict` flag.

**Architecture:** Extend the existing sources → reconcile → output pipeline. CODEOWNERS parser returns all teams. A new GitHub API module fetches per-repo admin teams. Reconciliation compares three sources using set intersection (default) or strict equality.

**Tech Stack:** Rust, octocrab 0.44, tokio, clap, serde, comfy-table, csv

**Working directory:** `/Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design/`

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src/sources/codeowners.rs` | Modify | Multi-team extraction from CODEOWNERS |
| `src/github/repo_teams.rs` | Create | Fetch per-repo admin teams from GitHub API |
| `src/github/mod.rs` | Modify | Add `pub mod repo_teams;` |
| `src/sources/fetcher.rs` | Modify | Add `admin_teams` to `RepoSources`, fetch admin teams |
| `src/reconcile/types.rs` | Modify | New fields, `AdminOnly` variant, `AuditSummary` update |
| `src/reconcile/alignment.rs` | Modify | Three-source reconciliation with intersection/strict |
| `src/cli.rs` | Modify | `--strict` flag, `AdminOnly` status filter |
| `src/config.rs` | Modify | Thread `strict` through `Scope` |
| `src/main.rs` | Modify | Wire new params, update team filter |
| `src/output/table.rs` | Modify | Admin teams column, `AdminOnly` row |
| `src/output/csv.rs` | Modify | New columns appended |
| `src/output/json.rs` | No change | Derives from struct (automatic via serde) |

---

## Chunk 1: Multi-Team CODEOWNERS

### Task 1: Update CODEOWNERS parser to return all teams

**Files:**
- Modify: `src/sources/codeowners.rs`

- [ ] **Step 1: Update `extract_team` tests to use new `extract_teams` function**

Replace the existing tests with tests for `extract_teams` returning `Vec<String>`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_wildcard() {
        let content = "* @acme/platform-team\n";
        assert_eq!(extract_teams(content), vec!["platform-team"]);
    }

    #[test]
    fn with_comments() {
        let content = "# Top-level owners\n* @acme/core-team\n/docs @acme/docs-team\n";
        assert_eq!(extract_teams(content), vec!["core-team"]);
    }

    #[test]
    fn multiple_teams_on_wildcard() {
        let content = "* @acme/team-a @acme/team-b\n";
        assert_eq!(extract_teams(content), vec!["team-a", "team-b"]);
    }

    #[test]
    fn mixed_users_and_teams() {
        let content = "* @acme/team-a @alice @acme/team-b\n";
        assert_eq!(extract_teams(content), vec!["team-a", "team-b"]);
    }

    #[test]
    fn duplicate_teams_deduplicated() {
        let content = "* @acme/team-a @acme/team-a @acme/team-b\n";
        assert_eq!(extract_teams(content), vec!["team-a", "team-b"]);
    }

    #[test]
    fn no_wildcard_rule() {
        let content = "/src @acme/backend\n/web @acme/frontend\n";
        assert_eq!(extract_teams(content), Vec::<String>::new());
    }

    #[test]
    fn username_not_team() {
        let content = "* @johndoe\n";
        assert_eq!(extract_teams(content), Vec::<String>::new());
    }

    #[test]
    fn empty_file() {
        assert_eq!(extract_teams(""), Vec::<String>::new());
    }

    #[test]
    fn only_comments() {
        assert_eq!(extract_teams("# just comments\n# nothing else\n"), Vec::<String>::new());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design && cargo test sources::codeowners 2>&1`
Expected: FAIL — `extract_teams` not found

- [ ] **Step 3: Implement `extract_teams` and keep `extract_team` as wrapper**

Replace the full content of `src/sources/codeowners.rs`:

```rust
/// Extract all top-level teams from CODEOWNERS content.
///
/// Looks for the `* @org/team-name` rule and strips the `@org/` prefix.
/// Returns all teams on the wildcard rule, deduplicated, preserving order.
pub fn extract_teams(content: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut teams = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.first() == Some(&"*") {
            for part in &parts[1..] {
                if let Some(team) = parse_owner(part) {
                    if seen.insert(team.clone()) {
                        teams.push(team);
                    }
                }
            }
            return teams;
        }
    }
    teams
}

/// Extract the first top-level team (backward compat wrapper).
pub fn extract_team(content: &str) -> Option<String> {
    extract_teams(content).into_iter().next()
}

fn parse_owner(owner: &str) -> Option<String> {
    let owner = owner.strip_prefix('@')?;
    if let Some((_org, team)) = owner.split_once('/') {
        Some(team.to_string())
    } else {
        None
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design && cargo test sources::codeowners 2>&1`
Expected: All 9 tests PASS

- [ ] **Step 5: Verify full project compiles**

Run: `cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design && cargo build 2>&1`
Expected: Compiles (callers still use `extract_team` which still exists)

- [ ] **Step 6: Commit**

```bash
cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design
git add src/sources/codeowners.rs
git commit -m "feat: extract_teams returns all CODEOWNERS teams with dedup"
```

---

## Chunk 2: Data Model & Admin Team Fetching

### Task 2: Update RepoOwnership and AlignmentStatus

**Files:**
- Modify: `src/reconcile/types.rs`
- Modify: `src/cli.rs`

- [ ] **Step 1: Update `AlignmentStatus` enum**

Add `AdminOnly` variant after `CodeownersOnly`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentStatus {
    Aligned,
    Mismatched,
    CatalogOnly,
    CodeownersOnly,
    AdminOnly,
    Stale,
    Missing,
}
```

Update `Display` impl to add:
```rust
AlignmentStatus::AdminOnly => write!(f, "admin-only"),
```

Update `matches_filter` to add:
```rust
StatusFilter::AdminOnly => *self == AlignmentStatus::AdminOnly,
```

- [ ] **Step 2: Update `StatusFilter` in cli.rs**

Add `AdminOnly` variant:
```rust
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
```

- [ ] **Step 3: Update `RepoOwnership` struct**

```rust
#[derive(Debug, Clone, Serialize)]
pub struct RepoOwnership {
    pub repo_name: String,
    pub pushed_at: Option<DateTime<Utc>>,
    pub catalog_owner: Option<String>,
    pub codeowners_teams: Vec<String>,
    pub admin_teams: Vec<String>,
    pub catalog_team_exists: Option<bool>,
    pub codeowners_teams_exist: Vec<(String, bool)>,
    pub alignment: AlignmentStatus,
    pub notes: Vec<String>,
}
```

- [ ] **Step 4: Update `AuditSummary`**

Add `admin_only` field:
```rust
pub struct AuditSummary {
    pub total: usize,
    pub aligned: usize,
    pub mismatched: usize,
    pub catalog_only: usize,
    pub codeowners_only: usize,
    pub admin_only: usize,
    pub stale: usize,
    pub missing: usize,
    pub repos: Vec<RepoOwnership>,
}
```

Update `from_repos`:
```rust
let admin_only = repos.iter().filter(|r| r.alignment == AlignmentStatus::AdminOnly).count();
```

- [ ] **Step 5: Verify it compiles (expect errors in other files)**

Run: `cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design && cargo check 2>&1`
Expected: Compile errors in `main.rs`, `alignment.rs`, `table.rs`, `csv.rs` — these reference old field names. That's expected; we'll fix them in subsequent tasks.

- [ ] **Step 6: Commit**

```bash
cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design
git add src/reconcile/types.rs src/cli.rs
git commit -m "feat: add AdminOnly status, multi-team fields to RepoOwnership"
```

### Task 3: Add `--strict` flag to CLI and config

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/config.rs`

- [ ] **Step 1: Add `--strict` to both subcommands in cli.rs**

Add to `Org` variant:
```rust
/// Require exact team set match across all sources (default: intersection)
#[arg(long)]
strict: bool,
```

Add to `Repo` variant:
```rust
/// Require exact team set match across all sources (default: intersection)
#[arg(long)]
strict: bool,
```

- [ ] **Step 2: Add `strict` to `Scope` variants in config.rs**

Add `strict: bool` to both `Scope::Org` and `Scope::Repo`.

Update the `from_cli` match arms to include `strict`:

In `Command::Org` destructuring, add `strict`.
In `Scope::Org` construction, add `strict`.

In `Command::Repo` destructuring, add `strict`.
In `Scope::Repo` construction, add `strict`.

- [ ] **Step 3: Commit**

```bash
cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design
git add src/cli.rs src/config.rs
git commit -m "feat: add --strict flag to CLI and config"
```

### Task 4: Create admin team fetcher

**Files:**
- Create: `src/github/repo_teams.rs`
- Modify: `src/github/mod.rs`

- [ ] **Step 1: Create `src/github/repo_teams.rs`**

```rust
use anyhow::Result;
use serde::Deserialize;

use crate::cache::file_cache::FileCache;

use super::client::GitHubClient;

#[derive(Deserialize)]
struct RepoTeam {
    slug: String,
    permission: String,
}

pub async fn fetch_repo_admin_teams(
    client: &GitHubClient,
    org: &str,
    repo: &str,
    cache: &FileCache,
    refresh: bool,
) -> Result<Vec<String>> {
    let cache_key = format!("admin_teams_{org}_{repo}");

    if !refresh {
        if let Some(cached) = cache.get::<Vec<String>>(&cache_key)? {
            return Ok(cached);
        }
    }

    let mut slugs = Vec::new();
    let mut page: u32 = 1;

    loop {
        let route = format!("/repos/{org}/{repo}/teams");
        let result: Result<Vec<RepoTeam>, _> = client
            .octocrab
            .get(&route, Some(&[("per_page", "100"), ("page", &page.to_string())]))
            .await;

        match result {
            Ok(teams) => {
                let before_len = slugs.len();
                for team in &teams {
                    if team.permission == "admin" {
                        slugs.push(team.slug.clone());
                    }
                }
                if teams.len() < 100 {
                    break;
                }
                page += 1;
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("403") || err_str.contains("404") {
                    break;
                }
                return Err(e);
            }
        }
    }

    cache.set(&cache_key, &slugs)?;
    Ok(slugs)
}
```

- [ ] **Step 2: Add module to `src/github/mod.rs`**

```rust
pub mod client;
pub mod repo_teams;
pub mod repos;
pub mod teams;
```

- [ ] **Step 3: Verify it compiles**

Run: `cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design && cargo check 2>&1 | head -5`
Expected: The new module should compile. Other files still have errors from Task 2.

- [ ] **Step 4: Commit**

```bash
cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design
git add src/github/repo_teams.rs src/github/mod.rs
git commit -m "feat: add per-repo admin team fetcher with caching"
```

### Task 5: Integrate admin teams into fetcher

**Files:**
- Modify: `src/sources/fetcher.rs`

- [ ] **Step 1: Add `admin_teams` to `RepoSources` and fetch in `fetch_all`**

Add `admin_teams: Vec<String>` to `RepoSources`.

Add admin team fetching alongside existing file fetches. In the `tokio::spawn` block, after fetching codeowners and catalog, fetch admin teams. Add caching for admin teams alongside existing cache writes.

The key changes:
- Import `crate::github::repo_teams::fetch_repo_admin_teams`
- Add `cached_admin` lookup similar to `cached_co` and `cached_cat`
- Add `admin_teams` fetch inside the spawned task
- Cache the result alongside co/cat
- Include `admin_teams` in `RepoSources` construction

- [ ] **Step 2: Verify it compiles**

Run: `cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design && cargo check 2>&1 | head -10`

- [ ] **Step 3: Commit**

```bash
cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design
git add src/sources/fetcher.rs
git commit -m "feat: fetch admin teams alongside source files in fetcher"
```

---

## Chunk 3: Reconciliation Rewrite

### Task 6: Rewrite reconciliation logic

**Files:**
- Modify: `src/reconcile/alignment.rs`

- [ ] **Step 1: Write new tests first**

Replace existing tests with comprehensive three-source tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn teams(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn sv(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    // Intersection mode (strict=false)

    #[test]
    fn all_three_agree() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-a"]), &sv(&["team-a"]), &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Aligned);
    }

    #[test]
    fn two_of_three_overlap() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-a", "team-b"]), &sv(&["team-a", "team-c"]), &teams(&["team-a", "team-b", "team-c"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Aligned);
    }

    #[test]
    fn no_overlap_across_three() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-b"]), &sv(&["team-c"]), &teams(&["team-a", "team-b", "team-c"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Mismatched);
    }

    #[test]
    fn two_sources_catalog_codeowners_aligned() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-a"]), &[], &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Aligned);
    }

    #[test]
    fn two_sources_catalog_codeowners_mismatched() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-b"]), &[], &teams(&["team-a", "team-b"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Mismatched);
    }

    #[test]
    fn catalog_only() {
        let result = reconcile("repo", None, Some("team-a"), &[], &[], &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::CatalogOnly);
    }

    #[test]
    fn codeowners_only() {
        let result = reconcile("repo", None, None, &sv(&["team-a"]), &[], &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::CodeownersOnly);
    }

    #[test]
    fn admin_only() {
        let result = reconcile("repo", None, None, &[], &sv(&["team-a"]), &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::AdminOnly);
    }

    #[test]
    fn missing_when_none() {
        let result = reconcile("repo", None, None, &[], &[], &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Missing);
    }

    #[test]
    fn stale_catalog_team_gone() {
        let result = reconcile("repo", None, Some("team-gone"), &sv(&["team-a"]), &sv(&["team-a"]), &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Stale);
    }

    #[test]
    fn stale_codeowners_team_gone() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-gone"]), &[], &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Stale);
    }

    #[test]
    fn stale_admin_team_gone() {
        let result = reconcile("repo", None, None, &[], &sv(&["team-gone"]), &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Stale);
    }

    #[test]
    fn case_insensitive_alignment() {
        let result = reconcile("repo", None, Some("Team-A"), &sv(&["team-a"]), &[], &teams(&["team-a", "Team-A"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Aligned);
    }

    // Strict mode (strict=true)

    #[test]
    fn strict_all_identical() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-a"]), &sv(&["team-a"]), &teams(&["team-a"]), true);
        assert_eq!(result.alignment, AlignmentStatus::Aligned);
    }

    #[test]
    fn strict_superset_mismatched() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-a", "team-b"]), &sv(&["team-a"]), &teams(&["team-a", "team-b"]), true);
        assert_eq!(result.alignment, AlignmentStatus::Mismatched);
    }

    #[test]
    fn strict_single_source() {
        let result = reconcile("repo", None, Some("team-a"), &[], &[], &teams(&["team-a"]), true);
        assert_eq!(result.alignment, AlignmentStatus::CatalogOnly);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design && cargo test reconcile::alignment 2>&1`
Expected: FAIL — signature mismatch

- [ ] **Step 3: Implement new `reconcile` function**

Replace the `reconcile` function body with the three-source logic:

```rust
use std::collections::HashSet;

use super::types::{AlignmentStatus, RepoOwnership};
use chrono::{DateTime, Utc};

pub fn reconcile(
    repo_name: &str,
    pushed_at: Option<DateTime<Utc>>,
    catalog_owner: Option<&str>,
    codeowners_teams: &[String],
    admin_teams: &[String],
    valid_teams: &HashSet<String>,
    strict: bool,
) -> RepoOwnership {
    let mut notes = Vec::new();

    // Phase 1: Stale detection
    let catalog_team_exists = catalog_owner.map(|t| valid_teams.contains(t));
    let codeowners_teams_exist: Vec<(String, bool)> = codeowners_teams
        .iter()
        .map(|t| (t.clone(), valid_teams.contains(t)))
        .collect();
    let admin_teams_stale: Vec<&String> = admin_teams
        .iter()
        .filter(|t| !valid_teams.contains(t.as_str()))
        .collect();

    let mut any_stale = false;
    if catalog_team_exists == Some(false) {
        notes.push(format!("catalog-info.yaml references non-existent team: {}", catalog_owner.unwrap()));
        any_stale = true;
    }
    for (team, exists) in &codeowners_teams_exist {
        if !exists {
            notes.push(format!("CODEOWNERS references non-existent team: {team}"));
            any_stale = true;
        }
    }
    for team in &admin_teams_stale {
        notes.push(format!("Admin team no longer exists: {team}"));
        any_stale = true;
    }

    if any_stale {
        return RepoOwnership {
            repo_name: repo_name.to_string(),
            pushed_at,
            catalog_owner: catalog_owner.map(String::from),
            codeowners_teams: codeowners_teams.to_vec(),
            admin_teams: admin_teams.to_vec(),
            catalog_team_exists,
            codeowners_teams_exist,
            alignment: AlignmentStatus::Stale,
            notes,
        };
    }

    // Phase 2: Alignment
    let has_catalog = catalog_owner.is_some();
    let has_codeowners = !codeowners_teams.is_empty();
    let has_admin = !admin_teams.is_empty();
    let present_count = [has_catalog, has_codeowners, has_admin].iter().filter(|&&b| b).count();

    let alignment = match present_count {
        0 => {
            notes.push("No ownership metadata found".to_string());
            AlignmentStatus::Missing
        }
        1 => {
            if has_catalog {
                notes.push("Only catalog-info.yaml has ownership".to_string());
                AlignmentStatus::CatalogOnly
            } else if has_codeowners {
                notes.push("Only CODEOWNERS has ownership".to_string());
                AlignmentStatus::CodeownersOnly
            } else {
                notes.push("Only admin team access found; consider adding CODEOWNERS or catalog-info.yaml".to_string());
                AlignmentStatus::AdminOnly
            }
        }
        _ => {
            // Build normalized sets for each present source
            let mut sets: Vec<HashSet<String>> = Vec::new();
            if has_catalog {
                sets.push([normalize_team(catalog_owner.unwrap())].into_iter().collect());
            }
            if has_codeowners {
                sets.push(codeowners_teams.iter().map(|t| normalize_team(t)).collect());
            }
            if has_admin {
                sets.push(admin_teams.iter().map(|t| normalize_team(t)).collect());
            }

            if strict {
                // All sets must be identical
                let first = &sets[0];
                if sets.iter().all(|s| s == first) {
                    AlignmentStatus::Aligned
                } else {
                    let labels = source_labels(has_catalog, has_codeowners, has_admin);
                    notes.push(format!("Strict mode: sources have different team sets ({})", labels.join(", ")));
                    AlignmentStatus::Mismatched
                }
            } else {
                // Intersection must be non-empty
                let intersection = sets.iter().skip(1).fold(sets[0].clone(), |acc, s| {
                    acc.intersection(s).cloned().collect()
                });
                if !intersection.is_empty() {
                    AlignmentStatus::Aligned
                } else {
                    let labels = source_labels(has_catalog, has_codeowners, has_admin);
                    notes.push(format!("No common team across sources ({})", labels.join(", ")));
                    AlignmentStatus::Mismatched
                }
            }
        }
    };

    RepoOwnership {
        repo_name: repo_name.to_string(),
        pushed_at,
        catalog_owner: catalog_owner.map(String::from),
        codeowners_teams: codeowners_teams.to_vec(),
        admin_teams: admin_teams.to_vec(),
        catalog_team_exists,
        codeowners_teams_exist,
        alignment,
        notes,
    }
}

fn normalize_team(team: &str) -> String {
    team.to_lowercase().trim().to_string()
}

fn source_labels(catalog: bool, codeowners: bool, admin: bool) -> Vec<&'static str> {
    let mut labels = Vec::new();
    if catalog { labels.push("catalog"); }
    if codeowners { labels.push("CODEOWNERS"); }
    if admin { labels.push("admin"); }
    labels
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design && cargo test reconcile::alignment 2>&1`
Expected: All 16 tests PASS

- [ ] **Step 5: Commit**

```bash
cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design
git add src/reconcile/alignment.rs
git commit -m "feat: three-source reconciliation with intersection and strict modes"
```

---

## Chunk 4: Wire Everything Together

### Task 7: Update main.rs orchestration

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update `run_org`**

Changes needed:
- Extract `strict` from `Scope::Org`
- Change `extract_team` call to `extract_teams`
- Pass `codeowners_teams`, `admin_teams`, and `strict` to `reconcile`
- Update team filter to check `codeowners_teams` (any match) and `admin_teams` (any match)

- [ ] **Step 2: Update `run_repo`**

Same changes as `run_org`:
- Extract `strict` from `Scope::Repo`
- Use `extract_teams`
- Pass new params to `reconcile`

- [ ] **Step 3: Verify it compiles**

Run: `cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design && cargo check 2>&1`
Expected: Errors only in output modules (table.rs, csv.rs reference old fields)

- [ ] **Step 4: Commit**

```bash
cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design
git add src/main.rs
git commit -m "feat: wire admin teams and strict mode through main orchestration"
```

### Task 8: Update output modules

**Files:**
- Modify: `src/output/table.rs`
- Modify: `src/output/csv.rs`

- [ ] **Step 1: Update table.rs**

In `print_summary`: add `Admin Only` row after `Codeowners Only`.

In `print_detail`: add "Admin Teams" column header and render `codeowners_teams.join(", ")` for CODEOWNERS column and `admin_teams.join(", ")` for Admin Teams column.

In `print_single_repo`: change `codeowners_team` to `codeowners_teams.join(", ")` or "(none)" if empty. Add "Admin:" line showing `admin_teams.join(", ")` or "(none)". Update `codeowners_team_exists` references to use `codeowners_teams_exist`.

- [ ] **Step 2: Update csv.rs**

Keep existing columns, update references:
- `codeowners_team` column: show `codeowners_teams.first()` or empty
- `codeowners_team_exists`: show first team's existence
- Append new columns at end: `codeowners_teams_all`, `admin_teams`

- [ ] **Step 3: Verify full build and tests pass**

Run: `cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design && cargo test 2>&1`
Expected: All tests PASS, no compile errors

- [ ] **Step 4: Commit**

```bash
cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design
git add src/output/table.rs src/output/csv.rs
git commit -m "feat: add admin teams to table and CSV output"
```

### Task 9: Smoke test with real org

- [ ] **Step 1: Run against pantheon-systems with team filter**

Run: `cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design && GITHUB_TOKEN=$(gh auth token) cargo run -- org pantheon-systems --team reporting --detail 2>&1`

Expected: Should now show subgraph-new-relic, riker-newrelic-cli, and traffic-insights (repos where reporting has admin access or is in CODEOWNERS as a secondary team).

- [ ] **Step 2: Test strict mode**

Run: `cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design && GITHUB_TOKEN=$(gh auth token) cargo run -- org pantheon-systems --team reporting --detail --strict 2>&1`

Expected: Fewer aligned repos (strict requires exact set match).

- [ ] **Step 3: Test admin-only filter**

Run: `cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design && GITHUB_TOKEN=$(gh auth token) cargo run -- org pantheon-systems --status admin-only --detail --limit 20 2>&1`

Expected: Shows repos with admin team access but no CODEOWNERS/catalog.

- [ ] **Step 4: Commit any fixes needed from smoke test**

```bash
cd /Users/robert.roskam/repos/ownrs/.claude/worktrees/admin-teams-design
git add -A
git commit -m "fix: adjustments from smoke testing"
```
