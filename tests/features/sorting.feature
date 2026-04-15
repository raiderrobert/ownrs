Feature: Sorting

  Background:
    Given the fixtures from "sorting"

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
