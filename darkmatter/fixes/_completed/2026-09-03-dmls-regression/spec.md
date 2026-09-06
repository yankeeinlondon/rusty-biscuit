---
status: proposed
created: 2026-09-03
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-03
review_iterations: 6
area: darkmatter
packages:
  - dmls
  - zed-dmls-cli
---

# DMLS stopped loading in Zed; make the Zed launch path testable

## Summary

Since 2026-08-30 the Darkmatter Language Server (`dmls`) has not been available
in Zed. Zed reports:

```
Failed to install dev extension: No extension manifest found for extension vscode-dmls
```

The investigation found **no code regression**. The `dmls` binary and the
`zed-dmls` extension source are healthy at `HEAD`:

- `dmls 0.1.0` (installed 2026-09-02) answers `initialize` over stdio with its
  full capability set.
- `zed-dmls` compiles for both `wasm32-wasip2` (what Zed builds) and
  `wasm32-wasip1` (what `just check-zed` checks).
- `extension.toml`, `Cargo.toml`, and `src/lib.rs` are unchanged since
  2026-07-31 (`521c87c91`), well before the regression window.

What broke is the **installation**, which lives outside the repository and is
therefore invisible to every test we have:

1. On 2026-07-11 the extension was installed into Zed as a *dev extension*.
   Zed records a dev extension as a symlink from its extensions directory to
   the source folder that was selected. The folder selected was inside a git
   worktree:
   `~/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/dmls/zed-dmls`.
2. On 2026-08-30 at 22:17 that `darkmatter` worktree was removed (the
   `feat-unifi` worktree was created in the same minute). The symlink now
   dangles.
3. At 22:18 the same evening Zed restarted and logged
   `No extension manifest found for extension dmls`. It has logged the same
   line on every start since. That is the moment DMLS disappeared from Zed.
4. On 2026-09-02 a re-install was attempted through
   `zed: install dev extension`. The folder chosen was
   `darkmatter/dmls/vscode-dmls/` (the VS Code extension, a sibling directory)
   rather than `darkmatter/dmls/zed-dmls/`. Zed derives the id in that error
   from the selected folder's name, which is why the message names
   `vscode-dmls`. `dmls` was also reinstalled that afternoon (14:16), which
   did not help because the binary was never the problem.

The dev-extension model ties Zed to a filesystem path we churn constantly
(worktrees are created and deleted per branch), and the two editor extensions
sit side by side under `dmls/` with names that are easy to mix up in a file
picker. Nothing in the repository detects either condition, and the one
packaging check we do have (`just check-zed`) checks the wrong target and skips
silently when the target is missing, which is its state on this host and in CI.

## Evidence

| Observation | Source |
|---|---|
| Dev extension symlink `dmls -> ~/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/dmls/zed-dmls` (created 2026-07-11) | `~/Library/Application Support/Zed/extensions/installed/` |
| Target path does not exist; worktree list has no `darkmatter` entry | `git worktree list`, `ls` |
| `feat-unifi` worktree born 2026-08-30 22:17; `.git/worktrees/` and the worktree parent directory last modified 22:17 | `stat -f %SB` |
| `No extension manifest found for extension dmls` at 2026-08-30 22:18 and on every later start | `~/Library/Logs/Zed/Zed.log` |
| `Failed to install dev extension: No extension manifest found for extension vscode-dmls` at 2026-09-02 14:13 and 14:16 | same log |
| Zed's manifest loader names the extension after the selected directory (`extension_dir.file_name()`) and looks for `extension.toml` then `extension.json` | zed `crates/extension/src/extension_manifest.rs` |
| Zed's extension builder compiles Rust extensions with `RUST_TARGET = "wasm32-wasip2"` and installs that rustup target itself if missing | zed `crates/extension/src/extension_builder.rs` |
| Zed stable (1.18.0 installed) loads extension API 0.7.0; other installed extensions on this host declare `lib.version = "0.7.0"` and run | Zed `extensions/index.json` |
| `just check-zed` targets `wasm32-wasip1`; `zed-dmls/README.md` says `wasm32-wasip2`; neither target was installed on this host, so the recipe printed `skip` | `darkmatter/justfile:275`, `zed-dmls/README.md` |
| `just check` (which folds in `check-zed`) is not run by `just ci-local`, by `_package-ci.yml`, or by `rust-latest-stable.yml` | `.github/workflows/*.yml`, `just/ci-local.just` |

## What existing tests cover, and the gap

| Test | Proves | Cannot see |
|---|---|---|
| `dmls/tests/lsp_session.rs` | request handlers over an in-memory connection | the binary, argv, stdio framing |
| `dmls/tests/stdio_subprocess.rs` | the compiled `dmls` binary completes `initialize → initialized → shutdown → exit` over real pipes | anything about the editor side |
| `dmls/tests/level2_editor_neovim.rs` | a real Neovim client drives the real binary, decodes tokens, repaints | Zed, VS Code |
| `dmls/tests/packaging_contract.rs` | `just dist` archive names match the names `zed-dmls/src/lib.rs` downloads | whether `extension.toml` exists or parses, whether the extension builds |
| `just check-zed` | the extension crate type-checks for wasm | it checks `wasip1` not `wasip2`; it skips when the target is absent; no gate runs it |

Three layers are unverified today:

1. **The extension directory is a loadable Zed extension.** No test reads
   `extension.toml`. A deleted or renamed manifest, a wrong `id`, a missing
   `[language_servers.dmls]` block, or a `Cargo.toml` that no longer produces
   a `cdylib` would ship without any red test.
2. **Zed can build it.** The only compile check targets the wrong triple and
   is never mandatory. Nothing exercises the extension through Zed's manifest
   loader and component-packaging path.
3. **The installed extension on a developer's machine is still wired to
   something that exists.** This is the layer that actually failed. It is
   environmental, so no repository test can *prevent* it; the goal is that
   the first thing a developer runs when something looks wrong tells them
   exactly what is broken, instead of leaving them to guess between the
   binary, the extension, and Zed.

## Goals

- A manifest or crate-shape change that makes `zed-dmls` unloadable by Zed
  fails the normal L1 `just test` gate for `dmls` on every CI OS.
- The extension is compiled and packaged on the target Zed actually uses. The
  gate is mandatory on its designated CI runner and never silently skips.
- The dev-extension install no longer depends on a worktree path.
- A single command diagnoses the whole Zed launch path on a developer host:
  binary present and version-compatible, dev extension registered, registered
  target resolvable, manifest valid, and relevant recent Zed log errors. It
  must have caught both the dangling registration and the wrong-folder install
  attempt. It must not claim that a same-version binary is current when it
  cannot prove that from an installation receipt.
- Documentation and the justfile agree on the wasm target, and the Zed install
  steps make it hard to pick `vscode-dmls` by mistake.

## Non-goals

- Driving a real Zed instance in CI. Zed has no headless mode and no CLI entry
  point for installing or rebuilding a dev extension; it also needs a display.
  The sanctioned headless proof is Zed's own `zed-extension` CLI (below).
- Publishing `zed-dmls` to the Zed extension registry. That removes the
  dev-extension symlink entirely and is the long-term answer, but it needs a
  public repository, a release pipeline for the four `dmls` archives, and a
  registry submission. Track separately; this fix keeps the dev-extension
  flow working and observable.
- Changing anything in the VS Code extension.

## Proposed changes

> **Reader's note (2026-09-03 review):** The initial draft put the real Zed
> packager in L2, made the area `lint` recipe its CI authority, and implemented
> installation/diagnostics as Bash. Those choices conflict with repository
> contracts: L2 is reserved for real terminal resources, package CI invokes
> root package gates rather than area recipe lists, host discovery belongs to
> `sniff`, and user-facing terminal output uses `TerminalRenderable`. The
> reviewed design keeps passive contract checks in cross-platform L1, runs the
> official packager as an Ubuntu companion gate, and gives host maintenance a
> small typed Rust CLI. This adds one host-only package, but avoids encoding
> macOS path and symlink assumptions in shell.

### 1. `zed_extension_manifest` test (L1, pure file reads)

Add `dmls/tests/zed_extension_contract.rs` next to `packaging_contract.rs`,
same style: no shell, no build, cross-platform. It parses
`zed-dmls/extension.toml` with the `toml` crate (already a workspace
dependency) and asserts the invariants Zed's loader depends on:

- the file exists and parses;
- `id == "dmls"`, `schema_version == 1`, non-empty `name` and `version`;
- the manifest version equals the extension crate's package version;
- `[language_servers.dmls]` exists with `languages == ["Markdown"]`;
- `zed-dmls/Cargo.toml` has `crate-type = ["cdylib"]` and depends on
  `zed_extension_api`;
- `Cargo.lock` exists and resolves exactly one `zed_extension_api` package;
- `src/lib.rs` exists. Detailed source behavior remains Rust's compiler's
  responsibility rather than being checked with brittle source-string tests.

This would not have caught the 2026-08-30 incident by itself. It closes layer
1 so the manifest can never regress unnoticed, and it costs nothing. Do not
assert that the sibling VS Code directory lacks Zed files: that negative
cross-package rule would forbid a future deliberate integration without
proving anything about `zed-dmls` itself.

### 2. Correct `check-zed` and separate checking from provisioning

- Change the target to `wasm32-wasip2`. Zed's builder is the authority and it
  says `wasip2`; the README already says so; only the recipe disagrees.
  Both targets compile today, so this is a correctness fix, not a behavior
  change for the extension.
- Run `cargo check --locked --manifest-path
  ./dmls/zed-dmls/Cargo.toml --target wasm32-wasip2` and fail with an explicit
  installation command when the target is absent. A verification recipe must
  not mutate the caller's Rust toolchain or turn a network failure into a
  compile failure.
- Provision `wasm32-wasip2` in the CI setup step for the designated Zed gate.
  In `rust-latest-stable.yml`, request the target from the existing Rust
  setup action before invoking the same recipe.
- Add `check-zed` to the Darkmatter area `lint` recipe after the three native
  package lint calls. This makes the canonical local area gate and the weekly
  latest-stable area gate check the extension, while preserving the package
  lint calls as the source of native Clippy/fmt behavior.
- Do not rely on adding `check-zed` to `darkmatter/justfile::lint` for package
  CI. `_package-ci.yml` invokes `just _lint dmls` directly and does not execute
  package-area recipe lists. Section 3 adds an explicit reusable-workflow step
  so the gate actually runs and produces attributable failure evidence.

### 3. Package with Zed's own CLI (companion verification, not L2)

`zed-extension` (`crates/extension_cli` in the Zed repo) is the tool
`zed-industries/extensions` CI runs on every submission. It loads the manifest
with the same `ExtensionManifest::load`, compiles the wasm with the same
builder, validates the manifest, and writes `archive.tar.gz` plus
`manifest.json`. Running it against `zed-dmls/` is the closest headless
equivalent to "Zed installed this extension".

- Add `zed-extension` to the `runner-tools` closed vocabulary and document that
  it is consumed by a package companion gate, not by the L2 backend-proof
  mechanism. Keep DMLS's existing `tiers`, `l2-backends`, and Neovim L2 test
  unchanged.
- Pin the Zed commit SHA, Linux download URL, and expected SHA-256 digest in one
  reviewed CI configuration location. Download the official
  `x86_64-unknown-linux-gnu` binary, verify its digest before execution, cache
  it by Zed SHA, and fail provisioning if it is unavailable. Do not silently
  fall back to compiling Zed from a floating Git repository; that changes both
  the trust input and the runtime cost of the gate.
- Add `just zed-package` for local use and `just zed-verify` to run
  `check-zed` followed by packaging into temporary output and scratch
  directories. The verification asserts that `manifest.json` identifies
  `dmls`, declares Markdown, and that the archive contains `extension.wasm`.
  Temporary directories must be cleaned on success and failure.
- Add a conditional Ubuntu step to `_package-ci.yml`'s `dmls` lint producer
  when `runner-tools` contains `zed-extension`. That step provisions the WASM
  target and verified packager, then runs `just zed-verify`. The existing lint
  producer must report failure if either verification fails. Update the CI
  scope tests to prove a change below the workspace-excluded
  `darkmatter/dmls/zed-dmls/**` still selects `dmls` and the companion gate.
- Provision the pinned packager in the weekly latest-stable workflow and run
  `zed-verify` there too. The cross-platform L1 manifest test remains the
  OS-matrix proof; packaging runs once on Ubuntu because the WASM artifact and
  manifest validation are host-independent.

This test is intentionally not named `level2_*` and does not call
`require_level!`: repository policy reserves L2 for tests needing a real
terminal or PTY. A missing packager in its designated CI step is a provisioning
failure, never a test skip.

### 4. Stable install location plus a typed doctor command

The incident happened because the dev extension pointed into a worktree.
The maintenance behavior is substantial enough that a shell recipe would be
both untestable and platform-fragile. Add a small host-only `zed-dmls-cli`
workspace package under `darkmatter/dmls/zed-dmls-cli/`. Its binary is
`zed-dmls`, with `stage` and `doctor` subcommands; area recipes are thin
wrappers:

- `just install-zed` runs `zed-dmls stage` and then `zed-dmls doctor`;
- `just zed-doctor` runs `zed-dmls doctor`.

The package must:

- capture OS discovery once with the focused `sniff` API and derive per-user
  directories with the `dirs` crate; do not infer the OS with `uname` or
  environment-variable folklore;
- render human output with `biscuit-terminal` `TerminalRenderable` components
  (`Prose`, lists, or tables as appropriate), with a stable plain-text mode for
  tests and automation;
- accept `--staging-dir`, `--zed-data-dir`, and `--zed-log` overrides so Zed
  Preview/custom installs and hermetic tests do not depend on ambient paths;
- default the staging directory to the platform data directory for
  `dmls/zed-dmls`, outside every checkout: `Application Support` on macOS,
  XDG data on Linux, and local app data on Windows;
- stage an allowlisted source set (`extension.toml`, `Cargo.toml`,
  `Cargo.lock`, `src/`, plus user documentation) through a sibling temporary
  directory, never copy `target/`, and swap it into place with rollback so an
  interrupted update retains the previous usable copy;
- never mutate Zed's `extensions/installed` directory. Zed has no supported
  headless dev-install API, so registration remains a one-time manual action.
  Print the exact stable directory to select; do not mutate the clipboard
  implicitly.

`zed-dmls doctor` performs these checks:

| Check | Failing message |
|---|---|
| `dmls` on `PATH`; `dmls --version` equals the package version | binary missing or version-incompatible; do not label a same-version binary "current" |
| Zed extensions directory has an `installed/dmls` entry | "dev extension not installed; run `just install-zed`" |
| the registered entry resolves, regardless of whether Zed represented it as a symlink, junction, or another supported link | "dev extension points at `<path>`, which no longer exists (worktree removed?); run `just install-zed`" |
| resolved `extension.toml` parses and has `id = "dmls"` | selected target is not the DMLS Zed extension |
| newest bounded Zed log tail contains a relevant manifest error | quote the extension id and distinguish `dmls` (broken existing registration) from another id (wrong folder selected during install) |

Directory defaults must match the paths documented by Zed's `paths.rs`, but
all filesystem logic operates on `Path`/`PathBuf`; it must not construct paths
with hard-coded separators. The log is supporting evidence: an old error does
not make a now-valid registration fail. It becomes fatal only when it
corroborates a missing or invalid current registration; otherwise report it as
historical context.

`just install-dmls` invokes the doctor only when Zed or an existing DMLS dev
registration is detected. A doctor failure is a warning and does not change
the binary install's successful exit status, because Neovim, Helix, and VS Code
users do not depend on the Zed extension. `just install-zed` itself does fail
when staging fails, but after successful staging it exits with a distinct
documented status when the remaining manual Zed registration is required.

Unit tests inject the OS, executable lookup, paths, filesystem, and log tail.
They cover macOS, Linux/XDG, and Windows path shapes; missing Zed; missing
binary; absent registration; dangling symlink/junction-equivalent resolution;
missing and wrong-id manifests; a wrong-folder log entry; an old resolved log
error; repeat staging; stale-file removal; and rollback after an interrupted
stage. No test launches or focuses Zed.

### 5. Documentation and drift

- `zed-dmls/README.md` and `docs/editors/zed.md`: one wasm target
  (`wasm32-wasip2`), one install procedure (`just install-zed`, then select
  the stable path), one troubleshooting entry (`just zed-doctor`) that quotes
  the two Zed error lines seen in this incident and what each means:
  `...for extension dmls` on startup means the symlink dangles;
  `...for extension <other-name>` during install means the wrong folder was
  selected.
- `dmls/README.md` editor table: replace "Install `zed-dmls/` as a dev
  extension" with the recipe name.
- `.claude/skills/darkmatter/dmls.md` Verification section: list the new
  gates.
- Root workspace membership, the Darkmatter area package lists, and
  `docs/dependencies.md` (including the area copy) must include the new
  `zed-dmls-cli` package and its `sniff`, `dirs`, and `biscuit-terminal`
  dependencies.
- Fix the drift already found: `docs/editors/zed.md` says Zed compiles for
  `wasm32-wasip2`, the justfile says `wasip1`. Code (Zed's builder) wins.

## Acceptance criteria

1. Deleting `zed-dmls/extension.toml`, renaming `id`, or removing the
   `[language_servers.dmls]` block turns `just test` red for `dmls` on macOS,
   Linux, and Windows. Prove non-vacuity by making each edit, observing red,
   and reverting.
2. `just lint` in `darkmatter/` fails when `zed-dmls` does not compile for
   `wasm32-wasip2`; when the target is absent it fails with the exact
   provisioning command and does not install anything. The designated CI and
   latest-stable setup steps provision the target before running the recipe.
3. The Ubuntu companion gate packages `zed-dmls` with the digest-verified,
   pinned `zed-extension` binary and validates `manifest.json` and
   `extension.wasm`. A missing tool or digest mismatch fails provisioning; no
   L2 environment variable or skip path is involved.
4. With Zed's `installed/dmls` registration pointed at a non-existent path,
   `just zed-doctor` exits non-zero and names that path and the remedy. With
   `installed/dmls` pointing at a folder that has no `extension.toml`, it
   exits non-zero and says the selected target is not a DMLS Zed extension.
   With a correct install it exits zero even if an older startup in the log
   failed.
5. `just install-zed` on a fresh machine leaves a copy of the extension at
   the stable path and prints the exact folder to select in Zed. Running it
   again after editing `src/lib.rs` updates the copy without touching Zed.
6. Removing a worktree no longer changes what Zed loads: after
   `just install-zed` from worktree A, deleting worktree A, and restarting
   Zed, the Zed log contains no `No extension manifest found for extension
   dmls` line.
7. All docs listed in section 5 agree with the justfile on the target and
   the install procedure; `git grep wasip1 darkmatter/` returns nothing.
8. `zed-dmls-cli` unit tests pass on macOS, Linux, and Windows, and its terminal
   output is produced through `TerminalRenderable`. The doctor/stager tests do
   not launch Zed, open a terminal window, mutate the clipboard, or use host
   input.
9. CI scope tests prove a change under `dmls/zed-dmls/**` selects `dmls` and
   the Zed companion verification even though the extension crate is excluded
   from the Cargo workspace.

## Immediate remediation for this host

Not part of the code change, recorded so the next person does not repeat
2026-09-02:

1. In Zed, open `zed: extensions`, remove the dangling `dmls` dev extension.
2. `zed: install dev extension`, select
   `darkmatter/dmls/zed-dmls/` from the main checkout at
   `/Volumes/coding/personal/rusty-biscuit`, not from a worktree, until
   `just install-zed` exists.
3. Open a Markdown file and confirm `dmls` is listed in the language-server
   status menu.

## Open questions

None. This review resolves the two questions in the draft:

- `install-dmls` warns when Zed is present but unhealthy; it never invalidates
  an otherwise successful editor-neutral binary install.
- The official packager runs in the ordinary `dmls` package scope, which is
  already path-sensitive through affected-scope calculation. It is not added
  as a separate always-on matrix or misclassified as L2.
