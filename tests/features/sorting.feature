Feature: Sorting

  Background:
    Given the valid teams are "team-a"
    And a test org with the following repos:
      | repo_name   | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | charlie-svc | team-a        | team-a           |             | 2026-04-10 |
      | alpha-repo  | old-team      | old-team         |             | 2026-04-14 |
      | beta-tool   | -             | -                |             | 2026-03-01 |

  Scenario: Sort by status
    When I run ownrs "org testorg --sort status"
    Then the first data row should start with "charlie-svc"

  Scenario: Sort by last-push
    When I run ownrs "org testorg --sort last-push"
    Then the first data row should start with "beta-tool"

  Scenario: Multi-column sort
    When I run ownrs "org testorg --sort status,repo"
    Then the first data row should start with "charlie-svc"

  Scenario: Sort indicator arrow on sorted column
    When I run ownrs "org testorg --sort repo"
    Then stdout should contain "REPO↑"
    And stdout should not contain "STATUS↑"
