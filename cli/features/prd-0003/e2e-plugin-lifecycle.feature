@prd-0003 @e2e @plugin-lifecycle
Feature: End-to-End Plugin Lifecycle
  As a knowledge worker
  I want to see which plugins are loaded and their capabilities
  So that I understand what import formats and features are available

  Background:
    Given an empty database

  Scenario: List plugins then get info
    When I run "kos plugin list"
    Then the output contains "Plugins"
    And the output contains "markdown-importer"
    When I run "kos plugin info markdown-importer"
    Then the output contains "markdown-importer"
    And the output contains "0.1.0"
    And the output contains "Import Markdown files"

  Scenario: Plugin info for all registered importers
    When I run "kos plugin info pdf-importer"
    Then the output contains "pdf-importer"
    And the output contains "0.1.0"
    When I run "kos plugin info url-importer"
    Then the output contains "url-importer"
    And the output contains "0.1.0"

  Scenario: Plugin info for nonexistent plugin shows error
    When I run "kos plugin info nonexistent-plugin"
    Then the error output contains "not found"

  Scenario: Import uses markdown-importer plugin
    Given I import a file "test.md" with content:
      """
      ---
      title: "Plugin Test"
      ---
      # Plugin Test

      Imported via markdown-importer.
      """
    Then the output contains "Created:"
    And the output contains "Article"
