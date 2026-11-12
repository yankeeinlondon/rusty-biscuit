---
kind: plan
total_phases: 5
created: 2026-11-08
phase: 6
agent: codex/gpt-6.1-sol
yolo: 'true'
source_files_during_phase_1:
- scripts/ci/distribution.py
- scripts/ci/test_distribution.py
- scripts/ci/requirements-distribution.txt
- dist-workspace.toml
- Cargo.toml
- biscuit-hash/cli/Cargo.toml
- biscuit-file/cli/Cargo.toml
- tree-hugger/cli/Cargo.toml
- worktree/cli/Cargo.toml
- sniff/cli/Cargo.toml
- darkmatter/cli/Cargo.toml
- darkmatter/dmls/Cargo.toml
- claudine/cli/Cargo.toml
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
- biscuit-hash-cli
- biscuit-file-cli
- tree-hugger-cli
- git-wt-cli
- sniff-detection-cli
- darkmatter-cli
- darkmatter-lsp
- claudine-cli
- repo-deps
- test-toolkit
source_files_during_phase_2:
- scripts/ci/distribution.py
- scripts/ci/test_distribution.py
- scripts/ci/fixtures/distribution/dist-plan-biscuit-hash-cli.json
- scripts/ci/fixtures/distribution/dist-plan-darkmatter-cli-brew.json
- dist-workspace.toml
- about.toml
- .github/distribution/build-images.json
- scripts/distribution/about.hbs
- scripts/distribution/build.sh
- scripts/distribution/inspect_archive.py
- scripts/distribution/installers.py
- scripts/distribution/licenses.sh
- scripts/distribution/native_notices.py
- scripts/distribution/test_inspect.py
- scripts/distribution/test_installers.py
- scripts/distribution/test_licenses.py
- scripts/distribution/fixtures/biscuit-hash-cli-installer.sh
- scripts/distribution/fixtures/dist-sha256.sum
- scripts/distribution/fixtures/notices/header.md
- scripts/distribution/fixtures/otool-libraries-bh-aarch64-apple-darwin.txt
- scripts/distribution/fixtures/otool-load-commands-bh-aarch64-apple-darwin.txt
- scripts/distribution/fixtures/readelf-bh-aarch64-unknown-linux-gnu.txt
docs_updated_during_phase_2: []
docs_created_during_phase_2:
- features/2026-10-28-publish/evidence/phase-2/README.md
- features/2026-10-28-publish/evidence/phase-2/license-review.md
skills_files_updated_during_phase_2:
- .claude/skills/rust-devops/deployment-platforms.md
- .claude/skills/os/macos.md
source_files_during_phase_3:
- .github/workflows/distribute-binaries.yml
- .github/actions/setup-dist/action.yml
- scripts/ci/distribution.py
- scripts/ci/test_distribution.py
- scripts/distribution/install_check.sh
- scripts/distribution/tap.py
- scripts/distribution/publish-tap.sh
- scripts/distribution/test_tap.py
- scripts/distribution/github-web-flow.gpg
- scripts/distribution/fixtures/formula/darkmatter-cli.rb
- scripts/distribution/verify_binstall.py
- scripts/distribution/test_verify_binstall.py
- scripts/distribution/fixtures/binstall/Cargo.toml.in
- tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
- justfile
- just/distribution.just
- scripts/Cargo.toml
- scripts/ci/affected_scope.py
- scripts/ci/test_affected_scope.py
- scripts/ci/distribution.py
- scripts/ci/test_distribution.py
- scripts/distribution/run_suites.py
- scripts/distribution/pinned-python.sh
- scripts/distribution/verify.sh
- scripts/distribution/verify_runtime.py
- scripts/distribution/test_verify_runtime.py
- scripts/distribution/inspect_archive.py
- scripts/distribution/installers.py
- scripts/distribution/licenses.sh
- scripts/distribution/native_notices.py
- scripts/distribution/tap.py
- scripts/distribution/verify_binstall.py
- scripts/distribution/test_inspect.py
- scripts/distribution/test_installers.py
- scripts/distribution/test_licenses.py
- scripts/distribution/test_tap.py
- scripts/distribution/test_verify_binstall.py
- tools/test-toolkit/Cargo.toml
- tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_4:
- .github/ci/README.md
docs_created_during_phase_4:
- features/2026-10-28-publish/evidence/phase-4/README.md
- features/2026-10-28-publish/evidence/phase-4/verify/README.md
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
- .github/workflows/distribute-binaries.yml
- .github/distribution/rollout.json
- scripts/ci/distribution.py
- scripts/ci/test_distribution.py
- scripts/ci/affected_scope.py
- scripts/ci/test_affected_scope.py
- scripts/ci/fixtures/distribution/dist-plan-sniff-detection-cli-brew.json
- scripts/distribution/inspect_archive.py
- scripts/distribution/test_inspect.py
- scripts/distribution/installers.py
- scripts/distribution/native_notices.py
- scripts/distribution/test_licenses.py
- scripts/distribution/verify.sh
- tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_5:
- docs/releases.md
- docs/topics/ci-cd.md
- .github/ci/README.md
- biscuit-hash/README.md
- biscuit-file/README.md
- tree-hugger/README.md
- worktree/README.md
- sniff/README.md
- darkmatter/README.md
- claudine/README.md
docs_created_during_phase_5:
- docs/distribution.md
- features/2026-10-28-publish/evidence/phase-5/plans/README.md
- features/2026-10-28-publish/evidence/phase-5/verify/README.md
skills_files_updated_during_phase_5:
- .claude/skills/rust-devops/SKILL.md
- .claude/skills/rust-devops/deployment-platforms.md
source_code:
- scripts/ci/distribution.py
- scripts/ci/test_distribution.py
- scripts/ci/requirements-distribution.txt
- dist-workspace.toml
- Cargo.toml
- biscuit-hash/cli/Cargo.toml
- biscuit-file/cli/Cargo.toml
- tree-hugger/cli/Cargo.toml
- worktree/cli/Cargo.toml
- sniff/cli/Cargo.toml
- darkmatter/cli/Cargo.toml
- darkmatter/dmls/Cargo.toml
- claudine/cli/Cargo.toml
- scripts/ci/fixtures/distribution/dist-plan-biscuit-hash-cli.json
- scripts/ci/fixtures/distribution/dist-plan-darkmatter-cli-brew.json
- about.toml
- .github/distribution/build-images.json
- scripts/distribution/about.hbs
- scripts/distribution/build.sh
- scripts/distribution/inspect_archive.py
- scripts/distribution/installers.py
- scripts/distribution/licenses.sh
- scripts/distribution/native_notices.py
- scripts/distribution/test_inspect.py
- scripts/distribution/test_installers.py
- scripts/distribution/test_licenses.py
- scripts/distribution/fixtures/biscuit-hash-cli-installer.sh
- scripts/distribution/fixtures/dist-sha256.sum
- scripts/distribution/fixtures/notices/header.md
- scripts/distribution/fixtures/otool-libraries-bh-aarch64-apple-darwin.txt
- scripts/distribution/fixtures/otool-load-commands-bh-aarch64-apple-darwin.txt
- scripts/distribution/fixtures/readelf-bh-aarch64-unknown-linux-gnu.txt
- .github/workflows/distribute-binaries.yml
- .github/actions/setup-dist/action.yml
- scripts/distribution/install_check.sh
- scripts/distribution/tap.py
- scripts/distribution/publish-tap.sh
- scripts/distribution/test_tap.py
- scripts/distribution/github-web-flow.gpg
- scripts/distribution/fixtures/formula/darkmatter-cli.rb
- scripts/distribution/verify_binstall.py
- scripts/distribution/test_verify_binstall.py
- scripts/distribution/fixtures/binstall/Cargo.toml.in
- tools/test-toolkit/tests/ci_workflow_contracts.rs
- justfile
- just/distribution.just
- scripts/Cargo.toml
- scripts/ci/affected_scope.py
- scripts/ci/test_affected_scope.py
- scripts/distribution/run_suites.py
- scripts/distribution/pinned-python.sh
- scripts/distribution/verify.sh
- scripts/distribution/verify_runtime.py
- scripts/distribution/test_verify_runtime.py
- tools/test-toolkit/Cargo.toml
- .github/distribution/rollout.json
- scripts/ci/fixtures/distribution/dist-plan-sniff-detection-cli-brew.json
documentation:
- features/2026-10-28-publish/evidence/phase-2/README.md
- features/2026-10-28-publish/evidence/phase-2/license-review.md
- features/2026-10-28-publish/evidence/phase-4/README.md
- features/2026-10-28-publish/evidence/phase-4/verify/README.md
- docs/distribution.md
- features/2026-10-28-publish/evidence/phase-5/plans/README.md
- features/2026-10-28-publish/evidence/phase-5/verify/README.md
- .github/ci/README.md
- docs/releases.md
- docs/topics/ci-cd.md
- biscuit-hash/README.md
- biscuit-file/README.md
- tree-hugger/README.md
- worktree/README.md
- sniff/README.md
- darkmatter/README.md
- claudine/README.md
- .claude/skills/rust-devops/deployment-platforms.md
- .claude/skills/os/macos.md
- .claude/skills/rust-devops/SKILL.md
completed_phase: 5
implemented: true
---

# Binary distribution implementation plan

## Summary and Definition of Done

Add package-scoped, immutable binary distribution to the release path owned by release-plz. A published package release selects one approved CLI, builds its exact tagged commit with dist 0.34.0, validates four native targets, and attaches archives, checksums, installers, and provenance to the existing GitHub Release. Enabled Homebrew formulas consume those same archives. Binstall uses explicit metadata for the repository's package tags. Existing integration distribution and ordinary CI scheduling retain their own contracts.

Success has two separately reported dimensions:

- **Implementation:** strict approval and revision preflight, deterministic dist configuration, exact command selection, portable build jobs, immutable retry behavior, channel publishing, focused regression coverage, and current documentation all pass their checkpoints. Disabled packages produce no publication. The default production configuration remains disabled because the current deployment file grants no approvals.
- **Live acceptance:** the first real `bh` chain and then each additional enabled package have recorded release URLs, revision, checksums, target installation results, binstall results, and applicable tap results. A successful build alone never establishes a working installation channel.

Execute all five phases without a human gate. This execution authorizes implementation and nonpublishing verification, not modifying approvals, provisioning secrets, merging a proposal, or publishing a live release. Unavailable App setup, tap access, approved tags, clean-runtime infrastructure, or live authorization is recorded as pending acceptance in `implementation-log.md`; it does not prevent completing independent implementation work. Failed code contracts or artifact checks are implementation failures and are repaired before completion. The final report must distinguish these outcomes and cannot claim full specification acceptance while live evidence is pending.

Stay in the starting worktree and branch; preserve unrelated changes. Do not commit, push, create branches/worktrees, move lifecycle directories, or run `cargo fmt`. Every task below is unchecked until its stated evidence exists. Waves are numbered globally; tasks within a wave may use separate agents with the listed file ownership. All agents must be told the entire session is non-interactive.

## Phase 1 — Establish the distribution contract

### Necessary Rules

1. **Consume the repaired release path; do not repair it here.** `2026-11-06-release-path-repair` owns App-created events, reviewed-proposal revision checks, and release-plz publication. Its plan is present, but this checkout still has the old `GITHUB_TOKEN` workflow and no `scripts/ci/release_path.py`. Preserve that prerequisite's eventual `git_release_enable = true`, `git_only = true`, `publish = false`, and complete `{package}-v{version}` tags. Production activation requires its recorded successful live evidence. Implement distribution and its fixture contracts now, with missing prerequisite evidence explicitly pending; do not silently broaden its initial `sniff`/`wt` release eligibility to manufacture a `bh` release. Evidence: `.github/workflows/release-plz.yml`, `release-plz.toml`, and the prerequisite plan's Summary and Definition of Done.
2. **Generate opt-ins, rather than maintaining checked-in approval copies.** Add `scripts/ci/distribution.py`, an importable Python module with a thin CLI, plus `scripts/ci/test_distribution.py`. Public functions are `load_policy(path, packages)`, `select_release(event, policy, packages)`, `render_dist_config(selection)`, `validate_plan(selection, plan)`, `reconcile_assets(expected, existing)`, and `select_tap_update(candidate, installed)`. Results distinguish selected, ignored, pending setup, and invalid input; errors carry file and field paths. Resolve packages and manifest paths from `cargo metadata --locked --no-deps --format-version 1`, not a directory lookup. Python already drives repository CI orchestration, and `ci_workflow_contracts.rs::python_interpreter` supplies the existing contract-test bridge.
3. **Use one strict YAML loader.** Pin `PyYAML==6.0.2` in `scripts/ci/requirements-distribution.txt`; subclass `SafeLoader` to reject duplicate mapping keys, unsupported tags, non-string keys, and YAML merge keys, retaining node marks for diagnostics. Accept exactly one mapping document. Use YAML 1.2-style boolean resolution: only `true` and `false` are booleans; names such as `on` remain strings. Treat `$schema` and `types` as descriptive top-level metadata, not executable schemas. This avoids interpreting the current `pm` schema's string description as permission to accept string `"false"`. Evidence: `docs/deployment.yaml` contains actual booleans and inline mappings alongside those schema descriptions; current Python CI has no shared general YAML loader.
4. **Approval is explicit and fails closed.** `packages` is required and mapping-valued. Every entry key must identify an actual workspace package; `crate` must equal that key or be null. Null crate is valid only for an unapproved entry with every channel disabled, matching existing `darkmatter`, `homelab`, and `messenger` entries. Absent `approved` defaults to false; absent channel settings default to false. Explicit null is invalid for approval or any channel. Channels accept only booleans or nonempty portable names matching `[a-z0-9][a-z0-9+._-]*`; `true` resolves to the crate name, a string resolves to that name, and false disables the channel. Reject channel opt-ins on unapproved entries and duplicate resolved names within each channel across the whole policy. Validate the existing channel vocabulary in `docs/deployment.yaml`; reject unknown entry fields. Do not enable `install` or any out-of-scope channel. A selected package with such an enabled channel fails with an unsupported-channel error rather than pretending it was published. Approval does not change release-plz eligibility.
5. **The capability roster is eight package/command pairs.** Use the spec's `biscuit-hash-cli/bh`, `biscuit-file-cli/bf`, `tree-hugger-cli/hug`, `git-wt-cli/wt`, `sniff-detection-cli/sniff`, `darkmatter-cli/md`, `darkmatter-lsp/dmls`, and `claudine-cli/claudine`. This is a binary capability allowlist, not approval and not a directory mapping. Check the named command exists in metadata. Manifest paths currently resolve to the corresponding `*/cli/Cargo.toml`, with `darkmatter-lsp` at `darkmatter/dmls/Cargo.toml`. Correct `repository` on all eight; `darkmatter/cli/Cargo.toml` currently names `dockhand`, and several omit it. Require description, homepage, and package-specific overview content for shipped archives; use the canonical repository and its `tree/<tag>` source link.
6. **Pin dist 0.34.0 and allowlist archive binaries.** Keep checked-in `dist-workspace.toml` with workspace `dist = false`, `precise-builds = true`, `pr-run-mode = "skip"`, `create-release = false`, `install-updater = false`, `source-tarball = false`, `checksum = "sha256"`, Unix `tar.xz`, and no generated publisher. Add `[profile.dist]` inheriting release with `strip = true` and `lto = "thin"`; retain developer profiles. In a temporary full repository copy, generate only the selected package's `[package.metadata.dist]` opt-in and `[package.metadata.dist.binaries] "*" = ["<command>"]`. Native Cargo default features remain enabled; all-features and additional test features remain disabled. Reject checked-in package dist opt-ins to prevent a second authority. S1 proved precise builds and package tags. Its assertion that fixture moves were required is superseded by pinned upstream `tasks.rs`, where `DistGraphBuilder::add_variant` reads `config.artifacts.archives.binaries` and replaces `packageables` before creating archive binaries ([source](https://github.com/axodotdev/cargo-dist/blob/v0.34.0/cargo-dist/src/tasks.rs)). Prove this with JSON plans and real `md` packaging; do not relocate fixtures.
7. **Gate the worktree helper.** Add an explicit `[[bin]]` for `render_dirty_tree`, its current `src/bin/render_dirty_tree.rs` path, and `required-features = ["terminal-tests"]` in `worktree/cli/Cargo.toml`. Its only active caller is `worktree/cli/tests/level2_dirty_tree.rs`, already gated by that feature. Leave the other test helpers and all existing caller paths intact. Binstall 1.26.2 `collect_bin_files`/`download_extract_and_verify` tolerates missing feature-required binaries but errors for missing unconditional ones; this gate closes its remaining mismatch without moving test infrastructure ([source](https://github.com/cargo-bins/cargo-binstall/blob/v1.26.2/crates/binstalk/src/ops/resolve.rs)).
8. **Published releases receive explicit uploads only.** New `.github/workflows/distribute-binaries.yml` listens to canonical `release: published` events and manual `tag` recovery. It does not execute dist's generated release creation/publication steps, edit release notes, mark releases latest, create tags, or run on PRs. Pin dist through checksum-verified upstream release binaries. Run `dist plan --tag=<tag> --output-format=json`, target-specific `dist build --tag=<tag> --artifacts=local --target=<target>`, and the global-artifact build after collecting local outputs. Set `allow-dirty = ["ci"]` for this deliberately handwritten workflow only. Upstream documents that `create-release = false` expects a draft; explicit upload avoids relying on that assumption ([configuration reference](https://axodotdev.github.io/cargo-dist/book/reference/config.html#create-release)).
9. **Revision and policy checks precede builds and repeat before mutation.** Read current default-branch policy and tagged policy, requiring approval and enabled channels in both; this permits current policy to revoke an old release but prevents current approval retroactively authorizing an unapproved tag. Resolve the complete tag to a peeled commit, require exact manifest version equality, a published non-draft Release, canonical repository, and the reviewed-proposal proof supplied by the prerequisite. Compare the resolved commit with the recorded proposal revision and the event's release/tag identity, not `target_commitish` alone. Manual recovery takes only an existing complete tag and performs the same checks. Non-roster/library/integration events return ignored; malformed candidate CLI tags, wrong revisions, revoked approvals, and manifest mismatches fail. Unapproved recognized releases are ignored with a named reason and no build matrix. Prereleases use their exact SemVer/tag and never update the stable Homebrew formula.
10. **Build four targets, leave Windows disabled.** Use `aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`, and `x86_64-unknown-linux-gnu`. No Windows artifact, PowerShell installer, musl, or Windows ARM is enabled in this implementation. The Windows prerequisite spec remains `implemented: false`; that is insufficient evidence to ship Windows. Keep Windows addition as a subsequent evidence-backed configuration change, not a runtime guess or a blocker for the four targets. WSL2 validates the Linux archive without generating another target.
11. **Pin Linux's ABI and build platform.** Use native architecture jobs on `ubuntu-22.04` and `ubuntu-22.04-arm`, running builds inside `ubuntu:22.04@sha256:076e7bf82319619f28dd0aa9c02ab34b055ad37100395b4f49a2ffba04fb5e11`. This multiarchitecture digest was resolved read-only from Docker Hub during planning; its amd64 child is `sha256:8a2321a92b438238f93b4346fff61bc4259c99fdd4d9b52ee86f153fee0c344f`, arm64 child `sha256:a8fb2ba88181746111cbbe2969826d6546b71226785b196c93d2c3445a25000d`. The container, rather than the runner, defines the ABI floor. Install `build-essential`, `pkg-config`, `curl`, `ca-certificates`, `xz-utils`, and `binutils`; add `libasound2-dev` only for claudine. Install the tagged `rust-toolchain.toml` version, currently 1.98.1, and build with the tagged lockfile. Clear Rust wrappers and ambient Cargo configuration. Refresh the digest monthly and for security advisories through an ordinary change with ABI smoke verification; never update it automatically within a release run.
12. **Use an explicit macOS floor.** Build natively on `macos-15` arm64 and `macos-15-intel` x86_64, never `*-latest`; set `MACOSX_DEPLOYMENT_TARGET=15.0` and document macOS 15+ for both architectures. This conservative initial floor matches the chosen build and runtime generation rather than promising compatibility on an untested older OS. Record runner image version and Xcode build in provenance because hosted labels pin the OS generation, not an immutable image snapshot. Use Xcode's libclang and a C compiler, and pkg-config. No cmake or OpenSSL headers are added. S3 identifies the actual native dependency requirements.
13. **Static PCRE2 and license notices are mandatory.** Set `PCRE2_SYS_STATIC=1` in every build, corroborated by S3's successful `hug` rebuild against locked `pcre2-sys 0.2.10`. Reject PCRE2 dylibs/shared objects, Homebrew prefixes, build-checkout paths, and unexpected dynamic libraries in finished binaries. Accept Linux system libc/libm/libgcc/loader, plus `libasound.so.2` for claudine; accept macOS system libraries/frameworks only. Keep native audio. Include root `LICENSE`, exact tagged source URL, and third-party license notices for the actual locked production dependency graph, including statically linked PCRE2 and vendored native dependencies. Use a pinned `cargo-about` configuration to collect notices; unresolved license text fails packaging. License review artifacts are retained for final review without delaying dependent implementation.
14. **Keep immutable names and byte comparisons.** Retain dist's `<package>-<target>.tar.xz` names inside version-specific release URLs and `<package>-<target>/<command>` layout, as measured in S1. Dist generates SHA-256 sidecars and `sha256.sum`; reuse that supported output. Repository retry logic downloads and compares existing assets byte-for-byte in bounded chunks, so no separate hashing implementation or SHA-256 extension to biscuit-hash is needed. Do not use Python hashlib or shell hashing to implement repository hash logic. Add a deterministic `distribution-provenance.json` asset containing package, version, tag, commit, targets, toolchain, dist pin, build-image identifiers, byte sizes, and dist checksums. Keep volatile run URLs/times in workflow artifacts and the implementation log rather than immutable assets. Recover missing assets only from the same retained successful build bundle; changed rebuild bytes fail visibly and require a new version.
15. **Binstall needs explicit package-tag URLs.** S2 already proved automatic discovery fails, so add its `pkg-url = "{ repo }/releases/download/{ name }-v{ version }/{ name }-{ target }{ archive-suffix }"`, `pkg-fmt = "txz"`, and `bin-dir = "{ name }-{ target }/{ bin }{ binary-ext }"` metadata only for channel-enabled packages in the generated release configuration; validate tagged manifest metadata before publication. Supply checked-in matching metadata on the eight capable packages with `disabled-strategies = ["crate-meta-data", "quick-install", "compile"]` while their binstall channel is disabled; enabling a channel requires the policy and metadata to change together in the release proposal. This keeps an opt-out distinguishable from a broken installation. Do not treat a CLI override of disabled metadata as a supported channel. Test with binstall 1.26.2 and `--strategies crate-meta-data --no-confirm --disable-telemetry`; compile fallback is excluded. Its `--git` clones default HEAD, and an exact `--version` overrides the manifest version rather than selecting a Git tag (`PackageInfo::resolve`, linked in rule 7). Document the Git route for the current manifest version and historical installation through an exact-tag checkout plus `--manifest-path <resolved-package-manifest>`, with explicit version equality checked before invoking binstall. Do not invent `--git --tag` syntax or claim Git version selection follows registry discovery.
16. **Homebrew is a post-upload projection.** Generate enabled formulas from the validated dist global artifacts and their SHA-256 values; configure formula names from the resolved `brew` channel and tap `yankeeinlondon/homebrew-tap`. Publish only after all referenced artifacts are downloadable and install checks pass. Add `depends_on "alsa-lib" if OS.linux?` for claudine. Serialize tap pushes across packages as well as distribution per package, with cancellation disabled. Compare installed and candidate versions using SemVer; older candidates are successful no-ops, identical versions require identical formula bytes, newer stable versions update. Never use lexical version ordering, force-push, or fall back to GITHUB_TOKEN for the tap. The formula publisher alone receives `HOMEBREW_TAP_TOKEN`; an enabled publisher without it fails actionably.
17. **Rollout evidence has a fixed schema.** Check in `.github/distribution/rollout.json` with `schema_version: 1` and `packages: {}`. Successful entries contain required package/tag/commit/distribution-run URL and ID, plus one passed archive/shell-install record for each of the four targets and passed records for every enabled channel; absent entries mean not yet rolled out, explicit null or incomplete entries are errors. Preflight verifies referenced successful runs through the forge and exact package/tag/commit equality before accepting the first-bh proof. Disabled channels have an explicit disabled state and cannot satisfy an enabled channel. This implements the spec's first-bh ordering without turning deployment approvals into test evidence. Include this loader and its fields in Phase 4's shared JSON/record/list matrix.
18. **No new measurement spike is necessary.** S1 establishes dist's build-only operation and package tags, S2 establishes metadata requirements, and S3 establishes sizes, PCRE2, ALSA, and thin-LTO tradeoffs. Pinned source settles the binary override and Git-version questions. Artifact ABI checks and runtime installation checks below are correctness acceptance, not repeated performance measurements. No additional size/performance study or full-workspace CI run is scheduled.

### Wave 1 — Policy foundation

- [x] **Implement policy preflight**
  - Own `scripts/ci/distribution.py`, its dependency pin, and its policy/event fixtures. Implement rules 2–5 and 9 with no network or subprocess side effects during parsing/selection; inject forge and command adapters at the execution boundary.
  - Expose CLI subcommands `preflight`, `config`, `validate-plan`, `reconcile-assets`, and `tap-update`. Pin Python `semver==3.0.4` alongside PyYAML and use `semver.Version.parse` for complete version validation and comparisons, rather than writing a partial parser. JSON output is the machine contract; errors go to stderr with document/field location and nonzero status. Build and upload functions cannot be reached without a successful preflight result.
  - Produce a selected record with package, command, resolved manifest path, exact version/tag/commit, effective channels, four targets, and tool pins. An ignored result has an empty matrix and explicit reason. Retain byte-preserved current/tagged policy snapshots as provenance; no policy digest implementation is introduced.

- [x] **Prepare package metadata**
  - Own the eight CLI manifests, root profile, checked-in dist base configuration, and package overview snippets. Apply rules 5–7 and 15; preserve test features and all fixture callers. Keep every actual deployment approval and channel value unchanged.
  - Add explicit disabled binstall metadata and canonical repository metadata; supply descriptions and readme paths pointing to each area's overview. Include source/license notices through generated distribution staging, not a duplicated monorepo README in every archive.
  - Update comments whose claims about default fixture distribution become inaccurate. Record the S1 departure in the implementation log; do not edit its historical measurement report.

Checkpoint: the untouched real deployment document is a valid all-disabled control; a one-entry approved fixture selects exactly one command; generated configuration cannot select another package. Metadata changes do not change release-plz scope or approvals.

## Phase 2 — Build and validate portable artifacts

### Wave 2 — Configuration and build machinery

- [x] **Generate dist inputs**
  - Own config/provenance generation in `distribution.py`, after Wave 1. Copy the complete tagged checkout into temporary distribution staging, preserving embedded files and path dependencies; apply generated dist metadata there only. The original checkout remains byte-clean.
  - Generate installer/channel settings from the effective selection, binary overrides for all four targets, and `min-glibc-version` 2.35 for the two GNU targets. No PowerShell output is generated. Require exact plan release/package/version, four target set, one shipping binary, production features, immutable download prefix, and enabled-channel correspondence.
  - Generate provenance deterministically in sorted target/asset order; obtain checksums from dist outputs, not new digest code. Archive all generated configuration, dist JSON plans, original policy snapshots, and command logs as workflow artifacts. The approval overlay changes distribution metadata only; compare staged production source and lockfile bytes against the original tagged checkout before building, and retain the overlay diff.

- [x] **Build target jobs**
  - Own `scripts/distribution/build.sh`, `scripts/distribution/inspect.py`, and `.github/distribution/build-images.json`. Encode rules 11–13 in checked-in data. Shell scripts use non-interactive one-shot commands with bounded timeouts; no background processes, signing dialogs, or terminal focus changes.
  - Build each target separately from the selected tagged commit using dist local artifacts, `--locked` Cargo invocations, stripped thin LTO, and static PCRE2. Inspect the actual build command/log for exact package selection and absence of workspace/all-features builds.
  - Inspect archive paths for traversal, absolute paths, symlink escapes, extra executables, wrong CPU/format, and missing notices/source URL. Inspect ELF `GLIBC_` version requirements and reject anything newer than 2.35; retain `readelf`/`ldd` evidence. Inspect Mach-O deployment version, architecture, rpaths, and `otool -L` output; reject non-system dependencies and build-host paths.
  - Use the same clean runtime image digest for Linux smoke checks, without installing build tools; install runtime `libasound2` only for claudine. Run archive `--version` and `--help` for all commands, including dmls, whose `main.rs` handles those before starting its server.

### Wave 3 — Installer and license outputs

- [x] **Harden generated installers**
  - Own `scripts/distribution/installers.py` and installer fixtures. Run after Wave 2's config contract. Retain dist-generated versioned download URLs and checksums. Add a checked, single-match transformation to the pinned shell template to perform a glibc 2.35 preflight before downloading/installing on Linux. Template drift is an error, never an unchecked text replacement.
  - Compare numeric glibc components; report required/found versions and exit before writes for 2.34, musl, missing/unparseable detection, and unsupported architectures. Accept 2.35 and newer; macOS follows the 15.0 floor. Preserve installer arguments/environment documented by the pinned template, including isolated installation-directory support and no PATH mutation during tests.
  - Leave archive bytes and their dist-generated SHA-256 sidecars/unified checksum output untouched. Dist 0.34.0 adds checksums to executable archives in `add_variant`; the generated shell script is a global installer, not one of those archives. Validate every checksum entry against its referenced immutable output and refuse a stale checksum; do not invent a dist checksum subcommand. The final shell installer is protected by HTTPS and the same immutable byte-comparison retry contract. Generate it once per package after collecting every target's metadata; never let target jobs race to create differing global assets.

- [x] **Bundle license notices**
  - Own `about.toml`, `scripts/distribution/licenses.sh`, and archive notice fixtures. Pin `cargo-about` 0.8.4; produce notices for each selected package's locked production dependency closure and all target-specific dependencies. Bundle native-library notices not covered by crate metadata, including PCRE2's vendored license.
  - Include root AGPL license, third-party notices, and `SOURCE.txt` with exact tag and commit URLs. Check that archive inclusion is configured before dist packages binaries, and validate the resulting archives rather than just checking the staging directory.

Checkpoint: pinned dist plans for `bh` and `md` select exactly one shipping command; actual local `bh` and `md` archives exclude every fixture. All four native target builds and Linux ABI checks have retained evidence or explicitly pending infrastructure status. No output has been uploaded to a Release.

## Phase 3 — Wire immutable publication and channels

### Wave 4 — Workflow and asset reconciliation

- [x] **Wire release workflow**
  - Own `.github/workflows/distribute-binaries.yml` and a small `.github/actions/setup-dist/action.yml`. Use rules 8–12. Separate preflight, local builds, global packaging, runtime verification, upload, and tap publication jobs with explicit dependencies. Concurrency is keyed on selected package, `cancel-in-progress: false`; never key only on a changing run ID.
  - Default workflow and build jobs to contents read, checkout with persisted credentials disabled, and no App/tap secrets. Grant contents write only to upload, and expose the tap token only to its publisher. Treat event fields as data passed through environment/arguments, never interpolate them into executable shell code.
  - Provide a nonpublishing manual verification mode for current implementation revisions and fixture approvals, storing workflow artifacts only. Production mode accepts only approved existing tags; verification mode cannot reach upload or tap jobs, even with forged fixture fields. Handwritten workflow contracts must prove this boundary.
  - Preserve integration workflow selection/assets and prerequisite release ownership. Missing release-proof/App evidence fails a selected production preflight with its setup instruction; it must not trigger a fallback release path.

- [x] **Implement immutable upload**
  - Own forge adapters and retry fixtures in `distribution.py`. Enumerate and compare the complete expected asset set before uploading any new asset. Existing identical bytes are reused; missing names are uploaded; a conflicting name fails with no replacement/deletion. An unavailable download or truncated body is an error.
  - Persist a verified complete build bundle as a workflow artifact before uploads. Manual recovery supports an explicit existing distribution-run ID for that tag; validate its successful build result, selected revision, target set, configuration, and bundle before reuse. A fresh rebuild is allowed only for a release with no distribution assets; any partial release requires its original bundle. This avoids nondeterministic rebuilds poisoning immutable releases.
  - Recheck tag/proposal identity and current policy immediately before mutation; reject races and revoked channels. Retain unrelated existing release assets. Do not modify release body, draft/published state, latest setting, or tags.

### Wave 5 — Channel adapters

- [x] **Publish tap formula**
  - Own `scripts/distribution/publish-tap.sh`, formula validation, and tap adapter tests. Follow rule 16. Use GitHub GraphQL `createCommitOnBranch` with `expectedHeadOid` and only the changed formula path, authenticated as the author through `HOMEBREW_TAP_TOKEN`. GitHub signs these commits server-side; this avoids introducing an undisclosed signing-key secret ([GitHub commit API](https://docs.github.com/en/graphql/reference/commits)). Require the credential owner to be the configured author, no attribution trailers, and a valid returned signature. Fetch the resulting commit into a temporary checkout and run `git verify-commit` using GitHub's published web-flow signing key in an isolated GPG home. A signature failure fails once; never bypass signing. Do not execute a real tap mutation during this implementation.
  - Add one tap-wide concurrency group around version-check/update. Fetch before the final version comparison; `expectedHeadOid` rejects a concurrent branch change. A branch-head conflict fails the publisher and is recovered by rerunning the same validated tag; no internal speculative retry or force update is used. Preserve unrelated formulas. Compare stable versions through the existing Python SemVer validator used by preflight, including numeric ordering tests such as 0.9 versus 0.10.
  - Validate `brew audit`, formula syntax, binary-only install behavior, glibc floor declaration, and claudine's Linux ALSA dependency. Download and verify every referenced asset before generating a tap commit.

- [x] **Verify binstall wiring**
  - Own a nonpublishing binstall verification driver and fixtures. Pin 1.26.2. Exercise `--git https://github.com/yankeeinlondon/rusty-biscuit <package>` for the current manifest version with compile fallback disabled, isolated Cargo home/root, exact target, and retained logs showing package/version and release URL.
  - Exercise historical version installation from the exact tagged checkout using `--manifest-path` and the resolved manifest path; require its manifest version to equal the tag before invocation. Disabled channels fail before network downloads. Worktree's missing feature-required helper is tolerated while `wt` remains required.
  - Fixture tests use a local forge/archive server and real pinned binstall to prove extraction and optional-helper handling without live publication. Keep network-dependent tool compatibility checks separate from fast parser tests; retain them as explicit nonpublishing acceptance commands, not full-workspace CI cells.

Checkpoint: fixture publication attaches only missing assets to a preexisting published release, preserves notes and tags, refuses byte conflicts, and cannot downgrade formulas. Credential scoping and manual-recovery checks are executable contracts. Disabled binstall/Homebrew channels produce no successful channel claim.

## Phase 4 — Prove failure handling and installation

### Input robustness contract

Every invalid document or load-bearing field shape produces a located error before builds, downloads, uploads, or tap changes. Keep one positive control from the real deployment document and real-shaped forge/dist outputs; each negative fixture changes one value. Public module preflight/execution results and an injected adapter's recorded effects are the assertion boundary, not private YAML constructors.

- Shared validators cover required nonempty strings, booleans, channel boolean-or-name values, mappings, lists of records, SemVer, complete tags, commit IDs, target identifiers, and asset paths. For each validator test absent, explicit null, wrong whole type, and empty value; collection validators additionally test a single invalid member in a mixed list and all-invalid members. Each wired field then gets a valid control and one invalid-shape test.
- Exceptions are exactly those in rule 4: absent approval/channels default false; null crate represents an explicitly nondistributable entry; absent metadata-only `$schema`/`types` is accepted. Empty `packages` is a valid all-disabled policy and produces ignored selection. Empty API asset lists are valid before first upload. Empty build-target/release/expected-asset collections on a selected distribution are errors. Unknown GitHub API fields are ignored; unknown policy entry fields are rejected. A 404 is valid absence only for an asset/formula lookup, never for the selected tag, Release, or proof.
- Load-bearing policy fields are `packages`, each package key, `crate`, `approved`, and every recognized channel. Load-bearing metadata fields are package name/version/repository/manifest path, binary target name/required-features, and dist/binstall settings. Load-bearing forge fields are repository identity, event type, tag, draft/prerelease state, release identity, peeled commit/proposal proof, asset name/size/download identity, and existing formula version. Load-bearing dist fields are announcement tag, releases/package/version/binaries, targets, artifact names/paths/download URLs/checksums, and installer/channel declarations. Validate the exact pinned dist JSON shape against S1's output; fixtures retain the upstream document rather than a reduced made-up schema.
- YAML loader tests cover duplicate keys at root/package/channel depth, `k:`/`k: null`, comments and flow mappings, quoted false versus boolean false, a second document, invalid trailing text, unsupported tags, and merge keys. JSON uses one `json.loads` wrapper with `object_pairs_hook` rejecting duplicates and non-finite values; test duplicates, trailing garbage, malformed input, null roots, and valid Unicode. TOML uses `tomllib`; test duplicate definitions, malformed/trailing content, legitimate tables, and empty documents. Both manifest/config reads share that loader; TOML has no null shape. Do not repeat full loader matrices per field.
- Add CLI wiring coverage per loader: positive fixture and one located malformed case proving the adapter launched nothing. Full matrices run in-process through the public module API. Grep changed Python for swallowed exceptions, `.get(..., [])`, `or []`, coercive bool/string conversions, and filtered invalid members. Inspect Rust test additions for `#[serde(default)]`, null-conflating `Option<T>`, `filter_map`, `unwrap_or_default()`, and `.ok()`. Fix silent loss; document only the explicit exceptions above. A matrix review finding must report every remaining shape of that validator and loader before closing it.

### Wave 6 — Regression and runtime coverage

- [x] **Add focused contracts**
  - Own the test-toolkit bridge in `tools/test-toolkit/tests/ci_workflow_contracts.rs`, dependency setup recipes, and focused Python suites. Add literal repository file reads and declare new source inputs in `tools/test-toolkit/Cargo.toml` so automation-only edits schedule the exact owning contracts. Reuse existing package-keyed CI planning; introduce no distribution test scheduler or full-workspace run.
  - Add `just distribution-check` and `just distribution-verify` recipes in `just/distribution.just`, imported at root. The first provisions a temporary pinned Python environment, runs policy/fixture and workflow checks through the existing test-toolkit contract path; the second runs the real pinned-tool compatibility and installation checks and retains a report. Do not permit absent Python/PyYAML/tools to make selected tests silently skip.
  - Cover approved/unapproved and channel combinations, duplicate names, foreign/library/integration tags, wrong versions/revisions, exact binary and feature selection, policy revocation, prerelease handling, first/identical/partial/conflicting retries, missing build bundle, missing credentials, tap equal/newer/older versions, unsafe archive paths, glibc detection, and installer template drift. Assert no effects after invalid input.

- [x] **Verify clean installs**
  - Own runtime verification scripts and evidence reports. Install actual generated archives and shell installers into isolated temporary roots for each of the four targets; run `--version` and `--help` and verify only the intended command was installed. Exercise the Linux artifact under WSL2 separately without claiming native Windows evidence. Never invoke playback, interactive provider flows, or an LSP server loop.
  - Use native clean macOS 15 runtime VMs without developer tools/Homebrew libraries and the pinned Ubuntu 22.04 runtime containers without compiler packages; retain OS/CPU, dynamic library observations, exact download identity, exit status, and command output. Use the `os` skill's host contract to locate native hosts; no cross-compile substitutes for runtime evidence. Missing clean macOS VM provisioning is pending infrastructure, never a successful clean-runtime claim. Hosted build-host smoke tests remain useful but are labeled separately.
  - For enabled Homebrew formulas, verify an isolated brew prefix on each supported macOS/Linux CPU target, then run the installed command. Homebrew's own platform support constrains this check; do not call an untested target complete. For disabled channels, record disabled rather than skipped-success. PowerShell/native Windows are explicitly outside the enabled target set.
  - Run unsupported-glibc installer tests before mutation and positive 2.35 runtime checks after extraction. Reject runtime library leakage even when build-host smoke passes. Record archive sizes as release information; there is no size rejection threshold.

Checkpoint: public-result robustness coverage is complete, retries and denied channels are proved, runtime reports distinguish native/WSL/build-host/clean-runtime evidence, and no speculative workspace-wide run was added.

## Phase 5 — Document and close implementation

### Wave 7 — Current documentation

- [x] **Document distribution operations**
  - Own `docs/releases.md`, new `docs/distribution.md`, relevant area README links, and `.claude/skills/rust-devops/SKILL.md` distribution contract. Explain package versus command names, four targets, macOS 15 and glibc 2.35 floors, excluded glibc 2.34 distributions, and WSL2's Linux route. Give claudine runtime commands: Debian/Ubuntu `apt-get install libasound2`, Fedora `dnf install alsa-lib`; Homebrew formulas declare their Linux dependency.
  - Explain published-but-assets-pending state, workflow run links, explicit tag recovery with retained build bundle, immutable conflict failures, monthly build-image refresh, exact source/licenses/checksums, no downgrade, credential scope, and approval changes through reviewed release proposals. Include a Mermaid lifecycle diagram. Keep current docs free of dated feature/fix references.
  - Include concrete command syntax, but advertise a channel as available only with actual tested version-specific URLs. Until live acceptance exists, label channel examples planned and omit runnable placeholder download URLs. Document Git default-HEAD binstall behavior and exact-tag manifest installation separately. README content stays an overview with links to topic guides.

### Wave 8 — Final validation and evidence

- [x] **Run scoped validation**
  - Run `just distribution-check` and `just test test-toolkit` once after integration; during development filter the changed contract slice through Nextest. Run `cargo clippy -p test-toolkit --all-targets -- -D warnings`. Do not run `just lint`, which includes formatting, or `cargo fmt`.
  - Validate the new workflow and composite-action references with actionlint 1.7.12, installed from checksum-verified upstream release assets. Run `just distribution-verify` for available native hosts and report missing infrastructure explicitly. Validate generated dist JSON plans for all eight capability packages under fixture approvals, including all helper-bearing packages, without changing real approvals.
  - Run worktree's focused affected L1 checks and `just test-l2` for its `level2_dirty_tree` slice through the package-area recipe, keeping windows unfocused; this verifies the explicit fixture gate did not strand its existing test. Reuse qualifying other package evidence because their source behavior did not change. Build the shipping commands through artifact validation, rather than rerunning eight unrelated test suites to inspect archive layouts.
  - Review final diffs for accidental policy enablement, broadened release-plz eligibility, integration changes, secrets, unsigned-publication bypasses, unbounded subprocesses, mutable latest URLs, unlocated parser errors, and configuration defaults that hide malformed data. Repair findings in the owning task and rerun only affected checks.

- [x] **Record rollout evidence**
  - Create `implementation-log.md` with rules implemented, S1 binary-override departure, exact commands/results, generated plans, artifact/runtime reports, license output, source revisions, and external setup status. Never rewrite the historical spec or move it to completed.
  - Read existing authorized release/proposal/run/tap evidence without causing publication. Populate the live ledger for `bh` first; keep every other rollout pending until that full chain succeeds and its own approval/artifact/install evidence exists. The automation must reject widening without a verified first-`bh` ledger entry tied to the successful distribution run. Store this rollout evidence in `.github/distribution/rollout.json`, validated by the same JSON loader, containing the successful package/tag/commit/run and target/channel results; entries are evidence, not substitutes for deployment approvals. The first `bh` run creates its ledger candidate as a workflow artifact; a subsequent reviewed repository change carries it into the durable ledger.
  - Missing approvals, App proof, tap repository/token, clean-runtime provisioning, or live-release authorization produce named pending ledger entries and an honest final acceptance report. Do not ask questions or wait. Finish all independent implementation tasks; do not mark a pending live target/channel as complete or enable another package to bypass the first-`bh` requirement.
  - End with implementation complete, ready for review only when code and focused verification pass. Report full live acceptance separately and only from real URLs/install results. The author performs lifecycle completion after review.