Feature: Collection Management
  As a knowledge worker
  I want to organize entities into named collections
  So that I can group related knowledge items together

  Background:
    Given an empty database

  Scenario: Create a collection with description
    When I run "collection create Papers_to_Read --description Research-papers-for-literature-review"
    Then the output contains "Collection created:"
    And the output contains "Papers_to_Read"
    When I run "collection list"
    Then the output contains "Papers_to_Read"
    And the output contains "Research-papers-for-literature-review"

  Scenario: Create a collection without description
    When I run "collection create Ideas"
    Then the output contains "Collection created:"
    And the output contains "Ideas"
    When I run "collection list"
    Then the output contains "Ideas"
    And the output contains "(no description)"

  Scenario: List collections shows count
    When I run "collection create First"
    Then the output contains "Collection created:"
    When I run "collection create Second"
    Then the output contains "Collection created:"
    When I run "collection list"
    Then the output contains "Collections (2):"

  Scenario: List collections when empty
    When I run "collection list"
    Then the output contains "No collections found."

  Scenario: Add entity to collection
    Given I import a file "paper.md" with content:
      """
      # Machine Learning Paper

      This paper covers advanced topics.
      """
    When I extract the entity ID from the last import
    And I run "collection create MLPapers"
    Then the output contains "Collection created:"
    When I extract the collection ID from the last output
    And I add the entity to the collection
    Then the output contains "Entity added to collection."

  Scenario: Duplicate member is rejected
    Given I import a file "paper.md" with content:
      """
      # Duplicate Test

      Testing duplicate membership.
      """
    When I extract the entity ID from the last import
    And I run "collection create TestCollection"
    Then the output contains "Collection created:"
    When I extract the collection ID from the last output
    And I add the entity to the collection
    Then the output contains "Entity added to collection."
    When I add the entity to the collection again
    Then the error output contains "already a member"

  Scenario: Remove entity from collection
    Given I import a file "paper.md" with content:
      """
      # Remove Test

      Testing member removal.
      """
    When I extract the entity ID from the last import
    And I run "collection create RemoveTest"
    Then the output contains "Collection created:"
    When I extract the collection ID from the last output
    And I add the entity to the collection
    Then the output contains "Entity added to collection."
    When I remove the entity from the collection
    Then the output contains "Entity removed from collection."

  Scenario: Collection members listed
    Given I import a file "paper1.md" with content:
      """
      # First Paper

      Paper one content.
      """
    And I import a file "paper2.md" with content:
      """
      # Second Paper

      Paper two content.
      """
    When I run "collection create ReadingList"
    Then the output contains "Collection created:"
    When I extract the collection ID from the last output
    And I extract the entity ID for "First Paper"
    And I add the entity to the collection
    Then the output contains "Entity added to collection."
    When I extract the entity ID for "Second Paper"
    And I add the entity to the collection
    Then the output contains "Entity added to collection."
    When I show the collection members
    Then the output contains "First Paper"
    And the output contains "Second Paper"

  Scenario: Empty collection shows empty message
    When I run "collection create EmptyCollection"
    Then the output contains "Collection created:"
    When I extract the collection ID from the last output
    And I show the collection members
    Then the output contains "is empty."

  Scenario: Delete collection cascades members
    Given I import a file "paper.md" with content:
      """
      # Cascade Test

      Testing cascade delete.
      """
    When I extract the entity ID from the last import
    And I run "collection create DeleteTest"
    Then the output contains "Collection created:"
    When I extract the collection ID from the last output
    And I add the entity to the collection
    Then the output contains "Entity added to collection."
    When I run "collection delete <collection-id>"
    Then the output contains "Collection deleted:"
    When I run "collection list"
    Then the output contains "No collections found."

  Scenario: Add entity to nonexistent collection
    Given I import a file "paper.md" with content:
      """
      # Nonexistent Test

      Testing nonexistent collection.
      """
    When I extract the entity ID from the last import
    When I run "collection add 00000000-0000-0000-0000-000000000000 <entity-id>"
    Then the error output contains "Collection"
    And the error output contains "not found"

  Scenario: Entity appears in multiple collections
    Given I import a file "paper.md" with content:
      """
      # Multi-Collection Test

      Testing multiple collection membership.
      """
    When I extract the entity ID from the last import
    When I run "collection create CollectionA"
    Then the output contains "Collection created:"
    When I extract the collection ID from the last output
    And I add the entity to the collection
    Then the output contains "Entity added to collection."
    When I run "collection create CollectionB"
    Then the output contains "Collection created:"
    When I extract the collection ID from the last output
    And I add the entity to the collection
    Then the output contains "Entity added to collection."
    When I run "collection list"
    Then the output contains "CollectionA"
    And the output contains "CollectionB"
