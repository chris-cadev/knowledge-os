@prd-0002 @pdf-import
Feature: PDF Import

  As a researcher
  I want to import PDF papers into Knowledge OS
  So that they become typed entities alongside my Markdown notes

  Background:
    Given an empty database

  # NOTE: These scenarios test error handling for invalid PDF files.
  # Real PDF import requires actual PDF files (not creatable in test env).
  # To test full PDF functionality, use real PDF files in integration tests.

  # US1: Import a PDF Paper
  @us1
  Scenario: Import an invalid PDF file reports error
    Given a file "attention.pdf" with content:
      """
      # Not a real PDF
      """
    When I run "kos import attention.pdf"
    Then the output contains "Errors: 1"

  @us1
  Scenario: Import invalid PDF without metadata reports error
    Given a file "no-meta.pdf" with content:
      """
      # Not a real PDF
      """
    When I run "kos import no-meta.pdf"
    Then the output contains "Errors: 1"

  @us1
  Scenario: Import invalid scanned PDF reports error
    Given a file "scanned.pdf" with content:
      """
      # Not a real PDF
      """
    When I run "kos import scanned.pdf"
    Then the output contains "Errors: 1"

  # US2: Import Directory with Mixed Formats
  @us2
  Scenario: Import directory with Markdown and invalid PDF files
    Given a directory with files:
      | filename    | content           |
      | notes.md    | # Notes           |
      | paper.pdf   | Not a real PDF    |
    When I run "kos import <directory>"
    Then the output contains "Total files: 2"
    And the output contains "Created: 1"
    And the output contains "Errors: 1"

  @us2
  Scenario: Import mixed directory with progress bar
    Given a directory with 5 files:
      | type | count |
      | md   | 3     |
      | pdf  | 2     |
    When I run "kos import <directory>"
    Then the output contains "Total files: 5"
    And the output contains "Created: 3"
    And the output contains "Errors: 2"

  # PDF Metadata Extraction
  @metadata
  Scenario: Import invalid PDF reports parse error
    Given a file "titled.pdf" with content:
      """
      # Not a real PDF
      """
    When I run "kos import titled.pdf"
    Then the output contains "Errors: 1"

  @metadata
  Scenario: Import invalid PDF with author metadata reports error
    Given a file "authored.pdf" with content:
      """
      # Not a real PDF
      """
    When I run "kos import authored.pdf"
    Then the output contains "Errors: 1"

  @metadata
  Scenario: Import invalid PDF with date metadata reports error
    Given a file "dated.pdf" with content:
      """
      # Not a real PDF
      """
    When I run "kos import dated.pdf"
    Then the output contains "Errors: 1"
