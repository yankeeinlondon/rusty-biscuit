---
spec: /Volumes/coding/rusty-biscuit/features/2026-10-28-publish/spec.md
plan: features/2026-10-28-publish/plan.md
implemented_by: claude/opus
started_phase: 1
decisions_for_review:
  - |-
    Packages excluded from the Cargo workspace still count as known policy entries. The plan says every
    `docs/deployment.yaml` key must be a workspace package resolved by `cargo metadata --no-deps`, but the
    real file has entries for `schematic-schema` and `zed-dmls`, which the root `Cargo.toml` lists under
    `[workspace] exclude`. Taken literally, the unedited real policy would fail to load, which breaks the
    plan's own checkpoint that it is a valid all-disabled control. I decided that the known names are the
    workspace members plus the packages at the root manifest's `exclude` paths, read from their own
    manifests. A misspelled package name is still rejected, and an excluded package can never be selected
    because it is not one of the eight capable packages. The alternative is to delete those two entries
    from `docs/deployment.yaml`. That is simpler, but it edits the approval file. Changing course means
    removing `excluded_package_names` and its use in `distribution.py`, then deleting the two entries.
  - |-
    How preflight treats approval or a channel that differs between the tagged and current policy. The
    plan requires approval and channels in both, and says a revoked approval fails. It also says the
    current policy "can revoke an old release". I chose: unapproved in the tagged policy → ignored (current
    approval never makes an old tag eligible); approved when tagged but not now → error; a channel enabled
    when tagged but disabled now → dropped from the effective channels and listed in `revoked_channels`,
    while the archives are still built; the same channel resolving to two different names → error. The
    alternative is to fail the whole run when any channel is revoked. That is stricter, but then turning
    off Homebrew for one package would block that package's archives too. The change would be a few lines
    in `select_release` plus one test.
  - |-
    The binstall channel must resolve to the package name. `docs/deployment.yaml` allows a string
    alternate name for every channel, but cargo-binstall installs by Cargo package name
    (`cargo binstall --git … biscuit-hash-cli`), so an alternate name has no meaning there. Preflight
    rejects `binstall: <other-name>` and accepts only `true`. The alternative is to accept and ignore the
    name, which would quietly misstate what is published.
  - |-
    The exact binstall metadata for the two states. Every capable manifest now carries the S2 `pkg-url`,
    `pkg-fmt`, and `bin-dir`. While its channel is disabled it also has
    `disabled-strategies = ["crate-meta-data", "quick-install", "compile"]`. The enabled form is defined as
    `["quick-install", "compile"]`, so compile fallback stays off even when enabled. Preflight requires the
    exact list that matches the channel state. The plan names the disabled form but not the enabled form.
    The alternative is to leave `compile` enabled once a package is approved. Changing course means
    editing `BINSTALL_ENABLED_STRATEGIES` and its tests.
  - |-
    The new descriptions for `bh`, `bf`, and `sniff` copy each CLI's existing doc-comment summary
    ("Hash content using various algorithms (xxHash, BLAKE3, Argon2id)", "File format conversion and
    extraction utility", "Detect system and repository information"). These CLIs use clap's bare `about`,
    which prints the Cargo description in `--help`, so a newly written description would have changed
    their help text. `hug` and `wt` set `about` explicitly, so they got fuller descriptions for Homebrew.
    For `bh` and `wt`, which have no package README, `readme` points to the area overview
    (`../README.md`). `homepage` is the canonical repository's `tree/main/<area>`. The exact tagged source
    link belongs in the archive's `SOURCE.txt` (Phase 2).
  - |-
    The proof that a tag came from a reviewed proposal. The plan asks preflight to compare the tag's commit
    with "the reviewed-proposal proof supplied by the prerequisite". The prerequisite records no proof
    artifact. The release path tags each proposal's final (head) commit, so preflight asks GitHub which pull
    requests contain the tag's commit (`commits/{sha}/pulls`). It accepts only a merged pull request whose
    head is that commit and which `release_path.select_publication` accepts as an App-authored release
    proposal. It reuses those checks rather than duplicating them. A missing `RELEASE_APP_BOT_LOGIN` is
    reported as `pending-setup` with exit 3, never treated as a pass. The alternative is for release-plz's
    publish job to upload a signed proof artifact that distribution then downloads. That needs a change to
    the prerequisite's workflow.
  - |-
    The checked-in `dist-workspace.toml` also sets `ci = "github"`, `installers = ["shell"]`, and
    `install-path = "CARGO_HOME"`. `check_dist_base` treats every key as reviewed and rejects an extra key.
    `ci = "github"` is needed for `allow-dirty = ["ci"]` to mean anything; S1 also showed that dist's GitHub
    mode supplies the repository URL. The Homebrew installer is added only in the generated per-package
    opt-in, when the `brew` channel is effective. If dist 0.34.0 rejects any of these during Phase 2's real
    `dist plan`, edit the file and `DIST_BASE` together.
  - |-
    A release event from a repository other than `yankeeinlondon/rusty-biscuit`, such as a fork, is
    ignored rather than treated as an error, matching `release_path.select_preparation`. A missing
    `GITHUB_REPOSITORY` is an error.
  - |-
    The Linux glibc floor (2.35) is set in the checked-in `dist-workspace.toml`, not in the generated
    per-package opt-in. The plan asks the generated configuration to carry `min-glibc-version`. A real
    dist 0.34.0 run showed that its shell installer reads that setting from the workspace configuration
    only: with the package-level setting the installer still checked glibc 2.31. The floor is the same for
    every package, so it is now one reviewed line in `dist-workspace.toml`, also enforced by `DIST_BASE`.
    The alternative is to generate it into the staged `dist-workspace.toml` per release, which adds a
    second staged edit for no difference in output. Changing course means moving the table into the
    overlay writer and allowing a second staged file in `verify_staging`.
  - |-
    The archive's generated files live at `<package dir>/.distribution/SOURCE.txt` and
    `.distribution/THIRD-PARTY-NOTICES.md` in the staged copy, and reach the archive through dist's
    `include` setting. The plan did not name a location or file names. A checked-in file at that path is
    refused, so a tagged commit cannot override the generated content. The alternative is a root-level
    directory with `../..` include paths, which ties the overlay to each package's depth. Renaming means
    changing `GENERATED_DIR`/`NOTICES_NAME` and regenerating the two plan fixtures.
  - |-
    The binary override is the single wildcard entry `"*" = ["<command>"]` rather than one entry per target.
    The plan says "binary overrides for all four targets". dist applies `"*"` to every target; per-target
    entries would leave any later-added target (such as Windows) falling back to all of the package's
    binaries, which could ship test helpers. The real plans for `bh` and `md` show one executable per
    archive on all four targets.
  - |-
    Checksum verification uses the platform's standard tool, `shasum -a 256 -c`, against dist's own
    `.sha256` files and `sha256.sum`. The plan requires every checksum to be checked against the bytes, and
    also forbids writing hashing in Python `hashlib` or shell. Checking without computing a digest is not
    possible, so the check calls the same verifier a user would run, and the repository computes no digest
    itself. The alternative is a SHA-256 command in `biscuit-hash` (`bh`). That is more code to own, and the
    plan says no such extension is needed. Changing course means replacing the `shasum` call in
    `installers.py check-sums`.
  - |-
    The lockfile is proven rather than passed as `--locked`, because dist 0.34.0 never passes `--locked` to
    cargo. `build.sh` runs `cargo metadata --locked` and fails if `Cargo.lock` changes during the build.
    `stage` already proves the staged lockfile equals the tagged one. An offline build after
    `cargo fetch --locked` was tried and broke: dist's own metadata step needs crates for every platform.
    The alternative is wrapping cargo through dist's `cargo-build-wrapper` hooks, which is brittle.
  - |-
    The plan's `scripts/distribution/inspect.py` is named `inspect_archive.py`. A script called
    `inspect.py` shadows Python's standard `inspect` module for every script in that folder, which broke
    `dataclasses` and `unittest` until each file hacked its import path. Renaming back means restoring
    those workarounds.
  - |-
    The Linux build container also installs `python3`, listed under its own `tool_packages` key in
    `build-images.json` so that rule 11's build-package list stays exact. Python reads the build settings
    and runs the archive inspection inside the container, and it is never linked into a binary. The
    alternative is to inspect on the runner host and pass the settings in from outside the container,
    which means more workflow plumbing in Phase 3.
  - |-
    Archive inspection checks for build-machine paths (the staging root, `/Users/`, `/home/runner`) only
    in load commands, rpaths, and dynamic entries, not anywhere in the binary. Rust embeds
    `$CARGO_HOME/registry/src/...` paths in panic messages, so a scan of the whole binary would always fail.
    Homebrew prefixes are refused in the same places.
  - |-
    The installer preflight does more than the plan lists:
    - it also refuses unsupported operating systems (FreeBSD, Windows POSIX shells)
    - it prints its refusal even under `--quiet`
    - `harden` refuses an installer whose own dist `check_glibc` calls are not 2.35, which catches a dropped
      floor in `dist-workspace.toml`

    Each is a small, separately removable piece of `installers.py`.
  - |-
    License notices differ from a plain cargo-about run in five ways:
    - The package header is written by `licenses.sh`, because cargo-about's template data does not name
      the package being rendered.
    - pcre2-sys 0.2.10 ships no PCRE2 `LICENCE` file, so the PCRE2 notice is the license blocks at the top
      of `pcre2_compile.c` and sljit's `sljitLir.c`.
    - cargo-about 0.8.4 leaves out some crates that Cargo links (Linux-only `yeslogic-fontconfig-sys` with
      dlib and libloading, and `rustls-platform-verifier` 0.7.0). `native_notices.py` adds them from
      `cargo tree` for each target.
    - Oniguruma, the ICU headers in tree-sitter, AWS-LC, and ring's BoringSSL notices are appended beside
      PCRE2.
    - The first-party `content-policy` crates lack a `license` field, so `about.toml` declares them
      AGPL-3.0-only.

    The cleaner fix for that last point is to add `license = "AGPL-3.0-only"` to
    `content-policy/{lib,cli}/Cargo.toml`. That was left out of this phase because it changes two packages
    outside the distribution work.
  - |-
    Phase 3: verification mode approves its one package from the run's own inputs and never reads
    `docs/deployment.yaml`. The plan asks for a "nonpublishing manual verification mode for current revisions and
    fixture approvals". A manual run with mode `verify` names a package (and, optionally, a formula name and the
    binstall channel). `distribution.py verify-select` builds a selection for the dispatched commit with
    `mode: verify` and `publishable: false`. It still runs the manifest checks. Three separate things keep it
    away from any write. First, the upload and tap jobs require a release event or mode `recover`. Second,
    they require `publishable == 'true'`, and every consumer refuses a verify record that claims to be
    publishable. Third, `distribution.py publish` has no fixture-forge option and rechecks the Release, tag, and
    proposal against GitHub before it writes. The alternative is to read a fixture policy file from the
    repository. That puts a second approval document in the tree. Changing course means replacing
    `verify-select` and the preflight step's verify branch.
  - |-
    Phase 3: a manual recovery reuses a build only when it is proven to be the original. With `bundle-run-id`,
    the run downloads that run's `distribution-bundle`. It then requires that run to be
    `distribute-binaries.yml` in this repository with a successful `Package` job, and the bundle's selection,
    plan, file set, sizes, and provenance to match this run's fresh preflight. A bundle from a verification run
    never qualifies. Without `bundle-run-id`, a recovery is a fresh rebuild, and `publish --fresh-rebuild`
    refuses once the Release holds any of the expected assets, before anything is downloaded or uploaded. A
    Release-event run that is re-run after a failed upload reuses its own bundle (artifacts survive re-runs).
    The plan does not say how the original run is proven. The alternative is to trust any artifact named
    `distribution-bundle`. Changing course means editing `check_bundle_run`.
  - |-
    Phase 3: same-size byte changes inside a reused bundle are not detected by `check-bundle`. Provenance
    records sizes and dist's own checksums, and the plan forbids hashing in Python. The upload job therefore
    also runs `installers.py check-sums` (the platform `shasum` verifier) on the bundle before publishing, and
    `publish` compares bytes with anything already on the Release. The alternative is a `bh`-based digest
    check, which needs a `bh` binary on the runner.
  - |-
    Phase 3: serialization uses job-level concurrency groups. Uploads queue on
    `distribute-binaries-<package>` and formula updates on one `homebrew-tap` group, never canceling a running
    job. GitHub keeps only ONE pending job per group: if three runs for the same package queue at once, the
    middle pending one is canceled (not the running one). The canceled run is recovered with a `recover` run
    for its tag. The plan asked never to cancel a publication in progress, which holds. The alternative is a
    hand-written lock (for example a lock asset or a ref), which is more moving parts for a rare case.
  - |-
    Phase 3: the workflow gained an `install` job, run after upload and before the Homebrew job. On each of
    the four targets it downloads the published shell installer, requires its bytes to equal the built one,
    and installs into an isolated directory. It sets dist's `<PACKAGE>_UNMANAGED_INSTALL` (flat layout, no PATH
    edits, no receipt), a scratch HOME, and a system-only PATH, and fails if a shell profile is written. Linux
    runs inside the pinned Ubuntu 22.04 runtime image with only `curl`, `ca-certificates`, `xz-utils`, and the
    package's runtime libraries (the installer needs a downloader and xz). The plan requires the formula to be
    published "only after all referenced artifacts are downloadable and install checks pass" but did not name
    this job. Phase 4's clean-runtime verification can extend `scripts/distribution/install_check.sh`.
  - |-
    Phase 3: the Homebrew formula is dist's output with three checked insertions. First, an install-time
    refusal on Linux hosts with glibc older than 2.35 (`OS::Linux::Glibc.system_version`). Second,
    `depends_on macos: :sequoia`, so macOS older than 15 is refused. Third, for `claudine-cli` only,
    `depends_on "alsa-lib" if OS.linux?`. Each anchor must match exactly once; anything else is "formula
    template drift". dist's template always triggers some `brew audit --strict` findings (version redundant
    with the URL, no `test do`, style cops). `tap.py` accepts exactly those and fails on any other finding.
    The alternatives are rewriting dist's template text, or dropping `--strict`.
  - |-
    Phase 3: `brew audit` rejects darkmatter-cli's current Cargo `description` (82 characters, starts with
    "A", ends with a period), and probably claudine-cli's lowercase one. The tap job audits before publishing,
    so enabling `brew` for those packages fails until their descriptions are shortened. The descriptions were
    left unchanged in this phase because they also feed `--help` text. Fix them in the release proposal that
    enables `brew` for those packages.
  - |-
    Phase 3: tap commits are verified in two ways. GitHub must report the signature as valid, and the commit's
    author must equal `HOMEBREW_TAP_AUTHOR` (a new repository variable naming the token owner's login). Then
    the commit is fetched and checked with `git verify-commit` in an isolated GPG home that holds only
    `scripts/distribution/github-web-flow.gpg`. That file was downloaded from `https://github.com/web-flow.gpg`
    on 2026-11-09, and its fingerprints are pinned in `tap.py` (current key
    `968479A1AFF927E37D1A566BB5690EEEBB952194`). When GitHub rotates the key, update the file and the pin
    together. The tap publisher checks credentials before anything else, so `--dry-run` needs them too. A
    prerelease returns `skip` before any credential check.
  - |-
    Phase 3: assets are uploaded with GitHub's REST upload endpoint from `distribution.py`, not
    `gh release upload`. Comparison, ordering (provenance last, so its presence marks a complete set), and the
    `already_exists` race all live in one tested function, and no `--clobber` path exists. Release downloads
    are made without a token, because the URLs are public and the token should not follow the redirect to the
    storage host.
  - |-
    Phase 3: cargo-about 0.8.4 and cargo-binstall 1.26.2 are pinned by SHA-256 in
    `.github/actions/setup-dist/action.yml`. dist stays pinned in `build-images.json`, which `build.sh` also
    reads inside the container. The binstall and cargo-about values are GitHub's recorded asset digests. The
    binstall macOS asset was also checked locally against its minisign signature. The pinned cargo-binstall
    stays off PATH (it is passed as `CARGO_BINSTALL`), so a runner's own copy cannot stand in for it.
  - |-
    Phase 3: binstall verification details beyond the plan.
    - `--no-discover-github-token` stops binstall from reading local credential files.
    - `--allow-insecure-http` is used only by the local `fixture-check`; binstall 1.26.2 otherwise refuses
      `http://` (exit 94).
    - Release-URL evidence is binstall's `resolve: Downloading package url=…` log line, because its "Winning
      URL" line prints a debug struct, not the URL.
    - The `git` route checks policy and metadata against a local checkout of the default branch, while
      binstall itself clones the remote default branch. The post-run version and URL checks catch any drift
      between the two.
  - |-
    Phase 4: the distribution Python suites reach CI as a `repo-deps` companion suite named `distribution`, not
    through a test-toolkit Rust test that runs Python. The plan says the checks run "through the existing
    test-toolkit contract path". A Rust bridge would need PyYAML and semver in every test-toolkit cell on all four
    operating systems, and one 19 s Python run inside a Rust test breaks the 5 s per-test budget. The suites live
    under `scripts/`, which is `repo-deps`'s own directory, and `repo-deps` already runs its Python suites as
    companion suites on `ubuntu-latest`. So the companion recipe is `just distribution-test`, which provisions the
    pinned environment itself. The Rust side keeps the workflow contracts (`distribution_*`), which `just
    distribution-check` runs after the suites. The cost is that the Python suites run on Linux only. To switch to
    the Rust bridge, remove the registry entry and `scripts/Cargo.toml` declaration, add per-suite Rust tests that
    run `run_suites.py`, and provision the pinned venv in `_package-ci.yml` for test-toolkit cells.
  - |-
    Phase 4: an edit to `docs/deployment.yaml`, `dist-workspace.toml`, `about.toml`,
    `.github/distribution/build-images.json`, or `just/distribution.just` now selects all of `repo-deps` (its Rust
    tests and every companion suite), through `SUITE_OWNER_PATHS`. Before, these files selected nothing, so an edit
    to the approval policy ran no check at all. Narrowing to only the `distribution` suite would need a new kind of
    planner rule (a companion-only selection), which is more planner code for files that change rarely. Reversing
    means deleting the five `SUITE_OWNER_PATHS` entries and the matching `test_affected_scope.py` test.
  - |-
    Phase 4: a skipped test fails the distribution suites (`run_suites.py`), and the three tool-gated tests (ruby,
    gpg, shellcheck) use the shared `tool_guard.require_tools` instead of `skipUnless`. The plan says absent tools
    must not make selected tests silently skip. Developers therefore need ruby, gpg, shellcheck, shasum, and cargo
    locally to run `just distribution-test`; `ubuntu-latest` and this macOS host have all of them. The alternative,
    skipping locally and failing only in CI, needs a CI-only declaration on the shared companion step.
  - |-
    Phase 4, robustness details the plan left open (subagents A and B):
    - a dist plan asset's `path` must equal its `name`, so every file sits at the top of the archive (true of the
      real plans). To loosen it, drop the equality check in `validate_plan`.
    - Release asset names must be a single file name (no `/`, `\`, `.`, `..`).
    - a selection's `tag` is still not checked against `{package}-v{version}` beyond what bundle reuse compares.
    - `inspect_archive.py` stops before running readelf or otool when the archive's member list already failed. That
      keeps "nothing launched" for unsafe archives, but a broken archive no longer also reports linkage problems.
    - `native_notices.py` now imports `distribution.py`, so it needs the pinned venv. The workflow already passes it
      as `PYTHON`; `inspect_archive.py` keeps its own stdlib loader because it runs in the build container.
    - `installers.py` no longer defaults the macOS floor to 15.0 when `build-images.json` is missing.
    - `installers.py` and `verify_binstall.py` `main` gained keyword-only test seams (`build_images=`, `runner=`,
      `base_env=`), not new CLI flags.
  - |-
    Phase 4, runtime verification details (subagent C):
    - `verify.sh` never builds. Without `--artifacts` it exits 2 and explains how to produce them (the Phase 2
      `stage` + `build.sh` steps). Adding a build mode is possible but would make `distribution-verify` take minutes.
    - the Ubuntu 20.04 image for the glibc-refusal check is pinned by digest inside `verify_runtime.py`
      (`OLD_GLIBC_IMAGE`), not in `build-images.json`. Moving it there means adding a key that `build-matrix` must
      then validate.
    - the installer reads archives through dist's `<PACKAGE>_DOWNLOAD_URL` override with a `file://` URL, so
      containers run with no network. The installer image adds only `curl` and `xz-utils` (the installer refuses to
      start without a downloader); the archive-only check uses the pinned 22.04 image unchanged.
    - macOS runtime libraries are observed with `DYLD_PRINT_LIBRARIES`, which also works on a clean VM without
      developer tools. Allowed locations are `/usr/lib/` and `/System/Library/`, wider than the direct-link list,
      because the system loader pulls in its own private frameworks.
    - x86_64 Linux under emulation on this arm64 Docker host is reported `pending-infrastructure`, never as evidence.
  - |-
    Phase 5: the rollout gate lives in preflight and reads the default branch's ledger. A release or recover run for
    any package other than biscuit-hash-cli is refused (exit 2, before anything is built) unless
    `.github/distribution/rollout.json` on the default branch holds a verified biscuit-hash-cli entry, and GitHub reports
    that entry's run as a completed, successful `distribute-binaries.yml` run in this repository. The plan said only
    "reject widening". Verification runs are not gated, because they publish nothing. The run's commit is not compared
    with the entry's commit, because a recover run is dispatched from a branch and so its run commit differs from the
    tag. The companion suite also checks the real `docs/deployment.yaml` against the real ledger, so approving a second
    package before the ledger permits it fails CI on the pull request, not just at release time. The alternative is a
    human-only rule in the docs. Changing course means removing `gate_rollout` from `_command_preflight`.
  - |-
    Phase 5: the ledger's shape. `first_package` is fixed to biscuit-hash-cli. `verified` entries record only passes:
    every one of the four targets must be `pass`, and archive and shell installer must be `pass`. Homebrew may be
    `pass`, `unchanged` (a prerelease, where the formula is never updated), or `disabled`, and binstall may be `pass`
    or `disabled`. `pending` holds named outstanding items per package (approval, release-app, homebrew-tap,
    clean-runtime-macos, x86_64-runtime, wsl2-runtime, live-release, first-chain, brew-description). Unknown fields
    and duplicate tags are refused. The plan did not fix a schema. One limit: when Homebrew is enabled and the tap job
    succeeds, the candidate records `pass` even if `tap.py` decided not to change the formula (an older retry); the
    tap report artifact shows which happened. Changing the shape means editing `parse_rollout` and the real ledger
    together.
  - |-
    Phase 5: the candidate entry is a new final workflow job, `rollout-candidate`. It runs only for a publishable
    release or recover run after every target's install passed, reads each channel job's result, and uploads the
    `distribution-rollout-candidate` artifact plus a run-summary copy. It needs only read access. The plan said the
    first bh run "creates its ledger candidate as a workflow artifact". Nothing writes the ledger automatically; a
    reviewed change copies the entry in, as the plan asks.
  - |-
    Phase 5: archives may carry the package's own `CHANGELOG.md`. dist 0.34.0 adds a changelog it finds in the
    package directory, and `sniff/cli/CHANGELOG.md` exists, so the sniff plan failed `validate-plan` when all eight
    packages were planned. `validate-plan` and `inspect_archive.py` now accept exactly one non-executable
    `CHANGELOG.md` and still refuse any other file. The alternative is turning off dist's auto-includes and listing
    LICENSE and README explicitly, which changes every package's overlay and both plan fixtures. That keeps the release
    changelog out of archives. Changing course means removing `CHANGELOG_NAME`/`OPTIONAL_FILES` and their tests.
  - |-
    Phase 5: `just lint` was not run. At the repository root it lints every package area, and the plan says not to run
    it because it includes formatting. The scoped equivalents were run instead: clippy with `-D warnings` for
    test-toolkit and repo-deps, pyflakes and shellcheck on the distribution scripts, and actionlint 1.7.12 on the
    workflow.
  - |-
    Phase 5: `native_notices.run_cargo` had no subprocess timeout (found in the final diff review). It now allows 30
    minutes, because `cargo metadata` and `cargo tree` may fetch the registry on a cold runner, and reports a timeout
    as a located notice error. A shorter limit would risk failing a legitimate cold-cache run.
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

# Implementation Log for 2026-10-28-publish (5 phases)

## Phase 1

- Started 2026-11-09. Read the plan, spec, and spikes S1/S2.
- `scripts/ci/release_path.py` now exists in this checkout (the plan said it did not); preflight reuses its `select_publication` identity checks for the reviewed-proposal proof and its `peeled_tag_target` for tag resolution instead of restating them. Its initial release eligibility (`APPROVED_RELEASE_SET` = `git-wt-cli`, `sniff-detection-cli`) and `release-plz.toml` are untouched.

### Wave 1 — Implement policy preflight

- New `scripts/ci/distribution.py` (importable module plus CLI) and `scripts/ci/requirements-distribution.txt` (`PyYAML==6.0.2`, `semver==3.0.4`).
    - One strict YAML loader: `StrictYamlLoader` subclasses `SafeLoader` with YAML 1.2-style plain-scalar resolution (only `true`/`false` are booleans; `on`, `yes`, `True` stay strings; no timestamps or merge resolution). `_convert` rejects duplicate keys, non-string keys, merge keys (`<<`), unsupported tags, and recursive aliases. It also requires exactly one mapping document and keeps `line:column` for every field path. JSON uses one `load_json` (rejects duplicate keys, non-finite numbers, and trailing content). TOML uses `tomllib`.
    - Public API: `load_policy(path, packages)`, `select_release(event, policy, packages)`, `render_dist_config(selection)`, `validate_plan(selection, plan)`, `reconcile_assets(expected, existing, read_remote)`, `select_tap_update(candidate, installed)`, plus `load_workspace`, `gather_facts`, and `check_dist_base`. The CLI subcommands are `preflight`, `config`, `validate-plan`, `reconcile-assets`, and `tap-update`. JSON goes to stdout (and `--output`); errors go to stderr as `file:line:col: field.path: message`. Exit codes: 0 selected/ignored/decided, 1 asset conflict, 2 invalid, 3 pending setup.
    - Parsing and selection have no side effects. Forge reads go through an injected `Forge` (`GitHubForge` for real runs, `FixtureForge` for JSON fixtures), and `cargo metadata`/`git` run only in the CLI layer. A library, integration, or foreign tag is ignored before any forge call.
    - `config` and `validate-plan` accept only a `Selection` built from a `status: selected` record. `render_dist_config` raises `TypeError` for anything else.
    - Preflight keeps byte-identical policy snapshots (`--snapshot-dir`); no policy digest is computed.
- Phase 2 and Phase 3 surface added now so the five subcommands exist: `validate_plan`, `reconcile_assets`, and `select_tap_update` are implemented and tested as pure functions. `validate_plan` uses dist-manifest field names (`announcement_tag`, `releases[].app_name/app_version/artifacts/hosting.github.artifact_download_url`, `artifacts{}.kind == "executable-zip"`, `target_triples`, `assets[].kind == "executable"`). These have **not yet been checked against a real dist 0.34.0 plan**; Phase 2 owns that.

### Wave 1 — Prepare package metadata

- The eight capable manifests now set `repository = "https://github.com/yankeeinlondon/rusty-biscuit"` (`darkmatter-cli` previously named `yankeeinlondon/dockhand`), plus `homepage`, a `description` where one was missing, and `readme = "../README.md"` for `bh` and `wt`, which have no package README. Each also carries the checked-in `[package.metadata.binstall]` block with `crate-meta-data` disabled (rule 15). No approval or channel value in `docs/deployment.yaml` changed.
- `worktree/cli/Cargo.toml`: added an explicit `[[bin]] render_dirty_tree` with `required-features = ["terminal-tests"]` (rule 7). `cargo build --locked -p git-wt-cli --features terminal-tests --bins` succeeds, and `cargo metadata` reports `render_dirty_tree` gated and `wt` ungated.
- Root `Cargo.toml`: added `[profile.dist]` (`inherits = "release"`, `strip = true`, `lto = "thin"`); developer profiles unchanged.
- New `dist-workspace.toml`: workspace `dist = false`, `precise-builds`, `pr-run-mode = "skip"`, `create-release = false`, `install-updater = false`, `source-tarball = false`, `checksum = "sha256"`, `.tar.xz`, four targets, `allow-dirty = ["ci"]`, and no publish jobs. Preflight rejects any checked-in `package.metadata.dist`.
- **S1 departure:** S1 concluded that the test-helper binaries in `darkmatter-cli`, `claudine-cli`, and `sniff-detection-cli` must move out before dist can build those packages. Rule 6 supersedes that. dist 0.34.0's `DistGraphBuilder::add_variant` replaces the packageables with `[package.metadata.dist.binaries]`, so the generated opt-in sets `"*" = ["<command>"]` and no fixture moves. S1's report is unedited. A real `md` packaging run (Phase 2) must still prove this.
- No comment claimed fixture binaries would be distributed by default, so no comment needed correcting. The existing `required-features` comments ("stays out of default builds and `cargo install`") remain accurate.

### Requirement-to-test mapping (`scripts/ci/test_distribution.py`, 82 tests)

Policy cases start from the real `docs/deployment.yaml` and real `cargo metadata`, changing one value per case.

| Requirement | Tests |
|---|---|
| Real policy is a valid all-disabled control (checkpoint) | `PolicyTests.test_the_real_policy_is_a_valid_all_disabled_control`, `SelectReleaseTests.test_the_real_policy_ignores_every_release`, `CliTests.test_preflight_with_the_real_policy_is_ignored` |
| One approved entry selects exactly one command (checkpoint) | `SelectReleaseTests.test_a_one_entry_approval_selects_exactly_one_command`, `…every_roster_package_passes_its_manifest_checks_when_approved` (all 8), `CliTests.test_preflight_selects_and_snapshots_the_policies` |
| Generated config cannot select another package (checkpoint) | `SelectionAndConfigTests.*` (overlay names only `md`; non-selected or forged records rejected) |
| YAML document shapes (rule 3) | `YamlLoaderTests.*`: duplicates at root, package, and channel depth; `k:`/`k: null`/`~`; `on`/`yes`/`True`/quoted `'false'`; second document; trailing garbage; `!!binary`/`!custom`/`!!set`; merge key; non-string keys; empty, list, or scalar roots; aliases and recursion |
| JSON/TOML document shapes | `JsonAndTomlLoaderTests.*` |
| Approval and channel fields (rule 4) | `PolicyTests.*`: `packages` absent, null, list, or `{}`; unknown top-level key, package, or field; `crate` absent, mismatched, int, `""`, or null (and null when approved); `approved` absent→false, null, empty, `'true'`, `on`, int, or list; channel absent, true, name, quoted `'false'`, null, int, list, bad name, or `""`; every channel wired to the shared validator; channel on an unapproved entry; duplicate resolved names |
| Capability roster and manifests (rules 5–7, 15) | `WorkspaceTests.*`, `SelectReleaseTests.test_manifest_checks`, `…binstall_channel_and_manifest_must_agree`, `…a_checked_in_dist_opt_in_anywhere_is_refused`, `DistBaseTests.*` |
| Release, revision, and policy checks (rule 9) | `SelectReleaseTests`: library, integration, and foreign tags ignored; malformed roster tags error; foreign repository ignored; current approval cannot authorize; revoked approval errors; channel intersection; prerelease never updates the formula; unsupported channels refused; release missing, draft, or wrong tag; tag absent; checkout ≠ tag; proposal missing or wrong head; pending setup; recovery mode |
| Event and forge gathering | `GatherFactsTests.*` (ignored events never reach the forge; located event errors; API must agree with the event), `GitHubProposalProofTests.*` (stubbed API; proof reuses `select_publication`; HTTP errors are not absence) |
| CLI wiring per loader | `CliTests.*` (located malformed policy, event, forge, and metadata; exit codes 0/1/2/3; required `GITHUB_REPOSITORY`) |
| Immutable assets / tap ordering (later phases' pure core) | `ReconcileAssetsTests.*`, `TapUpdateTests.*` (0.9 vs 0.10, prerelease, equal-version byte mismatch) |

- Mutation check: disabling the unapproved-channel rule, the merge-key rule, the revoked-approval rule, or making the tap comparison `>=` each fails at least one test.
- `pyflakes` is clean on both files.
- The suite runs with `python3 -m venv /tmp/dist-venv && /tmp/dist-venv/bin/pip install -r scripts/ci/requirements-distribution.txt && /tmp/dist-venv/bin/python scripts/ci/test_distribution.py`. It is not yet wired into `just`/CI (Phase 4: `just distribution-check` and the test-toolkit bridge). The host's `python3` has PyYAML but not `semver`, so the import fails loudly rather than skipping.

### Gates

- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `change_class: package`, 12 packages (`repo-deps`, `test-toolkit`, `biscuit-file-cli`, `claudine`, `claudine-cli`, `darkmatter`, `darkmatter-cli`, `darkmatter-lsp`, `simplified-schema`, `sniff-detection-cli`, `tree-hugger-cli`, `zed-dmls-cli`). The tree also holds earlier staged work from the release-path fix; scoping only this phase's files gives the same 12. The gates were therefore run, not skipped.
- `just test biscuit-hash-cli biscuit-file-cli tree-hugger-cli git-wt-cli sniff-detection-cli darkmatter-cli darkmatter-lsp claudine-cli`: **12562 passed**, 30 skipped, 0 failed. Nine tests took 5.1–6.1 s while the host load average was about 195 (`claudine-cli::l1 agent_text_is_data::*`, `error_guards::production_sources_pass_every_scan_backed_guard`, `level1_provider_overlay_home::*`, and `darkmatter-cli::l1 claudine_schema_bundle::*`). They are recorded as load-sensitive; this phase's metadata-only edits do not touch them.
- Dependents that read manifests and CI scripts: `just test repo-deps test-toolkit`. The first run failed one guard, `ci_workflow_contracts::no_shipped_reader_consumes_a_retired_environment_list_projection`. It flags any `["matrix"]` read in `scripts/ci/*.py`, and preflight's output key was `matrix`. I renamed that key to `build_matrix`; the rerun gave **1021 passed**, 0 failed. Other dependents (`claudine`, `darkmatter`, `simplified-schema`, `zed-dmls-cli`) compile against unchanged sources. Only `CARGO_PKG_*` metadata of the eight CLIs changed, and `claudine-cli`/`darkmatter-cli`, which consume them, passed above.
- Lint (`just -f <area>/justfile lint`): biscuit-hash, biscuit-file, tree-hugger, worktree, sniff, and claudine pass. darkmatter: library, CLI, dmls, and zed-dmls-cli clippy/fmt pass. Its final `check-zed` step fails because this host lacks the `wasm32-wasip2` rustup target (`rustup target add wasm32-wasip2`). That is a host environment gap, not a code finding.
- `scripts/ci/test_distribution.py`: 82 passed (temp venv with the pinned requirements); pyflakes clean.
- Not run: L2 `level2_dirty_tree` (Phase 5's scoped validation owns it). The gated helper builds with `--features terminal-tests`.
- No local OS test rigs (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS`) are configured; macOS evidence only.

## Phase 2

- Started 2026-11-09. No local OS test rigs are configured (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` unset); Linux aarch64 evidence comes from Docker on this macOS arm64 host using the pinned Ubuntu 22.04 digest.
- Pinned dist: downloaded `cargo-dist-aarch64-apple-darwin.tar.xz` for v0.34.0 and its `.sha256`; `d1eec30e63eed23e5a5bd48fe96eae314799811bffca254ec1d2264638ba1890` matched. `dist --version` reports `cargo-dist 0.34.0`.

### Wave 2 — Generate dist inputs

- Real `dist plan --tag=<tag> --output-format=json` runs (bh plain, md with the brew channel) in a staged full copy of the working tree, with the module's own overlay applied. Both are checked in unedited as `scripts/ci/fixtures/distribution/dist-plan-biscuit-hash-cli.json` and `dist-plan-darkmatter-cli-brew.json`.
- What the real plan corrected in Phase 1's guesses:
    - GitHub hosting is `releases[0].hosting.github.{artifact_base_url, artifact_download_path, owner, repo}`; there is no `artifact_download_url`.
    - Archive assets are `license` (`LICENSE`, auto-included from the workspace root), `readme` (the package `readme`), `executable`, and `unknown` for `include`d files. Each archive names its `.sha256` sidecar in `checksum`.
    - Release artifacts are exactly: four archives, four sidecars, `sha256.sum`, `<package>-installer.sh`, and `<formula>.rb` only when brew is effective. `validate_plan` now requires that exact set, so a PowerShell installer, an extra archive, or a missing formula is refused. It also checks `dist_version`, `announcement_is_prerelease` against the selected version, each archive's single target, checksum name, and contents (exactly one executable, LICENSE, README, and the two generated files, nothing else), and that the shell installer covers all four targets.
- **dist 0.34.0 ignores unknown configuration keys silently.** A `bogus-key = 1` in the opt-in produced the same plan with exit 0, so "dist accepted it" proves nothing. Each overlay key was checked against pinned `config/v0.rs` and by its effect: `binaries "*"` leaves only `md` in darkmatter-cli's archives (rule 6 proven at plan level), `formula` produces `darkmatter-cli.rb`, and `include` adds `SOURCE.txt` and `THIRD-PARTY-NOTICES.md` to every archive.
- **`min-glibc-version` must be workspace-level.** Pinned `platform.rs::get_glibc_override` reads `dist.inner.config.builds.min_glibc_version`, the workspace configuration. With the setting only in the package opt-in, the global build still emitted `check_glibc "2" "31"` (dist's default). With `[dist.min-glibc-version]` in `dist-workspace.toml` it emitted `check_glibc "2" "35"`. The floor therefore lives in the checked-in `dist-workspace.toml` and in `DIST_BASE` (one reviewed authority), and the generated overlay carries no glibc key.
- The global step embeds archive checksums in the installer and writes `sha256.sum` only when each target's dist JSON is present as `target/distrib/<target>-dist-manifest.json` (as dist's own CI does). Without it, `sha256.sum` was empty. `sha256.sum` lists archives as `<sha256> *<name>`; it did not list the shell installer.
- dist 0.34.0 never passes `--locked` (pinned `build/cargo.rs`): it runs `cargo build --profile dist --message-format=json-render-diagnostics --target <t> --package <p>` and overwrites `RUSTFLAGS`. Lockfile use is proven by the build script instead (see Wave 2 build).
- `distribution.py` additions:
    - `render_dist_config` adds `include = [".distribution/SOURCE.txt", ".distribution/THIRD-PARTY-NOTICES.md"]` (relative to the package directory).
    - `stage` / `stage_distribution`: requires the checkout to be at the selected commit with no tracked modification, copies only tracked files (symlinks and modes preserved; untracked `target/` never staged; a submodule is refused), appends the opt-in, writes `SOURCE.txt` (exact `tree/<tag>/<package dir>` and `commit/<sha>` URLs) and the notices, then `verify_staging` compares every staged file with the checkout byte for byte. The only allowed differences are the appended opt-in and the two generated files, so the staged production sources and `Cargo.lock` are the tagged ones. `--record-dir` keeps `overlay.diff` and `dist-config.toml` for the workflow artifacts. The checkout is never written.
    - `provenance` / `build_provenance`: validates the plan again, requires the assets directory to hold exactly the planned assets, reads checksums from dist's `sha256.sum` cross-checked with every `.sha256` sidecar (no digest computed), requires one build environment per target sharing one `rustc`, and emits `distribution-provenance.json` in sorted order with no run URL, run ID, or time.
- The dist plan's generated CI matrix (`ci = "github"`) already names the plan's runners: `macos-15`, `macos-15-intel`, `ubuntu-22.04`, `ubuntu-22.04-arm`.
- Defect found and fixed during integration: dist ends `sha256.sum` (and each sidecar) with a blank line, which `read_checksums` first rejected. It now skips blank lines; `ProvenanceTests.test_dist_real_unified_checksum_file_is_read` reads dist's real file (`scripts/distribution/fixtures/dist-sha256.sum`).

### Wave 2 — Build target jobs (delegated, then integrated)

- `.github/distribution/build-images.json`: rules 11–13 as data. It holds the image index and per-arch digests, runner labels, apt build and runtime lists (claudine extras separate, `python3` under a separate `tool_packages` key), `MACOSX_DEPLOYMENT_TARGET` 15.0, `PCRE2_SYS_STATIC=1`, max glibc 2.35, dynamic-library allowlists, banned Homebrew/build-path strings, and dist 0.34.0 SHA-256 values for all four platforms.
- `scripts/distribution/build.sh`: `build`, `smoke`, `container` (local Docker wrapper), and `smoke-container` modes.
    - Isolation: clears the wrapper variables and `RUSTFLAGS`, uses a fresh `CARGO_HOME`, and refuses an ambient `.cargo/` config. It pins the toolchain from the staged `rust-toolchain.toml` and requires dist 0.34.0.
    - Lockfile proof: `cargo metadata --locked`, then a `Cargo.lock` byte comparison around the build.
    - Build checks: requires exactly one `--package=<pkg>` selection line and no `--workspace`/`--all-features`/`--features`, and the dist manifest must name one app and one executable. It copies the manifest to `target/distrib/<target>-dist-manifest.json`.
    - Outputs: writes `build-env-<target>.json` in the shape `provenance` consumes, then inspects and smoke-tests.
- `scripts/distribution/inspect_archive.py`: stdlib only.
    - Archive: exactly one root and five entries; no traversal, absolute paths, symlinks, or hard links; one executable.
    - Binary header: ELF/Mach-O format and CPU type checked; fat binaries refused.
    - ELF: `GLIBC_` requirements at most 2.35, and NEEDED entries from the allowlist (no PCRE2).
    - Mach-O: minos 15.0, no rpath, dependencies only from `/usr/lib` and system frameworks, and no Homebrew or build paths in load commands.
    - `SOURCE.txt` must carry the exact tag and commit URLs.
- Renamed from the plan's `inspect.py`: that name shadows Python's standard `inspect` module, so every script in `scripts/distribution/` (installers, tests) needed `sys.path` workarounds. All workarounds were removed after the rename.

### Wave 3 — Installers and licenses (delegated)

- `scripts/distribution/installers.py harden` inserts a preflight into dist's generated shell installer.
    - Placement: it runs after argument parsing and before architecture detection, archive selection, `mktemp`, and download.
    - Anchors: five fixed template lines, each required exactly once and in order; drift is a named error.
    - Checks: glibc comes from `getconf GNU_LIBC_VERSION`, falling back to `ldd --version`. It compares major then minor numerically against 2.35 and refuses musl, an unknown libc, unsupported architectures and OSes, and macOS older than 15.0 (`sw_vers`). It also refuses an installer whose own `check_glibc` calls are not 2.35.
- `installers.py check-sums` verifies every `.sha256` sidecar and `sha256.sum` entry against the bytes with `shasum -a 256 -c`, the platform verifier. It requires full coverage of archives and refuses stale entries, missing files, and symlinks.
- `about.toml` (cargo-about 0.8.4) and `scripts/distribution/about.hbs`: four targets, dev and build dependencies ignored, an explicit accepted-license list, and one clarification for the first-party `content-policy`, whose manifests lack a `license` field.
- `scripts/distribution/licenses.sh` and `native_notices.py`.
    - Preconditions: refuses any cargo-about other than 0.8.4 and any unknown package before doing work.
    - Generation: runs `cargo-about generate --locked --fail` and checks that `Cargo.lock` is unchanged.
    - Gaps filled: adds crates that cargo-about's graph misses but Cargo links (from `cargo tree -e normal` per target), and appends native notices (PCRE2+sljit, Oniguruma, ICU headers in tree-sitter, AWS-LC, ring's BoringSSL).
    - Unreviewed native code: any `links`/`cc`/`cmake` crate that is not catalogued or reviewed fails generation.
    - Results: notices were generated for all eight packages; the review summary is `evidence/phase-2/license-review.md`.

### Evidence and checkpoint (`features/2026-10-28-publish/evidence/phase-2/README.md`)

- Pinned plans for `bh` and `md` select exactly one shipping command (the checked-in fixtures; `ValidatePlanTests.test_the_real_plans_ship_one_command_with_notices`).
- Real archives exclude every fixture: the `md` archives for aarch64-apple-darwin and aarch64-unknown-linux-gnu contain only `md` plus LICENSE, README.md, SOURCE.txt, and THIRD-PARTY-NOTICES.md.
- aarch64-apple-darwin, native, `bh` and `md`: minos 15.0, system libraries only, and smoke `--version`/`--help` passes.
- aarch64-unknown-linux-gnu, pinned container: `bh` needs at most GLIBC_2.34 and `md` exactly GLIBC_2.35 (the floor). NEEDED is only libc/libm/libgcc_s/the loader. Smoke and `ldd` pass in a clean runtime container of the same digest (no build tools, `--network none`). The `md` container build took 7 minutes.
- Integration: both rebuilt with the real notices (archive notices byte-identical to `licenses.sh` output). The `md` global step produced the installer (`check_glibc "2" "35"`) and `darkmatter-cli.rb`. `harden` and `check-sums` pass on the real output.
- **Pending infrastructure:** native x86_64-apple-darwin (needs `macos-15-intel`; a local cross-build of `bh` passes inspection but is not native evidence), x86_64-unknown-linux-gnu (needs an amd64 runner; not attempted under emulation), hosted-runner `ImageOS`/`ImageVersion`/Xcode values, and claudine/other packages' archive builds (Phase 5 rollout). No output was uploaded to any Release.

### Requirement-to-test mapping (Phase 2)

| Requirement | Tests |
|---|---|
| Overlay keys and their effect (rule 6; installer/channel settings) | `SelectionAndConfigTests.test_config_names_only_the_selected_package_and_command` (exact overlay; no package-level glibc key), `ValidatePlanTests.test_the_real_plans_ship_one_command_with_notices` |
| glibc floor in the reviewed base | `DistBaseTests.test_a_changed_or_extra_setting_is_refused` (2.34, missing target) |
| Plan validation against real dist output (control + one edit per cell) | `ValidatePlanTests.*`: real-plan controls; channel correspondence both directions; 31 single-edit refusals covering absent, null, wrong type, wrong element, and empty shapes; prerelease; non-mapping plan |
| Staging: byte-clean original, only the opt-in changes, round trip | `StageTests.*` (temporary git repo: copy fidelity incl. symlink/mode/untracked; staged manifest parses and equals the overlay; tampered lockfile/extra file/edited manifest refused; wrong commit, dirty checkout, reserved path, empty/missing notices, non-empty destination, no final newline) |
| Provenance determinism and dist-sourced checksums | `ProvenanceTests.*` (deterministic bytes; exact key set without run data; missing/extra assets; stale sidecar; malformed/duplicate sum lines; real dist `sha256.sum`; build-env coverage, duplicates, toolchain mismatch, wrong types) |
| CLI wiring (`stage`, `provenance`) | `CliTests.test_stage_and_provenance_commands` (control, located refusal with nothing staged, `--write` round trip) |
| Archive/ABI inspection | `scripts/distribution/test_inspect.py` (39 tests: each archive and linkage rejection, real `readelf`/`otool` fixtures, build-images contract against `distribution.TARGETS`/`MIN_GLIBC` and the plan's digests) |
| Installer preflight and checksums | `scripts/distribution/test_installers.py` (38 tests: anchors, drift, idempotence refusal, behavior under fake commands for glibc 2.34/1.99/2.35/2.39/2.100, musl, missing ldd, garbage, unsupported arch/OS, macOS 14.6/15.0/15.10/26.1; check-sums stale/missing/mismatched cases, real dist blank-line format) |
| License notices | `scripts/distribution/test_licenses.py` (30 tests: about.toml contract, version/package refusals before work, native-notice selection with and without pcre2-sys, missing-crate detection) |

- Suites: `test_distribution.py` 97, `test_inspect.py` 39, `test_installers.py` 38, `test_licenses.py` 30, all passing with `/tmp/dist-venv/bin/python`. pyflakes is clean on every Python file; `bash -n` and shellcheck are clean on `build.sh` and `licenses.sh`. These suites are not yet wired into `just`/CI (Phase 4).
- The behavior tests in `test_installers.py` have run only on macOS (they fake Linux via stub commands); they are written to run on Linux too.

### Gates

- Scope calculator over the whole uncommitted tree (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `change_class: package`, 12 packages (`repo-deps`, `biscuit-file-cli`, `claudine`, `claudine-cli`, `darkmatter`, `darkmatter-cli`, `darkmatter-lsp`, `simplified-schema`, `sniff-detection-cli`, `test-toolkit`, `tree-hugger-cli`, `zed-dmls-cli`). The tree still holds Phase 1's uncommitted manifest edits. Phase 2's own files alone select `repo-deps` and `test-toolkit`, so the gates were run, not skipped.
- `just test repo-deps test-toolkit`: **1021 passed**, 5 skipped, 0 failed (36 s). The ten other packages' sources and manifests were not touched in this phase; Phase 1 gated them (12562 passed).
- Lint: neither `scripts/` (repo-deps) nor `tools/test-toolkit` has a `lint` recipe; `cargo clippy --locked -p repo-deps --all-targets -- -D warnings` and the same for `test-toolkit` pass. Python and shell lints as above.
- No local OS test rigs (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS`) are configured. Evidence is from macOS plus Docker `linux/arm64`.

## Phase 3

- Started 2026-11-09. Three parallel subagents each owned their files: (D) the publication and bundle functions in `distribution.py`, (T) the Homebrew tap publisher, and (B) the binstall driver. I wrote the workflow, the setup action, the install check, and the Rust workflow contracts, then integrated everything. No local OS test rigs (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS`) are configured.
- Nothing was published. No workflow ran on GitHub, no Release was touched, and no tap commit was made. Live acceptance stays pending: no package is approved, and the release App, tap repository, `HOMEBREW_TAP_TOKEN`, and `HOMEBREW_TAP_AUTHOR` are not set up.

### Wave 4 — Release workflow (`.github/workflows/distribute-binaries.yml`)

- Triggers: `release: published`, plus `workflow_dispatch` with modes `recover` (an existing tag, optional `bundle-run-id`) and `verify` (the dispatched revision, fixture approval of one named package).
- Jobs, each with its own permissions:
    - `preflight` runs `distribution.py preflight`, or `verify-select` in verify mode, then `build-matrix`. Exit 3 (pending setup) fails the job with the setup instruction.
    - `prepare` writes the notices with pinned cargo-about, then runs `stage`, `dist plan`, and `validate-plan`.
    - `build` ×4 runs on native runners. Linux builds use `build.sh container` (the pinned image); macOS builds use `build.sh build`.
    - `package` generates the global artifacts, runs `harden`, `check-sums`, and `provenance --write`, then `bundle` and `check-bundle`, and keeps `distribution-bundle` before any upload.
    - `reuse-bundle` (recovery with `bundle-run-id`) checks the original run's bundle and Package job.
    - `smoke` ×4 runs the archived command; Linux runs in the clean runtime image.
    - `upload` (the only `contents: write` job) runs `publish`.
    - `install` ×4 installs from the Release URL into an isolated root.
    - `binstall` ×4 runs `verify_binstall.py manifest`, only when that channel is on.
    - `homebrew-tap` runs `tap.py prepare` and `audit`, then `publish-tap.sh`. It is the only job that sees `HOMEBREW_TAP_TOKEN`.
- Concurrency: `upload` queues on `distribute-binaries-<package>` and `homebrew-tap` on `homebrew-tap`, both with `cancel-in-progress: false`.
- Event values reach shell code only through `env:`. Every checkout drops its credentials. Each job that builds proves `HEAD` equals the selected commit.
- `.github/actions/setup-dist/action.yml` provides the venv with the pinned requirements, dist (pinned SHA-256 from `build-images.json`), cargo-about 0.8.4 (pinned SHA-256), and cargo-binstall 1.26.2 (pinned SHA-256, kept off PATH as `CARGO_BINSTALL`).
- `scripts/distribution/install_check.sh` (`native` / `container`): downloads the installer from the Release and requires it to be byte-identical to the built one. It installs with dist's `<PACKAGE>_UNMANAGED_INSTALL`, a scratch HOME, and a system-only PATH. It requires exactly the one command installed and no shell profile written, then runs `--version` and `--help`. Sanity runs with stub installers covered the pass case, an extra file, and a written `.zshrc`.
- actionlint 1.7.12 (with shellcheck) is clean on the workflow and the local action.

### Wave 4 — Immutable upload (`scripts/ci/distribution.py`, subagent D)

- `verify-select`, `build-matrix`, `bundle`, `check-bundle`, and `publish`.
    - `select_release` records carry `publishable: true`.
    - `Selection.from_json` requires `mode` to be release, recovery, or verify, and `publishable == (mode != "verify")`.
- `publish_release_assets` checks in this order:
    - verify selection refused;
    - current policy re-read: approval and every channel still hold;
    - Release id, draft state, tag commit, and proposal re-read from GitHub;
    - complete `reconcile_assets` before any upload: conflict → exit 1 and nothing uploaded; `--fresh-rebuild` with any expected asset present → refused before downloading anything.
- Uploads then add only missing assets, provenance last. An `already_exists` race reconciles that one asset. A final full reconcile must find every asset byte-identical. The only write call is a POST to the uploads endpoint, and downloads carry no token.
- `check_bundle_run` checks a reused run: workflow path, repository, run id, and exactly one `Package` job with conclusion `success`.

### Wave 5 — Tap publisher (`scripts/distribution/tap.py`, `publish-tap.sh`, subagent T)

- `prepare` validates dist's formula strictly and applies checked single-match insertions (see decisions):
    - validation: class name, version, four exact URLs, each `sha256` equal to `sha256.sum`, the license, and only the shipping command installed;
    - insertions: the glibc 2.35 refusal, `depends_on macos: :sequoia`, and claudine's `alsa-lib`.
- `publish` order:
    - credentials, then the viewer login equals `HOMEBREW_TAP_AUTHOR`;
    - prepare, then download every referenced URL and compare its bytes;
    - read the branch head and the installed formula at that head, then `select_tap_update`;
    - `createCommitOnBranch` with `expectedHeadOid` and one file (headline `<name> <version>`, no trailers), with no retry;
    - require a valid signature and author, then `git verify-commit` in an isolated GPG home holding only the pinned web-flow key.
- `audit` runs `brew tap-new`, `trust`, `audit --strict`, and `style` locally. Real Homebrew on this host found zero findings in the inserted lines; dist's own template findings are accepted by exact message. darkmatter-cli's `desc` fails the audit (see decisions). `ruby -c` passes on the prepared bh, claudine, and darkmatter formulas.

### Wave 5 — Binstall wiring (`scripts/distribution/verify_binstall.py`, subagent B)

- `git` and `manifest` routes:
    - Before any network access: the binstall version is exactly 1.26.2; the policy enables the channel and the manifest metadata is in its enabled form; for the manifest route, the tag version equals the manifest version and HEAD is the tag commit.
    - Isolated `CARGO_HOME`/`HOME`, `--strategies crate-meta-data`, and no compile fallback.
    - After the run: exactly the shipping command is installed, its `--version` is correct, and the release URL appears in binstall's download line.
- `fixture-check` (explicit acceptance, not in the fast suite) **passed** with the real cargo-binstall 1.26.2 (aarch64-apple-darwin).
    - Binary checks: its SHA-256 equals GitHub's asset digest, its minisign signature verified, and the signed comment names the `v1.26.2` tag commit.
    - Optional helper: the feature-gated helper's absence is tolerated and only the command is installed.
    - Missing required binary: binstall exits 94 and installs nothing.
    - Evidence is in `/tmp/binstall-fixture-check/` (not retained in the repo).
- With the real `docs/deployment.yaml` (all unapproved), both routes refuse with exit 2 before any runner call. A disabled channel is reported as `refused`, never as a skip.

### End-to-end (verify mode, real tools)

- On a temporary commit of this working tree, the following passed:
    - `verify-select --package biscuit-hash-cli`
    - `build-matrix` (runners `macos-15`, `macos-15-intel`, `ubuntu-22.04-arm`, `ubuntu-22.04`)
    - `licenses.sh` (cargo-about 0.8.4)
    - `stage`
    - `dist plan` (dist 0.34.0)
    - `validate-plan`
- A forged `publishable: true` on that verify record was refused (exit 2), and `publish` refused the verify record before any network call.

### Requirement-to-test mapping (Phase 3)

| Requirement | Tests |
|---|---|
| Credential scope: read-only by default, write only in upload, tap token only in the tap job, checkouts drop credentials | `ci_workflow_contracts::distribution_scopes_each_credential_to_the_one_job_that_needs_it` |
| Verification mode cannot reach upload or the tap, even with forged fields | `distribution_verification_runs_cannot_reach_publication`; `test_distribution.py` verify-select and forged-`publishable` cases; publish refuses a verify selection before any forge call |
| Never edit Releases, tags, or existing assets; no `latest`; release/dispatch triggers only | `distribution_never_edits_releases_tags_or_existing_assets`; `test_distribution.py` no-edit-endpoint tests (stubbed GitHub forge records only GET, uploads POST, and `ls-remote`) |
| Per-package and tap-wide serialization without cancellation | `distribution_serializes_publication_without_canceling` |
| Exact selected commit; no interpolation into scripts | `distribution_builds_only_the_selected_commit_and_interpolates_nothing` |
| Recovery: verified original bundle; fresh rebuild only for an empty Release | `distribution_recovery_reuses_only_a_verified_original_bundle`; `test_distribution.py` bundle round trip plus one edit each (commit, missing/extra asset, size, provenance, run docs, `Package` job state, workflow path, repository, run id) |
| Immutable upload: first/identical/partial/conflict, truncated download, revoked policy, Release/tag/proposal races, pending proof, `already_exists` race, unrelated assets kept, provenance last | `test_distribution.py` publish tests (fake `PublishForge` recording effects) |
| Tap: create/update/no-downgrade (0.9 vs 0.10)/identical/conflicting bytes/prerelease, credentials, author, download mismatch, head conflict without retry, signature, drift, claudine ALSA | `scripts/distribution/test_tap.py` (49) |
| Binstall: argv, disabled channel refused before network, version/HEAD/tag checks, post-run file and URL checks, optional helper | `scripts/distribution/test_verify_binstall.py` (23); `fixture-check` with real binstall 1.26.2 |
| Installer check after upload | Stub-installer sanity runs (pass, extra file, profile write); live run pending |

- Mutation check on the workflow contracts: dropping `publishable` from upload's condition, granting preflight `contents: write`, setting upload's `cancel-in-progress: true`, or neutering a HEAD proof each fails the matching contract.
- Input-robustness smells were grepped in the new Python (`.get(..., [])`, `or []`, `except Exception`, `.ok()`): none found. Phase 4 owns the full matrix.

### Gates

- Scope calculator over this phase's unstaged and untracked files (`git status --porcelain --untracked-files=all | grep -v '^A  \|^M  ' | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `change_class: package`, packages `repo-deps` and `test-toolkit`. The gates were run.
- `just test repo-deps test-toolkit`: **1027 passed**, 5 skipped, 0 failed (49 s).
    - 16 tests exceeded 5 s. All are pre-existing and none are in this phase's code: `ci-build archive::tests::*`, `archive_path_guard::*`, `junit_staging_contracts::*`, `kache_*`, and `the_shipped_planner_emits_plans_the_guards_reader_accepts`. They are recorded as load-sensitive.
- `cargo clippy --locked -p test-toolkit -p repo-deps --all-targets -- -D warnings`: clean. Neither package has a `lint` recipe.
- `just check-tier-coverage test-toolkit`: nothing stranded.
- Python suites (venv with the pinned requirements), all OK: `test_distribution.py` 131, `test_tap.py` 49, `test_verify_binstall.py` 23, `test_inspect.py`, `test_installers.py`, and `test_licenses.py`. pyflakes is clean on all of `scripts/distribution/*.py` and `distribution.py`, and shellcheck is clean on all of `scripts/distribution/*.sh`.
- Dependents: 25 packages depend on `test-toolkit`, but this phase changed only its integration test file `tests/ci_workflow_contracts.rs`, not its library. The `repo-deps` changes are Python, shell, YAML, and fixtures that no Rust code compiles in. The scope calculator, rerun after the plan and log edits, still selects only `repo-deps` and `test-toolkit`, so no dependent tests were run.

## Phase 4

- Started 2026-11-09. Work split: (A) a subagent completes the input-robustness matrix in `scripts/ci/distribution.py` and its suite; (B) a subagent does the same for the `scripts/distribution/` tools and removes their silent `skipUnless` tool skips; (C) a subagent writes the runtime install verification driver and produces evidence. I own the CI bridge (recipes, suite runner, companion-suite registration, scheduling of the configuration files, Rust contract reads). No local OS test rigs (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS`) are configured.

### Wave 6 — CI bridge (`just/distribution.just`, companion suite, contracts)

- New recipes in `just/distribution.just`, imported by the root `justfile`:
    - `just distribution-test [--counts-out FILE]` runs every distribution Python suite through `scripts/distribution/run_suites.py`, inside a temporary environment holding exactly `scripts/ci/requirements-distribution.txt` (`scripts/distribution/pinned-python.sh` creates and removes it).
    - `just distribution-check` runs `distribution-test`, then the `distribution_*` contracts in test-toolkit's `ci_workflow_contracts` through nextest.
    - `just distribution-verify OUT --artifacts DIR` runs `scripts/distribution/verify.sh` with the same pinned interpreter.
- `run_suites.py` refuses an interpreter whose PyYAML/semver differ from the pins (exit 2, before any test loads), refuses a `scripts/distribution/test_*.py` it does not list, and fails the run on any skipped test. It writes the shared five-field counts document (`suite_runner.counts_of`).
- CI: `distribution` is a new `repo-deps` companion suite in `SUITE_REGISTRY` (`ubuntu-latest`, recipe `just distribution-test`, JSON counts), declared in `scripts/Cargo.toml`. The registry's recipe-existence check now also searches the `just/*.just` files the root justfile imports (`(".", "distribution-test")`).
- Scheduling, checked with `affected_scope.py`:
    - `scripts/**` already selects `repo-deps` (its package directory), so every script or suite edit runs the companion.
    - `docs/deployment.yaml`, `dist-workspace.toml`, `about.toml`, `.github/distribution/build-images.json`, and `just/distribution.just` selected nothing before. They are now `SUITE_OWNER_PATHS` entries for `repo-deps`.
    - The scripts the workflow runs, plus `pinned-python.sh`, are `source-inputs` of test-toolkit. Each edit adds one narrowed L1 cell naming exactly the contracts that read it (for example `scripts/distribution/tap.py` → `distribution_workflow_calls_only_declared_script_entry_points`). `.github/actions/setup-dist/action.yml` and `requirements-distribution.txt` are found by the test-input scan without a declaration.
- Three new Rust contracts:
    - `distribution_workflow_calls_only_declared_script_entry_points`: every script the workflow runs exists, and every subcommand it names (including the ones passed through `args=(…)` and `mode=…`) is one the script dispatches. A mutation (`check-sums` → `check-sumz` in the workflow) fails it.
    - `distribution_tools_are_pinned_by_version_and_digest`: exact `==` Python pins; dist pinned per build host by SHA-256; the Ubuntu image pinned by digest; cargo-about and cargo-binstall pinned by version and SHA-256 and checked with `shasum -c` before unpacking; no `latest` or unpinned installs.
    - `distribution_suites_run_through_the_registered_companion_recipe`: the import, both recipes, the helper's venv provisioning, the registry entry, and the `scripts/Cargo.toml` declaration.
- `scripts/ci/test_affected_scope.py`: `distribution` added to `EXPECTED_COMPANION_OWNERS`, and `test_a_distribution_configuration_change_selects_repo_deps_alone`. `.github/ci/README.md`'s selection table gained the new row.
- `just distribution-test`: **413 tests, 413 passed, 0 skipped**, 19 s.

### Wave 6 — Robustness matrix, `distribution.py` (subagent A)

- `test_distribution.py`: 131 → 167 tests, 8 s (was 21 s). The time went into parsing the 1,400-line `docs/deployment.yaml` 411 times; a test-only cache keyed on text, source, and known package names now reuses successful parses (invalid cases always re-parse). Production code is unchanged by the speedup. 32 of the new tests fail against the pre-change module.
- New classes: `FieldValidatorTests` (one shape matrix per shared validator: required string, string list, record list, mapping, boolean, integer, commit id, SemVer, asset name), `WiredFieldTests` (one control and one invalid shape per wired field), `ExceptionContractTests` (the plan's listed exceptions), `LoaderGapTests` (JSON `-Infinity`, non-UTF-8, empty, nested duplicates, Unicode; TOML dotted-key redefinition, table/array clash, comment-only; YAML non-UTF-8), `LoaderWiringTests` (per loader through `main`: positive, then a malformed case where a fake forge, `urlopen`, and `subprocess.run` record no call), `MissingInputTests`.
- Silent loss fixed, each with a regression test:
    - a wrong-shaped parent (`release: null`) used to read as "child absent", and a defaulted boolean under a null parent took its default. `Fields.lookup` now reports the parent.
    - a missing input file crashed with exit 1, which is reserved for an asset conflict. It is now a located invalid error (exit 2).
    - a missing root `Cargo.toml`, or a missing or non-table `[workspace]`, silently meant "no excluded packages".
    - subdirectories and symlinks in asset directories were skipped by provenance and reconcile. They are now refused.
    - an empty build `image: {}` was accepted.
    - asset paths had no validator. `Fields.asset_name` now applies to Release asset names and dist plan asset `name`/`path`.

### Wave 6 — Robustness and no silent skips, `scripts/distribution/` (subagent B)

- The `ruby`, `gpg`, and `shellcheck` tests used `unittest.skipUnless(shutil.which(...))`. They now use `tool_guard.require_tools(..., enforced_by=…distribution companion…)`, and `run_suites.py` fails any skip. No skip path is left in these suites.
- Suites: `test_tap.py` 57, `test_verify_binstall.py` 26, `test_installers.py` 44, `test_inspect.py` 50, `test_licenses.py` 35, all passing.
- Silent loss fixed, each with a regression test:
    - `tap.py`: GitHub responses now go through `load_json`/`Fields`. Before, `bool(signature.get("isValid"))` read the string `"false"` as true, and `commit.get("signature") or {}` hid a missing signature. A missing or non-boolean `selection.prerelease` used to read as a stable release. An empty `brew --repository` answer used to put the audit tap in the current directory.
    - `verify_binstall.py`: a missing root manifest silently dropped `[workspace] exclude`.
    - `installers.py`: a missing `build-images.json` fell back to macOS 15.0. Non-UTF-8 inputs gave tracebacks rather than located errors.
    - `inspect_archive.py`: it has its own strict JSON loader (duplicates, non-finite numbers), because it runs inside the build container without PyYAML. Empty forbidden fragments were silently dropped. `SOURCE.txt` was decoded with `replace`. A binary linking `libc.so.6` with no `GLIBC_` version needs passed unchecked.
    - `native_notices.py`: `targets = "aarch64-apple-darwin"` (a string) passed the list check. Metadata is now validated field by field.
- New coverage-list tests: numeric (not lexical) tap ordering in both directions, unsafe archive members launching no tool and writing nothing outside the root, and glibc exactly at 2.35 accepted.
- `licenses.sh`'s comment now says its `PYTHON` must carry the pinned requirements, because `native_notices.py` imports `distribution.py`. In the workflow, `setup-dist` already exports that `PYTHON`.

### Wave 6 — Runtime verification (subagent C)

- New `scripts/distribution/verify_runtime.py` (`run`, `combine`), `test_verify_runtime.py` (34 tests, 1 s), and `verify.sh` (the `distribution-verify` entry point). `verify.sh` downloads cargo-binstall 1.26.2 checked against the pin when the local one differs (this host has 1.25.3), runs `verify_binstall.py fixture-check`, then `verify_runtime.py`, and writes a combined report.
- Real run over freshly built `bh` archives (macOS natively; Linux in the pinned container with the real cargo-about notices; then dist's global step, `harden`, and `check-sums`): exit 0, **6 passed, 0 failed, 10 pending, 2 disabled**, overall `incomplete`. Retained text evidence: `features/2026-10-28-publish/evidence/phase-4/` (`README.md`, `build/`, `verify/`). `.gitignore` excludes `*.log`, so the build logs there are not tracked; the transcripts and JSON reports hold the results.

| Target | Route | Evidence class | Result |
|---|---|---|---|
| aarch64-apple-darwin | archive, installer | native-build-host | pass |
| aarch64-apple-darwin | clean runtime | pending-infrastructure | no clean macOS 15 VM |
| x86_64-apple-darwin | archive, installer | pending-infrastructure | no x86_64 macOS host |
| aarch64-unknown-linux-gnu | glibc refusal (Ubuntu 20.04, glibc 2.31) | native-clean-runtime | pass: refused, and the install root, HOME, and TMPDIR stayed empty |
| aarch64-unknown-linux-gnu | archive, installer (Ubuntu 22.04 runtime image, no network) | native-clean-runtime | pass: only libc, ld-linux, and libgcc_s loaded |
| x86_64-unknown-linux-gnu | refusal, archive, installer | pending-infrastructure | Docker here is aarch64; emulation is not evidence |
| Linux artifact under WSL2 | archive, installer | pending-infrastructure | no WSL2 host; never counted as native Windows |
| all | Homebrew, binstall channels | disabled | not enabled in `docs/deployment.yaml` |
| all | binstall fixture-check | native-build-host | pass |

- Archive sizes (informational): macOS 344,728 bytes, Linux 350,416 bytes.

### Requirement-to-test mapping (Phase 4)

| Requirement | Tests |
|---|---|
| A shape matrix once per shared validator, then a control and one invalid shape per wired field | `test_distribution.py` `FieldValidatorTests`, `WiredFieldTests`, existing `PolicyTests.*_shapes`; `test_tap.py` `GitHubClientTest.test_nullable_object_validator` plus per-field tests; `test_inspect.py` `ConfigLoaderTests`; `test_licenses.py` `test_nullable_string_validator`, `test_each_metadata_field_rejects_an_invalid_shape` |
| Listed exceptions (empty `packages`, empty asset listing, 404 only for asset/formula lookups, missing token) | `ExceptionContractTests` |
| YAML/JSON/TOML loader matrices | `YamlLoaderTests`, `JsonAndTomlLoaderTests`, `LoaderGapTests`; `inspect_archive`'s own loader in `ConfigLoaderTests` |
| CLI wiring per loader: a valid case, then a located malformed case that launches nothing | `LoaderWiringTests`; `CommandLineTest` (binstall); `CommandLineTests` (inspect); tap and licenses CLI cases |
| Coverage list (retries, revocation, prerelease, missing bundle or credentials, tap ordering, unsafe paths, glibc, template drift) | existing Phase 1–3 tests plus `MissingInputTests`, `test_numeric_not_lexical_ordering_never_downgrades`, `test_unsafe_members_are_never_written_and_launch_no_tool`, `test_glibc_exactly_at_floor_is_accepted` |
| No silent skips; pinned environment | `run_suites.py` (refuses on skip and on a version mismatch, checked by hand: a host `python3` exits 2); `distribution_suites_run_through_the_registered_companion_recipe` |
| Configuration edits schedule the owning checks | `test_a_distribution_configuration_change_selects_repo_deps_alone`; `SuiteOwnershipRegistryTests`; narrowed cells confirmed with `affected_scope.py --resolved-plan` |
| Workflow ↔ script entry points; pinned tools | `distribution_workflow_calls_only_declared_script_entry_points` (mutation-checked), `distribution_tools_are_pinned_by_version_and_digest` |
| Runtime reports keep evidence classes apart; pending and disabled never count as a pass; leaked libraries refused; refusal before any change | `test_verify_runtime.py` (34), plus the real `verify.sh` run above |

### Gates

- Scope calculator over this phase's unstaged and untracked files (`git status --porcelain --untracked-files=all | grep -v '^[AM]  ' | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `change_class: package`, packages `repo-deps` and `test-toolkit`. The gates were run.
- `just test repo-deps test-toolkit`: **1030 passed**, 5 skipped, 0 failed (36 s). The 6 slow tests (over 5 s) are pre-existing ones listed in Phase 3 and none are in this phase's code. They are recorded as load-sensitive (three Docker builds were running on the host).
- `cargo clippy --locked -p test-toolkit -p repo-deps --all-targets -- -D warnings`: clean. `just check-tier-coverage test-toolkit` and `repo-deps`: nothing stranded.
- `just distribution-test`: 413 passed, 0 skipped. `python3 scripts/ci/suite_runner.py test_affected_scope.py`: 492 OK (712 s). `test_resolved_plan.py`: 103 OK.
- pyflakes is clean on every distribution script and on `affected_scope.py`. `test_affected_scope.py` has 5 pre-existing pyflakes findings, present before this phase. shellcheck is clean on `scripts/distribution/*.sh`.
- Dependents: neither package's library changed. test-toolkit changed only its integration test file and manifest metadata, and repo-deps changed only Python and shell. So no dependent tests were run.
- Live acceptance is still pending: no approval, App proof, tap credentials, or clean-runtime hosts exist.

## Phase 5

- Started 2026-11-09. Work split: a documentation subagent wrote `docs/distribution.md`, the `docs/releases.md` hand-off, the seven area README links, and the rust-devops skill contract. A second subagent planned all eight packages with the pinned dist under fixture approvals. I own the rollout ledger and gate, the workflow job, the drift fixes, validation, and this log. No local OS test rigs (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS`) are configured.

### Wave 7 — Current documentation

- New `docs/distribution.md` (12 sections, two Mermaid diagrams):
    - the package-to-command table; targets and minimum versions; the glibc 2.34 exclusions; WSL2; claudine's ALSA commands, including `libasound2t64` on Ubuntu 24.04 and Debian 13
    - every install route marked **planned**, with URL patterns only
    - binstall's default-branch `--git` route and its exact-tag `--manifest-path` route, documented separately
    - archive contents, the run lifecycle and the Release's asset states, approval, the rollout ledger, re-runs and recovery, the tap, credentials, the monthly build-image refresh, and maintainer checks
- `docs/releases.md` hands off to it. Its intro said binary distribution was "planned, not built". That was drift, now corrected.
- Area READMEs (`biscuit-hash`, `biscuit-file`, `tree-hugger`, `worktree`, `sniff`, `darkmatter`, `claudine`) gained one link each. None of them is read by compiled code.
- `.claude/skills/rust-devops/SKILL.md` has a new "Distribution contract" section, and `deployment-platforms.md` was refreshed.
- Drift found and fixed:
    - the workflow header, the `distribution.py` and `installers.py` docstrings, and `verify.sh` named `docs/releases.md` as the operational guide; they now name `docs/distribution.md`
    - `docs/topics/ci-cd.md` said distribution "is not implemented"
    - `.github/ci/README.md`'s path table lacked the ledger

### Wave 8 — Rollout ledger and gate (`scripts/ci/distribution.py`)

- New `.github/distribution/rollout.json`:
    - `first_package: biscuit-hash-cli`
    - `verified: []`
    - 23 named `pending` items, covering bh's approval, release-app, homebrew-tap, clean-runtime-macos, x86_64-runtime, wsl2-runtime, and live-release; `first-chain` and `approval` for the other seven; `brew-description` for darkmatter-cli and claudine-cli
- `parse_rollout` reads the ledger through the shared strict JSON loader and `Fields`. It rejects unknown fields and duplicate tags or pending names, and it requires the tag to be `{package}-v{version}`, canonical run and release URLs, all four targets `pass`, and an allowed result for each channel.
- `preflight` now calls `gate_rollout` on any selected release:
    - It reads the default branch's ledger (`--rollout` overrides it, for tests only; a contract forbids it in the workflow).
    - biscuit-hash-cli needs no entry.
    - Any other package needs a verified biscuit-hash-cli entry whose run GitHub reports as a completed, successful run of `distribute-binaries.yml` in this repository (`Forge.workflow_run`, `check_rollout_run`).
    - A missing run is an error, not an absence.
- `check_policy_rollout` runs against the real `docs/deployment.yaml` and the real ledger in the companion suite, so an approval that widens too early fails on its pull request.
- `rollout-candidate` (new command and new final workflow job, read-only) builds the proposed entry from the selection and each job's `needs.*.result`, and refuses one that the ledger would not accept. The job uploads `distribution-rollout-candidate` and prints the entry in the run summary.
- `.github/distribution/rollout.json` is now a `repo-deps` `SUITE_OWNER_PATHS` entry, checked by `test_a_distribution_configuration_change_selects_repo_deps_alone`.

### Wave 8 — Dist plans for all eight packages

- Evidence: `features/2026-10-28-publish/evidence/phase-5/plans/` (eight plans, eight validation results, and a README with the commands).
- dist 0.34.0 (aarch64-apple-darwin, SHA-256 `d1eec30e…1890`) was checked with `shasum -c` against `build-images.json` before extraction. The source was a throwaway clone with this branch's working tree committed (`e203e590`), outside the repository.
- Each package was selected with `verify-select --brew <command>` (a fixture approval), then staged, planned, and checked with `validate-plan`.
- Every plan ships exactly one executable on all four targets.
- Excluded helper binaries:
    - `render_dirty_tree` (wt)
    - `render_cicd_fixture` and `render_git_status_fixture` (sniff)
    - `darkmatter-argv-recorder` (md)
    - claudine's six fixture binaries
- **Defect found and fixed:** dist adds `sniff/cli/CHANGELOG.md` to sniff's archives, and `validate-plan` refused it. `validate-plan` and `inspect_archive.py` now accept exactly one non-executable `CHANGELOG.md`. The real sniff plan is now a fixture (`dist-plan-sniff-detection-cli-brew.json`), and all eight plans validate.

### Wave 8 — Validation commands and results

| Command | Result |
|---|---|
| `just distribution-check` | 432 Python tests OK (0 skipped); 9/9 `distribution_*` contracts |
| `just test repo-deps test-toolkit` | 1030 passed, 5 skipped; the 6 slow tests are the pre-existing ones listed in Phase 4 |
| `cargo clippy --locked -p test-toolkit -p repo-deps --all-targets -- -D warnings` | clean |
| `just check-tier-coverage test-toolkit` / `repo-deps` | nothing stranded |
| actionlint 1.7.12 (darwin_arm64; SHA-256 `aba9ced2…953f` matched both the release's checksums file and GitHub's recorded asset digest) on `distribute-binaries.yml` and `release-plz.yml` | clean, with shellcheck and the local `setup-dist` action resolved |
| pyflakes on the distribution scripts and suites; `shellcheck scripts/distribution/*.sh` | clean |
| narrowed darkmatter cells from the scope plan (`trigger_spelling_guard::no_active_file_spells_a_renamed_trigger_form`, two `shipped_schema_corpus` tests) | 3 passed |
| `python3 -m unittest test_affected_scope.ToolingPathOwnershipTests` (`SuiteOwnershipRegistryTests` too) | OK |
| worktree: git-wt-cli L1 (`cargo nextest run -p git-wt-cli` minus tier markers) | 614 passed |
| worktree: `just test-l2 level2_dirty_tree` (tmux backend, no window focus) | 1 passed; the feature-gated helper bin is not stranded |
| `just distribution-verify features/2026-10-28-publish/evidence/phase-5/verify --artifacts /tmp/phase4-assets` (Phase 4's bh build) | exit 0: 6 passed, 0 failed, 10 pending-infrastructure, 2 disabled (same as Phase 4) |

- Mutation checks:
    - Removing the `gate_rollout` call fails 3 `RolloutPreflightTests`.
    - Removing the changelog allowance fails the sniff plan test.
    - Dropping the publishable guard from `rollout-candidate` fails `distribution_verification_runs_cannot_reach_publication`.
- Final diff review over all phases:
    - no `approved: true` in `docs/deployment.yaml`
    - no release-plz or integration workflow changes
    - no secrets, and no signing bypass in the distribution code
    - `releases/latest` appears only in forbidden-pattern lists and negative tests
    - one unbounded subprocess: `native_notices.run_cargo` now has a 30-minute timeout with a located error, plus a test
- The Rust cargo cache warning ("failed to remove … hyper-tls … Permission denied") comes from cargo's global-cache auto-clean on this host and is unrelated.

### Requirement-to-test mapping (Phase 5)

| Requirement | Tests |
|---|---|
| Ledger fields: control plus one invalid shape each (absent, null, wrong type, wrong element, empty, duplicate, unknown); loader duplicate key and trailing text | `RolloutLedgerTests.test_every_invalid_field_shape_is_a_located_error` (32 cells), `test_empty_lists_are_valid_and_permit_nothing_but_bh` |
| Real ledger and real policy agree; only bh is permitted today | `test_the_real_ledger_records_pending_setup_and_permits_only_bh` |
| Widening needs a verified first-package entry, and the newest entry is used; a non-bh entry does not count | `test_only_a_verified_first_package_entry_permits_widening`, `test_the_newest_verified_first_package_entry_is_used`, `test_an_approval_without_the_first_chain_is_located_at_the_approval` |
| The entry's run must be the canonical workflow, completed and successful | `test_the_run_must_be_the_canonical_workflow_and_successful`; through `main`: `RolloutPreflightTests.test_the_bh_run_must_exist_and_have_succeeded` |
| Public preflight result: second package selected or refused before anything runs; malformed ledger located even for bh; ignored releases need no ledger | `RolloutPreflightTests` (6) |
| Candidate follows channels and job results, and round-trips into the ledger (write, read, write) | `RolloutCandidateTests` (2), `test_a_candidate_round_trips_through_the_ledger_and_permits_widening`, `test_the_candidate_command_writes_an_entry_the_ledger_accepts` |
| Workflow: candidate job only after a publishable production run; no ledger substitution | `distribution_verification_runs_cannot_reach_publication` (extended) |
| Ledger edits schedule the suite | `test_a_distribution_configuration_change_selects_repo_deps_alone` |
| Package changelog accepted, nothing else | `ValidatePlanTests.test_a_package_changelog_is_the_only_extra_document_accepted` (real sniff plan), `ArchiveStructureTests.test_a_package_changelog_is_accepted_but_no_other_document` |
| Bounded cargo subprocess | `NativeNoticeTests.test_a_cargo_command_that_hangs_is_bounded_and_reported` |

### Rollout evidence and external setup (read-only checks, nothing published)

- The `biscuit-hash-cli-v0.1.0` tag exists (peeled `907cd6e4…`).
- `distribute-binaries.yml` is not on the default branch, so it has no runs, and no live distribution, ledger candidate, or tap evidence exists.
- `gh` on this host has no GraphQL authentication, so Releases, repository variables, secrets, and the tap repository could not be read. None of them is known to exist.
- The ledger therefore records bh's chain as pending, and every other package is blocked behind it.

**Acceptance status.** The implementation is complete. **Live acceptance is pending**: no approval, no `RELEASE_APP_BOT_LOGIN`, no tap repository or `HOMEBREW_TAP_TOKEN`/`HOMEBREW_TAP_AUTHOR`, no clean macOS 15 VM, no x86_64 hosts, no WSL2 host, and no authorized live release. The specification is not fully accepted until the first bh chain is recorded in the ledger.
