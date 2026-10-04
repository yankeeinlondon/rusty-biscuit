---
$schema: feature-review.yaml
ready: true
human_review: true
human_review_items:
    - |-
        The existing choice about repair affecting other worktrees remains unresolved. Git's repair command can restore missing links and overwrite unusable `.git` files in other registered worktrees, even when only one path is named. The implementation and documentation currently accept and disclose that behavior. The author still needs to choose the intended contract:

        - Accept Git's repair behavior and disclose its effects on other worktrees, as currently implemented.
        - Refuse automatic repair when another worktree has a broken link; additional checks still leave a race.
        - Write and verify only the target's link, including support for Git's relative link paths.

        This item is carried forward unchanged from the previous review. It is an external design review, not a remaining implementation finding. No response is requested during this non-interactive review.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-04T02:14:13-07:00
spec: 2026-10-03-broken-worktree-entries/spec.md
implemented: false
description: "A **fix** review of `2026-10-03-broken-worktree-entries/spec.md`"
fix: 2026-10-03-broken-worktree-entries/review-3.md
previous: 2026-10-03-broken-worktree-entries/review-2.md
---

# Review 3: Broken worktree entries

**Production ready for the implemented, documented repair policy.** Both unblocked findings from review 2 are addressed. No further implementation or test-coverage defect was found within this fix. The existing human choice about repair affecting other worktrees remains separate from readiness, as requested.

## Previous findings

Review 2 contains two unblocked findings and no blocked findings. There was therefore no blocked code finding to become unblocked before this implementation. Its human repair-policy item remains open, with no changed decision to reaffirm.

| Previous finding | Implementation and verification | Result |
| --- | --- | --- |
| High: Missing-record removal does not revalidate split-index dependencies | In the `worktree` library, [inspect_missing](../../lib/src/remove/missing.rs:105) binds the primary index bytes and Git's complete entry reading before computing the report. [remove_missing_record](../../lib/src/remove/missing.rs:211) rechecks both. The Git listing includes entry flags through `--debug`, and Git errors propagate as refusal. Real-Git API and CLI matrices cover all five dependency edits, clean/staged controls, and force flags. | Addressed |
| Medium: Note wrapping interprets user text as its internal delimiters | In `worktree-cli`, [Note](../../cli/src/commands/list_table.rs:409) separates markup from copyable text. Every occurrence of the remaining stand-in, including genuine input occurrences, is recorded and restored by position. The public rendering matrix covers all prior projections, all five former delimiter characters, ordinary markup controls, and 400/60-column widths. The original Git-metadata CLI fixture now preserves the exact command. | Addressed |

Review 1's replacement-link guards, report-bound approval, branch-tip checks, and real-terminal requirements remain present. The full fresh test runs include those regressions.

## Unblocked Findings

None.

## Blocked Findings

None. The human design item in frontmatter is carried forward independently.

## Recurrence

No finding recurs in this review. Review 2's split-index finding repeated review 1's incomplete index-snapshot binding; the additional Git entry digest now covers the omitted dependency. The earlier renderer collision class is also resolved across its listed projections.

## Completed class sweeps

### Approval must cover the complete staged-state snapshot

**Class:** A destructive operation must bind approval to every dependency that determines the staged state it reports, and refuse when that state changes or becomes unreadable.

| Site / sibling decision | Shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| Missing-checkout primary index | Changed staged content, corrupt or disappeared primary index after inspection | Existing API/CLI regressions refuse and retain record/branch | Refuse |
| Missing-checkout split index | Unchanged clean and staged controls | Clean removal succeeds; staged CLI removal refuses without consent and succeeds with `--force-worktree` | Allow only the reported state with required consent |
| Missing-checkout split dependency | Corrupted, removed, emptied, trailing garbage, replaced by a directory; clean and staged variants | API matrix refuses all ten cases; CLI matrix exits 3 for unforced clean and all-force staged cases, retaining the record and branch while primary bytes stay unchanged | Refuse every unreadable dependency |
| Missing-checkout initial inspection | Missing/non-file/corrupt primary index; Git reading a split index | Primary-file regressions refuse; both logical listing and staged comparison propagate Git failures | Never treat failed inspection as clean |
| Healthy and repaired checkout, unforced execution | Whole index read by status and Git removal | Git failures propagate; these paths do not bypass inventory or force removal | Preserve unchecked state |
| Existing-checkout approved discard | Report-time fingerprint followed by fresh inventory/fingerprint comparison | Existing changed-index/content CLI regressions refuse; fingerprint reads index entries through Git | Consent covers reported state |
| First and second handoff runs | Report-time fingerprint, changed index/content/link/rules/baseline/branch | Existing binding and CLI tests refuse changed state; repaired-wrapper L2 scene passes | Revalidate approval after moving the shell |
| Administrative association and branch deletion | Reassociated record, moved HEAD, moved branch tip | Existing API/CLI regressions retain unapproved state; later branch failure reports partial completion | Do not delete unassessed state |
| Create/include destination listing | Git index listing used for copying | Read failure propagates; this is not record deletion or discard approval | No destructive snapshot binding required |

The newly changed missing-record code uses `biscuit-hash` and delegates index-format interpretation to Git. It introduces no second binary index parser. `--debug` additionally covers flags that a stage-only listing omits. The checks remain snapshots rather than an atomic lock against arbitrary concurrent Git writers; this fix does not claim transactional deletion.

### External display text must stay literal

**Class:** Names, paths, reasons, and error messages must remain display data rather than become markup or wrapping instructions.

| Site / sibling projection | Shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| Missing-entry label and removal command | Ordinary `<red>INJECTED` control; each of U+FDD0 through U+FDD4 | Public renderer preserves literal label and exact quoted command at both widths; original CLI metadata reproduction passes | Literal, selectable command |
| Unlinked repair command, target and base paths | Same six inputs at 400 and 60 columns | Both full paths remain in the exact command | Literal, complete command |
| Other-unavailable path; competing path in name-conflict explanation; unavailable fast-forward holder | Same matrix | Exact paths remain whole, including overlong runs | Preserve copyable paths |
| Git reason; path inspection error; `.git` inspection error | Same matrix | Literal input remains visible | No delimiter or markup interpretation |
| Default-branch note; missing-ref note; fallback-key note | Same matrix | Literal fields remain visible | Preserve external text |
| Shared note wrapping | Genuine stand-in and all former delimiters, normal/narrow widths | Positional restoration preserves genuine characters; ordinary input leaves no stand-ins; long commands remain whole | Collision-safe wrapping |
| Table labels, legends, caption, credentials, remove reports/refusals, dirty-tree output, verbose headings | Producer inspection and existing output tests | External text is escaped; these renderers do not parse the removed in-band delimiters | Literal text |
| PR status and refresh hints | All status producers inspected | Fixed wording and numeric ages | No external delimiter input |
| Create/reuse messages, go messages, verbose commit/ref formatting | New escape calls inspected; create and verbose formatting regression tests freshly pass | User text is escaped, including link attributes in create output | Literal external fields |

All prior renderer projections are covered, including the worktree label and both repair-command paths, which share matrix cases. Existing real-terminal listing scenes also pass after the renderer change. The additional escaping changes do not require a new UI interaction or keyboard-encoding test.

## Input robustness matrix

This fix reads Git porcelain, administrative path text, and Git-managed indexes. It adds no JSON, YAML, or TOML loader. The last repair changes snapshot binding, not those formats' parsing rules. The matrix below records the relevant outcomes; typed nulls and heterogeneous collection elements do not exist in these inputs.

| Shape | Porcelain `prunable` marker/reason | Administrative `gitdir` path | Primary index | Referenced split-index dependency |
| --- | --- | --- | --- | --- |
| Positive control | Real-Git missing/unlinked rows classified; healthy row stays healthy | Exactly one real-Git record associates | Clean removal; staged work requires consent | Unchanged clean/staged controls pass with required consent |
| Absent | No marker; status failure remains unknown | Association refuses | Inspection/removal refuses | Git read fails; removal refuses |
| Empty | Bare marker is present with empty reason | Refuses | Git read fails; refuses | Git read fails; refuses |
| Null / wrong typed element | Not a shape in this line format | Not a shape in this path format | Not a typed collection | Not a typed collection |
| Wrong whole-field kind | Non-marker text does not create a marker | Non-path text, directory, or unreadable file refuses | Non-file or invalid index refuses | Directory or corrupt bytes refuse |
| Duplicate | Both reasons retained; not used for a safety decision | Multiple path lines or duplicate matching records refuse | Git owns binary format validation | Git owns binary format validation |
| Trailing / invalid content | Reason remains opaque text; localized text never authorizes deletion | NUL, embedded lines, and non-path trailing text refuse; Git-compatible final line endings allowed | Git validation or changed primary digest refuses | Trailing garbage is one of the freshly passing refusal cells |
| Changed after report | Listing identity rechecked before missing-record removal | Association rechecked | Primary byte digest rechecked | Complete Git entry digest rechecked; unreadability refuses |

API and shipped-CLI tests assert removal/refusal and retained state, rather than relying only on a successful parser return. The shared-file matrix starts from Git-produced split indexes and changes one file per cell; the primary bytes are explicitly asserted unchanged. The note matrix similarly changes one external field per projection from its positive fixture.

## Requirement coverage and validation

| User-facing requirement | Appropriate level / strongest relevant verification | Result |
| --- | --- | --- |
| Preserve prunable reasons, distinguish confirmed absence, reject replacement links, and make failed status unknown | L1 parser, injected inspection/status failures, real-Git API and CLI regressions | Pass |
| Listing does not repair/remove; unavailable default-branch holders block fast-forward; unknown dirtiness is not reported as source changes | L1 command recording, real-Git listing/refresh/comparison/graph tests | Pass |
| Missing-directory removal protects surviving staged work, branch safety, identity, reappearance, and split-index dependencies | L1 public API and shipped CLI, including all-force boundary edits | Pass |
| Repair verifies exact administrative/common/top-level identity independently of exit status and preserves state on refusal | L1 real-Git, injected repair, and binary regressions | Pass |
| Repair cancellation keeps the restored link; shell wrapper moves before approved removal | L2 real-terminal repair/cancellation scenes plus L1 policy/protocol tests | Pass |
| Handoff rechecks files, index, link, rules, baseline, branch, and remote approval without repairing | L1 binding/CLI tests and L2 repaired-wrapper scene | Pass |
| Cross/question glyphs, styling, conditional legends, ordered dim notes, and narrow output | L2 real-terminal captures, including normal/80/60-column scenes; L1 literal-field matrices | Pass |
| Contextual errors retain categories, partial success is reported, and repair side effects are disclosed | L1 error/report/exit tests and README/topic-doc review | Pass |

There is no bare-modifier, hotkey, paste, IME, mouse, or terminal-input-encoding requirement in this fix; Level 3 is not needed. The inherited 60-column table-overflow limitation is already documented in review 2; the scenes verify the new rows and notes there without claiming that unrelated table layout is fixed.

Fresh validation on macOS:

- Sequential `just test`: **1,063 passed, 32 skipped**, 27.534 seconds. This includes the complete split-index and external-field matrices and the CLI metadata reproduction.
- `BISCUIT_L2_THREADS=2 BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2`: **39 passed**, 54.255 seconds. Backend proof records **33 tmux executions, zero skips/panics**; the run also includes six Kitty graph tests. Harness scenes use their existing non-focus-taking mechanisms.
- `just lint`: both packages pass.
- `just check-tier-coverage worktree`: **zero stranded tests**. L1 regression files are automatic integration targets. L2 targets require `terminal-tests`; the live recipe and CI metadata enable it.
- The first L1 attempt, run concurrently with the L2 build, stopped after 641 tests because a PR-worker test could not spawn `wt` (`NotFound`). The concurrent rebuild of the shared executable is the likely cause; the spawn error alone does not establish it. Running the complete L1 recipe after the L2 run finished passed every test, including the stopped test. No production or test-source change was made to resolve it.

Cross-OS evidence remains CI's responsibility and does not affect this decision. No formatter, commit, or lifecycle move was run. The review changes only review/spec metadata and this document; existing implementation edits are left intact.

The README, current list/remove topic pages, and worktree skill describe unknown status, repair side effects, report-bound consent, split-index revalidation, and literal recovery commands consistently. The existing implementation log also records an unrelated, pre-existing Unicode byte-slicing risk in verbose commit-subject truncation; that is outside this fix's contract and is not presented here as a newly reproduced or readiness-blocking defect.
