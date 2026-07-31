# Implementation plan — `to_portable_string`

Execution plan for [`spec.md`](./spec.md). Three independently landable commits,
each deleting the local implementation it replaces.

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
| `make_portable_relative_in_context` | MEDIUM | 38 (8 / 25 / 5) | 8 | 0 | included |
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
just lint
just test
just doctest
cargo test -p biscuit-file --no-default-features --lib path_text
```

The last command is the one that proves the API is genuinely unfeatured; no
`just` recipe expresses `--no-default-features`, so an exact package selector is
used instead. `darkmatter/dmls` depends on biscuit-file with
`default-features = false`, so this is a real build configuration, not a
hypothetical one.

**Done when:** all four pass, and `detect_changes()` reports only the new
`path_text` symbols.

## Commit 2 — darkmatter production adoption

Run `impact` on each symbol immediately before editing it and confirm the
figures above still hold.

### `path_to_markdown` — delete (HIGH risk, 16 direct callers)

Production call sites split by whether the value can carry a Windows prefix:

| Site | Replace with | Why |
|------|--------------|-----|
| `link_normalization.rs:23` (`comparison_path`) | `to_portable_string` | Feeds `starts_with`, not document text. Both sides of every comparison must decline together. |
| `link_normalization.rs:160`, `:168`, `:200` | `to_portable_string` | Each arm is guarded by `comparable_abs.starts_with(repo/home/var)`, so `rel` is a prefix-free relative path and the render always portabilizes. A decline branch here would be dead code. |
| `link_resolve.rs:100` | `try_portable_string`, branching | Writes `abs_path_str` — a canonicalized **absolute** destination, which on Windows can be verbatim or UNC. |

At `link_resolve.rs:100`, `None` means: leave `record`'s original target text in
place, do not call `find_target_range`, and add a compose-report warning naming
the path. Assert the guarded-relative reasoning for the three
`link_normalization` arms in a comment at `comparison_path`, not at each arm —
it is one fact about the function's contract, and CLAUDE.md's criterion A
(invariant not derivable from the types) is what earns it the line.

Then delete `compose/util.rs:17-32` and its entry in the `compose/mod.rs:146-148`
re-export list.

The remaining direct callers are test expectations in the same two files
(`link_normalization.rs:305,322,363,387,433-435,487-488,530,568-571,625-628`
and `link_resolve.rs:212,247,287,312,334,359,361`). Point them at
`to_portable_string` as well — they exist to build an expected string in the
same spelling the production path emits, and that reason survives the rename.

Two correctness notes:

- `comparison_path` is applied to **both** sides of every `starts_with` /
  `strip_prefix` in `normalize_links` (`:139`, `:145`, `:148`, `:185`), so a
  declined path staying native does not break comparison — both sides decline
  together. Preserve that symmetry; do not simplify one side away. Record it as
  an inline comment at `comparison_path` in this commit.
- On Windows these tests canonicalize `TempDir` paths, which come back verbatim,
  and build both the input document and the expectation through the same helper.
  Expectations therefore do not move with path length. What a `MAX_PATH`-length
  temp root changes is the *input*: the document would carry a declined
  `\\?\C:\…` destination, which `normalize_links` then refuses to rewrite, so
  `link_normalizations_applied` drops to 0 and the test fails loudly. Correct
  behavior, confusing failure — if a Windows runner starts failing these
  together, check the temp root's length before suspecting the rendering.

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
verbatim or UNC on Windows. On `None`, emit the link with the raw argument text
the author supplied rather than a native spelling CommonMark would re-parse
incorrectly, and record the warning.

The `desc` argument at `:2119` keeps `make_portable_relative_in_context` and its
native fallback: link *text* is not a destination, and CommonMark applies no
escape processing that would corrupt it.

### `display_path_with_forward_slashes` — delete (LOW risk, 0 callers)

Defined at `functions/mod.rs:401` behind `#[allow(dead_code)]`. Its only uses
are a test expectation at `:4467` and its own test at `:4938-4954`. Point
`:4467` at `to_portable_string`, delete the function, delete
`display_path_with_forward_slashes_normalizes_separators`, and let the generic
separator coverage added in commit 1 carry that case.

### Gates

From `darkmatter/`: `just lint`, `just test`, `just doctest`. Add the two
Windows integration cases the spec asks for at the public consumer boundary — a
safely canonicalized disk path, and a declined verbatim/UNC path whose
destination must come out byte-identical to the authored text with the warning
present in the report. Asserting only "unchanged" would pass equally if the
normalizer never ran, so assert the warning too. Run the suite
on Windows **and** a non-Windows host; the UNC policy change is invisible on
Unix. Check `darkmatter/docs/` for a `path_to_markdown` reference before
deleting (none exists as of 2026-07-31).

**Done when:** both platforms are green, no Unix snapshot moved, and
`detect_changes()` shows only the path-rendering symbols and compose/link flows.

## Commit 3 — darkmatter CLI and test utilities

- `cli/src/args/completion.rs:31` `complete_markdown_files_from` — the candidate
  is built at `:92-99` from `path`/`path.strip_prefix(base_dir)`. Render both
  branches with `to_portable_string` instead of `to_string_lossy().to_string()`.
  The `./` trim at `:101` and the trailing-`/` append at `:105` then operate on
  portable text on every platform.
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

From `darkmatter/`: `just lint`, `just test`. CLI completion tests must pass
without the deleted normalizer on both platforms.

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
   `//server/share/f.md` is now left exactly as the author wrote it, with a
   compose-report warning; non-Markdown consumers see `\\server\share\f.md`.
   Intended; see the spec's Prefix policy and Declined paths at the Markdown
   boundary. Any downstream report of "links on a network share stopped being
   normalized" is this change, and the fix is a URL-aware caller, not a
   separator replacement here.
3. **`--no-default-features` is a real configuration**, not a formality:
   `darkmatter/dmls` uses it. If commit 1's helper accidentally lands inside a
   feature-gated module, dmls breaks at commit 3, not commit 1.
