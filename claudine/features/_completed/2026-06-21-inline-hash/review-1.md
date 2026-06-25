---
ready: false
agent: codex/default
created: 2026-06-24T22:16:01
---

# Review 1 - `inline-compose` Document Hashing

## Verdict

Not ready for production.

The implementation covers much of the library-level hashing contract, but the
actual CLI inline-compose path can leave a stale `hash:` whenever the existing
post-closure markdown cleanup changes the generated body. I was also unable to
run the focused verification because the workspace currently fails to compile in
`darkmatter`.

## Findings

### High - CLI cleanup can invalidate the just-stamped `hash:`

**Requirement:** every successful `inline-compose` closure writes a `hash:`
frontmatter property describing the final on-disk document, and `md hash --diff`
round-trips with no false positives.

**Evidence:** `apply_inline_closure` computes and writes the hash before returning
([closure.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/closure.rs:121)).
The CLI then immediately calls `cleanup_inline_output`
([inline.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/inline.rs:176)),
which rewrites the body if Darkmatter cleanup changes it
([inline_cleanup.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/composition/inline_cleanup.rs:25)).

That means dirty provider output such as `# Title\nParagraph\n` is hashed in its
pre-cleanup form, then persisted in its post-cleanup form as
`# Title\n\nParagraph\n`. The stored body segment no longer describes the final
file, so `md hash --diff` will report drift even though the inline run succeeded.

**Verification level:** current library tests are Level 1 and validate
`apply_inline_closure` in isolation. The CLI integration test is also Level 1,
but its provider returns already-clean body text, so it does not exercise the
cleanup mutation path. No Level 2/3 coverage is required for this file-mutation
requirement.

**Suggested fix:** move cleanup before hash stamping, or re-stamp after cleanup as
part of the final write. Add a Level 1 CLI/closure-path test where provider output
requires cleanup and assert `md hash --diff` exits 0 on the final file.

### High - Current workspace does not compile, blocking verification

**Requirement:** the feature must compile and work as part of the monorepo.

**Evidence:** a focused test command failed before running tests:

```text
cargo nextest run -p claudine-cli try_inline_closure_writes_cleaned_body_to_disk --color=never
```

The build failed in `darkmatter` with API mismatches around
`frontmatter_interpolation::interpolate_frontmatter` and
`interpolate_frontmatter_best_effort` now requiring an additional
`&HashSet<String>` argument, plus a `HashSet::contains` borrow error in
`frontmatter_interpolation.rs:350`.

**Verification level:** no feature tests could be executed in this worktree.

**Suggested fix:** restore compile health first, then rerun at least the focused
`claudine-cli` inline closure/hash tests and the `claudine` library closure tests
with nextest.

### Medium - Cross-platform CLI hash verification is Unix-only

**Requirement:** all packages in this monorepo must compile and work on macOS,
Windows, and Linux. The feature applies to direct `claudine inline-compose` runs,
not just the pure library closure.

**Evidence:** the only end-to-end CLI hash test is gated with `#[cfg(unix)]`
([inline_compose_hash.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/inline_compose_hash.rs:45))
and uses a shell-script fake provider. That leaves no Windows CLI-level
verification that inline-compose stamps a hash which `md hash --diff` accepts.

**Verification level:** library behavior has Level 1 unit coverage on all
platforms that compile, but the direct CLI path has Level 1 coverage only on
Unix. Level 2/3 is not required because this is not terminal-rendering or
keyboard-encoder behavior.

**Suggested fix:** add a Windows-compatible fake provider fixture for this test,
or factor the provider shim through the shared test helpers so the same Level 1
CLI assertion runs on Windows as well.

## Coverage Notes

The library-level expectations from the spec are mostly represented: Simple hash
stamping, self-reference stability, malformed-hash rejection without write,
body-unchanged detection via the Simple body segment, non-Simple normalization,
frontmatter-change signaling, and deterministic output all have Level 1 unit
coverage in `composition::closure`.

The missing piece is coverage of the final user-facing artifact after the full
CLI post-processing pipeline. That is the path users actually run, and it is
where the stale-hash bug is introduced.

## Verification Attempted

I attempted a focused nextest run for the CLI inline closure test, but compilation
failed in `darkmatter` before tests executed. No Level 1/L2/L3 tests were
successfully run in this review.
