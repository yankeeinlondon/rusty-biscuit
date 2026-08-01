# `to_portable_string` — one path→text boundary for every consumer

## Summary

Add `biscuit_file::to_portable_string(&Path) -> String`: render a filesystem
path as portable, forward-slash-separated text when the Windows spelling can be
reduced faithfully. A safe Windows verbatim-disk (`\\?\C:\...`) prefix is reduced
through `dunce::simplified` before separator conversion. If `dunce` declines,
or the path is UNC/device namespaced, the function returns the native spelling
instead of manufacturing a URL-shaped or semantically different string. A
sibling `try_portable_string(&Path) -> Option<String>` exposes that decline so a
consumer which must not emit a native spelling can act on it.

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
/// Render a filesystem path as portable text, or `None` when no faithful
/// portable spelling exists.
///
/// Returns `None` exactly when [`to_portable_string`] would fall back to the
/// native spelling: a Windows UNC, device-namespace, or verbatim path that
/// [`dunce::simplified`] declined to reduce. Callers that must not emit a
/// native spelling — a Markdown link destination, for instance — branch on
/// this rather than inspecting the rendered string.
pub fn try_portable_string(path: &Path) -> Option<String>;

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

`to_portable_string` is defined as `try_portable_string(path)` falling back to
the lossy native spelling, so the two cannot disagree about what "declined"
means. The predicate exists because the decline is a decision consumers act on;
recovering it by searching the rendered string for a `\` would re-derive the
policy at every call site.

Additive only. No existing public signature changes.

## Rendering contract

The algorithm is intentionally small:

1. Call `dunce::simplified(path)` on every platform. It is an identity on
   non-Windows targets.
2. On Windows, inspect only the first `Path::components()` item to classify the
   remaining prefix:
   - no prefix or `Prefix::Disk(_)`: render lossily and replace `\\` with `/`;
   - `Prefix::UNC`, `Prefix::Verbatim`, `Prefix::VerbatimDisk`,
     `Prefix::VerbatimUNC`, or `Prefix::DeviceNS`: decline —
     `try_portable_string` yields `None`, and `to_portable_string` returns the
     lossy native spelling unchanged.
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

### Declined paths at the Markdown boundary

A declined spelling is a valid path but not valid Markdown. CommonMark
processes backslash escapes inside a link destination, so a native Windows path
written there does not survive a parse:

```text
[f](\\server\share\f.md)   parses back as   \server\share\f.md
[f](\\?\C:\repo\f.md)      parses back as   \?\C:\repo\f.md
```

The leading `\\` collapses to a single `\`. Escaping at the Markdown boundary
would round-trip, but it puts a Markdown-specific escaping layer under a
domain-neutral renderer — the condition this spec's own reopening triggers name
as cause to revisit the return type.

**Decided policy: a decline is handled according to the consumer's pipeline
stage.** Treating every Markdown writer alike is incorrect because some writers
are producing final text while others are preserving link identity across a
later move:

- `normalize_links` runs during Finalization, after transclusion. On `None` it
  leaves the authored destination byte-identical and records a compose-report
  warning. There is no later source-relative move that can change its meaning.
- `link_resolve` runs during Inline-Pre specifically to make local links
  absolute before a document is moved or transcluded. On `None` it returns
  `MarkdownError::Transform` naming the path. Leaving a relative target untouched
  here would make it relative to the root document after transclusion and
  silently retarget the link.
- `link()` generates a destination from a resolved absolute path. On `None` it
  returns `ExpressionError`; it does not fall back to the raw argument. The
  expression function has no compose-report warning channel, and a raw magic or
  source-relative reference is not necessarily a valid replacement for the
  resolved path.
- Non-Markdown consumers — diagnostics, completions, YAML scalars — keep using
  `to_portable_string` and its native fallback, which is correct text for them.

This changes darkmatter's UNC behavior rather than grandfathering it, and the
change is not a regression. `path_to_markdown` currently maps
`\\?\UNC\server\share\f.md` to `//server/share/f.md`, which a Markdown reader
resolves as a protocol-relative URL against the page's scheme. After adoption,
Finalization preserves an already-authored destination and warns; a pre-move or
generated destination that cannot be represented fails explicitly rather than
silently changing meaning. Non-Markdown consumers see the native spelling.

Warning-and-continue in `link_resolve` or `link()` is deliberately out of scope.
Supporting it safely would require either syntax-specific Markdown and HTML
destination encoders with parser round-trip guarantees, or structured resolved-
link metadata that survives transclusion. Neither belongs in a domain-neutral
path renderer.

## Non-goals

- **No structural component rendering.** Components are inspected only to
  classify the Windows prefix. The function does not rebuild paths component by
  component. This does not prohibit a private Darkmatter comparison key from
  normalizing equivalent Windows prefix spellings for `starts_with` checks; that
  key is not rendered text.
- **No URL conversion or escaping.** The helper does not add `file://`, percent
  encoding, or Markdown destination escaping.
- **No anchoring policy.** Repo-relative, `~`, and `${VAR}` selection belongs to
  [`2026-06-13-resolve-tuple`](../2026-06-13-resolve-tuple/spec.md).
- **No sniff or claudine adoption in this feature.** Their display sites belong
  to resolve-tuple's abbreviation rollout. Biscuit-file's own `bf reference`
  is not in that group and *is* adopted here — see step 4.
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
shared function. The commits are sequentially landable: step 1 is the foundation,
and steps 2 and 3 depend on its public API.

### 1. biscuit-file foundation

- Add an unfeatured path-text module and re-export `try_portable_string` and
  `to_portable_string` at the crate root.
- Make `dunce` unconditional and remove the redundant dev-dependency.
- Add unit tests and public documentation.
- Do **not** move, wrap, or otherwise edit `normalize_components` or
  `simplify_root`.

Because both functions are new and have no callers until later steps, this is an
additive LOW-risk foundation. It must be tested with `--no-default-features` to
prove the API is genuinely unfeatured. That configuration is not hypothetical:
`darkmatter/dmls/Cargo.toml:59` already depends on biscuit-file with
`default-features = false`, so the configuration is real. DMLS also enables the
`file-reference` feature, however, so it does not prove that this API is
unfeatured; the explicit biscuit-file `--no-default-features` test does.

### 2. darkmatter production

- Delete `compose/util.rs::path_to_markdown` and its `compose/mod.rs` re-export.
  Use `try_portable_string` or `to_portable_string` only at rendering boundaries,
  according to [Declined paths at the Markdown boundary](#declined-paths-at-the-markdown-boundary).
- Replace `comparison_path` with a private Windows comparison representation,
  not rendered text. It must treat `C:\...` and `\\?\C:\...` as the same disk
  root even when the descendant is too long for `dunce` to simplify, and must
  similarly equate legacy and verbatim UNC spellings. It must preserve the
  remaining components without lexical `.`/`..` collapse and keep device or
  unknown verbatim namespaces distinct. Use this representation consistently
  on both sides of `starts_with` and `strip_prefix`.
- The representation must not be a `PathBuf`, and must not be built through
  `to_string_lossy`. Re-entering `std::path` hands the suffix back to Windows
  path parsing, which drops a literal `.` and reads a literal `..` as a parent
  hop; `to_string_lossy` maps two paths differing only in unpaired surrogates
  onto one key, which is enough for the wrong anchor to match. Split components
  out of the raw platform units and compare them directly.
- Every anchored replacement drops the namespace prefix, so before emitting one
  for a path `try_portable_string` declined, prove the removal component by
  component: reject a literal `.`, `..`, reserved DOS name, trailing dot or
  space, invalid Win32 character, name longer than 255 UTF-16 units, or
  non-Unicode name, and preserve-and-warn instead. The length measure is UTF-16
  units because that is what Windows and `dunce` count; Unicode scalar values
  under-count an astral name — 128 emoji is 128 `char`s but 256 units — and
  would let one through. Length alone must not disqualify — an
  over-`MAX_PATH` descendant of a short root is exactly the case that has to
  keep normalizing. Audit only the names taken from the destination; the `..`
  hops the relative-path computation generates are its own.
- In `normalize_links`, relative replacements produced after a successful
  prefix comparison use `to_portable_string`; those paths are prefix-free. If
  no repo, home, or environment anchor applies and `try_portable_string` on the
  absolute destination returns `None`, preserve the authored destination and
  add a compose-report warning. This check belongs after the anchored
  replacements so a long verbatim descendant that can become a safe relative
  path is still normalized rather than warned about.
- In `link_resolve`, branch on `try_portable_string`; on `None`, return a compose
  `MarkdownError::Transform` naming the path rather than leaving a source-relative
  destination in content that may subsequently be transcluded.
- Refactor `path_projection` around a Path-valued projection result so raw and
  portable wrappers render the selected `&Path` directly. Do not convert the
  final anchored `String` back into a `Path` merely to call the shared helper.
  `make_portable_relative_in_context` then delegates separator/prefix policy to
  `to_portable_string` while preserving its `~/` prefix arm.
- In both local-path destination arms of `link_fn`, use `try_portable_string` and
  return `ExpressionError` on `None`. Do not add warning state to
  `ResolutionContext` for this feature.
- Make link-label escaping preserve literal backslashes as well as brackets.
  The one-argument `link()` description can receive a native fallback, and
  CommonMark processes backslash escapes in link text too.
- Delete `display_path_with_forward_slashes`; use the shared helper in its one
  test expectation and move its generic separator coverage to biscuit-file.

GitNexus impact, measured on the 2026-07-31 code index with tests included:

| Symbol | Risk | Impacted | Direct | Processes |
|--------|------|----------|--------|-----------|
| `path_to_markdown` | **HIGH** | 29 (16 / 13 by depth) | 16 | 0 |
| `make_relative_in_context` | **HIGH** | 38 (3 / 10 / 25) | 3 | 0 |
| `make_portable_relative_in_context` | MEDIUM | 38 (8 / 25 / 5) | 8 | 0 |
| `link_resolve` | **HIGH** | 15 | 15 | 0 |
| `normalize_links` | MEDIUM | 11 | 11 | 0 |
| `link_fn` | MEDIUM | 8 | 8 | 0 |
| `comparison_path` | LOW | 12 (1 / 11 by depth) | 1 | 0 |
| `escape_link_text` | LOW | 10 (1 / 1 / 8) | 1 | 0 |
| `display_path_with_forward_slashes` | LOW | 0 | 0 | 0 |

All three HIGH warnings are material even though many direct callers are tests.
Review every direct call-site replacement, especially `link_resolve`'s
pre-transclusion invariant, and run the compose/link suites on both Windows and
a non-Windows host.

### 3. darkmatter CLI and test utilities

- Render `complete_markdown_files_from` candidates from their `Path` values with
  `try_portable_string`, retaining whether the result is portable. On decline,
  use `to_portable_string` for the specified native fallback. Append `/` to
  portable directories and `std::path::MAIN_SEPARATOR` to native-fallback
  directories so a completion never mixes spelling conventions. GitNexus
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

### 4. `bf reference`

`biscuit-file/cli/src/main.rs:453` prints a resolved path with `Path::display`,
the one path→text site in the CLI. It was originally left out of scope, which
was wrong twice over: it is the same defect in the crate that owns the fix, and
its own tests already assumed the corrected behavior — they assert
`biscuit-file/cli/Cargo.toml` with forward slashes and had been failing on
Windows since before this feature began.

- Print through `to_portable_string`, so a script capturing stdout gets one
  spelling on every host. GitNexus measures `run_reference` as LOW risk: 1
  impacted, 1 direct (`main`), 1 execution flow.
- Its four Windows-failing tests carry a second, independent defect:
  `starts_with("/")` as a stand-in for "absolute". No Windows path satisfies
  that, portable or not, because a portable disk path begins `C:/`. Replace it
  with `Path::is_absolute` on the captured stdout rather than a string
  predicate. Keeping the `/`-separated `ends_with` assertions is what pins the
  renderer; a substring predicate that tolerated both spellings would let the
  CLI regress to `display` unnoticed.
- Update `biscuit-file/cli/README.md` and the biscuit-file skill's CLI
  reference: the output spelling is user-visible behavior, and the skill should
  stop an agent from reintroducing the `starts_with("/")` assertion.

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
- `try_portable_string` returns `Some` for every case above that portabilizes
  and `None` for every case that stays native, with the `Some` payload equal to
  `to_portable_string`'s result — the two must not be able to disagree;
- the tests compile and run with `--no-default-features`. This needs a durable
  gate, not a one-time local pass: nothing in the default-feature suite would
  notice the unfeatured guarantee lapsing, because the module would still
  compile and its tests would still pass. `biscuit-file`'s `test-minimal`
  recipe owns the second feature resolution and runs from `just test`, which is
  what CI invokes per area. It must skip itself when `just test` is passed
  `--archive-file`: that leg executes prebuilt binaries in a WSL2 guest which
  deliberately installs neither Cargo nor rustc, and a second feature
  resolution has to compile. The signal is the archive flag, not a missing
  Cargo — the latter would also swallow a native runner whose toolchain failed
  to provision. The three native runners own this gate.

The existing Windows `normalize_components_reduces_verbatim_paths` test remains
in place as resolver coverage; it is not renderer coverage and must not be
relocated.

### darkmatter

- Existing compose, link normalization, link resolution, expression, schema
  rewrite, DMLS discovery, and CLI completion tests pass on Windows and a
  non-Windows host.
- Add a Finalization test in which a declined absolute destination remains
  byte-identical and `normalize_links` records the warning.
- Add an Inline-Pre/transclusion regression composing a child into a root, not
  a direct `link_resolve` call, so the stage's position in the order is what is
  under test. The transclusion engine applies its general child-failure policy
  to the `MarkdownError::Transform`: it records a `transclusion` warning and
  replaces the directive with a failure notice, so the root compose returns
  `Ok`. Assert that outcome — no child link, absolutized or authored, reaches
  the root, and the gap is visible in the artifact. Pair it with an
  ordinary-path control, or a broken transclusion would satisfy the assertion,
  and with a `fail_fast` case that does return the `MarkdownError::Transform`,
  which is what separates a deliberate downgrade from a discarded error.
  Making that downgrade opt-in rather than the default is
  [`darkmatter/fixes/2026-07-31-error-handling-transclusions`](../../../darkmatter/fixes/2026-07-31-error-handling-transclusions/spec.md).
- Add `link()` tests proving both local destination arms return
  `ExpressionError` on decline.
- Add Windows comparison tests where a safely reducible short repository root
  contains an over-`MAX_PATH` verbatim descendant. The descendant must still be
  recognized as inside the repository and normalize to a relative portable
  path. Cover legacy/verbatim UNC equivalence through `starts_with`,
  `strip_prefix`, and the relative computation, not key equality alone.
- Add anchored regressions for each unsafe category, driven through
  `normalize_links` rather than the comparison helper, on all three anchor arms
  — repository, home, and environment: the destination stays byte-identical and
  warns. Each arm needs its own, because the audit is only one step of the
  decision and a helper-level example cannot show that an arm reaches it with
  the right slice. Include the over-255-UTF-16-unit category here rather than
  only at the helper level; it is the one whose two plausible measures disagree.
  Pair each arm with the over-`MAX_PATH` success case, or the gate cannot be
  distinguished from a blanket refusal to anchor declined paths. A non-Unicode
  component stays helper-level: a destination reaches this stage as document
  text, and a Rust `str` cannot carry an unpaired surrogate, so no authored
  link can produce one.
- Add component-length boundary tests at 255 and 256 UTF-16 units in ASCII,
  multi-byte BMP, and astral spellings. The three measures — bytes, scalar
  values, UTF-16 units — agree on ASCII and diverge in opposite directions on
  the other two.
- Add a Windows test proving two keys differing only in an unpaired surrogate
  stay distinct, and that neither is a prefix of the other.
- Parse generated one-argument links with `pulldown_cmark` and assert that a
  native-fallback label, including leading `\\` and backslashes before
  punctuation, round-trips as the exact visible text.
- Add a Windows completion test for a declined absolute UNC directory and
  assert that its directory suffix remains native rather than producing mixed
  separators. Test the enumerating entry point as well as the renderer:
  selection sits between them and decides whether a candidate keeps its
  enumerated absolute spelling at all. Reaching it needs a *local* decline,
  since `read_dir` against a real share is slow and unreliable — a
  trailing-dot directory created through a `\\?\` path is the cheapest one, and
  it must be created and removed verbatim, because Win32 strips the dot from
  every component and would silently build an ordinary directory instead.
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
- Document both public helpers, the declined-prefix policy, and the lossy
  conversion in `biscuit-file/README.md`, including which of the two a consumer
  should reach for.
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
5. **Declined paths in Markdown:** policy is stage-specific. Finalization leaves
   authored text unchanged and warns; pre-transclusion resolution and `link()`
   return errors because retaining raw text there can change link identity.
   `try_portable_string` exists to make each branch explicit.
6. **Path comparison:** rendered text is never a path identity key. Darkmatter
   uses a private comparison representation to equate safe and retained Windows
   prefix spellings without weakening the public renderer's decline policy.
   That representation is neither a `Path` nor a `String`: both would undo the
   guarantee, one by re-parsing the suffix and one by collapsing distinct
   non-Unicode names onto a shared key.
7. **Anchoring a declined path:** the decline is a fact about the whole
   spelling, and an anchored replacement asks a narrower question, so the two
   are decided separately. An anchor arm may emit text only after proving that
   every name it copies out of the destination still means itself without the
   namespace prefix; otherwise Finalization preserves and warns exactly as it
   does for an unanchored decline.

There are no open decisions in this feature.
