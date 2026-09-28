---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-27-agent-text-is-data/spec.md`
fix: 2026-09-27-agent-text-is-data/review-2.md
spec: 2026-09-27-agent-text-is-data/spec.md
previous: 2026-09-27-agent-text-is-data/review-1.md
reviewed_by: codex/gpt-6-sol
created: 2026-09-28T15:40:26-07:00
implemented: false
ready: true
human_review: false
recurrence: false
---

# Review 2: Agent-Produced Text Is Data

## Assessment

**Production ready.** The single unblocked finding from review 1 is fixed. Review 1 had no blocked findings. This pass found no new finding and no recurring defect class.

The review checked the inline repair and persistence path, Darkmatter's literal-token decoding and instruction scanning, Claudine's runtime output projections and frontmatter readers, lifecycle and sequence tests, documentation, and test placement. The code now keeps captured output raw in memory, stores agent-written instruction-looking frontmatter as tokens, decodes those tokens for consumers, and refuses malformed structured YAML without saving a partial edit.

## Review 1 finding: malformed structured YAML

**Defect class checked:** Agent-written YAML that starts a quoted scalar or collection must not be reinterpreted as plain text when malformed. The inline closure's [`repair_agent_frontmatter`](../../lib/src/composition/closure/persist.rs) is the only path that repairs agent-written source text. Lifecycle frontmatter effects and Darkmatter's hash writer serialize or restore already typed values; they do not repair source YAML.

The instance sweep used the same real inline document shape for every row: an authored `prompt` and `title`, an agent-added `added:` value, and a changed body. The shipped `claudine inline-compose --goose` command exercised all four formerly failing shapes and the plain-text control. The Claudine library's one-fixture matrix covers the remaining YAML value forms through the public repair result.

| Site and shape tested | Observed result | Expected result |
| --- | --- | --- |
| Inline closure, `added: "half" quoted` | Rejected on line 4, attributed to the agent; original document restored | Same; clean |
| Inline closure, `added: 'half' quoted` | Rejected on line 4, attributed to the agent; original document restored | Same; clean |
| Inline closure, `added: [a, b` | Rejected on line 4, attributed to the agent; original document restored | Same; clean |
| Inline closure, `added: {k: v` | Rejected on line 4, attributed to the agent; original document restored | Same; clean |
| Inline closure, `added: Fix: colons` | Saved as a quoted string | Same; clean |
| Library repair matrix, valid quotes, collections, block scalars, anchors, aliases, tags, comments, and typed scalars | Kept as their YAML values | Same; clean |
| Library repair matrix, malformed alias, tag, block scalar, sequence entry, complex key, mapping value, and nested value | Rejected with agent attribution | Same; clean |
| Lifecycle frontmatter effects and Darkmatter hash writer, typed values | Serialize or restore values without invoking source-text repair | Same; clean |

The new [`malformed_structured_agent_values_are_refused_not_saved_as_text`](../../cli/tests/l1/agent_text_is_data.rs) test asserts the shipped command's exit status, line, agent attribution, and byte-for-byte rollback. The [`repair_walks_every_value_shape_from_one_fixture`](../../lib/src/composition/closure/persist/tests.rs) matrix has an unedited control and changes one value at a time. Its `added` field rows distinguish empty, null, numeric, boolean, string, list, map, and invalid YAML values; separate tests cover duplicate keys and missing delimiters. Malformed structured values now follow the invalid-document outcome instead of becoming strings.

## Verification level by user-visible requirement

| Requirement | Strongest verified level | Assessment |
| --- | --- | --- |
| Composed frontmatter, body, transclusion, authored shell approval, literal-token decoding, and source diagnostics | Level 1 Darkmatter library and CLI tests | Appropriate for deterministic text and command decisions. |
| Loop and sequence output stays raw in prompts, predicates, nested arrays, and task overlays | Level 1 Claudine fake-provider process tests | Appropriate for the observable prompt bytes. |
| Inline repair, token persistence, decoded schema validation, hash, second run, and rollback | Level 1 Claudine process tests and library tests | Appropriate; the malformed structured YAML cases now run through the shipped CLI. |
| Lifecycle messages, `set:`, `proxy.with:`, and frontmatter effect writes | Level 1 Claudine process and library tests | Appropriate for emitted bytes and stored values. |

These requirements do not depend on terminal rendering or a terminal's keyboard encoder, so Level 2 and Level 3 testing is not applicable. The new process test is declared by `claudine/cli/tests/l1/main.rs`; the repair tests are in the Claudine library target. Their names keep them in Level 1, and `just check-tier-coverage claudine` reported no stranded tests.

## Checks run

- `just test agent_text_is_data` from `claudine/`: 28 passed.
- `just test repair_walks_every_value_shape_from_one_fixture` from `claudine/`: 1 passed.
- `just check-tier-coverage claudine` from the repository root: no stranded tests.

The implementation log records a passing full `just test` run and `just lint` run for the previous fix implementation. This review did not rerun those broad checks.
