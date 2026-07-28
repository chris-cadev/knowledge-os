@prd-0003 @e2e @traversal-views
Feature: End-to-End Traversal and Views
  As a knowledge worker
  I want to traverse entity relationships and view the results in different projections
  So that I can explore and understand my knowledge graph

  Background:
    Given an empty database

  Scenario: Traverse then view results as graph
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
    When I run "kos view graph"
    Then the output contains "Entity A"
    And the output contains "Entity B"
    And the output contains "Entity C"

  Scenario: Traverse then view as table
    Given a directory with files:
      | filename    | content                                       |
      | concept.md  | # Transformer\n\nType: concept                |
      | paper.md    | # Attention Is All You Need\n\nType: paper    |
    When I run "kos import <directory>"
    And I run "kos view table"
    Then the output contains "Transformer"
    And the output contains "Attention Is All You Need"
    And the output contains "Type"

  Scenario: Traverse with depth limit then view tree
    Given a directory with files:
      | filename    | content                                       |
      | entity-a.md | # Entity A\n\nReferences [[Entity B]].        |
      | entity-b.md | # Entity B\n\nReferences [[Entity C]].        |
      | entity-c.md | # Entity C                                    |
    When I run "kos import <directory>"
    And I extract the entity ID for "Entity A"
    And I run "kos traverse <entity-id> --depth 1"
    Then the output contains "Hop 1"
    And the output contains "Entity B"
    And the output does not contain "Entity C"
    When I run "kos view tree"
    Then the output contains "Entity A"
    And the output contains "Entity B"
    And the output contains "Entity C"

  Scenario: Graph view with start entity matches traversal
    Given a directory with files:
      | filename    | content                                       |
      | entity-a.md | # Entity A\n\nReferences [[Entity B]].        |
      | entity-b.md | # Entity B                                    |
    When I run "kos import <directory>"
    And I extract the entity ID for "Entity A"
    And I run "kos traverse <entity-id> --depth 1"
    Then the output contains "Entity B"
    And the output contains "Total:"
    When I run "kos view graph --from <entity-id> --depth 1"
    Then the output contains "Entity A"
    And the output contains "Entity B"

  Scenario: Timeline view after import
    Given a directory with files:
      | filename    | content                                       |
      | concept.md  | # Transformer\n\nType: concept                |
    When I run "kos import <directory>"
    And I run "kos view timeline"
    Then the output contains "Transformer"
