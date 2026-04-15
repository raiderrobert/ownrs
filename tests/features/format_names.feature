Feature: Format names

  Background:
    Given the following repos:
      | repo_name    | status  | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | beta-service | stale   | usermgmt      | usermgmt         |             | 2026-04-10 |
      | alpha-repo   | aligned | my-team       | my-team          |             | 2026-04-14 |

  Scenario: Names format outputs one repo per line alphabetically
    When I render with format "names"
    Then stdout should be:
      """
      alpha-repo
      beta-service
      """

  Scenario: Names format has no headers
    When I render with format "names"
    Then stdout should not contain "REPO"
    And stdout should not contain "STATUS"
