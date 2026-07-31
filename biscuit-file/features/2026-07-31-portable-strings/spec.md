# `to_portable_string` — one path→text boundary for every consumer

## Summary

Add `biscuit_file::to_portable_string(&Path) -> String`: render a filesystem
path as portable, forward-slash-separated text when the Windows spelling can be
reduced faithfully. A safe Windows verbatim-disk (`\\?\C:\...`) prefix is reduced
through `dunce::simplified` before separator conversion. If `dunce` declines,
or the path is UNC/device namespaced, the function returns the native spelling
instead of manufacturing a URL-shaped or semantically different string.

Make `dunce` unconditional so the helper is available without the heavyweight
`file-reference` feature. Then remove Darkmatter's production renderers and
test-only path conversions in favor of the shared function.

This spec does **not** reuse or modify `file_reference::resolve::normalize_components`.
That function applies lexical normalization for resolver comparisons; a public
renderer must not collapse `.` or `..` in a verbatim path, where those components
are literal names. Keeping the two policies separate removes the previously
measured HIGH-risk Biscuit refactor and, more importantly, preserves path
meaning.

## Motivation

Rendering a `Path` into text is a domain boundary: the source is an OS path and
the result may become a Markdown destination, YAML scalar, completion value, or
diagnostic. Darkmatter currently crosses that boundary with several local
implementations, and most blindly call `.replace('\\', "/")`.

The production inventory is:

| Site | Role | Verbatim-safe today? |
|------|------|----------------------|
| `darkmatter/lib/src/markdown/compose/util.rs:21` (`path_to_markdown`) | shared compose renderer | **no** — strips `\\?\` / `\\?\UNC\` unconditionally |
| `darkmatter/lib/src/markdown/compose/expression/path_projection.rs:81` (`make_portable_relative_in_context`) | persisted relative projection | **no** |
| `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:2124` | one-argument `link()` destination | **no** |
| `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:2140` | two-argument local `link()` destination | **no** |
| `darkmatter/cli/src/args/completion.rs:31` (`complete_markdown_files_from`) | completion candidate path | emits OS-native separators |

The original inventory mixed these production renderers with test utilities.
The following are test-only conversions and should use the shared helper so
tests exercise the real contract rather than maintain parallel expectations:

- `path_projection.rs:77` (`make_portable_relative`, `#[cfg(test)]`);
- `functions/mod.rs:401` (`display_path_with_forward_slashes`, called only by
  unit tests and otherwise allowed as dead code);
- `compose/preflight/mod.rs:453,494` and
  `compose/preflight/lifecycle.rs:307` (shell-command fixture paths);
- `dmls/src/workspace/discover.rs:140` (test result normalization);
- `cli/src/args/completion.rs:174` (test-only `normalize_path`, which masks the
  production completion behavior above).

One superficially similar expression is deliberately **not** a path renderer:
`darkmatter/lib/src/markdown/schemas/resolve.rs:317` replaces separators in
`file_ref.raw()` to classify a reference spelling as a bare name. Its input is
reference grammar in a `String`, not a `Path`; routing it through this API would
misstate the domain and can change grammar semantics. It remains local.

### The current prefix stripping is unsafe

`path_to_markdown` removes `\\?\` without checking whether the legacy spelling
is equivalent. The prefix exists precisely because Win32 parsing can differ:
legacy paths are subject to reserved DOS names, trailing-dot/space handling,
and length restrictions, while a verbatim path is sent to the filesystem with
minimal parsing. Removing the prefix can therefore change what the path names
or make it unusable.

`dunce::simplified` owns this judgment. It reduces a `VerbatimDisk` prefix only
when the legacy spelling is safe, and otherwise returns the input unchanged.
Biscuit-file already uses it at the resolver's root boundary
(`file_reference/resolve.rs:629`, `simplify_root`). The new helper applies the
same safety decision at the text boundary without importing the resolver's
lexical-normalization policy.

### Where verbatim paths come from

On Windows, `std::fs::canonicalize` commonly returns a verbatim spelling.
Darkmatter calls it directly in link resolution and normalization, including
the five production sites in `link_normalization.rs:76,114,138,144,149`, then
passes the resulting paths to renderers. Biscuit-file's resolver reduces
safe-to-reduce roots, but that does not cover paths canonicalized independently
by consumers, and genuinely verbatim-only paths may correctly survive the
resolver unchanged.

Finally, `path_to_markdown` is misnamed: the operation is not Markdown-specific.
Markdown is one consumer among several.

## Public API

```rust
/// Render a filesystem path according to biscuit-file's portable-text policy.
///
/// On Windows, a safely reducible verbatim-disk prefix is removed through
/// [`dunce::simplified`]. Disk, rooted, and relative paths are then rendered
/// with `/` separators. UNC, device-namespace, and verbatim paths that cannot
/// be reduced faithfully retain their native spelling.
///
/// The result uses [`Path::to_string_lossy`]: non-Unicode data is replaced with
/// U+FFFD. On Unix, `\` is a legal filename character and is rendered as `/`;
/// see the feature spec's accepted limitations.
pub fn to_portable_string(path: &Path) -> String;
```

Additive only. No existing public signature changes.

## Rendering contract

The algorithm is intentionally small:

1. Call `dunce::simplified(path)` on every platform. It is an identity on
   non-Windows targets.
2. On Windows, inspect only the first `Path::components()` item to classify the
   remaining prefix:
   - no prefix or `Prefix::Disk(_)`: render lossily and replace `\\` with `/`;
   - `Prefix::UNC`, `Prefix::Verbatim`, `Prefix::VerbatimDisk`,
     `Prefix::VerbatimUNC`, or `Prefix::DeviceNS`: decline and return the lossy
     native spelling unchanged.
3. On non-Windows targets, render lossily and replace `\\` with `/`.

After step 1, a `VerbatimDisk` prefix means `dunce` deliberately declined to
reduce it. Step 2 must respect that answer. An unconditional replacement after
`dunce::simplified` would recreate the original defect:

```rust
// WRONG: when dunce declines, `\\?\C:\CON` becomes `//?/C:/CON`.
dunce::simplified(path).to_string_lossy().replace('\\', "/")
```

The implementation uses `Path::components()` only for Windows prefix
classification, not structural rendering. Rust exposes
[six Windows prefix variants](https://doc.rust-lang.org/std/path/enum.Prefix.html)
and documents that verbatim prefixes treat `/` as a non-separator and perform
essentially no normalization. Microsoft likewise documents that
[`\\?\` requests minimal modification](https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation):
`/`, `.`, and `..` must not be reinterpreted.

## No lexical normalization

The prior draft required `.`/`..` collapse before calling `dunce` and proposed
extracting `normalize_components` into an unfeatured module. That was incorrect
for a general renderer.

Under `\\?\`, `.` and `..` can be literal filesystem names. For example,
`\\?\C:\repo\.\docs` does **not** mean the same thing as `C:\repo\docs` merely
because the components look relative. `dunce` declines that input for exactly
this reason. Collapsing first would bypass its safety check and silently change
the path.

`normalize_components` remains correct for its narrower, documented role:
lexical comparison inside file-reference resolution. GitNexus measured changing
it as HIGH risk (25 impacted excluding tests: 6 direct, 15 at depth 2, 4 at
depth 3; 37 with tests included; 0 execution flows either way). The six direct
callers are `validate`, `recursive_subdir_filter`,
`dedupe_candidates`, `normalize_absolute`, `diff_paths`, and `normalize_dotdot`.
This feature leaves that symbol and all of those callers untouched.

## Prefix policy

There is no universally portable path text for every Windows namespace:

- A normal or safely simplified disk path has a faithful slash-separated form,
  such as `C:/repo/file.md`.
- `\\server\share\file.md` rendered as `//server/share/file.md` is URL-shaped;
  in a Markdown destination it can be interpreted as a protocol-relative URL.
- `file://server/share/file.md` is a file URL, not path text. Inventing that
  scheme would be a layer violation for non-URL consumers.
- A declined verbatim or device path needs its native prefix and separators to
  preserve meaning.

Therefore the decided policy is: **render portable text only when a faithful
path spelling exists; otherwise preserve the native spelling.** Callers that
need a URL must build one explicitly with URL-aware code.

This changes darkmatter's UNC behavior rather than grandfathering it.
`path_to_markdown` currently maps `\\?\UNC\server\share\f.md` to
`//server/share/f.md`; after adoption a UNC destination keeps its native
`\\server\share\f.md` spelling. The change is deliberate: today's output is a
protocol-relative URL, not the path it claims to be.

## Non-goals

- **No structural component rendering.** Components are inspected only to
  classify the Windows prefix. The function does not rebuild paths component by
  component.
- **No URL conversion or escaping.** The helper does not add `file://`, percent
  encoding, or Markdown destination escaping.
- **No anchoring policy.** Repo-relative, `~`, and `${VAR}` selection belongs to
  [`2026-06-13-resolve-tuple`](../2026-06-13-resolve-tuple/spec.md).
- **No sniff or claudine adoption in this feature.** Their display sites belong
  to resolve-tuple's abbreviation rollout.
- **No new feature flag.** `dunce` becomes unconditional rather than gaining a
  `portable-path` feature.
- **No changes to file-reference lexical normalization.** In particular,
  `normalize_components` and `simplify_root` remain in place.

## Manifest changes

In `biscuit-file/lib/Cargo.toml`:

- remove `dep:dunce` from the `file-reference` feature list;
- change `dunce = { version = "1", optional = true }` to `dunce = "1"`;
- delete the `[dev-dependencies]` `dunce = "1"` entry and its feature-gating
  comment, because the unconditional dependency is already available to tests.

`dunce` 1.0.5 has no transitive dependencies and is a no-op off Windows. Making
it unconditional still adds a small direct compile dependency to minimal
biscuit-file builds; it is not literally free, but it avoids enabling
`file-reference` and its `gix`, `cargo_metadata`, `walkdir`, `dirs`, and `url`
stack for a small path renderer.

## Accepted limitations

The API returns text, so two lossy cases are explicit and tested:

1. A Unix filename containing a literal `\` renders that character as `/`:
   `my\report.md` → `my/report.md`. This is existing Darkmatter behavior and is
   accepted because such names are outside the document-path domain.
2. `Path::to_string_lossy` replaces non-Unicode path data with U+FFFD. Every
   implementation being consolidated already uses the same conversion, but the
   public API must state it rather than imply byte-for-byte round-tripping.

Declined Windows prefixes are not a third lossy case: their prefix and separator
structure is preserved in native form (subject only to the Unicode conversion
above).

Any one of these requirements is sufficient to reopen structural rendering or
the return type:

1. link destinations gain backslash escaping where a literal backslash must be
   distinguished from a separator;
2. UNC paths need a path-independent URL representation;
3. rendered text must round-trip byte-for-byte to `Path`;
4. non-Unicode filenames enter the supported document-path domain.

## Relationship to `2026-06-13-resolve-tuple`

That feature is still pending and unstarted as of 2026-07-31: no
`resolve_tuple`, `abbreviate`, `AbbreviationPolicy`, or `EnvOffset` exists in
`biscuit-file/lib/src`, and its scheduled deletion sites remain.

The two features are orthogonal:

- resolve-tuple chooses *which anchor* expresses an absolute path;
- this feature chooses *how a Path-valued spelling becomes text*.

The resolve-tuple spec already says `FileReference::abbreviate` must call
`to_portable_string`; that amendment is complete. Its Windows test requirement
still overpromises that every spelling contains `/` and no `\\?\` prefix. It
must be narrowed to the successful-portabilization cases and add a declined
verbatim/UNC case that preserves native spelling and still resolves correctly
where the reference grammar supports it.

Resolve-tuple deletes `make_portable_relative_in_context`, so the delegation in
this feature is temporary. It is still worth doing because eight direct callers
currently route through it and otherwise retain the defect until resolve-tuple
lands.

## Adoption and removal

Each step deletes the local implementation in the same change that adopts the
shared function. The commits remain independently landable.

### 1. biscuit-file foundation

- Add an unfeatured path-text module and re-export
  `to_portable_string` at the crate root.
- Make `dunce` unconditional and remove the redundant dev-dependency.
- Add unit tests and public documentation.
- Do **not** move, wrap, or otherwise edit `normalize_components` or
  `simplify_root`.

Because the function is new and has no callers until later steps, this is an
additive LOW-risk foundation. It must be tested with
`--no-default-features` to prove the API is genuinely unfeatured.

### 2. darkmatter production

- Delete `compose/util.rs::path_to_markdown` and its `compose/mod.rs` re-export;
  call `biscuit_file::to_portable_string` directly from `link_resolve.rs` and
  `link_normalization.rs`.
- Refactor `path_projection` around a Path-valued projection result so raw and
  portable wrappers render the selected `&Path` directly. Do not convert the
  final anchored `String` back into a `Path` merely to call the shared helper.
  `make_portable_relative_in_context` then delegates separator/prefix policy to
  `to_portable_string` while preserving its `~/` prefix arm.
- Use `to_portable_string` for both local-path destination arms in `link_fn`.
- Delete `display_path_with_forward_slashes`; use the shared helper in its one
  test expectation and move its generic separator coverage to biscuit-file.

GitNexus impact, measured on the 2026-07-31 code index with tests included:

| Symbol | Risk | Impacted | Direct | Processes |
|--------|------|----------|--------|-----------|
| `path_to_markdown` | **HIGH** | 29 (16 / 13 by depth) | 16 | 0 |
| `make_portable_relative_in_context` | MEDIUM | 38 (8 / 25 / 5) | 8 | 0 |
| `display_path_with_forward_slashes` | LOW | 0 | 0 | 0 |

The HIGH warning is material even though many direct callers are tests:
production compose reaches the helper through `comparison_path`,
`normalize_links`, and `link_resolve`. Review every direct call-site replacement
and run the unchanged compose/link suites on both Windows and a non-Windows
host.

### 3. darkmatter CLI and test utilities

- Render `complete_markdown_files_from` candidates from their `Path` values with
  `to_portable_string` before adding a trailing `/` for directories. GitNexus
  measures this symbol as LOW risk: 6 impacted (3 / 2 / 1 by depth), 3 direct,
  0 execution flows, tests included.
- Delete the CLI test's `normalize_path` helper and assert the candidate values
  directly; post-hoc normalization would continue hiding production drift.
- Replace the remaining test-only conversions listed in
  [Motivation](#motivation) — `path_projection.rs:77`, the three preflight
  shell fixtures, and `discover.rs:140` — with `to_portable_string`, passing the
  original `Path` rather than a pre-rendered `String`.
- Leave `schemas/resolve.rs::is_bare_name` unchanged for the domain reason
  documented above.

## Testing

### biscuit-file

Test `to_portable_string` directly:

- plain relative, rooted, and absolute disk paths use `/` separators;
- the Unix literal-backslash behavior is pinned explicitly;
- non-Unicode input matches `Path::to_string_lossy` plus the documented
  separator policy;
- `#[cfg(windows)]` a safe `\\?\C:\...` path becomes `C:/...`;
- `#[cfg(windows)]` reserved-name, trailing-dot/space, and over-`MAX_PATH`
  verbatim-disk inputs remain native and keep their `\\?\` prefix;
- `#[cfg(windows)]` `\\?\C:\repo\.\docs` remains native — this proves the
  renderer respects `dunce`'s refusal instead of collapsing literal names;
- `#[cfg(windows)]` legacy UNC, verbatim UNC, and device-namespace inputs remain
  native;
- `#[cfg(not(windows))]` an input without backslashes is unchanged;
- the tests compile and run with `--no-default-features`.

The existing Windows `normalize_components_reduces_verbatim_paths` test remains
in place as resolver coverage; it is not renderer coverage and must not be
relocated.

### darkmatter

- Existing compose, link normalization, link resolution, expression, schema
  rewrite, DMLS discovery, and CLI completion tests pass unchanged on Windows
  and a non-Windows host.
- Add focused Windows integration cases at the public consumer boundary for a
  safe canonicalized disk path and a declined verbatim/UNC path.
- No existing Unix snapshot should move for ordinary Unicode paths without
  literal backslashes.

Before each implementation commit, run GitNexus impact on every edited symbol.
Before committing, run `detect_changes()` and verify that only the expected
path-rendering symbols and compose/link flows are affected.

## Documentation impact

These changes land with the code that makes them true:

**With step 1:**

- Rewrite `biscuit-file/docs/dependencies.md:14-20`: `dunce` is unconditional
  and serves both the resolver root boundary (`simplify_root`) and the path→text
  boundary (`to_portable_string`). Remove the dev-dependency claim.
- Document the public helper and its declined-prefix/lossy behavior in
  `biscuit-file/README.md`.
- Update `.claude/skills/biscuit-file/SKILL.md` and any API references so agents
  find the shared renderer rather than writing another `.replace`.
- Verify root `docs/dependencies.md`; it does not currently mention `dunce`, so
  no edit is expected unless that changes before implementation.

**With step 2:**

- No `path_to_markdown` reference exists under `darkmatter/docs/` as of
  2026-07-31. Recheck before deletion in case documentation lands meanwhile.

## Decisions

1. **UNC/device policy:** preserve native spelling. A path renderer does not
   invent a URL scheme or emit protocol-relative URL syntax.
2. **Name:** `to_portable_string`. The function is domain-neutral; its rustdoc
   makes the lossy Unicode conversion explicit.
3. **Placement:** re-export at `biscuit_file::to_portable_string`, matching the
   crate's flat top-level helper surface.
4. **Normalization:** no lexical collapse and no extraction from
   `file_reference::resolve`. `dunce`'s refusal is authoritative at the public
   rendering boundary.

There are no open decisions in this feature.
