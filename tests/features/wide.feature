Feature: Wide output

  Background:
    Given the valid teams are "my-team, ops-team"
    And a test org with the following repos:
      | repo_name  | catalog_owner | codeowners_teams | admin_teams       | pushed_at  |
      | alpha-repo | my-team       | my-team          | my-team, ops-team | 2026-04-14 |
      | beta-svc   | old-team      | old-team         | my-team           | 2026-04-10 |

  Scenario: Wide flag adds Admin Teams and Notes columns
    When I run ownrs "org testorg --wide"
    Then stdout should contain "ADMIN TEAMS"
    And stdout should contain "NOTES"
    And stdout should contain "my-team, ops-team"
