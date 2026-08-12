---
area: claudine
status: draft
created: 2026-08-01
packages:
    - darkmatter
    - sniff
    - claudine
    - claudine-cli
related:
    - ../../features/2026-07-13-file-resolution/spec.md
    - ../../features/2026-08-01-faster-compose/spec.md
review_iterations: 0
---

# Propagate request context through Claudine startup

## Outcome

Claudine captures repository, filesystem-resolution, HOME, and environment
context once for an invocation and propagates that context through wrapper,
composition, system-prompt, sequence, and harness preparation.

Repository topology is detected at most once for each distinct repository an
invocation actually enters. Downstream consumers derive document-relative
views from the captured snapshot without repeating repository walks or reading
ambient process state.

The change preserves all intended user-visible behavior aside from two
deliberate departures: it removes the incidental Claudine composition of a
provider-owned memory file that does not opt into Claudine's harness, and it
re-anchors a sequence step's explicit task stack to the document that
authored the task:

- system-prompt discovery remains anchored to the launch CWD;
- document references remain anchored to the document that authored them;
- a task's `side_effect` action and its `setup`/`teardown` stacks now use the
  task's own origin document (`task.origin_dir`/`task.origin_path`) as their
  mutation root, `base_dir`, `source_path`, and `repo_root` instead of the
  document a step composes and runs, so `set_frontmatter` and other
  file-touching effects target files next to the task's origin document —
  including when that document lives in a different repository from the
  step's prompt;
- a source in another repository receives that repository's context;
- proxy, retry, and resume continue to reread mutable documents at their
  existing lifecycle boundaries;
- direct wrappers, composition, and sequence retain their current provider,
  workspace, prompt-delivery, and diagnostic semantics; and
- provider memory files still enable wrapper harness behavior when their
  frontmatter requests it; and
- ordinary provider memory files without harness properties remain
  provider-owned inputs and are not composed by Claudine solely to decide
  harness eligibility.

The performance contract is structural rather than tied to one machine's wall
clock: repeated consumers must reuse captured evidence. Wall-clock benchmarks
remain supporting evidence, not flaky test gates.

## Relationship to unified file-reference resolution

The Unified File-Reference Resolution feature (implemented through nine
recorded review iterations; its spec directory has not yet moved to
`_completed/`) established
`FileResolutionContext` as the explicit, request-scoped authority for CWD,
HOME, environment, repository root, package area, and configured roots. Its
acceptance criterion 12 requires Claudine and Darkmatter document-backed
resolution to perform no late ambient discovery.

That type and the Darkmatter `ComposeOptions::with_file_resolution_context`
entry point exist. Most document composition paths use them. This fix closes
the remaining propagation and ownership gaps:

- Claudine captures equivalent repository evidence several times while
  preparing the first composition document;
- sequence task preparation and failure reporting re-enter the context-free
  resolution entry point, repeating a full launch capture per task;
- system-prompt composition supplies a source path and runtime expression
  context but not a `FileResolutionContext` — even though explicit
  `--append`/`--replace` resolution already builds one and then discards it;
- wrapper harness eligibility fully materializes a provider memory file before
  deciding whether harness behavior is enabled; and
- performance reporting leaves material CWD-sensitive work unattributed.

This specification does not redefine file-reference syntax or precedence. The
existing file-resolution contract remains authoritative.

## Motivating investigation

An investigation into 48 slow `claudine-cli` tests and three tests that timed
out found that inherited CWD and HOME context could dominate otherwise small
test cases. Isolating those inputs materially improved the tests without
changing their CLI paths or original assertions.

Three initial observations motivated the deeper investigation:

1. repository-root execution appeared to add roughly 2.3--4.8 seconds;
2. root system-prompt discovery appeared to add roughly 1.4 seconds on one
   measured path; and
3. inherited CWD/HOME context activated unrelated repository and prompt work
   that became severe under concurrent nextest load.

The first label was too broad. The 2.3--4.8 second delta measured **ambient
repository-root startup context**, not repository topology discovery alone.
That context can activate all of the following:

- Git and workspace topology detection;
- package and package-area membership lookup;
- root `system-prompt.md` composition;
- repository `.claudine/non-interactive.md` composition;
- provider memory-file discovery and harness preparation;
- user Claudine/provider configuration under HOME; and
- source and link resolution rooted in the surrounding monorepo.

The corrected attribution is part of this fix. Performance reporting and
future regression tests must not describe the whole ambient-context delta as a
single repository scan.

## Observed evidence

The investigation used fake provider executables, an isolated HOME, disabled
rendezvous reporting, and identical commands from the rusty-biscuit root and a
temporary non-repository directory. Values below are single-host diagnostic
measurements, not portable acceptance thresholds.

| Path | Observed time | Relevant attribution |
|---|---:|---|
| Prompted direct wrapper from an isolated directory | about 88 ms total | Baseline with no repository prompt or provider memory file |
| Same wrapper from the repository root with an explicitly empty prompt | about 1.0 s total | about 184 ms environment setup plus about 760 ms later unattributed CWD-sensitive work |
| Direct wrapper at the root with automatic system prompt | about 948 ms environment setup | system-prompt processing dominated |
| Direct wrapper at the root with system prompt disabled | about 10 ms environment setup | confirms that locating a candidate filename was not the dominant prompt cost |
| Root composition preparation | about 815 ms | overlapping repository/context discovery dominated several prep buckets |
| Root composition system-prompt substage | about 1.5 s on a cold path | Darkmatter composition without a propagated file-resolution snapshot |

A standalone structure probe on this monorepo appeared to cost roughly
170--220 ms in the measured paths. Repeating it explains much of composition
preparation, but not the whole ambient startup delta.

## Terminology

### Ambient process context

Values read directly from mutable process or host state, including:

- `std::env::current_dir()`;
- `dirs::home_dir()` or equivalent host HOME discovery;
- `std::env::vars()` or individual environment reads;
- Git-root discovery from an unpropagated path; and
- repository/package topology discovery initiated by a downstream consumer.

Ambient reads are allowed at the invocation capture boundary. They are not an
acceptable substitute for propagated context later in the same request.

### Launch snapshot

The immutable context captured from the directory and environment in which
Claudine was invoked. It includes the launch CWD, HOME and environment
snapshots, Git/repository evidence when requested, package membership, and the
derived launch-facing contexts used by wrappers.

The launch snapshot remains fixed even after Claudine changes its process CWD
or creates a shadow HOME.

### Source context

A document-relative `FileResolutionContext` derived for one resolved source.
It uses that document as its authoring base while retaining the invocation's
captured HOME/environment data. If the document belongs to the launch
repository, derivation reuses the launch topology. If it belongs to another
repository, the invocation captures one topology snapshot for that distinct
repository and reuses it thereafter.

### Repository topology probe

Work that identifies workspace shape, packages, package areas, and nested
workspace membership. A cheap containment check against an already known root
is not a new topology probe. A call that walks workspace markers or invokes
`detect_repo_structure` is.

### Discovery and preparation

Discovery selects a candidate path, such as locating `system-prompt.md` or a
provider memory file. Preparation parses or composes the selected content.
Performance reporting must distinguish them.

## Current behavior and underlying causes

### Direct wrapper startup is already mostly consolidated

`detect_wrap_startup` uses one Sniff detection plan to produce environment,
launch, and workspace contexts. Promptless interactive wrappers launched at a
repository root can omit repository topology entirely.

One residual wrinkle: deciding whether the promptless-at-repo-root shortcut
applies performs its own `GitRepo::discover` before the detection plan runs,
so even this path pays two Git-root discoveries. The invocation owner should
absorb that probe into the shared capture.

This is the architectural pattern the other paths should follow. The fix must
not regress the direct wrapper into multiple independent scans.

### Composition prepares the same context repeatedly

The common first-document composition path currently performs overlapping
work:

1. `capture_file_resolution_context` discovers the launch Git root and
   repository structure so it can resolve the top-level reference.
2. `derive_request_context_for_source` discovers Git and repository structure
   again after the source resolves.
3. `prepare_and_run_active_document` calls that derivation again for the same
   initial source.
4. `CompositionPrepContext::new` performs a shared Git summary scan and a
   separate repository-structure scan for launch/workspace contexts.

The source re-anchoring step is semantically necessary when a selected source
lives in another repository. Repeating discovery when the source remains in
the launch repository is not.

Verified accounting for a single live `claudine compose <file>` run whose
source sits in the launch repository: the four sites above produce **four
unconditional repository-structure enumerations**, each with its own Git-root
discovery and no shared manifest parsing between them. When the document
references `ctx.repo` or `ctx.area`, Darkmatter's demand-driven runtime
capture adds up to **three more** — the preparation path captures a runtime
context for the same document twice with identical arguments (once for
shell-preflight compose options, once for the prepared context), and the
lifecycle path performs a third near-identical capture over the effective
frontmatter and prompt. Each of those captures also re-snapshots the full
process environment.

Beyond the numbered sites, three smaller paths repeat discovery:

- sequence task runs, JIT step reloads, and sequence failure reporting call
  the context-free resolution entry point, which performs a fresh CWD read,
  Git discovery, repository-structure scan, and environment snapshot per
  call;
- the file-reference error-enrichment path rebuilds prompt magic roots with
  its own CWD/Git/structure triple; and
- harness shell-option preparation runs a hand-rolled `.git` ancestor walk —
  twice per run on the composition path — even though the preparation context
  already holds the resolved source repository root.

The doc comment on `derive_request_context_for_source` claiming that
re-anchoring performs no later ambient reads is drifted: it holds for
HOME/environment but not for the Git and repository-structure discovery the
function performs. Per repository policy the code is authoritative; this fix
corrects the comment alongside the behavior change.

The underlying ownership problem is that each layer receives enough path data
to rediscover context, but no one layer owns and propagates the complete
evidence already collected for the invocation.

### Structure-only detection still has real cost

Sniff's structure request correctly avoids full file inventory and enrichment,
but workspace membership still requires detector orchestration and a bounded
walk for nested workspace markers. That is legitimate work. The primary defect
is invoking it repeatedly for the same canonical repository root.

This fix does not assume that structure detection can become free. It prevents
Claudine from asking the same question repeatedly.

### System-prompt file lookup is cheap; composition is not

System-prompt resolution checks a fixed ordered candidate list and reads the
first selected file. The expensive stage begins after selection.

`compose_prompt_markdown` builds Darkmatter `ComposeOptions` with a source file,
shell CWD, and a demand-driven/shared runtime `ComposeContext`. It does not
attach the already known `FileResolutionContext`.

When no snapshot is supplied, Darkmatter's compatibility paths rediscover Git,
package area, and HOME for expression, schema, transclusion, normalization, and
local-link phases. Package-area fallback invokes repository-structure
detection. Local-link resolution can repeat that fallback for each link.

The root system prompt is small and references only a small runtime context,
but it contains relative links. Its cost therefore reflects repeated context
reconstruction rather than document size.

Primary system prompts and non-interactive appendices already share one
demand-driven runtime expression context. The missing piece is the filesystem
resolution snapshot, not another runtime-context cache.

### Harness eligibility performs full work before the cheap decision

For a direct wrapper with a prompt, Claudine searches the provider's
repository-relative memory-file candidates. If one exists, it materializes a
complete passthrough harness seed and only then calls
`has_harness_properties`.

Most provider memory files are ordinary instructions without harness
frontmatter. Full body composition is unnecessary in that case. At the
rusty-biscuit root, this path accounted for roughly 760 ms that was not covered
by the environment-setup timer.

The materialization is also more expensive than it needs to be even when a
harness is enabled: it builds its compose options with the default
`ComposeOptions::new()`, which triggers Darkmatter's full ten-group runtime
capture — Git, repository structure, docs scan, OS, hardware, and GPU probes —
instead of the demand-driven capture the system-prompt path already uses, and
it attaches no `FileResolutionContext`.

The defect is not memory-file discovery itself. It is using full
materialization as an eligibility probe, and using the full ambient runtime
capture inside that materialization.

### CWD and HOME are legitimate inputs but accidental test dependencies

The production CLI intentionally uses CWD and HOME. Tests that inherit the
repository root or host HOME therefore opt into behavior unrelated to their
assertions:

- automatic root and user system prompts;
- non-interactive appendices;
- package/workspace classification;
- provider memory files and harness checks;
- user configuration, caches, and state; and
- shadow-HOME preparation.

Concurrent nextest processes multiply filesystem walks, manifest parsing,
Markdown composition, and access to user/provider state. This turns a modest
single-process tax into timeouts and high variance.

Test isolation is necessary, but it is not a substitute for fixing production
context propagation. Real users launch Claudine from repositories and should
not pay repeated discovery costs either.

### Performance attribution hides actionable work

The wrapper performance collector starts before environment setup and stops
that bucket before prompt delivery, harness detection, and execution. Material
harness eligibility/materialization work therefore lands in the synthetic
`unattributed` bucket.

The direct wrapper path currently records **no substages at all**: its
`prep phase` is hardcoded to zero, no substage marks exist between collector
start and environment-setup completion, and the environment-plan substages
that do exist are computed only behind a flag the wrapper never passes — so
they are never read back into the collector. Startup detection, the entire
system-prompt resolve-and-compose stage, child-environment construction, and
MCP composition all merge into one opaque environment-setup total, and that
total's entire duration reappears as its own `unattributed` child.

Without narrower measurements, a regression can be misdiagnosed as a Sniff
problem even when the dominant work is Darkmatter composition or harness
materialization.

## Required design

### D1 — One owner for invocation discovery evidence

Claudine must establish one request-scoped owner for launch discovery evidence.
The exact type name and owned/borrowed representation are implementation
decisions; introducing a new public abstraction is not required if existing
types can express the boundary clearly.

The owner must make the following available without later ambient reads:

- absolute launch CWD;
- captured HOME and environment values;
- launch Git root and Git summary when requested;
- optional launch `RepoInfo`/workspace topology;
- package and package-area membership for the launch CWD;
- launch `FileResolutionContext`;
- `LaunchContext` for system-prompt discovery;
- `LaunchWorkspaceContext` for child-CWD and repository behavior;
- `EnvironmentContext` for lifecycle/event data; and
- the typed launch-detection failure retained by strict consumers such as
  `--repo`.

These projections must be derived from shared evidence. Constructing several
context structs is acceptable; independently rediscovering the same repository
for each one is not.

### D2 — Repository topology is memoized for one invocation

Repository topology reuse is scoped to the invocation and keyed by canonical
or otherwise identity-safe worktree root.

Required behavior:

- the launch repository is probed at most once when topology is needed;
- a source contained by the launch repository reuses that `RepoInfo`;
- the first source in another repository may cause one probe for that
  repository;
- later documents in the same repository reuse its captured topology;
- sources outside Git receive an explicit no-repository source context and do
  not trigger an unbounded walk of their parent tree; and
- a discovery failure is retained and projected according to existing strict
  versus best-effort behavior rather than retried independently by every
  consumer.

The cache must not be process-global. A global cache risks stale manifests,
cross-worktree contamination, and incorrect behavior after repository changes.
Per-invocation reuse is sufficient and matches Claudine's existing immutable
launch-input contract.

Linked worktrees and nested repositories retain their own identities. Path
comparison must use platform-aware `Path` operations and must not assume POSIX
separators or case behavior.

### D3 — Composition derives source context once per source identity

The top-level source still requires two conceptual stages:

1. resolve its authored CLI reference against the launch snapshot; then
2. derive the definitive source context from its resolved location.

Those stages must share repository evidence. For a source in the launch
repository, stage 2 performs no second topology probe. The resulting source
context is passed directly into first-document preparation; the active-document
path must not immediately derive it again.

Proxy, sequence, loop, retry, and resume behavior follows these rules:

- entering a different resolved source derives a context for that source;
- entering another source in an already observed repository reuses topology;
- references authored by the new source use that source's directory;
- retry and resume continue to reread the document content at their current
  canonical boundaries;
- immutable launch CWD, HOME, environment, and system-prompt content are not
  recaptured; and
- a lifecycle transition must not inherit the wrong source base merely to
  avoid a probe.

The source context remains present in prepared input layers so schema,
expression, transclusion, link, lifecycle, and completion paths use the same
resolution provenance.

### D4 — System-prompt composition uses launch discovery and source context

`resolve_and_prepare_for_session`, or an equivalent internal API, receives the
invocation context owner in addition to `LaunchContext`.

Every file-backed primary system prompt and non-interactive appendix passes an
appropriately source-derived `FileResolutionContext` into
`ComposeOptions::with_file_resolution_context`. Candidate selection remains a
launch-scope operation, but references authored inside the selected prompt
remain source-aware:

- a prompt inside the launch repository reuses launch topology;
- a prompt inside another repository uses one request-local observation of
  that repository;
- a trusted HOME/magic prompt outside Git receives an explicit
  trusted-external source view with no invented repository; and
- HOME/environment values still come from the immutable launch capture.

This distinction preserves current source-relative composition behavior while
eliminating ambient rediscovery. It must not force the launch repository root
onto a prompt that belongs to a sibling repository or no repository.

The following semantics are required:

- standard prompt discovery remains package, package-area, repository, then
  user scope from the launch CWD;
- explicit prompt references retain the established `FileReference` grammar;
- a composition source in another repository does not move automatic
  system-prompt discovery away from the launch CWD;
- primary and appendix composition share runtime context and filesystem
  resolution evidence;
- an empty selected prompt still disables lower-priority prompt discovery;
- an empty appendix still falls through to the next appendix candidate;
- append/replace mode and every provider delivery strategy remain unchanged;
- `::shell` in system-prompt and appendix composition remains pinned to the
  launch repository root according to the existing contract; and
- built-in appendix text requires no fabricated file source.

Once the snapshot is attached, Darkmatter must not invoke its ambient Git,
package-area, or HOME compatibility fallbacks for any phase of that prompt,
including each local link.

### D5 — Harness eligibility is separate from materialization

Provider memory-file discovery continues to use the provider metadata registry
and the captured repository/CWD search root. Home-relative provider-native
memory files remain excluded from this repository harness search as they are
today.

For a discovered repository memory file:

1. read and parse only enough of the document to obtain authored frontmatter;
2. evaluate harness-property presence using the same canonical predicate;
3. return no harness without composing the body when no harness property is
   present; and
4. perform the existing canonical full materialization only when harness
   behavior is enabled.

The eligibility read must not introduce a second private frontmatter grammar.
It uses the same Markdown/frontmatter parser as full preparation. Malformed
frontmatter must retain its current typed failure rather than being silently
treated as “no harness.”

When harness behavior is enabled, the fully materialized prompt must be
identical to current behavior and must receive the correct propagated
file-resolution and shell contexts. The materialization must also stop using
the full ambient runtime capture: it composes with a demand-driven runtime
context anchored on the memory file, sharing the invocation's captured
repository evidence, matching the pattern the system-prompt path already
established. Lifecycle, approval, retry, resume, proxy, MCP-tag, and overlay
behavior must not be weakened.

### D6 — No late ambient context reads on propagated paths

After invocation capture, production preparation paths in scope must not use
ambient CWD, HOME, environment, Git, or package-area discovery when equivalent
context is available.

This applies to:

- top-level composition after its launch capture;
- active-document preparation;
- sequence step preparation, JIT step reloads, and sequence failure
  reporting;
- system-prompt and non-interactive appendix composition;
- Darkmatter expression, schema, transclusion, normalization, and link phases
  invoked by those paths;
- Darkmatter demand-driven runtime-context capture for the repository-backed
  `ctx.*` groups (`ctx.repo`, `ctx.area`, file changes), which must be able
  to consume the invocation's captured Git and topology evidence instead of
  probing — today only `ctx.area`/`ctx.os` can be pre-supplied through
  external state; and
- wrapper harness eligibility/materialization, including its hand-rolled
  `.git` ancestor walk, which must reuse the captured source repository root.

Compatibility APIs may retain ambient fallback behavior for callers that have
no request snapshot. Claudine's canonical CLI paths must not rely on those
fallbacks.

This fix may add narrow, additive evidence-aware capture seams to Darkmatter
and Sniff so Claudine can supply an already captured environment, Git handle,
or repository observation. It does not own Darkmatter compose-session caching,
collapsing repeated in-memory compose/capture passes, or reducing the cost of a
single Sniff detector. Those pipeline-internal optimizations remain with the
related faster-compose feature. Repeated compatibility calls may remain during
this fix, but canonical Claudine paths must make them projections of the same
request-scoped evidence rather than new ambient probes.

Changing process CWD for child execution or changing child HOME for a provider
overlay must never alter the already captured context.

### D7 — Preserve failure and fallback semantics

Consolidating discovery must not flatten distinct consumer policies:

- explicit `--repo` requirements continue to fail with the captured typed
  launch-detection error;
- best-effort display/event consumers may continue with absent repository
  fields when discovery fails;
- file-reference failures retain their typed candidate/provenance data;
- source paths outside a repository remain supported;
- missing system-prompt candidates remain a normal absence;
- malformed selected prompts and provider memory files remain errors; and
- provider installation, unsupported prompt delivery, and shell-preflight
  diagnostics retain their current identity and ordering unless a separately
  documented validation can safely occur earlier.

One failed topology observation must be shared as evidence, not silently
converted into several inconsistent retries and defaults.

### D8 — Performance reporting names the work performed

`--perf` must expose enough structure to distinguish at least:

- launch repository/Git discovery;
- source-context derivation, including whether topology was reused or probed;
- system-prompt candidate lookup;
- system-prompt runtime-context capture;
- primary prompt and appendix Darkmatter composition;
- system-prompt delivery preparation;
- provider memory-file eligibility; and
- enabled-harness materialization.

Structural children must reconcile with their existing parent timers according
to the current performance-tree contract. Work must not be double-counted to
make the report appear complete.

The large wrapper harness cost observed during this investigation must no
longer land entirely in top-level `unattributed`. Small residual unattributed
time remains acceptable.

### D9 — Performance regressions are tested by work, benchmarked by time

Deterministic tests must count or otherwise observe expensive operations
through narrow seams. They must not depend on elapsed-time assertions.

Required work invariants:

| Scenario | Maximum repository topology work |
|---|---|
| Direct wrapper in one repository | one launch topology probe when requested |
| Composition whose source is in the launch repository | one topology probe total |
| Sequence whose documents remain in one repository | one topology probe total |
| Source/proxy entering a second repository | one additional probe for that distinct repository |
| System prompt plus non-interactive appendix in the launch repository | zero additional topology probes after launch capture |
| Explicit system prompt in a second repository | one additional probe for that distinct repository |
| Prompt with any number of local links | zero per-link topology probes after context propagation |
| Valid provider memory file without harness properties | no full body composition/materialization |

Instrumentation used by tests must not introduce a mutable process-global
counter that makes parallel tests serial or flaky. Dependency injection,
request-local accounting, tracing capture, or another scoped seam is
acceptable.

Counting seams must observe every probe, not only the first: Darkmatter's
existing "one trusted discovery" capture instrumentation counts the primary
Git discovery but misses the second, uninstrumented discovery performed on
the file-changes capture thread, so its guard passes while two discoveries
occur. This fix either eliminates that second discovery or brings it under
the same accounting.

Ignored diagnostic benchmarks may record cold and warm wall-clock results.
Before/after evidence must use the same binary, provider stub, CWD, HOME,
prompt inputs, and host. A material reduction is expected, but absolute
millisecond thresholds are not CI gates.

### D10 — Test environments declare ambient context intentionally

The `claudine-cli` real-process test helpers must make CWD, HOME, and
rendezvous behavior explicit.

The default for a test that does not exercise discovery is:

- an isolated temporary working directory;
- an isolated temporary HOME;
- rendezvous reporting disabled;
- fake provider executables supplied through a platform-correct PATH; and
- no repository/system-prompt/provider-memory files except those authored by
  the fixture.

Tests whose purpose is repository, user, prompt, package-area, shadow-HOME, or
harness discovery opt into the corresponding fixture explicitly.

Isolation must not replace the behavior under test. The shell-preflight and
rich-error tests that motivated this work must continue to:

- invoke the real Claudine CLI path;
- exercise the same `::shell` discovery and preflight branches;
- preserve every original assertion about error identity, provenance, and
  rendered content; and
- add assertions only where needed to prove the absence of incidental ambient
  inputs.

PATH construction uses `std::env::join_paths` or an equivalent platform-aware
API. Tests must not assume `:` separators, `/tmp`, POSIX executable naming, or
symlink support.

## Scope

### In scope

- Invocation-scoped launch discovery ownership and propagation
- Per-invocation repository-topology reuse
- Composition and sequence source-context derivation
- System-prompt and non-interactive appendix file-resolution propagation
- Direct-wrapper provider-memory harness eligibility
- Performance attribution for the affected startup stages
- Hermetic defaults for unrelated real-CLI tests
- Unit, integration, concurrency, and diagnostic benchmark coverage
- Updates to affected code comments and Claudine architecture/skill documents

### Out of scope

- Changing file-reference syntax, candidate precedence, or diagnostics
- Changing system-prompt discovery order, delivery modes, or provider support
- Removing repository-aware or HOME-aware behavior from production
- Disabling provider memory-file harness support
- Changing lifecycle retry/resume document freshness
- Optimizing Sniff's individual workspace detectors or nested-marker walk
  (deferred to the faster-compose feature)
- Reducing work inside Darkmatter's compose pipeline itself — parse sharing,
  allocation reduction, schema/trigger caching (deferred to the
  faster-compose feature)
- Adding a process-global repository cache or persistent discovery database
- Caching fully composed prompts across invocations
- Weakening or replacing real CLI tests with mocked unit tests
- Treating wall-clock duration as a deterministic correctness assertion
- Broad refactoring of unrelated wrapper or composition code

## Implementation constraints

- Reuse `FileResolutionContext`, `RepoInfo`, Sniff detection plans, and existing
  launch/environment projection types rather than creating parallel models.
- Prefer one composed request owner over a collection of unrelated optional
  parameters, but do not make a new public abstraction solely for naming.
- Keep repository work bounded to a discovered Git/worktree root. Never run an
  unbounded structure walk from HOME or another arbitrary non-repository CWD.
- Preserve the distinction between launch workspace and source repository. The
  provider child CWD follows the existing launch-workspace contract even when
  the source document comes from another repository.
- Preserve authored path identity and current symlink/worktree semantics; this
  fix is not a canonicalization or sandbox redesign.
- Keep context immutable after capture. Derivation creates a new source view or
  reuses request-local evidence; it does not mutate ambient process state.
- Do not add crate cycles. In particular, `biscuit-file` must not depend on
  Sniff.
- Any behavior-changing edit must review and update nearby `///`, `//!`, and
  inline comments. Existing “discovery occurs exactly once” claims must describe
  the whole invocation accurately after this fix, not only one helper call.
- Preserve macOS, Linux, and Windows behavior even when verification can run
  only on Linux.

## Test requirements

### Context ownership and topology reuse — L1

- A launch snapshot projects file-resolution, launch, workspace, and
  environment contexts from one repository observation.
- A source inside the launch repository reuses the same topology evidence.
- The first source in a sibling repository receives that repository's package
  and package-area context and records exactly one additional probe.
- Two sources in the sibling repository reuse the sibling observation.
- A source outside Git receives no repository/package context and does not
  initiate an unbounded structure walk.
- A topology error retains the existing strict `--repo` versus best-effort
  behavior across every projection.
- A process CWD or HOME mutation after capture does not change any derived
  context.

### Composition and lifecycle propagation — L1

- Top-level compose, inline-compose, and sequence sources use one definitive
  source context without immediate re-derivation.
- Schema, expression, transclusion, and local-link resolution receive the same
  source context.
- Proxying to another file in the same repository performs no new topology
  probe and uses the proxied document as its authoring base.
- Proxying to another repository performs one new probe and uses that
  repository thereafter.
- Retry and resume reread mutable document content while retaining immutable
  launch context and request-local topology evidence.
- Sequence serial and parallel paths obey the same topology bounds without
  sharing mutable global test state.

### System prompt propagation — L1

- Automatic and explicit system-prompt files receive a source-derived
  `FileResolutionContext` from the invocation owner, without ambient reads.
- Repository and user non-interactive appendices reuse the same captured
  launch context.
- Frontmatter expressions, body expressions, schema references,
  transclusions, and multiple local links add no repository probes.
- A composition source from a sibling repository does not change automatic
  system-prompt discovery or the context derived for the selected prompt file.
- Empty-disable, empty-appendix fallback, append/replace mode, and composed
  output remain byte-for-byte or semantically identical as appropriate to
  existing tests.
- Every provider receives the same effective prompt and delivery plan it
  received before the optimization.

### Harness eligibility and materialization — L1

- No prompt means no provider memory-file harness lookup, matching current
  behavior.
- No memory file means no eligibility parse or materialization.
- A valid memory file without harness properties is parsed for frontmatter but
  its body is not composed.
- A valid enabled harness uses the full existing materialization path and
  preserves its prompt, frontmatter, lifecycle, approvals, overlays, MCP tags,
  and file-resolution context.
- An enabled harness materialization composes with a demand-driven runtime
  context and adds no ambient host probes (OS, hardware, GPU) or repository
  topology probes beyond the invocation's shared evidence.
- Malformed memory-file frontmatter retains its typed error.
- Repository-root and non-repository launch roots preserve their current
  candidate behavior.

### Real CLI regressions — L1 process integration

- The shell-failure rich block, false-block shell discovery, and source
  provenance tests run through the real `claudine` binary from isolated
  fixtures and preserve all original assertions.
- Dedicated root-context fixtures prove automatic repository prompt,
  non-interactive appendix, and provider memory-file discovery still work.
- A fake provider captures argv, environment, CWD, and delivered prompt to
  prove direct wrapper and composition behavior remains unchanged.
- A concurrent nextest run of the previously timing-out tests completes
  without timeouts or reliance on execution ordering.

Terminal or browser windows must not gain focus. Use L2 only for behavior that
requires a real terminal; repository/context propagation itself belongs in L1.

### Performance diagnostics

- The existing ignored system-prompt benchmark accepts an explicit propagated
  context and reports lookup, runtime-context, primary composition, appendix
  composition, and total cold/warm timings.
- A diagnostic composition benchmark reports topology-probe count and reuse for
  launch-equals-source and cross-repository cases.
- A wrapper benchmark separates provider memory-file eligibility from enabled
  harness materialization.
- `--perf` reconciliation tests cover the new substages and prove that the same
  duration is not counted structurally twice.

### Cross-platform coverage

- Linux/macOS repository paths and Windows drive/UNC paths can key
  request-scoped observations without string-prefix assumptions.
- HOME capture uses the existing cross-platform provider and is retained after
  child shadow-HOME mutation.
- Test executable discovery and PATH modification are platform-aware.
- Tests do not require symlinks where Windows permissions or filesystem support
  may differ; copy-based fixtures are acceptable when link identity is not the
  subject.

## Verification scope

Before implementation:

1. use the current GitNexus index to run impact analysis on the selected
   request-context owner, `derive_request_context_for_source`,
   `CompositionPrepContext::new`, `resolve_and_prepare_for_session`,
   `compose_prompt_markdown`, and `detect_wrapper_harness`;
2. inventory every production call to `detect_repo_structure`,
   `GitRepo::discover`/`detect_git`, and any hand-rolled `.git` ancestor walk
   in the Claudine package area and classify each as invocation capture,
   distinct-source capture, compatibility fallback, or redundant;
3. inventory every Claudine-created Darkmatter `ComposeOptions` for a
   file-backed document and verify that a request-scoped
   `FileResolutionContext` is supplied; and
4. capture repeatable before measurements for the root and isolated scenarios
   above.

After implementation:

1. run the relevant focused L1 tests while iterating;
2. run `just test` and `just lint` from the `claudine` package area;
3. run affected Darkmatter and biscuit-file tests if their code or contracts
   change;
4. run `just _test claudine-cli --no-fail-fast` from the repository root and
   confirm that the formerly timing-out tests complete;
5. run relevant L2 tests only where terminal rendering is part of the asserted
   contract; and
6. record before/after cold, warm, and concurrent diagnostic results without
   converting those wall-clock numbers into CI thresholds.

Workspace-wide testing is required only if GitNexus impact analysis or actual
changes identify consumers outside these package areas.

## Documentation maintenance

Update documentation alongside the implementation where behavior descriptions
or ownership boundaries change:

- Claudine architecture documentation must identify the invocation capture
  owner and distinguish launch from source context.
- System-prompt documentation must state that both runtime and file-resolution
  contexts are captured once and propagated through primary and appendix
  composition.
- Composition/file-resolution documentation must describe per-repository
  request-local reuse and cross-repository derivation.
- Performance documentation must distinguish discovery, preparation, and
  delivery and name the new `--perf` substages.
- Claudine skill material must be updated if its architecture or diagnostic
  workflow changes.
- Comments that claim one-time discovery must be audited against the complete
  invocation call path. Drifted comments are corrected or removed in the same
  behavior-changing change.

## Acceptance criteria

- [ ] One invocation-scoped owner supplies launch filesystem, repository,
      HOME/environment, workspace, and event context projections.
- [ ] A direct wrapper performs no more than one launch repository-topology
      probe when topology is required.
- [ ] Compose, inline-compose, and sequence perform no more than one topology
      probe when launch and all sources remain in one repository.
- [ ] Cross-repository sources add no more than one probe per distinct
      repository and receive the correct source-relative context.
- [ ] The first active composition document does not immediately repeat
      `derive_request_context_for_source` for the context already derived for
      it.
- [ ] System-prompt and non-interactive appendix composition receive
      source-derived `FileResolutionContext` values from the invocation owner;
      files in an already observed repository perform no additional
      repository/package-area discovery.
- [ ] Local links and other Darkmatter phases do not add per-reference
      topology probes when Claudine supplied a context.
- [ ] Automatic system-prompt discovery remains based on launch CWD even when
      the composition source belongs to another repository.
- [ ] Provider memory files without harness properties avoid full body
      materialization, while enabled and malformed harness files retain their
      complete existing user-visible behavior; enabled materialization uses a
      demand-driven runtime context with propagated file-resolution and
      repository evidence.
- [ ] Retry and resume preserve document freshness without recapturing
      immutable launch inputs or repeating topology work for an already seen
      repository.
- [ ] `--repo`, best-effort discovery, typed file-reference errors, prompt
      modes, provider delivery, shell CWD, lifecycle, and child workspace
      semantics do not regress.
- [ ] `--perf` separately attributes repository discovery, system-prompt
      lookup/composition/delivery, and harness eligibility/materialization with
      a reconciling performance tree.
- [ ] Performance regression tests assert bounded expensive-work counts rather
      than elapsed-time thresholds.
- [ ] The previously timing-out shell/preflight tests retain their real CLI
      paths and all original utility assertions while using explicit isolated
      CWD/HOME fixtures where ambient discovery is not under test.
- [ ] Dedicated fixtures continue to cover repository, root/user prompt,
      non-interactive appendix, provider memory-file, and shadow-HOME discovery.
- [ ] macOS, Linux, and Windows path, HOME, PATH, and worktree behavior remain
      supported.
- [ ] Required documentation and behavior-adjacent comments are updated.
- [ ] Relevant `just test`, `just lint`, and no-fail-fast `claudine-cli` gates
      pass, and before/after diagnostic results demonstrate a material startup
      improvement.
