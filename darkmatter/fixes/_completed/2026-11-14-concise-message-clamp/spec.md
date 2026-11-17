---
kind: fix
created: 2026-11-14
status: human-in-the-loop
area: darkmatter
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-14-edit-action
depends-on:
    - 2026-11-14-file-interpolate-option
next_spec: 2026-11-14-shared-process-tree
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
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-14
review_iterations: 1
completed: true
needs_rulings: false
required_rulings:
implemented: true
implemented_by: "claude/opus"
---

# Keep the whole error message; fit it at delivery

## Why this exists

Darkmatter's [concise_message](../../lib/src/lifecycle/envelope.rs)
removes terminal escapes, folds error text onto one line, and cuts it to
240 characters with an ellipsis. The same function is used by Darkmatter's
[LifecycleErrorInfo](../../lib/src/lifecycle/context.rs), which supplies the
`err` object to lifecycle actions, and Claudine's
[diagnostic snapshot constructors](../../../claudine/lib/src/diagnostics/snapshot.rs),
which prepare errors for serialization. The omitted text is gone before a
terminal, a JSON consumer, or an action such as `say: "{{ err.msg }}"` can
choose what to do with it.

There are earlier losses too. Claudine's
[failure_message](../../../claudine/lib/src/harness/runtime.rs), which chooses
the explanation for a failed provider attempt, takes only the first nonempty
line of an explicit provider error. Its
[task ledger](../../../claudine/lib/src/stream/task_ledger.rs), which reports
incomplete sub-agent work, separately caps the displaced provider failure.
Removing only the shared 240-character clamp would leave those losses in place.

A single source limit cannot suit every destination. Terminal rendering and
JSON can retain the explanation; a speech route may make it tiring to hear;
a desktop notification may display only part of it. Display space and a
transport's accepted payload size are different constraints. Neither is a
reason to discard the shared diagnostic.

The review of `2026-11-14-edit-action` surfaced this problem. The author ruled
that any design that limits how fully an error can be described is an
anti-pattern.

## Author rulings (2026-11-14)

- **Do not limit error verbosity at the source.** Errors keep their complete
  message end to end, subject to single-line hygiene described below.
- **Implement each concern once.** Darkmatter owns one message-hygiene
  implementation. A delivery channel owns its own size handling; packages
  do not maintain copies of an error clamp.
- **Keep current guidance consistent.** Documentation and skills describing
  the current message contract must agree and must stop promising a
  240-character limit.

## Scope and dependency

This fix changes Darkmatter's lifecycle messages and Claudine's diagnostic,
provider-attempt, and incomplete-task messages. It also removes upstream
error-text caps identified in the inventory. It does not turn an entire
process log, every error in a source chain, or a terminal's decorated report
into the error message. Existing diagnostic selection and one-level cause
selection remain authoritative.

`2026-11-14-file-interpolate-option` remains the declared predecessor (author
ruling, 2026-11-14: an ordering link for the serial pipeline and for the
caller-skill step, not a dependency of the message code). It
adds the option needed to publish canonical documentation examples containing
literal expressions into caller skills. This fix does not depend on that
option for its message implementation. When the predecessor lands, use its
final contract when updating and publishing caller skills; do not implement
transclusion behavior here.

No new shortening expression function (author ruling, 2026-11-14: a `truncate`-style
function is out of scope; authors write a deliberate short message, for example
with `err.code`), pipeline syntax, speech configuration,
provider capability model, or universal provider-limit registry is introduced
by this fix. Speech speaks the complete text (author ruling, see the
delivery table).

## Message contract

### Preserve the selected explanation

A complete message means all the text of the selected diagnostic's `Display`,
or all of an explicitly supplied provider/action error, after hygiene. It
includes text beyond character 240 and every substantive line, in order.
It does not mean retaining formatting or appending unselected errors.

Darkmatter's [DiagnosticSnapshot and DiagnosticCause](../../lib/src/lifecycle/envelope.rs)
carry this complete message, including the registered cause's message.
Darkmatter's [LifecycleErrorInfo](../../lib/src/lifecycle/context.rs) exposes
that same normalized message through `err.msg` when a snapshot is present.
Catalog-miss and unclassified action failures obey the same hygiene and
preservation rules. Code-based constructors in Claudine's
[snapshot module](../../../claudine/lib/src/diagnostics/snapshot.rs) must
normalize supplied text too; they must not become an alternate unchecked path.

Terminal diagnostics, `md compose` data output, Claudine's serialized snapshots,
and lifecycle error projections retain the complete explanation. These are
separate output formats: `md compose` does not gain Claudine's snapshot shape
or diagnostic catalog as part of this fix. Terminal output may wrap to its
configured width through `biscuit-terminal` components; wrapping is not loss.
The diagnostic message is one logical line even when a report contains headings,
source excerpts, or multiple rendered rows.

No fields are renamed or added, and snapshot schema version 1 stays in place:
the envelope's version rule reserves a bump for removed or retyped fields.
This intentionally changes a string's length contract. Existing stored snapshots
still deserialize, but previously discarded text cannot be recovered. No
migration, second `full_message` field, or duplicate raw-message store is needed.

### Keep hygiene without a length budget

Retain Darkmatter's public [concise_message and concise_message_with_suffix](../../lib/src/lifecycle/envelope.rs)
entry points, with documentation that makes clear that their names no longer
imply shortening. Remove the maximum-length constant and the private clamp.
Both entry points use one normalization implementation:

- Strip terminal escape sequences using the existing `biscuit-terminal`
  utility; do not introduce another escape parser in Claudine.
- Fold control-separated segments, including LF, CRLF, tabs, and stray control
  characters, into one line with a space between nonempty trimmed segments.
  Treat Unicode line and paragraph separators as line boundaries too. Preserve
  ordinary interior spaces and non-control Unicode text, including emoji.
- Do not add an ellipsis or impose any character or byte budget. Empty input
  remains empty. Running hygiene twice produces the same result.
- Normalize the complete base-plus-suffix text, including the suffix, so an
  escape or newline in either cannot bypass hygiene. The caller supplies the
  separator; the existing ` (attempt N)` suffix remains complete and is added
  only for attempts after the first.

Claudine calls these Darkmatter functions directly. Its public harness helper
may remain as a re-export; remove forwarding implementations and local budget
aliases. Keep host-specific message selection in Claudine.

### Remove losses before normalization

In Claudine's [provider failure selection](../../../claudine/lib/src/harness/runtime.rs),
preserve the existing source precedence: explicit error, structured guard
reason, timeout explanation, useful stderr fallback, then termination or exit
status. For an explicit error, use the complete nonblank normalized value
instead of its first line. An error containing only escapes or control
characters must not hide a useful fallback. The last-useful-line selection
for raw stderr remains: arbitrary process output is not an explicit error
field. Do not append the entire stderr log.

In Claudine's [task ledger](../../../claudine/lib/src/stream/task_ledger.rs),
remove both the displaced-provider clause budget and its clamp. Keep the
complete displaced failure and all incomplete task descriptions in declaration
order. Preserve the existing failure label, exit-code behavior, and structured
task outcomes. Those fields serve classification and must not be rebuilt by
parsing a longer message.

Claudine's [OpenCode error headline extraction](../../../claudine/lib/src/stream/logs/opencode/classify/mod.rs)
and [raw error-tag fallback](../../../claudine/lib/src/stream/logs/opencode/classify/asset.rs)
also impose numeric caps. Remove those caps (author ruling, 2026-11-14): these
are error producers, not bounded presentation previews. Existing parsing that distinguishes the error
message from an escaped stack trace or repeated cause remains in place;
this fix does not redesign OpenCode's error format.

**Reader's note:** the draft described the OpenCode sites as likely previews.
They populate classified error records used downstream, so keeping their caps
would contradict the author's complete-message ruling. By contrast, the
200-character site in Darkmatter's [shell executor tests](../../lib/src/markdown/compose/shell_expansion/executor.rs)
limits a helper's panic preview, not a lifecycle error. It may stay. Do not
rewrite unrelated previews or impose a comment on every numeric `take` call;
explain a retained bound only where its purpose is otherwise unclear.

## Delivery behavior and side effects

*(Author ruling, 2026-11-14: channels keep their existing size handling; a channel that cannot fit a long message fails through its existing delivery-failure path. No new size framework and no global cap. This is the meaning of ruling 2's "a delivery channel owns its own size handling".)*

All delivery methods receive the complete evaluated action text. Any fitting
happens after interpolation and provider-specific rendering, at the adapter
that knows the destination and its units. It must not mutate the original
snapshot, lifecycle value, log, or another channel's copy.

**Reader's note:** Messenger's [CapabilitySet](../../../messenger/lib/src/capabilities.rs)
contains feature flags, not text-length budgets. The draft's proposed lookup
there is unavailable. A new cross-provider size framework would expand this
fix substantially. Reuse the policies already implemented by provider
adapters and expose their existing failure behavior where no policy fits the
message; do not invent a global fallback cap.

| Destination | Behavior for a long message |
| --- | --- |
| Terminal, JSON, and logs | Preserve the complete message; terminal width controls wrapping only. |
| Messenger routes and webhooks | Keep existing provider size handling. For example, Messenger's [Discord webhook adapter](../../../messenger/lib/src/provider/discord_webhook.rs) splits its plain content into posts. Do not replace splitting with truncation or apply that content limit to embeds or other providers. An adapter without fitting may reject an oversized payload through its existing delivery-failure path. |
| Desktop notifications | Pass the full title/body to Messenger's desktop backend unless that backend already enforces a documented input limit. A notification center displaying fewer characters is not a shared-message limit. Do not invent a cross-platform 100–250-character clamp or duplicate notifications to show the remainder. |
| Speech | Speak the complete text (author ruling, 2026-11-14). No sentence picking and no speech budget; authors who want a short announcement write one. |

Existing actions such as `say: "{{ err.msg }}"` may now submit much longer
text. This is an intentional consequence of retaining the error. An OS may
show only part of a notification; a provider may reject a message that the
old source clamp happened to keep short. Keep the current delivery warning
and failure handling; do not convert such failures into silent success or
change lifecycle control flow. Documentation must distinguish submitting the
full message from guaranteeing that a remote service displays all of it.

For authors who want predictable short announcements, show a deliberately
written message using stable classification fields instead of advertising
`err.msg` as inherently short:

```yaml
failure:
  - say: "The run failed. Check the terminal for the full explanation."
  - notify: "Run failed: {{ err.code }}"
```

This example uses Claudine's classified `failure` event. Darkmatter's failure
event is named `failed`, and unclassified failures may not have `err.code`;
use fixed notification text in that case. Match behavior on classification
fields, never on the length or wording of the message.

## Documentation and wording

Update current contracts alongside implementation:

- Darkmatter's [lifecycle error handling](../../docs/topics/lifecycle/error-handling.md)
  and [repository lifecycle skill](../../../.claude/skills/darkmatter/lifecycle.md).
- Claudine's [lifecycle error fields](../../../claudine/docs/topics/flow-control/lifecycle.md),
  [incomplete-task behavior](../../../claudine/docs/topics/non-interactive-sessions.md),
  and [error architecture](../../../claudine/docs/topics/error-architecture.md)
  wherever they describe the affected message contract.
- Rust field/function comments on the affected constructors, envelope, harness,
  and task ledger. Replace claims such as “notification-safe,” “headline,” or
  “budget-reserved” where they imply bounded length. Keep the `edit` failure's
  reason-first wording because it is clearer on its merits, not to survive a cut.
- Canonical caller skills under
  [Claudine's use-skills directory](../../../claudine/docs/use-skills/use-claudine/)
  and any affected [Darkmatter caller skill](../../docs/use-skills/).
  Explain the longer-message behavior and revise examples that promise a brief
  announcement. Edit these repository sources, then run each changed area's
  `just publish-skills`; never edit installed user-scope copies directly.

Keep README changes limited to affected overview claims. Historical specs and
historical timeline entries remain snapshots; unrelated provider research or
skill-description limits are not this contract. A search for the number 240
alone is therefore not a valid completion check.

## Verification

Use the existing Test Toolkit and each affected area's L1 tests through
`just test` (nextest), plus `just lint` for changed Rust. Messenger checks are
needed only if its adapter code changes. This fix needs no live speech,
notification, remote-service, or focus-changing terminal/browser test.

1. **Shared hygiene:** compare the full normalized value for messages at 239,
   240, and 241 characters and one substantially longer Unicode message.
   Include ANSI styles, OSC hyperlinks, LF/CRLF, tabs, other controls, Unicode
   line separators, empty input, and idempotence. Verify an intact long base
   and suffix, including hygiene of suffix content, without a generated ellipsis.
2. **Projection parity:** cover a typed selected diagnostic, a registered
   one-level cause, a code-based snapshot, and a facet-less action/catalog miss.
   Assert exact normalized text, equality between the snapshot and lifecycle
   message, complete cause text, and serialization/deserialization retention.
   Use the actual cause field spelling; do not add an alias in this fix.
3. **Claudine producers:** an explicit multiline provider error keeps a unique
   ending beyond character 240; retry suffixes remain intact; blank errors
   still fall through; guard/timeout/stderr selection still works. A ledger
   with many named incomplete tasks retains every name and the full displaced
   failure. Long OpenCode fallback messages retain their endings too.
4. **Normal command paths:** use the existing isolated CLI fixtures to induce
   a long failure in `md compose` and Claudine. Check complete terminal/plain
   output, each command's existing JSON diagnostic format, and the message
   observed in error-carrying lifecycle events and `finalize`. Assert parsed
   fields or normalized text rather than host-specific paths and wrap points.
   `md compose` data output remains one envelope with no stray stderr output.
5. **Delivery isolation:** use recording emitters or existing mock transports
   to verify that a long evaluated action reaches the appropriate channel,
   existing adapter fitting still applies where supported, and no shortened
   delivery changes a subsequent terminal/JSON message. Assert that speech
   receives the complete text, unshortened. Do not build a new
   provider test matrix for unchanged adapters.
6. **Contract audit:** search affected production sources for the removed
   constants and error-message caps, and current docs/skills for old clamp
   promises. Inspect matches by purpose; unrelated preview limits and historical
   records are allowed. Check the published caller skill text as well.

No performance spike or benchmark is required. The change removes a length
budget and retains text already provided by the error producer. Keep hygiene
linear in input length and avoid repeated full-message processing in loops;
ordinary long-input regression coverage is sufficient for this risk.

## Open Questions

None.
