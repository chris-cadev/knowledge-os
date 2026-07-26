Feature: Semantic Search
  As a knowledge worker
  I want to search by meaning, not just keywords
  So that I can find conceptually related entities

  Background:
    Given an empty database

  Scenario: Keyword search still works (default)
    Given a directory with files:
      | filename   | content                          |
      | concept.md | # Transformer\n\nType: concept   |
    When I run "kos import <directory>"
    And I run "kos search transformer"
    Then the output contains "Transformer"

  Scenario: Semantic search warns when no provider configured
    Given a directory with files:
      | filename   | content                              |
      | concept.md | # Neural Network\n\nType: concept    |
    When I run "kos import <directory>"
    And I run "kos search deep-learning --semantic"
    Then the error output contains "Semantic search requires an AI provider"

  Scenario: Hybrid search warns when no provider configured
    Given a directory with files:
      | filename   | content                          |
      | concept.md | # Transformer\n\nType: concept   |
    When I run "kos import <directory>"
    And I run "kos search transformer --hybrid"
    Then the error output contains "Semantic search requires an AI provider"

  Scenario: --semantic and --hybrid are mutually exclusive
    Given an empty database
    When I run "kos search machine-learning --semantic --hybrid"
    Then the error output contains "mutually exclusive"

  Scenario: Keyword search still works after semantic flags added
    Given a directory with files:
      | filename   | content                          |
      | concept.md | # Gradient Descent\n\nType: concept |
    When I run "kos import <directory>"
    And I run "kos search gradient"
    Then the output contains "Gradient Descent"
