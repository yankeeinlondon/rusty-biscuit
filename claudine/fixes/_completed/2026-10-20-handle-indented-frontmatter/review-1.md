---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-20T18:54:56-07:00"
spec: "2026-10-20-handle-indented-frontmatter/spec.md"
fix: "2026-10-20-handle-indented-frontmatter/review-1.md"
implemented: true
implemented_by: "claude/opus"
description: "A **fix** review of `2026-10-20-handle-indented-frontmatter/spec.md`"
ready: true
findings: []
observations:
  - "Existing formatting guide incorrectly excludes tab recovery"
human_in_the_loop: null
human-in-the-loop: false
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
recurrence: false
---

# Review 1: Handle indented frontmatter

The fix is production ready under the specification's contract. No high,
medium, or low findings were identified. This is the first implementation
review, so there are no previous findings to reconcile.

## Decisions and deferred work

Two independent reviewers examined the specification and implementation log
before the implementation review began. Neither identified an unresolved
design decision or deferred implementation requiring human input. All four
implementation phases are complete.

The implementation made these routine choices:

- The YAML analyzer proves all of a root's prefix removals together. It keeps
  indentation inside multiline flow collections and quoted values to avoid
  conflicting edits; the final combined repair must still preserve the parsed
  value. This is a narrow variation from the spec's instruction to dedent each
  qualifying line, with no loss of the required root normalization or safety.
- A hash save with nothing to write checks for root indentation while retaining
  its existing behavior for other unsupported source forms. Saves that write
  still use the full shared validation.
- The new preparation error reuses the existing `composition.failed` code,
  retains its typed Darkmatter cause, and renders the document path. Rehearsing
  the real stamp also catches a malformed stored hash before launch and can
  report an uneditable baseline before a missing prompt.
- Claudine's recovery acceptance test calls Darkmatter's cleanup transaction
  in-process because a sibling `md` executable is unavailable in an archived
  test run. Darkmatter separately exercises the actual `md clean --save` and
  `md hash --save` commands with persisted file reads.

Preserving an agent's candidate after a failed closure remains explicitly
outside the specification. No additional approval is needed for that settled
scope decision.

## Findings

None.

## Contract and verification

The review covered both committed implementation changes and the remaining
working-tree acceptance tests and documentation. The source comparison starts
at the parent of `b12737aa1`, the first implementation commit.

| Required behavior | Implementation and verification | Level |
| --- | --- | --- |
| Normalize an ordinary indented root without changing its parsed value | biscuit-file's [root repair](../../../biscuit-file/lib/src/yaml/analyze/engine.rs) groups prefix edits and reparses the combined result. Its [tests](../../../biscuit-file/lib/src/yaml/analyze/tests/root_indentation.rs) exercise block scalars, blank scalar lines, explicit indentation indicators, quoted values, comments, nested flow values, line endings, overlapping whitespace repairs, excluded roots, and tab recovery. | L1, public analyzer result |
| Deliver the repair through cleanup and language-server formatting, with a stable second pass | Darkmatter's [cleanup tests](../../../darkmatter/lib/tests/l1/clean_frontmatter_repair.rs) assert values and applied edits; DMLS's [formatting transactions](../../../darkmatter/dmls/tests/l1/formatting_transaction.rs) apply the returned edit and verify that a second request offers none, including after tab recovery. | L1, public cleanup and language-server protocol |
| Refuse indented roots before text editing, including no-op hash saves and either restore input | Darkmatter's [editor validation and tests](../../../darkmatter/lib/src/markdown/hash/write.rs) cover hash saves, restoration, and leaf location. The [public restore tests](../../../darkmatter/lib/tests/l1/inline_document_text.rs) verify typed errors. Controls retain column-0, empty, and comment-only behavior and existing errors for unsupported roots. | L1, public editor results |
| Make `md hash --save` refuse without changing the file, then succeed after cleanup | The [CLI round trip](../../../darkmatter/cli/tests/l1/clean_frontmatter.rs) checks the refusal reason, unchanged bytes, cleaned values, column-0 hash, and successful hash comparison. | L1, shipped CLI and persisted file |
| Refuse the captured inline baseline before prompt commands, guardrail writes, or provider launch, including previews | Claudine's [preparation](../../../claudine/lib/src/composition/prepare.rs) calls the shared rehearsal before composition. Its [preparation tests](../../../claudine/lib/src/composition/prepare/tests.rs) check ordinary, loop-seed, and sequence-probe entry points. The [CLI acceptance tests](../../cli/tests/l1/inline_compose_indented_frontmatter.rs) use compiled recording fixtures to prove zero provider launches and no command execution, with a positive cleaned-document control. | L1, in-process preparation and shipped CLI |
| Keep closure validation and rollback when the agent introduces indentation later | Claudine's [closure tests](../../lib/src/composition/closure/tests.rs) verify refusal before persistence; the CLI acceptance test verifies automatic restoration of the original bytes after one provider launch. | L1, closure API and shipped CLI |
| Report the source path and retain a typed cause | Claudine's [error tests](../../lib/src/composition/error/tests.rs) check rendered path and reason, diagnostic details, and the typed error chain. | L1, diagnostic result and rendering |

All requirements concern parsing, file editing, command execution, or protocol
results. L1 is the appropriate verification level; no terminal input encoder,
visual rendering, or focus-dependent interaction is part of this fix.

The call-site audit followed the shared editor gate through hash saves,
restoration of both current and snapshot documents, and leaf location used by
Claudine's persistence path. Inline closure construction remains in the shared
preparation path; direct composition does not gain this check. The rehearsal
uses the same managed-property list, hash options, and known-change stamping as
the accepted closure, and discards its result without writing.

No configuration field validator was added or changed. For the changed YAML
document-shape decision, analyzer tests verify that duplicate keys, invalid
trailing content, and a second document remain invalid without a root repair.
The source review found no newly added permissive default, element filtering,
or discarded parse failure on the changed paths. A field-by-field configuration
matrix would not exercise a new decision here.

New tests are declared in compiled modules; the Claudine CLI module's
`test-fixtures` feature is enabled by its local recipe and CI metadata. Their
names remain selected by L1.

The following verification was rerun during this review on macOS:

| Area | Command | Result |
| --- | --- | --- |
| biscuit-file | `just test root_indentation` | 17 selected nextest tests passed; the recipe's additional no-default-features check also passed. |
| claudine | `just test-cli inline_compose_indented_frontmatter` | 4 passed. |
| claudine | `just test-library rehears inline_preparation inline_baseline_unstampable` | 9 passed. |
| claudine | `just test-library composition::closure::tests::` | 36 passed. |
| darkmatter | `just test indented` | 55 passed, including the CLI round trip and DMLS formatting. |
| darkmatter | `just test markdown::hash::write:: clean_frontmatter_repair::` | 89 passed. |

These selections overlap. Every selected nextest test completed within the
five-second L1 budget. `git diff --check` passed. Full-area tests and lint are
recorded in the implementation log; this review did not rerun those full gates
or claim new cross-OS evidence. No production source was edited during review.

## Observations

### Existing formatting guide incorrectly excludes tab recovery

The pre-existing [formatting guide](../../../darkmatter/docs/topics/markdown-formatting.md#frontmatter)
says tab-indented frontmatter receives no repair. Current tab recovery, the
updated cleanup reference, and the passing cleanup/formatter tests contradict
that statement. This fix did not introduce the statement or tab recovery, so
it is an observation rather than a repair request for this cycle. The author
can schedule a documentation correction separately.
