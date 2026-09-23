---
status: proposal
created: 2026-09-17
author: claude/claude-fable-5-1
related:
  - ../1-interactive-fm/spec.md
  - ../../../../prompts/_add/add-context-variables.md
  - ../../../../prompts/_add/add-expressions.md
---

# Author-Triggered Editor Handoff

## Problem

`claudine claude --edit` already hands the caller to their editor (`$VISUAL`, then
`$EDITOR`, then Darkmatter's probe order), waits for the buffer to close, and uses the
buffer as the prompt. The primitive is `darkmatter::editor::edit_text(seed, suffix)`
and the Claudine wrapper around it is `maybe_edit_prompt_source` in
`claudine/cli/src/commands/wrap/prompt_source.rs`.

A **prompt author** cannot reach that primitive. The `_add/*` prompts take a
`requirements` property that is naturally several paragraphs long, and today the only
ways to supply it are `requirements='…'` on the command line or the single-line
`biscuit-tui` text input that Interactive Mode drives for a missing required `string`.
Neither is a good place to write prose.

This spec proposes four ways to let a document ask for an editor buffer, evaluates each
against Claudine's existing seams (schema-driven Interactive Mode, lifecycle stacks and
flow directives, frontmatter effect values, and the direct-wrapper `--edit` flag), and
recommends a combination.

## Shared primitive (all options)

Whatever surface triggers it, the handoff is one service:

- resolve the editor through `darkmatter::editor::resolve_editor_command`;
- require both stdin and stderr on a TTY (stdout may be piped), `--silent` unset, and
  no inherited-stdin prompt, exactly the gate `--edit` applies today; a denied gate is a
  typed error naming the property, never a silent skip;
- seed the buffer with the property's current value when present, otherwise with the
  schema `default(...)` or an authored `seed`;
- write the buffer as a temp file with the requested suffix (`.md` by default) so the
  editor highlights it; an empty or unchanged buffer is a distinct outcome the caller can
  treat as "aborted";
- return the text to the **effective frontmatter** of the active document as a transient
  value with caller-override provenance. It is never written to disk unless a lifecycle
  action explicitly persists it with `set_frontmatter`.

Status output says which editor opened and for which property; tracing records the
property name and byte count, never the content.

## Option A — a schema constraint: `string(required; editor)`

Extend SimplifiedSchema's constraint vocabulary with `editor` (optionally
`editor(md)` / `editor(txt)` for the buffer suffix) on `string` and `string[]`
properties. Claudine's Interactive Mode widget table gains a row: a missing required
property whose type carries `editor` opens the editor instead of the one-line text
input.

```yaml
$schema:
    requirements: string(required; not-empty; editor(md)) -> what to add; opens your editor when omitted
```

Pros:

- Reuses the existing collection loop unchanged: the six Interactive Mode gates,
  `prompt_for_missing`, the `MissingProperties` error in non-TTY runs, the re-compose after
  collection, sequence prompt-once-and-reuse, and shell completion all keep working.
- `requirements='…'` on the command line still bypasses it, so scripts are unaffected.
- The constraint is discoverable in `md schema about`, DMLS hover, and the status report
  Claudine prints before collecting.

Cons:

- Only fires when the value is **missing**. It cannot re-open an existing value for
  editing, and it cannot run mid-lifecycle (after an agent produced something).
- It adds a presentation hint to the schema vocabulary. `match(...)` already is one
  (completion and partial resolution read it), so this is not a new category, but it is
  a Darkmatter change for a Claudine behavior.

## Option B — a lifecycle action verb: `edit`

Add `edit` to the lifecycle action vocabulary, valid in every event, positional and
key/value:

```yaml
initialize:
    stack:
        - when: "!requirements"
          action:
              - edit: requirements                 # positional: property name
        - when: "!requirements"
          action:
              - error: "no requirements were provided"
success:
    stack:
        - when: "file_exists(spec)"
          action:
              - action: edit
                property: reviewer_notes
                seed: "{{ frontmatter(spec, 'summary') }}"
                suffix: ".md"
                label: "Add review notes for the spec the agent just wrote"
              - resume: "The caller reviewed the spec and added notes: {{ reviewer_notes }}"
```

`edit` is a **safe mutator** in the sense the loop `set` mutator is: it writes one
property of the in-memory effective frontmatter for the active document and nothing on
disk. Because `initialize` runs before schema validation and the stabilized reread judges
the document afterwards, an `edit` in `initialize` satisfies a `required` property the
same way a `set_frontmatter` there does today, and the ordinary `MissingProperties`
verdict still fires if the buffer came back empty and nothing else supplied the value.

Pros:

- The author decides **when and why**: guarded by `when`, seeded from any expression,
  usable after the provider ran (`success` + `edit` + `resume` is a genuine
  human-in-the-loop review step), and composable with `proxy.with` to forward the text.
- No schema change; `$schema` keeps describing data, and the verb is documented with the
  other flow and side-effect verbs.
- The non-TTY case is the same typed placement error family as `resume` without a
  session (`LifecycleEditRequiresTerminal`); `no_error: true` turns it into a skip.

Cons:

- A new verb in the lifecycle vocabulary, with parse-time validation, positional and
  key/value forms, and docs in three places (lifecycle topic, skill, frontmatter
  reference).
- Introduces the first lifecycle action that mutates effective frontmatter in memory.
  The loop engine's `set` already does this between iterations, so the seam exists, but
  the lifecycle executor currently mutates only through Darkmatter's effect engine (disk).
  The transient write needs the same provenance rules as `proxy.with` (caller overrides
  still win; discarded on handoff).

## Option C — an effect-shaped frontmatter value: `$(edit)`

Treat the editor as a frontmatter effect in the same family as `$(cmd)` shell expansion,
which already runs at a fixed stage (frontmatter shell expansion), is discovered at
preflight, and is gated by consent:

```yaml
requirements: "$(edit:md)"
```

or, closer to the existing token grammar, `requirements: "$(edit --suffix md)"`. Preflight
lists "editor handoff for `requirements`" as an approvable effect; a caller override of the
property makes the value literal and the effect never runs.

Pros:

- Smallest syntax; nothing new in `$schema` or the lifecycle vocabulary.
- Fits the whole-value rule: the value is executable state and must resolve or fail.

Cons:

- Overloads `$()`, whose contract is "run a shell command, write trimmed stdout back".
  A pseudo-command inside it is a second grammar hidden in the first.
- It runs during Darkmatter's compose pipeline, so Darkmatter would own a terminal
  interaction. That contradicts the boundary that composition is effectful but
  non-interactive and that only the orchestrator talks to the user.
- Cannot be conditioned on lifecycle state and cannot run after the provider.

Not recommended.

## Option D — a caller-side flag: `claudine compose <file> --edit[=<property>]`

Extend the direct wrapper's `--edit` to the three composition commands. Bare `--edit`
opens the editor for the first required `string` property that is missing (or the one
carrying Option A's `editor` constraint); `--edit=requirements` names the property.

Pros:

- Symmetric with `claudine claude --edit`; nothing changes in any document.
- Cheap, and immediately useful for prompts that were never written with an editor in
  mind.

Cons:

- Caller-triggered, not author-triggered. It does not answer the request on its own; it
  is the ergonomic complement to A or B.

## A note on the unscheduled `interactive:` mapping

`1-interactive-fm` proposes an `interactive:` mapping of property names to TUI question
types. That design collides with today's boolean `interactive: true` (session mode) and
would need a new key, such as `inputs:`. If it proceeds, an `Editor` question type is the
natural home for the buffer widget, and Option A's `editor` constraint becomes the
schema-side spelling of the same widget so a document does not have to declare a property
twice.

## Recommendation

Implement **A + B + D** on the shared primitive above, in that order:

1. **A** delivers the `_add/*` use case with the least new surface and keeps scripts
   unaffected. It is the foundation because it is also the widget the other two routes
   reuse.
2. **B** is the author-triggered form the request asks for and the only option that
   supports post-provider review loops (`success` + `edit` + `resume`). It should be
   specified as a safe mutator with `proxy.with` provenance rules, documented next to
   `set_frontmatter`, and land after A so the widget and gating exist.
3. **D** is a small flag on top of the same service and can ship with A.

Option C is recorded so it is not re-proposed; it should not be built.

## Acceptance sketch

- A missing `string(required; editor)` property opens the editor under a TTY and
  produces `MissingProperties` otherwise; the collected value is transient and reaches
  composition, schema validation, and lifecycle strings.
- `edit` in `initialize` satisfies a required property; `edit` under a denied gate fails
  with a typed error unless `no_error: true`; `edit` never writes the file.
- `success` + `edit` + `resume` delivers the edited text to the resumed session.
- `--edit=<property>` on `compose`, `inline-compose`, and `sequence` opens the editor
  for that property before preparation and refuses on a non-TTY.
- No test opens a real editor: the resolver and `edit_text` are injected, as
  `maybe_edit_prompt_source_with` already allows.
