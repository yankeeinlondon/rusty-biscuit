---
area: sniff
status: implemented
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-26
review_iterations: 0
clarified: false
implemented: true
implemented_by: claude/opus
created: 2026-09-26
owner: Ken Snyder <ken@ken.net>
origin: review 3 of 2026-09-21-lockfile-provenance-cost
related:
    - 2026-09-21-lockfile-provenance-cost
human_review: false
message_to_agent: |-
    All five phases are implemented; the terminal state is "implementation complete, ready for
    review". Read "## Phase 5" of implementation-log.md first. Facts the commit and review steps need:

    1. Nothing is committed. The breaking-change commit(s) need the `!` marker (ruling R11), e.g.
       `feat(sniff)!:`, and must list the changed expectations recorded in the Phase 2-4 logs.
       Commits must be signed with the author identity, carry no agent trailers, and pass
       `git verify-commit`.
    2. Phase 5 fixed a real bug the corpus pass found on this repository: Cargo corroboration counted
       `[workspace].exclude` directories as members (false `members_missing` for
       `darkmatter/dmls/zed-dmls`). Fix: `lockfile/cargo.rs` skips every path that has an excluded
       seed (the detector's raw seeds hold an included AND an excluded seed for the same directory).
       Regression: L1 `lockfile_isolation::a_cargo_workspace_exclude_is_not_a_missing_member`.
    3. New L1 module `sniff/lib/tests/l1/lockfile_isolation.rs` (isolation, caching, precedence,
       path spellings, counters) and a 24-case complete-JSON matrix in `lockfile_provenance.rs`.
       Its normalizer sorts only leaf-marker layer members, because ext4 walk order differs from
       APFS/NTFS (recorded in the `os` skill).
    4. Pre-existing gaps recorded, not fixed: npm `"./dir/*"` workspace patterns match nothing
       (`sniff/fixes/_unscheduled/npm-dot-slash-workspace-patterns`), and a Cargo layer's
       `packages` lists a directory twice when `members` and `exclude` both match it.
    5. Cross-OS evidence on the final tree: `just cross-check sniff --os all` and
       `just cross-check sniff-cli --os all` pass on Linux, Windows, and WSL.
---

# Lockfile corroboration for every workspace standard

## Problem

The Sniff library recognizes 17 named workspace/build standards plus an
`Unknown` fallback in [MonorepoStandard](../../lib/src/filesystem/repo/standard.rs).
Three named standards are task orchestrators, not independent membership
authorities. Today, layer corroboration reads a lockfile for only three
authorities: `CargoWorkspace`, `PnpmWorkspaces`, and
`UvWorkspace`. Those are the three ecosystems the rusty-biscuit monorepo happens
to use. Nothing in Sniff's scope justifies that choice, and the feature was
never approved in that form.

The result is a signal that means different things for different
repositories:

- `NpmWorkspaces`, `YarnWorkspaces`, and `BunWorkspaces` are never
  corroborated, although `package-lock.json`, Yarn Berry's `yarn.lock`, and
  `bun.lock` all record workspace members.
- The Sniff library's [MonorepoLayer](../../lib/src/filesystem/repo/standard.rs),
  which describes one workspace and its members, has a `lockfile_match` field
  that is `None` both when a standard is never checked and when the lockfile is absent or unparseable. A consumer cannot
  tell "we do not check this ecosystem" from "your lockfile is missing".
- The comparison is not uniform: Cargo accepts extra lockfile entries, while
  pnpm and uv require an exact member set.

## Decisions

These were made by the owner on 2026-09-26.

1. **Coverage.** Support JavaScript/TypeScript, Go, Rust, PHP, and Python with a
   defined fallback. Corroborate every standard whose lockfile records
   workspace membership; report a truthful fallback status for everything else.
2. **Go, Poetry, PDM, and PHP get fallback status only.** Report
   `unverifiable` with the observed lockfile path rather than claiming that
   these dependency records prove workspace membership. The reporting location
   for standalone Python and PHP projects remains an open scope decision below.
3. **Replace the public API outright.** A structured per-layer lockfile
   observation replaces `lockfile_match: Option<bool>`. This is a breaking
   change to the library API and to the CLI's JSON. Document it in the READMEs
   and the Sniff skill. There is no deprecation period.
4. **This is a feature, separate from `2026-09-21-lockfile-provenance-cost`.**
   That fix keeps its cost work (the opt-in request setting and the typed Cargo
   parser). The typed pnpm parser written in response to its third review moves
   here as [pnpm-typed-parser.patch](pnpm-typed-parser.patch).

## Review notes and scope boundaries

This review preserves the owner's immediate API replacement and request-cost
split. It corrects assumptions about what lockfiles prove; it does not authorize
new workspace standards. The major choices under **Open questions** must be
resolved before finalizing the implementation plan. Recommendations there are
not additional owner decisions.

The manifest remains the membership authority. Reading a lockfile must never
add or remove packages, change workspace ownership, or repair files. Observe
the current filesystem, including uncommitted files; the existing source
comments calling these files “committed” are inaccurate and must be corrected
when the implementation changes. No Git blob reads, network access, package
manager execution, build-script evaluation, or recursive lockfile search is
allowed for corroboration.

## Lockfile observation

Replace the Sniff library's layer `lockfile_match` field with a required
`lockfile` object. It is always serialized, including when content reads are
disabled. This is a new public observation type in the same library, not logic
implemented by the CLI.

| Status | Meaning |
|---|---|
| `match` | A supported format provides a complete member set, and it equals the manifest-derived set |
| `mismatch` | A complete member set was recovered and differs from the manifest-derived set |
| `unverifiable` | The observed file or configuration cannot establish a complete member set; a reason explains why |
| `unreadable` | A required metadata probe, read, or supported-format parse failed |
| `absent` | All applicable, selected lockfile candidates were probed and are missing |
| `not_applicable` | The authority has no applicable lockfile source in the supported configuration |
| `not_requested` | A candidate exists, but the request disabled content-based corroboration |
| `members_present` | Cargo only (ruling R1): every manifest member has a matching source-less lockfile entry; set equality is not claimed |
| `members_missing` | Cargo only (ruling R1): at least one manifest member has no matching source-less lockfile entry |

The object has these fields, always serialized with the indicated empty value:

| Field | Contract |
|---|---|
| `status` | One of the snake-case values above |
| `paths` | Sorted, unique lockfile paths relative to the layer root; `[]` when none is known to exist |
| `reason` | Stable machine-readable reason, or `null` for ordinary match/mismatch/absence; distinguish unsupported version, unsupported layout, no membership data, ambiguous membership, and metadata/read/parse failure |
| `extra` | Sorted, unique layer-relative member paths present only in the lockfile; `[]` unless `mismatch` (always `[]` for Cargo) |
| `missing` | Sorted, unique layer-relative member paths present only in the manifest; `[]` unless `mismatch` or `members_missing` |

The complete `reason` vocabulary is frozen by ruling R5 under **Owner
rulings**. Use `request_disabled` for `not_requested`, and distinguish
`no_lockfile_source` from `unknown_standard` for `not_applicable`. An optional
human-readable diagnostic may include the failed candidate path, but must not
include lockfile contents or be needed to interpret `reason`. Final Rust type
names are implementation details; these JSON names and meanings are the
contract. A list of paths permits a recognized lockfile group without another
API break; it does not authorize merging unrelated package managers' files.

For example, a pnpm lockfile that omits one current member reports:

```json
{
  "status": "mismatch",
  "paths": ["pnpm-lock.yaml"],
  "reason": null,
  "extra": [],
  "missing": ["packages/ui"]
}
```

### Repository-level standalone lockfiles

Ruling R2 adds `standalone_lockfiles` to the repository result for Poetry, PDM,
and Composer lockfiles that no workspace layer covers. Each entry has these
fields, always serialized by the library:

| Field | Contract |
|---|---|
| `root` | Repository-relative, `/`-separated root the file was found at; `""` for the repository root |
| `tool` | `poetry`, `pdm`, or `composer` |
| `status` | `not_requested`, `unverifiable`, or `unreadable`; an absent file produces no entry |
| `paths` | Sorted lockfile paths relative to `root` |
| `reason` | `request_disabled`, `no_membership_data`, or `metadata_failed` |
| `extra`, `missing` | Always `[]` |

The library always serializes the list (`[]` when empty); the CLI omits an
empty list under the rule it applies to `monorepo_layers`.

### Selection and state precedence

Use the already-selected layer authority; do not select a different authority
because its lockfile produces a more favorable answer. Each authority declares
its bounded candidate paths and precedence. Cache metadata outcomes within the
request and reuse existing filesystem evidence when available.

1. No applicable source yields `not_applicable`, with no content read.
2. Probe only the applicable candidates. A permission or other metadata error
   is `unreadable`, not `absent`; a file that disappears before opening is also
   `unreadable`. A directory where a file is expected is a read failure.
3. If all selected candidates are missing, report `absent`. An intentionally
   configured but unsupported layout is `unverifiable`, not guessed absent.
4. A present candidate with corroboration disabled yields `not_requested`,
   even for a known fallback format. Do not inspect its contents to distinguish
   versions. Metadata errors still take precedence.
5. With corroboration enabled, a recognized fallback source needs no content
   read and yields `unverifiable`. Otherwise parse once and classify its version
   and membership data before comparing sets.

A syntactically invalid document or invalid required membership fields in a
supported format yields `unreadable`. A recognizable unsupported version yields
`unverifiable`; never interpret an unsupported shape as an empty workspace.
Historical formats without a numeric version need an explicit format signature.
Do not retry a lower-priority file after the selected file fails or has an
unsupported version. Unsupported configuration may have an empty `paths` list
when no concrete file can safely be selected.

### Membership and provenance

Compare member paths relative to the layer root, excluding the root itself from
both sets. This deliberately changes the current uv root-inclusion convention
for corroboration only; keep root packages in the package catalog. Empty sets
can match when both were established from valid, complete membership records.
Missing membership fields are not empty sets.

Paths are normalized by components, with `/` in serialized output. Normalize
`.` components and trailing separators without stripping leading dots from
names such as `.tools`. Resolve `..` against the format's documented base;
Rush importer paths may use a different base from the stored lockfile's parent.
Preserve case, do not require a stale lockfile member to exist, and do not
canonicalize each member through the filesystem. Never open paths taken from
lockfile entries. Reject absolute or unrepresentable member paths as
`unverifiable` rather than silently dropping them. Legitimate external members,
where the existing detector supports them, retain relative `..` components.
Use the repository's OS guidance for converting native paths and separators.

Do not intersect lockfile paths with current manifest members before comparison:
that would erase extra members and falsely report a match. Deduplicate valid
member identities, but reject conflicting identities or duplicate required
mapping keys. Incomplete manifest discovery or an ambiguous name-to-path mapping
cannot produce `match` or `mismatch`; report `unverifiable` with a reason.

The Sniff library's [PackageProvenance](../../lib/src/filesystem/repo/standard.rs)
records the evidence supporting a package's membership. Only an exact `match`
upgrades a layer and the packages it owns to `Lockfile`. Other statuses retain
manifest/inferred provenance. Nested or overlapping layers must not overwrite
the provenance of packages owned by another authority. Repeat calls with a
cheaper request must not inherit upgrades from an earlier call.

> **Reader's note:** Uniform path equality cannot be applied directly to Cargo.
> Its lockfile identifies packages, not their workspace paths, and local
> nonmembers can also lack `source`. The present name lookup even accepts
> registry packages with the same name. The Cargo choice under **Open questions**
> must be settled explicitly; neither the current lookup nor a source-less
> superset may silently become an exact `match`.

## Coverage by standard

“Compare” below means that a supported, unambiguous file can produce `match` or
`mismatch`; it does not promise those results for every version or layout.
All rows also obey the absence, error, and disabled-request rules above.

| Standard | Candidate source | Behavior with corroboration enabled |
|---|---|---|
| `PnpmWorkspaces` | `pnpm-lock.yaml` | Compare supported `importers` keys |
| `NpmWorkspaces` | `npm-shrinkwrap.json`, otherwise `package-lock.json` | Compare supported v2/v3 workspace records; v1 is `unverifiable` |
| `YarnWorkspaces` | `yarn.lock` | Compare supported Berry workspace resolutions; Classic is `unverifiable` |
| `BunWorkspaces` | `bun.lock`, otherwise `bun.lockb` | Compare supported text workspace records; binary is `unverifiable` |
| `CargoWorkspace` | `Cargo.lock` | Exact membership is not recoverable directly; `members_present` or `members_missing` per ruling R1, never `match`/`mismatch` |
| `UvWorkspace` | `uv.lock` | Compare supported member identities mapped to recorded local package paths |
| `RushStack` | Manager/configuration-selected lockfiles | Compare the ordinary single pnpm workspace layout after translating importer paths; every other layout is `unverifiable` + `unsupported_layout` (ruling R3) |
| `GoWorkspace` | `go.work.sum` | `unverifiable`; checksums do not enumerate workspace members |
| `GradleMultiProject` | Root `gradle.lockfile` or legacy root `gradle/dependency-locks/` files | `unverifiable`; dependency locks do not establish project membership |
| `Bazel` | `MODULE.bazel.lock` when Bzlmod applies | `unverifiable`; external module resolution is not local package membership |
| `MavenMultiModule`, `DotNetSolution`, `Pants`, `Buck2` | No built-in membership lockfile source | `not_applicable`; this does not claim that plugins or dependency resolvers cannot write lockfiles |
| `Nx`, `Turborepo`, `Lerna` | No independent authority lockfile | No synthetic layer; the underlying membership authority supplies the observation |
| `Unknown` | No known membership authority | `not_applicable` with `unknown_standard` |

For legacy Gradle locks, enumerate only the known root lock directory and report
actual lockfiles, not the directory as a file. Do not recursively inspect
subprojects or evaluate Gradle code. A missing root source does not assert that
no custom/subproject locks exist. Likewise, Go's absent checksum file is an
observation, not a claim that the workspace is invalid.

### Format-specific corrections and evidence

- **npm:** Paths outside `node_modules` are not automatically workspaces:
  local link targets are recorded too. Use the locked root workspace declarations
  and package/link records to establish the locked workspace set, including
  deleted members. If the accepted version cannot distinguish a local dependency
  from a workspace, report `unverifiable`. Do not scan installed `node_modules`.
  File precedence and link records are documented by
  [npm](https://docs.npmjs.com/cli/v11/configuring-npm/package-lock-json/).
- **Yarn:** Read canonical workspace paths from resolved workspace identities,
  not by splitting every descriptor at the first `@`. Scoped names and combined
  descriptors such as `workspace:^` occur in
  [Yarn's own lockfile](https://raw.githubusercontent.com/yarnpkg/berry/master/yarn.lock).
  Detect Berry's metadata version and exclude ordinary `link:`/`portal:` targets.
- **Bun:** The text format is JSONC, so strict JSON parsing is insufficient.
  Support its permitted comments and trailing commas without a hand-written
  comment stripper. Record the accepted lockfile versions; binary fallback must
  not invoke Bun to convert it. See [Bun's lockfile documentation](https://bun.sh/docs/pm/lockfile).
- **uv:** The current Sniff implementation reads `[workspace].members`, while
  the draft proposed `[manifest].members`. Do not preserve the former merely
  because synthetic tests use it. Establish supported layouts from generated
  fixtures; member names, where recorded, must be resolved through local package
  source records such as `editable` or `virtual`. Handle single-member/root
  special cases explicitly, and do not treat every local dependency as a member.
  uv expressly does not promise a stable lockfile format; see its
  [metadata documentation](https://docs.astral.sh/uv/reference/internals/metadata/).
  Using an external uv command instead would violate this feature's local,
  passive observation contract.
- **Rush:** Reusing the pnpm parser is only the syntax step. Configuration chooses
  the manager and importer base, and
  [subspaces](https://rushjs.io/pages/advanced/subspaces/) can use separate
  lockfiles. Legacy non-workspace installs and variants must not be interpreted
  as the ordinary pnpm layout. See the Rush open question.
- **Bazel:** The original “no lockfile” row was too broad:
  [Bazel documents a module lockfile](https://bazel.build/external/lockfile).
  Its presence warrants fallback reporting, not workspace corroboration.

Before implementation, record an explicit accepted-version matrix with exact
producer versions and checked-in generated fixtures for each supported layout.
A moving upstream example is evidence for this review, not a pinned test input.
The matrix must include pnpm and uv as well as the newly added parsers. Unknown
versions must have tests proving they cannot produce a mismatch.

## Request costs and parser requirements

The Sniff library's [RepoRequest](../../lib/src/request.rs) selects the work
performed by repository detection. Preserve its existing behavior: structure
and focused constructors disable corroboration, full enables it, the builder
can override either, and serialized requests omitting `lockfile_provenance`
still enable it for compatibility. This feature changes result data, not those
request defaults.

Presence probes now occur even when corroboration is disabled. That is an
intentional bounded cost increase over `2026-09-21-lockfile-provenance-cost`:
reuse captured evidence, count metadata work, and add no descendant walk.
Disabling corroboration forbids content reads *for this purpose*. Dependency
version enrichment may independently read Cargo.lock; it must reuse the same
request cache without upgrading provenance. Zero lockfile reads is required
for structure requests with corroboration disabled, not every full request
that disables it.

Extend the Sniff library's request-local
[ManifestStore](../../lib/src/filesystem/repo/detection.rs), which shares
manifest and lockfile results across detection, to retain typed success,
absence, unsupported-format, and failure outcomes. Cache failures as well as
successes. A selected file is opened and parsed at most once per request even
when multiple projections or dependency enrichment need it. Do not introduce
cross-request caching that can hide edited files.

Every production parser retains only fields needed for membership, format
recognition, and existing dependency-version consumers. It still validates
syntax through the end of the document. Do not allocate a generic tree for an
entire lockfile. The supplied [pnpm parser patch](pnpm-typed-parser.patch) is a
starting point, not a drop-in solution: it lacks version recognition, collapses
errors into an optional value, and drops non-string importer keys. This feature
must reject invalid required membership keys instead of silently filtering them.
Its deliberate tolerance for duplicate keys in ignored dependency sections may
remain, provided tests document that difference from the generic reference.
Do not require validation of every ignored dependency field's business meaning.

The Sniff library's [work counters](../../lib/src/performance/counters.rs)
measure actual work: lockfile reads count attempted content opens; parses count
parser invocations after successful reads; metadata probes are separate. Count
unsupported-version parsing when it occurs. Metadata-only fallback performs no
content read or parse. Preserve file-open and byte counters at their existing
shared read points and propagate collection into any workers.

Measure release-mode parsing time and retained/peak allocation behavior for
large JSON, JSONC, YAML, and TOML fixtures. Record input size, tool version,
request shape, host, repetitions, and cache conditions under the
`2026-09-20-repo-perf` protocol. Use work counters for stable regression assertions;
wall-clock thresholds do not belong in unit tests.

## CLI and compatibility

The Sniff CLI reports library observations and does not rediscover lockfiles.
Include the object wherever layers are serialized, including consolidated
repository JSON. Human output must identify the layer, status, selected paths,
and extra/missing members; fallback and skipped states need a plain-language
explanation. Render through `biscuit-terminal` components, considering `Prose`,
and preserve the existing plain-text and terminal fallback behavior.

Repository facts, including mismatches and unreadable observations, are main
content on stdout. CLI hints belong on stderr. JSON stdout must be exactly one
valid data document, with no hints or log lines mixed in. An observation such as
`mismatch` or `unreadable` does not itself change the command's exit status;
existing fatal detection errors keep their existing behavior.

Update all library constructors, serialized fixtures, CLI projections, and
in-repository consumers of the old field. Document removal of `lockfile_match`,
the required replacement object, root-exclusion behavior, and the resolved
Cargo semantics in the library/CLI READMEs, public docs, and Sniff skill.
Do not fabricate a new observation when deserializing old result JSON: old
payloads without the required object are incompatible by design. Request JSON
compatibility remains unchanged. If parser crates are added or removed, update
the root and area dependency documentation.

## Acceptance criteria

1. A Level 1 fixture matrix covers every authority and applicable status,
   including `Unknown`. Orchestrator-only fixtures prove that no artificial
   membership layer is created. Assert the complete serialized repository result
   using the Sniff library's existing
   [lockfile provenance tests](../../lib/tests/l1/lockfile_provenance.rs) as
   the integration pattern, updating their old Cargo/uv expectations explicitly.
2. Each accepted format/version has at least one fixture written by a recorded
   real tool version. Generate fixtures during development; normal tests run
   offline without those tools. Test stale extra members, missing members,
   unrelated local dependencies, malformed trailing content, unsupported
   versions, root-only/virtual roots, scoped names, duplicate keys, and missing
   required fields. Generic reference parsers check extraction parity, but real
   fixtures and independently specified expected sets prove membership meaning.
3. Nested layers, shared lockfile consumers, and repeat requests prove ownership
   isolation and request-local caching. Include filename precedence, read and
   metadata failures, symlink spellings, `.hidden` paths, and Windows-native
   path conversion. Permission-failure tests must be deterministic across OSes,
   not rely solely on Unix mode bits.
4. Disabled structure requests show zero content reads/parses and no added
   recursive walk. Enabled requests count each selected read/parse once;
   failures are cached. Test full dependency enrichment with corroboration
   disabled separately, because its reads remain authorized.
5. Exercise the shipped CLI on disposable repositories in both JSON and human
   output modes, using the existing Sniff CLI fixture harness. Assert valid JSON,
   stdout/stderr separation, unchanged success exit behavior, and all relevant
   layer projections. No live developer checkout or global package manager
   installation is a runtime test dependency.
6. Record parser measurements and passive corpus results. Check documentation
   and every old-field call site. `just test` and `just lint` pass in `sniff/`;
   also run all-target clippy with warnings denied for `sniff` and `sniff-cli`,
   which the area lint recipe alone does not cover. Run affected Darkmatter and
   Claudine tests and dependency-derived compile checks, not workspace-wide
   gates. Verify affected packages on macOS, Linux, native Windows, and WSL2,
   reusing qualifying evidence under existing repository policy. This feature
   does not change CI event scheduling or introduce additional CI matrix cells.

## Out of scope

- New Composer, Poetry, or PDM workspace authorities.
- Resolving dependencies, judging dependency freshness/security, or reproducing
  a package manager's full lockfile validation.
- Repairing or generating lockfiles during detection.
- Reading installed dependency trees or executing package-manager commands.
- Changing package-manager detection without resolving its scope question below.

## Owner rulings

> **Adopted default — override before Phase 2.** The plan runs with
> `yolo: true`, so each ruling below adopts the recommended option of the
> matching open question, with a concrete wire shape. The owner may override
> any ruling before Phase 2 starts. An override of R1, R2, or R5 changes the
> wire contract, so it must update this spec and the plan first. This spec
> stays `draft-spec` until the owner confirms.

### R1: Cargo reports partial evidence under separate statuses

- Two Cargo-only statuses are added: `members_present` and `members_missing`.
  `match` and `mismatch` keep their exact set-equality meaning on every
  ecosystem, and Cargo never produces either one.
- A member counts as present when `Cargo.lock` has a `[[package]]` entry with
  the member's name, the member's resolved manifest version (including
  `version.workspace = true` inheritance), and **no** `source`.
- `members_missing` lists the absent members, as layer-relative paths, in
  `missing`. `extra` is always `[]` for Cargo, because stale extra members
  cannot be recovered from `Cargo.lock`. `reason` is `subset_only` on both
  statuses, so no consumer can read either one as equality.
- Neither status upgrades provenance. Only `match` upgrades to `Lockfile`.
- The `Cargo.lock` parse keeps `source`. The dependency-version lookup
  (`CargoLockVersions::resolve`) keeps its current results byte for byte,
  proven by a parity test.
- A `Cargo.lock` without a numeric `version` key is v1 or v2. Classify it by
  its format signature, recorded in `accepted-versions.md`.

### R2: Standalone Python and PHP lockfiles are repository-level observations

- **Wire location:** `RepoInfo.standalone_lockfiles`, a list that is always
  serialized by the library (`[]` when empty). The CLI JSON projection omits
  the empty array under the same rule it applies to `monorepo_layers`.
- **Entry fields:** `root` (repository-relative, `/`-separated, `""` for the
  repository root), `tool` (`poetry` | `pdm` | `composer`), plus the same
  `status`, `paths`, `reason`, `extra`, and `missing` fields as a layer's
  `lockfile` object. `paths` is relative to `root`.
- **Roots probed:** the requested project root and every already-discovered
  package root. Reuse manifest-index evidence where it exists. Add no
  descendant walk and no `vendor/` or `.venv` search.
- **Candidates:** `poetry.lock`, `pdm.lock`, `composer.lock`.
- **Absence:** an absent file produces **no entry**.
- **Present file:** `not_requested` with `request_disabled` when corroboration
  is disabled, otherwise `unverifiable` with `no_membership_data`. A metadata
  error produces `unreadable` with `metadata_failed`.
- **Deduplication:** entries are unique by `(root, tool)`. A root that is also
  a workspace layer root still gets a standalone entry, because no layer
  authority covers these tools.
- Standalone Go modules (`go.sum`) are out of scope. Decision 2 covers Go only
  through `GoWorkspace` layers.

### R3: Rush compares only the ordinary single pnpm workspace layout

- The supported layout requires all of the following:
  - `rush.json` declares `pnpmVersion`;
  - subspaces are not enabled (`common/config/rush/subspaces.json` is absent
    or sets `subspacesEnabled: false`);
  - the pnpm configuration does not disable workspaces (`useWorkspaces`);
  - no variant directory exists under `common/config/rush/variants/`.
- In that layout the candidate is `common/config/rush/pnpm-lock.yaml`, and its
  importer keys resolve against `common/temp`.
- Every other layout reports `unverifiable` with `unsupported_layout` and any
  confidently identified paths: `npmVersion` and `yarnVersion` managers,
  enabled subspaces, the legacy non-workspace install, and variants.
- The Rush configuration files are manifests, not lockfiles. They are read only
  when corroboration is enabled, and each read counts as a manifest parse.

### R4: Package-manager detection stays separate

The uv-labeled-as-pip gap is tracked as the unscheduled fix
`package-manager-uv-label`. This feature does not change
`detect_package_managers`.

### R5: Status and reason vocabulary (frozen wire contract)

- **`status`** (snake_case): `match`, `mismatch`, `members_present`,
  `members_missing`, `unverifiable`, `unreadable`, `absent`,
  `not_applicable`, `not_requested`.
- **`reason`** (snake_case):

  | Reason | Used with |
  |---|---|
  | `request_disabled` | `not_requested` |
  | `no_lockfile_source` | `not_applicable` |
  | `unknown_standard` | `not_applicable` |
  | `unsupported_version` | `unverifiable` |
  | `unsupported_layout` | `unverifiable` |
  | `no_membership_data` | `unverifiable`, including fallback formats and binary `bun.lockb` |
  | `ambiguous_membership` | `unverifiable`, when the name-to-path mapping is ambiguous |
  | `incomplete_manifest_discovery` | `unverifiable` |
  | `invalid_member_path` | `unverifiable`, for absolute or unrepresentable paths |
  | `metadata_failed` | `unreadable` |
  | `read_failed` | `unreadable`, including a directory where a file is expected and a file that vanished between probe and read |
  | `parse_failed` | `unreadable`, for invalid syntax or invalid required membership fields |
  | `subset_only` | Cargo `members_present` and `members_missing` |

- `reason` is `null` for `match`, `mismatch`, and `absent`.
- The wire object has no human-readable diagnostic field. Failure detail (the
  candidate path and the error kind, never file contents) goes to
  `tracing::debug!` only.

### R6: `paths` contains only the selected source

- `npm-shrinkwrap.json` beats `package-lock.json`, and `bun.lock` beats
  `bun.lockb`. Only the winning file appears in `paths`.
- Legacy Gradle is the one multi-file group: `paths` lists each concrete
  `*.lockfile` directly under the root `gradle/dependency-locks/`.
- When the selected file fails or has an unsupported version, no
  lower-priority file is tried.

### R7: Authority selection is not revisited

Corroboration uses `layer.authority` as already selected. Nx, Turborepo, and
Lerna never produce a layer of their own.

### R8: Counter vocabulary

- `filesystem.repo.lockfile_probes` is added for lockfile metadata probes.
- The existing lockfile read and parse counters keep their meanings: reads
  count attempted content opens, and parses count parser invocations after a
  successful read.
- The shared file-open, bytes-read, and metadata-probe counters stay at their
  shared increment points.
- Rush configuration reads count as manifest parses.

### R9: Deterministic failure injection

- A read failure is injected with a directory in place of the lockfile, which
  fails on every OS.
- A metadata failure is injected through a `#[cfg(test)]` probe seam in the
  request-local store, because Unix mode bits are not portable.
- Unix permission tests may be extras, but none is the only proof of a
  behavior.

### R10: Signal for incomplete manifest discovery

A layer's manifest-side set is incomplete when any of these holds: a declared
member has no resolved package, a member manifest failed to parse, or the glob
expander reported its bound. Where no existing signal covers one of these, add
a crate-private flag on the layer outcome. Never infer completeness.

### R11: Breaking-change commit convention

Library and CLI commits that remove `lockfile_match` use the `!` conventional
commit marker (for example `feat(sniff)!:`) so release tooling bumps the
version correctly.

## Open questions

> Each question below is answered by an adopted-default ruling above: Cargo by
> R1, standalone Python and PHP by R2, Rush by R3, and package-manager
> detection by R4. The option analysis is kept as the record of why.

### How should Cargo report useful but incomplete evidence?

Cargo.lock does not provide member paths or distinguish workspace packages from
all other local packages. Exact path equality therefore cannot be implemented
from this file alone. The current shared Cargo parser also discards `source`,
so reusing its name lookup would retain false-positive matches.

- **Report `unverifiable` for present Cargo lockfiles.** Pros: preserves the
  exact meaning of `match`, requires no invented membership evidence, and keeps
  the API small. Cons: removes a previously useful positive signal and no longer
  upgrades Cargo provenance.
- **Add a separate partial-evidence result, recommended.** Check expected
  members by name, resolved version, and absent source; report “expected members
  present” or missing expected members without claiming set equality. Pros:
  retains useful Rust coverage while making its limit visible. Cons: needs an
  additional status/comparison basis and cannot identify stale extra workspace
  members; it must not upgrade provenance reserved for exact `match`.
- **Keep Cargo's subset check under `match`, with a comparison-basis field.**
  Pros: closest to current positive reporting. Cons: makes `match` mean different
  things by ecosystem, permits consumers to overstate certainty, and conflicts
  with the uniform equality goal.

Recommend separate partial evidence because the owner requested Rust coverage
and truthful fallback. Approval must settle the status/JSON shape and provenance
behavior before implementation; do not silently change the `match` contract.
The shared dependency-version lookup must retain its current behavior even if
membership validation becomes stricter.

### Where do standalone Python and PHP fallback observations belong?

Poetry, PDM, and Composer do not currently create workspace layers. A per-layer
field cannot satisfy the owner's requested coverage for these tools.

- **Add repository-level observations, recommended.** Probe known lockfile names
  at the requested project root and already-discovered package roots, retaining
  each root/tool/path association and deduplicating shared files. Pros: covers
  standalone and mixed repositories without false workspace layers. Cons: expands
  the public repository result and requires explicit request-cost and CLI rules.
- **Defer standalone fallback to a separate feature.** Pros: keeps this change
  confined to layers. Cons: explicitly leaves the owner's Python/PHP coverage
  unfinished and requires approval to narrow scope.
- **Create synthetic layers for these lockfiles.** Pros: reuses the layer field.
  Cons: a lockfile does not prove a workspace; this violates existing membership
  rules and misleads package ownership consumers.

Recommend repository-level observations because they satisfy coverage without
inventing membership. Before planning, specify their wire location, root/path
base, opt-out behavior, and absence semantics. Do not claim this coverage is
complete while leaving it only in prose or relying on a future Composer standard.

### Which Rush configurations must the first implementation compare?

Rush can use different managers, variants, and subspaces. A single hard-coded
pnpm path cannot cover them, and importer paths need translation from Rush's
installation layout into project paths.

- **Compare the ordinary single pnpm workspace layout; explicitly fall back for
  other layouts, recommended.** Pros: delivers useful support with bounded reads
  and fixtures; unsupported configurations remain visible. Cons: requires owner
  agreement that truthful fallback satisfies initial Rush coverage.
- **Support all documented manager, variant, and subspace layouts immediately.**
  Pros: widest coverage. Cons: needs selection rules for variants, several parser
  integrations, per-file failure reporting, and a rule for combining complete
  subspace sets; substantially enlarges this feature.

Recommend the bounded first option. Detect configuration from existing manifests;
never guess a single lockfile or report a partial subspace union as complete.
The accepted layout must have a real Rush-generated fixture with verified path
translation. Known but unsupported configurations report `unverifiable` with
`unsupported_layout` and any confidently identified paths.

### Should package-manager detection join this change?

The Sniff library's
[detect_package_managers](../../lib/src/filesystem/repo/detection.rs) reports
package tooling independently of layer corroboration, with narrower coverage
that currently identifies a uv project as pip.

- **Keep it separate, recommended.** Pros: preserves this feature's focus on
  membership evidence and avoids unrelated changes to package lists. Cons: the
  existing tooling-label gap remains and needs a separately tracked fix.
- **Expand detection here using shared format metadata.** Pros: could make tool
  labels consistent with fallback observations. Cons: couples per-package tool
  selection to workspace lockfile reporting and widens tests and public behavior.

Recommend a separate fix: tool identity and evidence of workspace membership are
different questions. Share metadata only where their selection rules actually
agree, rather than forcing both through one ecosystem table.
