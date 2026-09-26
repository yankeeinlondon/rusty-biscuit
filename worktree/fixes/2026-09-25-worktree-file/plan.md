---
total_phases: 5
created: 2026-09-25
phase: 1
agent: claude/opus
yolo: true
related:
    - 2026-09-25-worktree-file
    - 2026-09-24-ux-improvements
    - 2026-09-25-list-remove-performance
---

# Plan: `.worktreeinclude` support

## Summary and Definition of Done

### The work

`2026-09-25-worktree-file` adds the shared `.worktreeinclude` convention to both ends of a worktree's life:

1. **Include set.** The include set is every file that is gitignored and also matches a root `.worktreeinclude` pattern. Git's own ignore engine resolves it: first `git ls-files --others --ignored --exclude-from=.worktreeinclude -z`, then a separate batched, NUL-delimited check against the standard ignore rules. Paths stay as native bytes throughout, and the traversal never crosses nested repositories, submodules, nested worktrees, links, or junctions.
2. **`wt create` copies it.** It first resolves a *copy source*: the worktree holding the fork source, else the base checkout, and always the base checkout for a reused branch. It then clones each file (copy-on-write where supported, byte copy otherwise). Permissions and symbolic links are preserved, existing and index-tracked destinations are never overwritten, and each file goes through a temporary copy published without replacement. Finally it writes a versioned, per-worktree **copy record** to the user cache, bound to the Git registration, and reports on stderr.
3. **`wt remove` changes its consent rule.** Only included files that are *new*, *changed since copied*, or *unknown* need consent. Every other ignored entry, including an unchanged copy, is disposable and appears in one dim summary line. Comparison runs against the copy record. When there is no record it compares with a distinct copy source instead. The handoff fingerprint is re-scoped and its format is bumped, so it binds the effective rules, the record identity, and the no-record source decision.
4. **Maintenance.** `wt remove` deletes the record, `wt list` prunes stale records after a successful worktree listing, and `wt create` invalidates any old record before it reuses a destination path.

The code lands in the `worktree` library (`lib/src/include/`, `lib/src/copy_record.rs`, `lib/src/compare.rs`, `lib/src/worktree.rs`, `lib/src/remove/{inventory,handoff}.rs`) and in `worktree-cli` (`commands/create.rs`, `commands/remove/{mod,policy,report}.rs`). The README and the `worktree` skill document the feature.

### Prerequisite

The working tree currently holds uncommitted `2026-09-24-ux-improvements` review-5 changes in `remove/mod.rs`, `remove/report.rs`, `remove/safety.rs`, `remove/remote.rs`, and `cli/tests/remove.rs`. These are the same files Phase 4 edits. Land or commit them before Phase 4 starts, so this work does not tangle with that review cycle.

### Success looks like

- [ ] Every acceptance criterion (1–7) in the spec has at least one named test, at the level the spec assigns (L1, or L2 for prompts).
- [ ] `just test`, `just test-l2`, and `just lint` pass in `worktree/` on macOS.
- [ ] Linux evidence (L1) and native Windows evidence (L1, plus `level2_powershell` via `./scripts/cross-check.sh --os windows`) exist. WSL2 is covered by the nightly leg, or by an explicit cross-check when path/link code changed.
- [ ] `wt create` stdout is byte-identical to today's shell protocol in every test, including partial copy failure.
- [ ] No test observes deletion of a new, changed, or unknown included file without consent. No test observes a prompt for an unchanged copy or for `target/`.
- [ ] Removal cost is proven by counted file reads, not wall-clock thresholds.
- [ ] The README, the `worktree` skill, the `os` skill (for any new OS trap), and `docs/dependencies.md` (if a clone crate is added) match the code.
- [ ] The implementation ends at "implementation complete, ready for review". The spec is never moved into `_completed`.

---

## Phase 1 — Rulings and Spikes

Goal: remove every ambiguity and platform unknown before any production code is written. The phase's output is this plan's rulings (updated in place) plus short spike notes in `spikes.md` next to this plan.

### Necessary Rules

Each rule carries a **provisional ruling** that implementation follows under `yolo`. The author can overturn one before Phase 2 starts. Rules R1 and R2 answer the spec's two open questions.

- [x] **R1 — Large-file shortcut (spec open question 1).** *Confirmed by the author 2026-09-25 (spec Decision 7), overturning the provisional ruling:* included files are never judged unchanged from metadata. A size difference is `Changed` without reading; every same-size included file is hashed in full with `biscuit_hash::blake3_hash_reader`, at any size, at removal and in the handoff. Modification time is not recorded for included files. The size-and-mtime shortcut survives only as the dirty-file policy owned by `2026-09-25-list-remove-performance`.
- [x] **R2 — Unchanged copy whose source is gone (spec open question 2).** *Confirmed by the author 2026-09-25 (spec Decision 8):* keep creation-baseline semantics. The README must never say deletion "loses nothing". It must say that an unchanged copy may be the last copy of the original contents.
- [ ] **R3 — Ambiguous copy source.** "Fail source selection with a warning" means `wt create` still creates the worktree (exit 0), copies nothing, and prints one warning naming the checkouts it found. It does not fail the create.
- [ ] **R4 — Record identity.** A copy record is keyed by the worktree's canonical path (the file name is `<repo hash>.copy-<blake3(canonical path)[..16]>.json` via `cache::repo_cache_file`). It is bound to:
    - the worktree's admin directory (`git rev-parse --git-dir` run in the worktree, canonical);
    - a registration fingerprint chosen by spike S3.

  On load, a mismatch in either field makes the record *untrusted*. `wt create` deletes any record at the destination's key before it runs `git worktree add`.
- [ ] **R5 — Path representation.** The include set, the copy record, and the handoff carry repository-relative paths as git's `-z` bytes (`Vec<u8>`). Records serialize them as lowercase hex, never as lossy UTF-8. Conversion to `PathBuf` goes through `OsStr::from_bytes` on Unix, and through UTF-8 on Windows, where Git for Windows emits UTF-8. A Windows path that is not valid UTF-8 is reported as an unsupported entry, never guessed.
- [ ] **R6 — mtime encoding.** Included files do not record or use modification time (R1). Where the dirty-file policy from `2026-09-25-list-remove-performance` needs it, it is stored as `(secs: i64, nanos: u32)` from `Metadata::modified()`, and a platform or filesystem without mtime takes the hash path, never the shortcut.
- [ ] **R7 — Missing versus unreadable rules.** Missing means `NotFound` on `symlink_metadata` of `<root>/.worktreeinclude`. Every other state is *indeterminate*: a directory, a symlink resolving outside the checkout, a read error, or non-UTF-8 content, since git reads the file as bytes and that alone is fine. A symlink inside the checkout is read through. Creation treats *indeterminate* as "warn and skip copying". Removal treats it as "exit 1 before mutation". An existing zero-byte file is an empty set and suppresses the removal fallback.
- [ ] **R8 — Summary line contents.** The dim "Also deletes ignored files:" line lists the first path components of every ignored entry from the existing `git status --ignored=matching` inventory, minus the entries that need consent (those are listed with the dirty files). An unchanged included file therefore appears in the summary line. The line shows names only and never counts files. It replaces the current `IgnoredGroup` count display.
- [ ] **R9 — Exit codes.** A new, changed, or unknown included file behaves exactly like a dirty file in `policy::decide`: interactive mode asks (default No), and non-interactive mode requires `--force-worktree` or exits 3. An include set that cannot be determined exits 1 before mutation. In the handoff, a content or policy mismatch exits 3 (even with `--force-worktree`), and discovery errors exit 1.
- [ ] **R10 — Scope boundary with `2026-09-25-list-remove-performance`.** This plan creates the shared comparison contract (`lib/src/compare.rs`) and uses it for *included files* in the copy record, the removal classification, and the handoff. Dirty-file fingerprint changes and PR/live-remote reuse stay in the performance spec, which will adopt `compare.rs` rather than re-implement it.
- [ ] **R11 — Copy source label.** "Copied from `X`" uses the copy source's branch name, or `base` for the main checkout, escaped through `Prose::escape_text`.
- [ ] **R12 — Where new error variants live.** Every new `WorktreeError` variant (for example `IncludeRulesIndeterminate`, `IncludeSetDiscovery`) is added in Wave 2. This keeps `error.rs` and `cli/src/exit.rs` out of the parallel waves.

### Spikes (Wave 1, all concurrent)

Each spike produces a runnable probe (a throwaway `#[test] #[ignore]` or a shell script) and a short entry in `spikes.md`. No production code comes out of this wave.

- [ ] **S1 — Git matching semantics** (`git` 2.4x on macOS; repeated on Linux and Windows through `cross-check`)
    - Confirm the two-step pipeline. `ls-files --others --ignored --exclude-from=<file> -z` gives the candidates. `check-ignore --stdin -z --verbose --non-matching` gives the standard-rule test. Confirm that `check-ignore` reports a file inside an ignored *parent directory* as ignored. If it does not, find the working alternative (for example, filtering candidates against `status --ignored=matching` output) and record it.
    - Confirm the behavior of: nested `.gitignore`, `.git/info/exclude`, `core.excludesFile`, ordered negation, a negated child under an excluded parent, `**/` into a wholly ignored directory, and a leading-`/` anchor.
    - Confirm what `ls-files --others` emits for a nested repository, a submodule, a nested linked worktree, a symlink to a directory, and (on Windows) a junction. Decide the boundary filter from that output.
    - Confirm that `--exclude-from=<absolute path of base checkout's file>` run in another worktree resolves patterns relative to that worktree's root. The removal fallback depends on this.
- [ ] **S2 — Copy-on-write API**
    - Evaluate the `reflink-copy` crate (`reflink` without fallback) against direct calls (`clonefile` on macOS, `FICLONE` on Linux, `FSCTL_DUPLICATE_EXTENTS_TO_FILE` on ReFS).
    - Check three questions. Can it clone into a caller-created temporary path, then publish without replacement (`link`+`unlink` on Unix, `MoveFileExW` without `REPLACE_EXISTING` on Windows, or `renameat2(RENAME_NOREPLACE)` / `renamex_np(RENAME_EXCL)`)? Does a clone preserve mode or reset it? Which errors mean "unsupported" versus "failed"?
    - Record the chosen crate, its license, and its dependency cost. Decide whether it belongs in `worktree` or in `biscuit-file`.
- [ ] **S3 — Registration identity**
    - Decide what distinguishes a recreated worktree at the same path from its predecessor, given that git may reuse the admin directory name after `worktree prune`.
    - Candidates: the file ID (`dev`+`ino` / Windows file index) plus the birth or modification time of `<admin>/gitdir`; or a nonce stored in the record and cross-checked against `<admin>/gitdir`'s identity.
    - Prove the chosen field changes across remove → prune → re-add at the same path, on macOS, Linux, and Windows.
- [ ] **S4 — Windows links and junctions**
    - Find how symlink creation fails without Developer Mode or the right privilege. Determine the error kind, so the plan's "warn and skip" can match it.
    - Find how to detect junctions and directory reparse points through `std` metadata (`FileTypeExt`, `file_attributes()`), so neither traversal nor ancestor checks follow them.
    - Add every non-obvious finding to the `os` skill (`windows.md`).

### Checkpoint 1

- [ ] `spikes.md` records outcomes S1–S4. R1–R12 are confirmed or amended in this file.
- [ ] If S1 shows `check-ignore` cannot express the intersection, the include-set design in Phase 2 is amended before Wave 2 starts.

---

## Phase 2 — Library Building Blocks

Goal: four independent, fully unit-tested library modules with no CLI or flow changes yet. All work is in `worktree/lib`.

### Wave 2 — Shared scaffolding (single agent, small)

- [ ] **Byte-exact git helper**
    - Add `git::git_from_bytes(base, dir, args, stdin: Option<&[u8]>) -> Result<Vec<u8>, WorktreeError>`. It runs `git -C` from `base` like `git_from_raw`, and keeps the `count-git` recording.
- [ ] **Error variants**
    - Add `IncludeRulesIndeterminate { path, reason }` and `IncludeSetDiscovery(String)` (exit 1), plus any variants S1–S4 showed are needed. Map them in `cli/src/exit.rs` (see R12).
- [ ] **Test fixtures**
    - Extend `remove::test_support::TestRepo` with helpers: `write_ignored`, `write_include_rules`, `add_linked_worktree(branch)`, `with_global_excludes`, and a fixture-owned `XDG_CACHE_HOME` / `HOME` for the user cache.
    - The Windows user-cache caveat from the skill applies. Tests that seed a store use a fixture-owned directory, passed through an injectable store path rather than the real per-user path, where the API allows it.
- [ ] **Clone dependency**
    - Add the dependency S2 chose to `lib/Cargo.toml`, and document it in `docs/dependencies.md`.

### Wave 3 — Modules (4 concurrent agents; disjoint files)

- [ ] **Include set** (`lib/src/include/mod.rs`, `rules.rs`)
    - `IncludeRules::locate(root) -> Present(PathBuf) | Empty | Missing | Indeterminate(reason)`, per R7.
    - `resolve_include_set(base, worktree, rules_file) -> Result<IncludeSet, WorktreeError>` implements the two-step pipeline from S1, with the boundary filter and native-byte paths (R5).
    - `IncludeSet` holds `entries: Vec<IncludedEntry { path: Vec<u8>, kind }>` and `unsupported: Vec<(path, kind)>`. It never contains `.git` administrative files or empty directories.
    - L1 tests cover acceptance criterion 1, plus from criterion 5: empty versus missing rules, unreadable rules, a directory in place of the file, a symlinked rules file outside the checkout, ordered negation, the negated-child-under-excluded-parent example, global and nested excludes, filenames with spaces and newlines, nested repositories, submodules, and linked directories.
- [ ] **Comparison contract** (`lib/src/compare.rs`)
    - `Observation { kind: File | Symlink { target_bytes }, size, digest: Option<[u8; 32]> }`, and a `Policy` enum: `Included` (R1) and `DirtyHandoff` (reserved for `2026-09-25-list-remove-performance`, which adds `mtime` and `HASH_ALWAYS_BELOW` when it lands).
    - `observe(path, reader: &dyn ReadCounter) -> Result<Observation>` reads with `symlink_metadata` and never follows links.
    - `compare(baseline: &Observation, path, reader) -> Unchanged | Changed | Missing | Unknown(reason)`, with these rules:
        - under `Policy::Included`, a size difference is `Changed` without reading;
        - under `Policy::Included`, a same-size file is always hashed, at any size;
        - a kind change is `Changed`;
        - a read error is `Unknown`, never `Unchanged`.
    - `digest_file` uses `biscuit_hash::blake3_hash_reader`.
    - The injected `ReadCounter` lets tests count full-content reads (acceptance criterion 7).
    - L1 tests cover: a same-size edit that restores mtime, for a small and a large file (both detected); a size change classified without a read; a large same-size file that is read; symlink target changes; a kind change; an unreadable file.
- [ ] **Copy record store** (`lib/src/copy_record.rs`)
    - A versioned `CopyRecord { format_version, worktree: canonical path, admin_dir, registration: <S3 field>, source: canonical path + label, files: Vec<(path_hex, Observation)> }`.
    - `record_path(repo_root, worktree)`, `write_atomic` (via `cache::atomic_write`, with mode `0600` on Unix set *before* rename), `delete`, and `load(repo_root, worktree, expected_identity) -> Trusted(record) | Absent | Untrusted(reason)`. A corrupt, incompatible, or identity-mismatched record is always `Untrusted`, never an error.
    - `prune(repo_root, live_worktrees: &HashSet<PathBuf>)` scans only this repository's `copy-*` files.
    - L1 tests cover: round trip, corruption, a version bump, identity mismatch, two concurrent writers for different worktrees not clobbering each other, prune leaving live records alone, and unreadable-directory behavior.
- [ ] **Copy engine** (`lib/src/include/copy.rs`)
    - `copy_include_set(source_root, dest_root, set, dest_index: &HashSet<Vec<u8>>, ops: &dyn CopyOps) -> CopyOutcome { copied: Vec<(path, Observation)>, skipped: Vec<(path, SkipReason)>, failed: Vec<(path, String)> }`.
    - `CopyOps` is the injectable seam (clone / byte-copy / symlink / publish-no-replace). `RealCopyOps` uses the S2 API and falls back to byte copy on "unsupported" or cross-volume errors.
    - Rules:
        - temporary file in the destination directory, then permissions, then digest *of the temporary copy*, then publish without replacement;
        - partial temporary files are removed on every error path;
        - destination entries that exist (including dangling links) or are index-tracked are `Skipped`;
        - ancestor file or link conflicts, and source ancestors that are links, are rejected, checked with `symlink_metadata`;
        - symlinks are recreated with their exact target bytes, and a platform refusal is `Skipped(LinkUnsupported)` per S4;
        - junctions and reparse points are never traversed;
        - if the source changed during the copy (a pre/post metadata mismatch), the entry is `Copied` but without a trusted baseline, and a warning is emitted.
    - L1 tests use injected `CopyOps` for clone success, unsupported fallback, and failure cleanup. They also cover `0600` preservation, dangling and outside-pointing links, existing destinations, index conflicts, ancestor links, and that writing one copy leaves the other unchanged. One real-filesystem test asserts cloning on a supported volume, or prints an explicit "capability unavailable" note.

### Checkpoint 2

- [ ] `just test` and `just lint` pass in `worktree/`. The new modules have no callers yet, apart from tests.
- [ ] Code review of the four modules against R1–R12 happens before integration starts, because every later phase depends on these contracts.

---

## Phase 3 — `wt create` Integration

Goal: creation copies the include set, writes the record, and reports on stderr. Phase 3 and Phase 4 touch disjoint files and **may run concurrently** once Checkpoint 2 passes. The only shared file is `lib/src/lib.rs`, which already exports everything after Wave 2.

### Wave 4 — Library (single agent: `lib/src/worktree.rs`)

- [ ] **Copy source resolution**
    - Add the pure `copy_source(entries: &[WorktreeEntry], fork_branch: Option<&str>) -> CopySource::{Worktree(entry), Base(entry), Ambiguous(Vec<entry>)}`. It is called *before* `git worktree add`, so a reused branch always resolves to the base checkout.
    - L1 tests cover forked-with-worktree, forked-without-worktree, reused, and ambiguous (R3).
- [ ] **Extend `create_worktree`**
    - Order of work:
        1. validate (unchanged);
        2. resolve the copy source;
        3. delete any stale record at the destination key (R4);
        4. run `git worktree add`;
        5. record the fork origin (unchanged);
        6. locate the rules in the copy source;
        7. resolve the include set from the copy source;
        8. read the destination index (`ls-files -z`);
        9. `copy_include_set`;
        10. write the copy record with successful copies only.
    - Every failure after step 4 becomes a warning in `CreateResult.include: IncludeOutcome { source_label, copied, skipped, failed, warnings }` and never an `Err`.
    - Update the function's `///` docs (the Errors list is unchanged; add a paragraph on the copy).
- [ ] **List prune**
    - In `fill_worktree_statuses`, next to the fork-origin prune, call `copy_record::prune` with the canonical paths from the *successful* `worktree list`. A failed listing prunes nothing.
- [ ] **L1 tests** (lib)
    - Cover acceptance criterion 2 end to end against `TestRepo`:
        - a fork from a branch with a worktree gets that worktree's `.env`;
        - a fork from a branch without a worktree gets the base checkout's;
        - a reused branch gets the base checkout's;
        - the record lists size and digest;
        - a failed copy still returns `Ok`;
        - a record write failure (read-only cache directory) still returns `Ok` with a warning;
        - path reuse after remove and re-add never inherits the old record.

### Wave 5 — CLI (single agent: `cli/src/commands/create.rs`, `cli/tests/`)

- [ ] **Create report**
    - After the existing "Created worktree" block, render one `Prose` line to stderr: "Copied from `<label>`: a, b, c". Filenames are escaped, control characters are made visible, and the line wraps. When nothing was copied, print nothing.
    - Warnings (per-file failures, skipped links, ambiguous source, indeterminate rules, record write failure) are an `UnorderedList` to stderr. They never include file contents.
- [ ] **CLI tests** (`cli/tests/create_include.rs`, new)
    - Assert stderr lines with `NO_COLOR=1`.
    - Assert stdout is exactly the existing protocol (`cd:` lines only under the wrapper, nothing otherwise), including partial success.
    - Filenames with spaces or control characters render escaped.

### Checkpoint 3

- [ ] `just test` and `just lint` pass. Running `wt create` by hand in a scratch repository with `.env` + `.worktreeinclude` shows the copied line, and `wt list` shows no regression.

---

## Phase 4 — `wt remove` Integration

Goal: consent covers only new, changed, or unknown included files, and the handoff binds the new policy inputs. Prerequisite: the in-flight ux-improvements changes are landed (see Summary).

### Wave 6 — Library (2 concurrent agents)

- [ ] **Included-file classification** (agent A: `lib/src/remove/inventory.rs`, new `lib/src/remove/included.rs`)
    - `classify_included(base, worktree, entries) -> Result<IncludedAssessment, WorktreeError>` does the following:
        - locates the worktree's own rules; only on `Missing` does it fall back to the base checkout's file, evaluated in the removed worktree (per S1). `Empty` never falls back. `Indeterminate` is `Err` (exit 1);
        - resolves the include set;
        - loads the record with the S3 identity;
        - classifies each entry `New | Changed | Unknown(reason) | Unchanged | Missing` through `compare::compare`.
    - The no-record fallback resolves the source from the fork-origin store and the current `worktree list`. The source must be a checkout distinct from the worktree being removed (compare canonical paths). It compares regular files by full digest regardless of size, and links by target and kind. It never writes a record.
    - `IncludedAssessment` carries: `needs_consent: Vec<(path, Mark)>`, `rules: RulesBinding { location, presence, content_digest }`, `baseline: BaselineBinding::{Record { identity, content_digest } | NoRecord { source, per-file results } | None }`.
    - `Inventory` gains `included: IncludedAssessment`:
        - `needs_consent()` becomes "dirty is non-empty, or `included.needs_consent` is non-empty";
        - `disposable_ignored_names()` replaces `ignored_groups()` for the report (R8);
        - `fingerprint()` replaces its `ignored\0` lines with `included\0<path>\0<mark>\0<digest-or-observation>` lines, plus the rules and baseline bindings;
        - dirty-file and full-index coverage are unchanged.
    - Update the module and method docs, whose current "Ignored entries count like dirty files" wording becomes wrong.
- [ ] **Handoff format v3** (agent B: `lib/src/remove/handoff.rs`)
    - Bump `HANDOFF_FORMAT_VERSION` to 3 and add the rules and baseline bindings to `HandoffState` (the fingerprint already covers content). `verify` re-resolves membership against the fresh state. Any rules or baseline difference is a `HandoffRefusal` (exit 3), even when a protected file left the selected set.
    - L1 tests: a v2 record is treated as missing; a rules change, a record change, or a changed no-record source each refuse; a new `target/` artifact does not refuse.
- [ ] **Record deletion helper** (agent B)
    - Add `copy_record::delete_for(repo_root, worktree)`. It returns a warning string rather than failing.

### Wave 7 — CLI flow (single agent: `cli/src/commands/remove/{mod,policy,report}.rs`)

- [ ] **Flow**
    - `Facts::gather` passes `Indeterminate` and discovery errors up as exit 1 before any mutation.
    - `execute` deletes the copy record right after the directory is removed. A later branch or remote failure does not undo that deletion, and a record-deletion failure is a warning.
    - `run_handoff` compares the new bindings.
- [ ] **Policy**
    - Needing consent because of included files follows the same `policy::decide` path as dirty files (R9). Extend the pure matrix test with included-only, dirty+included, unknown-only, and unchanged-only (which asks nothing).
- [ ] **Report**
    - Included files needing consent are listed with the dirty files and marked `new`, `changed`, or `unknown`, and they are named in the confirmation question.
    - One dim "Also deletes ignored files: target/, .DS_Store" line (R8) replaces the counted groups.
    - Update `refusal_markup`, which currently says "ignored files (listed above)".
- [ ] **L1 CLI tests** (`cli/tests/remove.rs`)
    - Cover every non-interactive row of the consent table with its exit code.
    - An unchanged `.env` plus `target/` is removed without a question.
    - Editing the source's `.env` after creation causes no question.
    - A same-size edit that restores mtime asks, for a small and a large file.
    - A size change is classified without reading, and a large same-size file is read (read counter via the lib seam, or `count-git`-style instrumentation).
    - Without a record, the command falls back to the source. Self-comparison refuses. An unavailable source gives `unknown`.
    - The summary line appears, and the record is deleted with the worktree.
    - A failure leaves files, registration, and branch intact.
    - Handoff cases from acceptance criteria 4 and 6.

### Checkpoint 4

- [ ] `just test` and `just lint` pass. The existing `level2_remove` and `level2_powershell_remove` suites still pass. Their expectations about ignored files are updated deliberately, because the ruled policy change (Decision 3) makes some old "ignored needs consent" expectations wrong on purpose. Each changed assertion is listed in the implementation log.

---

## Phase 5 — L2, Documentation, and Cross-OS Validation

### Wave 8 (3 concurrent agents)

- [ ] **L2 prompts** (`cli/tests/level2_remove.rs`)
    - Using the existing focus-preserving tmux harness, add:
        - a representative interactive prompt for a `changed` included file (default No keeps everything);
        - an `unknown` file;
        - an unchanged-copy removal with no prompt;
        - a handoff that refuses after a `WT_TEST_BETWEEN` edit of `.env`.
    - No window gains focus.
- [ ] **Documentation**
    - `worktree/README.md`: the `.worktreeinclude` convention and its link to Claude Code and Worktrunk, syntax, the negated-child example, the copy source rules, copy-on-write, the consent table, that included files are always compared by full content (R1), the R2 "last copy" caveat, the consequence of editing patterns, and the advice to name files rather than `node_modules/`.
    - `.claude/skills/worktree/SKILL.md`: add `include`, `compare`, and `copy_record` sections, and rewrite the `wt remove` inventory bullet (ignored entries no longer all need consent; the handoff is v3).
    - `os` skill: add S2 and S4 findings.
    - `docs/dependencies.md`: add the clone crate.
- [ ] **Cross-OS evidence**
    - Linux L1 (`cross-check` or CI).
    - Native Windows L1, plus `level2_powershell` via `./scripts/cross-check.sh --os windows worktree-cli --features terminal-tests level2_powershell` (read the durations to confirm it ran and was not skipped).
    - WSL2 via cross-check, because link and path code changed.
    - Record every result in `implementation-log.md` (create it).

### Wave 9 — Close-out (single agent)

- [ ] **Performance spec handoff**
    - Add a note to `2026-09-25-list-remove-performance` that `compare.rs` exists and is the contract its dirty-file fingerprint must reuse (R10). Change only its text, not its code.
- [ ] **Final validation**
    - Run `just test`, `just test-l2`, and `just lint` in `worktree/`. Walk the Success checklist above and check each item with its evidence.
- [ ] **Status**
    - Set the spec's frontmatter to `status: implemented`, `implemented: true`, `implemented_by: claude/opus`. Stop at "implementation complete, ready for review". Do not move the spec and do not run `just complete`.

### Checkpoint 5

- [ ] Every item in "Success looks like" is checked, with a pointer to its test or evidence.

---

## Dependency Graph

```text
Wave 1 (S1–S4 spikes) ─► Checkpoint 1 (rules confirmed)
        │
Wave 2 (git bytes, errors, fixtures, clone dep)
        │
Wave 3 [include set | compare | copy_record | copy engine]  (parallel)
        │
   Checkpoint 2 ──────────────┬───────────────────────────┐
                              ▼                           ▼
             Wave 4 (create lib) ─► Wave 5 (create CLI)   Wave 6 [classification | handoff v3] ─► Wave 7 (remove CLI)
                              │                           │    (needs ux-improvements landed)
                              └──────────► Checkpoint 3/4 ◄┘
                                               │
                              Wave 8 [L2 | docs | cross-OS]  (parallel)
                                               │
                                     Wave 9 (close-out)
```

## Risks

| Risk | Mitigation |
|---|---|
| `check-ignore` does not treat files under an ignored parent as ignored | S1 decides the alternative before Wave 3 |
| Clone API cannot publish without replacement | S2. Fall back to cloning into a temporary file, then a no-replace rename or link |
| Registration identity is not stable across platforms | S3. When unsure, an untrusted record falls back to asking, which is safe |
| Phase 4 collides with the in-flight ux-improvements review | Prerequisite: land those changes first |
| Existing L2 remove tests encode the old "ignored needs consent" rule | Update them deliberately in Checkpoint 4 and list each change in the implementation log |
| Large include patterns (`node_modules/`) make create or remove slow | Documented. Cost proven by read counts. No wall-clock gate |
