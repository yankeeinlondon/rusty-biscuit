---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-31T20:41:06-07:00
spec: 2026-07-31-portable-strings/spec.md
implemented: true
description: "A follow-up **feature** review of `2026-07-31-portable-strings/spec.md`"
feature: 2026-07-31-portable-strings/review-2.md
previous: 2026-07-31-portable-strings/review-1.md
---

# Review 2

## Summary

The feature is **not production ready**. Review 1's lossy identity-key defect
is fixed, the comparison key no longer re-enters `PathBuf`, unsafe anchored
destinations are now audited, and the missing completion and transclusion
consumer tests have been added. The focused native-Windows tests pass.

The durable minimal-feature gate currently breaks the repository's WSL test
environment, however, because it invokes Cargo in a deliberately toolchain-free
archive guest. The normalization module also introduces a non-Windows
`-D warnings` failure. Finally, the namespace-removal validator measures
Unicode scalar values where Windows and `dunce` measure UTF-16 units, allowing
one class of overlong component that the corrected spec requires it to reject.

Per the user's direction, the separately changed file-transclusion error policy
and visible failure notice are treated as intentional and are excluded from
this review's findings.

## Findings

### High: the minimal-feature gate makes the WSL L1 job fail unconditionally

The Biscuit-file `test` recipe now invokes `test-minimal` after the ordinary
suite, and `test-minimal` runs plain `cargo test`
([justfile:65](../../justfile#L65), [justfile:79](../../justfile#L79)). That works
on the three native runners and passes locally, but the WSL job intentionally
installs neither Cargo nor rustc: it runs prebuilt nextest binaries from an
archive
([_wsl-ci.yml:233](../../../.github/workflows/_wsl-ci.yml#L233)). The guest still
calls the area's canonical `just test` with `--archive-file`
([_wsl-ci.yml:274](../../../.github/workflows/_wsl-ci.yml#L274)), so after the
archived suite succeeds the recipe reaches `cargo test` and fails with
`cargo: command not found`.

This affects Biscuit-file because every enabled area runs in `wsl2-ubuntu` by
default
([affected_scope.py:70](../../../scripts/ci/affected_scope.py#L70)). A dry run
confirmed that archive arguments are passed only to `_test_all`; the subsequent
`test-minimal` remains unconditional.

Keep the minimal-feature gate on native runners, but explicitly skip it in the
archive guest using the archive-mode contract (for example,
`BISCUIT_NEXTEST_BIN` or an explicit recipe input). Do not use a generic
"Cargo missing" skip, which would hide broken native provisioning. If WSL must
also prove the alternate feature resolution, build a separate no-default-
features archive on the Linux archive host and execute that in the guest.

### Medium: the new normalization import fails the non-Windows lint gate

`OsStr` is imported unconditionally
([link_normalization.rs:6](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L6)),
but every code use of that name is inside a `#[cfg(windows)]` function. On Linux
and macOS this produces `unused import: std::ffi::OsStr`. The shared lint recipe
runs Clippy with `-D warnings`
([devops.just:85](../../../just/devops.just#L85)), and the area CI invokes that
recipe on Ubuntu
([_area-ci.yml:383](../../../.github/workflows/_area-ci.yml#L383)), so the first
planned non-Windows validation will be red before exercising runtime behavior.

Gate the `OsStr` import with `#[cfg(windows)]`, or spell the Windows-only uses as
`std::ffi::OsStr`. A standalone `rustc -D warnings` probe confirmed that using
the import only in an intra-doc link does not satisfy the unused-import lint.

### Medium: overlong astral-Unicode components pass the namespace-removal audit

`survives_namespace_removal` rejects a component only when
`name.chars().count() > 255`
([link_normalization.rs:178](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L178)).
Windows component limits, and the `dunce` check this function says it mirrors,
measure UTF-16 code units. An astral character such as an emoji occupies one
Rust `char` but two UTF-16 units. A component containing 128 emoji therefore
reports 128 here but 256 to Windows and `dunce`.

For an anchored verbatim path, `try_portable_string` declines while this audit
accepts the suffix, allowing normalization to emit the legacy relative spelling
the spec requires it to preserve-and-warn about. Count
`component.encode_wide()` units on Windows rather than Unicode scalar values,
and add boundary tests at 255 and 256 UTF-16 units using both BMP and astral
characters.

### Low: anchored unsafe-category coverage is still incomplete

The corrected spec requires each unsafe category to be driven through
`normalize_links` on both repository and environment anchor arms
([spec.md:490](spec.md#L490)). The two end-to-end tests exercise only a literal
`.` component
([link_normalization.rs:1078](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L1078),
[link_normalization.rs:1155](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L1155)).
The helper-level table covers `.`, `..`, common DOS names, and trailing dot or
space, but not invalid Win32 characters, overlong components, or non-Unicode
components
([link_normalization.rs:1051](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L1051)).
Review 1's requested home-anchor regression also remains absent.

Add parameterized repository/environment consumer tests for every category and
at least one home-arm regression. These are ordinary Windows Level-1 tests. The
UTF-16 defect above demonstrates why helper-only examples are not enough to
verify the complete decision path.

## Review-1 Disposition

- **Partially resolved:** anchored normalization now uses a structured,
  non-lossy key and audits declined suffixes before replacement. The UTF-16
  length mismatch and incomplete anchored-category coverage remain.
- **Resolved:** distinct unpaired-surrogate paths retain distinct comparison
  identities.
- **Partially resolved:** a durable `--no-default-features` recipe exists and
  passes on native Windows, but its unconditional placement breaks WSL archive
  execution.
- **Resolved:** UNC comparison now exercises prefix stripping and relative
  computation; completion exercises the enumerating entry point; a full child
  transclusion regression and ordinary control are present. The intentional
  file-transclusion error-policy change is outside this review by user request.

## Verification

- `cargo test -p biscuit-file --no-default-features --lib path_text --color=never`
  — **10 passed**.
- `cargo test -p darkmatter --lib markdown::compose::link_normalization --color=never`
  — **18 passed**.
- `cargo test -p darkmatter --test declined_path_transclusion --color=never -j 1`
  — **3 passed** on the final reviewed tree.
- `cargo test -p darkmatter --lib markdown::compose::transclusion --color=never -j 1`
  — **55 passed**.
- `cargo test -p darkmatter-cli --lib args::completion --color=never -j 1`
  — **8 passed**.
- The first unscoped CLI attempt compiled unrelated integration targets and
  failed because the Windows paging file was exhausted. The scoped `--lib`
  retry above passed; this is recorded as a host-resource limitation, not a
  portable-strings failure.
- `git diff --check` — passed for tracked changes.

GitNexus was refreshed successfully. It reports `normalize_links` as HIGH risk
with 15 direct upstream callers and `complete_markdown_files_from` as LOW risk
with 7 affected symbols. Keyword query remained degraded because the local FTS
extension is unavailable; exact symbol context and focused source review were
used instead.

WSL, Linux, and macOS runtime suites were not executed on this Windows host.
Static review found the WSL recipe failure and non-Windows lint failure above;
no additional runtime-operation blocker specific to those systems was found.

## Production Readiness

Not ready for production. Fix the WSL archive-mode gate and non-Windows lint
failure before starting the cross-platform matrix, then correct and fully test
the UTF-16 component-length audit.
