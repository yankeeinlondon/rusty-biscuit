---
area: repo
status: implemented
created: 2026-09-23
clarified: true
clarified_by: opencode/zai-coding-plan/glm-5.3
reviewed: true
reviewed_by: opencode/zai-coding-plan/glm-5.3
reviewed_on: 2026-09-23
needs_rulings: false
review_iterations: 0
implemented: true
implemented_by: "opencode/zai-coding-plan/glm-5.3 (phases 1-4); claude-code/claude-opus-5-5 (phase 5)"
owner: "Ken Snyder <ken@ken.net>"
origin: "`just install` in darkmatter could not link the zed-dmls wasm extension on the dev Mac, 2026-09-23"
packages: []
references:
    spike-daemon-ignore-env.md: Measured proof (kache 0.23.1, scratch daemon) that the daemon honors [cache] ignore_env from the config file, with control run, socket/runtime-follows-store nuance, and raw-env probe-drip caveat.
    spike-shell-export-location.md: Traces the interactive-shell KACHE_CACHE_DIR export to ~/.env line 51 via adaptive.sh's allexport block, with neutralization proof and delete-only-line-51 removal guidance.
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
review_note: the clarification process served as a review
human_review: false
message_to_agent: |
    All five phases are implemented; the fix is ready for review (do not move
    it to _completed or run `just complete` — that is the author's call).
    The per-check verification summary is in implementation-log.md under
    "## Phase 5 → Spec verification summary". Points a reviewer should weigh:

    - Check 7: init's daemon version-mismatch restart trigger was not fired
      live (the daemon restarted because its executable was replaced). It is
      pinned only by the recipe text contract test
      ensure_kache_runs_the_spec_section_4_order.
    - Check 7's spec text ("place a regular file at the socket path") does not
      produce a failure on kache 0.26.3; a non-empty directory does. The spec
      text was left unedited.
    - Linux checks 1-4 are unmet for lack of a qualifying host (build-linux is
      ZFS without working block cloning).
    - Pre-existing, unrelated, build-linux only:
      ci_workflow_contracts::the_lint_step_measures_a_sub_second_command_instead_of_recording_zero.
    - Left to Ken: deleting the abandoned 56 GB ~/Library/Caches/kache, the
      live store being over its cap, and restarting the long-running
      `kache monitor` from a fresh shell.

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

1. kache is the latest release (never below the 0.23.0 floor), its daemon is
   installed and running, and on macOS it passes `DYLD_*` variables through to
   the compiler it wraps.
2. The store and the main checkout are on one filesystem that clones blocks,
   proven by a probe rather than inferred; the worktree base is on the same
   device once configured — policed by `kache-status`, not by the init verdict.
3. The store location has one source of truth — `[cache] local_store` written
   with `ignore_env = true` into `~/.config/kache/config.toml` — seen
   identically by interactive shells, the kache daemon, editors, agents, and
   scheduled jobs.
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
`just init` does not manage: the shell profile (its actual home is `~/.env`
line 51, exported by adaptive.sh's allexport block —
`spike-shell-export-location.md`) and the daemon's launchd plist
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

The verdict covers the main checkout (the repository root `init` runs in) and
the store. Both must be on **one device** (`stat -f %d` / `stat -c %d`), and a
clone probe from the store directory into the checkout must pass (`cp -c` plus
the device check on macOS, since `cp -c` silently copies across volumes;
`cp --reflink=always` on Linux; the volume's filesystem type on Windows, ReFS
only).

The probe runs before kache exists and before placement is final, so its
source is a candidate, not a configured store: `init` resolves the candidate
store directory with the placement cascade's pure directory logic (a `kache/`
directory at the volume's mount point on macOS when writable, else the user
cache dir when it is on the serving device, else a user-owned directory on
that volume — section 2), then clone-probes from that candidate into the
checkout and, when the base is configured, the worktree base.

The worktree base directory is resolved the way `wt` resolves it (`WT`, then
`~/.worktree.json`'s `base_dir`) and reported, never guessed — and it does not
gate the verdict (2026-09-23 ruling):

- **Unconfigured base.** `init` prints one line — "worktree base unconfigured —
  not covered by this verdict" — and qualifies on the checkout alone.
- **Configured base on another device.** No store placement can serve both.
  `init` names the two devices and does not move anyone's worktrees. The
  earlier draft's "kache stays off" gating is superseded by this ruling.

Init-time gating of the base buys no durable protection: the base can be
configured — or re-pointed to another volume — after activation regardless.
The durable guard is `kache-status`, which checks the checkout and base
devices once the base is configured. A worktree created under the base
directory inherits that directory's device, so one base on the serving device
covers every future worktree through the layout, not per-worktree bookkeeping —
that is how "create and destroy worktrees with no effect" is guaranteed.

The Windows branch ships unexercised (2026-09-23 ruling): the probe qualifies
ReFS by filesystem type exactly as `kache-status` does today, and a ReFS Dev
Drive host gets the full treatment. No Windows host exists to test on; the
branch is small and reuses existing probe logic.

### 2. Store placement follows the checkout, with one source of truth

- `kache doctor --json` reads the default store that feeds the placement
  decision. If that default is already on the serving device, that is the
  store — superseding this bullet's original "use it and set nothing"
  reading (2026-09-23 ruling): init always writes the ratified value even
  when it equals the default, because the pin is deliberate — a future kache
  release changing its default cannot silently relocate this host's store.
  Otherwise the store goes on the serving device:
  - macOS: a `kache/` directory at the volume's mount point when writable
    (this host's existing `/Volumes/coding/kache` is that rule applied), else
    the fallback cascade below;
  - the fallback cascade (2026-09-23 ruling): the user cache dir —
    `$XDG_CACHE_HOME/kache` on Linux, `~/Library/Caches/kache` on macOS —
    when it is on the serving device, the common single-disk btrfs/XFS case
    where it usually equals the default anyway; else a user-owned directory
    on the serving volume; else report failure with the reason. Never write
    a mount point such as `/` or `/home`; they are root-owned. One cascade
    rule, both OSes — it replaces the mount-point rule on Linux, and macOS
    adopts it when the mount point is not writable.
- `init` never moves an existing store's contents without saying so. It reports
  the old store's size and leaves deleting it to the human.
- **One source of truth: the user config file.** Verified empirically on kache
  0.23.1 (`kache doctor` against scratch configs): `~/.config/kache/config.toml`
  can carry the store path as `[cache] local_store`. Store resolution, highest
  priority first:

  1. env `KACHE_CACHE_DIR`
  2. `[cache] local_store` in the user config — wins over env when the same
     file sets `[cache] ignore_env = true`
  3. a `kache.toml` in the current working directory — a real but
     lowest-priority layer; it cannot carry the store path (host-specific),
     and its `ignore_env` has no effect: only the user-level file can gate env
  4. kache's built-in default

  `KACHE_DISABLED` is ungated — honored whichever layer wins.

  Ruling (2026-09-23): `init` writes `[cache] local_store = "<serving-device
  path>"` plus `ignore_env = true` into `~/.config/kache/config.toml` on every
  qualifying host — even when the value equals kache's default, because the
  pin is deliberate: a future kache release changing its default cannot
  silently relocate this host's store. The file always wins, for every
  process including the daemon — measured 2026-09-23 on kache 0.23.1: with
  `KACHE_CACHE_DIR` set in its environment and `ignore_env = true` in the
  config, a scratch daemon placed its store, socket, and index in the
  config-declared location, while a control run without `ignore_env`
  materialized the env store (`spike-daemon-ignore-env.md`; `ignore_env`
  gates all env overrides, `KACHE_RUNTIME_DIR` included, so the daemon
  socket follows the store when not explicitly separated). Stray env
  exports are inert for store resolution — kache's version-probe files
  still read raw env, so a stale `KACHE_CACHE_DIR` keeps dripping probe
  jsons into the old location even after `ignore_env` is written, one more
  reason the exports must actually be removed. The existing hand-set
  `KACHE_CACHE_DIR` exports are removed during implementation: the shell
  export at `~/.env` line 51 — exported by the `set -a` allexport block in
  `adaptive_setup()` (`~/.config/sh/adaptive.sh` lines 330–336), reaching
  interactive zsh only (`spike-shell-export-location.md`) — is deleted, and
  the daemon plist is regenerated (`kache daemon uninstall && install`) as
  cleanup hygiene, not a hard prerequisite of activation: the daemon
  resolves the ratified store even while its plist still carries the stale
  env.

  The write is a targeted merge, not a rewrite (2026-09-23 ruling): a
  TOML-aware edit of `~/.config/kache/config.toml`
  (`%APPDATA%\kache\config.toml` on Windows) that sets exactly `local_store`
  and `ignore_env` and preserves every other key and comment — this host's
  file carries hand-tuned caps, event-log sizing, and GC policy with
  rationale comments, and clobbering them is hostile. No comment-preserving
  TOML writer is available: python3's `tomllib` only parses, and `tomlkit`
  is not installed. The edit is therefore line-oriented and confined to the
  `[cache]` table: replace an existing `local_store =` / `ignore_env =` line,
  or insert the missing one at the end of that table, or append a `[cache]`
  table when there is none. It is written to a temporary file, **validated by
  parsing it with `tomllib`** (the whole file parses, and `cache.local_store`
  and `cache.ignore_env` hold the intended values), and only then renamed over
  the original, with the previous file kept as a dated backup (the pattern
  already used on this host). A validation failure leaves the original
  untouched and falls under the failure contract (section 4).

  That the daemon — not just the CLI — honors `ignore_env` was the one thing
  clarification could not verify; the spike closed it (2026-09-23, kache
  0.23.1, scratch daemon, control run — `spike-daemon-ignore-env.md`), and
  verification step 8 below is its end-to-end confirmation on the real host.
- `kache-status` reads the store location from `kache doctor --json`, never
  from its own reconstruction of kache's resolution rules. That is the defect
  behind today's false verdict.

### 3. Install means "latest, and usable"

`install-kache` (called by `init` on qualifying hosts):

- installs **or upgrades** to the latest release with `cargo binstall`. It no
  longer stops at "present and above the floor". `.github/kache-min-version`
  remains the floor the maintenance audit enforces, raised 0.15.0 → **0.23.0**
  (2026-09-23). Both keys are older (upstream release notes introduce
  `local_store` and `ignore_env` in v0.7.0); 0.23.0 is the floor because the
  0.23 line is the one measured here — the precedence stack and the daemon
  spike ran on 0.23.1. A below-floor install is always upgraded before `init`
  writes the config. Latest upstream was v0.26.3 on 2026-09-23, so the first
  qualifying `init` moves past the measured line; verification runs against
  whatever it installs.
- on macOS, re-signs the installed binary ad hoc (`codesign --force -s -`)
  immediately after every install or upgrade, and verifies the result with the
  env-passthrough probe (a stub compiler run through kache that confirms a
  `DYLD_*` variable arrives). The re-sign lives inside `install-kache`, not in
  `init`, so no upgrade path can skip it.
- the ownership split is explicit (2026-09-23 ruling): `install-kache` does
  no daemon work in either mode — its binary-only mode simply means no daemon
  work — while `init` owns the daemon lifecycle on qualifying hosts as its
  own step 6 (section 4), invoked strictly after the config write:
  `kache daemon install` when absent, ensure it is running, and restart it
  when either trigger fires:
  - **binary changed** — upgrades included — so the running daemon is never
    the old hardened build. Detection is explicit (2026-09-23 ruling):
    compare the daemon-reported version from `kache doctor --json`
    (`daemon version` field) against the installed binary's version; no
    mtime guessing.
  - **config changed** — the step-4 write modified the file. A daemon
    already running (this Mac's case) read its store and socket location at
    startup; writing `ignore_env = true` moves the socket into the store
    directory (`KACHE_RUNTIME_DIR` is gated too — `spike-daemon-ignore-env.md`),
    so without a restart the CLI looks for the socket where the old daemon
    is not listening. The version is unchanged, so the version trigger alone
    would never fire.

  The after-write ordering is the point of the split: on a fresh host the
  daemon's first read of the config is the ratified one. The daemon
  exists on the dev Mac today only because `kache daemon install` was run by
  hand once; the earlier wording ("restarts the daemon when the binary
  changed") said nothing about who installs it. Plist regeneration — shedding
  the hand-set `KACHE_CACHE_DIR` from the launchd environment — is host
  cleanup (Changes), not part of this lifecycle and not a gate on
  activation: the daemon resolves the ratified store even while the stale
  env persists (`spike-daemon-ignore-env.md`).

Re-signing is chosen over installing from source (`cargo install kache`, which
produces an ad hoc–signed binary by construction) because it costs a second
instead of a full compile, and keeps binstall as the single install path the
2026-07-30 decision chose. Source install is the fallback if a future release
cannot be re-signed.

Hosts that do not qualify (2026-09-23 rulings; the OS-name install rule from
the 2026-07-30 integration is superseded — the filesystem decides installation
too):

- **kache absent:** `init` never installs it. A person who wants kache on such
  a host can still run `just install-kache` by hand.
- **kache present:** `init` checks the version against the floor. Meets floor —
  report only. Below floor — interactive `init` asks the user to confirm
  upgrading to the latest via `just install-kache`; a non-interactive
  invocation (no terminal) stops and reports an error rather than upgrading or
  silently skipping. "For now" is the current stance, recorded as the
  2026-09-23 ruling. The confirmed upgrade is binary-only (same ruling):
  re-sign on macOS — same flag, harmless — and no daemon work follows; the
  daemon lifecycle stays scoped to qualifying hosts' step 6. `install-kache`
  keeps an explicit binary-only mode (or flag) that `init` selects from the
  verdict.
- **CI/CD:** never installs or upgrades kache at all. `docs/kache-strategy.md`
  records the desire ("CI: kache removed"; `Swatinem/rust-cache@v2` stays), and
  no workflow runs `just init` today — if one ever does, `init` must not fight
  it.

### 4. Order of operations and the failure contract

On a fresh host, `_ensure-kache` runs in this order (2026-09-23 ruling):

1. filesystem qualification probe — needs no kache installed: device ids plus
   clone probes between directories, sourced from the candidate store
   directory (section 1);
2. if qualifying, `install-kache` — install or upgrade to the latest
   (section 3);
3. `kache doctor --json` reads the default store that feeds the placement
   decision;
4. write the user config (`local_store` + `ignore_env`, targeted merge —
   section 2);
5. on macOS, re-sign plus env-passthrough verification — idempotent when
   `install-kache` already re-signed (section 3); the gate is the
   verification;
6. daemon lifecycle (section 3) — `kache daemon install` when absent, ensure
   it is running, restart when the doctor-reported daemon version mismatches
   the installed binary or step 4 changed the config — deliberately after
   the config write, so the
   daemon's first read of the store is the ratified one (a stale
   `KACHE_CACHE_DIR` in the plist cannot flip that read — verified,
   `spike-daemon-ignore-env.md`; the plist regeneration in Changes is
   cleanup hygiene, not a prerequisite of step 7);
7. activation, last.

A non-qualifying verdict ends the sequence at step 1 — the below-floor
upgrade path in section 3 is the only continuation, and it is binary-only.

Failure contract (same ruling): any failure before activation prints loud
WARNING lines, leaves kache off, and `just init` still completes its other
setup steps — this extends the read-only-config precedent on the build-linux
host, where drift is reported but not fatal so init completes. Activation
happens only when every prior check passed: a failed re-sign or daemon step
can never leave builds broken — the dyld failure would return. `kache
doctor` failing at its step (after install, when placement is decided) makes
placement undecidable: kache is skipped with that reason, in the same bucket.

### 5. Activation

On a qualifying host, `init` activates kache host-wide — last in the
sequence, after every prior check passed (section 4) — in
`$CARGO_HOME/config.toml` (`[build] rustc-wrapper = "kache"`), which is how
this host already runs, and never in a tracked `.cargo/config.toml`, which
stays forbidden. On a non-qualifying host, `init` does not activate. If it finds
kache already active there, it reports that as the drift the 2026-09-09 ruling
warns about and prints the undo, but leaves the change to the human.

"A `target/` is always wrapped or never wrapped" is enforced by the layout rule
in section 1. In clone mode a restored artifact is an ordinary writable file,
so the hardlink failure that motivated the rule cannot occur.

Recipes and CI that neutralize the wrapper with `RUSTC_WRAPPER=""` (empty
string) keep working against the config-file wrapper — Cargo treats the empty
value as no-wrapper — while code that merely unsets the variable now inherits
the wrapper on qualifying hosts. Standing cross-check clones that must stay
unwrapped use the empty-string form: the "always wrapped or never wrapped"
rule restated for the config-file era.

### 6. What `init` prints

One block, in the style of the other `_ensure-*` steps: the qualification
verdict and the fact that decided it (device ids, probe result), the
worktree-base line when the base is unconfigured or off-device, kache version
against the 0.23.0 floor, env-passthrough result, store path and whether it had
to move, daemon state, and activation state. A non-qualifying host gets one
line naming the reason, plus the floor check's result when kache is installed
there. A pre-activation failure prints its WARNING lines into the same block —
kache off, reason named, `init` completing — per the failure contract.

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
    never install kache where the probe does not qualify; on qualifying hosts
    run the ordered sequence from Design section 4 — upgrade, write the user
    kache config (`[cache] local_store` + `ignore_env = true`, targeted
    merge), daemon after the config write, activation last — under the
    failure contract, and report an old/abandoned store's size, leaving
    deleting it to the human; on a non-qualifying host with kache installed,
    apply the floor check, whose confirmed upgrade is binary-only, and report
    an already-active wrapper as drift with the undo printed, leaving the
    change to the human (section 5).
  - `install-kache`: always target the latest release; macOS ad hoc re-sign
    plus env-passthrough verification in both modes, so no upgrade path skips
    the re-sign — the binary-only mode simply means no daemon work; no daemon
    lifecycle here, it belongs to `_ensure-kache`'s post-config-write step
    (install when absent, ensure running, restart when the doctor-reported
    daemon version mismatches the installed binary or the config write
    changed the file).
  - `kache-status`: store from `kache doctor --json`; add the env-passthrough
    line and the checkout/worktree-base device check; share the probe script.
- `.github/kache-min-version`: 0.15.0 → 0.23.0.
- New shared probe script under `scripts/`, used by both recipes.
- `tools/test-toolkit/tests/ci_workflow_contracts.rs`: the contract tests
  assert today's justfile text ("never reinstalling", `init` must call
  `_ensure-kache`, the install-kache wording); update the assertions to the
  contract this spec defines.
- `docs/kache-strategy.md`: rewrite the repo-integration bullets and the
  2026-09-09 ruling table's install/activation wording to "`init` decides from
  the probe"; fix the Mac table's store path (it still says
  `~/Library/Caches/kache`); record the hardened-runtime finding.
- `README.md` (line ~93 still says kache "is installed by `just init` on macOS
  and Linux but never activated by the repository") and `docs/initialization.md`:
  both describe current behavior and drift when this lands; update them.
- `.claude/skills/kache/` (scope expanded, 2026-09-23): the
  hardened-runtime/`DYLD_*` trap and its symptom; a correction to the
  store-location table (the store is wherever `kache doctor` says it is); the
  verified env/file precedence stack including `local_store` and `ignore_env`;
  that `init` now owns store placement (it writes the user config), activation,
  and the daemon lifecycle; the 0.23.0 floor and why; and that a cwd
  `kache.toml` is a real lowest-priority layer that cannot carry the store path.
- `.claude/skills/os/`: a short entry for the macOS trap, pointing at the kache
  skill.
- This host, once the recipes are in: remove the hand-made `libLLVM.dylib`
  symlinks from the `1.98.1` and `stable` toolchains and confirm the wasm still
  links; remove the hand-set `KACHE_CACHE_DIR` exports — delete `~/.env`
  line 51 and nothing else: the `set -a` allexport block must survive, it
  exports every other line of `~/.env` (`spike-shell-export-location.md`);
  then regenerate the daemon plist (`kache daemon uninstall && install`) —
  cleanup hygiene, not an activation gate: the daemon resolves the ratified
  store regardless, and regeneration stops the raw-env probe drips
  (`spike-daemon-ignore-env.md`); report the abandoned 56 GiB store.

## Verification

Each of these must pass on the dev Mac, and 1 through 4 also on one Linux host
whose filesystem qualifies, if one is available:

1. **Pristine toolchain.** With the toolchain symlinks removed, `just zed-wasm`
   in `darkmatter` links through kache.
2. **Passthrough probe.** The stub-compiler probe shows `DYLD_*` arriving; the
   same probe against an unmodified binstall release shows it missing. That
   specimen is a pristine copy fetched to a temp location first — `init`
   re-signs the installed binary in place, so the installed one cannot serve
   as the unmodified release. This proves the probe can fail.
3. **Worktree lifecycle.** Create a worktree with `wt`, build a package, and
   record the store size. Destroy the worktree, create a fresh one, and rebuild.
   The rebuild is served from the store; apart from index and event-log churn
   — a few hundred MiB at this store's scale — the store size is unchanged
   after the destroy-and-rebuild cycle, and `kache doctor` and `kache stats`
   report no issues. Nothing kache-specific is run
   between those steps.
4. **Launcher parity.** A process started without the shell profile (a
   `launchctl` one-shot, or `env -i` with only `HOME` and `PATH`) resolves the
   same store as an interactive shell, because both read the config file. No
   managed launcher still sets `KACHE_CACHE_DIR`, and no new files appear under
   `~/Library/Caches/kache`.
5. **Idempotence.** A second `just init` changes nothing, restarts no daemon,
   and reports the same verdict.
6. **Negative host.** WSL ext4 is the canonical negative case; the APFS
   construction of one is a store and worktree base on one device with the
   checkout on another device — a checkout on a second APFS volume serving
   its own local store would legitimately qualify, so that alone is not a
   negative. On the negative host, `init` leaves kache off, never installs
   it, and names the reason; `kache-status` agrees. With kache installed
   there below the 0.23.0 floor, a non-interactive `init` stops with an
   error rather than upgrading; an interactively confirmed upgrade is
   binary-only — no daemon is installed or restarted there.
7. **Upgrade path.** Downgrade kache below the 0.23.0 floor, run `just init`,
   and confirm it upgrades to the latest, re-signs, restarts the daemon, and
   passes check 2. Then force a pre-activation failure by hand — stop the
   daemon and place a regular file at its expected socket path so the restart
   attempt fails — and confirm the failure contract: WARNING lines,
   kache left off, `init` completing its other steps.
8. **Daemon honors the config.** While the daemon's plist still carries
   `KACHE_CACHE_DIR` (before the regeneration, or re-set by hand for the
   test), the running daemon resolves the same store as the CLI. The
   mechanism is already proven in a scratch environment (2026-09-23, kache
   0.23.1, with control run — `spike-daemon-ignore-env.md`); this check is
   its end-to-end confirmation on the real host. The first `init` on this
   host runs against an already-running daemon, so it must restart it on the
   config-change trigger; afterwards `kache daemon` (status) from a fresh
   shell reaches the daemon at its new socket location.

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
- **A tracked `kache.toml` as policy.** The cwd `kache.toml` layer cannot carry
  the store path (host-specific) and cannot gate env, but a tracked one could
  someday carry host-agnostic policy — caps, exclusions. Follow-up, not this
  fix.
