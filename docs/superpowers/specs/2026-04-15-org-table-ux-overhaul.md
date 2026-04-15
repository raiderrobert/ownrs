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

## BDD stories

Output formatting tests using cucumber-rs. Tests feed canned `RepoOwnership` data into the rendering layer and assert on stdout. No GitHub API calls.

### Infrastructure

- `tests/bdd.rs` — World struct holding stdout/stderr capture, step definitions, main runner
- `tests/features/` — Gherkin feature files
- Depends on: `cucumber`, `futures`, `tempfile` dev-dependencies

### Feature: Default table output

```gherkin
Feature: Default table output

  Background:
    Given the following repos:
      | repo_name    | status  | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | beta-service | stale   | usermgmt      | usermgmt         |             | 2026-04-10 |
      | alpha-repo   | aligned | my-team       | my-team          | my-team     | 2026-04-14 |
      | gamma-tool   | missing | -             | -                |             | 2026-03-01 |

  Scenario: Default output shows detail table sorted by repo name
    When I render the table
    Then stdout should contain "REPO"
    And stdout should contain "STATUS"
    And stdout should contain "CATALOG OWNER"
    And stdout should contain "CODEOWNERS TEAMS"
    And stdout should contain "LAST PUSH"
    And the first data row should start with "alpha-repo"
    And the second data row should start with "beta-service"
    And the third data row should start with "gamma-tool"

  Scenario: Default output includes tally footer with percentages
    When I render the table
    Then stdout should contain "1 aligned (33%)"
    And stdout should contain "1 stale (33%)"
    And stdout should contain "1 missing (33%)"
    And stdout should not contain "0 "

  Scenario: Default output includes title line with count
    Given the team filter is "my-team"
    When I render the table
    Then stdout should contain "repos(my-team)["

  Scenario: Default output does not show Admin Teams or Notes columns
    When I render the table
    Then stdout should not contain "ADMIN TEAMS"
    And stdout should not contain "NOTES"
```

### Feature: Wide output

```gherkin
Feature: Wide output

  Background:
    Given the following repos:
      | repo_name  | status  | catalog_owner | codeowners_teams | admin_teams       | pushed_at  | notes                  |
      | alpha-repo | aligned | my-team       | my-team          | my-team, sec-eng  | 2026-04-14 |                        |
      | beta-svc   | stale   | usermgmt      | usermgmt         | my-team           | 2026-04-10 | references stale team  |

  Scenario: Wide flag adds Admin Teams and Notes columns
    When I render the table with "--wide"
    Then stdout should contain "ADMIN TEAMS"
    And stdout should contain "NOTES"
    And stdout should contain "my-team, sec-eng"
    And stdout should contain "references stale team"
```

### Feature: Summary flag

```gherkin
Feature: Summary flag

  Background:
    Given the following repos:
      | repo_name  | status  | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | alpha-repo | aligned | my-team       | my-team          | my-team     | 2026-04-14 |
      | beta-svc   | stale   | usermgmt      | usermgmt         |             | 2026-04-10 |

  Scenario: Summary flag shows status count table
    When I render the table with "--summary"
    Then stdout should contain "Status"
    And stdout should contain "Count"
    And stdout should contain "Aligned"
    And stdout should contain "1"
```

### Feature: Sorting

```gherkin
Feature: Sorting

  Background:
    Given the following repos:
      | repo_name    | status  | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | charlie-svc  | aligned | team-a        | team-a           |             | 2026-04-10 |
      | alpha-repo   | stale   | usermgmt      | usermgmt         |             | 2026-04-14 |
      | beta-tool    | missing | -             | -                |             | 2026-03-01 |

  Scenario: Sort by status
    When I render the table with "--sort status"
    Then the first data row should start with "charlie-svc"
    And the sort indicator should be on "STATUS"

  Scenario: Sort by last-push
    When I render the table with "--sort last-push"
    Then the first data row should start with "beta-tool"

  Scenario: Multi-column sort
    When I render the table with "--sort status,repo"
    Then the sort indicator should be on "STATUS"

  Scenario: Sort indicator arrow on sorted column
    When I render the table with "--sort repo"
    Then stdout should contain "REPO↑"
    And stdout should not contain "STATUS↑"
```

### Feature: Format names

```gherkin
Feature: Format names

  Background:
    Given the following repos:
      | repo_name    | status  | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | beta-service | stale   | usermgmt      | usermgmt         |             | 2026-04-10 |
      | alpha-repo   | aligned | my-team       | my-team          |             | 2026-04-14 |

  Scenario: Names format outputs one repo per line alphabetically
    When I render with "--format names"
    Then stdout should be:
      """
      alpha-repo
      beta-service
      """

  Scenario: Names format has no headers
    When I render with "--format names"
    Then stdout should not contain "REPO"
    And stdout should not contain "STATUS"
```

### Feature: Truncation

```gherkin
Feature: Long value truncation

  Background:
    Given the following repos:
      | repo_name  | status  | catalog_owner | codeowners_teams                                    | admin_teams | pushed_at  |
      | alpha-repo | aligned | my-team       | team-a, team-b, team-c, team-d, team-e, team-f     |             | 2026-04-14 |

  Scenario: Long values are truncated with ellipsis
    When I render the table
    Then stdout should contain "…"
    And no output line should exceed the terminal width
```
