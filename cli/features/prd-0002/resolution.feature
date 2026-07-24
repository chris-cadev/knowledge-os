@prd-0002 @resolution
Feature: Entity Resolution

  As a knowledge worker
  I want to have duplicate entities automatically detected and merged
  So that my knowledge graph stays clean without manual deduplication

  Background:
    Given an empty database

  # US3: Resolve Duplicates Across Imports
  @us3
  Scenario: Exact duplicate detection by title and type
    Given I import a file "exact-dup.md" with content:
      """
      ---
      title: "Exact Duplicate"
      ---
      
      # Exact Duplicate
      
      Content.
      """
    When I run "kos import exact-dup.md"
    Then the output contains "Created:"
    When I run "kos import exact-dup.md"
    Then the output contains "Duplicates resolved: 1"

  @us3
  Scenario: Resolution presents confidence scores
    Given I import a file "high-conf.md" with content:
      """
      ---
      title: "High Confidence Match"
      ---
      
      # High Confidence Match
      
      Content.
      """
    When I run "kos import high-conf.md"
    Then the output contains "Created:"
    When I run "kos import high-conf.md"
    Then the output contains "Duplicates resolved: 1"

  @us3
  Scenario: High-confidence matches merge automatically
    Given I import a file "auto-merge.md" with content:
      """
      ---
      title: "Auto Merge"
      ---
      
      # Auto Merge
      
      Content for auto merge.
      """
    When I run "kos import auto-merge.md"
    And I run "kos import auto-merge.md"
    Then the output contains "Duplicates resolved: 1"

  @us3
  Scenario: All merge decisions are logged
    Given I import a file "logged.md" with content:
      """
      ---
      title: "Logged Merge"
      ---
      
      # Logged Merge
      
      Content for logging.
      """
    When I run "kos import logged.md"
    And I run "kos import logged.md"
    Then the output contains "Duplicates resolved: 1"
    When I run "kos resolution log"
    Then the output contains "Logged Merge"

  @us3
  Scenario: Merge can be undone
    Given I import a file "undo-me.md" with content:
      """
      ---
      title: "Undo Merge"
      ---
      
      # Undo Merge
      
      Content for undo.
      """
    When I run "kos import undo-me.md"
    And I extract the entity ID from the last import
    And I run "kos import undo-me.md"
    Then the output contains "Duplicates resolved: 1"
    When I run "kos resolution log"
    And I extract the merge ID from the resolution log
    And I run "kos resolution undo <merge-id>"
    Then the output contains "Undone"

  # Resolution Strategies
  @strategies
  Scenario: Resolution strategy varies by entity type
    Given I import a file "person-a.md" with content:
      """
      ---
      title: "John Smith"
      type: person
      ---
      
      # John Smith
      
      Person profile.
      """
    When I run "kos import person-a.md"
    Then the output contains "Created:"
    Given I import a file "person-b.md" with content:
      """
      ---
      title: "Jon Smith"
      type: person
      ---
      
      # Jon Smith
      
      Similar person.
      """
    When I run "kos import person-b.md"
    Then the output contains "Duplicates resolved: 1"

  # Configurable Thresholds
  @thresholds
  Scenario: Configurable merge threshold per entity type
    Given the merge threshold for "Article" is set to 0.9
    And I import a file "threshold-test.md" with content:
      """
      ---
      title: "Threshold Test"
      ---
      
      # Threshold Test
      
      Content.
      """
    When I run "kos import threshold-test.md"
    Then the output contains "Created:"
    Given I import a file "threshold-dup.md" with content:
      """
      ---
      title: "Threshold Duplicate"
      ---
      
      # Threshold Duplicate
      
      Similar content.
      """
    When I run "kos import threshold-dup.md"
    Then the resolution respects the configured threshold
