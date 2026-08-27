Feature: Invalid CODEOWNERS detection

  Background:
    Given the valid teams are "my-team"
    And a test org with the following repos:
      | repo_name    | catalog_owner | codeowners_teams | admin_teams | pushed_at  |
      | broken-repo  | my-team       | !invalid         |             | 2026-04-10 |
      | healthy-repo | my-team       | my-team          | my-team     | 2026-04-14 |

  Scenario: A CODEOWNERS file with no usable rule is reported as codeowners-invalid
    When I run ownrs "org testorg"
    Then the command should succeed
    And stdout should contain "codeowners-invalid"
    And stdout should contain "1 codeowners-invalid (50%)"

  Scenario: Invalid CODEOWNERS repos can be filtered by status
    When I run ownrs "org testorg --status codeowners-invalid --format names"
    Then stdout should contain "broken-repo"
    And stdout should not contain "healthy-repo"

  Scenario: The reason appears in the notes column
    When I run ownrs "org testorg --wide"
    Then the command should succeed
    And stdout should contain "CODEOWNERS file found but"

  Scenario: Summary table counts invalid CODEOWNERS separately
    When I run ownrs "org testorg --summary"
    Then the command should succeed
    And stdout should contain "Codeowners Invalid"
