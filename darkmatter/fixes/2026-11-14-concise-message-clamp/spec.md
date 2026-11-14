---
kind: fix
created: 2026-11-14
status: draft-spec
area: darkmatter
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-14-edit-action
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: false
review_iterations: 0
---

# Keep the whole error message; fit it at the channel

## Why this exists

Every lifecycle failure message is shortened **when the error is built**:
`darkmatter::lifecycle::envelope::concise_message` strips escapes, folds the
text onto one line, and cuts it at `CONCISE_MESSAGE_MAX_CHARS` (240) with a
`…`. `LifecycleErrorInfo::from_action_failure_with`
(`darkmatter/lib/src/lifecycle/context.rs`) stores only that cut text, and
`DiagnosticSnapshot.message` (Claudine's `diagnostics/snapshot.rs`) does the
same. The full reason is gone before any consumer sees it, so the terminal
line, the JSON diagnostic, `err.msg` in `failed`/`finalize`, speech,
notifications, and webhooks all receive at most 240 characters.

The clamp exists so `err.msg` can be passed verbatim to a speech route, a
desktop notification, or a webhook (`say: "{{ err.msg }}"`) without reading
out a multi-line terminal block. That concern is real, but:

1. It is enforced at the source, so channels with room for the whole
   explanation (a terminal, a JSON diagnostic, a Slack message) lose it for
   the sake of channels that do not.
2. One number fits no channel: desktop notifications show roughly 100–250
   characters depending on the OS, Slack accepts about 40,000, speech is
   limited by listening time rather than characters, and a terminal has no
   limit.
3. It distorts how errors are written: authors order and trim messages to
   survive the cut instead of explaining the problem (the `edit` action's
   message order was chosen this way).

The review of `2026-11-14-edit-action` surfaced it. The author ruled that any
design that limits how fully an error can be described is an anti-pattern.

## Author rulings (2026-11-14)

- **Do not limit error verbosity at the source.** Errors keep their complete
  message end to end.
- **DRY.** Each concern is implemented once: one single-line hygiene
  function, one place per delivery channel that fits text to that channel.
  No package keeps its own copy of a clamp.
- **No contradictory guidance.** Every skill and doc that describes the
  message contract says the same thing after this change, and none still
  describes a 240-character limit.

## Requirements

1. **The full message is kept.** `LifecycleErrorInfo` and
   `DiagnosticSnapshot` hold the complete message. The terminal rendering,
   `md compose` and Claudine's JSON diagnostics, and `err.msg` carry all of
   it.
2. **Single-line hygiene stays, without a length limit.** `err.msg` is still
   one line with no escape sequences or control characters (it is
   interpolated into `say`, `notify`, and messages), produced by one
   function in `darkmatter::lifecycle::envelope`. `CONCISE_MESSAGE_MAX_CHARS`
   is removed.
3. **Channels fit text themselves.** A delivery channel that has a real
   limit applies it at delivery, in one place per channel, using limits that
   channel's owner already declares where they exist (the `messenger`
   library's provider capabilities; the desktop-notification backend; the
   speech path). A channel without a limit (terminal, JSON, log) never cuts.
4. **One implementation.** Remove the duplicate clamp in
   `claudine/lib/src/stream/task_ledger.rs` (`clamp_chars`,
   `PRIOR_FAILURE_CLAUSE_MAX_CHARS`) and the constant alias
   `FAILURE_MESSAGE_MAX_CHARS` in `claudine/lib/src/harness/runtime.rs`;
   Claudine calls Darkmatter's hygiene function directly. Audit the other
   `chars().take(N)` sites found in the inventory below and keep only those
   that are not error-message clamps, each with a comment saying what it
   bounds.
5. **Message wording is chosen for clarity, not length.** Revisit the
   messages whose order or wording was chosen to survive the clamp (the
   `edit` failure message keeps its reason-first order, which the author
   prefers on its merits).

## Inventory (2026-11-14)

Implementations:
- `darkmatter/lib/src/lifecycle/envelope.rs` — `CONCISE_MESSAGE_MAX_CHARS`,
  `concise_message`, `concise_message_with_suffix`, `clamp_chars`
- `darkmatter/lib/src/lifecycle/context.rs` — `from_action_failure_with`,
  the catalog-miss path
- `claudine/lib/src/harness/runtime.rs` — `concise_message`,
  `FAILURE_MESSAGE_MAX_CHARS`, `concise_message_with_suffix` wrapper
- `claudine/lib/src/diagnostics/snapshot.rs` — `message`
- `claudine/lib/src/stream/task_ledger.rs` — `clamp_chars`
- To audit (likely previews, not error contracts):
  `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs`
  (`take(200)`), `claudine/lib/src/stream/logs/opencode/classify/`
  (`take(297)`, `take(497)`)

Descriptions to make consistent:
- `.claude/skills/darkmatter/lifecycle.md` (the `edit` message note)
- `darkmatter/docs/topics/lifecycle/error-handling.md` ("at most 240
  characters")
- `claudine/docs/topics/flow-control/lifecycle.md` (`err.msg`: "length-clamped
  (~240 characters)")
- user-scope `use-claudine` lifecycle examples that interpolate `err.msg`

## Verification

- A failure whose reason is longer than 240 characters reaches the terminal,
  the JSON diagnostic, and `err.msg` complete and on one line.
- Each channel with a limit fits the same message to its own limit, tested
  at that channel.
- A repository search finds no `CONCISE_MESSAGE_MAX_CHARS`, no error-message
  clamp outside the channel fitters, and no skill or doc describing a
  240-character limit.

## Open questions

- Should authors get an explicit short form (for example a `truncate`
  expression function, `{{ err.msg | truncate(120) }}`)? None exists today.
  Requirement 3 may make it unnecessary.
- Speech has no character limit, only a listening-time one. Should the speech
  path fit by sentence (speak the first sentence) or not at all?
