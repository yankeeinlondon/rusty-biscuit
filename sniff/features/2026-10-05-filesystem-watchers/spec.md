---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
implemented: false
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-05
review_iterations: 0
related:
  - 2026-10-05-graceful-file-contention
human_review: false
human_review_items:
  - |-
    **Answered 2026-10-06 by the author: Option 1.** `wt` adopting the "stops scheduling work" contract stays a separate decision outside this feature (R8).

    **How strict is the `--timeout` / two-second limit? (Phase 2 is built on Option 1; please confirm before Phase 6 writes the CLI help)**

    The spec leaves this as an open question for you. Some operating-system calls can hang (for example on a stalled disk or network share), and no safe way exists to interrupt them inside the caller's program. Phase 2 has now built the public types and the budget logic on Option 1 below, as the plan allowed. Choosing Option 2 or 3 now means reworking that code.

    - **Option 1 - "stop starting new work after the limit" (the spec's recommendation; this is what the plan assumes).** Sniff keeps what it found so far, reports that the time budget ran out, and returns. A single slow system call that is already running can make the query take longer than the limit.
      - Pros: simple, no extra programs to ship, honest about what was checked.
      - Cons: `wt` and other callers cannot treat the limit as a hard guarantee of when the function returns.
    - **Option 2 - run the whole scan in a separate helper program that is killed at the limit.**
      - Pros: the caller is never stuck behind a hung call.
      - Cons: Sniff has to ship and locate an extra executable for every program that uses the library, plus a protocol between the two, which is a lot of work for every platform.
    - **Option 3 - a strict limit only for a narrow set of checks that can never hang.**
      - Pros: a hard promise where it is possible.
      - Cons: removes most of the useful evidence, especially on Windows.

    **Update after Phase 6 (2026-10-06):** the `sniff filesystem query` command is built with Option 1 wording: `--timeout` "stops scheduling discovery work" and the help says it "is not a limit on when the command returns". Choosing Option 2 or 3 now also means rewording that help text and `sniff/docs/cli/filesystem_query.md`.

    **Recommendation: Option 1** for macOS and Linux, where the spikes found no call that hangs in normal use. Windows is decided separately in the next item. Please also confirm that `wt` will accept "stops scheduling work" wording before it adopts the API (`wt` adoption itself is outside this feature).
  - |-
    **Answered 2026-10-06 by the author: Option D (added after spikes S2d-S2f), not A, B, or C.** Instead of resolving another process's handle, Sniff opens each tree entry itself (attribute-only, full sharing) and asks Windows which processes have it open (`NtQueryInformationFile`, `FileProcessIdsUsingFileInformation`). This query did not hang in the `LockFileEx`, synchronous `ReadDirectoryChangesW`, or pipe-server scenarios, needs no elevation and no access to other processes, and ships no helper. It costs about 6 ms per entry at 100k system handles, so it runs folders first, then files, on a bounded thread pool within the Option 1 budget, and reports what it did not reach. Details and numbers: Phase 1 addendum in `implementation-log.md`.

    **Windows: how to read which file another program has open without risking a hang (needed before Phase 5, the Windows backend)**

    **Update after Phase 5 (2026-10-06):** without your answer, Phase 5 built everything on Windows that both options need: Sniff finds programs and libraries (DLLs) loaded from inside a folder, and reliably identifies each process. It does **not** yet find files that other programs hold open on Windows; it says so in every report ("unsupported", with the reason). That is exactly what Option A ships, so choosing A needs no more code. Choosing B means adding the helper on top of what exists. Phase 6 (the command-line tool) does not depend on this choice, but the open-file test on Windows and the Phase 5 "Handle owners" task stay unfinished until you answer.

    A test on the Windows build machine found a problem. To learn which file another program's handle points to, Sniff must ask Windows for the file's ID. If that other program is itself waiting on the same file (for example, waiting for a file lock), the question hangs until the other program stops waiting, which might be never. Windows' "cancel" call does not help. The spec forbids leaving a stuck background thread behind and forbids killing threads inside the caller.

    A follow-up test showed that putting the question in a small separate helper process works: the helper can be killed at a deadline in about 1 ms, and the other program is unaffected (7 of 7 trials).

    - **Option A - leave it out.** On Windows, Sniff reports that open-file and current-directory discovery are unsupported, and shows only loaded modules (DLLs) and process identity.
      - Pros: no new moving parts; meets the spec as written.
      - Cons: Windows loses the main thing this feature exists for (finding what holds files inside a worktree).
    - **Option B - a small Windows-only helper process** for just the per-file ID questions. The main scan stays in-process; any question that does not answer within a short deadline (about 250-500 ms) has its helper killed and is reported as "could not be identified".
      - Pros: keeps Windows evidence; the caller can never get stuck; the cost is limited to one platform and one narrow operation.
      - Cons: Sniff must ship and find a helper executable for library users too (the spec rules out relying on an installed `sniff` command or on PATH, so it is a separate small binary), plus a tiny message format; more Phase 5 work. A handle stuck inside a broken network-drive driver might still resist being killed, so this is a strong mitigation, not an absolute guarantee.
    - **Option C - ask anyway in-process and accept the rare hang** (a stuck thread is abandoned).
      - Pros: simplest code.
      - Cons: breaks an explicit spec rule; a long-running caller could slowly accumulate stuck threads.

    **Recommendation: Option B**, because it keeps the feature useful on Windows without breaking the spec's safety rules. If the packaging cost is not acceptable for this release, choose **Option A** and treat Windows handle discovery as a follow-up feature; do not choose Option C.
message_to_agent: |-
  Phases 2-6 are built. Read the Phase 6 section of `implementation-log.md` first; Phase 5's notes on Windows still apply.

  Both review items were answered on 2026-10-06 (see `human_review_items` and the "Phase 1 addendum" in `implementation-log.md`):
  - R1 (deadline contract): Option 1 confirmed. The existing "stops scheduling work" wording in the CLI help and docs stays as it is.
  - R2 (Windows handle owners): Option D, the file-side query. Before Phase 7, finish the Phase 5 "Handle owners" task as now written in `plan.md`, plus the open-file/directory-handle part of "Windows tests". Read the S2d-S2f findings in the log and the `os` skill (`windows.md`, "Asking which processes have a path open") first. Then run Phase 7.

  Phase 6 facts that matter for Phase 7:
  - The CLI is `sniff/cli/src/{args/filesystem.rs,commands/filesystem_query.rs,output/filesystem/query.rs}`; tests are `sniff/cli/tests/l1/filesystem_query.rs` (18) plus unit tests in the CLI lib target.
  - `sniff/docs/cli/filesystem_query.md` was rewritten to the shipped behavior and the topic page's "Planned: CLI" marker was removed in Phase 6, so Phase 7 reviews those pages rather than writing them. The CLI README gained one line only; the library README was not touched.
  - Hidden test seam: `SNIFF_FILESYSTEM_QUERY_REPLAY=<report.json>` makes the command render that report instead of querying. It is documented as "not a supported interface". The test fixture scrubs inherited `SNIFF_*` variables.
  - Only sigil file references (`@ & ^ ~ vault:`) go through biscuit-file; ordinary paths go to the library unchanged. A directory resolves as the first `NonFile` candidate because biscuit-file matches regular files only.
  - The three pre-parse `std::env::args()` scans in `sniff/cli/src/commands/mod.rs` were switched to `args_os` (non-UTF-8 argv panicked before clap).
  - Gates: macOS `just test` 3270/3270, `just lint` and strict clippy (`--all-targets -D warnings`, with and without `test-fixtures`) clean; Linux 138/138 and native Windows 134/134 for the `filesystem` filter via `just cross-check sniff-cli`; WSL2 not run (no WSL-specific CLI path).
  - The Phase 7 acceptance-traceability row for spec item 7 maps to `filesystem_query.rs`; item 1's "no inventory" proof is `a_live_json_perf_query_is_one_object_and_does_no_inventory` (counters limited to `filesystem.query.*`/`filesystem.io.*`).
---

# Filesystem process and watcher discovery

## Outcome

Give library callers, including `wt`, a read-only query with a shared work budget
for processes associated with a file or directory tree. Expose the same
observation through `sniff filesystem query <path>`. Report process identity,
the evidence linking it to the path, and the limitations of discovery.

This feature discovers filesystem usage. A watcher registration, open handle,
or working directory does not by itself prove that deletion will fail. An empty
result never proves that no process is watching or writing. Callers own the
policy for warning, refusing, or proceeding with an operation.

The first release supports regular files and directories, including hidden and
ignored entries in a directory query. Git ignore rules and package boundaries
do not limit discovery. It does not identify which process performed a write,
monitor future activity, or inventory every possible watcher mechanism.

## Motivation and established limits

Concurrent writers can recreate files while a worktree is being removed.
Identifying a relevant process before removal helps users stop it themselves.
Discovery belongs in Sniff rather than in each filesystem client.

| Environment | Discoverable evidence | Important limitation |
| --- | --- | --- |
| Linux and WSL2 | Open descriptors, working directories, inotify registrations from `/proc/<pid>/fdinfo` | Permissions restrict visibility; watch registrations identify device/inode rather than ordinary paths; polling and other mechanisms remain outside coverage |
| macOS | Open vnode descriptors, working directories, event-only opens where available | No supported systemwide FSEvents subscription inventory; a recursive watcher can have no visible descriptor naming its watched tree |
| Native Windows | Directory/file handle ownership and loaded modules; process identity | A handle can permit deletion; ownership does not identify a `ReadDirectoryChangesW` subscription or establish its sharing mode |

WSL2 uses Linux discovery and does not inspect native Windows processes.
Polling watchers have no required kernel watch registration on any platform.
All observations can become stale immediately, and PID reuse must be accounted
for when merging observations.

Visibility is limited to processes exposed to the caller by the OS and its
permissions. Linux PID and mount namespaces can hide processes or give the same
path spelling a different meaning. Do not enter another process's namespace or
infer that its path refers to the caller's file. Unsupported watcher mechanisms,
including Linux fanotify in this release, are explicit coverage limits.

## Library API

Add a focused module `sniff::filesystem::query`, re-exporting its primary types
from `sniff::filesystem`. Do not expand ordinary host/repository inventory or
invoke full detection to answer this query.

The public entry point is:

```rust
pub fn query_path_usage(
    path: &Path,
    options: &PathUsageOptions,
) -> Result<PathUsageReport, PathUsageError>;
```

`PathUsageOptions::default()` queries the target and, for a directory, its
descendants. A builder can select the target only. The default deadline is
two seconds with a configurable positive duration. Deadline exhaustion returns
collected evidence and explicit incomplete coverage, not an empty success.
Enumeration and any helper work must be interruptible or otherwise bounded;
merely checking a deadline around a potentially blocking native call is
insufficient to guarantee return by that deadline. This strict return-time
requirement is provisional: resolve the deadline question under **Open
Questions** before finalizing the API or relying on it in `wt`.

Use one monotonic budget for root resolution, tree identity collection, process
enumeration, evidence inspection, and optional identity enrichment; do not give
each phase another two seconds. Report wall-clock timestamps for presentation
and monotonic elapsed time for duration. Preserve evidence before enriching
identity, and stop scheduling work when the budget expires. Zero durations and
durations that cannot form a valid deadline are typed invalid-option errors.
The library budget starts on entry. CLI file-reference resolution precedes that
entry and is included in CLI performance timing, outside the library budget;
the final deadline decision must not advertise an end-to-end CLI guarantee
without also accounting for resolution and output.

No automatic privilege elevation, termination of observed processes, target
filesystem mutation, network requests, or subprocess that can wait indefinitely.
Never forcibly terminate a thread inside the caller's process or return while
an abandoned worker continues scanning. Any implementation-owned helper must
have explicit cancellation, cleanup, and performance-collector propagation.

### Report contract

The report carries:

- The requested native path, resolved absolute target, native root identity,
  file/directory kind, recursive scope, observation start/end timestamps,
  configured budget, elapsed duration, and whether the budget was exhausted.
- `processes`: deduplicated process records containing PID, optional start time,
  name, executable path, user identity, and a list of evidence records. Missing
  identity fields stay explicitly unavailable. A disappeared process or failed
  enrichment never erases already collected evidence.
- `coverage`: process enumeration, tree identity collection when needed, and a
  separate record for each platform mechanism listed above, plus known
  unsupported watcher mechanisms. Each record identifies its scope, status,
  counts of attempted/successful inspections where observable, and structured
  limitations. A complete descriptor scan is not complete watcher discovery.
- `limitations`: permission denials, unavailable process identity, disappeared
  processes, unreadable tree entries, unsupported mechanisms, and exhausted
  budgets. Repeated failures may be aggregated with counts and representative
  examples; permission denial must never be silently converted to no matches.

Coverage statuses have these meanings:

| Status | Meaning |
| --- | --- |
| `complete` | Finished the mechanism over its declared visible-process and path scope without a known omission; this is not a systemwide snapshot |
| `partial` | Inspection started, but permissions, races, malformed records, an incomplete tree, or exhausted budgets left known gaps; counts identify how much usable work was retained |
| `unsupported` | The OS or chosen backend cannot inventory this mechanism; explain which capability is absent |
| `failed` | Attempted the mechanism but obtained no usable inspection because of an operational error |
| `not_attempted` | A supported mechanism was not started, for example because an earlier phase exhausted the shared budget; record the reason |

**Reader's note:** `not_attempted` is added to distinguish skipped work from a
failed native call or an unsupported OS capability. Without it, a timeout before
descriptor inspection could misleadingly look like a completed empty scan.
Propagate incomplete tree and process enumeration into every coverage record
that depends on them. Successfully inspecting an empty visible process set can
be complete; merely enumerating PIDs or resolving the root is not usable usage
discovery.

The library owns the overall report outcome (`usable`, `unavailable`, or
`unsupported`), so consumers do not invent different success rules. `usable`
means at least one applicable usage mechanism retained trustworthy evidence or
completed a usable inspection, even with no matches. `unavailable` means
supported usage mechanisms yielded neither, including when every one failed or
was left unattempted. `unsupported` means none is supported. Other mechanism
failures remain visible even when the overall outcome is usable. Inability to
read an executable/name is an enrichment limitation, not failure of an otherwise
successful handle inspection.

Evidence kinds are `watch_registration`, `open_handle`, `working_directory`,
and `loaded_module`. Each record includes its discovery mechanism, matched
path when known, and native identity when relevant (device/inode on Linux).
Watch evidence includes the mask and whether recursive coverage is known;
several per-directory watches do not imply a native recursive subscription.
Preserve the descriptor and watch identifier where supplied by the backend so
distinct registrations are not collapsed into a single path match. A PID-only
candidate list cannot establish an evidence kind or matched path: validate
candidates with a mechanism that supplies those facts, or report the coverage
gap without manufacturing evidence.
An event-only macOS descriptor remains an `open_handle` with an event-only
attribute unless an actual subscription has been established.

Open access flags may be included when observable. Absence of write access
does not prove that the process cannot write through another path or handle.
An unmatched inotify registration must not acquire an invented path. Unknown
deletion-blocking behavior stays unknown; the first release does not produce a `blocks_delete`
boolean or label every match a watcher.

Use native path and process-identity types internally, preserving non-UTF-8
paths. Serialized path values must preserve an explicit lossless native
representation when a display string cannot round-trip; display strings alone
must never drive matching or subsequent filesystem operations. Sort output by
PID and evidence path/kind for stable presentation. Process observations with
conflicting known start times must not be merged under one PID.

Use a consistent serialized path object with a human-readable `display` field
and an optional `native` field. When needed for lossless round-trip, `native`
contains an encoding tag and an array of Unix bytes or Windows UTF-16 code units;
this also preserves unpaired Windows surrogates. Use the same representation for
target, executable, evidence, and limitation paths. Native identifiers and
creation tokens too large for exact JSON-number consumers use strings.

The OS can reuse a PID after a process exits. Deduplicate records belonging to
one process lifetime, not by PID alone. Use a retained native process handle or
process-directory handle where available, and verify that later identity
enrichment refers to that same process lifetime. A native creation token (the
OS value identifying when the process started) retains its precision; a rounded
display timestamp is not an identity key.
Without a handle or comparable creation token, keep separately acquired records
separate and mark their identity association uncertain. In particular, never
attach the name of a newly started process to an exited process's evidence.
Sort evidence with native path keys and mechanism/descriptor identifiers as
tie-breakers; sorted presentation does not make a truncated scan deterministic.

Include genuine usage by the querying process; callers own exclusions. Do not
hide shells, ancestors, process groups, or programs by name. Close temporary
tree-walk handles before inspecting the query process, and omit only exact
handles opened by the detector itself. Do not read command lines, environments,
file contents, or enumerate users merely to enrich a matching process.

### Path and failure semantics

Resolve textual CLI path references through biscuit-file's
[`FileReference`](../../../biscuit-file/lib/src/file_reference/mod.rs), which
interprets the repository's file-reference syntax; the focused Sniff library API
accepts a real `Path`. Preserve the caller's original reference separately from
the native path passed to Sniff. A non-UTF-8 ordinary CLI path must remain a
native path argument rather than pass through a lossy string conversion. Reject
nonlocal references before probing; do not download, search remote repositories,
or invoke full inventory as part of resolution. Failure to resolve a reference
is a typed target error, never a fallback to the current directory.

Resolve a supplied root alias once and report both spellings. A root symlink or
junction queries its resolved file/directory. Do not follow descendant directory
symlinks, junctions, or reparse points, even when they point back inside the tree;
inspect the link entry without expanding its target. This prevents cycles and
accidental scope expansion. Reject sockets, devices, and FIFOs as unsupported
target kinds before any operation that could read or wait on them.

Match file identity where available, and otherwise use native component-aware
path comparison; never use a raw string prefix to determine containment.
Windows comparisons must account for drive/UNC/verbatim spellings and short-name
aliases, and must respect case-sensitive directories rather than unconditionally
lowercasing every path. A missing or deleted object cannot be matched solely
because its former path now names a different object.

For example, `/work/app-copy` is outside `/work/app`. A handle opened through a
hard link outside the tree can still identify a file present inside the tree:
retain the observed alias separately from the in-scope path(s) proving the match.
An identity can map to several in-scope paths; do not invent one preferred path.
Use identity matching across Linux mount namespaces only when identities can
be compared reliably; otherwise record the scope limitation. Text returned by
`/proc` for an unlinked file is descriptive, not an ordinary path to open.

A watch registration is a match only when its watched object is the target or
an included descendant. A watch on the parent or another ancestor is outside
this query, even if it could observe some events involving the target. Report
that limit; do not label every process watching `/work` as using `/work/app`.
A nonrecursive watch of the target directory remains target evidence under
`--target-only`, but does not imply observation of all descendants.

Invalid options, missing targets, and inability to establish the root's
identity are typed query errors. Individual process or descendant failures
produce a partial report. If all applicable mechanisms fail, the report still
contains their failure statuses; callers can distinguish this from a usable
partial observation. No result may claim an atomic snapshot.
Budget exhaustion before the root identity is captured is a typed root-validation
timeout, since the library cannot yet construct a report about a verified target.

Retain the captured root identity and recheck it before completing the query
when the shared budget permits. An exhausted budget or failed recheck records
that the root's continued identity could not be verified and makes coverage
partial; it does not authorize extra work beyond the budget.
If the root disappears, moves, or is replaced after capture, keep prior evidence
and mark coverage partial with a root-change limitation; do not restart against
a replacement. An inaccessible descendant makes dependent mechanisms partial,
not a root-validation error. Tree enumeration streams metadata without reading
file bodies or retaining a handle per entry. Release per-process native handles
after extracting evidence. Any backend size cap or result truncation is explicit
in the affected coverage, including the limit and omitted count when known.

## Platform implementations and crate reuse

Reuse Sniff's existing `sysinfo` dependency selectively for process identity. Its
[`Process::open_files()`](https://docs.rs/sysinfo/0.38.2/sysinfo/struct.Process.html#method.open_files)
supplies a count, not paths, so additional platform
discovery is required. Avoid full-system refreshes and CPU/memory/disk scans;
enrich only candidate processes and reuse backend-provided identity. On Linux,
report process IDs rather than adding each thread as another process.

- **Linux/WSL2:** evaluate `procfs` for process/descriptor enumeration. Version
  0.18.0 does not provide the needed inotify `fdinfo` parser; implement a focused
  parser if no suitable maintained crate exists. Collect tree device/inode
  identities once, then correlate watch registrations, rather than repeatedly
  walking the tree for each process. Normalize the kernel device encoding before
  comparison with filesystem metadata. Parse the documented decimal watch
  descriptor and hexadecimal inode/device/mask fields, accept unknown additional
  fields, and report malformed required fields without discarding valid sibling
  records. Preserve distinct registrations sharing an inotify descriptor.
  Do not consume another process's inotify event queue. Permission failures and
  observable descriptor replacement during inspection remain visible.
- **macOS:** evaluate `libproc` for vnode descriptor/cwd inspection and
  [`pids_by_type_and_path`](https://docs.rs/libproc/latest/libproc/processes/fn.pids_by_type_and_path.html),
  which returns candidate PIDs for a path or volume. A query for one path is not
  automatically a recursive tree query; verify the backend's scope. Report
  FSEvents discovery as unsupported. A PID-only path query is a candidate source, not a substitute for
  descriptor/cwd evidence; volume-wide matches are not tree matches. Do not rely
  on private kernel APIs to imply reliable coverage.
- **Windows:** evaluate `filelocksmith` for directory-tree handle/module owners.
  The [published 2.1.0 source](https://static.crates.io/crates/filelocksmith/filelocksmith-2.1.0.crate)
  returns only PID values and suppresses some discovery failures.
  It also terminates stalled native worker threads inside the caller's process.
  Do not use that implementation in-process. A PID-only adapter cannot recover
  discarded paths, evidence kinds, scan errors, or safe cancellation. Prefer a
  focused backend over existing Windows bindings; a revised upstream backend is
  acceptable only when it exposes evidence and satisfies the coverage, identity,
  and cancellation contracts. Report cwd and watcher subscription inspection as
  unsupported when unavailable; loaded modules are a distinct mechanism, not
  substitutes for open handles. State architecture/protected-process inspection
  limitations, and handle undersized native buffers without silently dropping
  entries.
  Its matching does not establish that a handle prevents deletion. Optional
  elevated execution is a user choice, never a requirement silently imposed
  on ordinary callers.

**Reader's note:** Windows crate reuse remains an option, but the published
implementation cannot meet the library contract through a thin wrapper.
Suppressing errors would turn restricted scans into misleading empty results;
terminating a native thread can damage a long-running library caller. Backend
choice must resolve these costs before claiming Windows discovery support.

Crate selection is an implementation decision constrained by these contracts,
not a requirement to add all three dependencies. Record the selected versions,
licenses, native build requirements, and runtime coverage in the implementation
log and dependency docs. `notify`/`inotify` create subscriptions; they do not
provide a general inventory of another process's subscriptions.

Gate backend modules, imports, and dependencies by target OS. The focused API
must be available with Sniff's default features, without enabling `remote` or
`network`. WSL2 compiles and runs the Linux backend; it is not native Windows
coverage.

Keep detector work request-scoped and reuse each observation for text and JSON.
Instrument process enumeration, descriptor inspection, tree identity reads,
and watch-registration reads using existing Sniff performance conventions.
Do not introduce a long-lived process cache or scan in default host inventory.

Use named counters at the Sniff-owned work boundaries in the Sniff package's
[`performance::counters`](../../lib/src/performance/counters.rs) module, which
records request costs. Count attempts, including failed native inspections,
once per unit of work. Disable timing/formatting overhead when collection is
off and propagate the collector into any worker. Counter assertions should
prove no per-process tree walk, repository inventory, or unrelated detection.
No performance spike or multi-host timing campaign is required by this spec.

## CLI contract

```sh
sniff filesystem query ./checkout
sniff filesystem query ./checkout --json
sniff filesystem query ./checkout --plain
```

Preserve the existing bare `sniff filesystem` report. The required path is the
query target regardless of repository root or global base-directory defaults;
a relative path is resolved against the invocation directory. Expose
`--target-only` and `--timeout <milliseconds>` for the library's scope and
deadline controls. No interactive prompt or process-stop operation.

Dispatch the focused query before ordinary detection-plan construction. Preserve
bare `filesystem --refresh-remotes` and `filesystem --latest-versions`; reject
those inventory/network flags when combined with `filesystem query` rather than
silently ignoring them. `--base` does not rebase this query or its file-reference
resolution. Add subcommand help and existing dynamic path/reference completions.
Use native positional arguments so ordinary non-UTF-8 paths remain accepted.
`--timeout` rejects zero, negative, malformed, or overflowing values with clap's
usage-error exit code (2). `--json` selects JSON even when `--plain` is also set.

Human output shows the target, processes with PID and available identity, matched
paths and evidence, then discovery limitations. Render through
`biscuit-terminal` `TerminalRenderable` components (`Prose`, lists, or `Table`)
and follow existing plain/color/width behavior. Literal paths and process names
must not be interpreted as markup.

Coverage and limitations describe the host observation and belong on stdout
with the report, including in plain mode. CLI advice, tracing, progress, and
rendered performance diagnostics belong on stderr. Escape literal content for
Prose and neutralize control characters in human display, including terminal
escapes and embedded newlines; preserve the original native value in JSON.
If human output is shortened, say how many records were omitted and keep the
library/JSON observation intact.

Example (illustrative evidence, not a claim that every Node process is visible):

```text
Filesystem usage: /work/checkout

PID 8124  node
  Working directory: /work/checkout
  Open handle: /work/checkout/.gitnexus/index.db

Discovery is partial.
  FSEvents subscriptions cannot be enumerated on macOS.
  No matches does not establish that the directory is unused.
```

`--json` emits one JSON object containing the serialized library report, without
CLI-side filtering or inference. With `--perf`, attach the existing structured
`performance` field using the Sniff CLI package's
[`print_json_value`](../../cli/src/output/mod.rs), which combines data and
performance into one JSON document. Any rendered performance summary stays on
stderr. Neither mode may trigger a second query to obtain another projection.

**Reader's note:** keeping structured `--perf` data in the JSON object preserves
Sniff's existing machine-output contract. Routing every kind of performance
output exclusively to stderr would silently change that contract.

Map the library's overall outcome to exit codes:

| Result | Exit code | Output |
| --- | --- | --- |
| `usable` report, including partial or empty | 0 | Full report on stdout |
| `unavailable` or wholly `unsupported` report | 1 | Full report, including coverage failures, on stdout |
| Typed target/options query error with no report | 1 | Human diagnostic on stderr; JSON mode emits `{"error": {"kind": "…", "message": "…"}}` on stdout |
| Invalid CLI arguments | 2 | Existing clap diagnostic on stderr; no query or report |

The runtime JSON error kind comes from the typed library error, or the typed
biscuit-file resolution error when failure precedes the query. Do not manufacture
an empty report for an error. Serialize before writing stdout so serialization
failure cannot leave half a JSON document; handle output-write failures through
the CLI's error path. Match presence does not itself change the exit code;
discovery is not a removal policy. In particular, this command does not use the
file-list commands' special failure behavior for empty results.

**Reader's note:** the JSON runtime-error object is an intentional addition for
this command. Scripts must check the exit code and the `error` field before
reading report fields. Argument parsing retains the established clap behavior;
the new error shape does not alter other Sniff commands.

## Acceptance and verification

1. Text and JSON are projections of one captured library report. Test their
   equivalence with a retained/injected observation rather than assuming two
   separate live queries will see identical state. Focused querying performs
   no repository inventory, network work, or unrelated system refresh.
2. A controlled child holding an open file/cwd is discovered on each supported
   OS where the mechanism permits it. Report limitations explicitly otherwise.
3. Linux tests correlate live inotify registrations to the correct device/inode,
   reject similarly prefixed sibling paths and out-of-scope ancestor watches,
   and preserve multiple watched paths/registrations and hard-link aliases.
4. Parser/backend tests cover permission denial, process exit and PID reuse,
   malformed native observations, shared-budget exhaustion before/during each
   phase, and missing identity. Enrichment after PID reuse cannot mislabel prior
   evidence; unavailable start times cannot authorize cross-observation merging.
5. A live macOS recursive FSEvents watcher is allowed to be absent from matches;
   the report must say that mechanism is unsupported. Never make a flaky test
   require it to be invisible through all other evidence.
6. Tree scope, root aliases, descendant links/reparse points, Unicode and native
   non-UTF-8 paths, hard links, root replacement, and platform path spellings
   preserve the matching and lossless-serialization contracts. Include Windows
   case-sensitive directories and UTF-16 values that are not valid Unicode.
7. Partial, empty, skipped, unsupported, and total-failure CLI outputs retain
   their coverage and exit semantics. `--json --perf` remains one valid JSON
   object with structured performance; runtime errors have the stated JSON
   shape. Human output does not overstate deletion contention, interpret literal
   markup, or execute control sequences from a path/process name.
8. Hidden/ignored files remain in scope. Genuine caller/ancestor usage is kept,
   detector-owned temporary handles are excluded precisely, and thread entries
   are not counted as separate Linux processes. Known required mechanisms are
   tested with controlled children; ordinary support cannot be satisfied merely
   by labeling every backend unsupported.

Use the repository Test Toolkit, nextest, and area `just test`/`just lint` recipes.
Parser, adapter, JSON/plain CLI, and controlled native-child tests are ordinary
first-tier tests, gated by OS where needed. An OS-specific handle/watch test does
not require a higher test tier just because it uses native APIs. Use the Sniff
CLI's existing isolated process fixture and runtime-resolved test binaries for
portability when test archives run on another host. Add terminal-tier tests only
for behavior requiring a real terminal. Child processes use readiness handshakes
and guaranteed cleanup on success, assertion failure, and timeout, with no sleeps
or focus changes.
Exercise required native mechanisms on macOS, Linux, Windows, and WSL2; record
permission-restricted coverage honestly without accepting a missing expected
controlled-child match as a passing test of a supported mechanism. Deterministic
adapter tests inject denial, malformed data, and stalled work rather than
depending on administrator/root permissions or timing races. Resolve the
deadline question before choosing its native-call acceptance checks.
Update library/CLI READMEs, topic docs, CLI docs, dependency docs, and Sniff/OS
skills alongside implementation. Planned docs are the current record until
implementation lands.

Before implementation, align the planned topic/CLI pages with the finalized
coverage statuses, outcome/exit rules, serialization, and deadline decision.
Leave planned markers until the behavior is built. Record implementation
departures in the implementation log, keep current docs accurate, and leave this
spec as the decision snapshot. The author closes the review lifecycle; an agent
does not move the feature into `_completed`.

## Open Questions

### Does two seconds limit scheduled work or guarantee the function has returned?

**Requires author decision before finalization.** The draft asks for a strict
return deadline, but canonicalization and metadata on a stalled filesystem,
native process enumeration, and Windows object-name queries can block without
a safe cancellation API. Checking the clock between calls cannot interrupt a
call already running. A timed-out thread is still doing work; killing that
thread inside a library caller is unsafe. The Windows crate's per-handle stall
checks also do not supply a two-second budget for the whole query.

Choose one of these contracts and update the API, CLI help, and planned docs:

1. **Shared scan budget with explicit native-call limits — recommended.** Stop
   scheduling inspections at two seconds, retain partial evidence, and report
   elapsed time and budget exhaustion. Document that a call already running
   can overrun the budget. Known hang-prone backend operations must be isolated
   safely or omitted with an explicit capability limitation; they cannot use
   abandoned or forcibly terminated threads.
   - **Pros:** keeps the focused synchronous library ergonomic, avoids an
     unprovable portable guarantee, and preserves useful evidence with honest
     coverage. Ordinary queries need no helper installation.
   - **Cons:** callers cannot treat `--timeout` as an absolute return deadline.
     Supporting risky native mechanisms may still require a helper; its
     packaging and cleanup must be specified before enabling that mechanism.
     `wt` must accept this latency contract explicitly before adoption.
2. **Isolate the whole scan in a supervised helper process.** Stream captured
   evidence back to the parent and stop the helper when the shared budget
   expires. Package the helper for library callers as well as the Sniff CLI;
   locating it cannot require invoking the installed `sniff` CLI or assuming it
   is on PATH.
   - **Pros:** prevents stalled native queries from trapping the caller's Rust
     threads and supports stronger latency isolation with partial results.
   - **Cons:** adds a shipped executable, protocol, process-start cost, and
     library deployment obligations. Uninterruptible OS I/O can still prevent
     immediate helper cleanup, so even this design must state the limits of
     its return and cleanup guarantees rather than promise an absolute bound.
3. **Offer a strict deadline only for an explicitly limited backend and target
   set.** Reject targets or omit mechanisms whose calls cannot satisfy it;
   publish precisely which observations remain available.
   - **Pros:** keeps a small implementation with an enforceable, narrow promise
     where the platform supplies suitable operations.
   - **Cons:** can remove the handle/watch evidence that motivated the feature,
     especially on Windows, and requires callers to cope with different
     capabilities for different targets. It must not masquerade as full
     cross-platform support.

Recommend the shared scan budget because this API provides advisory evidence,
not authority to delete, and its most useful portable contract is accurate
coverage without damaging or leaking work inside the caller. If `wt` requires
stronger latency isolation, choose the supervised helper and specify its
distribution and cleanup limits before implementation. No timing spike is
needed to choose the semantics: the question is whether native work can be
cancelled safely, not its average runtime.

## References

- [Linux fdinfo and inotify registration fields](https://man7.org/linux/man-pages/man5/proc_pid_fdinfo.5.html)
- [Apple FSEvents stream API](https://developer.apple.com/library/archive/documentation/Darwin/Conceptual/FSEvents_ProgGuide/UsingtheFSEventsFramework/UsingtheFSEventsFramework.html)
- [libproc path query](https://docs.rs/libproc/latest/libproc/processes/fn.pids_by_type_and_path.html)
- [filelocksmith implementation](https://github.com/velopack/filelocksmith-rs)
- [filelocksmith 2.1.0 published source](https://static.crates.io/crates/filelocksmith/filelocksmith-2.1.0.crate)
- [procfs process API](https://docs.rs/procfs/latest/procfs/process/struct.Process.html)
- [procfs 0.18.0 published source](https://static.crates.io/crates/procfs/procfs-0.18.0.crate)
