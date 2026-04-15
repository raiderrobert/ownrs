Feature: Format names

  Background:
    Given the valid teams are "my-team"
    And a test org with the following repos:
      | repo_name    | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | beta-service | old-team      | old-team         |             | 2026-04-10 |
      | alpha-repo   | my-team       | my-team          |             | 2026-04-14 |

  Scenario: Names format outputs one repo per line alphabetically
    When I run ownrs "org testorg --format names"
    Then stdout should be:
      """
      alpha-repo
      beta-service
      """

  Scenario: Names format has no headers
    When I run ownrs "org testorg --format names"
    Then stdout should not contain "REPO"
    And stdout should not contain "STATUS"
