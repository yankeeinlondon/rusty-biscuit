# Unified Events

This document will discuss functional requirements for our "unified events" system in Claudine as well as discuss at a high level some of the key technical details we've used to implement this.

## Functional Goal

For every Agentic CLI we support, we need a fully defined and understood enum that defines:
- event names defined
- parameters that the event provides strongly typed
- return types for the given event

So for example, for **Claude Code** we have:

- `ClaudeCodeEvent` **enum** which defines all of the events that Claude Code defines
- This **enum** implements the `Hooker` trait which ensures that the enum variants have the following utility functions:
    - `event` - this returns the data payload's structure/type when the event is fired
    - `response` - this returns the data structure/type that this event must return
    - `description` - a `&'static str` description of this event

All other supported Agentic CLI's which support Hooks will also have their own **enum** which supports the `Hooker` trait.

- `CodexEvent`

