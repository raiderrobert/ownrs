Feature: Summary flag

  Background:
    Given the valid teams are "my-team"
    And a test org with the following repos:
      | repo_name  | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | alpha-repo | my-team       | my-team          | my-team     | 2026-04-14 |
      | beta-svc   | old-team      | old-team         |             | 2026-04-10 |

  Scenario: Summary flag shows status count table
    When I run ownrs "org testorg --summary"
    Then stdout should contain "Status"
    And stdout should contain "Count"
    And stdout should contain "Aligned"
    And stdout should contain "1"
