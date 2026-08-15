---
total_phases: 5
created: 2026-08-15
phase: 1
agent: codex/default
yolo: "true"
---

# Remote (S3/R2) kache Backend for CI — Execution Plan

Reference: [`spec.md`](spec.md)

## Objective

Determine, with reproducible evidence, whether an R2-backed kache store beats
the current `Swatinem/rust-cache` path for each hosted CI operating system and
job family. Roll out only the combinations that meet the specification's hit
rate, wall-time, reliability, and cost gates while preserving the repository's
no-tracked-wrapper policy and fail-open behavior.

## Completion contract

The work is complete when:

1. Trusted hosted CI can activate exactly kache 0.12.0 through the pinned local
   composite action, while disabled, untrusted, fork, self-hosted, and
   incompletely configured jobs retain their existing safe cache path.
2. Every enabled OS/job-family combination has a same-commit three-arm A/B
   record in `measurement.md`, including all raw artifacts and the required
   acceptance calculations.
3. Only combinations that meet their own acceptance bar use kache in production;
   all others remain on `rust-cache` or are explicitly retired from the trial.
4. Missing credentials, an unreachable endpoint, and a corrupted pilot object
   have each been proven unable to turn a correct build red or silently select
   kache's GitHub Actions cache backend.
5. Repository contracts, workflow documentation, the kache strategy, the local
   kache skill, and the maintenance audit describe the shipped behavior without
   stale references to the retired trial.
6. Developer-machine reads remain disabled unless a separate security review
   explicitly accepts their provenance boundary.

## Governing decisions and dependencies

- The specification's Phases 0–4 are renumbered here as Phases 1–5 so execution
  begins with the required standard Phase 1.
- Cloudflare R2 is the only backend in this feature. A homelab MinIO backend is
  a separate follow-up because GitHub-hosted runners cannot reach the private
  LAN endpoint.
- Adopt the companion remote-runner specification's newer trust contract:
  immutable `CI_TRUSTED_ACTOR_IDS` is the publisher identity boundary, while
  `CI_TRUSTED_AUTHORS` remains supporting policy metadata. Do not implement the
  older `CI_TRUSTED_ACTORS` name from this specification draft.
- There must be one shared `trusted_ci_run` output. If the remote-runner feature
  has already created it, consume it; otherwise create it once in the hosted
  preflight so the remote-runner feature can consume the same output later.
- Production activation requires all of: `trusted_ci_run == 'true'`,
  `CI_KACHE_ENABLED == 'true'`, a GitHub-hosted runner, complete S3
  configuration, and successful verification of the pinned binary.
- The recommended provenance decision is accepted for planning: trusted CI may
  read and write the pilot bucket, but persistent developer-machine reads stay
  disabled pending a separate security review in Phase 5.
- Phases are sequential. Within a phase, tasks explicitly marked
  **Parallelizable** may proceed concurrently after their named prerequisite is
  complete.
- If the companion remote-runner feature lands before the pilot, re-measure the
  remaining trusted hosted/fallback population before provisioning R2. Stop
  after documenting the decision if that population cannot justify a useful,
  fresh remote cache.

## Phase 1 — Security boundary, R2 foundation, and pinned action

- [ ] **Task 1.1: Ratify the shared trust and rollout contracts before changing workflows.**
  - Compare the landed state of
    `features/2026-08-15-cicd-remote-runners/spec.md`, `.github/workflows/ci.yml`,
    and the hosted preflight implementation.
  - Record in `measurement.md` whether this feature creates or consumes
    `trusted_ci_run`; use `CI_TRUSTED_ACTOR_IDS` and `CI_TRUSTED_AUTHORS` in
    either case.
  - Prove the gate fails closed for fork pull requests, actors outside the ID
    allowlist, authors outside the email allowlist, and events without an
    auditable commit range. Keep `workflow_dispatch` outside the trusted path.
  - Align `spec.md` with the ratified actor-ID contract so the implementation
    does not leave known documentation drift behind.

- [ ] **Task 1.2: Provision the dedicated R2 cache boundary.**
  - **Parallelizable with Task 1.3 after Task 1.1.**
  - Create one bucket used only for this repository's kache objects.
  - Apply an administrator-managed lifecycle rule that expires artifacts,
    manifests, and shards 30 days after upload and cleans incomplete multipart
    uploads; do not expose the administrator credential to GitHub Actions.
  - Create a bucket-scoped Object Read & Write token for trusted CI. Do not
    grant account or bucket-administration permissions.
  - Add repository variables for bucket name, full R2 endpoint, region `auto`,
    `KACHE_CACHE_GENERATION=v1`, and `CI_KACHE_ENABLED=false`; add only the
    access-key ID and secret as repository secrets.
  - Record sanitized bucket, lifecycle, token-scope, and variable verification
    evidence in `measurement.md`; never write credential values to a repository
    file, command output, job log, or artifact.

- [ ] **Task 1.3: Implement `.github/actions/kache-s3/action.yml` as the only CI activation surface.**
  - **Parallelizable with Task 1.2 after Task 1.1.**
  - Define explicit inputs for `enabled`, bucket, endpoint, region, generation,
    experiment/rollout prefix, manifest key, namespace, credentials,
    `cache-executables`, `max-size`, and `min-compile-ms`.
  - Read the exact binary version from `.github/kache-version`; invoke
    `kunobi-ninja/kache-action@a257c055543c2840700a9bbca8f9c3094a421b1b`
    with that version and never a floating tag.
  - Pass `github-cache: false`, `sync: false`, `warm: true`, and
    `pr-comment: false` explicitly. Generate the S3 prefix as
    `artifacts/<generation>/<experiment-arm-or-rollout>`.
  - Validate every required S3 field before invoking the upstream action. On a
    disabled or incomplete configuration, leave `RUSTC_WRAPPER` empty, report
    the exact inactive reason in the job summary, and do not invoke the
    upstream action.
  - Verify `kache --version` after installation without failing the job. Export
    `active` and `inactive-reason` outputs; clear `RUSTC_WRAPPER` and report
    inactive when installation or version verification fails.
  - Keep the implementation portable across the action's Bash environments on
    macOS, Linux, and Windows; do not introduce OS-specific path separators,
    unguarded Unix utilities, or tracked Cargo wrapper configuration.

- [ ] **Task 1.4: Replace the retired no-kache assertion with durable workflow contracts.**
  - Update `tools/test-toolkit/tests/ci_workflow_contracts.rs` to retain the
    no-tracked-`.cargo/config.toml` and explicit-local-opt-in contracts while
    allowing CI activation only through `.github/actions/kache-s3`.
  - Assert the exact upstream action SHA, `.github/kache-version` authority,
    `github-cache: false`, `sync: false`, `warm: true`, `pr-comment: false`,
    complete-input validation, effective activation outputs, and absence of a
    floating `@v1` reference.
  - Assert call sites require the shared trust output and kill switch, preserve
    `rust-cache` when kache is inactive, do not activate on self-hosted runners,
    and never activate inside the WSL guest.
  - Assert manifest and namespace values are explicit, pilot arm prefixes are
    isolated, and evidence artifact names include arm, package, family, OS,
    run ID, and run attempt.
  - Extend `.github/workflows/maintenance-audit.yml` so the upstream action SHA
    is audited independently of the kache binary version.

- [ ] **Task 1.5: Add a trusted-push smoke path for all supported hosted operating systems.**
  - Create the initial push-only pilot workflow on one named trusted pilot
    branch; do not add `workflow_dispatch`.
  - Exercise the local composite on `ubuntu-latest`, `macos-latest`, and
    `windows-latest` with production callers otherwise unchanged and the kill
    switch still defaulted to false.
  - Verify each enabled smoke leg installs exactly kache 0.12.0, reports the
    expected active state, addresses R2 rather than GitHub Actions cache, and
    does not add `pull-requests: write` or broader permissions than
    `contents: read`.
  - Verify disabled and incomplete-configuration legs report inactive, keep
    `RUSTC_WRAPPER` empty, and select the existing `rust-cache` path.
  - If the pinned upstream action fails only on Windows, capture that evidence
    before implementing the documented non-interactive `cargo binstall`
    fallback; keep `.github/kache-version` as the sole binary authority.

- [ ] **Validation checkpoint 1: Prove the foundation is safe before compiling a pilot package.**
  - Run `python3 scripts/ci/test_affected_scope.py` if the shared preflight or
    scope logic changed.
  - From `tools/`, run `just test` and `just lint`; confirm the updated
    `ci_workflow_contracts` suite passes.
  - Inspect the three hosted smoke summaries and verify secrets are redacted,
    no kache-created GitHub cache entry exists, and incomplete configuration
    demonstrably falls back to `rust-cache`.
  - Keep `CI_KACHE_ENABLED=false` until Tasks 1.1–1.5 and this checkpoint are
    complete.

## Phase 2 — Reproducible Linux/macOS three-arm pilot

- [ ] **Task 2.1: Freeze the pilot population and measurement record before observing kache results.**
  - Create `features/2026-08-15-remote-kache/measurement.md` and record the
    slowest representative current `test` compile selected from baseline data,
    its package inputs, exact commands, toolchain, `Cargo.lock` hash, and the
    reason for selection.
  - Record the pilot branch, Linux/macOS runner labels, initial image IDs, kache
    version, upstream action SHA, generation, two isolated kache prefixes, and
    all exclusions before the first measured run.
  - Define three modes with stable names: control (`rust-cache` only), kache
    dependency-only, and kache with `cache-executables: true`.
  - Record the no-kache stopping outcome as valid before collecting data; do not
    redefine thresholds after results are visible.

- [ ] **Task 2.2: Add an experiment-only cache-mode input to the canonical package workflow.**
  - Default the input to today's `rust-cache` behavior so every production
    caller remains unchanged.
  - For `check` and `test`, make the three modes mutually exclusive. Invoke the
    local composite for a requested kache mode and run `Swatinem/rust-cache`
    whenever the composite's effective `active` output is not `true`.
  - Derive `manifest-key` as `<package>/<build-family>/<environment>` and
    `namespace` as `<build-family>/<runner-os>/<runner-arch>`. Keep `check`
    distinct from `test`; map later `l2` and `browser` work to family `test`.
  - Preserve the current `shared-key` exactly for every control or fallback
    leg. Never allow a restored `target/` and active kache wrapper in the same
    experimental job.
  - Set the local store cap from the prime-run peak plus at least 20% headroom,
    measuring the executable arm independently; retain `min-compile-ms=1000`
    initially and record any later tuning as a new comparison.

- [ ] **Task 2.3: Emit complete, collision-free evidence from control and kache jobs.**
  - Add an `if: always()` evidence step after the build/test command and before
    action post cleanup. For active kache jobs, write JSON and Markdown from
    `kache report --format json --since 24h` and the matching Markdown report.
  - Include requested and effective mode, inactive reason,
    `cache-executables`, prefix generation, manifest key, namespace, exact
    kache version/action SHA, hit/miss/error counters, weighted hit rate, time
    saved, transfer bytes, and setup/build/post durations when available.
  - Include commit SHA, lockfile hash, Rust toolchain, runner
    OS/architecture/image, package, family, exact command, run ID, and run
    attempt in both kache and control evidence.
  - Upload each sample under an artifact name containing arm, package, family,
    OS, run ID, and `${{ github.run_attempt }}`. Treat missing or unparsable
    evidence as an invalid sample, never as zero activity.

- [ ] **Task 2.4: Expand the pilot workflow into a same-commit A/B matrix.**
  - **Parallelizable with Task 2.3 after Task 2.2.**
  - Schedule all three cache modes for the selected package's canonical
    `check` and `test` setup and commands on Linux and macOS from one trusted
    push. Disable unrelated job families without shortening prerequisites,
    fixtures, native setup, or nextest installation.
  - Allow arms to execute in parallel, but isolate their prefixes and local
    targets so no arm can consume another arm's state.
  - Keep the trust gate and `CI_KACHE_ENABLED` requirement identical to the
    eventual production path.

- [ ] **Task 2.5: Prime and execute the repeatable pilot.**
  - Set `CI_KACHE_ENABLED=true` only after Checkpoint 1, then run one cold prime
    for each kache prefix and exclude those cold samples from warm aggregation.
  - Re-run the exact same workflow run at least five times so every measured
    attempt uses byte-identical source and configuration; preserve all control
    and kache artifacts.
  - Record every sample, not only medians, and flag runner-image drift rather
    than attributing it to caching.

- [ ] **Task 2.6: Execute and document the fail-open rehearsals.**
  - Prove a missing credential skips activation and retains `rust-cache`.
  - Prove a valid-looking credential pointed at an unreachable endpoint yields
    a successful ordinary compile through kache's local miss/pass-through path
    and records the remote error.
  - Corrupt one object only in a disposable pilot generation and prove kache
    rejects it as a miss/error rather than serving it or failing the build.
  - Verify generation increment abandons the corrupted contents immediately;
    keep the retired generation only long enough for diagnosis.

- [ ] **Validation checkpoint 2: Apply the acceptance bar without exceptions.**
  - For each Linux/macOS `check` and `test` result, require at least 60%
    compile-cost-weighted hits on repeat configurations, at least 20% lower
    median warm wall time versus control with the distribution clear of the
    hosted noise floor, zero kache-attributed red runs, complete reports, no
    GitHub-cache fallback, and a projected R2 bill no greater than $10/month.
  - Select `cache-executables: true` only where it improves median wall time by
    at least another 10% over dependency-only kache while staying within the
    cost cap.
  - Record R2 stored bytes, Class A/B operations, observed CI volume, cost
    projection, all exclusions, raw artifact links, and a pass/fail decision per
    OS and family in `measurement.md`.
  - If no combination clears the bar, set `CI_KACHE_ENABLED=false`, preserve
    the evidence, have an authorized operator delete the dedicated pilot bucket
    after confirming it contains no other data, document the negative result,
    and stop implementation before Phase 3.

## Phase 3 — Evidence-gated Linux/macOS rollout

- [ ] **Task 3.1: Activate only the Linux/macOS `check` and `test` combinations cleared in Phase 2.**
  - Pass the single `trusted_ci_run` result into the reusable package workflow
    rather than recomputing trust in each job.
  - Enable the local composite only for a cleared OS/family on a GitHub-hosted
    runner when `CI_KACHE_ENABLED == 'true'`; retain `rust-cache` for every
    untrusted, fork, disabled, self-hosted, inactive, or uncleared leg.
  - Use one rollout prefix within the active generation so packages can share
    content-addressed dependency blobs; do not add branch or package names to
    the prefix.
  - Preserve job commands, result identities, JUnit/status artifacts, timeouts,
    workflow permissions, and the wrapper-free preflight probe.

- [ ] **Task 3.2: Measure `lint`, `l2`, and `browser` independently before enabling them.**
  - **Parallelizable by job family after Task 3.1 is stable.**
  - Run the same mutually exclusive three-arm protocol and evidence contract
    for each family and operating system; do not infer acceptance from shared
    dependencies or current `rust-cache` keys.
  - Keep `lint` isolated from `test` because `RUSTFLAGS=-D warnings` changes the
    artifact key. Use family `test` for `test`, `l2`, and `browser` manifest and
    namespace identities while retaining package/environment-specific
    manifests.
  - Enable only family/OS pairs that independently meet the Phase 2 acceptance
    and cost bars; leave every other pair on its exact current `rust-cache`
    path.

- [ ] **Task 3.3: Update repository contracts and operational documentation to the shipped rollout.**
  - Update `docs/kache-strategy.md`, `.github/ci/README.md`, relevant workflow
    header comments, `.claude/skills/kache/remote-cache.md`,
    `docs/topics/ci-cd.md`, and `docs/testing-strategy.md` where their public CI
    behavior changed.
  - Rewrite the old action/Windows limitation as historical evidence and
    document the separate action-SHA and binary-version maintenance authorities.
  - Document the kill-switch, generation-bump, credential-rotation, report
    locations, 30-day non-read-refreshing lifecycle, cost review, and
    self-hosted no-cache policy.
  - Confirm no documentation implies kache caches C/C++ artifacts remotely,
    bounds `target/`, preserves Cargo incremental compilation, or activates on
    developer machines by default.

- [ ] **Validation checkpoint 3: Demonstrate a stable Linux/macOS production window.**
  - Run `python3 scripts/ci/test_affected_scope.py` when applicable, then run
    `just test` and `just lint` from `tools/`.
  - Observe at least five repeat production runs for each enabled combination;
    require the same 20% wall-time improvement and zero kache-attributed red
    runs used for pilot acceptance.
  - Toggle `CI_KACHE_ENABLED=false` in a controlled trusted run and prove all
    affected jobs immediately return to `rust-cache`; restore it only after the
    rollback evidence is recorded.
  - Confirm self-hosted jobs, fork/untrusted jobs, and WSL guest execution expose
    neither R2 credentials nor an active wrapper.

## Phase 4 — Windows copy-mode and WSL archive-builder evaluation

- [ ] **Task 4.1: Run a Windows three-arm pilot through the same pinned composite.**
  - Reuse the Phase 2 protocol and evidence schema on `windows-latest`, including
    separate prime and five-or-more identical-commit warm attempts.
  - Record copy restore time and download time against avoided compile time by
    crate size class; do not inherit the Linux/macOS decision on NTFS.
  - If selective warm prefetch is unreliable, create a separately labeled
    diagnostic arm for explicit `kache sync --pull`; do not silently replace
    selective warming, and reject a full pull that costs more than compiling.
  - Evaluate dependency-only and executable caching independently against the
    same 60% weighted-hit, 20% wall-time, additional 10% executable-arm, zero-red,
    and $10/month aggregate cost bars.

- [ ] **Task 4.2: Evaluate the Linux `_wsl-ci.yml` archive builder without touching the guest.**
  - Add the mutually exclusive cache modes only to the `ubuntu-latest` archive
    job that compiles `x86_64-unknown-linux-gnu` test binaries.
  - Use build family `wsl-archive`, an explicit package/environment manifest,
    and a `wsl-archive/<runner-os>/<runner-arch>` namespace.
  - Collect the same prime, repeated-run, executable-arm, transfer, reliability,
    and cost evidence as other families.
  - Assert the Windows-hosted WSL guest job receives no R2 credential, performs
    no kache setup, keeps `RUSTC_WRAPPER` empty, and continues to compile
    nothing.

- [ ] **Task 4.3: Roll out only independently accepted Windows/archive combinations.**
  - Enable kache per Windows family or archive-builder result only after its own
    acceptance record is complete; retain `rust-cache` everywhere else.
  - Reconcile the decision with the landed remote-runner routing policy: an
    accepted cache is used only for hosted fallback jobs, never to overwrite a
    self-hosted runner's warm per-slot target.
  - Record whether Windows used the pinned upstream installer or the evidenced
    fallback, and keep that choice covered by workflow contracts and the
    maintenance audit.

- [ ] **Validation checkpoint 4: Close cross-platform CI evaluation.**
  - Run the workflow contract and scope suites plus `just lint` from `tools/`.
  - Observe at least five repeat production runs for every newly enabled
    Windows/archive combination and confirm its accepted metrics persist.
  - Verify macOS, Linux, native Windows, and the Linux archive builder all use
    explicit per-platform decisions, while the WSL guest and every self-hosted
    runner remain kache-off.
  - Update `measurement.md` with the final per-OS/per-family decision matrix and
    the aggregate projected monthly R2 cost.

## Phase 5 — Developer-read security decision and feature closure

- [ ] **Task 5.1: Resolve the developer-machine provenance boundary before distributing read access.**
  - Conduct and record a separate security review of the specification's
    Options A–C. Performance results alone cannot authorize persistent clients
    to consume executable artifacts written under a long-lived CI credential.
  - If Option B remains selected, document that developer reads are deferred,
    create no read-only credential or sync recipe, and treat that explicit
    decision as successful Phase 5 closure.
  - If Option A is approved, record the accepted threat model and provision a
    distinct read-only bucket token outside the repository before Task 5.2.
  - If Option C is required, scope short-lived credentials and signed artifact
    verification as a separate feature; do not invent that contract inside this
    rollout.

- [ ] **Task 5.2: Add opt-in developer warming only if Option A received explicit approval.**
  - Add a root `just` recipe that wraps selective `kache sync --pull` for the
    approved namespace/generation without installing kache, exporting a
    persistent `RUSTC_WRAPPER`, writing credentials, or changing `just init`.
  - Read the exact local version from `.github/kache-version` and fail with an
    actionable message when the binary or required read-only configuration is
    absent.
  - Document per-host opt-in, credential storage outside the repository,
    generation changes, filesystem-specific restore behavior, and the fact that
    kache disables Cargo incremental compilation when active.
  - Validate on each intended macOS, Windows, and Linux host class with
    `kache doctor`, `kache stats --since 24h`, store/target filesystem and disk
    measurements, and a representative warm build before recommending use.

- [ ] **Task 5.3: Retire experiment-only surfaces and finalize the evidence record.**
  - Remove the temporary pilot workflow and experiment-only inputs after all
    accepted family decisions are encoded in production; retain reusable report
    generation needed for ongoing monitoring.
  - Confirm `CI_KACHE_ENABLED` reflects the final decision, the active
    generation is healthy, retired generations have an authorized deletion
    date, and lifecycle/cost monitoring has an owner.
  - Ensure `measurement.md` links all raw samples, lists invalid/excluded
    samples, records negative results as well as wins, and states the final
    backend, trust, executable-caching, and developer-read decisions.
  - Review every touched workflow comment and public document for behavioral
    drift, treating the shipped workflow as authoritative.

- [ ] **Validation checkpoint 5: Verify the final repository and operational contract.**
  - Run `python3 scripts/ci/test_affected_scope.py` when applicable, then run
    `just test` and `just lint` from `tools/` without running `cargo fmt`.
  - Verify the final workflow contracts cover the exact action SHA, binary
    version authority, trust/kill-switch/fallback rules, identity isolation,
    evidence artifacts, self-hosted exclusion, and WSL guest exclusion.
  - Perform one final trusted hosted run and one untrusted/fork-equivalent run;
    confirm the former uses only accepted kache combinations and the latter uses
    only the existing `rust-cache` path without receiving R2 secrets.
  - Confirm a bucket outage still degrades to a correct uncached compile, the
    projected aggregate bill remains at or below $10/month, and all enabled
    family/OS combinations retain their accepted performance over the final
    observation window.
