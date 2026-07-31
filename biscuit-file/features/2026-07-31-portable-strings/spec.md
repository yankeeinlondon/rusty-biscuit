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

### Where the verbatim paths actually come from

Not from biscuit-file. `df5cb5268` already reduces verbatim spellings at the
resolver's root boundary, so paths *returned by* `FileReference` are legacy-form
on Windows.

They come from darkmatter calling `std::fs::canonicalize` itself — five sites in
`link_normalization.rs` alone (`:76`, `:114`, `:138`, `:144`, `:149`), plus the
`schemas` and `compose` paths. Those results are handed straight to a renderer.
So the resolver being fixed does not make this redundant; it narrows the problem
to exactly the boundary this spec covers.

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
/// `.` and `..` are collapsed lexically, then a Windows verbatim (`\\?\`)
/// prefix is reduced via [`dunce::simplified`] — in that order, which is
/// load-bearing (see the spec's "Ordering constraint"). `dunce` declines to
/// reduce when the legacy spelling would not be equivalent (over-long paths,
/// reserved DOS device names, trailing dots or spaces), so such a path survives
/// intact rather than being silently corrupted. On every non-Windows target
/// `dunce::simplified` is a `const fn` returning false, making this provably
/// the identity apart from the separator pass.
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

## Ordering constraint

`dunce::simplified` **refuses** to reduce a verbatim path that still carries
components Win32 would not accept literally — because under a `\\?\` prefix
Win32 performs no path parsing, so `.`, `..`, and `/` are all literal filename
characters rather than syntax. `df5cb5268` discovered this twice and documented
it in both places:

- `normalize_components` (`resolve.rs:996`) reduces **after** `.`/`..` collapse,
  "because [`dunce::simplified`] refuses to touch a verbatim path that still
  carries relative components";
- `simplify_root` is applied to the root **before** any join, because "a
  component containing a slash is not a valid filename" — so simplifying an
  already-joined candidate declines.

The naive implementation is therefore wrong:

```rust
// WRONG: dunce declines on `\\?\C:\repo\.\docs`, the prefix survives,
// and the replace yields `//?/C:/repo/./docs`.
dunce::simplified(path).to_string_lossy().replace('\\', "/")
```

`to_portable_string` must collapse first, then reduce, then render — the same
order `normalize_components` already uses. A silent failure here is worse than
no function at all: it produces `//?/C:/…`, which is neither a path nor a URL,
from an input the caller had every reason to think was fine.

This is also why the accepted limitation is a *rendering* limitation only. The
prefix handling is exact, or it declines and says so by leaving the path
untouched.

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
Lift the collapse-then-reduce core out of `file_reference::resolve` into an
unfeatured module (decision 4); reduce `normalize_components` to a wrapper over
it. Add `to_portable_string` + rustdoc; make `dunce` unconditional; drop the
redundant dev-dependency; tests.

This step is **larger than a pure addition** — it edits a `pub(crate)` symbol
measured at **HIGH** risk. Deliberate: one implementation of the ordering rule
is the goal, and a second copy would defeat the spec.

Impact analysis, re-measured against the 2026-07-31 index (`epistemic: exact`),
correcting the MEDIUM/36 figures quoted from `df5cb5268`'s commit message:

| | Measured | `df5cb5268` claimed |
|---|---|---|
| Risk | **HIGH** | MEDIUM |
| Impacted | 25 (6 / 15 / 4 by depth) | 36 |
| Direct | 6 | 6 ✓ |
| Processes affected | 0 | — |

Direct callers, all in `biscuit-file/lib/src/file_reference/`: `validate`
(`context.rs`), plus `recursive_subdir_filter`, `dedupe_candidates`,
`normalize_absolute`, `diff_paths`, and `normalize_dotdot` (`resolve.rs`).

This **confirms `df5cb5268`'s reasoning**: the three consumers its message named
as depending on this symbol for lexical comparison — candidate dedupe,
repository containment, and `diff_paths`' common-prefix walk — are all present
as direct callers. The mechanism is real, only the severity was understated.

Exactly one consumer lives outside biscuit-file, as that commit claimed:
`resolve_document_file_ref_shape`
(`darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs`). Zero
execution flows are affected, which is what keeps HIGH tractable — the blast
radius is wide but shallow and wholly inside one module plus one known
consumer.

Verify biscuit-file's suite on both a Windows and a non-Windows host; the
existing behavior has never been executed on the former.

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
- **Ordering**, per [Ordering constraint](#ordering-constraint): a verbatim path
  carrying `.`/`..` (`\\?\C:\repo\.\docs`) reduces correctly rather than
  surviving as `//?/C:/repo/./docs`. This is the test that fails against the
  naive implementation, so it is the one that must exist.

**Verify the foundation while here.** `df5cb5268` closes with *"No Windows host
was available, so no Windows test was executed"* — it was validated by
`cargo check` and `clippy --target x86_64-pc-windows-msvc` only. Its
`simplify_root` / `normalize_components` behavior, which this spec builds
directly on, has never actually run on Windows. Step 1 is developed on a Windows
host, so it should execute biscuit-file's existing suite there and report the
result, rather than inheriting an unverified foundation.

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

1. **UNC policy — the one item needing sign-off.** `\\server\share\f.md`
   currently renders `//server/share/f.md`. In a Markdown destination a leading
   `//` is a *protocol-relative URL*, not a path — so today's behavior is wrong
   for the primary consumer. As a private darkmatter helper that was an internal
   quirk; as biscuit-file API it becomes a contract.

   The root difficulty: **there is no portable *path* text for a network
   location.** The correct portable form of `\\server\share\f.md` is the URL
   `file://server/share/f.md` — a UNC host is exactly a file URL's authority.
   But this function returns path text, not a URL; emitting a scheme would be a
   layer violation and would corrupt every non-URL consumer (YAML scalars,
   config values).

   - **(a) Keep the collapse** → `//server/share/f.md`. Rejected: silently
     produces a URL-shaped string with different meaning.
   - **(b) Emit `file://…`.** Rejected: a path renderer must not invent a
     scheme.
   - **(c) Decline — return the native spelling unchanged.** **Proposed.**

   (c) is not a cop-out; it reuses the rule already in the codebase. When
   `dunce::simplified` cannot faithfully reduce a verbatim path it leaves it
   intact rather than approximating. Applying the same rule to UNC gives the
   function one honest contract: *render portable text where a faithful portable
   form exists; otherwise return the input's native spelling.* Consumers that
   need a URL build one deliberately, with the information to do it correctly.

   Note this **changes darkmatter's current behavior** for UNC paths rather than
   grandfathering it, and the output can then contain `\` — so the doc comment
   must state that forward-slash output is guaranteed only where a portable form
   exists. Needs sign-off because it is a user-visible contract, not an
   implementation detail.
2. **Name.** `to_portable_string` proposed. Alternatives: `portable_display`,
   `to_portable_text`. Should not carry `markdown` — the function is
   domain-neutral and Markdown is one consumer.
3. **Module placement.** Crate root (`biscuit_file::to_portable_string`) versus
   a small `path` module. **Proposed:** crate root, matching the flat surface
   of the other top-level helpers.
4. ~~**Where the collapse-then-reduce core lives.**~~ **DECIDED 2026-07-31:
   lift the core into an unfeatured module**, with `normalize_components`
   reduced to a wrapper over it.

   `normalize_components` already implements exactly the required order, but it
   is `pub(crate)` inside `file_reference`, which is
   `#[cfg(feature = "file-reference")]` (`lib.rs:121`), while
   `to_portable_string` must work without that feature. The alternatives were
   duplicating the collapse (two copies of a subtle, load-bearing ordering rule
   — the exact failure mode this spec exists to remove) or gating
   `to_portable_string` behind `file-reference` (drags in gix, cargo_metadata,
   walkdir, dirs, and url for a path renderer). Both trade correctness for a
   smaller diff.

   Chosen with explicit direction that a wider rollout is acceptable when it
   buys the better decision. The cost is real, measured, and recorded in
   [step 1](#1-biscuit-file): **HIGH risk, 25 impacted, 6 direct, 0 execution
   flows** — worse than the MEDIUM this spec originally quoted from
   `df5cb5268`. The decision stands: the alternatives buy a smaller diff by
   duplicating a load-bearing ordering rule, and a second copy of that rule is
   the defect this spec exists to prevent. But it is now a HIGH-risk edit made
   knowingly rather than a MEDIUM one assumed.
