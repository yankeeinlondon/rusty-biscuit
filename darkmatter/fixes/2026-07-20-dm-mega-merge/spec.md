---
status: ready for planning and implementation
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-07-21
created: 2026-07-21
area: darkmatter
packages:
  - biscuit-file
  - biscuit-file-cli
  - sniff
  - sniff-cli
  - darkmatter
  - darkmatter-cli
  - dmls
  - claudine
  - claudine-cli
inputs:
  - ./_research.md
  - ./darkmatter-log.md
  - ./more-is-more-log.md
  - ./conflict-report.md
---

# Darkmatter and More-Is-More Integration Merge

## Summary

Merge the pinned `more-is-more` branch into a dedicated integration branch based
on the pinned `darkmatter` branch without losing, weakening, or silently
changing either branch's completed work.

This is an integration effort, not a routine conflict-marker cleanup. The two
branches contain 182 post-fork commits in total. Git predicts only six direct
conflicts across ten shared paths, but several of the paths that auto-merge join
independently developed behavior in Darkmatter's compose and schema pipelines.
Those semantic seams are the primary risk.

At the time of the research, both source branches pass their Level 1 and Level 2
tests and lints. The merged tree MUST re-establish those gates for the affected
scope on the available macOS host. Cross-platform behavior remains a design
constraint, but completion of this merge MUST NOT depend on access to Windows or
Linux hardware. Level 3 testing is outside this specification and is neither a
gate nor a required evidence source.

## Pinned Inputs and Control Artifacts

The research and predicted conflict set are valid only for these revisions:

| Input | Revision |
|---|---|
| Merge base | `d672388dd0fed4196295e7f21514cac6fa59f0ae` |
| Pinned `darkmatter` commit | `14dd391f45206d58383ba9d84adbf53c65520534` |
| Pinned `more-is-more` commit | `0584d8297f57f5eb30b52d03b1241ba55184bb44` |

The integration branch MUST start from the pinned `darkmatter` revision and
merge the pinned `more-is-more` revision. These object IDs are immutable; later
movement of the branch names does not change the pins and MUST NOT silently
repin the merge. Before work begins, each object MUST resolve to a commit and
the computed merge base MUST equal the pinned merge-base object. If an object is
missing, or if the integration is intentionally updated to a newer source
commit, the implementer MUST stop and regenerate the merge base, branch deltas,
shared-path inventory, and `git merge-tree` preview before editing conflict
resolutions.

> **Review note:** The reviewed specification and its `inputs` are control
> artifacts, not content of either pinned commit. Before Phase 0, record each
> artifact's path, Git status, and exact blob identity. They MUST remain
> readable throughout the merge but MUST NOT enter the integration index as an
> accidental copy of dirty source-worktree state. If these artifacts are meant
> to ship with the integration result, add them as a separately enumerated and
> reviewed documentation delta after conflict resolution; do not disguise that
> addition as source-branch merge content.

## Problem

The branches diverged from a shared performance-followup history and then
developed separate but adjacent capabilities:

- `darkmatter` adds list-aware cleanup and reflow, opaque directive-body
  protection, trusted-fresh reference validation, and invalid-frontmatter
  analysis and repair backed by `biscuit-file`.
- `more-is-more` adds structured expression literals, indexed-file functions,
  Git/worktree context, hermetic conflict prediction, remote provider queries,
  semantic meta-schema types, and their Sniff, DMLS, and Claudine integrations.

The work is intended to be additive, but the branches meet at authority
boundaries including schema exports, validator construction, compose command
plumbing, dependency features, DMLS overlays, terminal-test infrastructure, and
repository guidance. A resolution that merely compiles could still:

- remove one branch's public exports or validators;
- restore a superseded local test harness;
- weaken stale-graph or no-side-effect safety guarantees;
- route Git/provider behavior around Sniff;
- route YAML repair around `biscuit-file`;
- change schema-clean behavior through coercing validation;
- corrupt review history or generated metadata; or
- accidentally include unrelated source-worktree changes.

## Goals

1. Preserve the complete, intentional behavior of both pinned branch heads.
2. Resolve every predicted textual conflict by meaning, not by selecting an
   entire side.
3. Audit every shared path that Git auto-merges, with focused tests for the
   behavioral seams.
4. Preserve the established ownership boundaries among `biscuit-file`, Sniff,
   Darkmatter, DMLS, Claudine, and `biscuit-terminal`.
5. Keep source branches and source worktrees recoverable and unchanged while
   integration occurs in a separate clean worktree.
6. Establish a scoped, serial, reproducible macOS verification result for
   build, Level 1, Level 2, and lint gates.
7. Preserve the truthful status of historical performance and platform evidence
   without turning unrelated evidence collection into merge work.
8. Produce an auditable resolution record that maps every conflict and semantic
   seam to the invariant and test evidence that protects it.

## Non-goals

This work does not:

- redesign or extend any of the merged features;
- refactor adjacent code merely to make the merge look cleaner;
- change public APIs, CLI schemas, serialized shapes, diagnostics, exit codes,
  or feature acceptance criteria except where the two pinned branches already
  do so;
- replace Sniff's Git/provider authority, `biscuit-file`'s YAML-analysis
  authority, or Darkmatter's expression/schema authority;
- close the quiet-host evidence gap in `2026-07-15-performance-followup`;
- close the timing or native runtime evidence gaps in
  `2026-07-14-invalid-frontmatter`;
- invent performance claims when admissible measurements are unavailable;
- run or require Level 3 tests;
- require native Windows or Linux hardware, virtual machines, or remote runners;
- run workspace-wide Cargo gates or unscoped root lifecycle recipes;
- run `cargo fmt` or `rustfmt` in write mode; or
- modify, rebase, force-update, or delete either source branch.

## Normative Requirements

`MUST`, `MUST NOT`, `SHOULD`, and `MAY` are used normatively.

### R1. Integration isolation and recoverability

1. Before the merge, the implementer MUST record:
   - the three pinned revisions;
   - the path, status, and exact Git blob identity of this specification and
     each listed control artifact;
   - `git status --short` for both source worktrees;
   - every unrelated tracked modification and untracked file that must remain
     outside the integration result; and
   - any active Cargo, rustc, linker, Nextest, Criterion, or terminal-harness
     process that could interfere with verification.
2. Dedicated backup refs MUST be created for both pinned commits before the
   merge begins. Their names and targets MUST be recorded, and an existing ref
   MUST NOT be overwritten without first proving it already names the same
   object.
3. The merge MUST occur in a new, clean integration worktree and branch based on
   the pinned `darkmatter` commit. It MUST NOT occur directly in either source
   worktree.
4. Except for frozen control artifacts separately authorized under R12, source
   worktree-local files, including local settings and unrelated edits, MUST NOT
   be copied, staged, or committed as part of the integration.
5. The initial merge MUST use an explicit no-fast-forward, no-commit operation
   against the pinned incoming object so the complete resolution and two-parent
   topology can be inspected first. No merge commit may be created without
   separate authorization.
6. The source refs MUST remain available until the merged result has passed all
   required gates and the final staged diff has been approved.
7. If a checkpoint fails in a way that makes the resolution state unreliable,
   recovery MUST return to the clean integration ref or recreate the disposable
   integration worktree. Recovery MUST NOT rewrite either source branch.
8. Process discovery is read-only. The implementer MUST NOT terminate an
   unrelated process merely because it is Cargo-, Rust-, test-, benchmark-, or
   terminal-related. A process may be stopped only when it is attributable to
   this integration and stopping it is already authorized; otherwise the gate
   MUST wait, use isolated resources, or be deferred and reported.
9. Every Git command that could consult credentials MUST run with
   `GIT_TERMINAL_PROMPT=0`; all commands MUST be one-shot and non-interactive.

### R2. Scope discovery and change intelligence

1. Package and package-area scope MUST be recorded separately with
   `sniff repo packages` and `sniff repo package-areas`. Cargo workspace
   membership MUST be confirmed with
   `cargo metadata --locked --no-deps --format-version 1`, which remains the
   source of truth for workspace members.
2. Before editing affected symbols, GitNexus upstream impact analysis MUST be
   recorded for those symbols, including direct callers, affected execution
   flows, and risk level.
3. Any HIGH or CRITICAL impact result MUST be surfaced before the corresponding
   symbol is edited and MUST receive focused regression coverage.
4. The minimum candidate package-area scope is:

   | Area | Reason in scope |
   |---|---|
   | `biscuit-file` | Shared spans and the source-first YAML analysis/repair foundation |
   | `sniff` | Git discovery, worktrees, conflict prediction, remotes, providers, and credentials |
   | `darkmatter` | Compose, expressions, schemas, cleanup, references, and CLI |
   | `darkmatter/dmls` | Passive editor consumers of expressions, schemas, and graph data |
   | `claudine` | Downstream traversal and validation of container expressions and semantic schema values |

5. The final scope MUST expand beyond that minimum for every downstream package
   identified by GitNexus impact analysis or `sniff repo package-dependencies`,
   especially consumers of changed public types or enums. The exact packages,
   their discovered package areas, and the recipe or selector used for each
   MUST be recorded before gates run.
6. A candidate package may be removed from the final gate scope only when Sniff
   discovery/dependency evidence and GitNexus impact evidence both show that the
   merge result cannot affect it. Such a reduction MUST be documented.
7. `biscuit-terminal` is an unchanged upstream boundary, not a default affected
   package area: neither pinned branch has a post-fork net change under that
   area. Its tree MUST remain identical to both pinned inputs, and the
   Darkmatter real-terminal Level 2 tests MUST exercise the integration seam.
   Add `biscuit-terminal` gates only if the actual merge changes its tree or
   impact analysis expands the affected scope to it.

### R3. Feature preservation from `darkmatter`

The merged tree MUST preserve these contracts:

1. List cleanup recognizes prose inside ordered, unordered, task, nested, and
   blockquoted lists in default, preserve, and fixed-width modes.
2. Fixed-width wrapping reconstructs complete logical prose blocks and emits
   full list/blockquote continuation prefixes in Unicode display columns.
3. Cleanup preserves structural boundaries, opaque Darkmatter directive bodies,
   shell payloads, code, tables, HTML, hard breaks, nested blocks, and malformed
   source through the established source-preserving fallback.
4. Direct library cleanup, compose inline-post cleanup, CLI stdout/`--save`, and
   DMLS formatting remain structurally and behaviorally equivalent and
   idempotent where their existing contracts require it.
5. Fresh reference validation and `FileTree::ensure_built` skip redundant
   compatibility rereads, while caller-supplied graphs remain fail-closed and
   reject stale or mismatched descendants.
6. `PreparedHeadingSnapshot` continues to provide coherent fragment validation
   without entering `Debug` or serialized graph views.
7. Invalid-frontmatter behavior remains limited to frontmatter; fenced YAML in
   the Markdown body is not inspected or repaired.
8. YAML repairs remain deterministic, non-overlapping, UTF-8-safe, source-first,
   and constrained by the ratified safety matrix. Less-certain findings remain
   report-only.
9. Authored delimiters, untouched bytes, BOM/line-ending semantics, source
   spans, the version-1 JSON envelope, schema flags, trigger isolation, and
   zero-work/parse-count invariants remain intact.
10. The retained performance-followup improvements and compatibility fixes,
    including directory-hash membership, terminal caching, no-NTP Darkmatter
    capture, shell ordering, and graph ownership, MUST NOT regress.

### R4. Feature preservation from `more-is-more`

The merged tree MUST preserve these contracts:

1. `find_first_index(file)` and `find_last_index(file)` use the existing
   indexed-stem grammar and return portable lowest/highest on-disk family paths.
2. Expression array and object literals remain immutable, span-aware, computed,
   deterministic on duplicate keys, and compatible with postfix indexing.
3. `ctx.branch`, `ctx.worktree`, and `ctx.merge_conflicts` remain
   demand-driven, share repository discovery, and degrade independently.
4. Conflict prediction remains caller-repository anchored, direction-correct,
   committed-tip based, deterministic, and free of changes to HEAD, refs,
   worktree, live index, and object database.
5. Preferred-remote selection and provider/flavor discovery remain Sniff-owned
   across GitHub, GitLab, Gitea/Forgejo, Bitbucket, Azure DevOps, and supported
   self-managed variants.
6. `branch_exists_on_remote`, `remote_vendor`, `pr`, `pr_list`, `cicd`, and
   `cicd_list` retain exact query validation, bounded traversal, capability
   checks, typed failures, deterministic ordering, safe Markdown projection,
   and correct no-result behavior.
7. Remote execution remains deny-by-default, exact-host scoped, provider-aware
   in its credential handling, run-wide/single-flight, and consistent across
   body, frontmatter, and `$()` expression surfaces.
8. DMLS remains catalog-driven and passive: completion, hover, diagnostics,
   links, and graph integration MUST NOT execute shell commands, query the
   network, mutate repositories, or compose documents.
9. The `type-definition` and `schema` SimplifiedSchema types retain their
   nominal semantics, shared grammar/AST authority, descriptors, serializer,
   portable carrier domain, custom validators, array postfix support, and
   shared recursion limit.
10. The Darkmatter base schema continues to declare `$schema` as `schema`.
11. Source-aware schema parsing retains its sidecar spans, bounded reference
    resolution, cycle/depth distinction, frozen v1 presentation grammar, and
    passive DMLS activation, recovery, completion, hover, and diagnostics.
12. Claudine and Claudine CLI continue to traverse, validate, classify, and
    render container expressions and semantic schema values.

### R5. Authority boundaries

The merge MUST preserve one production authority for each concern:

| Concern | Authority | Required boundary |
|---|---|---|
| YAML source analysis and schema-agnostic repair | `biscuit-file` | Darkmatter layers schema knowledge without cloning the YAML engine |
| Git/worktree/remote/provider discovery | Sniff | Darkmatter captures/projects results without implementing a second Git or provider stack |
| Expression and schema semantics | Darkmatter | DMLS and Claudine consume shared descriptors and parse products |
| Passive editor intelligence | DMLS | No shell, network, repository mutation, or compose execution |
| Terminal discovery and terminal components | `biscuit-terminal` | Darkmatter uses the shared real-terminal harness and renderable components |
| Reference validation trust | Darkmatter reference module | Internal fresh graphs and public prebuilt graphs keep separate trust seams but one validation engine |

No conflict resolution may introduce a second parser, validator, formatter,
remote executor, Git implementation, or terminal harness for an already-owned
concern.

### R6. Predicted conflict resolutions

Each of the six predicted conflicts MUST have a written resolution entry and
MUST meet the following path-specific requirements:

| Path | Required resolution |
|---|---|
| `.claude/skills/darkmatter/SKILL.md` | Retain both branches' non-duplicated guidance, reconcile contradictions against the merged code, update `last_updated`, and recompute the file's Markdown-aware hash with Darkmatter after the body is final. |
| `.claudine/memory/commits.md` | Retain both the non-interactive signing/pinentry safety guidance and the prohibition on bypassing repository hooks. Preserve the incoming `--only` plus `-F -` argument-order guidance. |
| `CLAUDE.md` | Preserve all non-conflicting content from the two pinned commits, but do not import unrelated dirty-worktree edits. Refresh GitNexus once from the resolved tree; do not choose either stale generated count. |
| `darkmatter/cli/tests/level2_code_block_styling.rs` | Keep the centralized helper in `tests/common/level2.rs`; do not restore the removed local tmux harness. Port any unique incoming build-shim or terminal-discovery coverage and preserve serialization. |
| `darkmatter/cli/tests/level2_errors.rs` | Keep one canonically ordered `md_shim` import and ensure the tests use the current shared helper/build shim. |
| `darkmatter/features/2026-07-15-performance-followup/review-8.md` | Retain the `darkmatter` version. Restore the Review 7 -> 8 -> 9 -> 10 chain and retain the open quiet-host evidence status. |

Whole-file `ours` or `theirs` resolution MUST NOT be used for production files.
It MAY be used only when a path-level audit proves one side is intentionally
identical or obsolete and the resolution record includes that proof.

### R7. Auto-merged shared-path audits

The following four production paths MUST receive line-by-line semantic review
even when Git reports a clean auto-merge:

1. `darkmatter/cli/src/commands/compose.rs`
   - retain the shared baseline-disable rule used by clean;
   - retain the removal of obsolete approval-error bindings; and
   - preserve focused provider/approval error classification and rendering.
2. `darkmatter/lib/Cargo.toml`
   - retain Sniff's `remote` feature;
   - retain `git2` as a dev-only merge-prediction oracle;
   - retain the `clean_hot_paths` benchmark target; and
   - confirm the manifest without permitting lockfile drift by using
     `cargo metadata --locked --no-deps --format-version 1`.
3. `darkmatter/lib/src/markdown/schemas/mod.rs`
   - keep the extracted test-module layout;
   - retain clean-analysis exports, schema override/raw-validation seams, and
     deterministic problem ordering;
   - retain bounded schema references and source-aware parser/cursor/span
     exports required by DMLS; and
   - do not recreate a removed god-file.
4. `darkmatter/lib/src/markdown/schemas/validate.rs`
   - retain the helper visibility required by schema-clean analysis;
   - retain the URL-scheme, `type-definition`, and `schema` validator
     registrations; and
   - exercise raw and coercing validation separately.

The remaining six overlapping paths are covered by R6. Together, R6 and R7
account for all ten shared net-change paths from the pinned preview. Any newly
overlapping path discovered after a refreshed preview MUST be added to the same
audit record before work continues.

### R8. Directory-level semantic audits

After path-level resolution, the implementer MUST review these unions for
cross-file authority or propagation defects:

- `darkmatter/lib/src/markdown/compose/context`;
- `darkmatter/lib/src/markdown/compose/expression`;
- `darkmatter/lib/src/markdown/schemas`, especially `clean`, `simplified`,
  `reference`, `resolve`, `format`, and `validate`;
- `darkmatter/dmls/src/overlay`, `providers`, `diagnostics`, and graph substrate;
- `darkmatter/cli/src/commands/clean*` and `commands/compose.rs`;
- `sniff/lib/src/filesystem/git` and `sniff/lib/src/remote`; and
- Claudine's lifecycle, validation, classification, and rendering paths.

The audit MUST confirm that new state and policy objects propagate through all
intended entry points, including nested compose subtrees, frontmatter, body,
shell expressions, CLI commands, and DMLS passive projections.

### R9. Repository support files and evidence integrity

1. Root workflow, testing-strategy, review-schema, prompt, skill, and commit
   guidance changes MUST be treated as functional process changes and reviewed
   for accidental coverage reduction or duplicate jobs.
2. Existing benchmark samples, fixture hashes, review records, and historical
   evidence MUST NOT be rewritten merely because the branches merged.
3. Generated metadata MUST be refreshed only after production and test code is
   resolved. This includes the Darkmatter skill hash and GitNexus counts/index.
   The generator command and resulting changed paths MUST be recorded, and no
   unrelated generated output may be accepted with them.
4. Snapshots MUST NOT be accepted wholesale. Every changed snapshot MUST be
   reviewed and tied to an intentional merged behavior.
5. The performance-followup review chain MUST remain internally consistent and
   MUST continue to state that its quiet-host evidence is open.
6. The invalid-frontmatter timing and native Linux/Windows runtime gaps MUST
   remain visible as carried-forward evidence gaps. They are not merge
   completion gates under this specification.
7. Verification commands MUST NOT silently rewrite `Cargo.lock`, snapshots,
   fixtures, baselines, or authored documentation. Cargo inspection and gates
   MUST use the existing lockfile; any generated diff is a failure until it is
   tied to an intentional requirement and reviewed.

### R10. Cross-platform compatibility without cross-platform hardware gates

1. Production code and tests MUST continue to be designed for macOS, Windows,
   and Linux.
2. The merge review MUST check path handling, path separators, line endings,
   process invocation, executable discovery, credential/environment access,
   shell assumptions, terminal behavior, and conditional compilation for
   platform-specific regressions.
3. Portable test fixtures MUST remain portable; for example, cross-platform
   command fixtures SHOULD use compiled Rust helpers where the source branches
   already established that pattern.
4. Existing cross-platform CI definitions and evidence MUST not be weakened by
   conflict resolution.
5. Native Windows or Linux execution, remote runners, virtual machines, and
   access to non-macOS hardware MUST NOT be prerequisites for declaring this
   merge complete. If such evidence becomes available, it MAY be recorded as
   supplemental evidence only.

### R11. Resource isolation

1. One validation owner MUST run gates from the integration worktree.
2. Package-area gates MUST run serially, not concurrently across worktrees.
3. `CARGO_BUILD_JOBS` and `NEXTEST_TEST_THREADS` SHOULD be capped to a recorded
   host budget and held constant across comparable runs.
4. Real-terminal tests MUST retain their existing serialization and MUST NOT run
   concurrently from multiple worktrees.
5. Performance measurements whose specifications require a quiet host MUST be
   declined when the host does not meet that contract. A noisy result MUST NOT
   be substituted as acceptance evidence.
6. The integration MUST use a recorded, dedicated Cargo target directory, or
   otherwise prove that concurrent worktrees cannot write to the same build
   artifacts during validation.
7. Tests for remote-provider behavior MUST use loopback fixtures only. Ambient
   provider credentials, Git credential helpers, and network access MUST NOT
   influence acceptance results.

### R12. Evidence artifacts and handoff

1. The authoritative resolution record MUST be
   `darkmatter/fixes/2026-07-20-dm-mega-merge/resolution-record.md`.
2. The final handoff report MUST be
   `darkmatter/fixes/2026-07-20-dm-mega-merge/merge-report.md`.
3. Each verification entry MUST identify the working directory, exact command,
   selected package/package area, relevant job/thread and target-directory
   settings, exit status, and result. Large raw logs MAY remain external when
   the record provides a stable path and a concise failure/success summary.
4. The record MUST distinguish source-branch content, conflict-resolution
   edits, generated metadata, and separately authorized control-artifact
   additions. This distinction MUST also appear in the final staged-diff audit.
5. Handoff MUST leave the integration in an inspected, fully resolved
   no-commit merge state. Creating the merge commit is outside this
   specification unless separately authorized.

## Sequencing and Checkpoints

### Phase 0: Freeze the source state

1. Verify that the pinned objects are commits and that their computed merge
   base equals the pinned merge base. Do not substitute moving branch tips.
2. Record the status and Git blob identity of the reviewed control artifacts.
3. Regenerate the branch deltas, shared-path inventory, and conflict preview
   only if the source commits are intentionally repinned.
4. Inventory dirty/untracked source-worktree state and preserve unrelated work.
5. Inventory potentially interfering processes. Stop only an attributable,
   already-authorized integration process; otherwise isolate or defer the gate.
6. Record packages and package areas with Sniff, confirm Cargo membership with
   locked metadata, and capture applicable GitNexus impact data.
7. Create non-overwriting backup refs and a dedicated clean integration
   worktree from the pinned `darkmatter` commit.
8. Record the validation owner, the dedicated Cargo target directory, and the
   fixed job/thread budget.

Checkpoint: the source worktrees are unchanged, the integration tree is clean,
the pinned inputs are recoverable by name, and the conflict inventory is
current.

### Phase 1: Create the unresolved integration state

1. Merge the pinned `more-is-more` revision without committing.
2. Compare the actual conflicts with the predicted six.
3. Stop and update the conflict inventory if any unexpected conflict or
   modify/delete case appears.
4. Save the pre-resolution status and unmerged-path list in the merge record.
5. Verify that `HEAD` still equals the pinned `darkmatter` commit and
   `MERGE_HEAD` equals the pinned `more-is-more` commit.

Checkpoint: every unmerged path is understood and mapped to a requirement
before any conflict marker is edited.

### Phase 2: Resolve production authority seams

Resolve and audit production behavior before documentation or generated counts:

1. `darkmatter/lib/Cargo.toml`;
2. `darkmatter/lib/src/markdown/schemas/mod.rs`;
3. `darkmatter/lib/src/markdown/schemas/validate.rs`;
4. `darkmatter/cli/src/commands/compose.rs`; and
5. the directory-level compose, schema, DMLS, Sniff, and Claudine unions in R8.

Run `cargo metadata --locked --no-deps --format-version 1` after manifest
resolution and confirm that it produces no lockfile diff.
Perform applicable GitNexus impact analysis before editing symbols. Resolve
each authority seam additively and record the preserved invariants.

Checkpoint: production code has no unmerged index entries, metadata resolves
without changing the lockfile, and the authority-boundary audit is complete.

### Phase 3: Resolve tests and focused harness behavior

1. Resolve `level2_code_block_styling.rs` around the centralized common helper.
2. Resolve `level2_errors.rs` and its build-shim import.
3. Reconcile any auto-merged tests around schema, clean, expressions, DMLS,
   Sniff, and Claudine.
4. Confirm no local duplicate tmux harness, parser, validator, or formatter was
   restored.

Checkpoint: focused tests compile far enough to confirm imports, feature flags,
and test harness topology before broad package-area gates.

### Phase 4: Resolve documentation, policy, and generated metadata

1. Merge the Darkmatter skill content semantically.
2. Merge commit safety guidance without weakening either branch.
3. Retain Review 8 and repair the Review 7 -> 8 -> 9 -> 10 chain.
4. Review workflow, schema, prompt, and public-documentation unions.
5. Refresh the Darkmatter skill hash with `md hash <file> --save` only after its
   content is final, then verify it with `md hash <file> --diff`.
6. Refresh GitNexus and its generated counts only after the full source tree is
   final.
7. Add the frozen control artifacts only if they are explicitly in the intended
   integration deliverables, and record them as a separate documentation delta.

Checkpoint: `git ls-files -u` is empty, `git diff --check` passes, review links
are valid, and regenerated metadata describes the merged tree rather than
either source tree. Any conflict-marker scan MUST be limited to changed text
files and account for intentional conflict fixtures; a repository-wide textual
scan is not authoritative.

### Phase 5: Focused convergence tests

Run the smallest deterministic tests first so failures remain attributable:

1. schema exports, nominal validators, raw/coercing validation,
   references/cycles/depth, source projection, and DMLS schema consumers;
2. invalid-frontmatter analysis/repair, safety gates, spans/delimiters, JSON,
   flags/triggers, zero-work, and parse-count behavior;
3. expression literals, indexed-file functions, context capture, remote-runtime
   propagation, provider errors, and network-policy behavior;
4. fresh versus checked reference validation and changed-child behavior;
5. list cleanup/reflow, opaque directives, idempotency, and library/compose/CLI/
   DMLS parity;
6. Sniff Git parity, conflict prediction, remote discovery/resolution,
   provider filtering, and credential isolation;
7. DMLS no-side-effects, completion, hover, diagnostics, links, and session
   behavior; and
8. Claudine container traversal and semantic-schema lifecycle behavior.

Focused Rust tests MUST run through Nextest (or an area `just` recipe that uses
Nextest), never `cargo test`. Remote-provider cases MUST remain fixture-backed
and loopback-only.

Checkpoint: each semantic convergence seam has focused passing evidence before
the broader gates begin.

### Phase 6: Scoped macOS package-area gates

Run one affected package area at a time using its own recipes. The minimum
sequence is:

1. `biscuit-file`;
2. `sniff`;
3. `darkmatter` (`darkmatter` and `darkmatter-cli`);
4. `darkmatter/dmls` (`dmls`), unless the recorded parent-area recipes prove
   they already cover the same gates; and
5. `claudine`.

Append every downstream package or package area required by the final R2 scope.
Do not add `biscuit-terminal` merely because Darkmatter depends on it; preserve
that boundary with the tree-identity check and Darkmatter's serialized
real-terminal Level 2 coverage unless R2 expands the scope.

For every area retained by R2, run the applicable:

- `just build`;
- `just test` for Level 1;
- `just test-l2` for Level 2; and
- `just lint`.

Use exact package selectors when the area recipe supports a narrower recorded
scope. Do not use a bare root `cargo build`, `cargo check`, or `cargo test`; do
not use `--workspace`; and do not use an unscoped root lifecycle recipe.

Checkpoint: all affected macOS build, Level 1, Level 2, and lint gates pass in
serial with no source-worktree test process competing for resources.

### Phase 7: Final audit and handoff

1. Run GitNexus `detect_changes` against `main` and compare affected symbols and
   execution flows with the recorded scope.
2. Compare the resolved tree against each pinned parent as well as `main`.
   The first-parent comparison identifies the incoming integration delta; the
   second-parent comparison proves preservation of `darkmatter`-side work;
   `main` satisfies the repository-wide change-detection contract.
3. Inspect `git status`, the complete diff, and the staged diff from the
   integration worktree.
4. Reconfirm `HEAD`, `MERGE_HEAD`, the merge base, and the absence of unmerged
   index entries.
5. Confirm all ten shared paths have resolution/audit entries.
6. Confirm no unrelated source-worktree file, local setting, generated junk, or
   bulk snapshot update is present.
7. Confirm all carried-forward evidence gaps are still documented and have not
   been mislabeled as closed.
8. Re-run `git status --short` in both source worktrees and prove that the merge
   did not change their tracked or untracked state.
9. Produce a concise merge report listing pinned inputs, actual conflicts,
   resolutions, focused evidence, package-area gates, generated metadata, and
   any non-blocking follow-up.

Checkpoint: the integration result is ready for an explicitly authorized
commit, while both source refs and worktrees remain recoverable.

## Verification Matrix

| Seam | Minimum evidence |
|---|---|
| Schema clean + meta-schema | Raw and coercing validator tests; nominal keyword registration; clean override and deterministic diagnostics; references/cycles/depth; DMLS source projection |
| Compose + providers | Structured literal/index tests; demand-driven Git context; remote runtime propagation; fatal focused provider errors; exact-host deny-by-default policy |
| Git safety | Git/git2 parity fixtures and before/after repository-state assertions proving no mutation |
| Cleanup + DMLS formatting | Default/preserve/fixed-width list matrix; opaque mixed blocks; idempotency; library/compose/CLI/DMLS parity |
| Reference validation | Fresh path skips re-verification; prebuilt path rejects stale children; shared validation output and fragment snapshot coherence |
| Invalid frontmatter | Ratified repair matrix; report-only findings; UTF-8 and line-ending spans; delimiter preservation; stable JSON; zero-work and parse-once counters |
| DMLS passive behavior | No-side-effects suite plus completion/hover/diagnostic/link/session coverage |
| Claudine downstream behavior | Container traversal and semantic-schema classification, validation, lifecycle, and rendering tests |
| Terminal boundary | Centralized serialized Level 2 harness, build shim, stable code-block rendering, and single terminal discovery |
| Process/docs | Review chain, skill hash, GitNexus refresh/change detection, workflow/schema review, clean staged diff |

## Risk Register

| Risk | Severity | Primary controls |
|---|---:|---|
| Valid work is lost through whole-side conflict resolution | Critical | Pinned refs, backup refs, clean integration worktree, path resolution records, additive review |
| Auto-merged schema or compose code compiles but drops behavior | Critical | Mandatory semantic audits, focused seam tests before broad gates |
| DMLS begins executing network, shell, compose, or repository operations | Critical | Authority review and mandatory no-side-effects coverage |
| Conflict prediction or remote discovery mutates/leaks repository or credential state | Critical | Sniff ownership, hermetic parity/state snapshots, host-bound credential isolation tests |
| YAML repair corrupts authored bytes or applies uncertain edits | Critical | Shared `biscuit-file` engine, safety matrix, span/delimiter/property tests |
| Stale prebuilt reference graphs are accepted | High | Separate fresh/checked trust seams and changed-child regression tests |
| Duplicate tmux harness or concurrent real-terminal tests create flakes | High | Centralized helper, serialization, single validation owner |
| Dirty source-worktree state contaminates the merge | High | Preflight inventory, separate clean worktree, final staged-diff audit |
| Uncommitted control artifacts are lost or mistaken for pinned branch content | High | Blob identities, separate documentation-delta classification, explicit staged-diff audit |
| A moving branch name silently changes the integration inputs | High | Immutable object-ID pins, merge-base verification, explicit repin procedure |
| Public Sniff/schema changes break an omitted downstream consumer | High | Sniff dependency graph, GitNexus upstream impact, dynamically expanded gate scope |
| Generated counts/hash describe one source branch | Medium | Regenerate once, after code and docs are final |
| Inspection or test commands silently rewrite the lockfile or fixtures | High | `--locked`, before/after status checks, intentional-diff review |
| Historical evidence is silently relabeled or overwritten | Medium | Preserve artifacts, review links, and explicit carried-forward gaps |
| Unix-only assumptions enter production or tests | High | Cross-platform design audit and preservation of portable fixtures; macOS-only blocking gates |
| Host contention makes tests flaky or measurements misleading | Medium | Serial areas, bounded jobs/threads, no competing worktree processes |

## Resolution Record

The implementation MUST maintain the R12 resolution record with one entry per
conflict and auto-merged shared production path. Each entry MUST include:

- path and conflict type;
- behavior contributed by `darkmatter`;
- behavior contributed by `more-is-more`;
- chosen merged structure and why it preserves the authority boundary;
- symbols and flows identified by impact analysis, when applicable;
- focused tests or inspections used as evidence; and
- any follow-up that is explicitly outside merge completion.

Unexpected conflicts and intentionally changed snapshots MUST use the same
record format.

## Completion Criteria

The merge is complete only when all of the following are true:

1. The actual merge uses the pinned revisions, or the research and preview were
   regenerated for explicitly updated revisions.
2. Both source worktrees and source refs remain intact and recoverable.
3. `HEAD`, `MERGE_HEAD`, and the computed merge base prove the intended
   two-parent topology, and `git ls-files -u` is empty.
4. All six predicted conflicts and all ten shared paths have documented,
   requirement-linked resolutions or audits.
5. All R3 and R4 feature-preservation invariants hold in the merged tree.
6. All R5 authority boundaries remain singular and explicit.
7. No unresolved index entry, malformed review link, stale generated
   hash/count, or unintended snapshot change remains.
8. `cargo metadata --locked --no-deps --format-version 1` succeeds, confirms
   the intended package/features/dev-dependency shape, and leaves `Cargo.lock`
   unchanged.
9. Focused convergence tests in Phase 5 pass through Nextest-backed commands.
10. The affected package-area `just build`, Level 1, Level 2, and lint gates pass
   on the available macOS host.
11. Cross-platform compatibility has been reviewed without requiring native
    Windows or Linux execution.
12. No Level 3 result is used or required for acceptance.
13. Performance-followup and invalid-frontmatter evidence gaps remain honest,
    visible, and non-blocking for this merge.
14. GitNexus change detection against `main`, plus comparisons against both
    pinned parents, matches the recorded affected scope.
15. The final status and staged diff contain only the intended integrated work
    and separately enumerated control artifacts.
16. The R12 resolution record and merge report are complete, and the merge
    remains uncommitted unless separate authorization was given.
17. The final merge report makes no unsupported performance, platform, or
    production-readiness claim.

Passing compilation alone is insufficient. Completion requires evidence that
the behavioral, safety, passive-analysis, history, and ownership contracts of
both pinned commits survive together.
