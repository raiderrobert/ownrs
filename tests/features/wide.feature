Feature: Wide output

  Background:
    Given the fixtures from "wide"

  Scenario: Wide flag adds Admin Teams and Notes columns
    When I run ownrs "org testorg --wide"
    Then stdout should contain "ADMIN TEAMS"
    And stdout should contain "NOTES"
    And stdout should contain "my-team, ops-team"
