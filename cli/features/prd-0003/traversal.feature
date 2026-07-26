@prd-0003 @traversal
Feature: Graph Traversal
  As a knowledge worker
  I want to explore connections between entities
  So that I can understand how concepts relate to each other

  Background:
    Given an empty database

  @us1
  Scenario: Basic outgoing traversal
    Given a directory with files:
      | filename    | content                                       |
      | entity-a.md | # Entity A\n\nReferences [[Entity B]].        |
      | entity-b.md | # Entity B\n\nReferences [[Entity C]].        |
      | entity-c.md | # Entity C                                    |
    When I run "kos import <directory>"
    And I extract the entity ID for "Entity A"
    And I run "kos traverse <entity-id> --depth 2"
    Then the output contains "Hop 1"
    And the output contains "Hop 2"
    And the output contains "Entity B"
    And the output contains "Entity C"
    And the output contains "Total:"

  @us1
  Scenario: Bidirectional traversal
    Given a directory with files:
      | filename    | content                                       |
      | entity-a.md | # Entity A\n\nReferences [[Entity B]].        |
      | entity-b.md | # Entity B                                    |
    When I run "kos import <directory>"
    And I extract the entity ID for "Entity A"
    And I run "kos traverse <entity-id> --depth 1 --direction both"
    Then the output contains "Entity B"
    And the output contains "Total:"

  @us1
  Scenario: Depth limiting
    Given a directory with files:
      | filename    | content                                       |
      | entity-a.md | # Entity A\n\nReferences [[Entity B]].        |
      | entity-b.md | # Entity B\n\nReferences [[Entity C]].        |
      | entity-c.md | # Entity C\n\nReferences [[Entity D]].        |
      | entity-d.md | # Entity D                                    |
    When I run "kos import <directory>"
    And I extract the entity ID for "Entity A"
    And I run "kos traverse <entity-id> --depth 1"
    Then the output contains "Hop 1"
    And the output contains "Entity B"
    And the output does not contain "Entity C"

  @us1
  Scenario: Nonexistent entity error
    When I run "kos traverse 00000000-0000-0000-0000-000000000000 --depth 2"
    Then the error output contains "StartNotFound"
