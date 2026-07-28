@prd-0003 @e2e @collection-navigation
Feature: End-to-End Collection Navigation
  As a knowledge worker
  I want to organize entities into collections and view them in the tree
  So that I can group related knowledge items together

  Background:
    Given an empty database

  Scenario: Create collection, add entities, view in tree
    Given I import a file "paper1.md" with content:
      """
      # Machine Learning Paper

      This paper covers advanced ML topics.
      """
    And I import a file "paper2.md" with content:
      """
      # Deep Learning Survey

      A survey of deep learning methods.
      """
    When I extract the entity ID for "Machine Learning Paper"
    And I run "collection create ResearchPapers"
    Then the output contains "Collection created:"
    When I extract the collection ID from the last output
    And I add the entity to the collection
    Then the output contains "Entity added to collection."
    When I extract the entity ID for "Deep Learning Survey"
    And I add the entity to the collection
    Then the output contains "Entity added to collection."
    When I run "kos view tree"
    Then the output contains "ResearchPapers"
    And the output contains "Machine Learning Paper"
    And the output contains "Deep Learning Survey"

  Scenario: Collection with members listed
    Given I import a file "paper.md" with content:
      """
      # Reading Item

      Important reading material.
      """
    When I extract the entity ID from the last import
    And I run "collection create ReadingList"
    Then the output contains "Collection created:"
    When I extract the collection ID from the last output
    And I add the entity to the collection
    Then the output contains "Entity added to collection."
    When I show the collection members
    Then the output contains "Reading Item"

  Scenario: Collection appears in tree after creation
    Given I import a file "note.md" with content:
      """
      # Quick Note

      A quick note for later.
      """
    When I extract the entity ID from the last import
    And I run "collection create QuickNotes"
    Then the output contains "Collection created:"
    When I extract the collection ID from the last output
    And I add the entity to the collection
    Then the output contains "Entity added to collection."
    When I run "kos view tree"
    Then the output contains "QuickNotes"

  Scenario: Multiple collections in tree
    Given I import a file "paper.md" with content:
      """
      # Multi Collection Paper

      A paper for multiple collections.
      """
    When I extract the entity ID from the last import
    And I run "collection create CollectionA"
    Then the output contains "Collection created:"
    When I extract the collection ID from the last output
    And I add the entity to the collection
    Then the output contains "Entity added to collection."
    When I run "collection create CollectionB"
    Then the output contains "Collection created:"
    When I extract the collection ID from the last output
    And I add the entity to the collection
    Then the output contains "Entity added to collection."
    When I run "kos view tree"
    Then the output contains "CollectionA"
    And the output contains "CollectionB"
