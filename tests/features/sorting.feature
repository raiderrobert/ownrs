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
    Then stdout should contain "REPO"
    And stdout should not contain "STATUS↑"
