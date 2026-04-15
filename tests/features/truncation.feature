Feature: Long value truncation

  Background:
    Given the fixtures from "truncation"

  Scenario: Long values are truncated with ellipsis
    When I run ownrs "org testorg"
    Then stdout should contain "…"
