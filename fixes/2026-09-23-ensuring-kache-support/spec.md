---
area: repo
status: draft-spec
created: 2026-09-23
clarified: false
reviewed: false
review_iterations: 0
implemented: false
owner: Ken Snyder <ken@ken.net>
origin: `just install` in darkmatter could not link the zed-dmls wasm extension on the dev Mac, 2026-09-23
packages: []
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
---

# Make kache a host setup that `just init` owns end to end

## Outcome

On a host whose filesystem earns kache, `just init` leaves a build environment
in which kache is invisible: every build that works without the wrapper works
with it, every checkout and worktree of this repository restores from the store
by clone, and creating or destroying a worktree needs no kache step at all. On a
host whose filesystem does not earn it, `just init` leaves kache off and says
why.

Concretely, after `just init` on a qualifying host:

1. kache is the latest release, and on macOS it passes `DYLD_*` variables
   through to the compiler it wraps.
2. The store, the main checkout, and the worktree base directory are on one
   filesystem that clones blocks, proven by a probe rather than inferred.
3. The store location has one source of truth, seen identically by interactive
   shells, the kache daemon, editors, agents, and scheduled jobs.
4. kache is activated.
5. `just kache-status` reports all of the above correctly, and fails loudly when
   any of it drifts.

The decision is made by the **filesystem**, never by the OS name. macOS on APFS
will always qualify when the layout is right; Linux on btrfs, XFS with reflink,
or ZFS with working block cloning qualifies; Windows on a ReFS Dev Drive
qualifies. NTFS, ext4, and ext4-in-VHDX (WSL) do not.

## Problem and evidence

### The failure

`just install` in `darkmatter` rebuilds the Zed extension with
`cargo build --release --target wasm32-wasip2`. On the dev Mac it failed to link:

```text
dyld: Library not loaded: @rpath/libLLVM.dylib
  Referenced from: ~/.rustup/toolchains/1.98.1-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/rust-lld
error: failed to invoke LLD: signal: 6 (SIGABRT)
```

The recipe then fell back to staging the committed `extension.wasm`, so Zed kept
running the previously built artifact and the failure was easy to miss. CI was
green because the `zed-dmls` companion check runs `cargo check`, which never
links.

### Root cause: the prebuilt kache strips `DYLD_*`

From 1.98 on aarch64 macOS (and on current nightly; 1.97.1 does not), the
self-contained `rust-lld` links `libLLVM.dylib` dynamically. The toolchain ships
the library in `<toolchain>/lib/`, not beside `rust-lld`, and finding it depends
on rustup exporting `DYLD_FALLBACK_LIBRARY_PATH` to the processes it launches.

The kache release binary that `cargo binstall` installs is signed with the
**hardened runtime** (`codesign -dvv` → `flags=0x10000(runtime)`). dyld removes
every `DYLD_*` variable from a hardened process's environment at launch, so
kache cannot forward what it never sees, and `rustc` → `wasm-component-ld` →
`rust-lld` runs without it.

Measured on 2026-09-23:

| Setup | Result |
|---|---|
| Pristine 1.98.1 in a scratch `RUSTUP_HOME`, no wrapper | wasm links |
| Same toolchain, `RUSTC_WRAPPER=kache` (binstall release, hardened) | the exact failure above |
| Fake `rustc` that logs its env, run through the hardened kache | `FOO=bar` arrives, `DYLD_FALLBACK_LIBRARY_PATH` does not |
| Same binary re-signed ad hoc (`codesign --force -s -`, `flags=0x2(adhoc)`) | both arrive |

The kache 0.19.0 backup on this host is hardened too, so this is not a
regression in one release. The C/C++ shims under `~/.local/lib/kache/shims` are
symlinks to the same binary and inherit the same behavior.

A symlink placing `libLLVM.dylib` beside `rust-lld` also cures it, and one
already exists (hand-made, 2026-09-05) in the `stable` toolchain. That only
patches one toolchain, disappears on the next `just toolchain-upgrade`, and
leaves the actual cause in place. It is removed by this fix (see Changes).

### The store has two locations, and the status recipe checks the wrong one

The store was moved to `/Volumes/coding/kache` so it shares a volume with the
checkouts. The move is carried by `KACHE_CACHE_DIR`, set by hand in two places
`just init` does not manage: `~/.zshrc` and the daemon's launchd plist
(`ninja.kunobi.kache`). Consequences observed on 2026-09-23:

- **`just kache-status` gives a false verdict.** It resolves the store from
  `KACHE_DIR` (a name nothing sets) falling back to `~/Library/Caches/kache`, so
  it probes the old store on the Data volume and reports "active WITHOUT
  copy-on-write". `kache doctor` on the same host reports the real store,
  `/Volumes/coding/kache`, with FICLONE supported.
- **The old store is abandoned, not gone.** `~/Library/Caches/kache` still holds
  **56 GiB**; its `index.db` was last written 2026-09-12. The live store is
  102 GiB.
- **Processes that don't source `~/.zshrc` still land there.** The old directory
  keeps receiving kache's version-probe files (`rustc-ver-*`,
  `rustc-sysroot-*`, `linker-ver-*`, several hundred of them, the newest from
  today). Editors launched from the Dock, launchd jobs, and some agent sessions
  never see the variable, so they run with a different store location from the
  shell.

### What `just init` does today, and the rulings it encodes

`_ensure-kache` decides by OS name (macOS and Linux yes; Windows and WSL no),
installs kache only when it is absent, never upgrades one that is present, and
never activates. `docs/kache-strategy.md` records these as deliberate
(2026-07-30 integration, 2026-09-09 ruling): activation was left to the host
because kache's restore mode is a property of the filesystem and the repository
"cannot see it".

The first premise stands. The second no longer does: `just kache-status`
already implements the probe that sees it. This spec moves the decision from
"a human reads the probe and edits their Cargo config" to "`just init` runs the
probe and acts on it".

## Design

### 1. Qualification is a probe, run by `init`

`init` answers one question: *can this host clone a file from the store
location into every place this repository's `target/` directories live?* It
uses the probe logic now in `kache-status`, extracted into one script both
recipes call (`scripts/kache-host.sh` or similar), so the verdict and the report
cannot disagree.

The set of places a `target/` lives is:

- the main checkout (the repository root `init` runs in), and
- the worktree base directory, resolved the way `wt` resolves it: `WT`, then
  `~/.worktree.json`'s `base_dir`. If neither is set, the base is unconfigured,
  and `init` reports that rather than guessing a default.

All of them, and the store, must be on **one device** (`stat -f %d` /
`stat -c %d`), and a clone probe from the store directory into each must pass
(`cp -c` plus the device check on macOS, since `cp -c` silently copies across
volumes; `cp --reflink=always` on Linux; the volume's filesystem type on
Windows, ReFS only).

A worktree created under the base directory inherits that directory's device, so
qualifying the base qualifies every future worktree. That is how "create and
destroy worktrees with no effect" is guaranteed: through the layout, not
per-worktree bookkeeping.

When the checkout and the worktree base sit on different devices, no store
placement can serve both. `init` reports that as the reason kache stays off and
names the two devices; it does not move anyone's worktrees.

### 2. Store placement follows the checkout, with one source of truth

- If kache's default store (`kache doctor --json` → cache dir) is already on the
  serving device, use it and set nothing.
- Otherwise the store goes on the serving device. The default location is a
  `kache/` directory at the volume's mount point when writable (this host's
  existing `/Volumes/coding/kache` is that rule applied). `init` never moves an
  existing store's contents without saying so. It reports the old store's
  size and leaves deleting it to the human.
- **One source of truth.** If kache's `config.toml` can carry the store path,
  that is where it goes, because every process reads that file whatever
  environment it was launched with. If it cannot, `init` writes the variable
  into each launcher it manages (shell profile snippet, launchd plist or systemd
  unit) from one value, and `kache-status` checks that they agree. Which of the
  two applies is **Open question 1**.
- `kache-status` reads the store location from `kache doctor --json`, never
  from its own reconstruction of kache's resolution rules. That is the defect
  behind today's false verdict.

### 3. Install means "latest, and usable"

`install-kache` (called by `init` on qualifying hosts):

- installs **or upgrades** to the latest release with `cargo binstall`. It no
  longer stops at "present and above the floor". `.github/kache-min-version`
  remains the floor the maintenance audit enforces.
- on macOS, re-signs the installed binary ad hoc (`codesign --force -s -`)
  immediately after every install or upgrade, and verifies the result with the
  env-passthrough probe (a stub compiler run through kache that confirms a
  `DYLD_*` variable arrives). The re-sign lives inside `install-kache`, not in
  `init`, so no upgrade path can skip it.
- restarts the daemon when the binary changed, so the running daemon is never
  the old hardened build.

Re-signing is chosen over installing from source (`cargo install kache`, which
produces an ad hoc–signed binary by construction) because it costs a second
instead of a full compile, and keeps binstall as the single install path the
2026-07-30 decision chose. Source install is the fallback if a future release
cannot be re-signed.

### 4. Activation

On a qualifying host, `init` activates kache host-wide in
`$CARGO_HOME/config.toml` (`[build] rustc-wrapper = "kache"`), which is how
this host already runs, and never in a tracked `.cargo/config.toml`, which
stays forbidden. On a non-qualifying host, `init` does not activate. If it finds
kache already active there, it reports that as the drift the 2026-09-09 ruling
warns about and prints the undo, but leaves the change to the human.

"A `target/` is always wrapped or never wrapped" is enforced by the layout rule
in section 1. In clone mode a restored artifact is an ordinary writable file,
so the hardlink failure that motivated the rule cannot occur.

### 5. What `init` prints

One block, in the style of the other `_ensure-*` steps: the qualification
verdict and the fact that decided it (device ids, probe result), kache
version, env-passthrough result, store path and whether it had to move, and
activation state. A non-qualifying host gets one line naming the reason.

### Rejected alternatives

- **Symlink `libLLVM.dylib` into each toolchain.** Treats one symptom of the
  env stripping, per toolchain, and must be redone at every toolchain upgrade.
- **Export `DYLD_FALLBACK_LIBRARY_PATH` in recipes.** The hardened kache strips
  it no matter who sets it.
- **Bypass kache in the `zed-wasm` recipe (`RUSTC_WRAPPER=""`).** Fixes one
  recipe and leaves every other link through kache exposed to the same cause.
- **Keep deciding by OS name.** The 2026-09-09 ruling already says the
  filesystem decides. The OS switch in `_ensure-kache` is the one place that
  still contradicts it.

## Changes

- Root `justfile`
  - `_ensure-kache`: replace the OS `case` with the shared qualification probe;
    upgrade, place the store, and activate on qualifying hosts.
  - `install-kache`: always target the latest release; macOS ad hoc re-sign plus
    env-passthrough verification; daemon restart on binary change.
  - `kache-status`: store from `kache doctor --json`; add the env-passthrough
    line and the checkout/worktree-base device check; share the probe script.
- New shared probe script under `scripts/`, used by both recipes.
- `docs/kache-strategy.md`: rewrite the repo-integration bullets and the
  2026-09-09 ruling table's install/activation wording to "`init` decides from
  the probe"; fix the Mac table's store path (it still says
  `~/Library/Caches/kache`); record the hardened-runtime finding.
- `.claude/skills/kache/`: the hardened-runtime/`DYLD_*` trap and its symptom,
  and a correction to the store-location table (the store is wherever `kache
  doctor` says it is).
- `.claude/skills/os/`: a short entry for the macOS trap, pointing at the kache
  skill.
- This host, once the recipes are in: remove the hand-made `libLLVM.dylib`
  symlinks from the `1.98.1` and `stable` toolchains and confirm the wasm still
  links; report the abandoned 56 GiB store.

## Verification

Each of these must pass on the dev Mac, and 1 through 4 also on one Linux host
whose filesystem qualifies, if one is available:

1. **Pristine toolchain.** With the toolchain symlinks removed, `just zed-wasm`
   in `darkmatter` links through kache.
2. **Passthrough probe.** The stub-compiler probe shows `DYLD_*` arriving; the
   same probe against an unmodified binstall release shows it missing. This
   proves the probe can fail.
3. **Worktree lifecycle.** Create a worktree with `wt`, build a package, and
   record the store size. Destroy the worktree, create a fresh one, and rebuild.
   The rebuild is served from the store, the store does not grow, and `kache
   doctor` and `kache stats` report no issues. Nothing kache-specific is run
   between those steps.
4. **Launcher parity.** A process started without the shell profile (a
   `launchctl` one-shot, or `env -i` with only `HOME` and `PATH`) resolves the
   same store as an interactive shell. No new files appear under
   `~/Library/Caches/kache`.
5. **Idempotence.** A second `just init` changes nothing and reports the same
   verdict.
6. **Negative host.** On a non-qualifying filesystem (WSL ext4, or a checkout
   on a second APFS volume), `init` leaves kache off and names the reason, and
   `kache-status` agrees.
7. **Upgrade path.** Downgrade kache to an older release, run `just init`, and
   confirm it upgrades, re-signs, and passes check 2.

## Open questions

1. **Can kache's `config.toml` carry the store path?** If yes, it is the single
   source of truth (section 2). If no, `init` manages the variable in each
   launcher from one value. Check the 0.23 config schema before planning.
2. **Mount-point writability.** On Linux the checkout's mount point is often
   `/` or `/home`. Should the non-default store go under a user-owned directory
   on that device (for example `$XDG_CACHE_HOME` when it is on the same
   device) rather than at the mount point?
3. **Windows.** The probe would qualify a ReFS Dev Drive, which matches the
   2026-09-09 ruling. No such host exists yet. Is it acceptable to ship that
   branch unexercised, or should Windows report "not yet supported" until a
   Dev Drive is measured?

## Not done here, and follow-ups

- **Report upstream to kache:** a compiler wrapper should ship with the
  `com.apple.security.cs.allow-dyld-environment-variables` entitlement, or
  without the hardened runtime. When it does, the re-sign step becomes a no-op
  and can be removed.
- **CI never links the Zed extension.** The companion check is `cargo check`.
  Linking would not have caught this failure (CI does not use kache), so this is
  recorded, not scheduled.
- **The unversioned sweep wrapper** (`~/.local/bin/rusty-biscuit-sweep.sh`)
  lists its roots by hand. Deriving them from the same checkout and worktree
  base resolution would stop the list from drifting when worktrees move.
