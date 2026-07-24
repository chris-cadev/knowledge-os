@prd-0001 @search
Feature: Search and Retrieval

  As a knowledge worker
  I want to search for entities by topic
  So that I can find relevant information quickly

  Background:
    Given an empty database

  # US3: Search for Entities
  @us3
  Scenario: Search by keyword returns matching entities
    Given I import a file "transformer.md" with content:
      """
      ---
      title: "Transformer Architecture"
      tags:
        - transformer
        - attention
      ---
      
      # Transformer Architecture
      
      The transformer relies on self-attention mechanisms.
      """
    When I run "kos search transformer"
    Then the output contains "Found 1"
    And the output contains "Transformer Architecture"
    And the output contains "Snippet:"

  @us3
  Scenario: Search with type filter
    Given I import a file "article.md" with content:
      """
      ---
      title: "AI Article"
      ---
      
      # AI Article
      
      Content about artificial intelligence.
      """
    When I run "kos search AI --type Article"
    Then the output contains "Found 1"
    And the output contains "AI Article"

  @us3
  Scenario: Search with tag filter
    Given I import a file "tagged.md" with content:
      """
      ---
      title: "Tagged Content"
      tags:
        - machine-learning
        - deep-learning
      ---
      
      # Tagged Content
      
      Machine learning content.
      """
    When I run "kos search learning --tag machine-learning"
    Then the output contains "Found 1"
    And the output contains "Tagged Content"

  @us3
  Scenario: Search returns no results for unmatched query
    Given I import a file "test.md" with content:
      """
      # Rust Programming
      
      Content about Rust.
      """
    When I run "kos search quantum-computing"
    Then the output contains "No entities found."

  # US4: View Entity Details
  @us4
  Scenario: Get entity details shows all components
    Given I import a file "detail.md" with content:
      """
      ---
      title: "Detail Test"
      tags:
        - test
      ---
      
      # Detail Test
      
      Some content here.
      """
    When I extract the entity ID from the last import
    And I run "kos get <entity-id>"
    Then the output contains "Entity:"
    And the output contains "EntityType"
    And the output contains "Version: 1"
    And the output contains "Components:"
    And the output contains "Detail Test"

  @us4
  Scenario: Get entity shows relationships
    Given a directory with files:
      | filename  | content                        |
      | source.md | # Source\n\nSee [target](target.md). |
      | target.md | # Target                      |
    When I run "kos import <directory>"
    And I extract the entity ID for "Source"
    And I run "kos get <entity-id>"
    Then the output contains "Relationships (outgoing):"

  # US5: List Entities by Type
  @us5
  Scenario: List all entities
    Given I import files:
      | filename | content       |
      | a.md     | # Doc A       |
      | b.md     | # Doc B       |
    When I run "kos list"
    Then the output contains "Found 2"

  @us5
  Scenario: List entities filtered by type
    Given I import files:
      | filename | content               |
      | a.md     | # Article One         |
      | b.md     | # Article Two         |
    When I run "kos list --type Article"
    Then the output contains "Found 2"

  # Entity Lifecycle
  @lifecycle
  Scenario: Archive entity removes from search
    Given I import a file "archive-me.md" with content:
      """
      ---
      title: "Archive Test"
      ---
      
      # Archive Test
      
      Content to archive.
      """
    When I extract the entity ID from the last import
    And I run "kos archive <entity-id>"
    Then the output contains "Archived"
    When I run "kos search Archive"
    Then the output contains "No entities found."

  @lifecycle
  Scenario: Restore entity makes it searchable again
    Given I import a file "restore-me.md" with content:
      """
      ---
      title: "Restore Test"
      ---
      
      # Restore Test
      
      Content to restore.
      """
    When I extract the entity ID from the last import
    And I run "kos archive <entity-id>"
    And I run "kos restore <entity-id>"
    And I run "kos search Restore"
    Then the output contains "Found 1"

  # Search Index
  @index
  Scenario: Rebuild search index from canonical data
    Given I import a file "rebuild.md" with content:
      """
      # Rebuild Test
      
      Content for rebuild.
      """
    When I run "kos rebuild-index"
    Then the output contains "Rebuilt index:"
    When I run "kos search rebuild"
    Then the output contains "Found 1"
