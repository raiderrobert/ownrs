# ownrs org table UX overhaul

## Problem

The `ownrs org` command defaults to a summary-only view that requires `--detail` to see per-repo data. The detail table uses comfy-table cell wrapping which makes rows very tall when columns contain long team lists. The result is hard to scan and wastes vertical space.

## Design

### Default output

The detail table becomes the default. No `--detail` flag needed.

Output structure:
1. **Title line**: `repos(<filter>)[<count>]` — shows the active team filter and total repo count
2. **Column headers**: Uppercase, space-padded, no borders. Sort indicator arrow (`↑`/`↓`) on the primary sort column.
3. **Data rows**: Space-padded columns. Long values truncated with `…` instead of cell wrapping.
4. **Tally footer**: Status breakdown with inline percentages, zero-count statuses omitted. Format: `5 aligned (42%) · 5 stale (42%) · 1 codeowners-only (8%) · 1 admin-only (8%)`

Default columns: REPO, STATUS, CATALOG OWNER, CODEOWNERS TEAMS, LAST PUSH

Example:
```
repos(workspace-management)[12]

REPO↑                             STATUS           CATALOG OWNER        CODEOWNERS TEAMS     LAST PUSH
api-gateway                       aligned          workspace-management workspace-management 2026-04-10
auth0-config                      codeowners-only  -                    workspace-management 2026-04-13
clouddeploy-terraform             aligned          workspace-management workspace-management 2026-04-10

5 aligned (42%) · 5 stale (42%) · 1 codeowners-only · 1 admin-only (8%)
```

### --wide flag

Adds ADMIN TEAMS and NOTES columns to the default table. Same formatting rules (truncate, no wrapping).

### --summary flag

Shows the full summary table (status counts with percentages in a bordered table). Can combine with the default detail view or use standalone.

### --sort <columns>

Accepts comma-separated column names. Sorts by first column, then uses subsequent columns as tiebreakers. Arrow indicator on the primary sort column.

Valid values: `repo`, `status`, `catalog-owner`, `codeowners-teams`, `last-push`, `admin-teams`, `notes`

Default: `repo` ascending (alphabetical).

The old `stale` and `active` sort values are replaced by `last-push` (ascending = stale first, descending = active first).

### --format names

Outputs one repo name per line with no headers. Designed for piping to other commands.

### CLI changes

| Flag | Before | After |
|---|---|---|
| `--detail` | Required for per-repo table | Removed (detail is default) |
| `--summary` | N/A (was default) | Opt-in for full summary table |
| `--wide` | N/A | Adds Admin Teams + Notes columns |
| `--sort` default | `stale` | `repo` |
| `--sort` values | `stale`, `active`, `name` | Any column name, comma-separated |
| `--format` values | `table`, `csv`, `json` | `table`, `csv`, `json`, `names` |

### Table rendering

Replace comfy-table bordered output with custom padded-column rendering:
- Column widths calculated from data
- Values truncated with `…` when they exceed a reasonable max width
- No borders between rows or columns
- Uppercase headers

### Breaking changes

- `--detail` removed. Could keep as hidden no-op for one release cycle.
- `--sort stale` / `--sort active` no longer valid. Use `--sort last-push`.
- Default sort changes from `stale` to `repo`.
- Default output changes from summary table to detail table with tally footer.
