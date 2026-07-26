@prd-0003 @plugin
Feature: Plugin Management
  As a user
  I want to see loaded plugins
  So that I know what capabilities are available

  Background:
    Given an empty database

  @us5
  Scenario: List plugins
    When I run "kos plugin list"
    Then the output contains "Plugins"
    And the output contains "markdown-importer"
    And the output contains "pdf-importer"
    And the output contains "url-importer"

  @us5
  Scenario: Plugin info for markdown-importer
    When I run "kos plugin info markdown-importer"
    Then the output contains "markdown-importer"
    And the output contains "0.1.0"
    And the output contains "Import Markdown files"

  @us5
  Scenario: Plugin info for pdf-importer
    When I run "kos plugin info pdf-importer"
    Then the output contains "pdf-importer"
    And the output contains "0.1.0"

  @us5
  Scenario: Plugin info for url-importer
    When I run "kos plugin info url-importer"
    Then the output contains "url-importer"
    And the output contains "0.1.0"

  @us5
  Scenario: Plugin info for unknown plugin
    When I run "kos plugin info nonexistent"
    Then the error output contains "not found"
