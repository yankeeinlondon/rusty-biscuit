---
total_phases: 4
created: 2026-11-14
phase: 1
agent: claude/sonnet
yolo: true
---

# Plan: keep the whole error message; fit it at delivery

## Summary and Definition of Done

Darkmatter's `concise_message` strips escapes, folds text onto one line and
cuts it to 240 characters with `…`. That cut happens at the source, so every
destination (terminal, JSON, `err.msg`, `say`, `notify`) receives a shortened
explanation. Claudine adds three more losses of its own: `failure_message`
keeps only the first line of an explicit provider error, the task ledger caps
the displaced provider failure at 96 characters, and two OpenCode classifiers
cap error text at 300 and 500 characters.

This plan removes every one of those caps, keeps the hygiene (escape stripping,
one logical line, no stray controls), and leaves size handling to delivery
adapters, which already own it. No production delivery code changes.

**Done means all of the following hold:**

- `concise_message` and `concise_message_with_suffix` in
  `darkmatter/lib/src/lifecycle/envelope.rs` share one normalizer, add no
  ellipsis, impose no budget, and are idempotent. `CONCISE_MESSAGE_MAX_CHARS`
  and `clamp_chars` no longer exist anywhere.
- `DiagnosticSnapshot.message`, `DiagnosticCause.message` and `err.msg` carry
  the complete normalized text. `DiagnosticSnapshotExt::from_code` normalizes
  the text it is given.
- Claudine's `failure_message` uses the complete normalized explicit provider
  error. An escape-only error falls through to the next source. The `(attempt N)`
  suffix is intact.
- The task ledger keeps the complete displaced failure and every incomplete
  task description, in declaration order.
- The two OpenCode caps are gone.
- Docs, Rust doc comments and caller skills no longer promise a 240-character
  limit. `claudine`'s `just publish-skills` has been run.
- Verification: `just test` and `just lint` pass in `darkmatter/` and
  `claudine/`. The contract audit in Phase 4 is clean. No L2 or L3 test is run.

## Phase 1: Darkmatter hygiene (the single implementation)

### Necessary Rules

Each rule states the decision, then its evidence.

**R1. One private `normalize(text: &str) -> String` in `envelope.rs` is the only
hygiene implementation.** `concise_message(text)` returns `normalize(text)`.
`concise_message_with_suffix(base, suffix)` returns
`normalize(&format!("{base}{suffix}"))`, so an escape or newline in the suffix
cannot bypass hygiene and the caller's own separator (the leading space of
` (attempt N)`) is preserved by the interior-space rule. Evidence: the spec's
"Keep hygiene without a length budget"; the current pair at
`envelope.rs:222-234` already funnels through one function.

**R2. `normalize` is `collapse_to_line(&strip_ansi_codes(text))` with the
existing `strip_ansi_codes` import from `biscuit-terminal`.** Evidence:
`envelope.rs` already imports it for `concise_message_with_suffix`, and the spec
forbids a second escape parser.

**R3. The split predicate adds U+2028 and U+2029.** Use
`c.is_control() || matches!(c, '\u{2028}' | '\u{2029}')`. Evidence:
`char::is_control` covers only category Cc, and U+2028/U+2029 are Zl/Zp. The
spec requires treating them as line boundaries. Segments are still trimmed,
filtered for empty, and joined with one space, so the output is idempotent.

**R4. Delete `CONCISE_MESSAGE_MAX_CHARS` (pub const, `envelope.rs:45-46`) and
`clamp_chars` (`envelope.rs:250-256`).** The only users are:
`envelope.rs` (the two functions), `envelope/tests.rs:34-37`,
`claudine/lib/src/harness/runtime.rs:101,123,276` and
`claudine/lib/src/harness/runtime/tests.rs:191-300`. Nothing else in the
workspace references the constant (grep over `*.rs`).

**R5. Public names stay.** `concise_message` and `concise_message_with_suffix`
keep their names; their docs say they do not shorten. Evidence: spec "Retain
Darkmatter's public ... entry points".

**R6. No field rename, no schema bump.** `DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION`
stays 1. Evidence: the spec's "No fields are renamed or added".

**R7. `MdDiagnostics` (`darkmatter/lib/src/lifecycle/md_host.rs:320-340`)
needs no change.** Its `select`, `snapshot_for_code` and
`snapshot_for_error_kind` return `None`, so `md` failures are facet-less and
`err.msg` comes from `LifecycleErrorInfo::from_action_failure_with` or
`from_error_with` (`context.rs:194,259`), both of which already call
`concise_message`. `md compose` data output reads `failure.msg` at
`markdown/compose/report_diagnostics.rs:280`, so it retains the complete text
once the clamp is gone. No snapshot shape or catalog is added to `md`.

**R8. `cancellation.rs:52` needs no change.** It passes `info.msg`, which is
already normalized, to `snapshot_for_code`.

### Tasks (Wave 1, sequential: one file family)

- [x] **Rewrite hygiene**
    - In `darkmatter/lib/src/lifecycle/envelope.rs`, implement R1-R4.
    - Rewrite the `///` docs of `concise_message` and
      `concise_message_with_suffix`: strip escapes, fold to one line, no length
      limit, idempotent; the caller supplies any separator. Remove "clamped",
      "notification-safe" and the "budget" wording.
    - Keep the doc example `concise_message("first\n  second\t") == "first second"`.
- [x] **Fix field docs**
    - `envelope.rs:70-74` (`DiagnosticSnapshot.message`) and `:108`
      (`DiagnosticCause.message`): "Complete, single-line, escape-free message of the
      selected diagnostic; no length limit. Delivery adapters fit it to their
      destination." Do not call it "notification-safe".
    - `darkmatter/lib/src/lifecycle/context.rs:81` (`LifecycleErrorInfo.msg`) and
      `:250-252` (`from_action_failure_with`): the same wording; drop "cannot
      reach TTS or a webhook multi-line or escape-bearing" in favor of
      "normalized to one escape-free line, not shortened".
    - `context.rs:273` says "concise message"; change it to "normalized message".
- [x] **Rewrite envelope tests** (`darkmatter/lib/src/lifecycle/envelope/tests.rs`)
    - Replace `a_suffix_survives_clamping` with
      `a_long_base_and_suffix_stay_intact`: a 600-character base plus
      `" (attempt 2)"` equals the base followed by the suffix; no `…`.
    - Add `lengths_around_the_old_limit_are_unchanged`: for lengths 239, 240, 241
      and one 2,000-character message mixing ASCII, CJK and emoji, the output
      equals the input and the character count is unchanged.
    - Add `hygiene_table`: one test with a table of (input, expected) for ANSI
      SGR, an OSC-8 hyperlink with BEL terminator, LF, CRLF, tab, a stray `\u{7}`,
      U+2028, U+2029, U+0085, empty input (empty output), all-whitespace input
      (empty output), and interior double spaces (preserved).
    - Add `the_suffix_is_normalized_too`: suffix `" (attempt \u{1b}[31m2\u{1b}[0m\n)"`
      yields `"base (attempt 2 )"` with no escape or newline.
    - Keep `concise_message_is_idempotent`; extend it to run the whole table twice.
- [ ] **Validate Phase 1**
    - `cd darkmatter && just test` and `just lint` pass. (Claudine does not
      compile until Phase 2 removes its references to the deleted constant;
      that is expected and Phase 2 follows immediately.)

## Phase 2: Claudine producers

Depends on Phase 1. The three tasks edit disjoint files and run as one wave.

### Necessary Rules

**R9. `claudine/lib/src/harness/runtime.rs`: the local `concise_message` and
`sanitize_message` are deleted.** Replace the function with
`pub use darkmatter::lifecycle::envelope::concise_message;`. `harness/mod.rs:24`
(`pub use runtime::{..., concise_message, ...}`) and
`diagnostics/snapshot.rs:53` (`use crate::harness::runtime::concise_message`)
keep compiling unchanged. Evidence: spec "Its public harness helper may remain
as a re-export; remove forwarding implementations".

**R10. `failure_message` calls
`darkmatter::lifecycle::envelope::concise_message_with_suffix(&base, &suffix)`
directly.** The `FAILURE_MESSAGE_MAX_CHARS` test constant (`runtime.rs:101`) is
deleted. The suffix rule is unchanged: ` (attempt N)` only when `attempt > 1`.

**R11. Explicit provider error:** `base_failure_message` computes
`outcome.error_message.as_deref().map(concise_message).filter(|m| !m.is_empty())`
and returns it when present. This replaces `headline(text, LinePick::First)`.
An error made only of escapes or controls normalizes to empty and falls
through, so guard, timeout, stderr and exit-code sources still apply with the
existing precedence. Evidence: spec "Remove losses before normalization".

**R12. Stderr stays last-useful-line.** Remove the `LinePick` enum and rename
`headline` to `last_useful_line(text) -> Option<String>`: strip escapes, trim,
drop empty lines, take the last. The whole stderr log is never appended.
Evidence: spec "The last-useful-line selection for raw stderr remains".

**R13. Task ledger (`claudine/lib/src/stream/task_ledger.rs`):** delete
`PRIOR_FAILURE_CLAUSE_MAX_CHARS` and `clamp_chars`. `prior_failure_clause`
normalizes `kind` and `message` each with `concise_message` before the
blank-check, then returns the unclamped clause. Normalizing first keeps an
escape-only message from leaving a dangling `after provider failure:`.
`compose_message` is unchanged; it already lists every task in declaration
order, and `apply_to_summary` still passes the composed text through
`concise_message` once. `subagent_outcomes`, `error_kind`
(`INCOMPLETE_SUBAGENTS_ERROR_KIND`) and `exit_code` handling are untouched.
`incomplete_message` stays `pub` (its only caller is
`task_ledger/tests.rs:373`); only its doc changes.

**R14. OpenCode caps are removed, nothing else is redesigned.**
`classify/mod.rs:175-178` (the 300-character `error_headline` cap) and
`classify/asset.rs:69-76` (the 500-byte raw-tag cap) are deleted; both return the
whole value. The `\n` / `" cause=` stack cutting in `error_headline` stays.
Evidence: spec "Remove those caps (author ruling, 2026-11-14)".

**R15. Retained bounds, left alone and uncommented:** the 200-character panic
preview in `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs`,
and `claudine/lib/src/stream/logs/opencode/bridge/format.rs:157`
(`truncate_for_inline`, a tool-argument display preview, not an error
producer). Evidence: spec's reader's note.

### Wave 1 tasks (run concurrently)

- [ ] **Harness and snapshots** (`claudine/lib/src/harness/runtime.rs`,
  `runtime/tests.rs`, `claudine/lib/src/diagnostics/snapshot.rs`,
  `diagnostics/snapshot/tests.rs`)
    - Implement R9-R12.
    - `DiagnosticSnapshotExt::from_code` (`snapshot.rs:148`): set
      `message: concise_message(&message)` instead of `message`. This closes the
      unchecked path for `snapshot_for_code` and `snapshot_for_error_kind`
      (`snapshot.rs:69-75`). The existing test
      `every_error_kind_code_projects_a_catalog_shaped_detail_object`
      (`composition/lifecycle/context/tests.rs`) passes `String::new()` and stays valid.
    - Rewrite the docs on `failure_message`, `concise_message` (now a re-export,
      so its docs come from Darkmatter), and the module doc at
      `runtime.rs:95-101` ("headline, not the archive"). Say: the message is the
      complete selected explanation; the stderr log is not appended. Keep the
      cascade list 1-5 as is.
    - Update `runtime/tests.rs`:
        - `error_message_first_line_stripped_and_clamped` becomes
          `an_explicit_error_keeps_every_line`: input
          `"\x1b[31mBilling error\x1b[0m\n" + "x"*300 + "\nTHE-END"` yields
          `"Billing error " + "x"*300 + " THE-END"`.
        - `guard_context_pattern_over_cap_is_clamped`,
          `oversized_message_with_suffix_respects_cap_and_keeps_suffix` and
          `stderr_oversized_last_line_clamped_to_single_line` become
          retention tests: the 300-character value comes back whole, no `…`, one
          line, suffix intact for attempt 2.
        - Add `an_escape_only_error_falls_through_to_stderr`: `error_message =
          "\x1b[31m\x1b[0m\n\t"` with stderr `"warming\nreal failure"` gives
          `"real failure"`.
        - Add `a_blank_error_falls_through_to_the_exit_code_message`.
        - Rewrite `the_incomplete_headline_is_clamped_while_the_facts_stay_complete`
          (`runtime/tests.rs:418`) to `the_incomplete_headline_names_every_task`:
          the headline contains `...-39` and `sa_39` is still in
          `subagent_outcomes`; drop the `<= 240` assertion.
    - Add to the snapshot tests: `from_code("<any catalog code>", "a\n\x1b[1mb\x1b[0m")`
      has `message == "a b"`.
- [ ] **Task ledger** (`task_ledger.rs`, `task_ledger/tests.rs`)
    - Implement R13. Rewrite docs at `task_ledger.rs:395-398`, `:403-410`
      (deleted with the constant) and `:430-436`: the headline names every task and
      the complete displaced failure.
    - In `task_ledger/tests.rs`: rewrite
      `the_composed_message_still_honors_the_concise_message_contract` (line 313):
      keep the escape and multi-line assertions, assert the message contains
      `...-29` and the full `x`*500 tail of the displaced failure, drop the `<=
      240` and non-vacuity-truncation assertions. Rewrite
      `a_long_message_only_failure_keeps_its_reservation_and_the_task_list_gives_way`
      (line 460) to
      `a_long_message_only_failure_keeps_the_clause_and_every_task`: remove the
      `PRIOR_FAILURE_CLAUSE_MAX_CHARS` use; assert the clause text is complete
      and all 30 task names appear in declaration order (compare `find` indices
      ascending). Rewrite `the_headline_names_tasks_while_the_machine_list_stays_complete`
      (line ~365) so the message names all 40 tasks. Add
      `an_escape_only_displaced_message_leaves_no_dangling_clause`: kind `None`,
      message `"\x1b[0m"` produces no `after provider failure` text.
- [ ] **OpenCode** (`classify/mod.rs`, `classify/asset.rs`, `classify/tests.rs`)
    - Implement R14. Rewrite the doc at `mod.rs:155-163` ("apply the same
      char-boundary-safe cap") and the comment at `asset.rs:69-70` ("truncated if
      huge").
    - In `classify/tests.rs`: `non_json_error_tag_truncation_respects_char_boundaries`
      (line 1152) becomes `non_json_error_tag_is_kept_whole`: the message
      equals the whole tag text (`ends_with("something went wrong")`, no trailing
      `...`, `chars().count()` equals the tag's). `error_headline_cap_respects_char_boundaries`
      (line 1223) becomes `error_headline_keeps_a_long_multibyte_payload_whole`:
      `error == payload` and 400 characters.

### Validate Phase 2

- [ ] `cd claudine && just test` and `just lint` pass. Claudine's L1 CLI tests
  that mention 240 are handled in Phase 3.

## Phase 3: Parity, command-path and delivery-isolation tests

Depends on Phases 1 and 2. All tasks are test-only and run as one wave
(Wave 1). Per the spec, none runs L2, a live speech or notification backend, or
a focus-changing terminal or browser.

### Necessary Rules

**R16. Messenger needs no change and no test.** Evidence: no production
delivery code changes; Messenger's `CapabilitySet` has flags only, and
`messenger/lib/src/provider/discord_webhook.rs` already splits plain content.
No `just test` in `messenger/` is required.

**R17. The L2 files `claudine/cli/tests/level2/level2_incomplete_subagents_capture.rs`
and `claudine/cli/tests/common/incomplete_subagents.rs` get comment edits only
and are not run.** Their assertions anchor on the bullet glyph and the heading,
not on truncation. The words "truncated" (L2 line 106) and "240-character"
(common line 14) are updated to say the lifecycle headline now names every task.

### Wave 1 tasks

- [ ] **Projection parity** (`claudine/lib/src/diagnostics/snapshot/tests.rs`)
    - One test builds each of: a typed selected diagnostic with a registered
      one-level cause (via `DiagnosticSnapshot::select`), a code-based snapshot
      (`from_code`), a facet-less action failure
      (`LifecycleErrorInfo::from_action_failure` with an unclassified verb), and
      a catalog-miss `from_error_with`. Each carries a message of 241+ characters
      with ANSI, multiple lines and a unique ending.
    - Assert: the exact normalized text; `snapshot.message == info.msg`;
      `snapshot.cause.message` is complete (the real field is `cause`, with
      `DiagnosticCause.message`; add no alias); and
      `serde_json::to_value` then `from_value` round-trips the same message.
- [ ] **md compose command paths** (`darkmatter/cli/tests/l1/compose_lifecycle.rs`,
  `compose_data_output.rs`)
    - In `compose_lifecycle.rs`, model on
      `a_composed_error_runs_finalize_without_failed_and_exits_nonzero` (line 251):
      `error:` with a 600-character text whose unique last word is `TAIL-MARKER`,
      finalize `info: "event:finalize {{ err.msg }}"`; assert stderr contains
      `TAIL-MARKER` in both the terminal diagnostic and the finalize line.
    - In `compose_data_output.rs`, extend the failing leg of
      `a_lifecycle_run_writes_only_the_envelope_to_stdout` (line 467) with a
      second fixture: a long multi-line `error:`; assert stdout is exactly one
      JSON document, `report["diagnostics"][0]["message"]` equals the normalized
      full text, and stderr is empty.
- [ ] **Claudine command paths** (`claudine/cli/tests/l1/wrap_incomplete_subagents.rs`,
  `characterization_error_routes.rs`)
    - Rewrite `display_truncation_never_reaches_the_machine_record` (line 299) as
      `the_headline_and_the_machine_record_both_name_every_task`: delete the
      `<= 240` assertion and the "vacuous unless it truncated" assertion; assert
      the `failure-msg=` line contains every `long_task_name(index)` in order, the
      `session_end` extra keeps all facts, and the terminal diagnostic names
      all. Update the doc comment on the test, the `ManyLongNamedStops` doc
      (line 58) and `characterization_error_routes.rs:26` where it says the
      projection is concise or clamped.
    - Add one test to `characterization_error_routes.rs`, using its `run_route`
      helper: an `initialize` stack whose `error:` text is 500 characters with a
      unique ending, and a `finalize` stack that appends `{{ err.msg }}` to the
      events file via the existing `append_line` pattern; assert the recorded
      line ends with the unique ending.
- [ ] **Delivery isolation**
  (`claudine/lib/src/composition/lifecycle/tests/audio_emission.rs`,
  `.../executor/tests/event_time_interpolation.rs`)
    - In `audio_emission.rs`, model on
      `executor_publishes_an_effect_then_speech_in_stack_order` (line 331): a
      1,000-character speech text is recorded unshortened by the recording emitter.
    - In `event_time_interpolation.rs`, at the test around line 540: replace the
      `<= 240` assertion and the "~240-char cap" comment with a retention
      assertion, and add a sibling test: a failure with a 600-character
      `err.msg` and a `say: "{{ err.msg }}"` action; the recorder receives the
      full text and `info.msg` is unchanged afterward (no mutation by delivery).
    - Update the comment at `context/tests.rs:759` ("budget-reserved") to say the
      suffix survives intact.
- [ ] **Validate Phase 3**
    - `cd darkmatter && just test`, `cd claudine && just test`, and `just lint`
      in both. The L1 CLI tests run under `just test`; L2 stays unrun.

## Phase 4: Docs, skills and contract audit

Depends on Phases 1-3. Tasks 1-3 are one wave (Wave 1); the audit is Wave 2 and
needs the published skill.

### Necessary Rules

**R18. The Darkmatter caller skill (`darkmatter/docs/use-skills/use-darkmatter/`)
needs no change, so `just publish-skills` is not run in `darkmatter/`.**
Evidence: `grep -rn "err\.msg\|240\|concise" darkmatter/docs/use-skills` matches
nothing about message length.

**R19. The Claudine caller skill changes in `lifecycle.md` only.** The `err.msg`
examples in `lifecycle.md:50,146` and `SKILL.md:88` do not promise brevity and
stay. Add one paragraph to `lifecycle.md` after the `err` row of the variable
table: `err.msg` is the complete one-line explanation and can be long; to
announce briefly, write a fixed message or use `err.code`; match on
`err.code`, never on message text; and a notification center or remote service
may show only part of a long message. Include the spec's YAML example,
which uses `failure:`. Then run `just publish-skills` in `claudine/`. Never edit
`~/.claude/skills`.

**R20. Darkmatter's failure event is `failed`, Claudine's is `failure`, and
unclassified failures have no `err.code`.** The Darkmatter doc example uses
fixed notification text; the Claudine one uses `err.code`. Evidence: the spec's
note after the YAML example.

### Wave 1 tasks

- [ ] **Darkmatter docs**
    - `darkmatter/docs/topics/lifecycle/error-handling.md:166-168`: replace
      "folded onto one line of at most 240 characters" with "folded onto one
      line; it is never shortened, so a long error stays whole. A channel that
      cannot show it all fits it itself." At line 243, make the `msg` row say
      "complete, single-line, escape-free; do not branch on it". Add a short
      example with fixed notification text under `failed:` and the sentence that
      submitting the full message is not a guarantee that a remote service
      displays all of it.
    - `.claude/skills/darkmatter/lifecycle.md:74`: replace "A failure message is
      clamped to one 240-character line (`concise_message`), so `md` puts the
      reason before the target" with: failure messages are folded to one line and
      never shortened; `edit` keeps its reason-first wording because it reads
      better.
    - Rust docs on the `edit` failure that say "survive a cut": grep
      `darkmatter/lib/src` for `reason before` and `clamp` and reword; keep the
      reason-first order.
- [ ] **Claudine docs**
    - `claudine/docs/topics/flow-control/lifecycle.md`: the `err.msg` table row
      (line 1490), the `#### err.msg` section (lines 1520-1531, including the
      "(~240 characters)" and "what the clamp exists to prevent" sentences, and
      the `failure:` example's "safe: one clean line" comment), and the planned
      note at line 1557 ("will remain a concise headline"; change to "will remain
      the one-line selected message and will not include the response body").
    - `claudine/docs/topics/non-interactive-sessions.md:415`: the composed
      message names every incomplete task and the complete displaced provider
      failure, with no 240-character budget and no reservation.
    - `claudine/docs/topics/error-architecture.md`: add one sentence in the
      snapshot section (near line 184) that `message` is complete and normalized,
      not shortened. No other change there.
    - `claudine/lib/src/composition/lifecycle/context.rs:105-116`,
      `claudine/lib/src/composition/sequence/task/mod.rs:132`: reword "concise"
      and "budget-reserved" to "normalized, not shortened".
- [ ] **Claudine caller skill** (R19)
    - Edit `claudine/docs/use-skills/use-claudine/lifecycle.md`, then run
      `cd claudine && just publish-skills`. Confirm the installed
      `~/.claude/skills/use-claudine/lifecycle.md` contains the new paragraph.
      `{{` in the paragraph is written `{{{ ... }}}` per the composition rule
      for caller skills.

### Wave 2: contract audit and final validation

- [ ] **Audit**
    - `grep -rnE "CONCISE_MESSAGE_MAX_CHARS|clamp_chars|PRIOR_FAILURE_CLAUSE_MAX_CHARS|FAILURE_MESSAGE_MAX_CHARS|sanitize_message" --include='*.rs' darkmatter claudine`
      returns nothing.
    - `grep -rnE "notification-safe|budget-reserved|240-char|240 char|length-clamped|~240" --include='*.md' --include='*.rs' darkmatter claudine .claude`
      returns only historical specs under `fixes/` or `features/` or timeline
      entries. Fix any other match.
    - Inspect `take(` and `chars().count() >` matches in
      `claudine/lib/src/stream/logs/opencode/` and the Darkmatter shell executor;
      the only allowed survivors are R15's two.
    - `grep -n "240\|clamp" ~/.claude/skills/use-claudine/*.md` finds no old
      promise (the published copy).
- [ ] **Final validation**
    - `just test` and `just lint` in `darkmatter/` and `claudine/` pass. State in
      the final report that L2 and L3 were not run and why (the spec requires no
      live or focus-changing test).
    - Report as ready for review. Do not commit, do not move the spec to
      `_completed`, and do not run `just complete`.
