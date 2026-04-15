Feature: Default table output

  Background:
    Given the valid teams are "my-team"
    And a test org with the following repos:
      | repo_name    | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | beta-service | old-team      | old-team         |             | 2026-04-10 |
      | alpha-repo   | my-team       | my-team          | my-team     | 2026-04-14 |
      | gamma-tool   | -             | -                |             | 2026-03-01 |

  Scenario: Default output shows detail table sorted by repo name
    When I run ownrs "org testorg"
    Then the command should succeed
    And stdout should contain "REPO"
    And stdout should contain "STATUS"
    And stdout should contain "CATALOG OWNER"
    And stdout should contain "CODEOWNERS TEAMS"
    And stdout should contain "LAST PUSH"
    And the first data row should start with "alpha-repo"
    And the second data row should start with "beta-service"
    And the third data row should start with "gamma-tool"

  Scenario: Default output includes tally footer with percentages
    When I run ownrs "org testorg"
    Then stdout should contain "1 aligned (33%)"
    And stdout should contain "1 stale (33%)"
    And stdout should contain "1 missing (33%)"

  Scenario: Default output includes title line with count
    When I run ownrs "org testorg --team my-team"
    Then stdout should contain "repos(my-team)["

  Scenario: Default output does not show Admin Teams or Notes columns
    When I run ownrs "org testorg"
    Then stdout should not contain "ADMIN TEAMS"
    And stdout should not contain "NOTES"
