---
description: Plan evidence-based upgrades and dependency alignment across a Rust workspace, preserving intentional differences.
depends_on:
    binaries:
        - python3
        - cargo
        - git
initialize:
    info: "Preparing dependency evidence for {{ctx.repo_root}}. Collection may take several minutes."
start:
    info: "Starting dependency upgrade and alignment analysis. The agent will review evidence gaps and write a phased plan."
success:
    success: "Dependency planning session completed. Review the plan at {{ctx.repo_root}}/reviews/{{ctx.today}}-dependency-upgrade/plan.md."
blocked:
    warn: "Dependency planning was blocked before the agent started: {{err.msg}}"
failure:
    warn: "Dependency planning failed: {{err.msg}}. Any partial plan still needs review."
---
# Dependency Upgrade and Alignment Planner

Act as a senior technical analyst for the Rusty Biscuit monorepo. Create a
phased, evidence-based plan to upgrade third-party dependencies and synchronize
workspace declarations where doing so offers a concrete benefit.

This is a planning task. Write evidence artifacts and the plan only. Do not
modify Cargo manifests, Cargo.lock, source code, or toolchain configuration. Do
not run upgrades or implement the phases. A newer release alone is insufficient
justification for a change.

## Evidence collection

The collector requires Python 3.11+ and creates a fresh directory under
`target/dependency-report/` for each invocation. Cargo may access registries and
populate its cache. Missing cargo-outdated, command failures, timeouts, and
invalid JSON are recorded as unavailable evidence; they are not automatically
classified as resolver conflicts. Partial collection still permits a plan with
explicit blockers and uncertainty.

Composition runs the collector, including during Claudine `--dry-run`. The block
allows enough time for the collector's six reports, each bounded to 300 seconds,
plus tool and repository probes. Do not move setup into a `start` lifecycle
handler: this evidence must exist before the agent receives the composed prompt.

::shell-block timeout=2100
python3 scripts/dependency-report/collect.py
::end-block

Read the index emitted above before using any report. Its command records contain
exact arguments, working directory, exit status, stdout, stderr, and successful
JSON artifact paths. Its derived records identify successful reports and skipped
prerequisites. Use only artifacts from that invocation; an absent or failed
report is never evidence of an empty dependency set.

The collector runs at Claudine's launch repository root. If the helper is absent,
stop and report the missing prerequisite. Do not substitute evidence from another
checkout. Record the index path, repository revision, dirty state, collection
time, tool versions, and lockfile hash in the plan. If the lockfile changed during
collection, resolve the snapshot inconsistency before making firm recommendations.

### How to interpret the reports

- `metadata-declarations.json` preserves direct declarations without resolving
  external dependencies. Use it and `manifests.json` even when full resolution
  fails. If metadata discovery fails too, the snapshot covers only the root
  manifest: enumerate its workspace members and inspect their manifests before
  claiming complete declaration coverage.
- `metadata-default.json` and `metadata-all-features.json` are separate feature
  views. Record the actual scope of each; do not treat all-features as a universal
  valid build configuration. Metadata includes all target platforms by default;
  the tree reports use the host target. Obtain targeted evidence for relevant
  platform or feature differences before drawing conclusions from those reports.
- `workspace-direct-dependencies` and `workspace-requirements-by-dependency`
  reports include all declarations, including local dependencies. Separate
  internal workspace members from external registry, Git, and nonmember path
  dependencies. Group identity using name and source/path/registry, not name alone.
- `manifests.json` preserves authored TOML, including workspace inheritance,
  aliases, target tables, patches, and resolver settings. Compare root
  `[workspace.dependencies]` with member declarations. Inspect applicable Cargo
  configuration and toolchain files without copying credentials into reports.
- `resolved-duplicate-identities` reports distinguish version count from source
  count. Multiple package IDs do not necessarily mean multiple versions.
- `resolved-dependency-edges` and `reverse-dependency-edges` preserve exact package
  IDs, aliases, dependency kinds, and target conditions. Traverse these edges to
  establish which workspace packages retain each older version. Do not infer
  control merely because a crate name also appears in a direct declaration.
- Raw metadata retains external package descriptions, MSRV, license, repository,
  build-script/proc-macro targets, native `links`, and available features. Enabled
  features are recorded separately in `resolve.nodes[].features`. Use both when
  assessing risk and feature changes.
- `outdated.json`, when available, is a discovery input. Distinguish the installed
  version, latest compatible release, latest available release, and recommended
  target. A failed outdated query does not by itself prove the current workspace
  cannot resolve or build. Read the actual diagnostics.

## Analysis policy

### Upgrade value and risk

Prioritize concrete security fixes, relevant bug fixes, platform/compiler
compatibility, upstream support, public-type interoperability, and maintenance
cost. Identify the specific benefit of every proposed change. Do not promise
build-time or binary-size improvements merely from reducing lockfile duplicates.

For each recommended target, verify release availability, migration notes, MSRV,
feature changes, and supported platforms using authoritative registry, upstream,
and advisory sources. Cite links and retrieval dates. Treat missing evidence as
unknown; separate verified facts, inferences, and open questions. Do not claim
security coverage unless a current advisory review or scanner result supports it.

Flag build scripts, native links, proc macros, public API types, broad reverse
usage, and platform-specific behavior. Candidate families include CLI/TUI,
HTTP/TLS/networking, PDF/document parsing, SVG/image rendering, Git/native builds,
audio/platform bindings, serialization/configuration, and random/crypto
foundations. Name matching suggests investigation; actual dependency edges,
shared types, features, or upstream guidance must justify coordinated upgrades.
Do not assume a patch/minor change is low risk. Account for breaking pre-1.0
releases and any required Rust toolchain increase explicitly.

### Alignment is independent of duplicate removal

Examine every external dependency declared by multiple workspace packages,
including those currently resolving to a single version. Distinguish:

1. **Requirement alignment:** compatible declarations adopt a shared version policy.
2. **Workspace inheritance:** suitable common declarations move into
   `[workspace.dependencies]` with member `workspace = true` entries.
3. **Resolved-version consolidation:** a verified graph change can remove an
   unnecessary version or source identity.
4. **Intentional divergence:** incompatible requirements or justified differences
   remain documented with a revisit condition.

Compare requirements semantically, not only as strings. Preserve member-specific
features, default-feature behavior, optionality, dependency kinds, aliases,
sources, and target conditions. Inherited features are additive; putting features
in the workspace declaration can broaden every inheriting member. Keep shared
features minimal and inspect the workspace resolver. Validate affected packages
individually as well as in the relevant combined build to catch feature masking.

Do not standardize features just for consistency, assume centralization eliminates
duplicates, force all versions to converge, or use patches to disguise incompatible
requirements. Recommend centralization only where its maintenance benefit exceeds
migration cost. Retain public-type compatibility across package boundaries.

For each consolidation proposal, identify exact package IDs and sources, direct
requirements, the versions they actually resolve to, and reverse paths anchoring
older versions. State which specific changes would remove which identities and
which transitive anchors would remain. If that conclusion is unverified, label it
as a hypothesis and specify the smallest check that can resolve it.

### Ordering and validation

Use this priority order, adjusting it when evidence establishes a prerequisite:

1. Evidence/tooling blockers and confirmed urgent security or correctness issues.
2. Workspace-controlled alignment and consolidation with demonstrated benefit.
3. Low-risk leaf upgrades with independent validation.
4. Coordinated domain stacks and public-type migrations.
5. Old transitive anchors requiring upstream changes, replacement, or feature gating.

Transitive-only duplicates may remain when removing them offers little benefit or
requires disproportionate disruption. Include intentional exceptions rather than
silently omitting them. Separate independently reviewable changes; avoid phases
that combine unrelated migrations or cannot be reverted coherently.

Read the affected package-area recipes and repository testing guidance. Propose
exact commands with their working directories, package/feature/target scope,
expected results, and acceptance criteria. Include affected consumers when public
types or behavior change, and relevant macOS, Linux, native Windows, and WSL2
checks. Use actual repository commands; do not invent recipes or claim proposed
checks have run. Distinguish local verification from CI policy and reuse qualifying
passing evidence under repository rules. Do not schedule a full-workspace CI run
merely to discover scope. Include a post-change lockfile/graph comparison to verify
claimed consolidation, feature preservation, and remaining intentional duplicates.

## Required plan

Write an idiomatic Markdown plan with:

- Evidence inventory and limitations, including failed reports and actual causes
  where established. If evidence is insufficient, plan evidence recovery first
  and mark dependent recommendations provisional.
- An alignment decision table covering all external dependencies used by multiple
  workspace packages, including already-aligned entries and intentional exceptions.
- An upgrade recommendation table with dependency identity, current requirements
  and resolved versions, proposed requirement/target, affected packages/manifests,
  benefit, supporting evidence, confidence, and unresolved blockers.
- Separate action and compatibility classifications: `upgrade`, `align`,
  `centralize`, `consolidate`, `feature-gate`, `replace`, or `leave-alone`; and
  `patch`, `minor`, `major`, `pre-1.0-breaking`, `no-version-change`, or `unknown`.
  A recommendation may require multiple actions.
- Contiguous numbered phases. Each phase states its objective, prerequisites,
  concrete edits an implementer would make, MSRV/feature/public API/platform
  implications, validation commands, acceptance criteria, and rollback boundary.
- A deferred/intentional-divergence table explaining remaining duplicates and
  differing policies, their anchors, and measurable revisit conditions.
- Source citations for target-version and migration claims. Separate recommended
  validation from checks actually performed while preparing the plan.

## Closure

Create `reviews/{{ctx.today}}-dependency-upgrade/plan.md` relative to
`{{ctx.repo_root}}`. Include valid YAML frontmatter in the initial write:

- `start_phase`: integer index of the first phase, normally 1; use 0 for a distinct
  evidence-recovery or prerequisite phase.
- `phases`: integer index of the last phase, not the number of phases.

Number phases contiguously from `start_phase` through `phases`, inclusive. Verify
that the frontmatter matches the body, artifact/source links are accurate, and
every firm recommendation has supporting evidence. If the destination already
exists, read it first and preserve execution status or author annotations; do not
silently replace a plan already in progress. Finish by reporting the plan path,
principal recommendations, and unresolved blockers. Do not implement the plan.
