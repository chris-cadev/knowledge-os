Feature: Conversation CLI
  As a knowledge worker
  I want to manage AI conversations
  So that I can review and organize my chat history

  Background:
    Given an empty database

  Scenario: List empty
    When I run "conversation list"
    Then the output contains "No conversations found."

  Scenario: List shows recent first
    Given a conversation "Q3 review" with 5 messages
    And a conversation "Onboarding" with 2 messages
    When I run "conversation list"
    Then the output contains "Q3 review"

  Scenario: Rename conversation
    Given a conversation "Old name" with 1 message
    When I extract the conversation ID from the last output
    And I run "conversation rename <id> New name"
    Then the output contains "renamed"

  Scenario: Delete conversation archives it
    Given a conversation "Test" with 3 messages
    When I extract the conversation ID from the last output
    And I run "conversation delete <id>"
    Then the output contains "archived"
