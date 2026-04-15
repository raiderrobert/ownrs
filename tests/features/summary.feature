Feature: Summary flag

  Background:
    Given the following repos:
      | repo_name  | status  | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | alpha-repo | aligned | my-team       | my-team          | my-team     | 2026-04-14 |
      | beta-svc   | stale   | old-team      | old-team         |             | 2026-04-10 |

  Scenario: Summary flag shows status count table
    When I render the summary
    Then stdout should contain "Status"
    And stdout should contain "Count"
    And stdout should contain "Aligned"
    And stdout should contain "1"
