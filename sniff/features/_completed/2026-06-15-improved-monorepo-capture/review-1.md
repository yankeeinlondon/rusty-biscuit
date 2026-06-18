---
ready: false
agent: codex
model: ""
---

# Review: Improved Monorepo Capture

## Findings

### High: Binary resolution executes repository and PATH binaries during detection

- Code: `sniff/lib/src/filesystem/repo/detection.rs:295`, `sniff/lib/src/filesystem/repo/standard.rs:497`, `sniff/lib/src/filesystem/repo/standard.rs:513`, `sniff/lib/src/filesystem/repo/standard.rs:527`, `sniff/lib/src/filesystem/repo/standard.rs:589`
- Requirement: spec section 8 says "sniff never executes a binary to detect packages" and the acceptance criteria say no package detection test may require `cargo`, `pnpm`, `go`, `gradle`, or another external monorepo binary to be installed. Binary availability should use `LanguagePackageManager` / `ExecutableIndex` with synthetic entries in tests.
- Problem: every detected standard goes through `resolve_acting_binary`, which calls `probe_version`. That spawns the resolved PATH binary or repo-local wrapper script with `--version`. For wrappers such as `gradlew` / `mvnw`, this executes code from the repository being inspected. For PATH binaries, detection now depends on installed tools and can add up to a 2s timeout per standard.
- Impact: this violates the filesystem-only cost and trust boundary. It also makes host-dependent behavior observable in ordinary repo detection, not just in an optional binary availability path.
- Suggested fix: keep `ResolvedBinary` to "found / not found / wrapper vs PATH" from `ExecutableIndex` and wrapper existence checks only. Leave `version` and `satisfies_min_version` as `None` unless version data is already available from an existing registry cache that does not spawn. If active probing is still wanted, gate it behind an explicit non-default request outside package detection.
- Test rigor: strongest present coverage is Level 1 unit tests using `resolve_acting_binary_with_version`, which deliberately avoids spawning. There is no Level 1 regression test proving normal `detect_repo_structure` does not execute a fake wrapper or PATH binary. Level 1 is sufficient for this non-terminal requirement, but the wrong path is not covered.

### High: The topology model is not actually a forest for root-manifest standards

- Code: `sniff/lib/src/filesystem/repo/detection.rs:187`, `sniff/lib/src/filesystem/repo/detection.rs:206`, `sniff/lib/src/filesystem/repo/detection.rs:233`, `sniff/lib/src/filesystem/repo/detection.rs:278`; representative detector: `sniff/lib/src/filesystem/repo/cargo.rs:17`
- Requirement: topology detection is a forest. The spec explicitly calls out examples like a Cargo workspace containing a pnpm workspace several directories down, and says the walker should consult `spec().nesting_policy`.
- Problem: Cargo, npm, pnpm, Yarn, Bun, uv, Go, Gradle, Maven, .NET, Rush, Nx, Turbo, and Lerna are only invoked against the supplied `root`. Only Bazel/Pants/Buck2 return multi-root outcomes via leaf-marker walkers. `NestingPolicy` is metadata, but no generic marker walk uses it for root-manifest standards.
- Impact: nested workspaces are silently missed unless their marker also exists at the repo root. A repo with root Cargo plus `web/pnpm-workspace.yaml` will report only the Cargo layer, losing the nested pnpm standard, packages, matched marker, and binary metadata.
- Suggested fix: add a bounded workspace-root discovery pass over marker files from the descriptor table. For each candidate root, run the relevant detector and apply `NestingPolicy` to decide whether to descend or segment. Keep structure mode cheap by reusing the manifest inventory or a marker-only walk.
- Test rigor: existing Level 1 tests cover same-root mixed Cargo + pnpm and Bazel nested roots, but not nested root-manifest standards. Level 1 fixture coverage is sufficient here; add cases for root Cargo containing nested pnpm/uv/go and for nested Cargo being excluded or segmented according to policy.

### High: A virtual Cargo workspace with one member is reported as a monorepo

- Code: `sniff/lib/src/filesystem/repo/standard.rs:448`, `sniff/lib/src/filesystem/repo/standard.rs:452`, `sniff/lib/src/filesystem/repo/standard.rs:454`; Cargo descriptor: `sniff/lib/src/filesystem/repo/standard.rs:680`
- Requirement: the default non-degenerate rule is at least two resolved package boundaries, or one non-root member plus a root manifest that is also a package when the standard permits root membership. For Cargo, `RootMembership::WhenManifestDeclaresPackage` means `[workspace]` plus `[package]` in the root `Cargo.toml`.
- Problem: `membership_resolves_non_degenerately` treats `RootMembership::WhenManifestDeclaresPackage` the same as `Always`. It has only `layer.packages.len()` and never checks whether the root manifest declares `[package]`.
- Impact: `[workspace]\nmembers = ["one"]` with no root `[package]` produces one layer package and is considered non-degenerate. That makes `is_monorepo` true for a virtual single-member Cargo workspace, contrary to the spec's "honest" predicate.
- Suggested fix: carry a `root_is_package` / `root_package_included` fact on `MonorepoLayer`, or include the root package in `layer.packages` only when it exists and let the predicate count actual package boundaries. Add a degenerate fixture for Cargo single-member virtual workspace and a positive fixture for root package + one member.
- Test rigor: current Level 1 tests assert the helper behavior with synthetic layers, but they encode the incorrect assumption. No fixture test exercises the `[workspace]` without root `[package]` single-member case.

### Medium: Lockfile corroboration treats supersets as matches and upgrades provenance incorrectly

- Code: `sniff/lib/src/filesystem/repo/detection.rs:456`, `sniff/lib/src/filesystem/repo/detection.rs:490`, `sniff/lib/src/filesystem/repo/detection.rs:496`, `sniff/lib/src/filesystem/repo/detection.rs:524`, `sniff/lib/src/filesystem/repo/detection.rs:530`
- Requirement: when lockfile and manifest disagree, the manifest remains the authority and the mismatch is recorded. A stale lockfile must not change package membership.
- Problem: `pnpm_lockfile_matches` and `uv_lockfile_matches` return true when every manifest member appears in the lockfile, even if the lockfile contains extra stale members. That is still disagreement, but the layer is upgraded to `PackageProvenance::Lockfile`.
- Impact: stale lockfiles can be reported as authoritative lockfile-derived membership, hiding drift from consumers that rely on `lockfile_match`.
- Suggested fix: compare normalized sets for equality. If exact equality is too strict for root importers such as `"."`, normalize and document that exception explicitly rather than using subset matching.
- Test rigor: Level 1 tests cover a missing manifest member in pnpm lockfile, but not extra stale importers or uv lockfile drift. Level 1 fixture tests are sufficient.

## Test Rigor Notes

This feature is primarily library detection plus CLI JSON serialization. It does not define terminal rendering, key input, paste, mouse, or other behavior that would require Level 2 or Level 3 verification. The appropriate floor for the findings above is Level 1, with CLI JSON integration tests where public command output is affected.

The current suite has meaningful Level 1 coverage for descriptors, serde ids, several positive/degenerate fixtures, same-root authority/orchestrator layering, and some CLI JSON shape. The gaps above are not a lack of terminal-level testing; they are missing Level 1 cases for the specified filesystem behavior and one implementation path that violates the no-execution contract.

## Production Readiness

Not ready for production. The implementation exposes the new public model, but it currently violates the no-subprocess detection boundary and does not implement the requested forest topology for most standards. Those should be fixed before treating the feature as production-ready.
