# Spike S3: typed parser feasibility

Phase 1 spike for `2026-09-26-lockfile-corroboration`. The throwaway probes
live outside the repository in `/tmp/s3-spike` and are not merged. They use the
exact parser versions Sniff resolves through `biscuit-file`'s re-exports:
`serde_yaml_ng` 0.10.0, `toml` 1.1.2, and `serde_json` 1.0.149. Every probe ran
against the checked-in S1 fixtures under
`sniff/lib/tests/fixtures/lockfiles/`. Timings are in
[measurements.md](./measurements.md#s3-typed-parser-feasibility-phase-1-spike).

## Verdicts

| Question from the plan | Verdict |
|---|---|
| A real Berry `yarn.lock` (comment header, `__metadata`) parses as YAML through `serde_yaml_ng` | **Yes.** Yarn 3.8.7, 4.0.2, and 4.18.1 all parse into a typed visitor |
| Typed `serde_json` with `IgnoredAny` handles a large `package-lock.json` in one pass | **Yes.** 5.9 MiB in 5.8 ms with a 3.4 MiB peak, against 43 MiB for `serde_json::Value` |
| The `pnpm-typed-parser.patch` approach extends to reject non-string importer keys and capture `lockfileVersion` | **Yes, but not with `String` keys.** See the pnpm section |

## Per-format field paths and findings

### pnpm (`pnpm-lock.yaml`)

- **Version:** top-level `lockfileVersion`. pnpm 8.15.9 writes `'6.0'`; pnpm
  9.15.9 and 10.32.1 both write `'9.0'`, byte-identical for the same project.
  pnpm 5-era lockfiles wrote an unquoted number (`5.4`), so the field must
  accept a string or a number (an untagged enum works) and classify both.
- **Membership:** the keys of top-level `importers`. `.` is the root and is
  excluded. Keys are `/`-separated and relative to the lockfile's directory.
  `.tools/hidden` keeps its leading dot.
- **Unrelated local dependency:** `link:./local-lib` appears only as a
  dependency *value* (`version: link:local-lib`), never as an importer key.
- **Non-string keys:** deserializing keys as `String` does **not** reject them.
  `serde_yaml_ng` coerces the plain scalars `1`, `~`, and `true` to the strings
  `"1"`, `"~"`, and `"true"`. To reject them, deserialize each key as
  `serde_yaml_ng::Value`, as the patch does, and fail on any variant other than
  `Value::String`, instead of the patch's silent drop.
- **Duplicates:** a key-set visitor that tracks seen keys rejects the
  `duplicate-key` variant. A duplicate top-level `importers` key is rejected by
  serde's derive (`duplicate field`). A duplicate key inside `packages:` is
  tolerated, because that section is skipped as `IgnoredAny`. This is the
  documented divergence from the generic reference.
- **Missing section:** a derived struct without `#[serde(default)]` on
  `importers` reports `missing field`, which maps to `parse_failed`. The patch's
  `Option` plus `importer_keys_or_none` returned `None` for a non-mapping value;
  that must become `parse_failed` too.

### npm (`package-lock.json`, `npm-shrinkwrap.json`)

- **Version:** top-level numeric `lockfileVersion`. npm 11.6.4 writes `3`, or
  `2` with `--lockfile-version 2`. npm 6.14.18 writes `1`, which has no
  `packages` object.
- **The trap:** a `file:` dependency is recorded exactly like a member. Both
  `packages["local-lib"]` and `packages["node_modules/local-lib"]`
  (`link: true`, `resolved: "local-lib"`) appear beside
  `packages["packages/alpha"]` and `packages["node_modules/alpha"]`
  (`link: true`, `resolved: "packages/alpha"`). Path shape alone cannot tell
  them apart.
- **Membership rule that works on every fixture:** read the locked workspace
  declarations from `packages[""].workspaces` (`["packages/*",
  ".tools/hidden"]`). The locked member set is every non-`node_modules/`
  `packages` key, other than `""`, that those declarations match. `link: true`
  records whose `resolved` target matches the declarations corroborate it.
  This keeps `local-lib` out and keeps deleted members (such as the edited
  `packages/gamma`) in, because the rule reads the lockfile's own declarations
  rather than the current manifest.
- **Consequence for Phase 3:** matching needs a pure string glob matcher over
  `/`-separated paths, with the same dot-directory semantics the manifest-side
  expander uses. The existing expander in `glob.rs` walks the filesystem, so it
  cannot be reused as is.
- **Duplicates:** `serde_json` into a map is silently last-wins for the
  `duplicate-key` variant (`packages/beta` twice), so the `packages` map needs a
  seen-set visitor.
- **Missing section:** `packages` absent at v2 or v3 must be `parse_failed`.
  At v1 it is the normal shape, reported as `unsupported_version`.
- **Shrinkwrap:** `npm-shrinkwrap.json` is byte-identical to the v3
  `package-lock.json`; only the filename differs.

### Yarn (`yarn.lock`)

- **Classic signature first:** Classic 1.22.22 writes
  `# yarn lockfile v1` in its comment header, and its body is not YAML. The typed
  parse fails with `invalid type: string "version \"1.0.0\""`. The signature
  must be checked **before** parsing, so Classic reports `unsupported_version`
  instead of `parse_failed`. Classic records no workspace members at all.
- **Berry version:** `__metadata.version`, an integer: `6` for Yarn 3.8.7,
  `8` for 4.0.2, and `10` for 4.18.1.
- **Membership:** every top-level entry whose `resolution` is
  `<name>@workspace:<path>`. Take the path after the **last** `@workspace:`
  marker (`rfind`), which handles scoped names such as
  `@fixture/beta@workspace:packages/beta`. The entry key may hold combined
  descriptors (`"@fixture/beta@workspace:^, @fixture/beta@workspace:packages/beta"`),
  so read `resolution`, never the key. The root is `<root>@workspace:.` and is
  excluded.
- **Non-members:** `link:` and `portal:` entries have resolutions such as
  `local-lib@link:./local-lib::locator=...`, which never contain `@workspace:`.
  They also carry `linkType: soft` like members, so `linkType` is not a usable
  signal.
- **Completeness:** Yarn always writes the root workspace entry. When no
  `@workspace:.` resolution is present (as in the `missing-required-field`
  variant, which deleted every workspace entry), report `parse_failed`, never an
  empty set.
- **Duplicates:** the visitor's seen-set rejects the duplicated
  `alpha@workspace:packages/alpha` key.

### Bun (`bun.lock`, `bun.lockb`)

- **Version:** `lockfileVersion` is `1` for both Bun 1.2.0 and Bun 1.3.3.
  Bun 1.3.3 adds `configVersion: 1`; Bun 1.2.0 omits it. Membership does not
  depend on `configVersion`.
- **Membership:** the keys of top-level `workspaces`. `""` is the root and is
  excluded. `packages` also lists `<name>@workspace:<path>` tuples, but the
  `workspaces` keys are the authority.
- **Syntax:** real Bun output always has trailing commas, so strict JSON fails
  on every real `bun.lock`. The parser is `jsonc-parser` with strict options
  (see [spike-s2-jsonc.md](./spike-s2-jsonc.md)), plus a seen-set visitor for
  `workspaces`.
- **Binary:** `bun.lockb` starts with `#!/usr/bin/env bun\nbun-lockfile-format-v0\n`
  and then binary data. It is selected by filename and never read.
- **Precedence:** Bun 1.3.3 never leaves both files in place. The
  `precedence-both` fixture was produced by Bun 1.1.38 writing `bun.lockb` next
  to an existing `bun.lock`.

### uv (`uv.lock`)

- **Version:** top-level `version = 1` with `revision = 3` (uv 0.9.5).
- **Membership:** `[manifest].members` holds package **names**, including the
  root's name. Map each name to a path through that package's local source:
  `source = { editable = "<path>" }` for a packaged member, or
  `source = { virtual = "<path>" }` for a member without a build system and for
  a project root (`virtual = "."`). Exclude the entry whose path is `.`.
- **Unrelated local dependency:** `local-lib` has `source = { directory =
  "local-lib" }` and is not listed in `members`. An editable path dependency
  (`editable = true` in `[tool.uv.sources]`) would also have an `editable`
  source, so the source kind alone never proves membership. Only names listed in
  `members` count.
- **No `[manifest]` table:** uv omits `[manifest]` when the root is the only
  workspace member (checked-in `root-only-workspace` and `single-project`
  fixtures). An absent `[manifest]` therefore means "the locked member set is
  the root alone", which is empty after root exclusion. It is **not**
  `parse_failed`. The `missing-required-field` variant (with `[manifest]`
  deleted) consequently yields `mismatch`, with every current member missing.
- **Virtual root:** `virtual-root` lists only `alpha` and `beta`; the root
  appears nowhere in the lock.
- **Ambiguity:** a member name with no local package entry, or with more than
  one local package entry of that name, yields `ambiguous_membership`.
- **Sniff's current reader** looks for `[workspace].members`, which no uv
  version writes. The Phase 2 uv task deletes that expectation.

### Cargo (`Cargo.lock`)

- **Version:** v4 (`version = 4`, cargo 1.98.1), v3 (`version = 3`, cargo
  1.77.2), and v2, which has **no** top-level `version` key and an inline
  `checksum` on registry packages (cargo 1.52.0). v1 had no `version` key and a
  `[metadata]` table of checksums; there is no v1 fixture.
- **No membership data:** members, the root, and the excluded path dependency
  `local-lib` all appear as `[[package]]` entries with no `source`. Cargo
  permits a member named `itoa` 0.1.0 beside a renamed registry `itoa` 1.0.18
  (`source = "registry+..."`). Matching by name alone would find the registry
  entry in the `missing` variant; R1's name + resolved version + no-`source`
  rule does not.
- **Inherited version:** `alpha` uses `version.workspace = true` and is
  recorded as `0.3.0`. The manifest-side resolved version must follow the
  inheritance.
- **Missing `[[package]]`:** the `missing-required-field` variant has no
  `package` array. Cargo always writes an entry for every workspace member, so
  corroboration reports `parse_failed`. `CargoLockVersions` keeps treating a
  missing array as an empty index, which preserves its parity.

### Rush (`common/config/rush/pnpm-lock.yaml`)

- **Syntax:** the same pnpm `importers` map (Rush 5.179.0 ran pnpm 9.15.9, so
  the version is `'9.0'`).
- **Importer base:** keys are relative to `<repo>/common/temp`, not to the
  lockfile's directory. The keys are `.`, `../../.tools/hidden`,
  `../../packages/alpha`, and `../../packages/beta`. The `.` key is the
  synthetic `common/temp` project, not the repository root, so exclude it.
  Translate `../../x` to the repository-relative `x`.
- **Layout settings in the fixture:** `rush.json` declares `pnpmVersion:
  "9.15.9"`, `pnpm-config.json` sets `useWorkspaces: true`, and
  `subspaces.json` sets `subspacesEnabled: false`.
- **Pre-existing detection bug (blocks Rush corroboration):** real `rush.json`
  files are JSON with comments. `rush init` emits 93 comment lines, and the
  file begins with one. `parse_rush_project_folders`
  (`sniff/lib/src/filesystem/repo/npm.rs`) uses strict `serde_json`, so it
  returns no projects, and Sniff reports **no Rush layer** for the real fixture.
  With comments and trailing commas stripped (a scratch copy), the installed
  `sniff` reports a `rush-stack` layer with all three members. `rush.json` and
  `pnpm-config.json` must both be read with the S2 JSONC parser. This is added
  to the Phase 3 Rush task as a prerequisite.

### Fallback formats (metadata only; never parsed)

| Format | Fixture | Version field (for the record only) |
|---|---|---|
| Go `go.work.sum` | `go-1.27.1/workspace` | none; checksum lines only. Go writes it only after a command that needs extra sums (`go list -m all`) |
| Gradle root `gradle.lockfile` | `gradle-8.14.5/root-lockfile` | none |
| Gradle legacy root `gradle/dependency-locks/*.lockfile` | `gradle-5.6.4/root-legacy-lock-dir` (14 files) | none |
| Gradle subproject-only locks | `gradle-8.14.5/multi-project`, `gradle-5.6.4/legacy-lock-dir` | none. The root has no configurations, so no root lock exists and the result is `absent` |
| Bazel `MODULE.bazel.lock` | `bazel-8.4.2/bzlmod` | `lockFileVersion: 18` (JSON) |
| Poetry `poetry.lock` | `poetry-2.5.1/single-project` | `[metadata].lock-version = "2.1"` |
| PDM `pdm.lock` | `pdm-2.29.2/single-project` | `[metadata].lock_version = "4.5.1"` |
| Composer `composer.lock` | `composer-2.10.3/single-project` | none (`plugin-api-version` is the plugin API) |

## Parser cost finding (for Phase 2 and Phase 5)

A typed struct removes the `Value` allocation Sniff would otherwise build. It
does **not** make the YAML or TOML crates stream:

- `serde_yaml_ng` 0.10 buffers the document's whole event stream before
  deserializing, so a typed pnpm or Yarn parse still peaks at about 17x the
  input size (75 MiB for 4.4 MiB). That is about a third less than the generic
  `Value` parse.
- `toml` 1.1.2 deserializes through its own complete document tree, so typed
  and generic TOML parses have the **same** peak (74 MiB for a 4.3 MiB
  `Cargo.lock`; 182 MiB for an 8.3 MiB `uv.lock`). The typed parse is about 15%
  faster.
- `serde_json` and `jsonc-parser` do stream: the peaks are about 0.6x input or
  less.

The spec's rule "Do not allocate a generic tree for an entire lockfile" is
therefore met by Sniff's code (no `Value` is materialized). The crate-internal
buffering remains, and Phase 5's measurements must record it rather than claim
streaming. Replacing either crate with an event-level parser is out of this
feature's scope.
