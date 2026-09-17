---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-21T19:54:52-07:00
spec: 2026-07-20-dm-mega-merge/spec.md
implemented: false
description: A **fix** review of `2026-07-20-dm-mega-merge/spec.md`
fix: 2026-07-20-dm-mega-merge/review-1.md
---

# Review 1: Darkmatter and More-Is-More Integration Merge

## Verdict

Not ready for production.

The merged working files have strong Level 1 and Level 2 evidence, and the
recorded test levels are appropriate for the specified behavior. The handoff
state itself is incomplete, however: Git still reports six unmerged entries,
the required resolution and merge reports are untracked, and the generated
GitNexus metadata contradicts the merge report. There is no reproducible,
fully resolved index whose bytes can be shown to match the tested working tree.

## Findings

### Critical — The tested working tree is not a resolved merge candidate

The integration worktree remains in a conflicted index state. `git ls-files -u`
reports all six predicted conflicts:

- `.claude/skills/darkmatter/SKILL.md`
- `.claudine/memory/commits.md`
- `CLAUDE.md`
- `darkmatter/cli/tests/level2_code_block_styling.rs`
- `darkmatter/cli/tests/level2_errors.rs`
- `darkmatter/features/2026-07-15-performance-followup/review-8.md`

The working files are marker-free, but their reviewed resolutions have not
replaced the stage-2/stage-3 entries. The integration report acknowledges that
the result is not ready for commit authorization and that the six entries
remain unmerged (`merge-report.md:10-16`, `merge-report.md:160-167`). This fails
the specification's Phase 4 checkpoint and completion criteria 3, 7, and 15,
which require an empty unmerged index and an inspected staged diff
(`spec.md:554`, `spec.md:708-729`).

The Level 1 and Level 2 runs therefore verify transient working-tree bytes, not
a final candidate tree. A later staging mistake could select a parent blob or
omit one of the tested corrective edits while leaving the historical test logs
green.

Resolve the index with the exact reviewed working blobs, then record the staged
tree identity and prove that the cached diff contains every tested correction
and no parent-side conflict selection. Re-run affected gates if any staged byte
differs from the already tested working content.

### High — Required handoff artifacts are outside the candidate

Both required evidence files are untracked in the integration worktree:

- `darkmatter/fixes/2026-07-20-dm-mega-merge/resolution-record.md`
- `darkmatter/fixes/2026-07-20-dm-mega-merge/merge-report.md`

R12 makes these the authoritative resolution record and final handoff report
(`spec.md:445-459`), and completion criterion 16 requires both to be complete
(`spec.md:730-731`). Until their disposition is authorized, their bytes are
reviewed, and they are included in the final staged-diff audit, the evidence is
not part of the deliverable it purports to certify.

Stage the reviewed artifacts as the separately enumerated documentation delta,
then validate their frontmatter and links from the resolved integration tree.

### High — GitNexus metadata and the handoff report disagree

The report states that the final GitNexus refresh did not persist, that
`CLAUDE.md` retains temporary Darkmatter-parent counts, and that those counts
are `136293 / 270769 / 300` (`merge-report.md:101-109`). The actual marker-free
working file instead contains `138336 / 276514 / 300`. The stage-2 parent is
`136293 / 270769 / 300`, while the stage-3 parent is
`136465 / 272631 / 300`; the working values match neither parent and are not
explained by the report.

The resolution record also labels final change detection as stale-index
evidence because the required refresh was stopped before persistence
(`resolution-record.md:727-735`). This fails R9's generated-metadata provenance
requirement and completion criterion 14. The current counts cannot be treated
as validated output, and the report is not a truthful description of the tree
it accompanies.

Complete one bounded GitNexus refresh against the integration worktree, record
the producing command and registered index identity, update `CLAUDE.md` from
that result, rerun `detect_changes` for both `all` and `compare main`, and then
rewrite the report so its counts and status match the final bytes.

### High — The final staged/unstaged audit has not happened

In addition to the six unmerged paths, the integration worktree contains
unstaged Phase 5/6 corrections in Darkmatter CLI tests, Sniff tests and CLI
code, Claudine harness/lint paths, and the generated dispatch inventory. The
resolution record describes these corrections and reports downstream gates,
but explicitly states that the final staged/unstaged/status views cannot
satisfy the handoff invariant (`resolution-record.md:747-754`).

This is not merely administrative: the merge combines public schema, compose,
Git/provider, DMLS, and Claudine behavior, and GitNexus reports HIGH aggregate
risk across 870 changed symbols and 13 execution flows. Without a final cached
diff, there is no proof that the exact collection of corrections exercised by
the gates is the collection being handed off.

After resolving the index and adding the authorized evidence files, inspect the
complete cached and unstaged diffs against both pinned parents and `main`.
Require an empty unexpected-unstaged set before production readiness.

## Verification-Level Matrix

| User-observable requirement family | Strongest evidence present | Assessment |
|---|---:|---|
| Schema/meta-schema validation, invalid-frontmatter analysis and repair, expression literals/indexes, provider policy, reference trust, cleanup/formatting parity, Sniff Git/remote behavior, DMLS protocol behavior, and Claudine downstream behavior | Level 1 through focused Nextest runs and final area `just test` recipes | Appropriate for in-process semantics. The evidence applies to the working tree, not yet to a resolved candidate index. |
| Terminal code-block styling, terminal error rendering, schema-about color/theme behavior, and the centralized binary shim/harness | Level 2 through the Darkmatter area's `just test-l2` recipe; 18 Darkmatter, 69 CLI, and 3 DMLS tests reported passing | Appropriate and at the required real-terminal level. No wrong-level gap was found. |
| Modifier presses, hotkeys, paste, IME, or OS mouse/keyboard encoding | Not specified | Level 3 is not applicable. The specification explicitly excludes Level 3 as a gate. |
| macOS, Windows, and Linux compatibility | macOS gates plus static portability and CI-definition review | Matches R10, which intentionally does not require native Windows/Linux execution for this merge. |

## Code Quality, Ergonomics, and Performance

No additional production-code refactor, ergonomic abstraction, or performance
change should be added during this integration. The merged implementation's
authority boundaries and focused evidence are well recorded, and speculative
cleanup would widen an already high-risk merge. The necessary work is to make
the tested bytes reproducible and the evidence internally consistent.

## Production Readiness

The fix is not production ready. Functional convergence appears promising and
the selected verification levels are sound, but production readiness requires
a fully resolved index, included evidence artifacts, a completed GitNexus
refresh/change audit, and a handoff report that matches the final tree.
