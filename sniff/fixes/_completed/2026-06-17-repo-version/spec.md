---
reviewed: true
status: ready for planning and implementation
supersedes: "2026-06-14-more-repo/spec.md › Fix `sniff repo version`"
---

# Fix & Redesign `sniff repo version`

## Problem

`sniff repo version` returns nothing in this Cargo monorepo (and in any
monorepo whose root manifest carries no version), regardless of where the
user invokes it. Even from inside `sniff/lib` — which declares
`version = "0.1.0"` — the command prints empty text and `--json` emits
`{ "version": null }`.

### Root cause (measured)

The command is wired to the lightweight identity path, not the package
catalog:

- `sniff/cli/src/commands/mod.rs:875` (`RepoAction::Version`) calls
  `detect_repo_identity(dir)` and reads `identity.version`.
- `detect_repo_identity_with_repo` (`sniff/lib/src/filesystem/repo/identity.rs:92`)
  resolves the version with `resolve_version(root)`
  (`identity.rs:187`), which reads **only the git-repo-root manifest**
  (`Cargo.toml` `[workspace.package].version` / `[package].version`,
  then `package.json`, then `pyproject.toml`).
- This repo's root `Cargo.toml` is a **pure virtual workspace**: no
  `[package]`, no `[workspace.package]`. So `resolve_version` finds nothing
  and returns `None`, no matter the CWD.

The per-package version data already exists and is correct — `Package.version`
(`types.rs:208`) is populated during full repo detection by
`resolve_package_version` (`detection.rs:733`). The command simply never
consults it. (Verified: all 75 crates in this repo declare a literal
`version = "..."`, so `Package.version` is populated for every one.)

### What the original spec asked for vs. what is now wanted

The `2026-06-14-more-repo` spec scoped this fix narrowly: read the root /
package-root manifest version and keep the focused leaf shape
`{ "version": string | null }`. Under that contract, a pure virtual
workspace legitimately has no version and `null` is "correct" — but it is
useless. This fix **supersedes** that section with a context-aware,
per-package model modeled directly on `sniff repo test-runner`.

## Goal

`sniff repo version` reports the version(s) of the package(s) in the
**current CWD scope**, with the same scope/collapse/rendering ergonomics as
`sniff repo test-runner`, plus explicit scope-override flags.

Success criteria:

1. From a package directory, the command reports that package's version.
2. From a package-area directory, it reports the versions across the area's
   packages (collapsed to distinct values; a unique list when they vary).
3. From the monorepo root with no flag, it reports across **all** packages
   (CWD-derived `Repo` scope).
4. `--package <NAME>`, `--package-area <NAME>`, and `--all` override the
   CWD-derived scope.
5. A non-monorepo or single package reports its singular version.
6. Output mirrors `test-runner`: default styled comma-separated, `--csv`,
   `--list`, `--md`, `--json`, and `--verbose` evidence.
7. All business logic lives in the library; the CLI only selects scope +
   format and renders.

## Design — mirror `sniff repo test-runner`

The reference implementation is `handle_repo_test_runner`
(`sniff/cli/src/commands/repo.rs:503`) + `output/test_runner_report.rs` +
`aggregate_test_runners` (`lib/src/filesystem/repo/aggregate.rs:204`).
Version follows the same shape, substituting "version + manifest source"
for "runner + evidence source".

Reader note: the original focused JSON contract was
`{ "version": string | null }`. This spec intentionally changes the focused
command to `{ "versions": [...] }` because the command now answers a scoped
package-catalog question and can legitimately return more than one value.
Keep the bare `sniff repo --json` aggregate's top-level
`version: string | null` for compatibility.

### Library

Add to `sniff/lib/src/filesystem/repo/aggregate.rs` (or a sibling
`version.rs` re-exported from `repo::mod`), peer to `TestRunnerAttribution` /
`aggregate_test_runners`:

```rust
/// Where a package's version was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionSource {
    /// Manifest filename the version was read from
    /// (`Cargo.toml`, `package.json`, `pyproject.toml`).
    pub manifest: String,
    /// Repo-relative path to that manifest, for hyperlinks.
    pub path: String,
    /// True when the value was inherited from the workspace root
    /// (Cargo `version.workspace = true` → root `[workspace.package].version`).
    pub inherited: bool,
}

/// One physical version source, grouped with the packages that used it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionSourceAttribution {
    pub source: VersionSource,
    /// In-scope packages that read this version from this source.
    pub packages: Vec<String>,
}

/// A distinct version value across a scope, with the packages that carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionAttribution {
    pub version: String,
    /// In-scope packages contributing this exact version (first-seen order).
    pub packages: Vec<String>,
    /// Distinct manifest sources contributing this exact version.
    pub sources: Vec<VersionSourceAttribution>,
}

/// Collapse per-package versions across `scope` into distinct entries.
///
/// Packages sharing the same version string collapse into one entry carrying
/// all of them (e.g. every crate at `0.1.0`). Differing versions stay
/// separate. Packages with no resolvable version contribute nothing.
pub fn aggregate_versions(
    packages: &[Package],
    scope: &AggregateScope,
) -> Vec<VersionAttribution>;
```

- Collapse key: the **version string** (so uniform repos collapse to one
  entry). Source evidence is grouped separately in `sources`, keyed by
  `(manifest, path, inherited)`, so JSON never implies that a version came from
  only one manifest when several packages read the same value from different
  manifests.
- Reuse the same scope predicate as `aggregate_test_runners`. Since
  `in_scope` is currently private in `aggregate.rs`, either keep
  `aggregate_versions` in that module or extract a small private helper shared
  by both aggregation functions. Do not duplicate divergent scope semantics.

**Capturing `VersionSource`.** `resolve_package_version` (`detection.rs:733`)
currently returns only `Option<String>` and discards which manifest it read.
Use approach A unless implementation proves a concrete gap:

- **A, recommended: derive source at aggregation time by reusing manifest
  parsing helpers.** Locate the same manifest that produced
  `Package.version`, in the same priority order as `resolve_package_version`.
  For Cargo, inspect the package manifest and, when `version.workspace = true`,
  resolve the value from the workspace root. For npm and Python, use the
  package root `package.json` / `pyproject.toml` when present. This avoids a
  serialized `Package` schema change and keeps the implementation surgical.
- **B: enrich detection.** Change `resolve_package_version` to return a
  `(version, source)` pair and store source data on `Package`. This is more
  direct but touches the public serialized package shape and every construction
  site, so it should be used only if A cannot produce accurate source paths.

**Workspace-inheritance robustness (Cargo).** `cargo_package_version`
(`cargo.rs:194`) reads `[package].version` as a string and returns `None` for
`version = { workspace = true }`. This repo uses literal versions so it is
unaffected, but other Cargo workspaces inherit. Resolve
`version.workspace = true` against the root `[workspace.package].version` so
inheriting crates report the workspace version (with `inherited: true`). This
belongs in the library version-resolution path, shared by detection and
aggregation.

When resolving workspace inheritance, do not shell out to Cargo. Parse TOML
with the existing repo manifest parsing stack so the command remains
cross-platform and non-networked.

**Scope override resolver.** Add a helper that turns the CLI flags into an
`AggregateScope`, validating named targets against the catalog:

```rust
/// Resolve an explicit scope override, falling back to CWD-derived scope.
/// Errors when a named package/area does not exist in `info`.
pub fn resolve_scope_with_overrides(
    info: &RepoInfo,
    cwd: &Path,
    all: bool,
    package: Option<&str>,
    package_area: Option<&str>,
) -> Result<AggregateScope>;
```

- `--all` → `AggregateScope::Repo`.
- `--package <NAME>` → `AggregateScope::Package(NAME)` (error if no such
  package; error as ambiguous if more than one catalog package has that name).
- `--package-area <NAME>` → `AggregateScope::PackageArea(NAME)` (error if no such area).
- none → `resolve_scope(info, cwd)` (`aggregate.rs:108`, today's behavior).
- The three overrides are mutually exclusive (`conflicts_with_all` at the CLI
  arg level).

### CLI

Replace the `RepoAction::Version` arm (`commands/mod.rs:875`) with a handler
modeled on `handle_repo_test_runner` (`commands/repo.rs:503`):

1. Discover repo root (same root/`base_dir` resolution as test-runner).
2. `detect_repo_structure(&root)`.
3. Resolve scope via `resolve_scope_with_overrides` (CWD default + flags).
4. Entries:
   - Monorepo with packages → `aggregate_versions(packages, &scope)`.
   - Non-monorepo / no packages → resolve the directory's own manifest version
     directly into a single `VersionAttribution` with empty `packages` and one
     source attribution when a manifest source is known (the test-runner
     fallback shape).
5. Render: `--json` → array shape (below); else text via a new
   `output/version_report.rs` mirroring `output/test_runner_report.rs`.
6. Empty result → emit nothing on stdout, hint on stderr when not `--plain`,
   exit 1 unless `--no-error` (preserve the existing `--no-error` / `--on-error`
   flags).

**Args** (`args/repo.rs:708`, the `Version` variant) — add, mirroring the
`TestRunner` variant plus the scope flags:

```rust
Version {
    // existing
    no_error: bool,
    on_error: Option<String>,
    // rendering (mirror TestRunner)
    csv: bool,    // conflicts_with_all = ["list", "md"]
    list: bool,   // conflicts_with_all = ["csv", "md"]
    md: bool,     // conflicts_with_all = ["csv", "list"]
    // scope overrides (mutually exclusive)
    all: bool,                      // conflicts_with_all = ["package", "package_area"]
    package: Option<String>,        // conflicts_with_all = ["all", "package_area"]
    package_area: Option<String>,   // conflicts_with_all = ["all", "package"]
}
```

`--verbose`/`-v` and `--plain` are already global CLI flags (as for
test-runner); the handler receives `verbose: u8` and `plain: bool`.

The implementation must update the help examples in `args/mod.rs` that
currently describe `sniff repo version --json` as `{ "version": "0.1.0" }`.

### Text rendering

Mirror `test_runner_report.rs` exactly; substitute version for runner name.

- **Default** (no `--csv`/`--list`/`--md`): styled, comma-separated distinct
  versions (the `render_entries` analogue).
- **`--csv`**: plain comma-separated on one line.
- **`--list`**: one entry per line.
- **`--md`**: one `- entry` per line.
- **Non-verbose** entry markup: just the version, e.g. `0.9.3`.
- **Verbose** entry markup (per the requested form):

  ```
  0.9.3 (<dim><i>from </i><blue><a href="file://…/package.json">package.json</a></blue> <i>in</i> <b>{package}</b></dim>)
  ```

  - Workspace-inherited Cargo: name the source as `[workspace.package]` in the
    root `Cargo.toml` rather than a single package, following the
    test-runner shared-config disambiguation (`entry_markup`,
    `test_runner_report.rs:108` — name a package only when `packages.len() == 1`).
  - When one collapsed version has multiple manifest sources, verbose text
    should not print a misleading first source. Render one concise source
    detail only when `sources.len() == 1`; otherwise render a summary such as
    `from 12 manifests` and include package names only for single-package
    source groups.
  - `--plain` strips styling, as elsewhere.

All terminal output goes through `Prose` / `biscuit-terminal`, never
hand-written ANSI (same as `test_runner_report.rs`).

### JSON shape

Focused `sniff repo version --json` returns the array shape (mirrors
`build_test_runner_json`, `test_runner_report.rs:22`):

```jsonc
{ "versions": [
  { "version": "0.9.3",
    "packages": ["web"],
    "sources": [
      { "manifest": "package.json",
        "path": "web/package.json",
        "href": "file://…/web/package.json",
        "inherited": false,
        "packages": ["web"] }
    ] }
]}
```

- Empty result → `{ "versions": [] }` on stdout; exit 1 unless `--no-error`.
- The `--on-error` text applies to text mode only; `--json` stdout stays valid
  JSON (`{ "versions": [] }`), never a message string.
- Remove or replace `output::repo_json::version_outcome`; leaving both focused
  JSON builders around with different shapes invites accidental reuse of the
  obsolete `{ "version": ... }` contract.

### Bare `sniff repo --json` aggregate — top-level `version`

The consolidated `SniffRepo` aggregate keeps a single top-level
`version: string | null` (it is a repo-wide identity field). Recompute it as
the **`AggregateScope::Repo` collapse**:

- exactly one distinct version across all packages → that string;
- zero or more-than-one distinct versions → `null`.

This replaces the current root-manifest-only value (which is `null` here).
`RepoIdentity.version` and the `repo name` path are unaffected; only the bare
aggregate's top-level `version` switches to the collapse. claudine's
`repo --json` consumer reads `version` as `string | null` either way, so the
type is unchanged — only the value improves.

The aggregate builder currently assigns `identity.version.clone()` to
`SniffRepo.version` in `output/repo_json.rs`; replace that assignment with a
library-derived repo-scope collapse. Do not make the CLI JSON builder inspect
package manifests itself.

## Out of scope

- Reworking the bare `sniff repo --json` aggregate beyond the single
  top-level `version` value (the broader aggregate redesign is owned by
  `2026-06-14-more-repo`).
- Adding version reporting for ecosystems Sniff does not already parse
  (Go remains `null`; JVM/.NET/PHP/Ruby/Elixir only if the existing parser
  reads them safely).
- Adding registry lookups or "latest version" checks. This command reports
  declared package versions only and must not perform network requests.
- `--package` / `--package-area` semantics for any other `repo` subcommand
  (owned by `2026-05-07-repo-package-consistency`); this fix only adds them to
  `version`.

## Acceptance criteria

1. **Monorepo root** (`sniff repo version` at repo root) lists the distinct
   version(s) across all packages — for this repo, `0.1.0`. `--json` →
   `{ "versions": [ { "version": "0.1.0", … } ] }`.
2. **Package scope** (`cd sniff/lib && sniff repo version`) → `0.1.0`.
3. **Package-area scope** (`cd sniff && sniff repo version`) → the distinct
   version(s) across `sniff/*` packages.
4. **Explicit overrides**: `--all`, `--package <NAME>`, `--package-area <NAME>`
   select scope independent of CWD; an unknown name errors clearly.
5. **Variance** renders a unique list across `--csv` / `--list` / `--md`;
   uniform collapses to a singular value.
6. **Verbose** appends the manifest source (hyperlinked) and, when
   disambiguating, the package — workspace-inherited Cargo versions name
   `[workspace.package]`, not a single crate.
7. **Single-package / non-monorepo** reports its singular version.
8. **Empty** (no resolvable version anywhere in scope) prints nothing and
   exits 1; `--no-error` exits 0; `--json` always emits `{ "versions": [] }`.
9. **Library-owned**: `aggregate_versions` + scope resolution live in
   `sniff/lib`; the CLI only selects scope/format and renders via
   `biscuit-terminal`.
10. Library unit tests cover collapse (uniform, variant, single, empty,
    multiple sources for the same version, and workspace-inheritance) mirroring
    the `aggregate_test_runners` tests (`aggregate.rs:383+`); CLI integration
    tests cover scope flags, formats, verbose, JSON shape, and exit codes.
11. Bare `sniff repo --json` reports a top-level string only when repo-scope
    aggregation finds exactly one distinct package version; it reports `null`
    for zero or multiple distinct versions.
12. The `sniff` skill's `repo version` line, CLI README, and relevant help
    examples are updated to the new scope/format/JSON contract.
