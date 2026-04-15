Feature: Wide output

  Background:
    Given the following repos:
      | repo_name  | status  | catalog_owner | codeowners_teams | admin_teams       | pushed_at  | notes                 |
      | alpha-repo | aligned | my-team       | my-team          | my-team, ops-team  | 2026-04-14 |                       |
      | beta-svc   | stale   | old-team      | old-team         | my-team           | 2026-04-10 | references stale team |

  Scenario: Wide flag adds Admin Teams and Notes columns
    When I render the table with "--wide"
    Then stdout should contain "ADMIN TEAMS"
    And stdout should contain "NOTES"
    And stdout should contain "my-team, ops-team"
    And stdout should contain "references stale team"
