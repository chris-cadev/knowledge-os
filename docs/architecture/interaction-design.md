# Interaction Design

> Design interfaces that minimize cognitive load, maximize user confidence, and keep users focused on their goals rather than on the interface itself. Every design decision reduces mental effort, increases predictability, and makes progress feel effortless.

---

## Primary Goal

Knowledge OS interfaces minimize cognitive load, maximize user confidence, and keep users focused on their goals rather than on the interface itself. Every design decision reduces mental effort, increases predictability, and makes progress feel effortless.

This document defines the interaction design principles that govern all Knowledge OS projections. These principles apply across all interface types: CLI, TUI, web, desktop, and API. Every view, every command, every interaction must satisfy these principles.

---

## Core Principles

### 1. Goal-Oriented Design

Every screen makes the next action obvious. User goals take priority over exposing system capabilities. Dead ends and ambiguous navigation do not exist.

**Rules:**

- Every interface presents a clear primary action.
- The user's goal is always visible and reachable.
- System capabilities are revealed only when they serve the user's current goal.
- Navigation paths always lead somewhere useful.

### 2. Reduce Cognitive Load

The number of simultaneous decisions is minimized. Only information relevant to the current task is displayed. Recognition is preferred over recall. Progressive disclosure reveals complexity only when needed.

**Rules:**

- Show only what the user needs for the current task.
- Break complex workflows into sequential steps.
- Use visual hierarchy to guide attention.
- Provide context-sensitive help and hints.
- Default to the simplest useful view (see [Progressive Disclosure](ui-philosophy.md#progressive-disclosure)).

### 3. Predictability

User actions produce expected outcomes. Surprising behavior is eliminated. Similar components always behave the same way.

**Rules:**

- Consistent interaction patterns across all views.
- Standard keyboard shortcuts work everywhere (see [Keyboard Navigation](ui-philosophy.md#keyboard-navigation)).
- Visual affordances match behavior (buttons look clickable, links look clickable).
- State changes are visible and understandable.

### 4. Immediate Feedback

Every interaction acknowledges user input. Visual, auditory, or haptic feedback is provided where appropriate. Users never wonder whether an action succeeded.

**Rules:**

- Every action produces an immediate visual response.
- Loading states are explicit (skeletons, spinners, progress bars).
- Success and failure states are clearly communicated.
- System state is continuously visible.

### 5. Consistency

Layouts, terminology, colors, spacing, iconography, and interaction patterns remain consistent throughout the application. Similar actions look and behave identically.

**Rules:**

- Use design tokens from the [UI Design System](ui-design-system.md).
- Component behavior is consistent across all views.
- Terminology is consistent with the [Domain Model](domain-model.md).
- Visual hierarchy follows the same patterns everywhere.

### 6. Intelligent Defaults

User preferences and context are remembered. Known information is pre-filled. Likely next actions are suggested. Repetitive work is reduced.

**Rules:**

- Remember the user's last-used view, filters, and sort order.
- Pre-fill forms with sensible defaults.
- Suggest entities based on recent activity and relationships.
- Auto-save drafts and in-progress work.

### 7. Prefer Selection Over Input

Free-form typing is replaced with structured controls whenever practical. Dropdowns, toggles, chips, cards, calendars, autocomplete, and searchable pickers replace raw text fields.

**Rules:**

- Use autocomplete for entity selection.
- Provide searchable pickers for large sets.
- Use toggles and chips for categorical choices.
- Offer date pickers for temporal input.
- Only use free-form text when structured input is insufficient.

### 8. Error Tolerance

Users make mistakes. Workflows are forgiving rather than restrictive. Errors are prevented when possible without adding unnecessary friction.

**Rules:**

- Validate input early and provide clear error messages.
- Offer suggestions when input is invalid.
- Allow users to correct mistakes without losing progress.
- Destructive actions require confirmation; non-destructive actions do not.

### 9. Reversible Actions

Every non-destructive action is undoable whenever technically feasible. Undo is preferred over excessive confirmation dialogs. History, versioning, draft states, trash bins, and recovery mechanisms are provided. Irreversible actions are extremely rare and clearly communicated.

**Rules:**

- All entity modifications support undo.
- Relationship changes are reversible.
- View changes (filters, sort, layout) are reversible.
- Archive operations are reversible (see [Entity Lifecycle](domain-model.md#entity-invariants)).
- Delete operations are archive operations (soft delete).
- Hard delete does not exist in the user interface.

### 10. Performance Perception

Perceived speed is optimized as much as actual speed. Skeletons, optimistic updates, partial rendering, loading indicators, and background processing keep users informed during long operations.

**Rules:**

- Display skeletons while data loads.
- Use optimistic updates for fast-feeling interactions.
- Render progressively as data becomes available.
- Show progress indicators for operations > 500ms.
- Perform long-running operations in the background.

### 11. User Confidence

The interface makes users feel competent. Error messages explain what happened, why, and how to recover. Users are never blamed. Exploration is encouraged by making the system feel safe.

**Rules:**

- Error messages are specific, actionable, and non-blaming.
- Provide recovery paths for every error.
- Show success states to confirm actions.
- Make system state visible and understandable.
- Encourage exploration by making actions reversible.

---

## Interaction Heuristics

Every interface in Knowledge OS follows these heuristics:

| Heuristic                            | Description                                                                       |
| ------------------------------------ | --------------------------------------------------------------------------------- |
| One primary action per screen        | Each view has a single clear primary action                                       |
| Minimize clicks without sacrificing clarity | Reduce interactions, but never at the cost of clarity                       |
| Reduce context switching             | Keep the user in the current context when possible                                |
| Avoid interrupting user flow         | Notifications and modals are used sparingly                                       |
| Do not ask users to remember information already known by the system | Pre-fill, auto-complete, and suggest              |
| Do not expose unnecessary complexity | Hide advanced features behind progressive disclosure                              |
| Every visible element has a clear purpose | Remove decorative or redundant elements                                     |
| Every additional interaction must justify its existence | If it doesn't help, remove it                                 |
| Design for interrupted workflows and easy resumption | Auto-save, bookmarks, and session restoration                     |
| Make system state continuously visible | Show loading, success, error, and idle states                                   |

---

## Decision Framework

For every UI element or interaction, ask:

1. **Does this reduce or increase cognitive load?** If it increases load, remove or simplify it.
2. **Is the next action obvious?** If not, add visual cues or reorganize the layout.
3. **Can the user recover from mistakes?** If not, add undo or confirmation.
4. **Does the interface inspire confidence?** If not, improve feedback and error messages.
5. **Is feedback immediate?** If not, add loading states or optimistic updates.
6. **Is there unnecessary friction?** If yes, remove it.
7. **Is the user's goal easier after adding this feature?** If not, remove the feature.
8. **Can this interaction be simplified?** If yes, simplify it.
9. **Is the behavior consistent with the rest of the system?** If not, make it consistent.
10. **Does this help users stay in flow?** If not, redesign it.

---

## Optimization Targets

### Maximize

- **Clarity.** Every element is understandable.
- **Confidence.** Users feel competent and in control.
- **Predictability.** Interactions produce expected outcomes.
- **Learnability.** New users understand the interface quickly.
- **Recoverability.** Users can undo mistakes.
- **Efficiency.** Users accomplish goals with minimal effort.
- **Perceived responsiveness.** The interface feels fast.
- **User autonomy.** Users control their workflow.

### Minimize

- **Cognitive load.** Users make fewer decisions.
- **Decision fatigue.** Users are not overwhelmed.
- **Waiting uncertainty.** Users know what is happening.
- **User anxiety.** Users feel safe exploring.
- **Memory requirements.** Users do not need to remember information.
- **Unnecessary interactions.** Every click is justified.
- **Irreversible mistakes.** Users can always recover.
- **Interface complexity.** The interface is as simple as possible.

---

## Golden Rule

The best interface is the one users barely notice because it continuously guides them toward their goal while making every interaction feel natural, safe, predictable, and reversible.

---

## Relationship to Other Documents

This document defines the interaction design principles for Knowledge OS. It complements:

- [UI Philosophy](ui-philosophy.md) -- Defines what views are and how they are structured as projections.
- [UI Design System](../engineering/ui-design-system.md) -- Defines visual tokens, component specifications, and accessibility standards.
- [Domain Model](domain-model.md) -- Defines the entities, relationships, and components that interfaces render.
- [Pipeline](pipeline.md) -- Defines how views are generated from canonical data.

The hierarchy is:

1. **UI Philosophy** -- What views are and how they work (conceptual).
2. **Interaction Design** -- How users should feel and how interactions should behave (principles).
3. **UI Design System** -- How interfaces look and what components are available (implementation).

---

## Further Reading

- [UI Philosophy](ui-philosophy.md) -- The conceptual foundation for views and projections.
- [UI Design System](../engineering/ui-design-system.md) -- Visual tokens and component specifications.
- [Domain Model](domain-model.md) -- The entities and relationships that interfaces render.
- [Mental Model](mental-model.md) -- The projection model that underlies all views.
- [Progressive Disclosure](ui-philosophy.md#progressive-disclosure) -- How complexity is revealed.
