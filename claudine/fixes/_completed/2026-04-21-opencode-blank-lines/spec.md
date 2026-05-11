---
fixed: 2026-04-21
agent: "1"
---

# Functional Specification: OpenCode Stream Deduplication & Blank Line Management

## Overview

This specification addresses the issue of excessive blank lines appearing in OpenCode non-interactive sessions. The fix focuses on centralizing stream state in `LiveSemanticSink` and ensuring that invisible events do not disrupt the layout logic.

## Problem Statement

Current output rendering often results in multiple consecutive blank lines during transitions between tool execution (stderr) and assistant responses (stdout). Additionally, hidden lifecycle events (like `step_start` and `step_finish` in OpenCode) are triggering section changes in the `SectionTracker`, causing redundant separators to be injected even when no visible content was produced.

## Requirements

### 1. Stricter Tail Tracking for Stream Deduplication

The `LiveSemanticSink` must maintain a unified "fresh row" state that tracks the tail of both `stdout` and `stderr`.

- **Global Fresh Row State**: If the last character printed to _either_ stream was a newline, the sink is considered to be at a "fresh row."
- **Separator Suppression**: The automatic section separator (the blank line) must be suppressed if the sink is already at a fresh row and the next section starts with its own newline, or if injecting it would result in more than one consecutive blank line.
- **Goal**: Ensure transitions between tools (typically stderr) and assistant text (stdout) always result in exactly one blank line.

### 2. Invisible State Preservation for Section Transitions

Events that do not produce visible output must be transparent to the section tracking logic.

- **State Preservation**: Suppressed events (e.g., OpenCode `step_start` and `step_finish` Info events hidden from the UI) must **NOT** update the `SectionTracker`'s internal state.
- **Goal**: Prevent the tracker from detecting a "section change" for events that are filtered out, which currently leads to redundant separators being injected between tool batches.

### 3. Core Aesthetic Requirements

The rendering engine must strictly adhere to the following layout rules:

- **Intra-section Packing**: Items within the same section (e.g., consecutive tool results) must have a single newline between them (tightly packed, no blank lines).
- **Inter-section Separation**: Different sections (Tools, Thinking, Final Output) must be separated by exactly one blank line.
- **Deduplication**: There must never be more than one consecutive blank line in the output under any condition.

## Implementation Strategy

### LiveSemanticSink Updates

- Introduce a shared state in `LiveSemanticSink` to track if the last emitted character across all managed streams was `\n`.
- Update the write logic for `stdout` and `stderr` to update this shared state.
- Refactor the separator injection logic to query this shared state before writing a blank line.

### SectionTracker Refinement

- Modify the event processing loop to check if an event is "visible" before calling `SectionTracker::update`.
- Ensure that OpenCode's lifecycle Info events are correctly identified as "invisible" in this context.

## Acceptance Criteria

| Feature                 | Expected Behavior                                                                          |
| :---------------------- | :----------------------------------------------------------------------------------------- |
| **Tool Transitions**    | Consecutive tool executions are tightly packed with only a single newline between them.    |
| **Section Changes**     | Exactly one blank line exists between a tool block and the subsequent assistant text.      |
| **Invisible Events**    | `step_start`/`step_finish` events do not cause extra blank lines or trigger separators.    |
| **Blank Line Limit**    | No combination of stream output and injected separators results in `\n\n\n`.               |
| **Stream Interleaving** | Newlines at the end of `stderr` are correctly detected when the next write is to `stdout`. |
