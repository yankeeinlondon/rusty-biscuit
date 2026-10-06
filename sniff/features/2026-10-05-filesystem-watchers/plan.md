---
total_phases: 7
created: 2026-10-05
phase: 1
agent: claude/sonnet
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/linux.md
  - .claude/skills/os/macos.md
  - .claude/skills/os/windows.md
packages: []
---

# Plan: Filesystem process and watcher discovery

Spec: `2026-10-05-filesystem-watchers` (`spec.md`, same directory).

## Summary and definition of success

### Work required

Add a read-only, request-scoped query, `sniff::filesystem::query::query_path_usage`,
that reports processes associated with a file or directory tree, the evidence
linking each to the path, and structured coverage and limitations. Expose the
same captured report through `sniff filesystem query <path>`
(`--json`, `--plain`, `--target-only`, `--timeout <ms>`, `--perf`).

The work splits into:

1. **Contract layer (platform-neutral):** options, typed errors, report/coverage/
   evidence/limitation types, lossless path serialization, outcome rules, shared
   monotonic budget, root identity capture and recheck, tree identity collection,
   containment/identity matching, process-lifetime identity and merging, counters.
2. **Backends (OS-gated):** Linux/WSL2 (`/proc` descriptors, cwd, inotify `fdinfo`),
   macOS (`libproc` vnode descriptors and cwd; FSEvents reported unsupported),
   Windows (handle and loaded-module owners through a focused backend; no
   in-process terminate-thread behavior).
3. **CLI:** `filesystem query` subcommand, dispatch ahead of detection-plan
   construction, `FileReference` resolution, native positional argument,
   human rendering through `biscuit-terminal`, JSON/`--perf`, exit-code mapping.
4. **Docs and skills:** library/CLI READMEs, `docs/topics/filesystem-query.md`,
   `docs/cli/filesystem_query.md`, `docs/dependencies.md`, `sniff` and `os` skills,
   implementation log.

### Done means

- [ ] `just test` and `just lint` pass in the sniff area on macOS and Linux; the
      Windows and WSL2 legs pass for the mechanisms each can exercise.
- [ ] Every acceptance item (1-8) in the spec is covered by a named test (see the
      traceability table in Phase 7).
- [ ] A controlled child holding an open file and a cwd is discovered on every
      supported OS; a live inotify registration on Linux/WSL2 resolves to the right
      device/inode; macOS reports FSEvents as `unsupported`.
- [ ] Text and JSON are projections of one captured report; `--json --perf` is one
      valid JSON object; runtime errors use the stated JSON shape.
- [ ] No default-inventory scan, process cache, network, privilege elevation,
      process termination, or abandoned/killed worker thread exists.
- [ ] Docs describe current behavior; **planned** markers removed only by the change
      that lands the code; implementation log records every departure from the spec.
- [ ] Agent terminal state is "implementation complete, ready for review"; the
      feature directory is **not** moved to `_completed`.

### Waves

Waves are numbered globally. Tasks inside one wave are independent and can be
given to concurrent subagents; a wave starts only after the previous wave's
validation checkpoint passes.

| Wave | Phase | Contents |
| --- | --- | --- |
| 1 | 1 | Rulings recorded; spikes S1-S3 run in parallel |
| 2 | 2 | Contract types, budget, path serialization (parallel) |
| 3 | 2 | Root identity, tree identity, matching, process identity merge |
| 4 | 3 | Linux backend: `/proc` scan and fdinfo parser (parallel with Wave 5, 6) |
| 5 | 4 | macOS backend |
| 6 | 5 | Windows backend |
| 7 | 6 | CLI: args, dispatch, rendering, JSON, exit codes (parallel) |
| 8 | 7 | Docs, skills, dependency records, cross-OS verification |

Waves 4-6 depend only on Wave 3 and run concurrently.

## Phase 1: Rulings, spikes, and scaffolding decisions

Goal: remove the decisions and unknowns that would otherwise force rework in
later phases.

### Necessary Rules

The spec leaves these unresolved or ambiguous. Each needs an explicit ruling
(author or planning agent as marked) before the phase it gates starts. Recorded
in the implementation log; the spec stays unchanged.

- [x] **R1 - Deadline contract (author decision; gates Phases 2-6).** The spec's
      Open Question is unanswered. This plan proceeds on the spec's recommended
      option 1 (shared scan budget: stop scheduling at the deadline, report
      elapsed and exhaustion, a running native call may overrun). If the author
      chooses option 2 (supervised helper) or 3, Phases 2, 5, and 6 change
      materially (helper packaging, protocol). `wt` adoption of the latency
      contract is a separate decision outside this feature.
      Consequence under option 1: `--timeout` and CLI help must say "stops
      scheduling work", never "returns within".
- [x] **R2 - Hang-prone native operations (gates Phase 5).** Under option 1 the
      spec says such operations must be isolated safely or omitted with an
      explicit capability limitation. Ruling: omit any Windows operation that
      can block indefinitely and has no safe cancellation (notably
      `NtQueryObject` name queries on synchronous pipe/console handles) and
      report it as a structured limitation; do not add a helper executable in
      this release.
- [x] **R3 - Per-thread Linux enumeration.** Ruling: enumerate `/proc/<pid>` only
      (thread-group leaders), never `/proc/<pid>/task/*`, and read fd tables from
      the process directory.
- [x] **R4 - Result truncation.** The spec requires an explicit cap with limit and
      omitted count but no number. Ruling needed: a default cap on retained
      evidence records per process and in total (proposed: 1000 per process,
      10000 total), recorded in coverage when hit.
- [x] **R5 - Human-output truncation.** The spec says shortened output states how
      many records were omitted but no threshold. Proposed: show the first 50
      processes and 20 evidence lines per process in human output only; JSON is
      never truncated by the CLI.
- [x] **R6 - Outcome mapping for per-mechanism statuses.** Ruling needed on the
      precise `usable` rule: proposed "at least one applicable mechanism is
      `complete`, or is `partial` with at least one successful inspection."
      `unavailable` when every applicable mechanism is `failed` or `not_attempted`
      (or `partial` with zero successful inspections); `unsupported` when none is
      supported. (Spec: "retained trustworthy evidence or completed a usable
      inspection.")
- [x] **R7 - Serialized field names and enums.** The spec names concepts but not
      exact JSON keys. Ruling: snake_case keys matching the spec's terms
      (`processes`, `coverage`, `limitations`, `evidence`, `kind`, `status`,
      `outcome`); `kind` values `watch_registration`, `open_handle`,
      `working_directory`, `loaded_module`; path object `{display, native?}`
      with native tagged `unix_bytes` or `windows_utf16`; 64-bit identifiers
      as strings. Record a `schema_version` field on the report.
- [x] **R8 - Wider measurement.** The spec says no performance spike or
      multi-host timing campaign is required. Not scheduled. Author to decide
      separately if `wt` wants latency numbers before adopting the API.
- [x] **R9 - Crate selection criteria.** Selection of `procfs`, `libproc`, and a
      Windows binding (`windows-sys` vs `windows` crate) is an implementation
      decision, finalized by spikes S1-S3. Constraint: select only what the
      contracts need; each target OS gates its own dependency; record version,
      license, and native build requirements in `docs/dependencies.md`.
- [x] **R10 - Windows elevation.** Ruling: no optional elevated mode in this
      release; protected-process inspection limits are reported as limitations.
- [x] **R11 - Test-input and CI scope.** New fixtures/tests read no repository
      files outside the temp dir, so no `source-inputs` metadata is needed;
      confirm during Phase 7 via `just ci-local --plan`.

### Spikes

Each runs once, before the work it informs, on one host. No timing spike
(per the spec and R8). Output: a short finding appended to the implementation
log and a ruling in R9.

- [x] **S1 - macOS `libproc` scope (host: macOS).** Question: does
      `pids_by_type_and_path` give recursive or volume-wide matches, and do
      `proc_pidinfo` vnode-path calls expose event-only (`O_EVTONLY`) opens and
      the flag needed to label them? Decision: libproc as candidate source plus
      descriptor validation, or direct bindings. Gates Phase 4.
- [x] **S2 - Windows handle enumeration (host: native Windows, see `os` skill for
      how to reach it).** Question: can a focused backend enumerate handle owners
      for a directory tree and loaded modules using calls that cannot block
      indefinitely (R2), with size-retry for undersized buffers and without
      suppressing errors? Decision: binding crate and which calls are omitted.
      Gates Phase 5.
- [x] **S3 - Linux `procfs` fit (host: Linux or WSL2).** Question: does
      `procfs` 0.18 give descriptor/cwd/starttime access with per-process error
      visibility, or is a thin hand-rolled `/proc` reader smaller and safer? The
      fdinfo parser is hand-written either way (spec). Decision: dependency or
      none. Gates Phase 3.

### Tasks

- [x] **Record rulings** - Write R1-R11 outcomes (and any author overrides) into
      `implementation-log.md` in the feature directory.
- [x] **Run spikes** - S1, S2, S3 run concurrently (Wave 1) on their respective
      hosts; each records crate decision and any capability limitation text.
- [x] **Load skills** - Implementers load `sniff`, `cli`, `biscuit-terminal`,
      `os`, `rust-testing`, `rust` before their wave.

Checkpoint: R1 answered (or explicitly accepted as option 1) and S1-S3
decisions logged. Do not start Phase 2 backend-facing API shape without R1.

## Phase 2: Platform-neutral contract (library core)

Module: `sniff/lib/src/filesystem/query/` (re-exported from
`sniff::filesystem`). Must compile on all OSes with default features; no
backend code yet.

### Wave 2 (parallel)

- [ ] **Options and errors** - `PathUsageOptions` (default recursive, builder for
      target-only, deadline default 2 s); `PathUsageError` typed variants:
      invalid option (zero/unrepresentable duration), missing target, unsupported
      target kind (socket/device/FIFO, rejected from `symlink_metadata` before any
      open), root-validation timeout, root identity failure. Each has a stable
      `kind` string for the CLI JSON error.
- [ ] **Report types** - `PathUsageReport`, `ProcessRecord`, `Evidence`,
      `EvidenceKind`, `Coverage`, `CoverageStatus` (`complete`, `partial`,
      `unsupported`, `failed`, `not_attempted`), `Limitation`, `Outcome`
      (`usable`, `unavailable`, `unsupported`), `WatchInfo` (mask, recursion
      known, descriptor, watch id), `NativeIdentity`. Unavailable fields are
      explicit (`Option` serialized as an explicit state, not omitted silently).
- [ ] **Lossless path object** - `{display, native?}` serializer/deserializer for
      `OsStr`/`Path`: Unix bytes, Windows UTF-16 units (including unpaired
      surrogates); `native` emitted only when `display` is not round-trippable.
      64-bit native ids and creation tokens serialize as strings.
- [ ] **Shared budget** - one monotonic `Budget` created on entry; `remaining()`,
      `expired()`; used by every phase; wall-clock start/end timestamps for
      presentation, `Instant` for elapsed. A `not_attempted` coverage record with
      a reason is the only way a skipped phase is represented.
- [ ] **Counters** - add named counters in `performance::counters` for process
      enumeration, descriptor inspection, tree identity reads, and watch-
      registration reads; count attempts once including failures; no timing or
      formatting cost when collection is off.

Checkpoint: types compile on macOS/Linux/Windows targets (`cargo check` per
target where available); serde round-trip tests for the path object pass.

### Wave 3 (parallel)

- [ ] **Root resolution and identity** - `symlink_metadata`, resolve root alias
      once and keep both spellings; capture native root identity (dev/ino or
      Windows file id + volume serial); budget exhaustion here is the typed
      root-validation timeout; recheck identity before completing when budget
      allows, else record limitation and mark coverage partial; root replacement
      keeps prior evidence and adds a root-change limitation.
- [ ] **Tree identity collection** - single streaming walk (`walkdir`, no
      following of descendant symlinks/junctions/reparse points, no file body
      reads, no per-entry handle retention) building identity -> in-scope paths
      (an identity may map to several paths: hard links; none preferred). Include
      hidden/ignored entries (no `.gitignore` filtering). Unreadable entries
      become aggregated limitations with counts and representative examples.
      Budget checked between entries; exhaustion yields partial tree coverage that
      propagates to every dependent mechanism.
- [ ] **Containment and matching** - identity-first matching, native component-
      aware path comparison fallback (never string prefix; `/work/app-copy`
      outside `/work/app`); Windows drive/UNC/verbatim spellings, short-name
      aliases, and per-directory case sensitivity (no blanket lowercasing);
      deleted object never matched by former path alone; observed alias kept
      separately from in-scope paths.
- [ ] **Process lifetime identity** - `ProcessKey` built from PID plus native
      creation token (or retained handle); merging rule: conflicting known start
      times never merge; missing start time keeps records separate with
      identity-association-uncertain flag; enrichment (name, exe, user) only after
      verifying the same lifetime; evidence is preserved when enrichment fails or
      the process vanishes. Identity enrichment through `sysinfo` on candidate
      PIDs only, no full refresh, no cmdline/environ.
- [ ] **Orchestrator and outcome** - `query_path_usage` runs: validate options,
      resolve root, collect tree identity, enumerate processes, run each
      applicable backend mechanism, enrich, recheck root, assemble report,
      compute `Outcome` per R6, sort by PID then evidence path key, then kind,
      then descriptor/watch id. Backend trait (`UsageBackend`) accepts injected
      observations so tests can retain/replay them. Detector-owned handles
      closed before inspecting self; exact detector handles omitted; other self
      usage kept.

Checkpoint (Phase 2 exit): unit tests with an injected fake backend cover: all five
coverage statuses, outcome rules, budget exhaustion before/during each phase,
root replacement, PID reuse (same PID/different start), missing start time,
hard-link alias, sibling-prefix rejection, descendant symlink not expanded,
non-UTF-8 and (Windows-gated) unpaired-surrogate path round trip, hidden files
in scope. Counter test proves one tree walk regardless of process count.

## Phase 3: Linux and WSL2 backend

Gated `#[cfg(target_os = "linux")]`. Depends on Wave 3 and S3.

### Wave 4 (parallel, with Phases 4 and 5)

- [ ] **Process and descriptor scan** - enumerate `/proc/<pid>` leaders (R3);
      read `/proc/<pid>/stat` start time as the creation token; for each fd,
      `fstat`-equivalent identity via `/proc/<pid>/fd/<n>` metadata (not link text,
      which is descriptive for unlinked files); read cwd the same way; classify
      `open_handle` and `working_directory`; record access flags from
      `fdinfo` `flags` when present. Permission denial counted and surfaced
      (`partial`), never converted to no matches. Descriptor replacement during
      inspection recorded as a limitation. Mount/PID namespace limitation recorded
      when device identities cannot be compared reliably.
- [ ] **inotify fdinfo parser** - parse `inotify wd:<dec> ino:<hex> sdev:<hex>
      mask:<hex>` lines; normalize the kernel `sdev` encoding to match
      `st_dev` (major/minor decode); ignore unknown extra fields; malformed
      required fields reported without discarding valid sibling lines; preserve
      multiple registrations per inotify descriptor (fd number + wd). Identify
      inotify fds by `fdinfo` presence of inotify lines, not by link text alone.
      Never read from the inotify fd itself (no event queue consumption).
- [ ] **Watch matching** - a registration matches only when its inode is the
      target or an included descendant; ancestor watches are excluded and
      reported as a limitation; non-recursive target watch kept under
      `--target-only`; unmatched registrations never gain an invented path; mask
      and `recursive_known=false` (inotify has no native recursion) recorded.
- [ ] **Coverage records** - separate records for process enumeration, descriptors,
      cwd, inotify; a fanotify record fixed at `unsupported`; "complete
      descriptor scan is not complete watcher discovery" stated in the
      limitation text.
- [ ] **Linux tests** - controlled child (readiness handshake over a pipe,
      guaranteed cleanup via guard on success/failure/timeout, no sleeps) with an
      open file, cwd in the tree, and a live inotify watch (using the `inotify`
      crate as a **dev-dependency only** to create it). Assert correct dev/inode,
      sibling-prefix rejection, ancestor-watch exclusion, multiple watched paths,
      hard-link alias, thread-not-counted-as-process.

### Input Robustness Matrix (fdinfo parser)

The parser reads `/proc/<pid>/fdinfo/<fd>` text; its load-bearing fields are
`wd`, `ino`, `sdev`, and `mask` (they decide the match and the evidence). Each
shape has a defined outcome. One table-driven test walks the whole matrix from a
real captured fdinfo fixture with one edit per cell, asserts through the public
`PathUsageReport` (not the parser's return), and includes a control row proving
the unedited fixture matches.

| Shape | Applied to `wd`/`ino`/`sdev`/`mask` | Defined outcome |
| --- | --- | --- |
| control (unedited) | all four valid | match evidence produced |
| absent (line lacks the field) | field omitted | that line malformed: no evidence, malformed-record limitation; sibling lines still processed; inotify coverage `partial` |
| explicit null analog (`field:` with empty value) | `ino:` empty | same as malformed; never read as inode 0 |
| wrong type (non-hex in hex field, non-decimal `wd`) | `ino:zz`, `wd:x` | malformed; never parsed as 0 or skipped silently |
| wrong type, one element of several lines | one bad line among valid | only that line dropped; counted in limitation with example |
| wrong type, every line | all lines bad | no evidence; coverage `partial` with zero successes (feeds R6 outcome), not `complete`/empty |
| empty (file has no inotify lines) | fdinfo for non-inotify fd | not an inotify descriptor; no limitation |
| empty (zero-length fdinfo) | `""` | treated as unreadable record limitation, not as "no registrations" |
| duplicate key within a line | `ino:1 ino:2` | malformed line; never last-wins |
| duplicate registration (same wd twice) | repeated line | both preserved as distinct evidence (spec: do not collapse) |
| trailing/invalid content | valid line + garbage line | garbage reported, valid line kept; trailing garbage on the same line after known fields is accepted only if it is a documented `key:value` extra |
| unknown additional fields | `foo:1` appended | accepted and ignored |
| overlong hex / overflow | `ino:` > u64 | malformed; never truncated |

Code smells to grep before declaring done: `#[serde(default)]`,
`unwrap_or_default()`, `.ok()` and `filter_map` over parse results in the
fdinfo module.

Checkpoint: Linux L1 tests green on a Linux host (and WSL2 via the nextest
archive path from the `os` skill). WSL2 runs this backend; it does not inspect
native Windows processes.

## Phase 4: macOS backend

Gated `#[cfg(target_os = "macos")]`. Depends on Wave 3 and S1.

### Wave 5 (parallel, with Phases 3 and 5)

- [ ] **Descriptor and cwd inspection** - per-process vnode descriptor listing and
      cwd via `libproc` (or bindings per S1); identity from the vnode info
      (dev/ino) matched against the collected tree identities; `open_handle` and
      `working_directory` evidence; event-only opens stay `open_handle` with an
      `event_only` attribute; permission denial (other users' processes) recorded.
- [ ] **Candidate source** - if S1 shows `pids_by_type_and_path` is useful, use it
      only as a candidate PID source; candidates are validated through descriptor/
      cwd evidence; volume-wide matches never reported as tree matches.
- [ ] **FSEvents coverage** - fixed `unsupported` record with the capability
      explanation ("no supported systemwide FSEvents subscription inventory").
- [ ] **macOS tests** - controlled child with open file and cwd (handshake and
      cleanup as in Phase 3); `/var` vs `/private/var` alias test via the
      symlinked temp dir trap in the `os` skill; assert the FSEvents record is
      `unsupported`; no test depends on a live FSEvents watcher being invisible
      (spec item 5).

Checkpoint: macOS L1 tests green locally.

## Phase 5: Native Windows backend

Gated `#[cfg(windows)]`. Depends on Wave 3, R2, and S2. Not WSL2.

### Wave 6 (parallel, with Phases 3 and 4)

- [ ] **Handle owners** - focused backend (binding per S2); do not use
      `filelocksmith` in-process; enumerate system handles with a size-retry
      loop for undersized buffers (no silently dropped entries); resolve only
      handles that can be inspected without risking an indefinite block (R2);
      compare by file id + volume serial where possible, else Windows-aware path
      comparison; evidence `open_handle`; protected-process and architecture
      (WOW64) inspection failures recorded as limitations, never swallowed.
- [ ] **Loaded modules** - enumerate modules per candidate process as a distinct
      `loaded_module` mechanism with its own coverage record; handle failures
      (access denied, process exited) recorded.
- [ ] **Unsupported records** - cwd and `ReadDirectoryChangesW` subscription
      inspection reported `unsupported` with explanations; state that a matched
      handle does not establish deletion blocking.
- [ ] **Identity and cancellation** - process creation time as the native
      creation token; open process handle retained for the lifetime of
      enrichment; no threads terminated, no worker left running past return
      (any worker used must be cooperatively cancelled and joined, with counter
      collector propagated).
- [ ] **Windows tests** - controlled child with open file and directory handle
      (handshake, guard cleanup); case-sensitive directory test; verbatim/UNC/
      short-name spelling tests; UTF-16 path with an unpaired surrogate round-trip
      (serialization test runs on any host; matching test Windows-gated).
      Reach the Windows host per the `os` skill.

Checkpoint: Windows L1 tests green on the native Windows host; any capability
omitted under R2 documented as a limitation in report output and tests.

## Phase 6: CLI

Business logic stays in the library; the CLI resolves, calls, and renders.
Depends on Phase 2 and at least one backend for end-to-end tests (all three
for the final checkpoint). Load the `cli` and `biscuit-terminal` skills.

### Wave 7 (parallel)

- [ ] **Args** - `filesystem query <PATH>` as a subcommand of `filesystem` in
      `cli/src/args/mod.rs`; `PATH` is an `OsString`/`PathBuf` positional (non-
      UTF-8 accepted); `--target-only`; `--timeout <MS>` with a clap value parser
      rejecting zero, negative, malformed, and overflowing values (exit 2);
      bare `filesystem` behavior unchanged; reject `--refresh-remotes` and
      `--latest-versions` when combined with `query` through clap
      (usage error, exit 2); `--base` does not rebase. Subcommand help text per
      R1 wording; dynamic path/reference completions reused.
- [ ] **Dispatch** - handle `filesystem query` before ordinary detection-plan
      construction (no repository inventory, remote, or network work); resolve
      textual references through `biscuit-file` `FileReference` (local only; non-
      local rejected before probing; failure is a typed target error, never a
      current-directory fallback); keep the original reference separate from the
      native path; ordinary non-UTF-8 paths bypass string conversion.
- [ ] **Human rendering** - new module under `cli/src/output/filesystem/` using
      `biscuit-terminal` `TerminalRenderable` (`Prose`, lists, `Table`): target,
      processes with PID and available identity, matched paths and evidence,
      then coverage and limitations. Coverage/limitations on stdout, also in
      `--plain`. Literal text escaped for Prose; control characters and escape
      sequences and newlines neutralized in display only; truncation per R5 with
      an omitted-count line; never overstates deletion contention. Rendering
      consumes the captured report (no second query).
- [ ] **JSON and perf** - `--json` serializes the library report once before
      writing; `--json` beats `--plain`; with `--perf`, use `print_json_value` so
      data and `performance` are one document; rendered performance summary on
      stderr; stdout always valid JSON.
- [ ] **Exit codes and errors** - `usable` -> 0; `unavailable`/`unsupported` ->
      1 with full report on stdout; typed query or resolution error -> 1 with
      stderr diagnostic (and `{"error":{"kind","message"}}` on stdout in JSON
      mode); output-write failure via the CLI error path; match presence never
      changes the exit code; no special-empty-result failure.
- [ ] **CLI tests** - in `cli/tests/l1` using the existing isolated process
      fixture and runtime-resolved test binary: text/JSON equivalence from an
      injected/retained observation (hidden test seam, not a public flag);
      partial, empty, skipped, unsupported, and total-failure outputs with exit
      codes; `--json --perf` single valid object; runtime error JSON shape;
      invalid args exit 2 including combined inventory flags; non-UTF-8 argument
      (Unix-gated); hostile path/process names (markup, ANSI, newline) not
      interpreted; stdout/stderr separation; `--plain` retains coverage.

Checkpoint: `sniff filesystem query .` works on the dev host in text, plain, and
JSON modes; `sniff filesystem` bare output snapshot unchanged.

## Phase 7: Documentation, skills, verification

### Wave 8 (parallel)

- [ ] **Topic and CLI docs** - update `docs/topics/filesystem-query.md` and
      `docs/cli/filesystem_query.md` to the finalized coverage statuses (add
      `not_attempted`), outcome/exit rules, serialization, and the R1 deadline
      wording; remove **planned** markers only for built behavior; docs never name
      a feature/fix directory; audience is a developer new to the repo (compact
      examples, a Mermaid flow of the query phases and budget).
- [ ] **READMEs and dependency docs** - sniff library and CLI READMEs;
      `docs/dependencies.md` records each selected crate's version, license, native
      build requirements, runtime coverage; note `inotify` as dev-dependency only.
- [ ] **Skills** - update `.claude/skills/sniff/` (new API, counters, request
      tier note) and `.claude/skills/os/` (per-OS evidence facts learned in
      Phases 3-5, including any Windows/WSL2/macOS traps found).
- [ ] **Implementation log** - `implementation-log.md` with rulings, spike
      outcomes, crate selections, and every departure from the spec.
- [ ] **Comment and doc drift pass** - every behavior-bearing symbol's `///`/`//!`
      reviewed; drift resolved in favor of the code and reported.

### Verification

- [ ] **Local** - `just test` and `just lint` in `sniff/`; `just test` on a Linux
      host and WSL2 (nextest archive per `os` skill) and native Windows.
- [ ] **CI scope review** - `just ci-local --plan` reviewed before pushing; no
      area-keyed stores; confirm no extra `source-inputs` required (R11).
- [ ] **Acceptance traceability** - table in the implementation log mapping each
      spec acceptance item 1-8 to its tests:

| Spec item | Covered by |
| --- | --- |
| 1 text/JSON equivalence, no inventory | Phase 6 equivalence + counter tests (Phase 2) |
| 2 controlled child per OS | Phases 3, 4, 5 child tests |
| 3 Linux inotify correlation | Phase 3 tests and fdinfo matrix |
| 4 denial, exit, PID reuse, malformed, budget, missing identity | Phase 2 fake-backend tests; Phase 3 matrix |
| 5 macOS FSEvents unsupported | Phase 4 coverage test |
| 6 tree scope, aliases, links, Unicode, Windows spellings | Phases 2, 5 |
| 7 CLI outputs, exit codes, JSON, hostile text | Phase 6 tests |
| 8 hidden/ignored, self usage, threads, no all-unsupported cheat | Phases 2, 3, 4, 5 |

- [ ] **Handoff** - set spec status per lifecycle, report "implementation
      complete, ready for review"; do not move to `_completed` and do not run
      `just complete`.
