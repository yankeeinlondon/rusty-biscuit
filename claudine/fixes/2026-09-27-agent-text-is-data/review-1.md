---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-27-agent-text-is-data/spec.md`
fix: 2026-09-27-agent-text-is-data/review-1.md
spec: 2026-09-27-agent-text-is-data/spec.md
reviewed_by: codex/gpt-6-sol
created: 2026-09-28T06:51:15-07:00
implemented: false
ready: false
findings:
    - "high: Malformed structured YAML is silently converted into text"
human_review: false
recurrence: false
---

# Review 1: Agent-Produced Text Is Data

## Assessment

**Not production ready.** The main data-origin paths have process-level tests and the literal-token encoder has a Unicode property test. One YAML repair class contradicts the spec: malformed quoted values and collections are accepted as text. That can silently change an agent's intended frontmatter type, and the successful inline run stamps the changed document.

The review covered the Darkmatter composition and token paths, Claudine runtime overlays, inline closure, lifecycle writes, sequence readers, documentation, and test placement. The input robustness matrix applies to agent-written YAML frontmatter. Its control, absent, null, wrong-type, empty, duplicate-key, and invalid-document shapes are represented in the existing repair and closure tests. The malformed quoted and collection shapes below are the exception.

## Finding

### Malformed structured YAML is silently converted into text (high)

**Defect class:** The inline YAML repairer treats a value that starts as a quoted scalar or flow collection as plain text when its YAML is malformed.

The spec permits repair only for an **unquoted, single-line scalar unambiguously meant as text**. It explicitly excludes quoted scalars and collections. In Claudine, [`needs_quoting`](../../lib/src/composition/closure/persist.rs) instead returns `true` for a value beginning with `"`, `'`, `[`, or `{` whenever parsing fails. [`repair_agent_frontmatter`](../../lib/src/composition/closure/persist.rs) then wraps that entire value in double quotes. The existing `repair_walks_every_value_shape_from_one_fixture` test even expects `broken: "half" quoted` to become a string, so this is an asserted behavior rather than an untested corner.

I copied the inline command's real fixture shape: frontmatter with `prompt` and `title`, followed by `old body`. A fake Goose agent added one `added:` line and changed the body. Each row ran the shipped `claudine inline-compose --goose <document>` command with a fresh fixture. The plain-text repair passed; the library matrix supplies an unedited control. The table sweeps every branch that handles this input class; each edit changed only the `added:` value.

| Site / input shape | Observed result | Expected result |
| --- | --- | --- |
| Double-quoted scalar: `added: "half" quoted` | Exit 0; saved `added: "\"half\" quoted"` | Reject malformed quoted YAML, report its line, restore the original document |
| Single-quoted scalar: `added: 'half' quoted` | Exit 0; saved `added: "'half' quoted"` | Reject and restore |
| Flow sequence: `added: [a, b` | Exit 0; saved `added: "[a, b"` | Reject malformed collection and restore |
| Flow mapping: `added: {k: v` | Exit 0; saved `added: "{k: v"` | Reject malformed collection and restore |
| Plain text: `added: Fix: colons` | Exit 0; saved `added: "Fix: colons"` | Repair as text; clean |
| Valid quoted scalar and valid flow sequence/map | Existing fixture matrix leaves them unchanged | Leave unchanged; clean |

The same `needs_quoting` branch handles all four malformed shapes. Keep the narrow plain-scalar repair, let malformed quoted or collection syntax reach the existing YAML rejection path, and update both the matrix and CLI rollback test to assert the line, agent attribution, and unchanged original bytes. The relevant existing test is declared in the Claudine library target and selected by the normal L1 tier; I ran `just test repair_walks_every_value_shape_from_one_fixture` from `claudine/` and it passed (1 test). Its passing expectation currently locks in the defect. I also ran `just test agent_text_is_data` (27 passed); that process-level suite lacks these four malformed shapes.

## Verification level by user-visible requirement

| Requirement | Strongest evidence reviewed | Assessment |
| --- | --- | --- |
| Composed frontmatter, body, transclusion, authored shell approval, literal tokens, and diagnostics | Darkmatter library and CLI Level 1 tests, including `data_origin`, `literal_token`, and `compose_value_provenance` | Appropriate: these are deterministic text and command decisions; no terminal encoder behavior is involved. |
| Loop and sequence output remains raw in prompts, predicates, nested arrays, and task overlays | Claudine CLI Level 1 fake-provider tests in `agent_text_is_data` | Appropriate for the observable prompt bytes. |
| Inline repair, token persistence, decoded schema validation, hash, second run, and rollback | Claudine CLI Level 1 fake-provider tests plus closure unit tests | The malformed structured-YAML cases above are missing and currently behave incorrectly. |
| Lifecycle messages, `set:`, `proxy.with:`, and frontmatter effect writes | Claudine CLI Level 1 fake-provider tests and persistence unit tests | Appropriate for emitted bytes and stored values. |

The new CLI tests are declared by `claudine/cli/tests/l1/main.rs` and the library tests by their existing test target. Their names have no marker that removes them from L1. The feature does not assert terminal rendering or physical keyboard behavior, so the Level 2 and Level 3 terminal requirements do not apply.
