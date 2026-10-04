---
$schema: feature-review.yaml
ready: false
findings:
    - title: Missing-record removal does not revalidate split-index dependencies
      priority: high
    - title: Note wrapping interprets user text as its internal delimiters
      priority: medium
human_review: true
human_review_items:
    - |-
        The existing choice about repair affecting other worktrees remains unresolved. Git's repair command can restore missing links and overwrite unusable `.git` files in other registered worktrees, even when only one path is named. The implementation and documentation currently accept and disclose that behavior. The author still needs to choose the intended contract:

        - Accept Git's repair behavior and disclose its effects on other worktrees, as currently implemented.
        - Refuse automatic repair when another worktree has a broken link; additional checks still leave a race.
        - Write and verify only the target's link, including support for Git's relative link paths.

        This item is carried forward unchanged from the previous review. It does not block either repair requested below, and no response is requested during this non-interactive review.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-04T01:41:38-07:00
spec: 2026-10-03-broken-worktree-entries/spec.md
implemented: true
implemented_by: claude/opus
log: worktree/fixes/2026-10-03-broken-worktree-entries/log.md
description: "A **fix** review of `2026-10-03-broken-worktree-entries/spec.md`"
fix: 2026-10-03-broken-worktree-entries/review-2.md
previous: 2026-10-03-broken-worktree-entries/review-1.md
next: 2026-10-03-broken-worktree-entries/review-3.md
---

# Review 2: Broken worktree entries

**Not production ready.** The previous review's specific reproductions are addressed, but its index-snapshot defect class remains in Git's split-index format. The new command-wrapping implementation also introduces an escaping defect. Both findings can be fixed without human input.

## Previous findings

The previous review has a `Findings` section rather than separate `Unblocked Findings` and `Blocked Findings` sections. All three findings were explicitly unblocked. There were no blocked code findings to become unblocked before this implementation. The separate human decision about Git's repository-wide repair behavior remains open; the spec, implementation log, and documentation consistently retain the disclosed current behavior.

| Previous finding | Implementation checked | Result |
| --- | --- | --- |
| Replacement directory links bypass removal guards | Unconditional final-component link inspection in availability; preparation and first handoff consume it; post-repair kind verification; second handoff refusal; final removal primitive kind guard | Addressed. Regressions cover readable replacement links, retained files/records/branches, forced and unforced removal, both handoff runs, post-repair replacement, and the ancestor-alias control. |
| Missing-record removal discards index changes after inspection | Report-time index digest, administrative association and commit revalidation; ordinary forced-discard fingerprint; report-time handoff fingerprint; expected branch tip before branch deletion | The reported ordinary-index reproduction is addressed. Split-index dependencies were missed; see the high finding below. |
| New markers and recovery notes lack real-terminal verification | Five new tmux scenes, including normal/80/60-column cases, long paths, glyph/style checks, conditional legends, ordered dim notes, and ordinary controls | Addressed. All 19 listing terminal scenes passed, including these five. At 60 columns the scenes intentionally omit border/legend assertions for existing narrow-table overflow; this is documented in the test and not a new readiness finding. |

## Unblocked Findings

### High: Missing-record removal does not revalidate split-index dependencies

**Defect class:** A destructive operation binds approval to only one physical file even though the staged-state snapshot depends on additional files.

Git supports a split index: the worktree's administrative `index` references a neighboring `sharedindex.<object-id>` file. In the `worktree` package, [inspect_missing](../../lib/src/remove/missing.rs:96), which checks a deleted checkout's surviving staged state, hashes only `index` before asking Git to read the complete index. [remove_missing_record](../../lib/src/remove/missing.rs:179), which removes the administrative record, later hashes only that same file. A changed, disappeared, or unreadable referenced shared index therefore does not invalidate the earlier report.

**Reproduction through the shipped CLI:** Create an isolated local repository on `main`, commit `a`, and add `target` on `topic`. Run `git -C target update-index --split-index`; identify the administrative directory with `rev-parse --path-format=absolute --git-dir`. Confirm it contains both `index` and `sharedindex.*`, then delete `target`. Put a one-shot Git shim first on `PATH`: delegate `diff-index` to real Git, capture its successful clean output, overwrite the referenced shared index with `corrupt`, then return the original output and status. Delegate every other command unchanged. Run `wt remove topic` from the base with closed stdin and no force flags.

The command exits **0**, claims the index matches the last commit, removes the administrative record, and deletes `topic`. The primary `index` bytes never changed. Deleting the shared index instead produces the same result. This is a deterministic boundary edit, with no sleeps or network activity. It establishes deletion of state that can no longer be inspected, rather than demonstrating a newly staged valid version being discarded.

| Site / sibling decision | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Missing checkout: ordinary primary index | Primary index changed, corrupted, or disappeared after inspection | Existing public-API and CLI regressions refuse; records and branches retained | Clean |
| Missing checkout: split index, unchanged control | Real Git split index and its referenced file remain unchanged | Exit 0; record and approved branch removed | Clean |
| Missing checkout: split index execution | Referenced shared index corrupted after `diff-index` | Exit 0; record and branch removed | Exit 3; retain record and branch; require a fresh inspection |
| Missing checkout: split index execution | Referenced shared index removed after `diff-index` | Exit 0; record and branch removed | Same refusal |
| Missing checkout: split index execution | Referenced shared index emptied, given trailing garbage, or replaced by a directory after `diff-index` | Exit 0 in all three cases; record and branch removed | Same refusal |
| Missing checkout: initial split-index inspection | Referenced shared index absent, empty, garbage, trailing garbage, or a directory before inspection | Exit 3 for every shape; record and branch retained | Clean |
| Healthy checkout: ordinary execution | Same shared-index corruption immediately after successful status | Git refuses; exit 1; working files, record, and branch retained | Clean for preservation |
| Repaired checkout: ordinary execution | Same shared-index corruption after successful post-repair status | Git refuses; exit 1; restored link, files, record, and branch retained | Clean for preservation |
| Existing-checkout handoff | Same shared-index corruption between approval and the second invocation | Inventory refuses; exit 1; working files, record, and branch retained | Clean for preservation; this review does not introduce a new error-category requirement |
| Existing-checkout approved discard and handoff fingerprint | Index inspection through `git ls-files --stage -z` | The fingerprint reads the logical index through Git, including split-index dependencies; its failures propagate | Clean by implementation inspection and the handoff reproduction |
| Branch and administrative identity | Commit moves or record is associated with a different administrative directory after inspection | New public-API regressions refuse; expected-tip branch-deletion regression keeps the moved branch | Clean |

**Fix:** Bind and revalidate the complete index state that informed the report. Asking Git for a logical index listing with the same `GIT_INDEX_FILE` before the report and again before removal avoids implementing Git's binary split-index format. Retain the primary-file existence and identity checks; an unreadable dependency must refuse, including with force flags. Use `biscuit-hash` for any new digest. Add a real-Git split-index matrix covering unchanged clean/staged controls, consent, and all five dependency edits above, asserting the public removal outcome and retained state.

### Medium: Note wrapping interprets user text as its internal delimiters

**Defect class:** Unescaped in-band delimiters allow display data to become renderer instructions, changing literal names, paths, and suggested commands.

In the `worktree-cli` package, [notes_list](../../cli/src/commands/list_table.rs:349), which renders closing notes and PR status bullets, and [keep_whole](../../cli/src/commands/list_table.rs:402), which protects commands from wrapping, use Unicode characters `U+FDD0` through `U+FDD4` as internal markers. External text can contain those characters. The collision guard checks only the last three; it does not protect the opening and closing delimiters. `keep_whole` scans the whole markup string, including text that was already escaped.

**Shipped CLI reproduction from real Git metadata:** Create two linked worktrees: `target` on branch `topic`, and directory `topic` on another branch. Delete `target`. Change only its administrative `gitdir` back-reference to a missing path whose basename is `target` followed by `U+FDD1` and `<red>INJECTED`, ending the back-reference with `/.git`. Git lists this missing path normally. The branch name `topic` is ambiguous, so the recovery note correctly tries to suggest the unique directory basename. Run `wt list`.

The table and note label preserve the literal `<red>` text, but the proposed command becomes `wt remove 'targetINJECTED'` with an extra delimiter outside the quote. The `<red>` token is interpreted as markup and disappears. The suggested argument no longer identifies the recorded entry. This reproduction uses metadata because APFS on this host rejects `U+FDD1` in an actual filename; missing Git-recorded paths are still valid review inputs.

I also swept every external text projection through the public `worktree-cli` rendering API. Each control used ordinary `<red>INJECTED` text; subsequent cells added one of the five delimiter characters. Temporary observation probes were compiled in the existing declared L1 integration target and then removed, restoring the source bytes. The probes reported preservation results; their passing process exit was not treated as proof of correctness.

| Site / sibling projection | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Missing-entry removal command | `U+FDD1` inside the selected basename | Command loses literal text; reproduced through CLI and API | Preserve the exact quoted argument and show markup literally |
| Unlinked-entry repair command: target path | `U+FDD1` in target path | Full target path not preserved | Preserve the exact quoted path |
| Unlinked-entry repair command: base path | `U+FDD1` in base path | Full base path not preserved | Preserve the exact quoted path |
| Other-unavailable note: observed path | `U+FDD1` in path | Full path not preserved | Preserve literal path |
| Name-conflict explanation: competing paths | `U+FDD1` in a competing path | Full competing path not preserved | Preserve literal path |
| Fast-forward refusal: unavailable holder path | `U+FDD1` in holder path | Full holder path not preserved | Preserve literal path |
| Git reason and filesystem-inspection error messages | `U+FDD0` in each reason / path-error / `.git`-error field | Opening delimiter removed; literal text changes | Preserve external text without treating it as a delimiter |
| Worktree label, default-branch note, missing-ref note, fallback-key note | `U+FDD0` in each field | Opening delimiter removed in all four projections | Preserve external text |
| Every copyable-path projection above | `U+FDD2`, `U+FDD3`, or `U+FDD4`; 400-column control | Literal field preserved | Clean at wide width |
| Shared wrapping protection | Each of `U+FDD2`, `U+FDD3`, `U+FDD4` in a long base path; 60-column pane width | Protection is disabled for the entire note list; ordinary wrapping breaks the path and the whole command is no longer present | Preserve the whole path/command, including genuine input characters |
| All thirteen external-field projections above | Ordinary markup-like text without delimiter collision | Literal text preserved | Clean controls |
| Table labels, legends, caption, and remove refusal renderer | Same delimiter-bearing inputs, code-path sweep | They do not use `notes_list`'s delimiter parser; the table label is literal in the CLI reproduction | Clean for this class |
| PR age/failure status and refresh hint | All `render_status` producers inspected | Static wording and numeric ages; no external names, paths, or reasons enter this parser | Clean for this class |

**Fix:** Keep copyable text as structured renderer data, or introduce collision-safe encoding that covers every external field and all five markers. Do not fall back to corrupting long commands when input contains a marker. Retain literal text escaping and the existing long-path behavior. Add one public-renderer matrix covering all affected projections, all five characters, ordinary markup controls, and normal/narrow widths; include a CLI regression for the metadata fixture. Continue using the existing `biscuit-terminal` rendering components.

## Recurrence

The high finding repeats **review-1.md, “Missing-record removal discards index changes made after inspection.”** That repair should have swept the complete surviving index in both ordinary and split-index form, including the shared file consumed by Git, alongside healthy/repaired execution, handoff fingerprints, branch tips, and administrative association. It swept the latter sites and the primary index but omitted the shared dependency. This review's tables include the failing split-index shapes and the clean sibling paths, so the next repair has a complete list rather than another isolated primary-file case. The delimiter finding below is a new class, not a recurrence of the prior terminal-verification finding.

## Blocked Findings

None. The existing human design choice is recorded in frontmatter and does not block implementing these findings.

## Input robustness sweep

The changed readers consume Git porcelain, administrative path text, Git-managed binary indexes, and opaque bytes for hashing. This fix does not introduce a JSON/YAML/TOML configuration reader. Typed nulls, wrong-type collection elements, and duplicate object keys do not exist in these formats; they are not silently defaulted.

| Load-bearing input | Absent / empty | Invalid / trailing / duplicate | Public outcome and coverage |
| --- | --- | --- | --- |
| Porcelain `prunable` marker / reason | Absence means no marker; bare marker means present with empty reason | Duplicate reasons retained; no localized reason is used as a safety decision | Existing parser, list, and no-status-call tests pass; unconditional target-link checks now cover marker absence |
| Administrative `gitdir` back-reference | Missing/empty refuses association | Multiple path lines, NUL, and trailing non-path content refuse; relative paths and platform byte handling are covered | Prior review's CLI matrix remains applicable; no reader change in this repair |
| Primary surviving `index` | Missing/non-file refuses; invalid index refuses initial inspection | Changed bytes after inspection now refuse; unchanged controls pass | New API/CLI regressions pass, including staged replacement, corruption/disappearance, branch movement, and administrative reassociation |
| Referenced split-index file | Missing/empty refuses initially but is accepted after inspection | Garbage, trailing garbage, and directory replacement have the same before/after split | All five shapes freshly tested through CLI; high finding carries the complete matrix |
| Names, paths, reasons, and errors passed to note wrapping | Empty/ordinary values covered by existing rendering cases | Ordinary markup escapes correctly; internal delimiter characters do not | Public projection matrix and CLI metadata reproduction identify the medium finding |

The new hashing check reads opaque index bytes; Git remains the format parser. The repair should therefore use Git to validate the complete logical state rather than create a second binary parser. The existing handoff JSON loader was not changed by this implementation; its fingerprint is now captured before consent, and its version/approval checks retain their existing behavior.

## Requirement coverage and validation

| User-facing requirement | Strongest relevant verification | Result |
| --- | --- | --- |
| Preserve prunable reasons; inspect absence without following target links; never treat a status failure as checked-clean | L1 parser, injected inspection/status errors, real-Git API tests, and binary regressions | Pass |
| Listing never repairs/removes; prunable holders block fast-forward; unknown dirtiness does not claim source changes | L1 recorded-command, real-Git, refresh, comparison, and graph tests | Pass |
| Missing-directory removal checks surviving staged state, branch safety, administrative identity, and reappearance | L1 public API and shipped CLI | Primary-file cases pass; split-index dependencies fail |
| Repair verifies exact repository/admin/top-level identity and real directory kind independently of repair exit status | L1 real-Git, injected repair, and binary replacement-link regressions | Pass |
| Repair cancellation leaves restored metadata and files; wrapper moves before approved removal | L2 detached tmux scenes, plus L1 policy/protocol tests | Both repair scenes freshly pass |
| Handoff refuses changed link, contents/index, rules/baseline, branch, or approved remote state | L1 binary and binding tests; L2 repair wrapper scene | Covered; corrupted split index is also freshly shown to retain files/record/branch |
| Cross/question glyphs, styles, conditional legend, ordered dim notes, and long commands | L2 captures at normal/80 columns; rows/notes at 60 columns; L1 render matrices | Ordinary cases pass; delimiter-bearing text fails |
| Error context and partial completion preserve categories; repair side effects are disclosed | L1 exit/category/report tests and current-doc review | Pass in covered paths |

Fresh validation on macOS:

- `just test`: **1,053 passed, 32 skipped**, 41.861 seconds.
- `BISCUIT_L2_THREADS=2 BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_list`: **19 passed**, 26.528 seconds; backend proof records 19 tmux executions, zero skips/panics.
- The repaired-wrapper and decline-after-repair scenes were separately selected by name: **one passed in each run**, with tmux execution proof. These scenes use detached panes and do not take focus.
- `just check-tier-coverage worktree`: **zero stranded tests**. Automatic integration targets compile the L1 regressions; `level2_list_verbose` is declared, requires `terminal-tests`, and the L2 recipe and CI test features enable it.
- New independent CLI reproductions used real Git, disposable repositories, closed stdin, disabled credential prompts, and no remotes. Split-index matrix controls and boundary edits were repeated in separate repositories. Public rendering probes were removed after observation; no test or production Rust source was left changed by this review.
- `git diff --check` was clean before the review-document edits. No formatter, commit, or lifecycle move was run. Cross-OS results are left to CI and do not affect this readiness decision.

The current README, list/remove topic pages, and worktree skill describe the repaired link guards, report-bound consent, unchanged-index check, and terminal output consistently. Their claim that the inspected index is revalidated needs to include its split-index dependencies once repaired. The spec remains incomplete because of the two findings above.
