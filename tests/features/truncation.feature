Feature: Long value truncation

  Background:
    Given the following repos:
      | repo_name  | status  | catalog_owner | codeowners_teams                                | admin_teams | pushed_at  |
      | alpha-repo | aligned | my-team       | team-a, team-b, team-c, team-d, team-e, team-f |             | 2026-04-14 |

  Scenario: Long values are truncated with ellipsis
    When I render the table
    Then stdout should contain "…"
