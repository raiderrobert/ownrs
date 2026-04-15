Feature: Long value truncation

  Background:
    Given the valid teams are "team-a, team-b, team-c, team-d, team-e, team-f"
    And a test org with the following repos:
      | repo_name  | catalog_owner | codeowners_teams                                 | admin_teams | pushed_at  |
      | alpha-repo | team-a        | team-a, team-b, team-c, team-d, team-e, team-f   |             | 2026-04-14 |

  Scenario: Long values are truncated with ellipsis
    When I run ownrs "org testorg"
    Then stdout should contain "…"
