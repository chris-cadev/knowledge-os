@prd-0003 @e2e @search-workflows
Feature: End-to-End Search Workflows
  As a knowledge worker
  I want to search across my knowledge base using different search modes
  So that I can find relevant entities efficiently

  Background:
    Given an empty database

  Scenario: Import then search by keyword
    Given a directory with files:
      | filename   | content                                       |
      | paper.md   | # Transformer Architecture\n\nSelf-attention.  |
      | concept.md | # Neural Networks\n\nDeep learning models.      |
    When I run "kos import <directory>"
    And I run "kos search transformer"
    Then the output contains "Transformer Architecture"
    And the output contains "Found"

  Scenario: Import, search, then traverse results
    Given a directory with files:
      | filename   | content                                       |
      | paper.md   | # Transformer Paper\n\nAttention mechanism.     |
      | concept.md | # Self Attention\n\nKey concept in ML.          |
    When I run "kos import <directory>"
    And I run "kos search transformer"
    Then the output contains "Transformer Paper"
    And the output contains "Found"

  Scenario: Search with type filter after import
    Given I import a file "concept.md" with content:
      """
      ---
      title: "Gradient Descent Concept"
      type: Concept
      ---
      # Gradient Descent Concept

      Type: concept
      """
    And I import a file "paper.md" with content:
      """
      ---
      title: "Gradient Descent Paper"
      type: Paper
      ---
      # Gradient Descent Paper

      Type: paper
      """
    When I run "kos search gradient --type Concept"
    Then the output contains "Gradient Descent Concept"

  Scenario: Keyword search still works as default
    Given a directory with files:
      | filename   | content                                       |
      | concept.md | # Transformer\n\nType: concept                 |
    When I run "kos import <directory>"
    And I run "kos search transformer"
    Then the output contains "Transformer"

  Scenario: Search result has snippet
    Given a directory with files:
      | filename   | content                                       |
      | paper.md   | # Attention Paper\n\nThe transformer uses self-attention mechanisms. |
    When I run "kos import <directory>"
    And I run "kos search attention"
    Then the output contains "Snippet:"
