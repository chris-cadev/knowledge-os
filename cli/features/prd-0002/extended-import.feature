@prd-0002 @extended-import
Feature: Extended Import Features

  As a knowledge worker
  I want rich import capabilities for various reference formats
  So that my knowledge graph captures all connections

  Background:
    Given an empty database

  # Entity Type Inference
  @type-inference
  Scenario: Infer entity type from frontmatter type field
    Given a file "typed.md" with content:
      """
      ---
      title: "Typed Entity"
      type: paper
      ---
      
      # Typed Entity
      
      This should be a Paper entity.
      """
    When I run "kos import typed.md"
    Then the output contains "Created:"
    And the output contains "Paper"

  @type-inference
  Scenario: Default to Article when type field missing
    Given a file "untyped.md" with content:
      """
      ---
      title: "Untyped Entity"
      ---
      
      # Untyped Entity
      
      This should be an Article.
      """
    When I run "kos import untyped.md"
    Then the output contains "Created:"
    And the output contains "Article"

  @type-inference
  Scenario: Invalid type field is preserved as-is
    Given a file "invalid-type.md" with content:
      """
      ---
      title: "Invalid Type"
      type: not-a-real-type
      ---
      
      # Invalid Type
      
      Should default to Article.
      """
    When I run "kos import invalid-type.md"
    Then the output contains "Created:"
    And the output contains "not-a-real-type"

  # Extended Cross-Reference Extraction
  @cross-refs
  Scenario: Extract wikilinks as references
    Given I import a file "wikilink.md" with content:
      """
      # Wikilink Test
      
      See [[Important Concept]] for more details.
      """
    When I run "kos search wikilink"
    Then the output contains "Found 1"

  @cross-refs
  Scenario: Extract URLs as references
    Given I import a file "urls.md" with content:
      """
      # URL Test
      
      Check out [this article](https://example.com/article) for context.
      """
    When I run "kos search URL"
    Then the output contains "Found 1"

  @cross-refs
  Scenario: Extract @mentions as references
    Given I import a file "mentions.md" with content:
      """
      # Mention Test
      
      Thanks @john-smith for the contribution.
      """
    When I run "kos search Mention"
    Then the output contains "Found 1"

  # Batch Import Progress
  @progress
  Scenario: Batch import shows progress bar
    Given a directory with 10 files:
      | type | count |
      | md   | 10    |
    When I run "kos import <directory>"
    Then the output contains "Total files: 10"
    And the output contains "Created: 10"

  @progress
  Scenario: Batch import reports created, updated, and errors
    Given I import a file "existing.md" with content:
      """
      # Existing
      
      Content.
      """
    And a directory with files:
      | filename    | content           |
      | new-1.md    | # New 1           |
      | new-2.md    | # New 2           |
    When I run "kos import <directory>"
    Then the output contains "Total files: 3"
    And the output contains "Created: 2"

  # Mixed Format Import
  @mixed
  Scenario: Import handles mixed Markdown and PDF
    Given a directory with files:
      | filename    | content           |
      | notes.md    | # Notes           |
    When I run "kos import <directory>"
    Then the output contains "Total files: 1"
    And the output contains "Created: 1"

  @mixed
  Scenario: Import shows resolution summary for batch
    Given I import a file "batch-original.md" with content:
      """
      ---
      title: "Batch Item"
      ---
      
      # Batch Item
      
      Original.
      """
    And a directory with files:
      | filename         | content                    |
      | batch-original-2.md | # Batch Item\n\nDuplicate. |
    When I run "kos import <directory>"
    Then the output contains "Duplicates resolved: 2"
