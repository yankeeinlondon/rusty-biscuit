---
status: draft
created: 2026-07-29
area: claudine
packages:
  - claudine
  - claudine-cli
depends_on:
  - PR #19 (docs/cross-platform-ci-plan) must merge first
---

# Make Windows Path Handling Work

## Summary

Path matching in `claudine` hardcodes `/` as the path-segment boundary. On
Windows, where paths use `\`, every directory-scoped comparison returns a
negative answer — and because each matcher is a `bool` with no error channel, a
separator mismatch is indistinguishable from a legitimate "this path is not under
that prefix".

For two of these checks the negative branch is the **permissive** one, so the
control silently turns off rather than failing:

| check | `false` means | Windows consequence |
|---|---|---|
| allow rule | not granted | fails closed — the feature does not work |
| **deny rule** | not denied | fails **open** — the denial never applies |
| **sensitive path** | not sensitive | fails **open** — `~/.ssh` is unprotected |

This was found by the first full cross-platform CI run (30427703024), not by a
report, because nothing in the product said anything was wrong.

## Interim state — already landed

`protect::path::warn_windows_path_matching_is_broken()` emits a one-time
`tracing::warn!` naming the affected paths and this spec. It is called from
`SensitivePathChecker::is_sensitive` and `permissions::matchers::path_matches`.

That is instrumentation, not a fix. It exists because a red CI cell never reaches
the user whose `~/.ssh` protection is off, and because `claudine`'s Windows leg
currently times out before reporting anything at all (see the build-time problem,
which is a separate issue). **Remove the warning as part of this fix.**

## The defects

### 1. `permissions/matchers.rs` — directory-scoped rules never match

Three branches test for a `/` byte at the boundary:

```rust
// pattern ends with "/*"
path_str.as_bytes().get(prefix.len()) == Some(&b'/')
// implicit prefix match
path_str.starts_with(pattern) && path_str.as_bytes().get(pattern.len()) == Some(&b'/')
```

A pattern is also matched with `strip_suffix("/*")` and `ends_with('/')`, so a
Windows-style *pattern* is not recognised either.

Surfaced by `permissions/providers/claude/tests.rs:182`.

### 2. `protect/path.rs` — the absolute-allow branch is unreachable

`is_path_allowed` detects absoluteness with `allowed.starts_with('/')`. A Windows
absolute path is `C:\…`, so the branch is never entered, and the boundary is then
built with a literal `/` after the value has already been canonicalized to native
separators.

Surfaced by `path.rs:339`.

### 3. `protect/path.rs` — home-relative sensitive paths are never classified

```rust
let full_prefix = format!("{home_str}/{prefix}");
```

With `home_str = C:\Users\ken` this yields `C:\Users\ken/.ssh`, compared against
a normalized `C:\Users\ken\.ssh`. Hardcoded `/` on both sides of the seam.

`~/.ssh`, `~/.aws`, `~/.gnupg`, and `~/.claude` are therefore not sensitive on
Windows. Surfaced by `path.rs:389`; `path.rs:373` and likely `:353` fail
identically.

### 4. `render/prompt/system.rs` — mixed separators in output

`:49-50` emits `./.claude\system-prompt.md` — neither correct POSIX nor correct
Windows. `:86` builds `file://C:\Users\…`, an invalid Windows file URL, so the
OSC8 hyperlink is broken. Sibling `tests.rs:177` fails identically.

## Design decision required

Two coherent approaches; pick one and apply it uniformly rather than patching
separators site by site.

**Normalize at the boundary.** Convert to one canonical separator on the way in
and compare in that space. This is the approach `biscuit-file` took with
`dunce::simplified`, so there is in-repo precedent and a consistent mental model.
Risk: a value that escapes normalization reintroduces the bug silently, which is
exactly how this arose.

**Compare separator-agnostically.** Replace boundary checks with a helper that
accepts either separator, e.g. `is_segment_boundary(byte)` matching `/` and — on
Windows only — `\`. More local, no canonical form to maintain, but every call
site must use the helper and nothing enforces that.

Whichever is chosen, the boundary logic MUST exist in exactly one place. Four
independent copies of `== Some(&b'/')` is what produced this.

## Required behavior

- A directory-scoped allow rule grants access to paths under that directory on
  every platform.
- A directory-scoped **deny** rule refuses access to paths under that directory
  on every platform. This is the security-relevant half.
- `~/.ssh`, `~/.aws`, `~/.gnupg`, `~/.claude` are classified sensitive on every
  platform.
- An absolute allow entry is recognised on Windows.
- Rendered paths and `file://` URLs use one separator, valid for the platform.
- A pattern written with either separator matches the equivalent path.

## Test plan

All L1. The matchers are string-based — `path_matches` takes `&Path` and
immediately does `to_string_lossy()`; `is_prefix_match` takes `&str` — so
**Windows-shaped inputs can be tested on any host**. Do not gate these behind
`#[cfg(windows)]`: a test that only runs on the Windows leg is invisible until
that leg's timeout problem is fixed, and the bug is in platform-independent
string logic anyway.

`SensitivePathChecker`'s `home_dir` field is private but its own in-module tests
can construct `Self { home_dir: Some(…) }` directly, so the home-relative case
needs no environment manipulation.

Required cases:

- a Windows-shaped path under a Windows-shaped directory pattern matches
- a Windows-shaped path under a POSIX-shaped pattern matches, and vice versa
- a directory-scoped **deny** rule applies to a child path, asserted separately
  from the allow case so a regression cannot hide in the shared helper
- `C:\Users\<user>\.ssh\id_rsa` is sensitive given a Windows home
- an absolute Windows allow entry is honoured
- a sibling directory whose name merely *prefixes* the pattern is NOT matched
  (`C:\proj2` must not match a rule for `C:\proj`) — the boundary check exists to
  prevent this, so removing it is not an acceptable fix
- rendered paths and `file://` URLs contain no mixed separators

Add a guard that the boundary logic has a single definition, so the four-copy
situation cannot recur.

## Out of scope

- The test-side POSIX assumptions catalogued separately (7 sites: verbatim
  `\\?\` prefixes, `pwd`/`echo` as spawnable binaries, rooted-but-prefix-less
  paths, `HOME` inertness under Windows' known-folder API). Those are test bugs,
  not product bugs, and are independent of this change.
- `claudine`'s build-time budget. Unrelated: it is CI resource accounting, and no
  product code is wrong. It only matters here because it currently prevents the
  Windows leg from reporting.

## Acceptance criteria

- [ ] Directory-scoped deny rules apply on Windows.
- [ ] Directory-scoped allow rules apply on Windows.
- [ ] Home-relative sensitive paths are classified on Windows.
- [ ] Absolute allow entries are recognised on Windows.
- [ ] Boundary logic has exactly one definition, enforced by a guard.
- [ ] Prefix-but-not-child paths are still rejected.
- [ ] No rendered path or URL mixes separators.
- [ ] Tests run on every platform, not gated to the Windows leg.
- [ ] `warn_windows_path_matching_is_broken` and both call sites are removed.
- [ ] `cargo xwin check --target x86_64-pc-windows-msvc -p claudine
      --all-targets` is clean (plain `cargo check` fails first on `aws-lc-sys`
      needing `windows.h`, which is a C build-script issue, not Rust).
