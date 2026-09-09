---
area: claudine
status: draft
created: 2026-09-07
reviewed: true
reviewed_by: opencode/zai-coding-plan/glm-5.3
reviewed_on: 2026-09-07
implemented: true
review_iterations: 2
depends-on: claudine/fixes/_completed/2026-08-01-cli-slow-tests/spec.md
coordinates_with:
    - claudine/fixes/_completed/2026-08-31-silent-success-and-startup-stall/spec.md
packages:
    - claudine
    - claudine-cli
    - claudine-contract
    - claudine-catalog-types
    - claudine-gen
    - rendezvous-core
    - rendezvous-daemon
    - rendezvous-client
---

# Faster Claudine tests through complete evaluation and explicit fixtures

## Outcome

Evaluate every Claudine test family, including tests that have never been slow,
and remove unnecessary setup, inherited state, repeated work, and waiting while
preserving the behavior each test proves. Complete the remaining L1 process
fixture migration. A fast test with weak assertions or no canonical execution
path is a finding too.

This is a draft specification, not an implementation or a completed audit.
Timing targets will be ratified after baseline collection. Follow the
[rust-testing design contract](../../../.claude/skills/rust-testing/SKILL.md)
and [audit guidance](../../../.claude/skills/rust-testing/test-suite-audits.md).

## Relationship to prior work

The [2026-08-01 fix](../_completed/2026-08-01-cli-slow-tests/spec.md) migrated 29 formerly
slow CLI binaries plus launch-context subjects to `CliProcessFixture`, added
spawn/isolation guards, and reduced test-specific timeout floors. Its CI
acceptance evidence remains a separate obligation; this work neither replaces
nor weakens those targets.

The predecessor's [log](../_completed/2026-08-01-cli-slow-tests/log.md) records a residual
census of 170 raw sites in 36 exempt files: 168 sites in 34 files excluded by
scope and two Windows console-control sites needing a live child. These are
historical observations, not this fix's baseline. It also records inherited
`COLUMNS=44` failures in `compose_schema_cli` and `composition_outputs`.

Sequencing is decided, not open: the predecessor lands on `main` first, and
its three consecutive green CI runs — the evidence its log still defers —
are collected before any change from this fix lands. Those same runs are
this fix's CI baseline, so one push closes the predecessor's obligation and
opens this one's attribution window. Interleaving the two would make the
predecessor's evidence measure this fix's changes too.

The remaining source scans, rendering tests, library context capture, and
daemon fixtures were not covered by a migration selected from slow CLI cases.
A threshold-based follow-up would repeat the same selection defect.

## Scope

Inventory all tests and shared fixtures in the eight packages above: embedded
unit tests, integration targets, L1/L2/L3/real-resource tests, doctests, and
benchmark/fuzz entry points where present. Shared fixture machinery — the
command builder, `common/` helpers, and area pre-builds such as `_ensure-md` —
is part of that inventory. Include excluded and ignored tests, with their
reason and actual execution route. Inventory the rendezvous family explicitly
(`just test` inside `claudine/rendezvous`, or `just test-rendezvous` from the
parent area); the parent area's ordinary `just test` recipe covers only the
five claudine crates.

Already migrated tests receive evaluation and regression verification, not a
second mechanical rewrite. Production behavior changes, new provider features,
wholesale CLI/library restructuring, CI runner redesign, and generalized
cross-package fixture frameworks are outside scope. A production defect found
during attribution gets a linked issue/spec with evidence.

## Required behavior

### 1. Reconciled inventory

Produce `inventory.md` with every test identity assigned to exactly one
evaluation row or explicitly enumerated family. Record purpose, assertion
quality, shared helpers, required effects, CWD/home/cache/environment/tool
dependencies, timing floor, runner overrides in force, resource ownership,
tier/features/platforms, canonical recipe, observed cost, and disposition.

Reconcile source definitions with runner discovery and package manifests.
List cfg/feature exclusions separately from executed tests. A family is only a
grouping convenience: exceptions need their own entries. No test is excluded
because it is quick. Dispositions are satisfactory, remediation in this fix,
or a linked follow-up naming the unmet requirement and reason for deferral.

### 2. Finish the L1 process contract

Migrate every remaining ordinary L1 Claudine spawn to the existing fixture
policy, including completion, loop, sequence, context, schema, and resource
linking tests. Remove every generic "outside this fix's scope" exemption.

Extend the existing builder with one method that yields a
`std::process::Command` carrying the same environment policy — scrubbed
inheritance, pinned CWD and home variables, composed `PATH` — for live-child
tests. The policy is one shared implementation invoked by both the
`assert_cmd` path and the raw path, so the two interfaces cannot drift. The
call site keeps only what is the test's subject: Windows
`CREATE_NEW_PROCESS_GROUP`, targeted console signals, output draining, and
child reaping. Do not solve this by moving tests to L3 or dropping Windows
coverage.

Retain named, justified escapes and containment checks. Fixtures must remain
outside the checkout, including after canonicalization. Preserve native
Windows command requirements and portable PATH construction. Audit helper
commands such as fixture Git invocations too; isolating only the Claudine
child does not protect those commands from inherited Git plumbing.
Compile-verify the Windows-only arms with the area's `just check-windows`
(mingw `x86_64-pc-windows-gnu`, `--tests`) wherever that toolchain is
present; the `windows-latest` CI leg remains the authority where it is not.

The spawn and post-build isolation guards must cover the resulting command
forms. Keep meaningful negative cases and stale-exemption failures. The
isolation scan is textual and cannot resolve receivers, and this fix widens
its governed population from 32 files to the rest of the L1 suite, so a
`.current_dir` or `PATH` edit on an unrelated command in a governed file —
parent-side `git`, `rustc`, or `md` invocations — is resolved by an
allow-list entry naming the command the site targets, never by weakening
the detector or dropping stale-entry failure. Any remaining technical
exemption needs a specific necessity and equivalent isolation proof.

### 3. Audit non-spawn costs and test quality

Inspect all consumers of expensive context/discovery helpers, repeated source
scans, rendering/layout setup, and corpus loaders. Use synthetic context when
discovery is incidental; retain real discovery against test-built repositories
when it is the assertion's subject. Keep real shipped-prompt coverage using an
isolated copy with preserved relative references.

Evaluate the source-scan families (`error_guards`, placement, dispatch, spawn),
context rendering, and loop pause tests without assuming which dominates.
Nextest executes tests in separate processes, so a process-local cache alone
does not share a scan across test cases. Consolidate repeated work only when
coverage, diagnostic attribution, and selective execution remain useful; where
it is justified, the established shape is the shared passive corpus test — one
binary scanning once, extended instead of a new rescanning process per
regression — with representative end-to-end cases retained through real
shipped artifacts.

Pre-existing runner overrides are findings, not furniture. The inventory
records every `slow-timeout` override in force — the 30 s entry covering
`context_reports_preserve_all_columns_at_minimum_supported_width` is the
known example — and each is kept only with a justification tying its floor
to a real contract, or removed once the cost it was hiding is fixed.

Repair tautological assertions and stale test identities in separately
reviewable changes. Put exhaustive combinations at a cheaper boundary where
equivalent proof exists, retaining representative real-binary cases. Record
every assertion or test-population change and its replacement coverage.

### 4. Bound time and resource ownership

Replace readiness sleeps with bounded observation of the final required
condition. Retain intentional sleeps in timeout contracts and justify their
budgets, polling cadence, and shutdown margin. Coordinate with the
[startup-stall fix](../_completed/2026-08-31-silent-success-and-startup-stall/spec.md)
where clock semantics overlap: that fix is implemented, so timeout floors and
budgets recorded here are derived under its spawn-fallback silence clock
rather than the pre-fix first-event grace.

Daemon/session/IPC tests own isolated endpoints, data directories, processes,
and cleanup; unrelated tests disable reporting. Audit cleanup on panic, error,
and cancellation — proven for the process-owning cohorts with nextest's
per-test leak policy plus the root `just test-leaks` post-run sweep, not by
inspection alone — and keep runner-visible serialization only for resources
that are actually shared. No terminal or browser may gain focus.

### 5. Measurement and verification

Capture the baseline from the predecessor's post-landing CI record (see
Relationship to prior work), recording revisions, toolchain, features,
profile, concurrency, cache state, test identities, failures, skips, and
environment. Keep build/setup time, runner elapsed time, and summed test
duration separate.

Collect five alternating warm local runs per revision for changed cohorts and
the relevant full L1 suites. Re-run changed timeout/concurrency cases ten times
under representative suite load. Use work counters or sentinel effects to
prove eliminated discovery and unrelated launches independently of timing.

Retain baseline and three consecutive candidate CI runs for each configured
package/environment leg, including Claudine CLI's native Linux/macOS/Windows
and WSL2 legs. Record intervening failed attempts. Compare matched tests within
each environment; report additions and platform exclusions separately.
Document numeric budgets in `inventory.md` beside the baseline table — after
attribution and before remediation is evaluated — so budget review and
evidence review are one act. Report misses rather than inventing a universal
speedup percentage.

Use area `just test` / `just lint`, `just test-cli` for focused CLI work,
`just test-daemon` when relevant, `just test-contract` / `just test-gen` /
`just doctest` / `just bench` for those cohorts, and rendezvous's own recipes
(`just test` inside `claudine/rendezvous`, or `just test-rendezvous` from the
parent area). Run affected L2/L3/real tests through their canonical recipes
only with the needed resources. Unavailable runtime evidence remains pending.
No new slow-timeout override, retry, tier change, or disabled assertion may
substitute for a fix.

## Acceptance criteria

- Every discovered test/family has a reviewed disposition and a reconciled
  platform/feature/tier execution path; none is omitted by timing threshold.
- Generic residual spawn exemptions are zero; live-child and ordinary command
  paths share the fixture policy, with negative guard tests and Windows proof.
- Previously observed inherited-width failures are covered; representative
  application-variable, Git-plumbing, home/cache, PATH, and checkout-ancestor
  contamination probes cannot alter unrelated test results. Probes use only
  disposable state, never edits to the real checkout or user configuration.
- Shared-setup, cleanup, assertion, and reachability findings within scope are
  resolved. Every deferred finding has evidence, a reason, and a linked owner
  document; generic fixture migration cannot be deferred as out of scope.
- Every pre-existing runner override in the scoped packages is justified in
  the inventory or removed together with the cost it hid; none was added.
- Relevant local gates pass — including `just check-windows` for the
  Windows-only arms wherever the mingw toolchain is present — platform
  limitations are explicit, and the ratified performance budgets have
  compatible CI evidence.
- `results.md` records measurements, coverage changes, residual findings, and
  separate implementation/local/CI completion status. Area docs and skills are
  updated only where workflow or architecture changed.

## Draft decisions to resolve during baseline review

- Which non-spawn families account for the remaining execution cost?
- Which source scans can share work without losing independent failure detail?
- Which technical exceptions remain necessary after live-child support?
- What per-family timing budgets are justified on the existing CI runners?
