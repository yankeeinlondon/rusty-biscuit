# `to_portable_string` — one correct path→text rendering for every consumer

## Summary

Add `biscuit_file::to_portable_string(&Path) -> String`: render a filesystem
path as portable, forward-slash-separated text, reducing a Windows verbatim
(`\\?\`) prefix through `dunce::simplified` first. Make `dunce` a
non-optional dependency so the helper is available without the heavyweight
`file-reference` feature.

Then delete darkmatter's eight divergent hand-rolled copies of this operation —
**seven of which do not handle verbatim prefixes at all** — and point them at
the shared function.

This is deliberately *not* a move to `Path::components()`. See
[Non-goals](#non-goals) and [Accepted limitation](#accepted-limitation).

## Motivation

Rendering a path into text is a domain boundary: `Path` on one side, a
Markdown link destination / URL / YAML scalar on the other. Something must
cross it. Darkmatter currently crosses it in eight places, and they disagree:

| Site | Reduces `\\?\`? |
|------|-----------------|
| `darkmatter/lib/src/markdown/compose/util.rs:31` (`path_to_markdown`) | yes — hand-rolled |
| `darkmatter/lib/src/markdown/compose/expression/path_projection.rs:86` | **no** |
| `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:402` | **no** |
| `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:2124` | **no** |
| `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:2140` | **no** |
| `darkmatter/lib/src/markdown/schemas/resolve.rs:317` | **no** |
| `darkmatter/dmls/src/workspace/discover.rs:140` | **no** |
| `darkmatter/cli/src/args/completion.rs:174` | **no** |

Two defects follow from this.

**The one implementation that reduces prefixes does so unsafely.**
`path_to_markdown` calls `strip_prefix(r"\\?\")` unconditionally. Win32 treats
`\\?\C:\x` and `C:\x` as different paths whenever the legacy spelling is not
equivalent — paths over `MAX_PATH` (260), reserved DOS device names (`CON`,
`NUL`, `COM1`…), and components with trailing dots or spaces. In those cases
stripping corrupts the path instead of portabilizing it. `dunce::simplified`
exists to make exactly this judgment, and biscuit-file **already relies on it**
for the lookup-side equivalent (`file_reference/resolve.rs:629`, `simplify_root`)
with the reasoning documented there.

**The other seven mangle verbatim paths outright.** `\\?\C:\x` becomes
`//?/C:/x` — neither a valid path nor a valid URL. Reachability depends on
whether the caller happens to have simplified first; that is safety by luck in
a caller, not by construction.

Consolidating also gives the operation a home next to its sibling. biscuit-file
already owns Windows path-spelling reduction for **lookup** (`simplify_root`);
this is the same reduction for **display**. One crate should own the fact that
Windows spells paths several ways.

Finally, the name is wrong today: nothing in `path_to_markdown` is
Markdown-specific. Markdown is one consumer among many.

## Non-goals

- **No `Path::components()` rewrite.** Structural rendering would fix the Unix
  backslash-in-filename case by construction, but costs an explicit policy for
  five Windows `Prefix` variants plus `RootDir` handling, to fix an input that
  does not occur. It also would not remove `dunce` — components tells you a
  prefix *is* verbatim, not whether reducing it is *safe*. Rejected as
  speculative; see [Accepted limitation](#accepted-limitation) for the triggers
  that would reopen it.
- **No change to anchoring policy.** Which short form a path is expressed as
  (repo-relative, `~`, `${VAR}`) is owned by
  [`2026-06-13-resolve-tuple`](../2026-06-13-resolve-tuple/spec.md). This spec
  governs only how the chosen path is rendered as text.
- **No sniff or claudine changes.** Their display sites are resolve-tuple's
  adoption scope, not this one.
- **No new feature flag.** `dunce` becomes unconditional rather than gaining a
  `portable-path` feature.

## Public API

```rust
/// Render a path as portable, forward-slash-separated text.
///
/// A Windows verbatim (`\\?\`) prefix is reduced first, via
/// [`dunce::simplified`], which declines to reduce when the legacy spelling
/// would not be equivalent (over-long paths, reserved DOS device names,
/// trailing dots or spaces) — so such a path survives intact rather than being
/// silently corrupted. On every non-Windows target this is a plain separator
/// pass with no prefix work.
///
/// ## Notes
///
/// A `\` is a legal filename character on Unix, and this function cannot
/// distinguish one from a separator: `my\report.md` renders as `my/report.md`.
/// See the spec's "Accepted limitation" for why this is accepted and what
/// would change it.
pub fn to_portable_string(path: &Path) -> String;
```

Additive only. No existing signature or behavior changes.

## Manifest changes

`dunce` moves from optional to unconditional in `biscuit-file/lib/Cargo.toml`:

- remove `dep:dunce` from the `file-reference` feature list;
- change `dunce = { version = "1", optional = true }` to `dunce = "1"`;
- **delete the `[dev-dependencies]` `dunce = "1"` entry** — it exists solely
  because the real entry is feature-gated, and its comment says so. Making the
  dependency unconditional removes the reason for the duplicate.

`dunce` has zero transitive dependencies and compiles to a passthrough off
Windows, so this costs nothing in practice. Gating a five-line helper behind
`file-reference` would drag in `gix`, `cargo_metadata`, `walkdir`, `dirs`, and
`url` — defeating the reuse this spec exists to enable.

## Accepted limitation

This solution is **final, not a waypoint.** It has one known-wrong input class,
accepted deliberately:

> A Unix filename containing a literal `\` renders with that character turned
> into a separator. `my\report.md` → `my/report.md`.

Accepted because such names are vanishingly rare, and the structural fix costs
an explicit five-variant prefix policy to address an input that does not occur
in this codebase's domain.

The limitation is pinned by test, not by comment alone (see
[Testing](#testing)), so changing it is a deliberate act rather than a silent
regression.

**Triggers that would reopen the components decision** — record these at the
function definition, and treat any one of them as sufficient cause:

1. Link destinations gain backslash escaping (`\(`, `\ `). An escape and a
   separator would then be indistinguishable at this layer.
2. UNC paths must render as something other than `//server/share`.
3. Destinations must round-trip back to `Path`, making the lossiness
   bidirectional rather than terminal.

No TODO, no tracking issue, no "phase 1" framing. The triggers are the only
forward-looking language.

## Relationship to `2026-06-13-resolve-tuple`

That spec is **pending and unstarted** — verified 2026-07-31: no
`resolve_tuple`, `abbreviate`, `AbbreviationPolicy`, or `EnvOffset` exists in
`biscuit-file/lib/src`, and every site it schedules for deletion is still
present.

The two are **orthogonal layers**, and this one is the foundation:

- resolve-tuple decides *which anchor* an absolute path is expressed against.
- this spec decides *how the resulting path becomes text*.

`resolve-tuple` does not currently address separator portability or Windows
path spelling anywhere in its text. If it lands first, it will centralize seven
divergent abbreviation copies into biscuit-file and **still emit OS-native
separators**, at which point every consumer re-adds its own
`.replace('\\', "/")` — regenerating, one layer up, the exact duplication it
exists to eliminate.

**Amendment required there:** `FileReference::abbreviate` must render its
spelling through `to_portable_string`, and its round-trip property
(`relative.resolve()? == Some(abs)`) must be asserted on Windows, where the
portable spelling differs from the native one.

**Overlap on adoption.** resolve-tuple deletes `make_relative_in_context`
entirely. This spec's rewiring of `make_portable_relative_in_context` is
therefore transient — two lines, worth doing now because that function carries
the verbatim defect today and `relative()` calls it on every compose. Every
other site here is outside resolve-tuple's scope and survives unchanged.

## Adoption & removal

Each step deletes the local implementation in the same change that adopts the
shared one. Three commits, independently landable:

### 1. biscuit-file
Add `to_portable_string` + rustdoc; make `dunce` unconditional; drop the
redundant dev-dependency; tests. No consumer changes.

### 2. darkmatter — core
- **Delete** `compose/util.rs::path_to_markdown` and its `compose/mod.rs`
  re-export.
- **Adopt** `to_portable_string` in `link_resolve.rs:100` and
  `link_normalization.rs` (`comparison_path`, and the three replacement arms at
  `:160`, `:168`, `:200`).
- **Rewire** `make_portable_relative_in_context` to delegate rather than
  `.replace` — closes the verbatim gap for `relative()` and the eight sibling
  call sites that route through `path_display_components`.

### 3. darkmatter — remaining sites
Fold in `functions/mod.rs:402/2124/2140`, `schemas/resolve.rs:317`,
`dmls/workspace/discover.rs:140`, `cli/args/completion.rs:174`.

Two of these need review before mechanical replacement, because their input is
already a `String` rather than a `&Path`: `schemas/resolve.rs:317` operates on
`file_ref.raw()` (a *reference spelling*, not a path — it may not belong in
this consolidation at all), and `cli/args/completion.rs:174` takes a
pre-rendered string.

## Testing

In biscuit-file, against `to_portable_string`:

- separator conversion on a plain relative and absolute path;
- **the accepted limitation, asserted explicitly**: a path whose filename
  contains `\` renders with it converted. This is the pin that keeps the
  limitation a decision rather than a latent bug. Relocate darkmatter's
  existing `portable_relative_normalizes_separators_to_forward_slash`
  (`path_projection.rs:151`) here, which already documents this case.
- `#[cfg(windows)]`: a `\\?\C:\…` path reduces to `C:/…`;
- `#[cfg(windows)]`: a path `dunce` **declines** to reduce (reserved device
  name or `MAX_PATH`-exceeding) survives with its prefix intact rather than
  being corrupted — this is the defect the current hand-rolled strip has;
- `#[cfg(not(windows))]`: no-op for any path without a backslash.

In darkmatter, existing compose and link tests must pass unchanged on both
platforms; no snapshot should move, since on Unix the new function is
byte-identical to the current `.replace` for all non-backslash inputs.

## Documentation impact

These are **not** done in the spec commit — they describe what the code will
be, so they land with the change that makes them true. Enumerated here so the
obligation survives.

**With step 1 (biscuit-file):**

- `biscuit-file/docs/dependencies.md:14-20` — the current entry says `dunce`
  "is gated behind `file-reference`" and "is also a dev-dependency because the
  integration tests must build expectations in the same spelling." **Both
  become false.** Rewrite for an unconditional dependency, and extend the
  rationale: `dunce` now serves two boundaries, the resolver's root boundary
  (lookup, `simplify_root`) and the path→text boundary (display,
  `to_portable_string`).
- `biscuit-file/README.md` — document the new public function.
- `.claude/skills/biscuit-file/SKILL.md` (and `references/` where the API
  surface is enumerated) — add `to_portable_string`, including the accepted
  limitation, so an agent reaching for path rendering finds the one correct
  function rather than writing a ninth `.replace`.
- Root `docs/dependencies.md` — verify whether it tracks per-crate feature
  gating; it does not currently mention `dunce`, so it may need no change.

**With step 2 (darkmatter):** check `darkmatter/docs/` for any reference to
`path_to_markdown` before deleting it.

## Open decisions

1. **UNC policy.** `\\server\share\f.md` currently renders `//server/share/f.md`,
   which in a Markdown destination reads as a protocol-relative URL rather than
   a path. As a private darkmatter helper that was an internal quirk; as shared
   biscuit-file API it becomes a contract. Decide before publishing: commit to
   the collapse and document it, or leave UNC explicitly unspecified.
   **Proposed:** keep the collapse (it matches today's behavior, so adoption
   changes nothing) and document it as intentional.
2. **Name.** `to_portable_string` proposed. Alternatives: `portable_display`,
   `to_portable_text`. Should not carry `markdown` — the function is
   domain-neutral and Markdown is one consumer.
3. **Module placement.** Crate root (`biscuit_file::to_portable_string`) versus
   a small `path` module. **Proposed:** crate root, matching the flat surface
   of the other top-level helpers.
