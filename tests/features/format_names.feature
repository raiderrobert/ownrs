Feature: Format names

  Background:
    Given the fixtures from "format_names"

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
