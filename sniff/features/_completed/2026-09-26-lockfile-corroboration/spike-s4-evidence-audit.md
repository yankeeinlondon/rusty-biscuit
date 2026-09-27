# Spike S4: probe-evidence and completeness audit

Read-only audit of the tree at `fix/sniff` (HEAD `661dc21c0`), done on
2026-09-26. Line numbers refer to that tree. No production code was changed.

## 1. Lockfile call-site table

Search: `rg -n` over `sniff/lib/src` and `sniff/cli/src` for every lockfile
name the plan lists. `sniff/cli/src` has no hits. `npm-shrinkwrap.json`,
`poetry.lock`, `pdm.lock`, `composer.lock`, `go.work.sum`, `gradle.lockfile`,
`MODULE.bazel.lock`, and `Gemfile.lock` have **no** filesystem access anywhere
today. `go.sum` appears only in name classification.

Counter key: **MP** = `FS_METADATA_PROBES`, **LR** = `REPO_LOCKFILE_READS`,
**LP** = `REPO_LOCKFILE_PARSES`, **FO** = `FS_FILE_OPENS`, **BR** =
`FS_BYTES_READ`, **CN** = `FS_CANONICALIZATIONS`.

`probe_exists` (`detection.rs:490-493`) increments MP once and calls
`Path::exists()`. It follows symlinks and folds every error into `false`, and
it has no cache.

| file:line | function | lockfile(s) | purpose | counters | reaches `ManifestStore`? | recommendation |
|---|---|---|---|---|---|---|
| `detection.rs:536,550` | `has_workspace_marker` | `yarn.lock` (1 of 18 markers, short-circuits on first hit) | Gate for building the shared observation walk | MP per marker probed | **No.** It is called from `detect_repo_inner_with_request` (`detection.rs:131`) before any store exists. The store is created at `detection.rs:597`. | Leave alone. This is a "could this be a workspace" pre-gate, and it runs before the store's lifetime. |
| `npm.rs:158-160` (callers `npm.rs:167`, `npm.rs:214`) | `has_bun_lockfile` via `detect_bun_workspace` / `detect_npm_workspace` | `bun.lock`, `bun.lockb` | Bun-versus-npm authority disambiguation | MP ×1–2 per call. A root with `package.json` and no Bun lockfile costs 4 MP, because both detectors probe both names. | **Yes.** Both callers take `manifests: &ManifestStore` (`npm.rs:162-166`, `npm.rs:202-206`). Nested dispatch passes it too (`nested.rs:587,590`). Add a `manifests` parameter to `has_bun_lockfile`. | **Share.** It asks the same question the Bun layer's observation asks next, at the same layer root with the same candidates. Map `Present` → `true` and `Absent`/`Failed` → `false` to keep today's `exists()` behavior. |
| `npm.rs:249` | `detect_yarn_workspace` | `yarn.lock` | Yarn authority marker | MP ×1 | **Yes**, through the `manifests` parameter (`npm.rs:244-248`, `nested.rs:589`). | **Share.** This is the Yarn layer's own candidate. |
| `npm.rs:410-420` | `resolve_js_package_manager` | `pnpm-lock.yaml`, `yarn.lock`, `bun.lock`, `bun.lockb` | Labels the JS package manager used to parse `package.json` dependency specs | MP ×0–4, short-circuited by `package_managers` and standard | **Reachable but not wired.** The only caller is `create_package_with_request` (`detection.rs:1823`), where `ctx.manifests` is in scope. The signature has no store. | Leave alone in Phase 2. This is a labeling question (R4 spirit), probed per package at `owner_root`. It runs only when `wants_dependencies()`. |
| `detection.rs:1439-1440` | `detect_package_managers` | `pnpm-lock.yaml`, `yarn.lock` | Package-manager labels per package directory | MP ×2 per package, unconditionally | **Reachable but not wired.** The caller `create_package_with_request` (`detection.rs:1771`) has `manifests`. The signature takes only `path`. | **Leave alone.** R4 freezes `detect_package_managers`. It probes member directories, not layer roots, so sharing would only hit on a root package. |
| `detection.rs:397-405` → `manifest_index.rs:39-51` | `ManifestStore::cargo_lock` → `CargoLockVersions::parse` | `Cargo.lock` | Read and parse. Callers: corroboration (`detection.rs:1045-1046`), dependency enrichment (`detection.rs:175`, `detection.rs:1687`) | LR+FO on every miss, **including absent files** (no presence probe first). BR+LP after a successful read. CN per lookup (`normalized_key` → `canonicalize_path`, `detection.rs:1346-1348`). | **Is the store** | Replace with the typed outcome cache, gated by `lockfile_presence`. |
| `detection.rs:407-417` via `read_counted_lockfile` (`detection.rs:476-483`) | `ManifestStore::pnpm_lock`, called from `pnpm_lockfile_matches` (`detection.rs:953-954`) | `pnpm-lock.yaml` | Corroboration | Same as the `Cargo.lock` row: LR+FO even when absent, then BR+LP, plus CN | **Is the store** | Replace with the typed cache, gated by presence. |
| `detection.rs:419-429` via `read_counted_lockfile` | `ManifestStore::uv_lock`, called from `uv_lockfile_matches` (`detection.rs:1000-1001`) | `uv.lock` | Corroboration | Same as the `Cargo.lock` row | **Is the store** | Replace with the typed cache, gated by presence. |
| `path_kind.rs:575-577`, `file_types/registry.rs:73-86` | name classification | many | String matching on names already walked. No filesystem access. | none | n/a | Not a probe. Out of scope. |

Test-only sites (`nested.rs:896,1083,1838`, `detection.rs:3107-3313`,
`manifest_index.rs:878`) are fixtures and assertions, not probes.

### Existing evidence that could answer presence

`RepoEvidence` (`detection.rs:61-71`) carries `manifest_index`,
`manifest_dirs`, `nested_markers`, and `inventory`. None of these records
lockfile paths as such.

`inventory` classifications could prove that a file is **present**, but they
cannot prove it is **absent**, for three reasons:

- they are filtered by `.gitignore`;
- the walk skips directories by name;
- the list is capped at `MAX_FILES` (`system_view.rs:194-204`).

`inventory` is also `None` on every structure request (`detection.rs:131-132`).
The recommendation is to not use it, and to probe metadata directly.

### How `lockfile_presence` avoids double counting

The precedent is `root_config_exists` (`detection.rs:432-441`). On a cache
miss it increments its domain counter (`REPO_ROOT_CONFIG_PROBES`) once and then
calls `probe_exists`, which increments MP once. On a hit it increments
nothing. Mirror that pattern:

1. **One syscall, one MP.** On a miss, `lockfile_presence` increments
   `REPO_LOCKFILE_PROBES` once and MP once. It must **not** call
   `probe_exists`, which would add a second MP. If the implementation follows a
   symlink (`symlink_metadata` then `metadata`), count one MP per logical probe,
   matching `probe_exists`. Otherwise state the two-syscall count explicitly.
2. **Hits are free.** A cached answer increments nothing. The shared detector
   sites (`npm.rs:159`, `npm.rs:249`) then make the layer observation's later
   probe of the same file a hit. Today that probe would be a second MP.
3. **Separate sites stay out of the lockfile counter.** The sites left alone
   (`detection.rs:550`, `detection.rs:1439-1440`, `npm.rs:410-420`) keep
   calling `probe_exists`. They increment MP only, never
   `REPO_LOCKFILE_PROBES`. So `filesystem.repo.lockfile_probes` counts unique
   cached lockfile questions, and MP stays the total number of syscalls. A
   separate site can still stat the same path as the cache, but each stat is
   counted exactly once, under MP only.
4. **Reads come after presence.** The typed read path increments LR only after
   `Present`. An absent lockfile therefore goes from 1 LR (today) to 0 LR plus
   1 probe. The expectation in the existing test
   `absent_cargo_lock_counts_one_read_attempt_and_no_parse`
   (`detection.rs:3114-3127`, which asserts LR=1) must change in Phase 2.
5. **Cache key.** `normalized_key` canonicalizes on every lookup, costing CN
   plus one syscall (`seed.rs:97-99`, `detection.rs:1346-1348`). For a missing
   file it falls back to lexical normalization. Consider keying presence on
   `normalized_key(layer_root).join(name)`, or on the lexical path, so each
   probe does not pay a hidden extra syscall.

## 2. `ManifestStore` today

Definition: `detection.rs:196-215`. The store is created once per detection at
`detection.rs:597`, or at `detection.rs:160` for standalone synthesis. It is
single-threaded, using `RefCell` and `Rc`. All keys are
`normalized_key(path)`, which canonicalizes with a lexical fallback
(`seed.rs:97-99`).

| field (line) | value type | failure cached as |
|---|---|---|
| `cargo` (205) | `Option<Rc<toml::Value>>` | `None`. Absent and parse failure are indistinguishable. |
| `npm` (206), `pyproject` (207), `pnpm_workspace` (208) | `ManifestOutcome<T>` = `Result<Rc<T>, ManifestFailure>` (216) | `ManifestFailure::{Io{kind,..}, Parse}` (218-226) |
| `go_mod` (209), `raw_text` (210) | `Option<Rc<String>>` | `None` |
| `cargo_locks` (211) | `Option<Rc<CargoLockVersions>>` | `None` |
| `pnpm_locks` (212) | `Option<Rc<serde_yaml_ng::Value>>` | `None`. The generic tree is kept whole. |
| `uv_locks` (213) | `Option<Rc<toml::Value>>` | `None`. The generic tree is kept whole. |
| `root_configs` (214) | `bool` | n/a (presence only) |

How the lockfile caches are keyed and populated:

- **`pnpm_locks` and `uv_locks`.**
  - Keyed by `normalized_key(layer.root.join(name))`.
  - Populated only by the private accessors `pnpm_lock` and `uv_lock`
    (`detection.rs:407-429`). Their only callers are `pnpm_lockfile_matches`
    (`detection.rs:954`) and `uv_lockfile_matches` (`detection.rs:1001`).
  - Both read through `read_counted_lockfile` (`detection.rs:476-483`). A read
    error and a parse error both become `None`.
- **`cargo_locks`.**
  - Keyed by `normalized_key(<root>/Cargo.lock)`, where `<root>` is the layer
    root, the standalone root, or `seed.owner_root`.
  - Populated by `cargo_lock` (`detection.rs:397-405`) through
    `CargoLockVersions::parse` (`manifest_index.rs:39-51`).
  - `from_lock_str` (`manifest_index.rs:53-64`) deserializes the typed
    `CargoLockDocument` (`manifest_index.rs:100-117`). It keeps only `name` and
    `version`, and `source` is skipped as `IgnoredAny`. It returns `None` only
    on a read error or invalid TOML.

`CargoLockVersions` (`manifest_index.rs:24-98`) is a
`HashMap<String, Vec<String>>`. `resolve(name)` returns the first version
(`manifest_index.rs:95-97`).

Where it is built and consumed:

| Site | Gate | Use |
|---|---|---|
| `detection.rs:174-178` (standalone synthesis) | `request.wants_dependencies()` | Passed as `lock_versions` to `create_package_with_request` |
| `detection.rs:779-784` → `lock_versions_for_seed` (`detection.rs:1679-1689`) | `request.wants_dependencies()`, and seed standard `CargoWorkspace` or `Unknown` | Per-seed, at `seed.owner_root` |
| `PackageBuildContext.lock_versions` (`detection.rs:505-519`) → `cargo_dependencies_from_value` (`detection.rs:1808`, `cargo.rs:94-160`) | same | Dependency enrichment: `actual_version = lock_versions.resolve(name)` (`cargo.rs:160`) |
| `cargo_lockfile_matches` (`detection.rs:1040-1068`) | `request.wants_lockfile_provenance()` (`detection.rs:749`) | Corroboration: `resolve(name).is_none()` means mismatch |

Gates: `wants_lockfile_provenance` is at `request.rs:850-852` and
`wants_dependencies` at `request.rs:860-862`. The corroboration loop at
`detection.rs:749-753` runs **before** `merge_seeds` (`detection.rs:776`) and
before dependency enrichment (`detection.rs:777-788`). Because both paths go
through the same `cargo_locks` key, each `Cargo.lock` is parsed once per
request.

## 3. R10 completeness signals

Context: `layer.packages` is copied from `outcome.seeds[].relative`
(`topology.rs:66-70`). The corroboration loop at `detection.rs:749-753` has
three values in scope:

- `outcomes`, which is `Vec<DetectorOutcome>`;
- `seeds`, the flat seed list, not yet merged;
- `manifests`.

`MonorepoLayer` is public, and Claudine builds it with a struct literal
(`claudine/lib/src/events/environment.rs:568`). A crate-private field on it
would break that literal. Any new flag must therefore live on crate-private
types.

**(a) A `layer.packages` entry with no resolved seed. No stored signal, but it
can be derived exactly where it is needed.**

- The only evidence is the inline lookup `seeds.iter().find(|s| s.relative ==
  key)`. Each corroborator handles a miss differently:
  - pnpm: `detection.rs:975-983` (`filter_map`) silently drops the entry,
    which turns incompleteness into a smaller set and so into `mismatch`;
  - uv: `detection.rs:1024-1031` does the same;
  - Cargo: `detection.rs:1050` (`?`) returns `None`, which is today's "no
    answer".
- Frames can differ:
  - `collect_outcomes` rebases only the flat copies, so the outcome seeds (and
    therefore `layer.packages`) stay layer-relative (`detection.rs:877-882`);
  - `nested.rs:627` rebases in place;
  - `collect_outcome` does not rebase (`detection.rs:856`).
- **Recommendation:** no new flag. `observe_layer_lockfile` resolves every
  `layer.packages` entry to a seed and treats any miss as
  `incomplete_manifest_discovery`. It must never `filter_map`. This is direct
  evidence, not inference.

**(b) A member manifest that failed to parse. Absent before corroboration.**

- Membership detectors parse only root manifests, through `required_*`
  (`cargo.rs:27`, `uv.rs:24`, `npm.rs:128,176,218,258`). Members are matched
  by marker presence (`glob.rs:31`, `glob.rs:78-84`, `glob.rs:101-108`), so no
  member parse outcome exists when the loop runs.
- The first member parse is inside corroboration, at `detection.rs:1051-1058`
  (Cargo, `manifests.cargo`):
  - its cache is `Option` (`detection.rs:205`), fed by
    `read_counted_manifest`, whose `.ok()?` loses the difference
    (`detection.rs:468-474`);
  - a failure is reported as `Some(false)` mismatch
    (`detection.rs:1060-1062`).
- For `npm` and `pyproject`, the store does keep `ManifestFailure::Parse` and
  `ManifestFailure::Io` (`detection.rs:216-226`). The non-required accessors
  discard it with `.ok()` (`detection.rs:300-309`, `detection.rs:328-338`),
  while `required_npm` and `required_pyproject` expose it as a `SniffError`
  (`detection.rs:315-326`, `detection.rs:343-354`).
- **Recommendation:**
  - change `cargo` to `ManifestOutcome<toml::Value>`, keeping the `cargo()` and
    `required_cargo()` return types;
  - add `pub(crate) fn member_manifest_outcome(&self, kind, path) ->
    ManifestOutcome<..>` (or one `*_outcome` accessor per kind) so the engine
    can map `Err(Parse | Io)` to `incomplete_manifest_discovery`.

  No layer flag is needed, because the failure is observed at the comparison
  site.

**(c) The glob expander reporting its bound. Absent. There is no bound to
report.**

- `expand_membership_globs` (`glob.rs:49-116`) has no count or size cap. It is
  bounded only spatially, by literal-prefix walk roots (`glob.rs:194-200`,
  `resolve_walk_roots`).
- The shared-walk `manifest_dirs` are deliberately never truncated by the
  inventory cap (`system_view.rs:236-240`). The cap applies to `inventory`
  only (`system_view.rs:194-204`).
- The real completeness losses are silent (debug log at most):
  - walker errors dropped at `glob.rs:227` (`filter_map(Result::ok)`);
  - shared-walk entry errors dropped at `system_view.rs:154-156`;
  - a walk root that is not a directory skipped at `glob.rs:206-208`;
  - an invalid glob skipped at `glob.rs:162`;
  - an unsupported Cargo glob rejected at `glob.rs:87-93`.
- **Recommendation:** add `pub(crate) membership_incomplete: bool` to
  `DetectorOutcome` (`topology.rs:27-31`). `expand_membership_globs` sets it by
  returning `(Vec<PackageSeed>, bool)` or a small struct whenever it drops a
  walk error. The corroboration loop finds the layer's outcome by
  `(root, authority)` in `outcomes`.
- Covering errors on the shared-walk path would also need a flag on
  `FilesystemSystemView` and `RepoEvidence`. That is a larger change, so it is
  an owner decision. Whether an invalid or rejected pattern counts as
  incomplete is also an owner decision. Cargo itself refuses such manifests,
  so treating it as incomplete is defensible, but it is not required by R10's
  wording.

## 4. `lockfile_match` occurrences

`rg -n --hidden lockfile_match --glob '!**/_completed/**'` (the hidden flag
catches `.claude/`), grouped by package.

**sniff (lib)**

- `lib/src/filesystem/repo/standard.rs:387-396`: field and docs.
  `standard.rs:2054`: test literal.
- `lib/src/filesystem/repo/topology.rs:77`: constructor.
- `lib/src/filesystem/repo/types.rs:513,522`: docs. `types.rs:914`: literal.
- `lib/src/filesystem/repo/detection.rs:892,912`: doc and assignment.
  `detection.rs:902-904,948,995,1040`: `*_lockfile_matches` names. Tests at
  `detection.rs:3126,3153,3195,3291,3313`.
- `lib/src/filesystem/repo/aggregate_view.rs:171`: comment.
- `lib/src/request.rs:789`: doc link.
- `lib/tests/l1/integration.rs:747,765,797,821,840,856,1080`.
- `lib/tests/l1/lockfile_provenance.rs`: 22 hits, at
  157,186,192,216,309,322,329,460,537,538,599,603,614,626,634,653,661,756,762,765-767.
- `lib/README.md:684,685`.

**sniff-cli**

- `cli/src/output/repo_json.rs:1674,1716,1748,2782`: fixtures.
- `cli/tests/l1/cli.rs:1038,1177,1196`.
- `cli/README.md:491`.

**claudine**

- `claudine/lib/src/events/environment.rs:568`: struct literal.

**repo-level docs and skills**

- `.claude/skills/sniff/performance.md:80`.

**This feature's own specs** (expected): `plan.md` (15 hits) and `spec.md`
(5 hits).

## 5. Counter names

From `sniff/lib/src/performance/counters.rs`:

| constant | string | line |
|---|---|---|
| `FS_FILE_OPENS` | `filesystem.io.file_opens` | 42 |
| `FS_BYTES_READ` | `filesystem.io.bytes_read` | 45 |
| `FS_METADATA_PROBES` | `filesystem.io.metadata_probes` | 48 |
| `FS_READ_DIRS` | `filesystem.io.read_dirs` | 51 |
| `FS_CANONICALIZATIONS` | `filesystem.io.canonicalizations` | 54 |
| `REPO_MANIFEST_PARSES` | `filesystem.repo.manifest_parses` | 62 |
| `REPO_LOCKFILE_READS` | `filesystem.repo.lockfile_reads` | 68 (docs 64-67: counted before the read, so an absent file counts) |
| `REPO_LOCKFILE_PARSES` | `filesystem.repo.lockfile_parses` | 71 (incremented after a successful read, before the parse: `detection.rs:481`, `manifest_index.rs:50`) |
| `REPO_CONFIG_PARSES` | `filesystem.repo.config_parses` | 74 |
| `REPO_ROOT_CONFIG_PROBES` | `filesystem.repo.root_config_probes` | 76 |

`rg -n --hidden 'lockfile_probes|LOCKFILE_PROBES'` outside `_completed` and
`target` finds only `spec.md:543`, `plan.md:249`, and `plan.md:418`.
**`filesystem.repo.lockfile_probes` does not exist yet.**

The docs for `REPO_LOCKFILE_READS` (`counters.rs:64-67`) have two problems to
fix in Phase 2:

- they say "absent ... still counts one attempt", which stops being true once
  presence gates reads (§1, point 4);
- their example list names only three formats.
