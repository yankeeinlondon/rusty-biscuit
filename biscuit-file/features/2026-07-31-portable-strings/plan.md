# Implementation plan — `to_portable_string`

Execution plan for [`spec.md`](./spec.md). Three sequentially landable commits,
each deleting the local implementation it replaces. Commits 2 and 3 depend on
the public API introduced by commit 1.

Every file, line number, symbol, and impact figure below was verified against
the working tree and the GitNexus index on 2026-07-31. Line numbers are given so
the implementer can confirm they are still standing before editing; if one has
moved, re-locate by symbol name rather than trusting the number.

## Verified starting state

| Fact | Evidence |
|------|----------|
| `dunce` is optional and feature-gated | `biscuit-file/lib/Cargo.toml:28` (`file-reference = [… "dep:dunce" …]`), `:78` |
| `dunce` is duplicated as a dev-dependency | `biscuit-file/lib/Cargo.toml:88-91`, with a comment naming the feature gate as the reason |
| `dunce` 1.0.5 has no transitive dependencies | `Cargo.lock:3565-3568` (no `dependencies` block) |
| Unfeatured modules are private `mod` + crate-root `pub use` | `biscuit-file/lib/src/lib.rs:96-107` |
| `file_reference` is feature-gated | `biscuit-file/lib/src/lib.rs:121-122` |
| A real consumer builds biscuit-file without default features | `darkmatter/dmls/Cargo.toml:59` (`default-features = false, features = ["file-reference"]`) |
| `path_to_markdown` strips `\\?\UNC\` and `\\?\` unconditionally | `darkmatter/lib/src/markdown/compose/util.rs:21-32` |
| It is re-exported for in-crate use | `darkmatter/lib/src/markdown/compose/mod.rs:146-148` |
| `dunce::simplified` reduces only a *safe* `VerbatimDisk` prefix and is `&Path -> &Path` | dunce 1.0.5 API; already relied on at `biscuit-file/lib/src/file_reference/resolve.rs:629` |

GitNexus impact, `direction: upstream`, 2026-07-31 index:

| Symbol | Risk | Impacted | Direct | Flows | Tests |
|--------|------|----------|--------|-------|-------|
| `path_to_markdown` | **HIGH** | 29 (16 / 13) | 16 | 0 | included |
| `make_relative_in_context` | **HIGH** | 38 (3 / 10 / 25) | 3 | 0 | included |
| `make_portable_relative_in_context` | MEDIUM | 38 (8 / 25 / 5) | 8 | 0 | included |
| `link_resolve` | **HIGH** | 15 | 15 | 0 | included |
| `normalize_links` | MEDIUM | 11 | 11 | 0 | included |
| `link_fn` | MEDIUM | 8 | 8 | 0 | included |
| `comparison_path` | LOW | 12 (1 / 11) | 1 | 0 | included |
| `escape_link_text` | LOW | 10 (1 / 1 / 8) | 1 | 0 | included |
| `complete_markdown_files_from` | LOW | 6 (3 / 2 / 1) | 3 | 0 | included |
| `display_path_with_forward_slashes` | LOW | 0 | 0 | 0 | included |

`normalize_components` is **not** edited by this plan. Its HIGH-risk figure is
recorded in the spec only to justify leaving it alone.

## Commit 1 — biscuit-file foundation

Additive. No existing symbol changes behavior, so nothing outside biscuit-file
needs re-verification.

### Files

1. **`biscuit-file/lib/Cargo.toml`**
   - `:28` — drop `"dep:dunce"` from the `file-reference` feature list.
   - `:78` — `dunce = { version = "1", optional = true }` → `dunce = "1"`.
     Keep the existing comment; extend it to name both boundaries.
   - `:88-91` — delete the `[dev-dependencies]` `dunce = "1"` entry **and** the
     two-line comment above it, which only explains the feature gate.

2. **`biscuit-file/lib/src/path_text.rs`** (new)

   ```rust
   //! The path→text boundary: rendering an OS path as portable text.

   use std::path::Path;

   /// Render a filesystem path as portable text, or `None` when no faithful
   /// portable spelling exists.
   ///
   /// Returns `None` exactly when [`to_portable_string`] would fall back to the
   /// native spelling: a Windows UNC, device-namespace, or verbatim path that
   /// [`dunce::simplified`] declined to reduce. Callers that must not emit a
   /// native spelling — a Markdown link destination, for instance — branch on
   /// this rather than inspecting the rendered string.
   pub fn try_portable_string(path: &Path) -> Option<String> {
       let reduced = dunce::simplified(path);

       #[cfg(windows)]
       {
           // A prefix surviving `simplified` is one `dunce` declined to reduce,
           // or a namespace with no faithful slash-separated form. Replacing
           // separators here would produce `//?/C:/CON` — neither a path nor a
           // URL — from a path the caller had every reason to think was fine.
           if has_non_disk_prefix(reduced) {
               return None;
           }
       }

       Some(reduced.to_string_lossy().replace('\\', "/"))
   }

   /// Render a filesystem path according to biscuit-file's portable-text policy.
   ///
   /// On Windows, a safely reducible verbatim-disk prefix is removed through
   /// [`dunce::simplified`]. Disk, rooted, and relative paths are then rendered
   /// with `/` separators. UNC, device-namespace, and verbatim paths that cannot
   /// be reduced faithfully retain their native spelling; use
   /// [`try_portable_string`] when that fallback is not acceptable.
   ///
   /// The result uses [`Path::to_string_lossy`]: non-Unicode data is replaced
   /// with U+FFFD. On Unix, `\` is a legal filename character and is rendered as
   /// `/`; see the feature spec's accepted limitations.
   pub fn to_portable_string(path: &Path) -> String {
       // `try_portable_string` returns `None` only for a prefix `simplified`
       // left untouched, so the fallback can render `path` itself.
       try_portable_string(path).unwrap_or_else(|| path.to_string_lossy().into_owned())
   }

   #[cfg(windows)]
   fn has_non_disk_prefix(path: &Path) -> bool {
       use std::path::{Component, Prefix};
       matches!(
           path.components().next(),
           Some(Component::Prefix(prefix)) if !matches!(prefix.kind(), Prefix::Disk(_))
       )
   }
   ```

   Keep the `Component`/`Prefix` imports inside the `cfg(windows)` helper —
   hoisting them to the module head makes them dead on Unix and trips
   `-D warnings`.

3. **`biscuit-file/lib/src/lib.rs`**
   - add `mod path_text;` to the block at `:96-100` (alphabetical: after
     `mod list_format;`);
   - add `pub use path_text::{to_portable_string, try_portable_string};`
     alongside the other unfeatured re-exports (`:125-134`), with a one-line
     section comment matching the surrounding style.

### Tests

`#[cfg(test)] mod tests` at the foot of `path_text.rs`, matching `detect.rs:173`
and `format.rs:135`.

Platform-independent:

- `docs\file.md` → `docs/file.md`; `docs/file.md` unchanged.
- rooted `/repo/file.md` unchanged.
- the Unix literal-backslash limitation, pinned explicitly:
  `my\report.md` → `my/report.md` (`#[cfg(not(windows))]`).
- non-Unicode input equals `to_string_lossy` plus the separator policy.
  Construction is platform-specific — `OsStrExt::from_bytes(b"bad\xFF.md")` on
  Unix, `OsStringExt::from_wide` with an unpaired surrogate on Windows — so this
  is two `cfg`-gated tests, not one.

`#[cfg(windows)]`:

- `C:\repo\file.md` → `C:/repo/file.md`.
- `\\?\C:\repo\docs\file.md` → `C:/repo/docs/file.md` (the reduction that works).
- `\\?\C:\repo\.\docs` → **unchanged**. This is the test that proves the
  renderer respects `dunce`'s refusal instead of collapsing a literal `.` name.
- `\\?\C:\CON` (reserved name), `\\?\C:\repo\trailing.`, `\\?\C:\repo\trailing `
  (trailing dot / space), and a path exceeding `MAX_PATH` → unchanged, `\\?\`
  intact.
- `\\server\share\file.md` (legacy UNC), `\\?\UNC\server\share\file.md`
  (verbatim UNC), `\\.\COM1` (device namespace) → unchanged.

Agreement between the pair, asserted over the same input set — one test, a table
of `(input, Option<expected>)`:

- `try_portable_string(p) == Some(s)` ⇒ `to_portable_string(p) == s`;
- `try_portable_string(p) == None` ⇒ `to_portable_string(p) == p.to_string_lossy()`.

Nothing else may consult the decline; if the two can disagree, every consumer
branch built on the predicate is wrong.

Leave `normalize_components_reduces_verbatim_paths`
(`biscuit-file/lib/src/file_reference/resolve.rs:1318`) where it is. It is
resolver coverage, not renderer coverage.

### Documentation

- Rewrite `biscuit-file/docs/dependencies.md:14-20`. Both current claims become
  false: `dunce` is no longer "gated behind `file-reference`" and no longer
  "also a dev-dependency". State the two boundaries it now serves —
  `simplify_root` for lookup, `to_portable_string` for text.
- `biscuit-file/README.md` — document the function, the declined-prefix policy,
  and the lossy Unicode conversion.
- `.claude/skills/biscuit-file/SKILL.md` (and `references/` where the API
  surface is enumerated) — add it, so an agent reaching for path rendering finds
  this instead of writing another `.replace`.
- Root `docs/dependencies.md` does not mention `dunce`; confirm that is still
  true and leave it alone if so.

### Gates

From `biscuit-file/`:

```
just build
just lint
just test
just doctest
cargo test -p biscuit-file --no-default-features --lib path_text
```

The last command is the one that proves the API is genuinely unfeatured; no
`just` recipe expresses `--no-default-features`, so an exact package selector is
used instead. `darkmatter/dmls` depends on biscuit-file with
`default-features = false`, so this is a real build configuration, but it also
enables `file-reference`; the explicit command above is what proves the new API
is genuinely unfeatured.

Run the platform-sensitive tests on Windows and a non-Windows host.

**Done when:** all five commands pass on the applicable hosts, and
`detect_changes()` reports only the expected manifest, module/export,
`path_text`, test, and documentation changes, with no affected execution flow.

## Commit 2 — darkmatter production adoption

Run `impact` on each symbol immediately before editing it and confirm the
figures above still hold. `link_resolve`, `path_to_markdown`, and
`make_relative_in_context` are HIGH risk; warn before editing any of them if the
refreshed result remains HIGH or becomes CRITICAL.

### `path_to_markdown` — delete (HIGH risk, 16 direct callers)

Production call sites split by whether the value can carry a Windows prefix:

| Site | Replace with | Why |
|------|--------------|-----|
| `link_normalization.rs:23` (`comparison_path`) | private Windows comparison representation | Rendering and path identity have different contracts; safe roots and long verbatim descendants do not necessarily decline together. |
| `link_normalization.rs:160`, `:168`, `:200` | `to_portable_string` | Each arm is guarded by `comparable_abs.starts_with(repo/home/var)`, so `rel` is a prefix-free relative path and the render always portabilizes. A decline branch here would be dead code. |
| `link_resolve.rs:100` | `try_portable_string`, erroring on `None` | Writes a canonicalized **absolute** destination before documents move; retaining a relative authored target would break transclusion identity. |

At `link_resolve.rs:100`, `None` means: return
`MarkdownError::Transform(String)` naming the resolved path and explaining that
no faithful Markdown destination exists. Do not leave the original target in
place: `link_resolve` exists to make child-relative links absolute before
movement, so warning-and-continue would silently retarget a link after
transclusion. Reuse the existing error variant; this feature does not add a new
public error type.

Replace `comparison_path` with a private comparison representation whose Windows
contract is independent of rendering:

- ordinary disk and verbatim-disk prefixes for the same drive compare equal,
  even when `dunce` retains the descendant because it is over `MAX_PATH`;
- legacy and verbatim UNC prefixes compare equal if UNC normalization is
  supported by the comparison key;
- remaining components are preserved without lexical `.`/`..` collapse;
- device and unknown verbatim namespaces remain distinct;
- every `starts_with` and `strip_prefix` operand in `normalize_links` uses the
  same representation.

This may be a small private enum or a comparison-only `PathBuf` projection. It
must never be emitted as document text. Document the invariant at the helper;
it is not derivable from the types.

Then delete `compose/util.rs:17-32` and its entry in the `compose/mod.rs:146-148`
re-export list.

The remaining `path_to_markdown` callers are test expectations in the same two files
(`link_normalization.rs:305,322,363,387,433-435,487-488,530,568-571,625-628`
`to_portable_string` only where the expected value is rendered relative text.
Tests for an absolute declined path must instead assert the stage-specific
branch: Finalization preserves and warns; Inline-Pre errors. Do not build both
the input and expectation through one renderer, because that can hide a broken
comparison key.

Add a Windows normalization regression with a short, safely reducible repository
root and an over-`MAX_PATH` verbatim descendant. The descendant must still pass
the repo prefix test and become relative portable text. Add the corresponding
legacy/verbatim UNC comparison case; the key must support that equivalence.

After the repo/home/environment branches, if no replacement was selected and
`try_portable_string(&abs_path)` is `None`, leave the authored destination
byte-identical and add the compose-report warning. Run this check only after the
anchored branches so a declined absolute spelling that can become a safe
relative replacement is still normalized.

### `make_portable_relative_in_context` — delegate (MEDIUM risk, 8 direct)

`path_projection.rs:40-87` currently returns a `String` from four arms, so the
prefix policy has nowhere to apply. Refactor `make_relative_in_context` to
return the selected value as a `&Path`-bearing result (a small private enum, or
`(PathBuf, bool /* home-anchored */)`), then:

- `make_relative_in_context` renders it with `to_string_lossy` exactly as today;
- `make_portable_relative_in_context` renders it with `to_portable_string`,
  keeping the `~/` prefix arm.

Do **not** convert the finished anchored `String` back into a `Path` to call the
helper — the prefix decision must be made on the real path, not on re-parsed
text. Update the module docs at `:1-21` and the item docs at `:25-34` and
`:69-75`, which describe the current String-valued contract.

This function is deleted by `2026-06-13-resolve-tuple`, so the refactor is
temporary. It is still worth doing: eight direct callers route through it today
and would otherwise keep the defect until that feature lands.

### `link()` destinations

`compose/expression/functions/mod.rs:2124` and `:2140` —
`path.to_string_lossy().replace('\\', "/")` → `try_portable_string(&path)`, both
branching. `path` comes from `resolve_path_arg`, so it is absolute and can be
verbatim or UNC on Windows. On `None`, return `ExpressionError` naming the path.
Do not emit the raw argument and do not add a warning sink to `ResolutionContext`:
the raw value may be a magic or source-relative reference, and the expression
function currently has no compose-report warning channel.

The generated `desc` at `:2119` keeps
`make_portable_relative_in_context`, including its native fallback. Update the
link-label escaping path so literal backslashes survive CommonMark parsing;
`escape_link_text` currently escapes only `[` and `]`, but backslash escapes are
active in link text too. Prefer a whitespace-preserving inline escape helper
over the provider helper that deliberately collapses whitespace. Parse focused
labels through `pulldown_cmark` and assert exact visible text, including leading
`\\` and backslashes before punctuation.

### `display_path_with_forward_slashes` — delete (LOW risk, 0 callers)

Defined at `functions/mod.rs:401` behind `#[allow(dead_code)]`. Its only uses
are a test expectation at `:4467` and its own test at `:4938-4954`. Point
`:4467` at `to_portable_string`, delete the function, delete
`display_path_with_forward_slashes_normalizes_separators`, and let the generic
separator coverage added in commit 1 carry that case.

### Gates

From `darkmatter/`: `just build`, `just lint`, `just test`, `just doctest`.
Add the stage-specific Windows cases from the spec:

- safe canonicalized disk paths continue through resolution and normalization;
- `normalize_links` preserves a declined authored absolute destination and
  records a warning;
- `link_resolve` returns an error for a declined resolved destination, including
  a transclusion-oriented regression that would fail if a child-relative link
  were left behind;
- both local `link()` destination arms return `ExpressionError` on decline;
- the mixed safe-root/long-verbatim-descendant comparison normalizes correctly;
- native-fallback link labels round-trip through `pulldown_cmark`.

Run the suite on Windows **and** a non-Windows host; the prefix policy change is
invisible on Unix. Check `darkmatter/docs/` for a `path_to_markdown` reference
before deleting (none exists as of 2026-07-31).

**Done when:** both platforms are green, no Unix snapshot moved, and
`detect_changes()` shows only the path-rendering symbols and compose/link flows.

## Commit 3 — darkmatter CLI and test utilities

- `cli/src/args/completion.rs:31` `complete_markdown_files_from` — the candidate
  is built at `:92-99` from `path`/`path.strip_prefix(base_dir)`. Call
  `try_portable_string` on the selected `&Path` and retain whether it succeeded;
  on `None`, call `to_portable_string` for the specified native fallback rather
  than duplicating that policy locally. The `./` trim at `:101` applies only to
  portable output. At `:105`, append `/` to portable directories and
  `std::path::MAIN_SEPARATOR` to native-fallback directories. Do not produce a
  mixed `\\server\share\dir/` candidate.
- `cli/src/args/completion.rs:173-175` — delete the test-only `normalize_path`
  helper and assert candidate values directly. Post-hoc normalization in the
  test is what hides the production behavior above.
- Remaining test-only conversions → `to_portable_string`, passing the original
  `Path` rather than a pre-rendered `String`:
  - `path_projection.rs:77` (`make_portable_relative`, `#[cfg(test)]`);
  - `compose/preflight/mod.rs:453`, `:494` and
    `compose/preflight/lifecycle.rs:307` (shell-command fixture paths);
  - `dmls/src/workspace/discover.rs:140`.
- Leave `schemas/resolve.rs:317` (`is_bare_name`) alone. Its input is
  `file_ref.raw()` — reference grammar in a `String`, not a path — and routing
  it through this API would misstate the domain.

### Gates

From `darkmatter/`: `just build`, `just lint`, `just test`. CLI completion tests
must pass without the deleted normalizer on both platforms. Add a Windows case
for a declined absolute UNC directory and assert its spelling and suffix remain
native.

## Verification scope

Affected package areas: `biscuit-file` (lib, cli) and `darkmatter` (lib, cli,
dmls). Commit 1 is purely additive to biscuit-file's public surface, so no other
consumer of biscuit-file (`sniff`, `claudine`, …) needs re-verification; commits
2 and 3 are confined to darkmatter. Do not substitute a workspace-wide build for
this scope.

## Standing constraints

- Never run `cargo fmt` / `rustfmt` write-mode. Match surrounding style by hand.
- Every behavior change above carries a docs pass in the same commit — module
  `//!`, item `///`, README, dependency docs.
- Run `impact` before editing each named symbol and `detect_changes()` before
  each commit.

## Watch items

1. **The `\\?\C:\repo\.\docs` test is the load-bearing one.** It fails against
   the naive `dunce::simplified(path).to_string_lossy().replace(…)`. If it ever
   starts passing for the wrong reason — because someone reintroduced a lexical
   collapse — the spec's central decision has been silently reversed.
2. **UNC output changes.** A Markdown destination that used to be rewritten to
   `//server/share/f.md` now follows a stage-specific policy: Finalization leaves
   authored text unchanged and warns, while Inline-Pre resolution and `link()`
   return errors rather than emit ambiguous Markdown. Non-Markdown consumers see
   `\\server\share\f.md`. Intended; see the spec's Prefix policy and Declined
   paths at the Markdown boundary.
3. **`--no-default-features` is a real configuration**, not a formality:
   `darkmatter/dmls` uses it but also enables `file-reference`, so DMLS alone
   cannot catch accidental placement inside that feature. Commit 1's explicit
   `cargo test -p biscuit-file --no-default-features --lib path_text` gate is the
   proof.
4. **Comparison is not rendering.** A short repository root can simplify while
   its long verbatim descendant cannot. If either operand goes through
   `to_portable_string`, valid repo/home/env normalization can silently stop.
