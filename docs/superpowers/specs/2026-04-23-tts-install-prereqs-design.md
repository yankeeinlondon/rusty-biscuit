# TTS Install System-Prerequisite Support Design

Adds first-class support for system-level prerequisites to `sniff`'s install-plan machinery so that `so-you-say install <provider>` completes with a *working* provider rather than an installed-but-broken one.

## Motivation

On Linux, `so-you-say install kokoro` installs the `kokoro-tts` Python package via `uv tool install`, but the package's `sounddevice` dependency `dlopen`s `libportaudio.so.2` at import time. Python wheels for `sounddevice` on Linux do not bundle PortAudio, so the first invocation of `kokoro-tts` fails with:

```
OSError: PortAudio library not found
```

The user has no way to know this from the `so-you-say install` output, which reports success. The same class of problem affects `piper` (needs `espeak-ng` binary) and `echogarden` (needs `ffmpeg`). Each of these would be resolved by installing one system package (`libportaudio2`, `espeak-ng`, `ffmpeg`) via the host's default OS package manager before the tool-level install runs.

## Goals

- Declare system-level prerequisites as part of a program's metadata.
- Detect whether a prereq is already present on the host (skip silently if so).
- Resolve prereqs to a concrete install command using the same OS/manager bucket logic as the existing install plan.
- Show one upfront plan covering all prereqs + the main install, gated by one consent prompt.
- Fail fast when any required prereq cannot be installed on the current host (no partial installs).
- Cover `kokoro-tts`, `piper`, and `echogarden` as the three v1 consumers.

## Non-Goals

- Optional prereqs (all v1 prereqs are required).
- Version pinning of prereqs (we install whatever the OS package manager provides).
- Prereq uninstall or cleanup.
- Prereqs that depend on other prereqs (flat list only).

## Approach

Extend `sniff` with a `SystemPrerequisite` type, add a `system_prerequisites` field to `ProgramInfo`, and build a `FullInstallPlan` wrapper around the existing `InstallPlan`. A parallel `run_full_install_interview` drives execution with one combined announcement and one consent prompt. The existing `build_install_plan` / `run_install_interview` entry points are retained for callers that don't need prereq handling.

Rejected alternatives:

- **Fold prereqs into `installation_methods`** — conflates "pick one" (the existing `InstallPlan` semantics) with "run all" (prereq semantics). Different models; don't overload.
- **Orchestrate prereqs CLI-side in `so-you-say`** — keeps consent/sequencing outside `sniff` where every future consumer would reinvent it. Sniff owns the install lifecycle.

## Data Model

`sniff/lib/src/programs/types.rs`:

```rust
pub enum PrereqProbe {
    /// Presence check via the dynamic-linker search path.
    /// Linux: ldconfig -p | grep <name>
    /// macOS: search dyld default paths (/usr/local/lib, /opt/homebrew/lib, /usr/lib, DYLD_LIBRARY_PATH)
    SharedLibrary(&'static str),    // e.g. "libportaudio.so.2"
    /// Presence check via PATH.
    Binary(&'static str),           // e.g. "ffmpeg"
}

pub struct SystemPrerequisite {
    /// User-facing name shown in the plan rendering.
    pub name: &'static str,          // e.g. "PortAudio"
    /// Probe used to decide whether to skip installation.
    pub probe: PrereqProbe,
    /// OS-specific install methods. Resolved with the same bucket/OS logic
    /// already used by `build_install_plan`; exactly one winner per host.
    pub methods: &'static [InstallationMethod],
}
```

`sniff/lib/src/programs/schema.rs`:

```rust
pub struct ProgramInfo {
    // ...existing fields...
    /// System-level prerequisites required before the tool-level install runs.
    /// Empty slice for most programs. Treated as required — all must resolve
    /// on the host for the `FullInstallPlan` to be successful.
    pub system_prerequisites: &'static [SystemPrerequisite],
}
```

All existing `ProgramInfo` declarations gain `system_prerequisites: &[]`. The `const fn standard` / `with_repo` constructors default the field to `&[]` so only the three consumers that need prereqs mention it explicitly.

## Plan Building

`sniff/lib/src/programs/install_plan.rs`:

```rust
pub struct FullInstallPlan {
    pub program: String,
    pub website: &'static str,
    pub successful: bool,
    /// Prereqs whose probe failed (not already satisfied).
    /// Already-satisfied prereqs are omitted — nothing to do.
    pub prerequisites: Vec<PrereqPlan>,
    /// Per-program install plan for the main tool.
    pub main: InstallPlan,
}

pub struct PrereqPlan {
    pub name: &'static str,
    pub probe: PrereqProbe,
    /// Mirrors InstallPlan: one option has `choose = true` when viable.
    pub successful: bool,
    pub options: Vec<InstallPlanOption>,
}

pub fn build_full_install_plan<P: ProgramMetadata>(
    program: &P,
    host: &HostCapabilities,
) -> FullInstallPlan;
```

### Key behaviors

- **Probe at plan-build time.** `build_full_install_plan` runs each prereq's `PrereqProbe` against the host. Satisfied prereqs are omitted from `prerequisites`. This keeps the announcement short and avoids re-prompting sudo on repeat installs.
- **Prereq selection reuses the bucket algorithm.** The method-selection core of `build_install_plan` (bucket computation, OS gating, manager availability, sudo checks) is extracted into a helper `select_method(methods: &[InstallationMethod], os_availability: &[OsType], host: &HostCapabilities) -> Vec<InstallPlanOption>` used by both the main plan and each `PrereqPlan`. No duplicated logic.
- **`successful = main.successful && all prereqs successful`.** Any unresolved required prereq makes the whole `FullInstallPlan` unsuccessful. The per-prereq `PrereqPlan.successful` and its `options` carry the same `InstallPlanReason` vocabulary as the main plan, so the UI can explain *which* prereq failed and *why*.
- **Dry-run still probes.** The rendered plan shows what *would* run on this host right now; already-satisfied prereqs don't appear.
- **Backwards compatibility.** `build_install_plan` stays. Existing callers are untouched.

### Probe implementation

- `PrereqProbe::Binary(name)`: delegate to the `which` crate (already a workspace dependency).
- `PrereqProbe::SharedLibrary(name)`:
  - Linux: shell out to `ldconfig -p` and grep for `name`. Cheap (cache lookup, no disk scan).
  - macOS: search dyld default paths in order: `DYLD_LIBRARY_PATH` entries, `/usr/local/lib`, `/opt/homebrew/lib`, `/opt/local/lib`, `/usr/lib`. First match wins.
  - Windows: report satisfied. On Windows, shared libraries are bundled with the Python wheel that needs them (sounddevice ships `portaudio.dll` in its wheel), so a system-wide `SharedLibrary` probe has no meaningful target. Reporting satisfied means the prereq is silently skipped on Windows — which is correct for the v1 consumers. Documented in the `PrereqProbe` doc comment.

Probe functions live in a new module `sniff/lib/src/programs/prereq_probe.rs` with unit tests that mock `ldconfig` output and probe paths.

## Interview Runner

`sniff/lib/src/programs/install_interview.rs`:

```rust
pub struct FullInstallInterviewInput {
    pub program: String,
    pub website: &'static str,
    pub plan: FullInstallPlan,
}

pub enum FullInstallInterviewOutcome {
    Installed,
    DryRun,
    AbortedByUser,
    PrereqUnavailable { name: &'static str, reason: String },
    PrereqFailed { name: &'static str, attempted: Vec<InstallationMethod> },
    MainFailed { attempted: Vec<InstallationMethod> },
    NotInstallable,
}

pub fn run_full_install_interview<D: InstallInterviewDelegate>(
    input: &FullInstallInterviewInput,
    options: &InstallInterviewOptions,
    delegate: &mut D,
) -> Result<FullInstallInterviewOutcome, SniffInstallationError>;
```

### Sequence

1. **Unsuccessful plan.** If `!plan.successful`, emit one terminal `Status::Error` listing the failing prereq(s) or missing main method. Return `NotInstallable` or `PrereqUnavailable` as appropriate.
2. **Combined announcement.** Emit one `Announcement` event whose body renders:
   ```
   Installing kokoro-tts:
     Prereq: PortAudio — sudo apt install libportaudio2
     Main:   uv tool install kokoro-tts  (bootstraps uv via astral.sh)
   ```
   When the main method or any prereq method is `UvWithInstall` or `RemoteBash`, the announcement body also includes the consent-warning text that `build_remote_script_warning` produces today.
3. **Single consent.** Emit one `ConfirmProceed` via a new delegate method:
   ```rust
   fn confirm_full_plan(&mut self, prose: &str) -> Result<bool, SniffInstallationError>;
   ```
   A new delegate method (not reuse of `confirm_remote_script`) because the semantics differ — this prompt is "proceed with the whole plan", and it subsumes remote-script consent when relevant.
4. **Run prereqs.** Execute each `PrereqPlan` via the existing per-method execution path (`execute_install_captured`, same `CapturedOutput`/`Status` events). Stop on the first prereq failure and return `PrereqFailed { name, attempted }`. No retry loop for prereqs — if apt failed, there's nothing to fall back to; the user has to fix the root cause manually.
5. **Run main.** Hand off to the existing interview logic for the main `InstallPlan`, honoring its retry flow for lower-priority alternatives.
6. **Success.** Emit one `Status::Success` on full completion. Return `Installed` or `DryRun` per `InstallOptions`.

### Preserving today's behavior

When `plan.prerequisites` is empty and only the main plan runs, the combined announcement degenerates to a single-item list. The UX is effectively identical to today's for tools with no prereqs. Existing `run_install_interview` callers are untouched — the new function lives alongside it.

## CLI Integration (`so-you-say`)

`biscuit-speaks/cli/src/main.rs::install_client_via_interview` (line 1073) is the only caller being migrated:

- `build_install_plan(&client, &host)` → `build_full_install_plan(&client, &host)`.
- `run_install_interview(&input, &options, &mut ui)` → `run_full_install_interview(&input, &options, &mut ui)`.
- `InstallInterviewInput` → `FullInstallInterviewInput`.
- Outcome `match` gains three new arms: `PrereqUnavailable`, `PrereqFailed`, `MainFailed`. Each prints a clear error using the existing styling (`"✗".red().bold()` + cause text + manual install hint for unavailable prereqs).
- `Installed` / `DryRun` arms collapse (no need to carry `method` for the combined flow).

`SoYouSayInstallUi` (`biscuit-speaks/cli/src/install_ui.rs`) gains one method:

```rust
fn confirm_full_plan(&mut self, prose: &str) -> Result<bool, SniffInstallationError> {
    // Render the announcement Prose (already passed in), then inquire::Confirm.
}
```

The existing `confirm_remote_script` stays (used by the legacy single-plan runner for other callers that haven't migrated).

### Dry-run output

`so-you-say install kokoro --dry-run` on Linux without PortAudio renders:

```
Would install kokoro-tts (https://github.com/nazdridoy/kokoro-tts):
  sudo apt install libportaudio2
  uv tool install kokoro-tts
```

On a host that already has `libportaudio2`, the prereq line is absent.

### Flags and help text

No new CLI flags. `so-you-say install --help` gets a one-line note that some providers install system libraries automatically when available.

## Concrete Prereq Declarations

`sniff/lib/src/programs/enums/metadata.rs`:

```rust
pub(crate) static PORTAUDIO_PREREQ: SystemPrerequisite = SystemPrerequisite {
    name: "PortAudio",
    probe: PrereqProbe::SharedLibrary("libportaudio.so.2"),
    methods: &[
        InstallationMethod::Apt("libportaudio2"),
        InstallationMethod::Dnf("portaudio"),
        InstallationMethod::Pacman("portaudio"),
        InstallationMethod::Brew("portaudio"),
        // Windows: sounddevice wheels bundle portaudio — no prereq needed.
    ],
};

pub(crate) static FFMPEG_PREREQ: SystemPrerequisite = SystemPrerequisite {
    name: "FFmpeg",
    probe: PrereqProbe::Binary("ffmpeg"),
    methods: &[
        InstallationMethod::Apt("ffmpeg"),
        InstallationMethod::Dnf("ffmpeg"),
        InstallationMethod::Pacman("ffmpeg"),
        InstallationMethod::Brew("ffmpeg"),
        InstallationMethod::Chocolatey("ffmpeg"),
        InstallationMethod::Scoop("ffmpeg"),
        InstallationMethod::Winget("Gyan.FFmpeg"),
    ],
};

pub(crate) static ESPEAK_NG_PREREQ: SystemPrerequisite = SystemPrerequisite {
    name: "eSpeak NG",
    probe: PrereqProbe::Binary("espeak-ng"),
    methods: &[
        InstallationMethod::Apt("espeak-ng"),
        InstallationMethod::Dnf("espeak-ng"),
        InstallationMethod::Pacman("espeak-ng"),
        InstallationMethod::Brew("espeak-ng"),
        InstallationMethod::Chocolatey("espeak-ng"),
        InstallationMethod::Winget("espeak-ng.espeak-ng"),
    ],
};
```

Wired to each `ProgramInfo`:

| Program | `system_prerequisites` |
|---------|------------------------|
| `KokoroTts` | `&[PORTAUDIO_PREREQ]` |
| `Piper` | `&[ESPEAK_NG_PREREQ]` |
| `Echogarden` | `&[FFMPEG_PREREQ]` |
| everything else | `&[]` (default) |

Implementation-time verification (not design-time): package names per distro will be confirmed against official packaging (especially Arch's `portaudio` vs `portaudio-bin` and the winget IDs) before shipping. The design leaves room for adjustment without structural change.

## Error Types

One new variant in `sniff::error::SniffInstallationError`:

```rust
pub enum SniffInstallationError {
    // ...existing...
    PrerequisiteUnavailable {
        program: String,
        prereq: &'static str,
        reason: String,
    },
}
```

Produced when `build_full_install_plan` resolves a prereq that has no eligible install method on the host (wrong OS, missing PM, missing sudo). Surfaced by the interview runner as the `PrereqUnavailable` outcome.

## Testing Plan

### Unit tests — `sniff/lib/src/programs/install_plan.rs`

- `build_full_install_plan` on Linux+apt host, kokoro with missing `libportaudio.so.2` → main + one prereq, both successful.
- Same host but `libportaudio.so.2` present → main only, no prereq listed.
- `build_full_install_plan` on Linux host without sudo → `successful = false`, prereq has `RequiresSudoNotAvailable` reason.
- `build_full_install_plan` on Windows + kokoro → main successful, no prereq (probe reports `SharedLibrary` satisfied on Windows; see probe implementation notes).
- `build_full_install_plan` with echogarden on a host that has `ffmpeg` on PATH → main only.
- `select_method` helper called with empty method slice → empty options vec.

### Unit tests — `sniff/lib/src/programs/prereq_probe.rs` (new)

- `Binary` probe: mock PATH with / without the binary.
- `SharedLibrary` probe on Linux: mock `ldconfig -p` output with / without the lib.
- `SharedLibrary` probe on macOS: mock directory contents in dyld paths.
- `SharedLibrary` probe on Windows: returns satisfied unconditionally (documented platform behavior).

### Unit tests — `sniff/lib/src/programs/install_interview.rs`

- `run_full_install_interview` with empty prereqs → single-step behavior equivalent to existing runner.
- With one unsatisfied prereq + successful main, user accepts → combined announcement emitted, prereq runs, main runs, outcome `Installed`.
- With one unsatisfied prereq, prereq install returns failure → outcome `PrereqFailed { name, attempted }`, main never runs.
- User rejects `confirm_full_plan` → outcome `AbortedByUser`, nothing runs.
- `plan.successful = false` → outcome `PrereqUnavailable` or `NotInstallable`; single `Status::Error` event; no further events.
- Dry-run: prereq + main both emit announcement bodies containing their would-be commands; neither executes.

### CLI tests — `biscuit-speaks/cli/tests/cli_test.rs`

- Extend existing `so-you-say install --dry-run kokoro` assertion to check that the rendered command list on a Linux fake-host includes `sudo apt install libportaudio2` before the main uv command.
- Assert that `so-you-say install --dry-run kokoro` on a macOS fake-host renders only the main command (PortAudio is available via bundled wheel or brew-already-installed).

### Scope of integration tests

No new end-to-end tests that actually install system packages (they'd require privileged test environments). The unit-test coverage via mocked host capabilities is sufficient.

## Rollout

Pure code change; no migration, no breaking API change for consumers that don't opt in. Existing `build_install_plan` / `run_install_interview` retained. When the main install surface migrates away from them in a follow-up (out of scope here), they become removable.

## Open Questions

None identified at design time. Implementation will verify distro package names against upstream packaging pages before the metadata declarations land.
