@prd-0001 @import
Feature: Markdown Import Pipeline

  As a knowledge worker
  I want to import Markdown files into Knowledge OS
  So that they become typed entities with components

  Background:
    Given an empty database

  # US1: Import a Markdown File
  @us1
  Scenario: Import a single Markdown file
    Given a file "notes.md" with content:
      """
      ---
      title: "My Notes"
      tags:
        - rust
        - learning
      ---
      
      # My Notes
      
      These are my learning notes about Rust.
      """
    When I run "kos import notes.md"
    Then the output contains "Created:"
    And the output contains "EntityType"
    And the output contains "My Notes"
    And the output contains "rust"
    And the output contains "learning"

  @us1
  Scenario: Import extracts title from YAML frontmatter
    Given a file "paper.md" with content:
      """
      ---
      title: "Attention Is All You Need"
      ---
      
      # Body
      
      Content here.
      """
    When I run "kos import paper.md"
    Then the output contains "Created:"
    And the output contains "Attention Is All You Need"

  @us1
  Scenario: Import extracts title from first H1 heading
    Given a file "article.md" with content:
      """
      # Learning to Code
      
      Some content without frontmatter.
      """
    When I run "kos import article.md"
    Then the output contains "Created:"
    And the output contains "Learning to Code"

  @us1
  Scenario: Import uses filename as title fallback
    Given a file "my-document.md" with content:
      """
      Content without any heading or frontmatter.
      """
    When I run "kos import my-document.md"
    Then the output contains "Created:"
    And the output contains "my-document"

  @us1
  Scenario: Import extracts tags from YAML frontmatter
    Given a file "tagged.md" with content:
      """
      ---
      tags:
        - machine-learning
        - transformer
        - attention
      ---
      
      # Tagged Content
      
      Body text here.
      """
    When I run "kos import tagged.md"
    Then the output contains "Created:"
    And the output contains "machine-learning"
    And the output contains "transformer"

  @us1
  Scenario: Import stores provenance metadata
    Given a file "provenance.md" with content:
      """
      # Provenance Test
      
      Content.
      """
    When I run "kos import provenance.md"
    Then the output contains "Created:"

  @us1
  Scenario: Import checks for duplicates before storage
    Given a file "duplicate.md" with content:
      """
      ---
      title: "Same Title"
      ---
      
      # Same Title
      
      Original content.
      """
    When I run "kos import duplicate.md"
    Then the output contains "Created:"
    When I run "kos import duplicate.md"
    Then the output contains "Duplicates resolved: 1"

  # US2: Import a Directory of Markdown Files
  @us2
  Scenario: Import a directory of Markdown files
    Given a directory with files:
      | filename   | content          |
      | doc-a.md   | # Doc A          |
      | doc-b.md   | # Doc B          |
      | doc-c.md   | # Doc C          |
    When I run "kos import <directory>"
    Then the output contains "Total files: 3"
    And the output contains "Created: 3"

  @us2
  Scenario: Import ignores non-Markdown files
    Given a directory with files:
      | filename   | content          |
      | doc-a.md   | # Doc A          |
      | readme.txt | This is text     |
      | notes.md   | # Notes          |
    When I run "kos import <directory>"
    Then the output contains "Total files: 2"
    And the output contains "Created: 2"

  @us2
  Scenario: Import extracts cross-references between files
    Given a directory with files:
      | filename   | content                           |
      | source.md  | # Source\n\nSee [target](target.md). |
      | target.md  | # Target                          |
    When I run "kos import <directory>"
    Then the output contains "Created: 2"

  # US3: Idempotent Reimport
  @us3
  Scenario: Reimporting same file updates existing entity
    Given a file "update-me.md" with content:
      """
      ---
      title: "Update Test"
      ---
      
      # Update Test
      
      Version 1 content.
      """
    When I run "kos import update-me.md"
    Then the output contains "Created:"
    Given the file "update-me.md" is updated with content:
      """
      ---
      title: "Update Test"
      ---
      
      # Update Test
      
      Version 2 content with more information.
      """
    When I run "kos import update-me.md"
    Then the output contains "Duplicates resolved: 1"

  # US4: Import Error Handling
  @us4
  Scenario: Import handles non-existent file gracefully
    When I run "kos import nonexistent.md"
    Then the error output contains "ERROR"

  @us4
  Scenario: Import handles empty file gracefully
    Given an empty file "empty.md"
    When I run "kos import empty.md"
    Then the output contains "Created:"
