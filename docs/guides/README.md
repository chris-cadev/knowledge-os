# Guides

[Home](../../README.md) > [Documentation](../README.md) > Guides

How-to guides, tutorials, and AI agent workflows. These documents are task-oriented -- they answer "how do I?" rather than "what is?"

---

## Documents

| Document                                         | Purpose                                                                                                |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| [plugin-development.md](plugin-development.md)   | How to build plugins -- plugin types, manifest format, development workflow, testing, and distribution |
| [ai-agent-guidelines.md](ai-agent-guidelines.md) | How AI agents work -- agent principles, agent types, context construction rules, and safety boundaries |
| [bdd-testing.md](bdd-testing.md)                 | How to write and maintain BDD tests -- step definitions, feature file conventions, debugging failures  |

## Tutorials

Step-by-step learning experiences. Each tutorial walks through a complete workflow from start to finish.

| Tutorial                                                                 | Purpose                                                                                                                    |
| ------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| [tutorials/first-import.md](tutorials/first-import.md)                   | Import your first document -- create a sample file, run the import command, and trace it through the pipeline              |
| [tutorials/build-custom-importer.md](tutorials/build-custom-importer.md) | Build a custom importer plugin -- scaffold a plugin crate, implement the adapter trait, parse a custom format, and test it |

---

## Reading Order

New plugin developers should start with [plugin-development.md](plugin-development.md), then follow [tutorials/build-custom-importer.md](tutorials/build-custom-importer.md) for a hands-on walkthrough. AI integration begins with [ai-agent-guidelines.md](ai-agent-guidelines.md). BDD testing is covered in [bdd-testing.md](bdd-testing.md). The first-import tutorial is the quickest way to see the system in action.
