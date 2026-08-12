# `FileReference::resolve_tuple` — resolved absolute path paired with a compact spelling

## Summary

Add a `resolve_tuple()` method to `FileReference` that returns **both** the
fully-qualified absolute path *and* a compact, re-resolvable spelling of it
(`(absolute, relative)`). `resolve()` and all existing methods are unchanged.

The "relative" half is whichever short anchored form best expresses the
absolute path against the roots `FileReference` already knows — package area,
git root, cwd/base dir, an environment variable, or `$HOME` (`~`). This
centralizes path-abbreviation logic that currently exists in **seven** divergent
copies across sniff, darkmatter, and claudine (enumerated below), then deletes
each one.

## Motivation

`FileReference` resolves a reference spelling → absolute path. The inverse
*display* problem — given an absolute path, produce a short anchored spelling —
has been re-implemented independently, and the copies disagree on policy:

| Site | Forms produced | Env policy | Precedence |
|------|----------------|-----------|------------|
| `darkmatter/.../compose/link_normalization.rs` `normalize_links` | repo-rel, `~`, `${VAR}` | opt-in whitelist (`PROJECT_ROOT`, `DOCS_BASE`) | home **before** env |
| `darkmatter/.../compose/expression/functions.rs` `make_relative` | repo-rel, base-rel, `~` | — | — |
| `darkmatter/.../compose/mod.rs` (`~/` strip) | `~` only | — | — |
| `sniff/cli/src/output/filesystem/mod.rs` `alias_path` | `${VAR}`, `~`, absolute | scan-all minus `PWD`/`OLDPWD` | env **before** home |
| `sniff/cli/src/commands/mod.rs` (`worktrees --verbose`) | `~` only | — | — |
| `claudine/lib/src/stream/path_link.rs` `format_file_link` | cwd-rel, `~` | — | — |
| `claudine/.../wrap/composition/mod.rs` `resolve_prompt_display_path` (+ `completion/.../compose.rs`) | repo-rel, cwd-rel, `~` | — | — |

`FileReference` is the correct owner: it already captures the exact context this
needs — `ResolutionContext { cwd, home_dir, env }` plus `find_git_root` /
`find_package_area` — and it already understands the reference grammar
(`~`, `$VAR`, `@magic`, relative, absolute) that the compact spelling is drawn
from.

## Non-goals

- **Rendering stays in consumers.** `resolve_tuple` returns paths only. OSC8
  hyperlinks, `<blue>` markup, truncation, and width-aware wrapping remain the
  caller's job (CLI presentation layer).
- **`resolve()` is not modified.** No signature, behavior, or precedence change
  to any existing method. This is purely additive.
- **No new resolution roots.** Abbreviation uses the same roots resolution
  already uses; it does not introduce new search semantics.

## Dependency: portable path rendering

This spec decides *which anchor* a path is expressed against. It does **not**
address how the resulting path is rendered as text — separator normalization
and Windows verbatim (`\\?\`) prefix reduction. That is owned by
[`2026-07-31-portable-strings`](../2026-07-31-portable-strings/spec.md), which
adds `biscuit_file::to_portable_string`.

The two compose, and the ordering matters:

- `abbreviate` **must** render its spelling through `to_portable_string`.
  Without it, this spec consolidates seven divergent abbreviation copies into
  biscuit-file and still emits OS-native separators — at which point every
  consumer re-adds its own `.replace('\\', "/")`, regenerating the exact
  duplication this spec exists to eliminate, one layer up.
- The [round-trip property](#round-trip-property) needs a Windows case: the
  portable spelling differs from the native one there, so `relative.resolve()`
  must accept the portable form.

Landing `portable-strings` first is the cheaper order — it is additive,
consumer-free, and has no open decisions. This spec then consumes it rather
than duplicating it.

## Public API

```rust
impl FileReference {
    /// Resolve to the absolute path AND a compact, anchored spelling of it.
    ///
    /// The first element is identical to what `resolve()` returns. The second
    /// is a `FileReference` whose `raw()` is the shortest re-resolvable spelling
    /// of that path under the configured abbreviation policy (see
    /// `with_abbreviation`) — e.g. `${PROJECT_ROOT}/some/file.md`, `~/notes.md`,
    /// or a repo-relative `docs/spec.md` — or the absolute path itself when no
    /// anchor applies.
    pub fn resolve_tuple(&self)
        -> Result<Option<(PathBuf, FileReference)>, FileReferenceError>;

    /// As `resolve_tuple`, resolving relative/`@`/`!` forms against `base`
    /// rather than the ambient CWD (mirrors `resolve_from`).
    pub fn resolve_tuple_from(&self, base: &Path)
        -> Result<Option<(PathBuf, FileReference)>, FileReferenceError>;

    /// Configure how the "relative" half of `resolve_tuple` is chosen.
    /// Stored on the builder alongside `magic_paths` / `vault_roots`.
    pub fn with_abbreviation(self, policy: AbbreviationPolicy) -> Self;

    /// Abbreviate an **already-absolute** path to a compact, re-resolvable
    /// spelling — *without resolving or touching the filesystem for existence*.
    ///
    /// This is the standalone core that `resolve_tuple` wraps. Use it directly
    /// when you already hold a validated absolute path (and possibly a
    /// directory, or a path that may not exist) and only want the label —
    /// `resolve()`'s `is_file()` gate must not apply. Infallible; performs I/O
    /// only when `policy` enables the repo/package/cwd rungs (root discovery).
    ///
    /// `home`/`env` are read from `context` when supplied, else from process
    /// state, so the operation is unit-testable without mutating globals.
    pub fn abbreviate(
        absolute: &Path,
        policy: &AbbreviationPolicy,
        context: Option<&AbbreviationContext>,
    ) -> FileReference;
}
```

`resolve_tuple` is defined in terms of the two halves:

```text
resolve_tuple()  ==  let abs = resolve()?;
                     Some((abs, FileReference::abbreviate(&abs, &policy, ctx)))
```

`Ok(None)` is returned in exactly the cases `resolve()` returns `None` (well-formed
reference, no existing file — note `resolve_direct` gates on `is_file()`, so a
**directory never resolves**). The absolute member always equals `resolve()`.
Consumers that hold a path which may be a directory or may not exist — such as
sniff's worktree root — must call `abbreviate` directly rather than
`resolve_tuple`, since the `is_file()` gate would otherwise yield `None`.

`AbbreviationContext` is the public, caller-suppliable subset of the internal
`ResolutionContext` (cwd, home_dir, env) needed by abbreviation; `None` means
"read from ambient process state."

### `AbbreviationPolicy`

```rust
#[derive(Debug, Clone)]
pub struct AbbreviationPolicy {
    /// Express relative to the enclosing Cargo package area when an ancestor.
    pub package_relative: bool,     // default: true
    /// Express relative to the enclosing git workdir when it is an ancestor.
    pub repo_relative: bool,        // default: true
    /// Express relative to the context cwd / base dir when it is an ancestor.
    /// (claudine's cwd form, darkmatter `make_relative`'s base_dir form.)
    pub cwd_relative: bool,         // default: false
    /// Environment-variable offsetting strategy.
    pub env: EnvOffset,             // default: EnvOffset::Off
    /// When both an env var and `$HOME` match, prefer the `${VAR}` form.
    pub prefer_env_over_home: bool, // default: false
    /// Emit `~/…` for paths under `$HOME`.
    pub home_tilde: bool,           // default: true
}

#[derive(Debug, Clone)]
pub enum EnvOffset {
    /// Never offset against environment variables.
    Off,
    /// Only consider these variable names (e.g. ["PROJECT_ROOT", "DOCS_BASE"]).
    Whitelist(Vec<String>),
    /// Consider every variable whose value is an absolute path, except `deny`.
    ScanAll { deny: Vec<String> },
}

impl Default for AbbreviationPolicy { /* fields above */ }
```

`Default` gives a safe, general policy: package/repo-relative on, cwd-relative
**off**, **no** env offsetting (avoids the `PWD`/`OLDPWD` surprise by default),
`~` for home.

## Abbreviation algorithm

`resolve_tuple` builds the `ResolutionContext` **once**, resolves the absolute
path through the existing `resolve::resolve` path, then derives the relative
half. Candidate forms are tried in this order; the first that applies wins:

1. **Package-area-relative** — when `package_relative` and a package area
   (`find_package_area`) is an ancestor → strip it → bare relative path.
2. **Repo-relative** — when `repo_relative` and the git workdir
   (`find_git_root`) is an ancestor → strip it → bare relative path.
3. **Cwd/base-relative** — when `cwd_relative` and `context.cwd` is an
   ancestor → strip it → bare relative path.
4. **Env-var offset** — per `env`: among candidate variables whose value is an
   **absolute path** and a **component-wise prefix** of the target, take the
   **longest** prefix; ties break on the lexicographically-first variable name.
   Emit `${VAR}/rest`, or bare `${VAR}` on exact match.
5. **Home** — when `home_tilde` and under `$HOME` → `~/rest` (or `~`).
6. **Absolute** — the path itself.

The relative rungs (1–3) are **ancestor-strip only** (descendant paths, no `../`
segments), keeping spellings clean and re-resolvable — distinct from
`resolve_relative`, which uses `diff_paths` and *can* emit `../`.
`prefer_env_over_home` swaps the relative ordering of steps 4 and 5. The strip
rungs are unaffected by it. "Longest prefix" is measured in path
**components**, not bytes, so a trailing slash in an env value cannot outrank a
real ancestor. Variables whose values are not absolute paths (`TERM`, `LANG`,
`SHLVL`, …) are skipped implicitly by the absolute-path test — only path-valued
positional vars (`PWD`, `OLDPWD`) need an explicit `deny` entry.

## Return type: `(PathBuf, FileReference)`

The relative member is **not** a `PathBuf`. A spelling like `${FOOBAR}/file.md`
is a *reference template containing unexpanded descriptive text*, not a
filesystem path — typing it as `PathBuf` would advertise path operations
(`exists`, `canonicalize`, `parent`) that are nonsense on a `${FOOBAR}`
component, and `PathBuf`'s `OsString` backing is the wrong domain for
human-facing text.

It is returned as a `FileReference`:

- `relative.raw()` yields the descriptive spelling verbatim
  (`${FOOBAR}/some/file.md`, `~/notes.md`, `docs/spec.md`).
- It is **re-resolvable by construction** — the round-trip property is
  type-level obvious because you hold a `FileReference` you can `.resolve()`.
- It can expose the matched anchor (e.g. the environment variable name) via its
  parsed `ReferenceKind`, which **darkmatter requires** for its
  "offset of the `<VAR>` environment variable" warning. A bare `String` would
  force a re-parse of `${…}`; a `PathBuf` could not carry it at all.

To support the env rung, parsing must round-trip `${VAR}/rest` into an
env-templated `ReferenceKind` (the parser already handles env-var segments). A
small accessor — `FileReference::anchor_var() -> Option<&str>` — surfaces the
matched variable name for consumers that need it.

### Round-trip property

For anchored forms the spelling is **re-resolvable**:

```text
relative.resolve()? == Some(abs)   // for `~/…` and `${VAR}/…`
```

holds for `~/…` and `${VAR}/…` (both first-class `ReferenceKind`s). Repo/package-
relative forms round-trip **only when resolved from the matching root** — an
inherent property of relative references, asserted via `resolve_tuple_from(root)`.

## Consumer configuration

| Consumer | Entry point | `AbbreviationPolicy` |
|----------|-------------|----------------------|
| **sniff** worktree display | `abbreviate` (path is a directory) | `package_relative=false, repo_relative=false, cwd_relative=false, env=ScanAll{deny:[PWD,OLDPWD]}, prefer_env_over_home=true` |
| **darkmatter** `link_normalization` | `abbreviate` (links may point at non-files) | `repo_relative=true, env=Whitelist([PROJECT_ROOT, DOCS_BASE]), prefer_env_over_home=false` |
| **darkmatter** `relative()` fn | `abbreviate` w/ base ctx | `repo_relative=true, cwd_relative=true, env=Off` |
| **claudine** `format_file_link` | `abbreviate` w/ cwd ctx | `cwd_relative=true, repo_relative=false, env=Off` |
| **claudine** `resolve_prompt_display_path` | `abbreviate` w/ cwd ctx | `repo_relative=true, cwd_relative=true, env=Off` |

sniff disables every strip rung because the worktree root **is** the repo (and
package/cwd) root, so those forms would collapse to `"."` — useless for a
"located at" label. sniff and darkmatter use `abbreviate` (not `resolve_tuple`)
because their inputs are directories / possibly-absent paths that the
`is_file()` gate would reject.

## Adoption & removal

All three consumers already depend on `biscuit-file`; biscuit-file depends on
none of them, so there is **no cycle**. The one new edge is `sniff/cli` (which
must add a direct `biscuit-file` dependency — `sniff/lib` already has one).

Each step **deletes** the local implementation in the same change that adopts
the shared API — the duplication is the thing being eliminated, not just
shadowed.

### 1. biscuit-file
Land `abbreviate`, `resolve_tuple{,_from}`, `with_abbreviation`,
`AbbreviationPolicy`/`EnvOffset`/`AbbreviationContext`, `anchor_var()`, and the
`relative_form` core + tests. No behavior change elsewhere yet.

### 2. sniff  (`sniff/cli`)
- **Adopt:** in `output/filesystem/mod.rs`, `worktree_path_link_absolute` calls
  `FileReference::abbreviate(path, &sniff_policy, None).raw()` for the visible
  label; the OSC8 `<blue><a href>` wrapper stays. Apply the same to the
  `worktrees --verbose` branch in `commands/mod.rs:762`.
- **Delete:** `alias_path`, `alias_path_with`, `join_alias`,
  `POSITIONAL_PATH_VARS`, and the `alias_path_*` unit tests (coverage moves to
  biscuit-file). Delete the inline `~/` strip in `commands/mod.rs`.
- **Add:** direct `biscuit-file` dependency to `sniff/cli/Cargo.toml`.
- **Keep (out of scope):** `relative_path_between` — it labels *other* worktrees
  relative to the current one and **can** emit `../sibling` (a `diff_paths`
  concern, not absolute-path abbreviation). Not part of this consolidation.

### 3. darkmatter  (`darkmatter/lib`)
- **Adopt:** `compose/link_normalization.rs` `normalize_links` replaces rungs
  3.6–3.8 with `FileReference::abbreviate(...)` under the whitelist policy; it
  keeps `find_target_range`/`replace_range` rewriting and emits its
  `ComposeReport` warning using `anchor_var()`. `compose/expression/functions.rs`
  `make_relative` (the `relative()` template fn) delegates to `abbreviate` with
  a base-dir context.
- **Delete:** the hand-rolled rung bodies in `make_relative`, the `~/` strip in
  `compose/mod.rs:138`, and the env-prefix loop in `normalize_links`.
- **Keep:** `ComposeOptions::{with_env_path_whitelist, effective_env_path_whitelist,
  default_env_path_whitelist}` as darkmatter's public config surface — now they
  *feed* `EnvOffset::Whitelist(...)` instead of driving a private loop.

### 4. claudine  (`claudine/lib`, `claudine/cli`)
- **Adopt:** `stream/path_link.rs` `format_file_link` computes the visible label
  via `abbreviate` (cwd context, `cwd_relative=true`); the OSC8 link, ellipsis
  truncation, and `escape_prose`/`escape_href` stay. `wrap/composition/mod.rs`
  `resolve_prompt_display_path` and `completion/composition/compose.rs`'s `~/`
  helper delegate likewise.
- **Delete:** the strip/`~/` bodies in those three sites and the private
  `strip_prefix` helper in `path_link.rs` once unused.
- **Keep:** all Prose/OSC8/truncation/escaping (presentation).

## Internal changes

- Factor a private `fn relative_form(abs: &Path, ctx: &ResolutionContext,
  policy: &AbbreviationPolicy) -> ParsedReference` (or the raw spelling `String`
  that `resolve_tuple` wraps into a `FileReference`) in the `resolve` module.
  Pure over its inputs (env supplied via `ctx.env`) so it is unit-testable
  without mutating process state — the same seam sniff's `alias_path_with` uses
  today.
- `resolve_tuple{,_from}` build the context once and call both
  `resolve::resolve` and `relative_form`; `resolve()` keeps building its own
  context and is otherwise untouched.
- `FileReference::abbreviate` is the public, infallible, resolution-free wrapper
  around `relative_form` for callers that already hold an absolute path.
- `FileReference` gains an `abbreviation: AbbreviationPolicy` field
  (`Default`-initialized in `new`). `AbbreviationContext` exposes `{cwd,
  home_dir, env}` publicly (a thin re-export/builder over `ResolutionContext`)
  so `abbreviate` callers can inject a base dir or test fixture.

## Testing

- `relative_form` unit tests, context passed explicitly: env offset wins;
  longest prefix; deterministic tie-break; `~` preferred over equal/shorter env
  prefix; exact-match collapses to bare prefix; non-absolute/empty env ignored;
  `PWD`/`OLDPWD` denied; package/repo/cwd strip-rung precedence and ancestor-only
  (no `../`); `prefer_env_over_home` swap. (These supersede sniff's deleted
  `alias_path_*` tests.)
- `abbreviate` is infallible and **does not require existence**: a directory and
  a non-existent path both yield a label (the sniff worktree case), where
  `resolve_tuple` on the same path returns `None` via the `is_file()` gate.
- Round-trip: `relative.resolve()? == Some(abs)` for `~`/`${VAR}` forms;
  strip-rung forms via `resolve_tuple_from(root)`. `anchor_var()` returns the
  matched variable for `${VAR}` forms, `None` otherwise.
- `resolve_tuple` absolute member always equals `resolve()`; relative is the
  absolute spelling when no anchor applies.
- `#[cfg(windows)]`: every emitted spelling uses `/` separators and carries no
  `\\?\` prefix (see [Dependency: portable path rendering](#dependency-portable-path-rendering)),
  and round-trips back to the same absolute path.

## Open decisions

1. **Default `EnvOffset`** — proposed `Off` (safe; opt-in offsetting). Alternative:
   a built-in whitelist mirroring darkmatter's `PROJECT_ROOT`/`DOCS_BASE`.
2. **Relative-half type** — `FileReference` proposed (honest domain type;
   re-resolvable; carries the matched anchor for darkmatter's warning).
   Lighter alternative: a plain `String` spelling (loses the var-name accessor).
   Either way it is **not** a `PathBuf`. The pair could also be a named
   `ResolvedPair { absolute: PathBuf, relative: FileReference }` for call-site
   clarity over a bare tuple.
3. **Builder vs argument** — policy as a stored builder field
   (`with_abbreviation`) proposed, so `resolve_tuple()` stays argument-free and
   symmetric with `resolve()`. Alternative: `resolve_tuple(&self, &AbbreviationPolicy)`.
