# Criteria: Per-Repo Admin Teams as Third Ownership Signal

## Problem

ownrs reconciles ownership across CODEOWNERS and catalog-info.yaml, using GitHub Teams API only to validate team existence. It does not check which teams actually have admin access to each repo. This means a repo can be "aligned" (CODEOWNERS and catalog agree) but the named team may not actually have admin permissions on the repo.

## Constraints (decided by user)

1. Only `admin` permission level counts as ownership
2. CODEOWNERS parser must return ALL teams on the `*` rule (currently returns only first)
3. Default alignment: intersection (any team overlap across all present sources = aligned)
4. `--strict` flag: requires all sources to have the exact same team set
5. Admin teams fetched via `GET /repos/{owner}/{repo}/teams`, filtered to admin permission
6. Results cached with same 24h TTL as existing caches
7. Tool stays read-only — no mutations

## Evaluation Dimensions

### Correctness
- Alignment logic handles all combinations of 0-3 sources being present
- Intersection vs strict mode produce correct results for edge cases
- Multi-team CODEOWNERS parsing doesn't break existing single-team behavior
- Stale detection works across all three sources

### Performance
- Additional API calls are bounded by existing semaphore (20 concurrent)
- Caching prevents repeated fetches
- No regression in startup time for cached runs

### Code Quality
- Follows existing module structure (sources/, reconcile/, output/)
- Existing tests updated, new tests added for multi-team and admin team scenarios
- Minimal changes to public interfaces where possible

### Output Quality
- Table/CSV/JSON outputs clearly show admin teams alongside existing columns
- Notes explain WHY something is mismatched (which source disagrees)
- Detail view is still readable with multiple teams per column

### Backward Compatibility
- Default behavior (no --strict) should not break existing CI pipelines using exit codes
- JSON schema changes are additive (new fields, not renamed/removed)
- CSV adds columns at the end

## Current Architecture (for reference)

```
sources/fetcher.rs    — fetches CODEOWNERS + catalog-info.yaml per repo
sources/codeowners.rs — extracts single team from * rule
sources/catalog.rs    — extracts single owner from spec.owner
github/teams.rs       — fetches all org team slugs (existence check)
reconcile/alignment.rs — compares catalog owner vs codeowners team, checks existence
reconcile/types.rs    — RepoOwnership, AlignmentStatus, AuditSummary
output/table.rs       — table rendering
output/csv.rs         — CSV rendering
output/json.rs        — JSON rendering
cli.rs                — CLI arg parsing
main.rs               — orchestration
```
