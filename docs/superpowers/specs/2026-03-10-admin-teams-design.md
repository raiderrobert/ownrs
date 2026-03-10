# Design: Per-Repo Admin Teams as Third Ownership Signal

## Summary

Add GitHub repo admin team permissions as a third ownership signal alongside CODEOWNERS and catalog-info.yaml. Change CODEOWNERS parsing to return all teams (not just first). Add `--strict` flag for exact-set matching vs default intersection.

## Data Model Changes

### `reconcile/types.rs`

```rust
pub enum AlignmentStatus {
    Aligned,
    Mismatched,
    CatalogOnly,
    CodeownersOnly,
    AdminOnly,      // NEW: admin teams present, no metadata files
    Stale,
    Missing,
}

pub struct RepoOwnership {
    pub repo_name: String,
    pub pushed_at: Option<DateTime<Utc>>,
    pub catalog_owner: Option<String>,           // unchanged (Backstage spec.owner is scalar)
    pub codeowners_teams: Vec<String>,            // was: codeowners_team: Option<String>
    pub admin_teams: Vec<String>,                 // NEW
    pub catalog_team_exists: Option<bool>,         // unchanged
    pub codeowners_teams_exist: Vec<(String, bool)>, // was: codeowners_team_exists: Option<bool>
    pub alignment: AlignmentStatus,
    pub notes: Vec<String>,
}
```

JSON serialization: keep a computed `codeowners_team` field (first element of `codeowners_teams`) for backward compat using `#[serde(serialize_with)]` or custom Serialize impl. New fields are additive.

### `sources/fetcher.rs`

```rust
pub struct RepoSources {
    pub repo_name: String,
    pub codeowners: Option<String>,
    pub catalog_info: Option<String>,
    pub admin_teams: Vec<String>,    // NEW
}
```

## Source Changes

### `sources/codeowners.rs`

`extract_team` becomes `extract_teams`, returning `Vec<String>`:

```rust
pub fn extract_teams(content: &str) -> Vec<String> {
    // Find the * wildcard rule
    // Collect ALL @org/team patterns (not just first)
    // Filter out plain @usernames (no slash = not a team)
    // Deduplicate while preserving order
    // Return vec (empty if no teams found)
}

// Backward compat wrapper (used nowhere after migration, can remove)
pub fn extract_team(content: &str) -> Option<String> {
    extract_teams(content).into_iter().next()
}
```

### `github/repo_teams.rs` (NEW)

Fetch per-repo admin teams via `GET /repos/{owner}/{repo}/teams`:

```rust
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

    // Paginate using octocrab's pagination pattern (check .next.is_none())
    // Filter to teams where permission == "admin"
    // Handle 403/404 gracefully: return empty vec + log warning
    // Cache result

    cache.set(&cache_key, &slugs)?;
    Ok(slugs)
}
```

Implementation note: verify octocrab 0.44 supports `GET /repos/{owner}/{repo}/teams`. If not, use `reqwest` directly (already a dependency).

### `sources/fetcher.rs`

Add admin team fetching inside `fetch_all`. The admin teams call runs alongside the existing CODEOWNERS and catalog-info.yaml fetches within the same semaphore permit per repo. Cache key: `admin_teams_{org}_{repo}`.

## Reconciliation Logic

### `reconcile/alignment.rs`

New signature:
```rust
pub fn reconcile(
    repo_name: &str,
    pushed_at: Option<DateTime<Utc>>,
    catalog_owner: Option<&str>,
    codeowners_teams: &[String],
    admin_teams: &[String],
    valid_teams: &HashSet<String>,
    strict: bool,
) -> RepoOwnership
```

Algorithm (two phases):

**Phase 1: Stale detection (runs first, takes priority)**
- Check catalog_owner against valid_teams
- Check each codeowners team against valid_teams
- Check each admin team against valid_teams
- If ANY referenced team doesn't exist → `Stale` with notes listing which teams are missing

**Phase 2: Alignment (only if no stale teams)**

Determine which sources are "present":
- catalog: `catalog_owner.is_some()`
- codeowners: `!codeowners_teams.is_empty()`
- admin: `!admin_teams.is_empty()`

Count present sources:

| Present sources | Result |
|---|---|
| 0 | `Missing` |
| 1 (catalog only) | `CatalogOnly` |
| 1 (codeowners only) | `CodeownersOnly` |
| 1 (admin only) | `AdminOnly` |
| 2+ | Compute intersection or strict (below) |

For 2+ present sources, normalize all team names via `normalize_team` (lowercase + trim), convert each source to a `HashSet<String>`:
- catalog → `{catalog_owner}`
- codeowners → `{codeowners_teams...}`
- admin → `{admin_teams...}`

**Intersection mode (default):** Compute global intersection of all present sources' sets. Non-empty intersection → `Aligned`. Empty intersection → `Mismatched` with notes explaining which sources disagree.

**Strict mode (`--strict`):** All present sources must have exactly the same team set. Equal → `Aligned`. Not equal → `Mismatched` with notes.

## CLI Changes

### `cli.rs`

Add `--strict` flag to both `Org` and `Repo` subcommands:
```rust
/// Require exact team set match across all sources (default: intersection)
#[arg(long)]
strict: bool,
```

Add `AdminOnly` to `StatusFilter` enum.

### `config.rs`

Add `strict: bool` to both `Scope::Org` and `Scope::Repo` variants. Thread from CLI through to `reconcile()`.

## Output Changes

### Table (`output/table.rs`)

Summary table: add `AdminOnly` row.

Detail table: add "Admin Teams" column. Multiple teams comma-separated. If >5 teams, show first 5 + "+N more" in table view only.

Single-repo view: add "Admin Teams:" line.

### CSV (`output/csv.rs`)

Existing columns preserved. Changes:
- `codeowners_team` column kept (shows first team for backward compat)
- New columns appended at end: `codeowners_teams_all`, `admin_teams`
- Multi-team values comma-separated within quoted CSV fields

### JSON (`output/json.rs`)

Additive changes:
- `codeowners_team`: kept as computed first element (backward compat)
- `codeowners_teams`: new array field
- `admin_teams`: new array field
- `codeowners_teams_exist`: new array of `{team, exists}` objects
- `AdminOnly` added to alignment enum serialization

## Caching

- Admin teams cached per repo: key `admin_teams_{org}_{repo}`, 24h TTL
- Cache versioning: if deserialization fails on old cache entries, treat as cache miss (graceful fallback)
- `--refresh` flag clears admin team caches alongside existing caches

## Error Handling

- 403/404 on `/repos/{owner}/{repo}/teams`: treat as empty admin teams, add note "Unable to fetch admin teams (insufficient permissions)"
- Do NOT fail the entire org scan for individual repo permission errors
- Propagate unexpected errors (5xx, network)

## Team Filter Update

`--team` filter must check all three sources:
```rust
let cat_match = /* catalog_owner matches */;
let co_match = /* any codeowners_team matches */;
let admin_match = /* any admin_team matches */;
cat_match || co_match || admin_match
```

Apply to both `run_org` and `run_repo`.

## Test Cases

### CODEOWNERS parsing (unit)
1. Single team → `vec!["team-a"]`
2. Multiple teams → `vec!["team-a", "team-b"]`
3. Mixed users and teams → filters out `@username`, keeps `@org/team`
4. Duplicate teams → deduplicated, preserves order
5. No wildcard rule → empty vec
6. Empty file → empty vec
7. Only comments → empty vec

### Reconciliation — intersection mode (unit)
8. All three agree → `Aligned`
9. Two of three agree (intersection non-empty) → `Aligned`
10. No overlap across present sources → `Mismatched`
11. Catalog only → `CatalogOnly`
12. Codeowners only → `CodeownersOnly`
13. Admin only → `AdminOnly`
14. None present → `Missing`
15. Any team stale → `Stale` (regardless of other alignment)
16. Two sources, no admin teams → backward compat (same as current behavior)
17. Case-insensitive matching → `Aligned`

### Reconciliation — strict mode (unit)
18. All three identical sets → `Aligned`
19. Superset (codeowners has extra team) → `Mismatched`
20. Two sources only, identical → `Aligned`
21. Single source → same as intersection (CatalogOnly/CodeownersOnly/AdminOnly)

### Integration
22. `--team` filter matches admin teams
23. `--status admin-only` filter works
24. `--strict` flag threads through to reconciliation
25. CSV output has new columns at end
26. JSON output has both `codeowners_team` and `codeowners_teams`
27. Cached admin teams avoid API calls on second run

## Implementation Order

1. **CODEOWNERS multi-team** — `extract_teams`, update callers, update tests. Ship independently; no new API calls.
2. **Admin team fetching** — new `github/repo_teams.rs`, integrate into `fetch_all`, caching.
3. **Reconciliation rewrite** — new three-source alignment logic with intersection/strict.
4. **CLI + config** — `--strict` flag, `AdminOnly` status filter.
5. **Output updates** — table, CSV, JSON changes.
6. **Exit code fix** — while here, fix the `--status aligned` exit code semantics.
