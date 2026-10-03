---
fix: 2026-09-27-agent-text-is-data
review: claudine/fixes/2026-09-27-agent-text-is-data/review-1.md
implementation_1: "2026-09-28T15:30:24-07:00"
---

# Agent-Produced Text Is Data: Implementation Log

## Implementation of Review Findings #1

> **started at:** 2026-09-28T15:30:24-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/fixes/2026-09-27-agent-text-is-data/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Malformed structured YAML is silently converted into text' at 15:30:40
        - defect class: the inline YAML repairer treated an agent value whose leading indicator commits it to a structured YAML form (quoted or block scalar, flow collection, anchor, alias, tag, comment, `- `/`? `/`: `) as plain text, quoting it instead of letting a malformed one be rejected
        - class sweep: agent-written YAML values quoted as text; sites checked: `closure/persist.rs` `needs_quoting` / `repair_agent_frontmatter` / `reparse`, `closure::persisted_data` (lifecycle frontmatter writes), `composition/guardrails.rs` plain-scalar rule text, darkmatter `markdown/hash/write.rs` `restore_properties_text`, darkmatter `literal_token::encode_yaml_scalar`, darkmatter `schemas/simplified/query.rs` `yaml_double_quote`, dmls `yaml_scalar_literal`, biscuit-file `yaml/analyze/recover.rs`; fixed here: `needs_quoting` (every indicator branch) and the rejection line in `repair_agent_frontmatter`; clean: `persisted_data`, `restore_properties_text`, `encode_yaml_scalar`, `yaml_double_quote`, `yaml_scalar_literal` (all serialize a known value, never reinterpret malformed input), guardrails text (still accurate for plain values), biscuit-file `recover` (not reachable from the inline closure; its own proof requires the quoted string to equal the lexeme)
        - discovery: the old `& * !` branch also rewrote *valid* structure into text (`anchor: &a text` became the string `"&a text"`, an alias to a real anchor became `"*b"`), and `added: # todo: later` (a comment) became a string because the comment contained `: `
        - discovery: the whole-document reparse locates an unclosed flow collection at the line *after* it (the closing `---`) and a custom tag (`!important note`) with no line at all, so relying on it alone would not name the agent's line
        - decision: reserved indicators `%`, `@`, and backtick keep being quoted, since no YAML form starts with one and the value can only be text; `,`, `]`, `}` keep their existing plain-text handling
        - change: new `opens_structure` (YAML 1.2 c-indicators that open a form; `-`/`?`/`:` only before a space or end) makes `needs_quoting` return false; new `malformed_structure` judges each changed single-line structured value alone (same parse + JSON conversion as the reparse) and rejects it with its own line, the key, the form it started, and `agent_edit`; aliases are left to the whole-document reparse because their anchor lives elsewhere
        - change: docs updated — `claudine/docs/topics/composition.md` (Repair bullet), `.claude/skills/claudine/SKILL.md` (Inline persistence row), `.claude/skills/claudine/architecture.md` (repair bullet); guardrails text unchanged
        - tests: rewrote `closure::persist::tests::repair_walks_every_value_shape_from_one_fixture` as a three-outcome matrix (Quoted / Unchanged / Rejected) with the unedited-fixture control row, 38 rows covering every indicator (4 review shapes, `*`, `!`, `- `, `? `, `: `, `|`, `>`, anchor with malformed tail, nested) plus valid anchor, tag, comment, `-5 apples`, `@`, `%`
        - tests: added `malformed_quoted_and_flow_values_are_rejected_not_quoted` (line, property, form, agent) and `an_alias_to_an_anchor_in_the_document_is_left_alone` in the same file
        - tests: added CLI L1 `agent_text_is_data::malformed_structured_agent_values_are_refused_not_saved_as_text` (four shapes through `inline-compose --goose` with a whole-file fake agent: non-zero exit, `(line 4, `added`)`, "The agent wrote this line", original bytes unchanged; plus a `added: Fix: colons` control that still repairs)
        - results: `just lint` passed; `just test` passed (7726 run, 7726 passed, 9 skipped); two earlier full runs each had a different unrelated L1 failure (`sequence_initialize_include_preflight`, `compose_caller_file_provenance`) that passed in isolation and on the third full run, consistent with concurrent edits in the worktree
        - orchestrator verification: re-ran `repair_walks_every_value_shape_from_one_fixture` and `agent_text_is_data::malformed_structured_agent_values_are_refused_not_saved_as_text`; both passed
- work completed for 'Malformed structured YAML is silently converted into text' at 15:40:15

### Successful Completion

The implementation of review cycle 1 has completed successfully in 11 minutes. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- note for the reviewer: values starting with the YAML reserved indicators `%`, `@`, or a backtick are still repaired as text, because no YAML form starts with them and such a value can only be text; every other structure-opening indicator is now rejected when malformed and left unchanged when valid
