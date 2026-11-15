---
kind: feature
name: publish
date: 2026-10-28
status: finalized-spec
depends-on:
  - 2026-11-06-release-path-repair
next_spec: 2026-11-06-ubuntu-archive-disk
related:
  - 2026-10-03-binary-releases
  - 2026-11-06-windows-wsl-test-failures
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-08
review_iterations: 2
completed: true
implemented: true
implemented_by: "claude/opus"
needs_rulings: false
rulings:
  - id: linux-glibc-floor
    decided_by: Ken Snyder
    decided_on: 2026-11-08
    decision: >-
      Linux downloadable binaries support glibc 2.35 and newer, built in a
      pinned Ubuntu 22.04 container image; one artifact per Linux target.
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
  clarified: boolean -> indicates whether the specification was built -- in part -- with the 'clarify.md' prompt
  implemented: boolean -> indicates whether this spec's plan has been implemented
  implemented_by: string -> the agent who implemented the plan
---

# Publish installable binaries

## Why

General CLI binary distribution has not been built: there is no `dist`
configuration or general binary release workflow. The earlier design,
`2026-10-03-binary-releases`, selected the channels; its completed status does
not prove those channels work. Existing integration archives are separate.

The prerequisite, `2026-11-06-release-path-repair`, must first demonstrate
package tags, GitHub Releases, and delivery of release events using the release
App. This specification consumes that path rather than replacing it.

## Outcome

For an approved CLI package release, build archives from its exact tag, attach
them to that package's GitHub Release, and provide these installation routes:

| Channel | Example for `bh` |
|---|---|
| GitHub Release archive | Download the archive matching the user's OS and CPU, verify its checksum, and unpack it |
| Shell installer | Run the version-specific shell installer linked from that release |
| PowerShell installer | Run the version-specific PowerShell installer when Windows artifacts are enabled |
| cargo-binstall | `cargo binstall --git https://github.com/yankeeinlondon/rusty-biscuit biscuit-hash-cli` |
| Homebrew (own tap) | `brew install yankeeinlondon/tap/bh` |

Replace installer placeholders in user documentation with tested URLs before
advertising a channel. Do not use the repository-wide `releases/latest` URL:
packages have independent versions and a different package may be latest.

## Scope and approval

Prove the complete chain on `bh` first, then enable the remaining first-set
commands after their own artifact and install checks:

| Command | Cargo package |
|---|---|
| `bh` | `biscuit-hash-cli` |
| `bf` | `biscuit-file-cli` |
| `hug` | `tree-hugger-cli` |
| `wt` | `git-wt-cli` |
| `sniff` | `sniff-detection-cli` |
| `md` | `darkmatter-cli` |
| `dmls` | `darkmatter-lsp` |
| `claudine` | `claudine-cli` |

Use `docs/deployment.yaml` as the approval authority. Its entries currently
have approval disabled; this list is a rollout plan, not permission to turn
all entries on. Require `approved: true` for binary distribution and additionally
the relevant channel setting for Homebrew or binstall. A channel's `false`
means disabled; `true` uses the crate name and a string supplies an alternate
name. For example, the Homebrew setting for `biscuit-hash-cli` is `brew: bh`.
`install` refers to `cargo install`, not the generated shell installer; leave
it disabled while registry publication is out of scope.

Generate dist opt-ins from this file, or validate checked-in opt-ins against
it; choose one implementation and fail on drift before building or uploading.
Reject unknown package names, duplicate channel names, and channel opt-ins on
unapproved packages. Keep all other packages opted out by default. Approval
of binary distribution does not alter release-plz's workspace tag policy.

Correct repository metadata on all eight CLI packages to the canonical
`https://github.com/yankeeinlondon/rusty-biscuit` URL. Resolve package paths
from manifests, not a second directory mapping. Build from the repository
checkout, preserving path dependencies and embedded files outside package
directories; crates.io packaging is not required here.

## Release ownership and execution

**Reader's note:** this intentionally supersedes the earlier proposal to give
dist GitHub Release ownership. The prerequisite preserves release-plz's
ownership of versions, changelogs, tags, and GitHub Releases. Keep
`git_release_enable = true`, `git_only = true`, and `publish = false`.
Dist builds and packages binaries; the upload step adds assets to the existing
release without replacing its notes or publishing a second release.

Start from the App-created published-release event and select exactly the
approved Cargo package whose complete `{package}-v{version}` tag matches.
Ignore library and integration releases. Check that the tag version matches
that package's manifest, resolve and record its commit, and build that commit
with the pinned repository toolchain and lockfile. Do not build current `main`
or all approved packages when one tag arrives. Retain the prerequisite's
reviewed-revision guarantees; never move a published tag.

Use a pinned dist version and checked-in configuration with `precise-builds`.
A package-tag plan must demonstrate that only the intended command is shipped;
`precise-builds` alone is not a package or binary allowlist. Exclude auxiliary
executables and test features, including `darkmatter-argv-recorder` in
`darkmatter-cli`. Use a dedicated distribution profile inheriting `release`,
with stripping and thin LTO; preserve ordinary developer profiles.

Confirm the pinned dist's build/package-only path before wiring publication.
Its documented [`create-release = false` behavior](https://axodotdev.github.io/cargo-dist/book/reference/config.html)
expects a preexisting **draft** release and later publishes it. That is not a
promise that its generated publisher can safely consume the already published
release from this repository. Use explicit asset upload if necessary. Do not
silently change the prerequisite to draft releases to fit a generated workflow.

```mermaid
flowchart LR
    Proposal[Reviewed release proposal passes CI] --> Release[release-plz creates package tag and release]
    Release --> Approval[Check package and channel approval]
    Approval --> Build[Build exact tagged source]
    Build --> Assets[Validate and upload archives and installers]
    Assets --> Channels[Publish enabled Homebrew formula]
```

Because release-plz publishes first, a release page can briefly have no binary
assets. Document that state and link the distribution run. Publish a formula
only after all artifacts it references are available. The integration release
workflow keeps its existing package selection and assets.

Build jobs need read access only. Restrict repository write access to upload
and restrict tap credentials to the formula publisher. Do not expose release
credentials to pull-request builds. Serialize work per package without
canceling a publication in progress. A manual recovery run must name an
existing package tag, repeat approval and revision checks, and cannot create
tags or calculate versions.

On retry, reuse identical existing assets and upload missing ones; refuse a
same-name asset with different bytes rather than overwriting it. Record
package, version, tag commit, target, toolchain, dist version, and checksums.
Publish checksums using the distribution tool's supported format; repository
code that computes hashes must use `biscuit-hash`. Keep archive names and URLs
version-specific. Homebrew updates must not downgrade a newer version after an
older run is retried. A bad release is corrected by a new version, never by
moving its tag or replacing its archives.

## Targets and portable binaries

The initial target set is macOS arm64 and x86_64, and Linux GNU arm64 and
x86_64. Add `x86_64-pc-windows-msvc` only after the Windows failures described
in `2026-11-06-windows-wsl-test-failures` are verified fixed. Windows is a
target-specific condition, not a blocker for macOS and Linux publication.
WSL2 consumes the matching Linux artifact; it is not a separate binary target.
Musl and Windows ARM remain outside this scope.

**Reader's note:** the old statement that Windows and WSL2 await event-driven
CI is stale. Current policy schedules them nightly and offers `ci:all-os` and
manual all-OS runs. Keep that policy; distribution must not introduce a second
full-workspace test scheduler.

The completed [size and native-dependency measurement](spikes/S3-release-sizes-and-native-deps.md)
from November 7 answers the requested measurement. Do not repeat it because
of age. It found Homebrew PCRE2 paths in six macOS commands, Linux glibc
requirements as high as 2.39, and Linux ALSA linkage for `claudine`.

- Set `PCRE2_SYS_STATIC=1` for release builds that use PCRE2 and inspect final
  artifacts for unwanted Homebrew or build-host paths. Validate the supported
  setting against the pinned dependency. Build-host success is insufficient.
- Keep native audio in `claudine`; document its Linux `libasound.so.2` runtime
  requirement and distro-specific installation instructions. Build prerequisites
  include ALSA headers for Linux `claudine`, a C compiler and pkg-config where
  needed, and libclang for affected macOS builds. The measurement found no
  cmake or OpenSSL requirement; do not add them from the older spec.
- Pin release build images rather than using moving `*-latest` labels. Record
  minimum supported glibc and macOS versions and CPU architectures in the
  install guide. The Linux floor is glibc 2.35 (see the ruling below).
- Inspect each target's dynamic dependencies and test on a clean runtime
  without developer tools or Homebrew libraries. Bundle required license
  notices with statically linked dependencies and include the project's
  `AGPL-3.0-only` license and a link to the exact tagged source.
- Record observed archive sizes as release information. The measured 100+ MB
  binaries do not block these channels; PyPI limits belong to later work.

## Compatibility checks and acceptance

Before a live release, run the pinned dist's plan for `bh` and one package with
auxiliary binaries. Check exact package selection, tag parsing, target list,
features, archive contents, installer URLs, and approval enforcement. Reject
unrelated tags, version mismatches, disabled channels, and wrong revisions.
Exercise retry cases with fixture assets. Use existing focused test-toolkit
coverage and workflow lint for changed automation, following the repository's
Just/Nextest conventions; add no speculative full-scope CI run.

Confirm binstall's git route against this monorepo and package-tag layout.
Start with automatic discovery, then add explicit metadata only if required,
as recommended by [binstall's metadata guide](https://github.com/cargo-bins/cargo-binstall/blob/main/SUPPORT.md).
Prove that the package and version resolved from Git match the downloaded
release. Disable source-compilation fallback during verification, so a local
compiler cannot hide missing binary assets. Document tested version-selection
syntax rather than assuming registry version selection also works for Git.

For each shipped target, install into a temporary isolated directory from the
actual archive and applicable installer, and run a harmless version/help check.
Verify binstall for that target and Homebrew on supported macOS/Linux targets
where the formula is enabled. PowerShell applies to native Windows; shell
installation applies to macOS/Linux and WSL2. Keep terminal windows unfocused
and do not invoke playback or other interactive features. State a platform as
pending until its check succeeds; do not claim Windows installation from a
cross-compile alone. Reuse qualifying existing test evidence rather than
rerunning passed package suites to validate archive layout.

Record the first real `bh` release URLs, source revision, distribution run,
checksums, install results, and tap update in the implementation log. Widen
only after that chain succeeds. For remaining packages, record the same
artifact checks and applicable install results before calling their rollout
complete. This review authorizes specification edits only; live publication
requires the normal author-reviewed release proposal.

Update `docs/releases.md` and a distribution topic page with concrete commands,
platform requirements, package-versus-command names, partial-release states,
and retry instructions. Keep READMEs as overviews linking to those guides;
update the rust-devops skill's distribution contract with the final workflow.

## Needs from Ken

Approve package/channel entries in `docs/deployment.yaml`, create the
`yankeeinlondon/homebrew-tap` repository, and supply a narrowly scoped
`HOMEBREW_TAP_TOKEN` with Contents write access to that tap. The prerequisite's
release App setup must already be verified. Missing credentials fail the
relevant enabled publisher with an actionable message; they never silently
skip it or fall back to broader credentials. Report setup or live-release
verification still pending rather than claiming completion.

## Open Questions

### Which Linux versions should downloaded binaries support?

**Ruled 2026-11-08: glibc 2.35 and newer, one artifact per Linux target.**

Build Linux release binaries in a pinned Ubuntu 22.04 (glibc 2.35) container
image. This supports Ubuntu 22.04 and 24.04, Debian 12 and 13, current Fedora,
and WSL2's default Ubuntu, while retaining `claudine`'s native audio. RHEL 9
and Amazon Linux 2023 (glibc 2.34) are not supported; document that.

The floor is a compatibility choice, not a security exposure: glibc is not
bundled, so binaries run against the host's own patched glibc, and Rust's
default hardening does not depend on the build glibc. The security obligations
belong to the build image and bundled dependencies:

- Pin the build image by digest, not by tag or hosted runner label, so the
  floor survives GitHub retiring its `ubuntu-22.04` runner image.
- Refresh the pinned digest on a documented schedule so the image's compiler,
  headers, and native development packages receive security updates.
- Bundled Rust crates and statically linked C libraries (for example PCRE2)
  follow the repository's normal dependency-update practice.

Installers check the host glibc before installing and report the required and
found versions on an unsupported host. For the floor, inspect the produced
symbol requirements (no `GLIBC_` symbol newer than 2.35) and perform a runtime
check on a glibc 2.35 system; do not infer compatibility from the image name.
This is an installation correctness check, not a new performance study.

Rejected alternatives: a glibc 2.39 floor (simpler build, but excludes Ubuntu
22.04 and Debian 12), separate older- and newer-glibc variants (doubles Linux
artifacts with no dependency that needs 2.39), and a glibc 2.34 floor (adds
RHEL 9 and Amazon Linux 2023 at the cost of a less familiar build image).

## Not in scope

npm, PyPI, and crates.io: `2026-11-06-publish-more-channels`. No package
renaming, registry publication, new CI cadence, automatic updater, additional
binary targets, or distribution of the integration packages is introduced here.
