---
kind: feature
name: rust-evidence-client
date: 2026-11-06
status: finalized-spec
depends-on:
  - 2026-10-26-next-gen-pipeline
  - 2026-11-01-rust-pre-push
  - 2026-10-28-publish
next_spec: 2026-11-06-any-host-evidence
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
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-08
review_iterations: 0
implemented: false
needs_rulings: false
rulings:
  - id: evidence-cli-distribution-approval
    decided_by: Ken Snyder
    decided_on: 2026-11-08
    decision: >-
      Authorize public binary distribution of a dedicated evidence-client-cli
      package (binary evidence-client, paired with the evidence-client library)
      as versioned GitHub Release assets for macOS, Linux, and native Windows,
      usable by CI and WSL2. Every package-manager channel stays disabled.
---

# Move evidence to the store, in Rust

## Why

Hosts and CI both run the same large evidence scripts, reconciling notes,
artifacts, and receipts. With one store, every producer can do the same three
things: ask what is missing, run it, and write the result. CI becomes just
another host.

## Outcome

- A Rust client and verifier library, plus a CLI, is the only way to read or
  write evidence, on hosts and in CI.
- The store is authoritative for observations. The per-OS
  `refs/notes/ci-local/<environment>` notes are no longer written or read, and
  the replaced Python is deleted. The plan's own note,
  `refs/notes/ci-local/scope`, stays until `2026-11-06-rust-ci-planner` moves it.
- Behavior is unchanged: the same cells qualify as before. Changing the rules is
  the next spec's job, so that any difference found here is a bug.

## Scope and ownership

Provide a shared Rust client/verifier library and a thin CLI. Producers submit
attempts and consumers ask which planned cells remain unproved; neither the
client nor the store chooses additional tests. A cell remains keyed by
`{package, environment, gate}`. Package area is only a display grouping.

Inventory every command, import, and caller of these Python modules before
replacement, recording its Rust replacement or reason for retirement in the
implementation log:

| Existing module under `scripts/ci/` | Responsibility that must survive |
|---|---|
| `local_evidence.py` | Candidate qualification, failure/conflict resolution, local and rig recording, scope receipt recording and verification |
| `observation_notes.py` | Immutable observation identities, collection validation, retry links, concurrent publication safety |
| `hosted_observations.py` | Hosted completion/report validation, publication and recovery of unpublished attempts |
| `reuse_validation.py` | Same-tree pull-request validation reuse, bounded waiting, nightly anchors and coverage certification |
| `evidence_index.py` | Inspection of retained local runs, including unpublished runs and commits absent from the clone |

**Reader's note:** this replaces evidence storage, not all the responsibilities
hidden in those files. `local_evidence.py` also handles scope receipts, and
`reuse_validation.py` also handles whole-run reuse and nightly coverage.
Deleting either without replacements would change planning or repeat work.
Port their still-needed behavior here; do not force those records into an
observation table that cannot represent them. Keep scope receipts on
`refs/notes/ci-local/scope` with their existing format and transport until
`2026-11-06-rust-ci-planner` moves them. Its `--apply-to` planner operation also
stays there; it calls the Rust verifier rather than retaining a Python verifier.

The Worker API and durable report storage come from
`2026-10-26-next-gen-pipeline`. Consume its agreed API and authentication
contract; do not access D1 directly, submit SQL, or implement another store.
Release-readiness policy and any-host qualification are outside this port.

## Preserved qualification contract

The behavioral references are `docs/cicd/evidence.md`,
`docs/cicd/schema-versions.md`, and the existing verifier at implementation time.
Freeze fixtures before replacing it. In particular:

- Use the supplied canonical resolved plan, including its event, coverage
  boundary, filters, features, backends, build identity, and governed gaps.
  Never recompute scope or schedule environments the event did not select.
- Preserve Git reachability and coverage boundaries. Recompute gate-input
  equivalence against both committed trees; a stored input digest is a claim,
  not proof. Missing Git objects cannot earn reuse.
- Ordinary evidence qualifies only for its own environment. Preserve the
  existing exception for exact-tree narrowed test-input runs with the same
  filter from another host. Do not broaden rig or archive-mode eligibility.
- Gather qualifying failures as well as passes. Resolve attempts by ancestry
  and valid explicit retry links, never timestamps, insertion order, or an
  SQL query for the latest pass. A newer qualifying failure defeats an older
  pass; unordered pass/fail evidence remains a conflict and executes again.
- Preserve report size/digest checks, proof-document validation, and current
  foreign-host provenance rules. Complete failing attempts remain visible;
  incomplete attempts cannot satisfy coverage. A report-location event alone
  never certifies completion.
- Preserve check-versus-L1 reuse restrictions, non-reusable dependent checks,
  companion coverage, accepted gaps, and execution constraints. A gap remains
  distinct from a pass. Constraints stay local; the shared store cannot grant
  permission to execute a prohibited cell.

Transport may improve without changing qualification. For example, retaining
an artifact's bytes in durable storage can make its report available longer,
but cannot make an incompatible or untrusted attempt qualify.

## Identity and planner boundary

Move gate-input computation and qualification into the shared Rust library now.
Until the planner is ported, Python passes the canonical plan and explicit
base/head to the CLI and reads its structured decision output. The CLI computes
identities from repository objects and the existing recipe-level input rules;
Python must not keep a second implementation of that calculation. Use `sniff`
for discovery and in-process `gix` for Git reads. Scope-note fetch/push is an
explicit transport boundary honoring the user's Git configuration.

Preserve existing observation IDs and canonical bytes, including Python's
sorted, compact JSON and its escaping of non-ASCII characters. Existing
identities use SHA-256, not the default fast hash or BLAKE3. Use `biscuit-hash`;
if its public API lacks compatible SHA-256, add that support there rather than
silently changing algorithms or adding a private implementation. Verify
Unicode, nested objects, numeric values, and omission of `observation_id` from
its own identity against frozen Python outputs. Version-2 receipt adaptation
must be identical to the store's adapter; version-3 payloads remain unchanged.

Keep machine output versioned and separate from diagnostics: one decision per
requested cell, selected observation IDs, and stable rejection reasons. Human
output uses `biscuit-terminal`. Distinguish an ordinary reuse miss from invalid
input or a service failure so callers cannot turn a failed invocation into an
empty successful result. Preserve existing caller exit behavior at adapters.
Document format/version changes and every affected reader in
`docs/cicd/schema-versions.md`; do not bump unrelated document versions.

## Store reads, publication, and outages

Read candidates and report locations from one primary-backed evidence snapshot,
with the repository revision and both watermarks defined by the store contract.
Read every page through those watermarks, including failure candidates and retry
predecessors. Partial history, failed pages, or unavailable reports cannot
produce reuse. Check evaluation expiry and bind cached decisions to repository,
plan, head/tree, verifier policy, and evidence revision. A stale revision at
publication requires a fresh evaluation; expired or withdrawn locations cannot
revive an older attestation. This snapshot rule does not add a new merge gate
or promise to freeze evidence for an entire running workflow.

Upload and verify report and proof bytes before publishing reusable references.
Keep immutable original report references and attach new locations separately,
so moving bytes does not change an observation ID. Retry delivery with the same
ID and payload; equal duplicates succeed, conflicting duplicates fail visibly.
Preserve complete retry collections and their context during import and upload.

Use the prerequisite's host tokens and GitHub Actions identity permissions;
never trust repository or producer permissions supplied in observation JSON.
Keep credentials out of payloads, logs, locators, and test fixtures. Network
operations have bounded timeouts and retries. No authentication flow prompts
from a hook or unattended workflow, and untrusted pull-request code receives
no host token or publication credential.

When the store is unavailable, do not fall back to passing Git-note evidence or
an unvalidated cache. Leave affected required cells scheduled and report why.
If a local execution constraint then prohibits them, refuse execution using
the existing constraint behavior. Retain reports, proof documents, immutable
payloads, and retry context in a durable local pending-upload directory, with
an explicit retry command. A failed upload does not erase results or turn a
failed test green; preserve the existing distinction between gate failure and
advisory evidence-publication failure. CI retains recoverable attempt artifacts
when direct publication fails. Recovery imports attempts through the same Rust
client, not a second acceptance path.

## Migration and cutover

1. Inventory the live policy and transport-only reconciliation separately.
   Remove note-fetch/merge repair only when the store replaces its purpose;
   retain failure ordering, provenance, proof checks, and recovery.
2. Import version-2 and version-3 evidence using the original repository,
   environment, head, tree, and payload context. Record resumable progress and
   counts of imported, duplicate, rejected, and report-unavailable records.
   Importing metadata cannot repair expired reports. Resolve version-1 handling
   below before cutover.
3. Compare both verifiers on pinned plans and identical frozen candidate/report
   availability snapshots. Compare accepted cells, chosen IDs, rejection codes,
   conflicts, and nightly blocking evidence. Storage retention improvements are
   tested separately so they cannot disguise a policy difference.
4. Briefly pause old observation publishers, drain/import their final records,
   and switch all producers and readers together. Verify migration completion
   before disabling per-environment note reads/writes. Existing notes may remain
   as historical data; do not delete refs as part of cutover.
5. Remove replaced Python and its production imports only after every caller
   has moved, including the hook, Just recipes, CI producers, nightly anchor
   checks, seed/cross-check tools, and inspection commands. Keep only planner
   glue and scope-note transport still required by their documented boundary.

No lasting dual-write or automatic fallback reader is allowed. A rollback
requires an explicit coordinated change and export of post-cutover attempts
before restoring old readers; otherwise failures recorded only in the store
could disappear. The pending-upload path remains usable during recovery.

## Bootstrap and distribution

Pin the CLI's exact package release, target, checksum, and supported API/schema
versions in CI. Use a version-specific asset, never a repository-wide latest
release. Reject incompatible versions before reading evidence. The release
workflow uses an already published, known-good version, not the binary being
released; recovery can pin another compatible release or explicitly build a
reviewed revision with the lockfile. Failed installation must be visible and
cannot silently skip planning or verification.

Keep the pre-push launcher's worktree-aware Cargo freshness behavior established
by `2026-11-01-rust-pre-push`; CI prebuilt installation does not authorize using
a stale global binary from a developer hook. Supporting native Windows and
WSL2 is required even if Windows distribution approval is still pending.

**Reader's note:** `2026-10-28-publish` currently lists eight initial commands;
this tool is not one of them, and Windows assets have a separate readiness
condition. That dependency supplies a distribution mechanism, not approval to
ship a ninth command. The ruling below approves this package for GitHub
Release assets only; produce those assets before switching CI.

## Verification and acceptance

Use the repository's `rust-testing` skill, test toolkit, canonical Just recipes,
and nextest. Exercise pure decisions with frozen fixtures and transport through
a local fake service; ordinary tests need no live credentials or focused
terminal/browser window. Register ownership through existing CI test-input
rules rather than creating a separate all-OS workflow.

Acceptance requires:

- Decision parity on exact-tree and equivalent-input candidates, divergent
  histories, same-commit retries, conflicts, newer failures, narrowed filters,
  feature/backend changes, missing objects, legacy inputs, and unavailable or
  withdrawn reports. Every mismatch is fixed or explicitly reviewed as a
  separate policy change before cutover.
- Client cases for pagination, changed snapshots, expiry, interrupted uploads,
  duplicate/conflicting delivery, unauthorized publication, and recovery.
- Focused store integration proving authenticated import, durable report
  retrieval, and restartable publication against the prerequisite's API.
- Passing client/verifier coverage on macOS, Linux, native Windows, and WSL2,
  reusing qualifying evidence. This is portability verification, not a new
  performance study or a change to CI's event schedule.
- No production references to retired per-environment note paths or Python
  evidence readers, except the explicit migration/import tool. The scope note
  still works, and pinned-plan whole-run reuse and nightly anchors retain their
  previous decisions. Evidence inspection includes unpublished retained runs.
- Updated `docs/cicd/evidence.md`, relevant CI topic pages and Just help,
  `docs/dependencies.md` if dependencies change, and the rust-devops skill.
  Keep READMEs as overviews. Record migration results, parity coverage, package
  choice, and departures from this snapshot in the implementation log.

No performance spike is required by this port. Avoid per-report startup costs
and unbounded buffering through the shared client and paginated API; do not
add a multi-host benchmark program without an author decision.

## Open Questions

### How should version-1 receipts survive the storage cutover?

The current verifier accepts version-1 whole-environment receipts on exact tree
identity. The proposed store explicitly excludes them. Removing note reads
without addressing this would contradict the promise of unchanged behavior.
Settle this before implementation declares parity.

- **Add an explicit legacy envelope to the store (recommended).** Preserve the
  original receipt and context, and apply only its existing exact-tree rules.
  Pros: preserves behavior without pretending legacy receipts contain modern
  per-cell proof. Cons: requires a small prerequisite API/schema extension and
  legacy reader maintenance. Recommend this because unchanged qualification
  is this port's central constraint.
- **Explicitly retire version-1 reuse at cutover.** Pros: keeps the store model
  simpler and only causes extra tests. Cons: changes behavior and can make a
  previously satisfied local constraint impossible to meet without new proof.
  Requires an author-approved exception, documented impact, and rerun guidance.
  Do not silently synthesize modern observations from missing legacy fields.

### Which package and release assets own the shared client and CLI?

**Ruled 2026-11-08: a dedicated `evidence-client` / `evidence-client-cli`
package area, approved for GitHub Release assets only.**

Create the package area following the `{name}` / `{name}-cli` convention, and
make `repo-deps` consume the library. The CLI's binary is `evidence-client`.

Prebuilt assets are required rather than building from source in CI: the WSL2
leg executes on a toolchain-free host, and compiling the client in every job
would add build time to every cell.

Add the package to `docs/deployment.yaml` with `approved: true` and every
channel (`brew`, `binstall`, `install`, and the rest) set to `false`:

```yaml
    evidence-client-cli:
        {
            crate: evidence-client-cli,
            approved: true,
            brew: false,
            # ...every other channel: false
        }
```

Its release attaches version-specific, checksummed assets for macOS, Linux
(the publishing feature's glibc 2.35 floor), and native Windows through the
publishing feature's mechanism. This may be the first approved package; that
does not approve any other package or channel.

The plan still settles target selection, the first pinned release, API and
schema compatibility checks, and the recovery version before CI cutover.
Windows asset readiness cannot be inferred from macOS/Linux release success:
until the Windows asset exists, the native Windows and WSL2 cutover remains
pending even where macOS and Linux have switched, and the pinned-install
acceptance condition for those environments stays pending.

Rejected alternatives: keeping library modules and a CLI target in
`repo-deps` (ships unrelated repository tools' dependency closure with the
client), enabling package-manager channels (no consumer outside this
repository), and withholding approval (blocks the CI cutover and the Python
retirement behind it).

## Not in scope

Any-host qualification, new event triggers, merge/release policy changes,
porting the planner, moving scope-note storage, or retiring Python outside the
replaced evidence path. Removing redundant reconciliation does not authorize
removing a qualification rule.
