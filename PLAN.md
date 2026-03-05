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
    pushed_at: Option<DateTime>,        // last push timestamp, from GraphQL
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

## Use Cases

### Broken PR review gates

CODEOWNERS enforces who must approve PRs via branch protection. If the referenced team doesn't exist anymore, the gate silently fails — PRs merge without the right reviewers.

```
Given an org that requires CODEOWNERS approval via branch protection
And some teams have been renamed or dissolved after a reorg
When I run `ownrs org acme-corp --status stale --format csv`
Then I get a list of repos where CODEOWNERS references teams that no longer exist
So I can fix the gates before unreviewed code ships to production
```

### Preventing wrong-team incident routing

When Backstage drives on-call routing and CODEOWNERS drives code review, ownership drift means incidents get routed to the wrong team. This isn't a tool you run mid-incident — it's a proactive check run on a regular cadence to catch drift before it matters.

```
Given Backstage drives incident routing
And CODEOWNERS drives code review
And ownership sources can drift apart over time
When I run `ownrs org acme-corp --status mismatched,stale --detail` on a regular cadence
Then I catch repos where routing would go to the wrong team
So I can fix them before the next outage
```

### Post-reorg cleanup

Teams get renamed, merged, split. Ownership metadata across hundreds of repos doesn't update itself.

```
Given "team-legacy" was dissolved and its repos split between "team-alpha" and "team-beta"
When I run `ownrs org acme-corp --team team-legacy`
Then I see every repo that still references "team-legacy" in either source
So I know what needs updating and nothing falls through the cracks
```

### Backstage rollout — bootstrapping the catalog

You're adopting Backstage. You need to know the starting state: which repos already have catalog-info.yaml, which have CODEOWNERS you could bootstrap from, which have nothing.

```
Given an org with 400 repos, some with CODEOWNERS, few with catalog-info.yaml
When I run `ownrs org acme-corp --status codeowners-only,missing --detail`
Then "codeowners-only" repos are where I can auto-generate catalog-info.yaml from existing CODEOWNERS
And "missing" repos are where I need to start from scratch
```

### Compliance audit

SOC2/SOX/etc. require designated owners for production services. An auditor asks "show me every service has an owner."

```
Given an upcoming SOC2 audit
When I run `ownrs org acme-corp --status missing --format csv`
Then I get a list of repos with no ownership metadata at all
So I can remediate before the audit window
And re-run to produce evidence that coverage is 100%
```

### Pre-shipping ownership check

A developer is about to enable branch protection with CODEOWNERS required reviews on their repo. They want to make sure everything is wired up correctly first.

```
Given I'm in my repo's directory
And I just added a CODEOWNERS file and catalog-info.yaml
When I run `ownrs repo`
Then it tells me whether both files reference the same team
And whether that team actually exists in the org
So I know branch protection won't silently fail when I turn it on
```

### Stale repo cleanup

Finding candidates for archiving: repos nobody has touched in a long time that also have no owner.

```
Given an org with hundreds of repos accumulated over years
When I run `ownrs org acme-corp --sort stale --status missing --detail`
Then I see unowned repos ordered by how long since they were last pushed
With pushed_at timestamps so I can distinguish 6-month-old from 6-year-old repos
So I can build an archival shortlist
```

### Use case → command mapping

| Use case | Command |
|----------|---------|
| Broken PR review gates | `ownrs org acme-corp --status stale` |
| Proactive incident routing | `ownrs org acme-corp --status mismatched,stale --detail` |
| Post-reorg cleanup | `ownrs org acme-corp --team team-legacy` |
| Backstage rollout | `ownrs org acme-corp --status codeowners-only,missing --detail` |
| Compliance audit | `ownrs org acme-corp --status missing --format csv` |
| Stale repo cleanup | `ownrs org acme-corp --sort stale --status missing --detail` |
| Pre-shipping check | `ownrs repo` |

### Future use cases

These are out of scope for v0 but worth considering for future iterations:

- **Drift trending / diff over time** — compare the current audit against a prior run to show what changed since last check. Would require persisting results and adding a `--diff` or `--since` flag.
- **Multi-org support** — some companies span multiple GitHub orgs. Supporting `ownrs org acme-corp,acme-platform` or a config file listing orgs would cover this.
- **Team-level rollup** — "show me everything team-alpha owns across both sources." The `--team` filter currently finds legacy refs, but a positive "what does this team own?" view would be useful for team leads.
- **Suggested fixes / auto-remediation** — for "codeowners-only" repos, generate a `catalog-info.yaml` from CODEOWNERS. For "catalog-only" repos, generate a CODEOWNERS entry. Could be a `--fix` flag or a separate `ownrs fix` subcommand.
- **Nested CODEOWNERS / path-level reconciliation** — the plan extracts a "top-level team" but repos often have per-path ownership. Reconciling path-level owners against Backstage could surface deeper mismatches.

## Usage Lifecycle

1. **Ad hoc discovery** — run once to see the state of the world, triage what needs fixing.
2. **CI enforcement** — add to a scheduled GitHub Actions workflow so ownership never drifts again.

The tool supports both modes: human-readable tables for ad hoc use, structured output (`--format json`) and non-zero exit codes for CI.

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | All repos pass (or no repos matched the filter) |
| 1 | One or more repos matched the `--status` filter (drift detected) |
| 2 | Runtime error (auth failure, network error, etc.) |

When `--status` is specified, exit code 1 means "the filter matched something" — i.e., the problem you were looking for exists. When `--status` is not specified (full report mode), exit code is always 0 — the tool is reporting, not asserting.

### CI examples

**Org-wide audit** — platform team runs on a schedule to catch drift across the whole org:

```yaml
on:
  schedule:
    - cron: '0 9 * * 1'  # weekly Monday 9am
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - run: ownrs org my-org --status stale,mismatched,missing --format json
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Team-scoped audit** — a team runs on their own repos:

```yaml
on:
  schedule:
    - cron: '0 9 * * 1'
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - run: ownrs org my-org --team my-team --status stale,mismatched,missing --format json
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Single repo check** — runs in a repo's own CI when ownership files change:

```yaml
on:
  pull_request:
    paths:
      - 'CODEOWNERS'
      - '.github/CODEOWNERS'
      - 'catalog-info.yaml'
jobs:
  ownership-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: ownrs repo --status stale,mismatched,missing
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## CLI Design

Two subcommands:

```
ownrs org <ORG> [OPTIONS]
ownrs repo [ORG/REPO] [OPTIONS]
```

### `ownrs org <ORG>`

Audit repos across a GitHub org. Default output is summary stats.

```
Options:
  --limit <N>          Audit only the first N repos (default: all)
  --sort <ORDER>       Sort: stale (default), active, name
  --team <TEAM>        Filter to repos referencing this team (comma-separated for multiple)
  --status <STATUS>    Filter by alignment status (comma-separated: aligned, mismatched, stale, missing, catalog-only, codeowners-only)
  --format <FMT>       Output: table (default), csv, json
  --detail             Show per-repo breakdown with notes (default is summary only)
```

### `ownrs repo [ORG/REPO]`

Audit a single repo. If `ORG/REPO` is omitted, detect from the git remote of the current directory. Default output is full detail — all three sources, alignment status, and notes.

```
Options:
  --status <STATUS>    Filter by alignment status (comma-separated: aligned, mismatched, stale, missing, catalog-only, codeowners-only)
  --format <FMT>       Output: table (default), json
```

### Global options (all subcommands)

```
  --refresh            Force re-fetch cached data
  --cache-dir <DIR>    Cache directory (default: ~/.cache/ownrs)
  --cache-ttl <SECS>   Cache TTL in seconds (default: 86400)
  --token <TOKEN>      GitHub token (default: GITHUB_TOKEN env var)
```

## Implementation Steps

### Step 1: CLI skeleton + config

Set up `clap` with two subcommands (`org`, `repo`). Map args to a `Config` struct with a `Scope` enum (`Org`, `Repo`).

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

**`--team` is a content filter, not a scoped fetch.**
The `--team` flag filters by matching the team name in CODEOWNERS and catalog-info.yaml file contents, not via GitHub's team→repo API. This means it works for dissolved teams (the post-reorg use case), but it doesn't make the command faster — the full org is fetched either way.

## Non-Goals (v1)

- Backstage Catalog API integration (querying a running Backstage instance) — file-based only for now
- Auto-fix / PR generation (generating PRs to fix misalignment)
- CODEOWNERS syntax validation (use `mszostok/codeowners-validator` for that)
- Per-file ownership resolution (this tool is repo-level only)
- GitHub App auth (PAT only)
