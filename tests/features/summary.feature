Feature: Summary flag

  Background:
    Given the fixtures from "summary"

  Scenario: Summary flag shows status count table
    When I run ownrs "org testorg --summary"
    Then stdout should contain "Status"
    And stdout should contain "Count"
    And stdout should contain "Aligned"
    And stdout should contain "1"
