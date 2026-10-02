---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-31T17:53:37-07:00
spec: 2026-07-31-portable-strings/spec.md
implemented: true
description: "A **feature** review of `2026-07-31-portable-strings/spec.md`"
feature: 2026-07-31-portable-strings/review-1.md
---

# Review 1

## Summary

The feature is **not production ready**. The shared public boundary, the
stage-specific error policy, link-label escaping, path projection, completion
separator policy, and documentation are substantially implemented. However,
the Windows comparison representation re-enters `std::path::PathBuf` before
prefix and relative-path operations. That lets Windows path parsing reinterpret
the literal components of exactly the verbatim paths that `dunce` declined,
and an applicable repository, home, or environment anchor then bypasses the
decline warning and emits a different destination.

No static production-code issue specific to WSL, Linux, or macOS was found.
Those hosts still need the planned test run; the non-Windows path is compiled
out on the Windows host used so far.

## Findings

### High: anchored normalization can silently change a declined verbatim path

`comparison_key` copies the text after the prefix but returns a `PathBuf`
([link_normalization.rs:47](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L47)).
The result is subsequently passed to `Path::starts_with`, `Path::strip_prefix`,
and `diff_paths`, which collects `Path::components`
([link_normalization.rs:205](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L205),
[link_normalization.rs:325](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L325)).
Those are path-semantic operations, not raw-component comparisons.

On native Windows, `Path::components` turns the comparison key
`C:\repo\.\docs` into `C:`, root, `repo`, `docs`; stripping `C:\repo` returns
`docs`. It treats `..` as `ParentDir`. In the original
`\\?\C:\repo\.\docs` or `\\?\C:\repo\..\docs`, however, those names are literal
verbatim components. The resulting relative replacement therefore names a
different location. The same unsafe-prefix removal occurs for descendants such
as `\\?\C:\repo\CON`, `trailing.`, and `trailing `: an anchor match emits the
reserved or trimmed legacy spelling even though `try_portable_string` declined
because removing `\\?\` is not faithful.

The decline check runs only after all anchor arms and only when no replacement
was selected
([link_normalization.rs:275](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L275)),
so these cases receive neither preservation nor a warning. The existing long-
descendant test deliberately exercises the one declined case that can safely
become relative
([link_normalization.rs:754](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L754)),
while the preservation test disables every anchor
([link_normalization.rs:798](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L798)).
Neither detects the unsafe anchored cases.

Keep comparison identity in a representation whose suffix components are not
parsed as an ordinary Windows path. Before emitting an anchored replacement,
separately prove that removing the namespace preserves every suffix component.
An over-`MAX_PATH` path with otherwise legacy-safe components should remain
eligible, as required by the spec; literal `.`/`..`, reserved DOS names, and
trailing-dot/space components should remain byte-identical and warn. Add
repository, home, and environment-anchor regressions for each unsafe category,
alongside the existing long-path success case.

### Medium: the comparison identity key is lossy

UNC server/share names, device/verbatim prefixes, and the full suffix are built
with `to_string_lossy`
([link_normalization.rs:53](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L53)).
Distinct Windows paths containing different unpaired UTF-16 code units can
therefore collapse to the same U+FFFD-containing key. A false `starts_with` or
`strip_prefix` match can then select the wrong repository, home, or environment
anchor and emit a destination for a different path.

Lossy output is an explicit contract of the public **renderer**, but this value
is an identity key that the spec says must preserve remaining components. Build
the private key from `OsString`/wide units (or a structured prefix plus native
suffix) without passing through `String`. Add a Windows test using two distinct
unpaired-surrogate component names and assert that their keys and anchor
relationships remain distinct.

### Low: the required no-default-features configuration has no durable gate

The new boundary is correctly unfeatured and `dunce` is unconditional
([Cargo.toml:14](../../lib/Cargo.toml#L14),
[Cargo.toml:78](../../lib/Cargo.toml#L78)), but the feature spec explicitly
requires its tests to compile and run with `--no-default-features`. The
biscuit-file CI area still supplies only the ordinary package arguments
([areas.json:65](../../../.github/ci/areas.json#L65)), and no repository test or
workflow command covering `biscuit-file --no-default-features` was found.

Add a persistent CI/Just gate such as
`cargo test -p biscuit-file --no-default-features path_text` (or the full
no-default-features package suite). A one-time local pass would validate the
current change but would not protect the public unfeatured API from regressing.

### Low: several required regressions stop below the consumer boundary

The direct unit tests are useful, but three specification scenarios are only
partially covered:

- `declined_resolved_destination_errors_before_transclusion` calls
  `link_resolve` directly rather than composing a child into a root, so pipeline
  ordering and error propagation through transclusion are not verified
  ([link_resolve.rs:674](../../../darkmatter/lib/src/markdown/compose/link_resolve.rs#L674)).
- The UNC completion test calls `candidate_value` directly, not
  `complete_markdown_files_from`, so selection/enumeration can still alter the
  path before the tested renderer is reached
  ([completion.rs:273](../../../darkmatter/cli/src/args/completion.rs#L273)).
- The legacy/verbatim UNC test checks key equality only; it does not exercise
  `starts_with`, `strip_prefix`, or an actual normalization
  ([link_normalization.rs:732](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L732)).

Add one compose/transclusion integration regression and test the completion and
UNC normalization consumers through injectable/local fixtures where direct SMB
enumeration would be slow or unreliable.

## Requirement Verification

| Requirement | Review result |
| --- | --- |
| Public `try_portable_string` / `to_portable_string` API and unconditional `dunce` | Implemented with direct ordinary, verbatim, UNC/device, non-Unicode, and renderer/predicate agreement tests. |
| Do not lexically normalize verbatim `.` / `..` | Implemented in the renderer, but violated by the normalization comparison/relative-path pipeline. |
| Finalization preserves and warns on an unanchored decline | Implemented; unsafe anchored declines bypass it. |
| Inline-Pre and `link()` reject declined destinations | Implemented for both link arms; full transclusion coverage is missing. |
| Path-valued projection and shared boundary adoption | Implemented. |
| Backslash-safe generated link labels | Implemented with parser round-trip coverage. |
| Completion uses consistent portable/native separators | Implemented at the helper; end-to-end UNC completion coverage is missing. |
| No-default-features support | Code has the required dependency shape; no durable verification gate was found. |
| WSL/Linux/macOS compatibility | No static blocker found; not executed as part of this Windows review. The documented Unix literal-backslash-to-slash limitation remains intentional. |

## Review Method and Test Status

- Reviewed the implementation commits from `f2d85e5f0` through `b285b5ea1`
  against the corrected spec at `74cd95e59`; the later background test-speed
  commits were treated as functionally out of scope.
- Used GitNexus context and change analysis, but the index was still at
  `74cd95e59`; an attempted refresh timed out, and the branch-wide comparison to
  `main` was dominated by unrelated branch history. Findings were therefore
  verified against the focused implementation diff and current source.
- Ran a small native-Windows `std::path` probe confirming that an ordinary
  `PathBuf` key drops the interior `.` component and returns `docs` when
  stripping `C:\repo` from `C:\repo\.\docs`.
- Did not start Cargo test suites because the user reported concurrent
  background test runs. Existing tests were inspected but are not reported as
  freshly passing here.

## Production Readiness

Not ready for production. Fix the anchored declined-path identity bug before
cross-platform validation. The lossy comparison key should also be corrected;
the remaining items are coverage gaps that can be closed alongside the planned
Windows, WSL, Linux, and macOS runs.
