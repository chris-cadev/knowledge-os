@prd-0007 @html-import
Feature: HTML Import

  As a knowledge worker
  I want to import HTML files into Knowledge OS
  So that web pages and saved HTML documents become typed entities with components

  Background:
    Given an empty database

  @us1
  Scenario: Import a single HTML file
    Given a file "page.html" with content:
      """
      <!DOCTYPE html>
      <html lang="en">
      <head><title>My Web Page</title></head>
      <body><p>Hello world.</p></body>
      </html>
      """
    When I run "kos import page.html"
    Then the output contains "Created:"
    And the output contains "My Web Page"

  @us1
  Scenario: Import extracts title from HTML title tag
    Given a file "article.html" with content:
      """
      <html>
      <head><title>Understanding Rust</title></head>
      <body><p>Content about Rust.</p></body>
      </html>
      """
    When I run "kos import article.html"
    Then the output contains "Created:"
    And the output contains "Understanding Rust"

  @us1
  Scenario: Import falls back to h1 when no title tag
    Given a file "no-title.html" with content:
      """
      <html>
      <body><h1>Heading Title</h1><p>Some content.</p></body>
      </html>
      """
    When I run "kos import no-title.html"
    Then the output contains "Created:"
    And the output contains "Heading Title"

  @us1
  Scenario: Import falls back to filename when no title or h1
    Given a file "my-page.html" with content:
      """
      <html><body><p>No title anywhere.</p></body></html>
      """
    When I run "kos import my-page.html"
    Then the output contains "Created:"
    And the output contains "my-page"

  @metadata
  Scenario: Import extracts author from meta tag
    Given a file "authored.html" with content:
      """
      <html>
      <head>
        <title>Authored Page</title>
        <meta name="author" content="Jane Doe">
      </head>
      <body><p>Content.</p></body>
      </html>
      """
    When I run "kos import authored.html"
    Then the output contains "Created:"
    And the output contains "Authored Page"

  @metadata
  Scenario: Import extracts keywords as tags
    Given a file "tagged.html" with content:
      """
      <html>
      <head>
        <title>Tagged Page</title>
        <meta name="keywords" content="rust, web, knowledge">
      </head>
      <body><p>Content.</p></body>
      </html>
      """
    When I run "kos import tagged.html"
    Then the output contains "Created:"
    And the output contains "rust"

  @metadata
  Scenario: Import extracts language from html lang attribute
    Given a file "french.html" with content:
      """
      <html lang="fr">
      <head><title>Page Francaise</title></head>
      <body><p>Contenu ici.</p></body>
      </html>
      """
    When I run "kos import french.html"
    Then the output contains "Created:"
    And the output contains "Page Francaise"

  @links
  Scenario: Import extracts links as cross-references
    Given a file "links.html" with content:
      """
      <html>
      <head><title>Links Page</title></head>
      <body>
        <a href="https://example.com">Example</a>
        <a href="other.html">Other Page</a>
      </body>
      </html>
      """
    When I run "kos import links.html"
    Then the output contains "Created:"
    And the output contains "Links Page"

  @htmx
  Scenario: Import htm extension files
    Given a file "legacy.htm" with content:
      """
      <html>
      <head><title>Legacy Page</title></head>
      <body><p>Old school HTML.</p></body>
      </html>
      """
    When I run "kos import legacy.htm"
    Then the output contains "Created:"
    And the output contains "Legacy Page"

  @batch
  Scenario: Import directory with mixed HTML and Markdown files
    Given a directory with files:
      | filename     | content                                          |
      | notes.md     | # Notes                                          |
      | page.html    | <html><head><title>Page</title></head><body></body></html> |
    When I run "kos import <directory>"
    Then the output contains "Total files: 2"
    And the output contains "Created: 2"
