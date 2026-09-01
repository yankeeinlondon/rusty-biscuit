---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-01T19:54:22+01:00
spec: /Users/ken/.claudine/worktrees/rusty-biscuit/feat-unifi/claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md
implemented: false
description: "A **fix** review of `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-unifi/claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md`"
fix: 2026-09-01-inline-compose-frontmatter/review-1.md
---

# Review 1: Inline Compose Frontmatter

## Verdict

Not ready for production. The implementation addresses the principal serialization and response-frontmatter design, and the relevant tests exercised during this review pass. However, source drift is not reported accurately in several cases required by AC11, and the exact byte-preservation matrix required by AC1 and AC5 is not fully verified.

## Findings

### High: Added or structurally invalid frontmatter drift is restored silently, while frontmatter-only shape drift can be reported as body drift

`detect_source_drift` compares only keys present in the original mapping (`claudine/lib/src/composition/closure.rs:558-565`). A property added to the file while the provider is running is removed when the original snapshot is restored, but it never appears in `restored_frontmatter_properties`. If either frontmatter document cannot be parsed as a mapping, the function returns no restored property notices (`claudine/lib/src/composition/closure.rs:545-557`). Both paths violate AC11's requirement to report any source drift and each value-drifted property that is restored.

The fallback body comparison also uses the entire document whenever only one side has a recognized frontmatter block (`claudine/lib/src/composition/closure.rs:539-542`). Adding, removing, or damaging only the frontmatter delimiters can therefore set `body_drift_restored` even when the body bytes did not change. The CLI would then issue a misleading body-restoration status.

Compare the union of original and current property names, with explicit handling for additions and removals. When mapping-level comparison is impossible, return a truthful generic frontmatter-drift result rather than silently claiming no frontmatter drift. Derive body drift from each document's body region independently so frontmatter shape changes cannot masquerade as body edits. Add tests for an added property, a removed property, malformed/non-mapping frontmatter, frontmatter delimiter changes with an unchanged body, and the resulting CLI notices.

### High: The required generic `md hash --save` preservation matrix is incomplete

AC5 explicitly requires LF and CRLF fixtures containing trailing-space multiline content for default Simple, longhand Structured/Detailed, custom `HASH_PROPERTY`, quoted semantic keys, and the unsupported-flow no-write path. The current integration coverage provides one CRLF Simple multiline fixture, one LF custom-property fixture without multiline content, and one LF Detailed fixture without multiline content (`darkmatter/cli/tests/hash_kind_save_diff.rs:210-294`). There is no Structured CLI fixture and no cross-product that exercises both newline conventions and trailing-space block scalars for each representation.

The library tests similarly cover LF Simple and CRLF replacement of a longhand node with a Body hash (`darkmatter/lib/src/markdown/hash/write.rs:598-660`), but do not lock down Structured and Detailed output under both newline conventions. Since the generic writer is shared beyond Claudine, these omissions leave the central byte-preservation contract insufficiently protected. Add table-driven library tests and CLI integration fixtures for the complete AC5 matrix, asserting exact unchanged prefix/suffix bytes and successful `--diff` after each save.

### Medium: Several acceptance checks are represented only by weaker proxy assertions

The main closure preservation test checks selected substrings instead of proving that every non-managed frontmatter byte is identical as AC1 requires (`claudine/lib/src/composition/closure/tests.rs:162-198`). Its idempotence coverage does not repeat the exact AC1 multiline fixture. There is also no explicit authorization-removal case for AC7, the failing legacy guardrail migration test does not assert the required warning from AC9, and AC11's user-facing status text is not integration-tested.

Strengthen these tests with an exact expected document assembled from the original frontmatter plus only the permitted `hash` and `last_updated` node replacements. Reuse that fixture for the second-run idempotence assertion, and add the missing authorization-removal, migration-warning, and drift-status cases.

## Requirement Verification Levels

All requirements in this fix concern file transformation, provider-output parsing, or process-level CLI behavior. Level 1 is the appropriate verification level; no requirement depends on terminal-emulator rendering/input encoding or physical keyboard events, so Level 2 and Level 3 are not warranted.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| AC1 authored-byte preservation | Level 1 unit | Partial: substring checks do not establish exact preservation. |
| AC2 no escaped prompt regression | Level 1 unit | Appropriate. |
| AC3 hash consistency/idempotence | Level 1 unit and CLI process | Partial: not exercised with the exact AC1 fixture. |
| AC4 Structured-to-Simple downgrade | Level 1 unit | Appropriate semantically; exact surrounding bytes depend on the AC1 gap. |
| AC5 generic textual hash writer | Level 1 unit and CLI process | Gap: required representation/newline fixture matrix is incomplete. |
| AC6 authorized response harvest | Level 1 unit and provider-stub CLI process | Appropriate. |
| AC7 authority and immutability | Level 1 unit | Partial: authorization removal is not explicit. |
| AC8 invalid harvest rejection | Level 1 unit | Appropriate. |
| AC9 guardrail migration | Level 1 unit | Partial: failure atomicity is covered, warning emission is not. |
| AC10 end-to-end inline compose | Level 1 provider-stub CLI process | Appropriate; a real terminal adds no relevant signal. |
| AC11 source drift restoration/reporting | Level 1 unit | Gap: implementation and cases are incomplete as described above. |
| AC12 unchanged-body rejection | Level 1 unit | Appropriate. |

## Validation

- `darkmatter/just test`: passed (7,550 passed, 50 skipped).
- `darkmatter/just lint`: passed.
- `claudine/just lint`: passed, including error guards and all five crate lint passes.
- `claudine/just test`: the relevant new closure, preparation, guardrail, and CLI tests passed, but the package run did not complete: `shipped_prompt_contract` failed on the unrelated untracked `prompts/fixes/2026-09-01-file-param-anchoring/plan.md`, whose body contains an invalid `{{ ... }}` expression. At cancellation, nextest reported 4,587 passed, 1 failed, 11 skipped, and 2,049 not run. This environmental failure is not attributed to this fix, but it prevents claiming a completely green package suite from the current worktree.

## Positive Notes

The textual hash writer correctly avoids whole-frontmatter serialization, preserves semantic quoted-key matching, rejects flow mappings without writing, and uses the shared `FileReference` path. Response properties are allowlisted before composition, managed properties remain protected, provider response parsing accepts the sanctioned frontmatter channel, and terminal output uses `TerminalRenderable` status components. The implementation is directionally strong once the drift semantics and acceptance evidence above are completed.
