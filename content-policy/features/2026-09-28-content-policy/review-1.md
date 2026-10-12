---
$schema: feature-review.yaml
description: A **feature** review of `2026-09-28-content-policy/spec.md`
feature: 2026-09-28-content-policy/review-1.md
spec: 2026-09-28-content-policy/spec.md
created: 2026-09-29T04:07:52-07:00
reviewed_by: codex/gpt-6-sol
implemented: false
ready: false
findings:
  - title: Ordinary documents bypass content-policy editor validation
    priority: high
  - title: Content edits do not consistently prove a UTC baseline update
    priority: high
  - title: Styled terminal tables lack real-terminal verification
    priority: high
human_review: true
human_review_items:
  - |-
    Decide how `md schema validate` should check a Markdown document that has no `$schema` declaration. The feature requires such a document's `content_policy` rules to be checked, but the command currently reports that no schema applies. Please choose one behavior after the dependent schema work is available:

    - Make `md schema validate` apply Darkmatter's base document schema by default, so it checks `content_policy` in ordinary documents.
    - Keep the command limited to explicitly selected schemas and revise the feature requirement; require an explicit schema flag for this command while the editor still uses the base schema.

    The review recommends the first option because it matches the written acceptance criterion and gives the command and editor the same result.
has_blocked_findings: true
blocked: false
recurrence: false
---

# Content Policy implementation review

The feature is **not production ready**. The policy library and CLI pass their local Level 1 suite, but two user-visible parts of the specification remain incomplete, and the styled terminal presentation is verified only as output bytes. This review records the current behavior; it does not change implementation code.

## Findings

### High: Ordinary documents bypass content-policy editor validation

**Defect class:** Schema validation is exercised on isolated policy entries, while the ordinary Markdown document paths do not attach the content-policy type to the document's `content_policy` list.

I copied the real stamped-note fixture, changed only its rule to `Duration(3mo)` or its action to `delete`, and ran the built `md schema validate --format json` command. All three documents returned exit 0 with `"schema": null`, `"valid": true`, and no problems. The package's schema file itself is tested through Darkmatter's validator, but those tests attach an explicit schema to individual `p0`, `p1`, and similar properties; they do not exercise an ordinary `content_policy` list. [Darkmatter's base schema](../../../darkmatter/docs/schemas/darkmatter.yaml) has no `content_policy` property, and [the editor schema tests](../../../darkmatter/lib/tests/l1/content_policy_editor_schema.rs) document the current `policy[]` limitation.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `md schema validate`, ordinary document | Unedited stamped-note fixture; then one rule changed to `Duration(3mo)` | All three return `schema: null`, valid, no problems | The control is valid under the base schema; the unknown rule is flagged |
| `md schema validate`, ordinary document | One list entry changed to `{rule: ValidFor(3mo), action: delete}` | Valid, no problems | The unknown action is flagged |
| Darkmatter base schema and editor path | Same `content_policy` list without a document `$schema` | The base schema has no policy type, so the editor cannot check the list | The editor checks compact and long entries in an ordinary document |
| Explicit per-entry schema through Darkmatter's validator | Compact entry, long entry, unknown rule, unknown action | Existing Level 1 tests flag the invalid entries correctly | Clean for the narrower, explicitly typed entry path |

**Impact and resolution:** An author can write a policy the CLI rejects while schema validation and editor feedback say nothing. The dependent recursive-schema implementation must make a list of union-typed policy entries usable, then this feature must connect the type in the base schema and test both `md schema validate` and Darkmatter's language server with an ordinary document. The command's behavior without `$schema` also needs the human decision described in the frontmatter. This finding is **blocked** on that dependency and decision; the current implementation cannot meet the specification's ordinary-document editor validation requirement by changing this package alone.

### High: Content edits do not consistently prove a UTC baseline update

**Defect class:** The three content-writing paths that promise to renew `last_updated` have different behavior and incomplete path-level UTC tests.

The spec says each writer advances the `last_updated` baseline when it edits content, with a test at an instant when a local date is ahead of UTC. [Darkmatter's effect writer](../../../darkmatter/lib/src/effects/verbs.rs) calls `plan_hash_save(None, ...)` even when the document has a stored `hash`. The `None` branch of [that decision function](../../../darkmatter/lib/src/markdown/hash/save.rs) always sets `bump_last_updated` to `false`. I copied its existing public `set_frontmatter` test, added `last_updated: 2020-01-01` to the fixture and an assertion for an advanced date, and ran that one test with nextest. The assertion failed: the written document had a new hash and `status: in-progress`, but still had `last_updated: 2020-01-01`. I restored the temporary test edit afterward.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `md hash --save` | Stored stale hash and old date, injected `2026-09-28T23:30:00Z` | Existing command-path test writes `2026-09-28`; clean | UTC date written |
| Darkmatter effect `set_frontmatter` with auto-rehash | Stored hash, old date, and a content edit | Focused nextest reproduction updates `hash` and `status` but leaves `last_updated: 2020-01-01` | The same write advances `last_updated` to the UTC date |
| Claudine inline closure write-back | Accepted content edit with an injected date | An existing path test writes the injected date, but does not exercise an instant whose local date differs from UTC | A path-level test at the UTC boundary proves the persisted date is UTC |

**Impact and resolution:** A document whose content an effect changes can remain stale under `ValidFor(..., @last_updated)`, contrary to the feature's renewal contract. Feed the effect writer the actual stored hash, preserve its existing hash comparison behavior, and assert both the hash and date in a public-path test. Add a UTC-boundary test through Claudine's write-back path. The shared helper's UTC test alone cannot prove what Claudine persists. This closes acceptance criterion 20 for every writer, including the one that currently fails.

### High: Styled terminal tables lack real-terminal verification

**Defect class:** The visual `policy` output is checked as strings from a spawned process, but no test observes the rendered tables or styling inside a terminal emulator.

The specification promises terminal-formatted reports and renewal previews. [The CLI renderer](../../../content-policy/cli/src/output.rs) emits styled summary text and box-drawing tables. The existing binary tests assert ANSI bytes, Unicode characters, and at most 80 Rust `char`s per table line. Those Level 1 assertions do not establish the display-cell width, wrapping, or colors a terminal actually shows. The package's `test-l2` recipe is a "not applicable" stub, so no Level 2 test is selected.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `policy check` styled report | Time rule and long `FileChanged` rule | Level 1 checks styled bytes and a plain table's character count; no rendered-pane capture | Level 2 capture checks readable rows, display width, and status styling in a real terminal |
| `policy renew` styled preview | Date change and long fingerprint | Level 1 checks preview text and plain table characters; no rendered-pane capture | Level 2 capture checks table layout and preview styling in a real terminal |
| `--plain`, `--json`, `--needs-action` | Text or machine-readable output | Level 1 byte and JSON assertions exist | Clean at Level 1; these modes do not promise styled terminal rendering |

**Impact and resolution:** A terminal may wrap a table row or show unexpected styling even when the raw output test passes, especially for wide characters in a document path or rule. Add a real-terminal test for each styled table shape using the repository's terminal test harness, with no focus change to the user's window; make `test-l2` a live recipe so the test actually runs. Keep the current Level 1 tests for exact text and JSON behavior.

## Coverage and verification

- `just test` in `content-policy/`: **167 passed, 0 skipped**. The YAML frontmatter, fingerprint, and serialized-policy input matrices use a real fixture and exercise absent, null, wrong type, empty, duplicate, and trailing or invalid shapes through public evaluation or deserialization results. I found no additional failure in those matrix cells. The focused Darkmatter effect-writer reproduction failed as described above; the temporary assertion was removed after the run.
- Policy evaluation, renewal, `policy check`, and `policy renew` are exercised at Level 1, which is sufficient for their file, JSON, and command results. The styled table requirement needs Level 2 rendered-pane evidence. The editor requirement needs a document-level validator and language-server test that currently does not exist. The writer requirement needs a test through each actual write path; the effect path fails, and Claudine lacks the UTC-boundary case.
- The missing base-schema connection is already marked **planned** in the current topic documentation. The Darkmatter effect-writer discrepancy is also described there; the code is the basis for this finding, so the review does not treat the outdated promise in the spec as current behavior.
