# Common Rules for Knowledge OS Agents

Apply these rules to every output. Non-negotiable.

---

## Writing Rules

- **Write as affirmation.** "X is..." not "X should be..." or "we think..."
- **Be specific.** Include trait signatures, file paths, SQL patterns, type definitions where they clarify the point.
- **No speculative language.** Never use "might," "could," "should consider." Write definitive statements.
- **Follow existing patterns.** Read neighboring code to match error handling (`thiserror`, `?` propagation, no `unwrap()`), naming (`snake_case` functions, `PascalCase` types, `SCREAMING_SNAKE_CASE` constants), module structure, and imports (`std`, external crates, internal crates).
- **Reference evidence.** Link to architecture docs, ADR decisions, PRD requirements, landscape research. Never assert without a source.
- **No redundant text.** Each concept appears once. Do not restate the same idea in different words.

---

## Self-Correction Protocol

When a verification step fails or you discover an error:

1. **Capture the exact error.** Copy the full output. Do not summarize.
2. **Classify the failure:**
   - **Environmental:** File not found, dependency missing, permission denied → fix the environment
   - **Cognitive:** Wrong type, incorrect API, logic error → re-read the source material
   - **Contextual:** Lost track of earlier work, repeating a failed approach → re-read the relevant files
   - **Structural:** Compilation error, type mismatch → read the compiler message; it tells you exactly what is wrong
   - **Semantic:** Code compiles but produces wrong behavior → write a failing test capturing the bug, then fix it
3. **Apply a targeted fix.** Each retry must be meaningfully different from the previous one.
4. **Maximum 3 attempts.** After 3 failures, stop and report the blocker with full context so the user can resolve it.

---

## Verification Rules

- **Verify before claim.** Never state "tests pass" without the command output showing it.
- **Never weaken existing tests.** If a test fails because of your change, fix your code — not the test.
- **Read before write.** Always read existing code before modifying it. Understand the current state before introducing changes.
