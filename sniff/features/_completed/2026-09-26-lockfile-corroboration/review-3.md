---
$schema: feature-review.yaml
ready: false
findings:
    - priority: high
      title: PHP-only Composer projects lose their standalone lockfile observation
    - priority: high
      title: A root-only uv workspace has no layer to report its lockfile match
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-27T09:20:35-07:00
spec: 2026-09-26-lockfile-corroboration/spec.md
implemented: true
implemented_by: claude/opus
log: sniff/features/2026-09-26-lockfile-corroboration/implementation-log.md
description: "A **feature** review of `2026-09-26-lockfile-corroboration/spec.md`"
feature: 2026-09-26-lockfile-corroboration/review-3.md
previous: 2026-09-26-lockfile-corroboration/review-2.md
next: 2026-09-26-lockfile-corroboration/review-4.md
---

# Review 3 — Lockfile corroboration

## Verdict

**Not ready for production.** The sole unblocked finding from review 2 is fixed, and that review had no blocked findings to revisit. Two other supported project shapes still lose the lockfile observation that the specification calls for. Both can be resolved in code and Level 1 tests without a new owner decision.

## Previous-review disposition

| Review 2 finding | Result |
|---|---|
| Malformed Node member manifests abort repository detection instead of yielding an incomplete observation | Resolved. The Sniff library's [nested workspace discovery](../../lib/src/filesystem/repo/nested.rs) skips a malformed `package.json` when it is merely a candidate nested workspace root. Its [lockfile comparison](../../lib/src/filesystem/repo/lockfile/mod.rs) then reports `unverifiable` with `incomplete_manifest_discovery` for the enclosing layer. Declared Level 1 tests require the complete result and unchanged provenance for npm, pnpm, Yarn, Bun, and Rush under both structure and full requests; a separate test keeps malformed actual workspace roots fatal. |

Review 2 recorded no blocked findings. No prior finding became unblocked between reviews.

## Unblocked Findings

### High — PHP-only Composer projects lose their standalone lockfile observation

The specification requires a repository-level `standalone_lockfiles` entry for a Composer lockfile at the requested project root, including a standalone PHP project. The Sniff library's [root-package fallback](../../lib/src/filesystem/repo/detection.rs) returns no repository result unless the root has a Cargo, Node, Python, or Go marker. It does not recognize `composer.json`. A normal PHP-only project with `composer.json` and `composer.lock` therefore returns `None` from [repository detection](../../lib/src/filesystem/repo/types.rs), before the Composer lockfile probe can populate `standalone_lockfiles`. The CLI has no repository result from which to report it.

The [Composer fixture test](../../lib/tests/l1/lockfile_fixtures.rs) adds an unrelated `package.json` before detection, so it proves a mixed PHP and Node project works while concealing the standalone case. Make the requested root observable when its Composer manifest is present, without inventing a workspace layer or treating dependency locks as membership proof. Add a Level 1 public-result test using the unmodified Composer fixture for enabled and disabled requests, and a shipped CLI JSON and plain-output test for that same PHP-only shape. Assert the observation, empty layer list, and successful output.

### High — A root-only uv workspace has no layer to report its lockfile match

The specification says a complete empty manifest member set may match a complete empty lockfile member set and explicitly calls for root-only uv cases. The real uv fixture has `[tool.uv.workspace]` with `members = []` and a valid root-only `uv.lock`. The Sniff library's [uv workspace detector](../../lib/src/filesystem/repo/uv.rs) treats the empty array like an absent workspace declaration and returns no layer. Consequently the public result contains no `lockfile` object for this declared workspace, even though the parser can establish the root-only member set. The [current fixture test](../../lib/tests/l1/lockfile_fixtures.rs) asserts that omission rather than the specified observation.

Distinguish an explicitly declared empty workspace from a missing workspace declaration. Let the declared root-only workspace reach the ordinary lockfile observation path while preserving the repository's existing rule for whether a one-package repository is called a monorepo. Replace the omission assertion with a Level 1 public-result assertion for `match`, `paths: ["uv.lock"]`, empty `extra` and `missing`, and the correct package provenance. Also assert its disabled-request result so an empty declaration does not bypass request costs.

## Verification level by requirement

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| Layer status, paths, differences, and package provenance across supported authorities | Level 1 complete-result and real-tool fixture tests | Appropriate for data behavior; the root-only uv fixture currently asserts the missing result, so its Level 1 requirement is unmet. |
| Standalone Poetry, PDM, and Composer observations | Level 1 library and shipped CLI tests | Appropriate for noninteractive results; Composer is tested only after adding a Node manifest, leaving PHP-only behavior unverified and broken. |
| Request-scoped work and lockfile precedence | Level 1 result and counter tests | Appropriate. |
| JSON stdout, plain output, and success exit behavior | Level 1 shipped CLI tests | Appropriate; the PHP-only project needs a CLI case after its result exists. |
| Styled lists, wrapping, and status colors | Level 2 capture of the shipped CLI inside tmux | Appropriate; the focused Level 2 test passed locally. |
| Keyboard, mouse, paste, and input encoding | No such feature requirement | Level 3 is unnecessary. |

## Review checks

I read the specification, both earlier reviews, the library's detection and lockfile paths, the fixture matrix, and the CLI test declarations. `just check-tier-coverage sniff` reported zero stranded tests. `just test lockfile_isolation::` passed all 26 selected Level 1 tests, including the repaired malformed-member cases. `just test-l2 level2_lockfile_rendering::` passed its one real-terminal test. Cross-OS evidence and human approval are outside this readiness decision, as requested.

## Human review

No human review is needed to resolve these findings. The specification already requires standalone Composer reporting and root-only uv verification.
