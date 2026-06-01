---
ready: false
agent: codex
model: ""
---

# Review: Schema Support in Claudine

## Verdict

Not ready for production. Iteration 5 fixes the two iteration-4 findings: path-qualified and negated `file(match(...))` completions now use relative-path glob matching, and the direct `compose` interactive status report no longer marks template-bearing provider values as invalid.

The remaining gaps are both in the `sequence` path. Non-interactive sequence errors for unsupported missing shapes no longer follow the spec's aggregate-by-step contract, and Interactive Mode for sequence has no PTY coverage despite being a user-observable prompt flow. The status report shown before sequence prompting also does not use the per-step effective frontmatter, so it can display false missing/invalid statuses for values supplied by CLI setters or sequence overlays.

## Findings

### High: sequence Interactive Mode is unverified at the required PTY level

- Requirement: `claudine sequence` must collect missing required values before any provider session starts, dedupe the same missing property across steps, and reuse the answer for later steps unless a step overlay supplies a different value.
- Verification present: Level 1 process coverage exists for non-interactive aggregation (`sequence_aggregates_missing_required_properties_across_steps`) and setter-supplied success (`sequence_set_override_satisfies_required_schema`) in `claudine/cli/tests/sequence_cli.rs`. The PTY prompt tests in `claudine/cli/tests/level2_schema_prompt_pty.rs` exercise direct `compose` only.
- Required level: Level 1 PTY is the minimum for this behavior because the feature depends on stdin/stderr TTY detection, `biscuit-tui` prompts, entered bytes, cancellation/submit behavior, and post-collection provider launch. The current sequence tests do not enter the prompt path at all.
- Impact: a regression in sequence-specific prompting, deduplication, retry, or "prompt before provider launch" behavior would not be caught. This is not production-ready under the review rubric because a user-observable prompt requirement has no appropriate-level test.
- Fix direction: add PTY tests for `claudine sequence` that cover at least one missing string value across two steps, answer reuse/deduplication, and provider launch only after collection. Add a second case where a step overlay supplies the value for one step so only the genuinely missing step participates.

### Medium: non-interactive sequence unsupported shapes bypass aggregated MissingProperties

- Requirement: when Interactive Mode is not allowed, sequence must return aggregated `MissingProperties` errors by step so the user can fix the full sequence in one edit. Unsupported interactive shapes should not force a TUI path in non-TTY fallback.
- Implementation: `run_phase_1c_with_schema` promotes the first unsupported missing shape to `UnsupportedInteractiveSchema` before it checks whether prompting is allowed at `claudine/cli/src/commands/wrap/sequence.rs:713`. Direct `compose` does the inverse: it returns `MissingProperties` first when `interactive.allowed()` is false at `claudine/cli/src/commands/schema_interactive.rs:210`.
- Evidence: a non-TTY `sequence --goose seq.md` with `$schema.config: object(required)` exits with `unsupported interactive schema`, not an aggregated `sequence missing properties` report. The existing test only asserts that `config` appears in stderr, so it does not catch the wrong error class.
- Impact: users lose the sequence-specific per-step aggregation for unsupported shapes, and sequence behavior diverges from direct compose for the same non-interactive missing property.
- Fix direction: move the unsupported-shape promotion after `interactive.allowed()` in the sequence path, matching direct compose. Strengthen the non-TTY test to assert `sequence missing properties`, both step labels when applicable, and absence of `unsupported interactive schema`.

### Medium: sequence prompt status report ignores setters and step overlays

- Requirement: the status report shown before Interactive Mode should reflect the schema status of the effective composition run: caller setters apply to every step, and reserved per-step overlay values win.
- Implementation: `collect_sequence_missing_values` re-resolves the source file and calls `build_schema_status_report(&source, None)` at `claudine/cli/src/commands/wrap/sequence.rs:946`, discarding both user `key=value` / `--set` overrides and the per-step overlay that `run_phase_1c_attempt` validated with.
- Reproduction shape: a sequence schema requiring `state` and `topic`, with `sequence: [alpha]`, will correctly treat `state` as supplied by the step overlay and prompt only for `topic`. The pre-prompt status report is rebuilt from raw frontmatter with no overlay and can still show `state` as missing. The same false status happens for a required value supplied by a CLI setter while another property remains missing.
- Impact: users can see an inaccurate diagnostic immediately before the prompt. This is the sequence analogue of the direct-compose status drift fixed in iteration 5.
- Fix direction: carry the per-step effective override map into `SequenceMissingPropertiesStep` or into the status-rendering loop, then call `build_schema_status_report` with those overrides. Add a PTY assertion that a setter-supplied or overlay-supplied required property is not reported as missing while prompting for a different property.

## Test Rigor Classification

- Direct `compose` missing, invalid required, invalid optional drop, setter-supplied required, and templated required enum: Level 1 process coverage present.
- Direct `compose` Interactive Mode for string, enum, boolean, numeric retry, `--silent`, and templated status-report regression: Level 1 PTY coverage present; appropriate for this prompt-input behavior.
- `inline-compose` prompt-property precedence and missing schema values: Level 1 process coverage present.
- `sequence` non-interactive missing aggregation and setter-supplied success: Level 1 process coverage present.
- `sequence` Interactive Mode prompt/dedupe/retry: no PTY coverage present; this is a high verification gap.
- Schema setter completion: Level 1 `__complete` coverage present for required ordering, supplied-property filtering, enum values, `inline-compose`, `sequence`, basename `file(match(...))`, path-qualified globs, and negated path-qualified globs.
- Raw JSON Schema non-interactive missing path: manually spot-checked through the CLI; typed `MissingProperties` is emitted with pointer/name fallback metadata.

## Verification Run

- `cargo test --color=never -p claudine-cli --test compose_schema_cli completion_file_match -- --nocapture`
- `cargo test --color=never -p claudine-cli --test sequence_cli sequence_unsupported_shape_surfaces_typed_error_under_tty_pref -- --nocapture`
- Manual non-TTY CLI repro for raw JSON Schema missing required.
- Manual non-TTY CLI repro for sequence `object(required)` showing `UnsupportedInteractiveSchema` instead of aggregated sequence missing properties.
