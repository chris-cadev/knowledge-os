@prd-0001 @entity-management
Feature: Entity Management

  As a knowledge worker
  I want to manage entities in my knowledge graph
  So that I can maintain an organized knowledge base

  Background:
    Given an empty database

  # Entity CRUD
  @crud
  Scenario: Create entity with type and components
    Given a file "new-entity.md" with content:
      """
      ---
      title: "New Entity"
      tags:
        - test
      ---
      
      # New Entity
      
      Content here.
      """
    When I run "kos import new-entity.md"
    Then the output contains "Created:"
    And the output contains "EntityType"
    And the output contains "New Entity"

  @crud
  Scenario: Update entity increments version
    Given a file "version-test.md" with content:
      """
      ---
      title: "Version Test"
      ---
      
      # Version Test
      
      Version 1.
      """
    When I run "kos import version-test.md"
    And I extract the entity ID from the last import
    And I run "kos get <entity-id>"
    Then the output contains "Version: 1"
    Given the file "version-test.md" is updated with content:
      """
      ---
      title: "Version Test"
      ---
      
      # Version Test
      
      Version 2 with more content.
      """
    When I run "kos import version-test.md"
    And I run "kos get <entity-id>"
    Then the output contains "Version: 2"

  @crud
  Scenario: Archive entity preserves history
    Given a file "preserve.md" with content:
      """
      # Preserve Test
      
      Content to preserve.
      """
    When I run "kos import preserve.md"
    And I extract the entity ID from the last import
    And I run "kos archive <entity-id>"
    Then the output contains "Archived"
    When I run "kos get <entity-id>"
    Then the output contains "Entity:"
    And the output contains "Active: false"

  @crud
  Scenario: Restore archived entity
    Given a file "restore.md" with content:
      """
      # Restore Test
      
      Content to restore.
      """
    When I run "kos import restore.md"
    And I extract the entity ID from the last import
    And I run "kos archive <entity-id>"
    And I run "kos restore <entity-id>"
    Then the output contains "Restored"
    When I run "kos get <entity-id>"
    Then the output contains "Active: true"

  # Version History
  @versioning
  Scenario: Entity version history is queryable
    Given a file "history.md" with content:
      """
      # History Test
      
      Version 1.
      """
    When I run "kos import history.md"
    And I extract the entity ID from the last import
    Given the file "history.md" is updated with content:
      """
      # History Test
      
      Version 2.
      """
    When I run "kos import history.md"
    And I run "kos get <entity-id>"
    Then the output contains "Version: 2"
    And the output contains "Events:"

  # Relationship Management
  @relationships
  Scenario: Create typed relationships between entities
    Given a directory with files:
      | filename  | content                        |
      | source.md | # Source\n\nSee [target](target.md). |
      | target.md | # Target                      |
    When I run "kos import <directory>"
    Then the output contains "Created: 2"

  # Querying
  @query
  Scenario: Query entities by tag
    Given I import a file "tag-query.md" with content:
      """
      ---
      title: "Tag Query"
      tags:
        - rust
        - programming
      ---
      
      # Tag Query
      
      Rust content.
      """
    When I run "kos search rust --tag rust"
    Then the output contains "Found 1"
    And the output contains "Tag Query"
