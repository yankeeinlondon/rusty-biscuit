---
$schema: feature-review.yaml
ready: true
human_review: true
human_review_items:
    - |-
        Please confirm these two rules before closing the fix:

        1. The user's `.claudine` prompt folders are searched after local files, even when the command starts in the home folder.
        2. An `@` reference inside a prompt loaded from elsewhere searches the tree where the command started. `./`, bare, `&`, and `^` references keep the prompt's own location.

        Confirm both rules, or identify which rule should change. Also confirm whether command-line tests that require a replaceable home folder may remain skipped on native Windows; Windows still runs unit tests of the ordering with an explicitly supplied home folder.
    - |-
        Please choose how the standalone `md compose` command should resolve an `@` reference inside a document outside the tree where the command started:

        1. Keep the document's tree, which is its current documented behavior.
        2. Use the command's launch tree, matching Claudine.
        3. Track the difference in a separate fix (recommended; it keeps this specification's scope intact).
reviewed_by: codex/gpt-6-sol
created: "2026-09-24T04:42:06-07:00"
spec: 2026-09-23-local-before-home/spec.md
implemented: false
description: "A **fix** review of `2026-09-23-local-before-home/spec.md`"
fix: 2026-09-23-local-before-home/review-2.md
previous: 2026-09-23-local-before-home/review-1.md
---

# Review 2: Local Before Home

## Verdict

**Production ready on implementation and test coverage.** Both findings from Review 1 are fixed. No implementation finding remains. The human decisions above are separate from the `ready` verdict, as the review instructions require.

## Review 1 Findings

### High — Repository-local prompt symlinks lose local priority: resolved

[`with_prompt_magic_roots`](../../../claudine/lib/src/composition/resolve.rs) now suppresses a local `.claudine` convention only when the local root itself is the home directory, including a different symlink spelling of that same directory. A `.claudine` path lexically inside another repository stays in the local tier even if it points to the user's `.claudine` directory. The new Level 1 [CLI regression](../../../claudine/cli/tests/l1/compose_prompt_tiers.rs) puts a file behind that symlink and a competing file under the home root, then asserts the CLI selects the symlinked local file. A [library test](../../../claudine/lib/src/composition/resolve/tests.rs) checks the registered tier and resolved path. Both passed in this review.

### Medium — `@/` no-match diagnostics show the wrong payload: resolved

[`FileReference::payload()`](../../lib/src/file_reference/mod.rs) uses the parser's recorded payload offset, removing only the recursive modifier, sigil, and optional separator. The Claudine renderer uses that value. [Grammar tests](../../lib/tests/reference_grammar.rs) cover compact, separated, recursive, and literal-leading-`@` forms. Claudine's Level 1 library test checks the miss line across those forms; its CLI test checks that `@/absent-payload-probe.md` names `absent-payload-probe.md` once. All selected tests passed.

Review 1 has no `## Blocked Findings` section, so there was no blocked item to reassess for unblocking before the implementation.

## Findings

None.

## Requirement Verification

| Requirement | Strongest verification | Assessment |
|---|---|---|
| R1/R2: launch local root and every local `@` tier precede home for direct and recursive references | Level 1: biscuit-file `magic_local_roots` plan, recursive, and first-match tests; Claudine CLI dry runs | Appropriate for file selection. Includes no-repository launch, home overlap, and the Review 1 symlink regression. |
| R3: completion follows the same ordered roots as execution | Level 1: biscuit-file completion/plan parity and Claudine completion-to-compose round trips | Appropriate for emitted completion text and selected files. |
| R4: an `@` miss names its payload once, lists ordered roots, and retains structured probes; other misses keep their report | Level 1: Claudine renderer and CLI stderr assertions, including `@/`; biscuit-file parser tests | Appropriate for the specified text and data. The spec imposes no terminal styling or glyph requirement. |
| R5: Claudine's local prompt conventions precede its user conventions in repository, plain-directory, home-overlap, and symlink layouts | Level 1: Claudine registration unit tests and CLI dry runs | Appropriate and exercises the affected command boundary. |
| Nested `@` uses the launch tree while `./`, bare, `&`, and `^` keep source anchors | Level 1: biscuit-file derived/seeded-context tests and Claudine CLI composition from user and sibling-repository prompts | Appropriate for file selection. |
| Darkmatter identities include launch scope, tier override, and package root | Level 1: identity comparison tests for both cache products | Appropriate for cache-key behavior. |
| R6: public descriptions reflect the search order | Static review of topic docs, README, and skill references | The stated local-before-home rule matches the implementation. |

These requirements concern filesystem lookup, completion output, diagnostics, and cache identity. None asserts real-terminal rendering or OS keyboard encoding, so Level 2 and Level 3 are not required for this fix.

## Test Reachability and Validation

- Biscuit-file's `reference_grammar` file is an auto-discovered integration target. Claudine CLI's `compose_prompt_tiers` module is declared by its `l1` target in `tests/l1/main.rs`; the target has no required feature, and CI enables its fixture feature. Claudine library tests are compiled in the library test target. The selected tests have ordinary L1 names and no tier marker or repository-file input.
- `just check-tier-coverage biscuit-file` and `just check-tier-coverage claudine` each reported zero stranded tests.
- Focused nextest runs passed two biscuit-file grammar tests, two Claudine library tests, and two Claudine CLI tests. One initial biscuit-file filter selected zero tests because it used a module prefix absent from the integration binary's test names; the corrected filter selected and passed both intended tests.
- `git diff --check` found no whitespace errors. Full suites and other operating-system legs were not rerun in this review; cross-OS evidence is a separate CI step and does not affect the `ready` flag.

The only minor documentation polish I noticed is that `FileReference::payload()` says a kind without a sigil keeps its "whole text," while recursive `%` still comes off an implicit or explicit relative reference. Its opening sentence and examples state the actual behavior, so this does not change the verdict.
