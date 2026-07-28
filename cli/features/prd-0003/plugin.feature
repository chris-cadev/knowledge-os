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

  @us5 @install
  Scenario: Install a plugin from a directory
    Given a plugin directory "my-importer" with manifest:
      """
      [plugin]
      name = "my-importer"
      version = "0.1.0"
      description = "Custom importer plugin"
      author = "Test"
      """
    When I run with plugin dir "my-importer": "kos plugin install <directory>/plugins/my-importer"
    Then the output contains "Plugin 'my-importer' v0.1.0 installed"

  @us5 @install
  Scenario: Install a plugin without plugin.toml fails
    Given an empty file "empty-dir-marker"
    When I run with plugin dir "empty-plugin": "kos plugin install <directory>"
    Then the error output contains "No plugin.toml found"

  @us5 @uninstall
  Scenario: Uninstall an installed plugin
    Given a plugin directory "removable-plugin" with manifest:
      """
      [plugin]
      name = "removable-plugin"
      version = "0.2.0"
      description = "Plugin to remove"
      author = "Test"
      """
    When I run with plugin dir "removable-plugin": "kos plugin install <directory>/plugins/removable-plugin"
    Then the output contains "Plugin 'removable-plugin' v0.2.0 installed"
    When I run with plugin dir "removable-plugin": "kos plugin uninstall removable-plugin"
    Then the output contains "Plugin 'removable-plugin v0.2.0' uninstalled"

  @us5 @uninstall
  Scenario: Uninstall a nonexistent plugin fails
    When I run "kos plugin uninstall nonexistent-plugin"
    Then the error output contains "not installed"

  @us5 @install
  Scenario: Install duplicate plugin fails
    Given a plugin directory "dup-plugin" with manifest:
      """
      [plugin]
      name = "dup-plugin"
      version = "0.1.0"
      description = "Duplicate plugin"
      author = "Test"
      """
    When I run with plugin dir "dup-plugin": "kos plugin install <directory>/plugins/dup-plugin"
    Then the output contains "Plugin 'dup-plugin' v0.1.0 installed"
    When I run with plugin dir "dup-plugin": "kos plugin install <directory>/plugins/dup-plugin"
    Then the error output contains "already installed"
