# ownrs — Implementation Plan

Three-way ownership reconciliation CLI: CODEOWNERS + Backstage `catalog-info.yaml` + GitHub Teams.

No existing OSS tool does this. CODEOWNERS parsing is well-covered (`codeowners-rs`, `mszostok/codeowners-validator`), and Backstage has a one-way `CodeOwnersProcessor`, but nobody compares across all three sources. Commercial products (Cortex, OpsLevel, Sourcegraph Own) partially address it but aren't open source.

This tool fills the gap.

## Prior Art / Dependencies

| Crate | Purpose |
|-------|---------|
| `codeowners-rs` (hmarr) | CODEOWNERS parsing — hand-written parser with NFA matching |
| `serde_yaml` | Parse `catalog-info.yaml` |
| `octocrab` | GitHub REST/GraphQL API (teams, repo contents) |
| `clap` | CLI argument parsing |
| `comfy-table` | Terminal table output |
| `serde` / `serde_json` | Serialization, JSON output |
| `tokio` | Async runtime |
| `reqwest` | HTTP (pulled in by octocrab, also usable for Backstage API) |
| `directories` | XDG-compliant cache paths (`~/.cache/ownrs/`) |

## Architecture

```
src/
├── main.rs              # CLI entry point, clap setup
├── cli.rs               # Clap arg definitions
├── config.rs            # Runtime config from args + env
├── github/
│   ├── mod.rs
│   ├── client.rs        # GitHub API wrapper (octocrab)
│   ├── teams.rs         # Fetch + cache org teams
│   └── repos.rs         # Fetch repo list (GraphQL pagination)
├── sources/
│   ├── mod.rs
│   ├── codeowners.rs    # Parse CODEOWNERS, extract top-level team
│   ├── catalog.rs       # Parse catalog-info.yaml, extract spec.owner
│   └── fetcher.rs       # Fetch source files from repos (async, parallel)
├── reconcile/
│   ├── mod.rs
│   ├── alignment.rs     # Core reconciliation logic
│   └── types.rs         # AlignmentStatus, RepoOwnership, AuditResult
├── cache/
│   ├── mod.rs
│   └── file_cache.rs    # File-based cache with TTL
└── output/
    ├── mod.rs
    ├── table.rs          # comfy-table output
    ├── csv.rs            # CSV output
    └── json.rs           # JSON output
```

## Data Model

```rust
enum AlignmentStatus {
    Aligned,          // Both sources present, match, team exists
    Mismatched,       // Both present but disagree
    CatalogOnly,      // Only catalog-info.yaml has ownership
    CodeownersOnly,   // Only CODEOWNERS has ownership
    Stale,            // Referenced team(s) don't exist in GitHub
    Missing,          // No ownership metadata anywhere
}

struct RepoOwnership {
    repo_name: String,
    catalog_owner: Option<String>,      // from catalog-info.yaml spec.owner
    codeowners_team: Option<String>,    // from CODEOWNERS top-level rule
    catalog_team_exists: Option<bool>,  // validated against GitHub teams
    codeowners_team_exists: Option<bool>,
    alignment: AlignmentStatus,
    notes: Vec<String>,
}

struct AuditSummary {
    total: usize,
    aligned: usize,
    mismatched: usize,
    catalog_only: usize,
    codeowners_only: usize,
    stale: usize,
    missing: usize,
    repos: Vec<RepoOwnership>,
}
```

## Implementation Steps

### Step 1: CLI skeleton + config

Set up `clap` with all flags. Map args to a `Config` struct.

```
ownrs [OPTIONS]

Options:
  --org <ORG>          GitHub org (required, or detected from git remote)
  --limit <N>          Audit first N repos (default: 25)
  --all                Audit all repos
  --sort <ORDER>       Sort: stale (default), active, name
  --team <NAME>        Filter to repos owned by team
  --missing-only       Only show repos with no ownership
  --format <FMT>       Output: table (default), csv, json
  --table              Show per-repo breakdown
  --detail             Per-repo breakdown with notes
  --refresh            Force re-fetch cached data
  --cache-dir <DIR>    Cache directory (default: ~/.cache/ownrs)
  --cache-ttl <SECS>   Cache TTL in seconds (default: 86400)
  --token <TOKEN>      GitHub token (default: GITHUB_TOKEN env var)
```

**Files**: `main.rs`, `cli.rs`, `config.rs`

### Step 2: GitHub client + team fetching

Wrap octocrab for org team listing with pagination. Cache team slugs to `~/.cache/ownrs/teams.json` with TTL.

**Files**: `github/client.rs`, `github/teams.rs`, `cache/file_cache.rs`

### Step 3: Repo listing via GraphQL

Paginated GraphQL query for repo listing. Support sort order (pushed_at asc/desc, name), limit, cursor-based pagination. Exclude archived and forked repos.

**Files**: `github/repos.rs`

### Step 4: Source file fetching

Async parallel fetch of `catalog-info.yaml` and `CODEOWNERS` (check both root and `.github/` locations) for each repo. Use a semaphore for concurrency limits. Cache fetched files with TTL.

**Files**: `sources/fetcher.rs`

### Step 5: CODEOWNERS parsing

Use `codeowners-rs` to parse CODEOWNERS content. Extract the top-level (`*`) rule's team. Handle `@org/team-name` format, strip org prefix.

**Files**: `sources/codeowners.rs`

### Step 6: catalog-info.yaml parsing

Parse with `serde_yaml`. Extract `spec.owner`. Handle `group:team-name` prefix stripping. Handle multi-document YAML correctly (unlike naive regex/sed approaches).

**Files**: `sources/catalog.rs`

### Step 7: Reconciliation engine

The core logic: given catalog owner, codeowners team, and set of valid GitHub teams, determine `AlignmentStatus`.

**Files**: `reconcile/alignment.rs`, `reconcile/types.rs`

### Step 8: Output formatters

- **Table**: Summary stats + optional per-repo table via `comfy-table`
- **CSV**: Header + rows
- **JSON**: `serde_json` serialization of `AuditSummary`

**Files**: `output/table.rs`, `output/csv.rs`, `output/json.rs`

### Step 9: Integration + progress reporting

Wire everything together in `main.rs`. Add progress indicators (stderr) during fetching. Handle errors gracefully (partial results on API failures).

### Step 10: Tests

- Unit tests for CODEOWNERS parsing edge cases
- Unit tests for catalog-info.yaml parsing (multi-document, nested, missing fields)
- Unit tests for reconciliation logic (all 6 alignment states)
- Integration tests with mock GitHub API responses

### Step 11: Polish for release

- `--help` text and examples
- Error messages that suggest fixes
- GitHub Actions CI (cargo test, clippy, fmt)
- LICENSE (MIT)

## Design Decisions

**Why octocrab over raw reqwest?**
Typed GitHub API client, handles pagination and auth. Less boilerplate.

**Why not shell out to `gh` CLI?**
A standalone binary should own its HTTP calls — faster, no subprocess overhead, works without `gh` installed. Support `GITHUB_TOKEN` env var for auth.

**Why file-based cache instead of SQLite?**
Simplicity. File-based caching with TTL is sufficient at this scale. SQLite adds a dependency for no real benefit.

**Why not make the org configurable via config file?**
Keep it simple. `--org` flag + env var is enough. Config files are scope creep for v1.

**Org-agnostic from day one.**
`--org` is required (or auto-detected from git remote). No hardcoded org names anywhere.

## Non-Goals (v1)

- Backstage Catalog API integration (querying a running Backstage instance) — file-based only for now
- Auto-fix / PR generation (generating PRs to fix misalignment)
- CODEOWNERS syntax validation (use `mszostok/codeowners-validator` for that)
- Per-file ownership resolution (this tool is repo-level only)
- GitHub App auth (PAT only)
