# Accepted lockfile versions

This is the accepted-version matrix for `2026-09-26-lockfile-corroboration`.
The spec requires it before implementation. Each accepted version has at least
one checked-in fixture written by the recorded real tool. A version that is not
listed as accepted is `unverifiable` + `unsupported_version`; it never yields
`match` or `mismatch`. Accepting another version means adding a real-tool
fixture and a row here in the same change.

- **Fixture root:** `sniff/lib/tests/fixtures/lockfiles/`, one
  `<tool>-<exact version>/<case>/` directory per fixture. Each directory has a
  `PROVENANCE.md` with the tool version, commands, host, date, and expected
  membership. Paths below are relative to the fixture root.
- **Evidence:** field paths and parser findings are in
  [spike-s3-parsers.md](./spike-s3-parsers.md). The JSONC crate choice is in
  [spike-s2-jsonc.md](./spike-s2-jsonc.md).
- **Member sets:** every set below is relative to the layer root, excludes the
  root, and is sorted. The standard JS workspace fixture declares
  `.tools/hidden`, `packages/alpha`, and `packages/beta`, and depends on the
  non-member `local-lib` through the ecosystem's local-path mechanism.

## Compared formats

| Format | Accepted version values | Producer (fixture) | Membership field path | Known unsupported versions → `unsupported_version` | Signature for versions without a numeric field |
|---|---|---|---|---|---|
| pnpm `pnpm-lock.yaml` | `lockfileVersion` `'6.0'`, `'9.0'` (string; a number with the same value is equivalent) | pnpm 8.15.9 (`pnpm-8.15.9/workspace`), 9.15.9 (`pnpm-9.15.9/workspace`), 10.32.1 (`pnpm-10.32.1/workspace`) | keys of top-level `importers`; `.` is the root | `5.x` numeric (pnpm ≤ 7) and any other value | none needed: every pnpm lockfile has `lockfileVersion` |
| npm `npm-shrinkwrap.json`, else `package-lock.json` | `lockfileVersion` `2`, `3` | npm 11.6.4 (`npm-11.6.4/workspace`, `workspace-lockfile-v2`, `shrinkwrap`) | non-`node_modules/` keys of `packages` (other than `""`) matched by the locked declarations `packages[""].workspaces`; `link: true` records whose `resolved` matches corroborate | `1` (npm 6.14.18, `npm-6.14.18/single-project`: no `packages` object) and any other value | none needed |
| Yarn Berry `yarn.lock` | `__metadata.version` `6`, `8`, `10` | Yarn 3.8.7 (`yarn-3.8.7/workspace`), 4.0.2 (`yarn-4.0.2/workspace`), 4.18.1 (`yarn-4.18.1/workspace`) | every top-level entry's `resolution` of the form `<name>@workspace:<path>`, split at the last `@workspace:`; `<path>` `.` is the root | any other `__metadata.version` | Yarn Classic: the header line `# yarn lockfile v1` (Yarn 1.22.22, `yarn-1.22.22/workspace`) → `unsupported_version`, checked before parsing |
| Bun `bun.lock`, else `bun.lockb` | `bun.lock` `lockfileVersion` `1` (`configVersion` optional) | Bun 1.2.0 (`bun-1.2.0/workspace`, no `configVersion`), Bun 1.3.3 (`bun-1.3.3/workspace`) | keys of top-level `workspaces`; `""` is the root | any other `lockfileVersion` (for example `0` from pre-1.2 builds) | `bun.lockb` (`bun-1.3.3/workspace-binary`) is selected by filename, never read, and reports `unverifiable` + `no_membership_data` |
| uv `uv.lock` | `version` `1` with `revision` `3` | uv 0.9.5 (`uv-0.9.5/workspace`, `virtual-root`, `root-only-workspace`, `single-project`) | `[manifest].members` names, each mapped through that package's `source.editable` or `source.virtual` path; path `.` is the root. No `[manifest]` = the root is the only member | any other `version`, or `version` `1` with a `revision` other than `3` | none needed |
| Cargo `Cargo.lock` | `version` `3`, `4`; v2 by signature | cargo 1.98.1 (`cargo-1.98.1/workspace`, v4), cargo 1.77.2 (`cargo-1.77.2/workspace-v3`), cargo 1.52.0 (`cargo-1.52.0/workspace-v2`) | none: `[[package]]` entries with `name`, `version`, and no `source` (ruling R1: `members_present` / `members_missing`, never `match` / `mismatch`) | any other numeric `version`; v1 | v2: no top-level `version` key **and** no `[metadata]` table (registry entries carry an inline `checksum`). v1: no `version` key **and** a `[metadata]` table → `unsupported_version` (no fixture exists, so v1 is not accepted) |
| Rush (pnpm layout) `common/config/rush/pnpm-lock.yaml` | as pnpm | Rush 5.179.0 running pnpm 9.15.9 (`rush-5.179.0/pnpm-workspace`) | pnpm `importers` keys resolved against `<repo>/common/temp`; `.` is the synthetic `common/temp` project and is excluded (`../../packages/alpha` → `packages/alpha`) | as pnpm | not a lockfile version: the layout is classified from `rush.json` (`pnpmVersion`), `pnpm-config.json` (`useWorkspaces`), `subspaces.json` (`subspacesEnabled`), and `variants/` per ruling R3 |

**Rush configuration syntax:** `rush.json` and `pnpm-config.json` are JSON
with comments. They must be read with the S2 JSONC parser. Strict JSON rejects
every real `rush.json`, which is a pre-existing detection bug described in
spike S3.

## Expected results for the compared-format fixtures

Every fixture below is judged against the manifests in its own directory, with
corroboration enabled. A real fixture is unedited tool output. An `-edited-`
variant changes only the lockfile, as described in its `PROVENANCE.md`.

| Fixture | `status` | `reason` | `extra` | `missing` |
|---|---|---|---|---|
| `pnpm-8.15.9/workspace`, `pnpm-9.15.9/workspace`, `pnpm-10.32.1/workspace` | `match` | `null` | `[]` | `[]` |
| `npm-11.6.4/workspace`, `workspace-lockfile-v2`, `shrinkwrap` | `match` | `null` | `[]` | `[]` |
| `yarn-3.8.7/workspace`, `yarn-4.0.2/workspace`, `yarn-4.18.1/workspace` | `match` | `null` | `[]` | `[]` |
| `bun-1.2.0/workspace`, `bun-1.3.3/workspace` | `match` | `null` | `[]` | `[]` |
| `bun-1.3.3/workspace-edited-comments-trailing-commas` | `match` | `null` | `[]` | `[]` |
| `uv-0.9.5/workspace`, `uv-0.9.5/virtual-root` | `match` | `null` | `[]` | `[]` |
| `uv-0.9.5/root-only-workspace` | `match` (both sets empty) at the parser and engine level; through detection there is no `UvWorkspace` layer, because the uv detector requires a non-empty `members` (Phase 2 finding) | `null` | `[]` | `[]` |
| `cargo-1.98.1/workspace`, `cargo-1.77.2/workspace-v3`, `cargo-1.52.0/workspace-v2` | `members_present` | `subset_only` | `[]` | `[]` |
| `rush-5.179.0/pnpm-workspace` | `match` | `null` | `[]` | `[]` |
| `npm-6.14.18/single-project` | `unverifiable` | `unsupported_version` | `[]` | `[]` |
| `yarn-1.22.22/workspace` | `unverifiable` | `unsupported_version` | `[]` | `[]` |
| `bun-1.3.3/workspace-binary` | `unverifiable` | `no_membership_data` | `[]` | `[]` |
| `bun-1.3.3/precedence-both` | `match`; `paths` = `["bun.lock"]` only (ruling R6) | `null` | `[]` | `[]` |
| `*-edited-stale-extra` for pnpm, npm, Yarn, Bun, uv | `mismatch` | `null` | `["packages/gamma"]` | `[]` |
| `cargo-1.98.1/workspace-edited-stale-extra` | `members_present` (Cargo cannot recover stale extras) | `subset_only` | `[]` | `[]` |
| `*-edited-missing` for pnpm, npm, Yarn, Bun, uv | `mismatch` | `null` | `[]` | `["packages/beta"]` |
| `cargo-1.98.1/workspace-edited-missing` | `members_missing` (the registry `itoa` 1.0.18 does not satisfy the member `itoa` 0.1.0) | `subset_only` | `[]` | `["crates/beta"]` |
| `*-edited-malformed-trailing` (all six formats) | `unreadable` | `parse_failed` | `[]` | `[]` |
| `*-edited-duplicate-key` (pnpm, npm, Yarn, Bun) | `unreadable` | `parse_failed` | `[]` | `[]` |
| `*-edited-unknown-version` (all six formats) | `unverifiable` (never `mismatch`) | `unsupported_version` | `[]` | `[]` |
| `*-edited-missing-required-field` for pnpm, npm, Yarn, Bun, Cargo | `unreadable` | `parse_failed` | `[]` | `[]` |
| `uv-0.9.5/workspace-edited-missing-required-field` | `mismatch` (an absent `[manifest]` means "root only") | `null` | `[]` | `[".tools/hidden", "packages/alpha", "packages/beta"]` |

Notes on the table:

- **Cargo member paths:** the Cargo fixture's members are `.tools/hidden`,
  `crates/alpha` (inherited version `0.3.0`), and `crates/beta` (crate `itoa`
  0.1.0). `local-lib` is excluded from the workspace but is also source-less
  in the lock, which is why R1 cannot claim equality.
- **uv `single-project`** is not a uv workspace, so no `UvWorkspace` layer
  exists. It is kept as evidence of the absent-`[manifest]` shape.
- **Duplicate keys in TOML** cannot be written without a syntax error, so
  Cargo and uv have no `duplicate-key` variant. Their `malformed-trailing`
  variant covers invalid syntax.
- **Tolerated duplicates:** a duplicate key inside a section the parser skips
  (for example pnpm `packages:`) is tolerated. This deliberate difference from
  the generic reference parser is covered by a Phase 2 test.

## Fallback formats (metadata only)

These are never opened or parsed. With corroboration enabled, a present file
is `unverifiable` + `no_membership_data`. With corroboration disabled, it is
`not_requested` + `request_disabled`. No version is inspected, so no version is
"accepted" or "unsupported".

| Standard | Candidate | Fixture | Expected with corroboration enabled |
|---|---|---|---|
| `GoWorkspace` | `go.work.sum` | `go-1.27.1/workspace` (Go 1.27.1) | `unverifiable` + `no_membership_data`, `paths` `["go.work.sum"]` |
| `GradleMultiProject` | root `gradle.lockfile` | `gradle-8.14.5/root-lockfile` (Gradle 8.14.5) | `unverifiable` + `no_membership_data`, `paths` `["gradle.lockfile"]` |
| `GradleMultiProject` | legacy root `gradle/dependency-locks/*.lockfile` | `gradle-5.6.4/root-legacy-lock-dir` (Gradle 5.6.4, 14 files) | `unverifiable` + `no_membership_data`, `paths` = the 14 `gradle/dependency-locks/<configuration>.lockfile` paths |
| `GradleMultiProject` | subproject locks only | `gradle-8.14.5/multi-project`, `gradle-5.6.4/legacy-lock-dir` | `absent`; subproject lockfiles are never searched |
| `Bazel` | `MODULE.bazel.lock`, only when `MODULE.bazel` exists | `bazel-8.4.2/bzlmod` (Bazel 8.4.2 via bazelisk) | `unverifiable` + `no_membership_data`, `paths` `["MODULE.bazel.lock"]` |
| `MavenMultiModule`, `DotNetSolution`, `Pants`, `Buck2` | none | none needed (table test) | `not_applicable` + `no_lockfile_source` |
| `Unknown` | none | none needed (table test) | `not_applicable` + `unknown_standard` |

## Standalone lockfiles (ruling R2)

Repository-level observations, not layers. Metadata only, and absent files
produce no entry.

| Tool | Candidate | Fixture | Expected entry with corroboration enabled |
|---|---|---|---|
| `poetry` | `poetry.lock` | `poetry-2.5.1/single-project` (Poetry 2.5.1; `[metadata].lock-version = "2.1"`) | `root` `""`, `unverifiable` + `no_membership_data`, `paths` `["poetry.lock"]` |
| `pdm` | `pdm.lock` | `pdm-2.29.2/single-project` (PDM 2.29.2; `[metadata].lock_version = "4.5.1"`) | `root` `""`, `unverifiable` + `no_membership_data`, `paths` `["pdm.lock"]` |
| `composer` | `composer.lock` | `composer-2.10.3/single-project` (Composer 2.10.3) | `root` `""`, `unverifiable` + `no_membership_data`, `paths` `["composer.lock"]` |

### Detection notes from Phase 3

- **Composer:** Sniff recognizes no PHP-only package, so a project with only
  `composer.json` yields no repository result, and no observation. The L1 test
  adds a `package.json` to the Composer fixture (a common PHP-plus-JS
  layout).
- **Bazel:** the Bazel detector needs leaf `BUILD` packages, which the
  `bazel-8.4.2/bzlmod` fixture does not ship. The L1 test adds two empty
  `BUILD.bazel` files to its temporary copy. A Bazel root without
  `MODULE.bazel` has no lockfile source: `not_applicable` +
  `no_lockfile_source`, even beside a stray `MODULE.bazel.lock`.
- **Rush configuration that cannot be read or parsed** (`pnpm-config.json`,
  `subspaces.json`) cannot be classified, so the layer is `unverifiable` +
  `unsupported_layout`, not `unreadable`: the lockfile itself is fine. An
  omitted `useWorkspaces` is Rush's default, `false`, which is the legacy
  install. Without `pnpm-config.json`, the legacy `rush.json`
  `pnpmOptions.useWorkspaces` decides.
- **npm without locked declarations:** a root record with no `workspaces` is
  `ambiguous_membership` when the lockfile records any local package path
  (it could be a workspace or a `file:` dependency), and an empty member set
  otherwise. A declaration that is not a valid glob is also
  `ambiguous_membership`.

## JSONC parser choice (spike S2)

`bun.lock`, `rush.json`, and `pnpm-config.json` are parsed with
**`jsonc-parser` 0.33.2** (`serde` feature) through `parse_to_serde_value` and
an explicit strict `ParseOptions` literal: comments and trailing commas are
allowed, and every other extension is off. It streams (0.03 MiB peak on a 15.7
MiB `bun.lock`) and rejects trailing garbage. It is a new direct dependency of
`sniff`, so Phase 5 updates `sniff/docs/dependencies.md` and the root
`docs/dependencies.md`. `json-five`, already in the graph through
`biscuit-file`, was rejected because it materializes a full AST (a 149 MiB
peak on the same input) and cannot turn off JSON5 extensions.
