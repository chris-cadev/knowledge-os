@prd-0003 @views
Feature: View Projections
  As a knowledge worker
  I want to view my knowledge in different projections
  So that I can navigate and compare entities effectively

  Background:
    Given an empty database

  @us2
  Scenario: Tree view groups entities by type
    Given I import a file "concept.md" with content:
      """
      ---
      title: "Transformer"
      type: concept
      ---
      # Transformer
      """
    And I import a file "paper.md" with content:
      """
      ---
      title: "Attention Is All You Need"
      type: paper
      ---
      # Attention Is All You Need
      """
    When I run "kos view tree"
    Then the output contains "Concept"
    And the output contains "Paper"
    And the output contains "Transformer"
    And the output contains "Attention Is All You Need"

  @us2
  Scenario: Tree view with entity type filter
    Given I import a file "concept.md" with content:
      """
      ---
      title: "Transformer"
      type: concept
      ---
      # Transformer
      """
    And I import a file "paper.md" with content:
      """
      ---
      title: "Attention Is All You Need"
      type: paper
      ---
      # Attention Is All You Need
      """
    When I run "kos view tree --type Concept"
    Then the output contains "Concept"
    And the output does not contain "Paper"

  @us3
  Scenario: Graph view displays nodes and edges
    Given a directory with files:
      | filename    | content                                         |
      | entity-a.md | # Entity A\n\nReferences [[Entity B]].          |
      | entity-b.md | # Entity B                                      |
    When I run "kos import <directory>"
    And I run "kos view graph"
    Then the output contains "Entity A"
    And the output contains "Entity B"

  @us3
  Scenario: Graph view with depth limit
    Given a directory with files:
      | filename    | content                                         |
      | entity-a.md | # Entity A\n\nReferences [[Entity B]].          |
      | entity-b.md | # Entity B\n\nReferences [[Entity C]].          |
      | entity-c.md | # Entity C                                      |
    When I run "kos import <directory>"
    And I extract the entity ID for "Entity A"
    And I run "kos view graph --from <entity-id> --depth 1"
    Then the output contains "Entity A"
    And the output contains "Entity B"
    And the output does not contain "Entity C"

  @us4
  Scenario: Table view displays entities
    Given a directory with files:
      | filename    | content                                    |
      | concept.md  | # Transformer\n\nType: concept             |
    When I run "kos import <directory>"
    And I run "kos view table"
    Then the output contains "Transformer"
    And the output contains "Type"

  @us4
  Scenario: Table view with sort
    Given a directory with files:
      | filename    | content                                    |
      | alpha.md    | # Alpha\n\nType: concept                   |
      | beta.md     | # Beta\n\nType: concept                    |
    When I run "kos import <directory>"
    And I run "kos view table --sort Title"
    Then the output contains "Alpha"
    And the output contains "Beta"

  Scenario: Timeline view orders by creation time
    Given a directory with files:
      | filename    | content                                    |
      | concept.md  | # Transformer\n\nType: concept             |
    When I run "kos import <directory>"
    And I run "kos view timeline"
    Then the output contains "Transformer"

  Scenario: View with empty database shows no entities
    When I run "kos view tree"
    Then the output contains "No entities found."
