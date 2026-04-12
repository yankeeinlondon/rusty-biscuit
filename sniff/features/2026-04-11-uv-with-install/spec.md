# UvWithInstall — Make Python-Installable Tools Actually Install

## Summary

Add `InstallationMethod::UvWithInstall(pkg)` to Sniff's program installer
*and* make plain `Pip(_)` and `Uv(_)` methods selectable by adding them to
the install-plan bucket order. The combined change closes the install gap
for programs whose only methods are `Pip(_)` or `Uv(_)` (aider, goose,
kimi-cli, conan, poetry, and the TTS cluster) on every host where Sniff
runs.

The selection rule encoded in the new buckets is:

| Host state | Action |
|---|---|
| `uv` is installed | Use `uv` (declared `Uv(_)` if present, else synthesized `UvWithInstall(_)` which skips its bootstrap step at execute time) |
| `pip` is installed, `uv` is not | Use `pip` (declared `Pip(_)`) — don't install new software when existing software works |
| neither is installed | Bootstrap `uv` via `astral.sh/uv/install.sh` (or `install.ps1` on Windows), then `uv tool install <pkg>` |

`UvWithInstall(_)` is auto-appended at plan-build time whenever a program
declares `Pip(_)` or `Uv(_)` and does not already declare an explicit
`UvWithInstall(_)`. Program metadata in `metadata.rs` is **not** edited.

## Motivation

Today, a program whose only install methods are `Pip(_)` or `Uv(_)` cannot
be installed by `sniff <category> install <name>` at all — *regardless* of
whether pip or uv is present on the host. The plan builder's `bucket_for`
helper routes both `Pip(_)` and `Uv(_)` to `Bucket::Other`, which is
deliberately excluded from `bucket_order` (the install-improvements
feature left them out, presumably to avoid blindly picking unverified
language package managers).

The gap affects real tools, including:

- AI CLIs: `aider`, `goose`, `kimi-cli`
- Build tools: `conan`, `poetry`
- TTS cluster: `gtts`, `coqui-tts`, `sherpa-onnx`, `kokoro-tts`, `mimic`,
  `mimic3`, `piper`

`uv` itself is a single static Rust binary that does *not* require a
pre-existing Python interpreter. The astral installer fetches a
self-contained binary, and `uv tool install` will use a uv-managed Python
runtime if none is on the host. This means we do not need to gate the
fallback on Python being on `PATH`; bash (Unix) or PowerShell (Windows)
is the only environmental precondition.

## Goals

1. Every program with a `Pip(_)` or `Uv(_)` install method gains a
   runnable install path on every supported host.
2. When `uv` is already installed, prefer `uv` over `pip` — this is the
   user's stated preference and aligns with the modern Python tooling
   direction.
3. When `pip` is installed and `uv` is not, prefer the already-installed
   `pip` over bootstrapping a new copy of `uv` — don't install new
   software when existing software works.
4. When neither is installed, bootstrap `uv` from astral and proceed.
5. The fallback runs only when no higher-priority method is runnable —
   it never preempts a working brew / apt / cargo / etc.
6. The fallback reuses the existing remote-bash consent flow; no new CLI
   flags, no new error variants the user has to learn.
7. The fallback works end-to-end on macOS, Linux (including WSL), and
   native Windows in the first delivery.
8. Program metadata (`installation_methods` slices in `metadata.rs`) does
   **not** need to be touched for existing programs to benefit.

## Non-Goals

- Detecting or managing the Python interpreter itself. `uv` handles its
  own Python; Sniff does not need to know whether `python3` is on `PATH`.
- Introducing a `PipWithInstall` variant. `uv tool install` subsumes
  `pip install --user` for CLI tools.
- Bootstrapping `uv` through any mechanism other than the astral
  installer. Alternatives (`pip install uv`, `brew install uv`,
  `cargo install uv`) are already covered by uv's own `UV_INSTALL`
  metadata entry and would be selected through the normal plan if
  their prerequisites were present.
- Modifying the user's shell profile, `PATH`, or environment. The astral
  script writes `uv` to `~/.local/bin/`; Sniff calls that absolute path
  for the follow-up install and leaves environment configuration to the
  user.
- Fixing `Bucket::Other` for the *other* language package managers
  (`Pnpm` unverified case, `Bun`, `Yarn`, `Poetry`, `Cpan`, `Cpanm`,
  `LuaRocks`, `VcPkg`, `Conan`, `Nuget`, `Hex`, `GoModules`, `Composer`,
  `SwiftPm`). They stay in `Bucket::Other`. This spec narrowly addresses
  the Python-installable tool gap.
- Auto-synthesizing a `Pip(pkg)` from a `Uv(pkg)` declaration. We do not
  assume that the PyPI package name for a uv-declared tool is identical
  to what `pip install` would resolve, so a program declaring only
  `Uv("kimi-cli")` on a pip-only host will fall through to the
  bootstrap path rather than guess.

## Design Decisions

| # | Decision | Notes |
|---|---|---|
| 1 | Only `UvWithInstall` — no `PipWithInstall`. | `uv tool install` subsumes `pip install --user` for CLI tools. |
| 2 | No Python detection. | `uv` doesn't require a pre-existing Python interpreter. The astral installer is self-contained, and `uv tool install` manages Python automatically. No `HostCapabilities` change, no cache schema bump. |
| 3 | Bootstrap is hard-coded to the astral installer. | Unix: `sh -c "curl -LsSf https://astral.sh/uv/install.sh \| sh"`. Windows: `powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 \| iex"`. |
| 4 | Consent reuses `RemoteBashConsentRequired` + `approve_remote_bash`. | No new error variant, no new CLI flag. The `url` field reports the astral installer URL. Consent is demanded at plan level even if the bootstrap step turns out to be skipped at execute time — the plan asks for worst-case approval. |
| 5 | New bucket `PipUvDirect` placed after `SudoNpm` and before `UvBootstrap`. | Holds plain `Pip(_)` and `Uv(_)` facts so they can finally be selected. |
| 6 | `Pip(_)` eligibility is conditional on `uv` being absent. | This is what enforces "uv wins over pip when both installed." Within the same `PipUvDirect` bucket, at most one of `Pip` and `Uv` is ever eligible, so within-bucket ordering is moot. |
| 7 | `UvWithInstall(_)` lives in its own dead-last `UvBootstrap` bucket. | Eligibility = `has_bash` (Unix) or `is_windows`. Always available as a final fallback. |
| 8 | Auto-append at plan-build time. | `build_install_plan` synthesizes a tail `UvWithInstall(pkg)` whenever declared methods include `Pip(_)` or `Uv(_)` and no explicit `UvWithInstall(_)` is already present. The package name is taken from the first `Uv(_)` if present, else from the first `Pip(_)`. |
| 9 | `UvWithInstall` execution skips its bootstrap step when `uv` is already on `PATH`. | Decided at execute time via a fresh `which uv` probe — not from cached host capabilities — so a user who installed uv between plan-build and execute does not trigger a redundant bootstrap. |

## Architecture

### New enum variant

In `sniff/lib/src/programs/types.rs`:

```rust
pub enum InstallationMethod {
    // ...existing variants...

    /// Install `pkg` via `uv tool install`, bootstrapping uv from
    /// astral.sh/uv/install.sh (or install.ps1 on Windows) first if
    /// uv is not already on PATH. Runnable whenever bash (Unix) or
    /// PowerShell (Windows) is available — no Python on the host is
    /// required because the astral installer is self-contained and
    /// `uv tool install` manages Python on its own. Uses the
    /// RemoteBash consent flow via the existing `approve_remote_bash`
    /// option; consent is demanded even when the bootstrap step will
    /// be skipped at execute time.
    UvWithInstall(&'static str),
}
```

The variant carries the PyPI package name (same shape as `Pip(_)` /
`Uv(_)`). The astral installer URL is hard-coded inside the installer
module — not stored on the variant — so there is one authoritative
source per platform.

`InstallationMethod::package_name`, `manager_name`, `manager_binary`,
`is_os_package_manager`, and the existing tests that exhaustively
enumerate variants all need a one-line addition for `UvWithInstall`.

### HostCapabilities

**No changes.** No new field, no new detection, no cache schema bump.
The `python_on_path` idea from an earlier draft of this spec was
removed because `uv` does not require Python on the host.

### Plan-builder changes

In `sniff/lib/src/programs/install_plan.rs`:

#### 1. New buckets

```rust
enum Bucket {
    DefaultOsPm,
    VerifiedPnpm,
    NpmNoSudo,
    AltOsPm,
    RemoteBash,
    Cargo,
    SudoNpm,
    PipUvDirect,   // new — holds plain Pip(_) and Uv(_)
    UvBootstrap,   // new — holds UvWithInstall(_)
    Other,         // pre-existing catch-all
}
```

#### 2. Updated `bucket_order`

```rust
fn bucket_order() -> [Bucket; 9] {
    [
        Bucket::DefaultOsPm,
        Bucket::VerifiedPnpm,
        Bucket::NpmNoSudo,
        Bucket::AltOsPm,
        Bucket::RemoteBash,
        Bucket::Cargo,
        Bucket::SudoNpm,
        Bucket::PipUvDirect,
        Bucket::UvBootstrap,
    ]
}
```

`Bucket::Other` continues to be excluded from `bucket_order`. Only
`Pip` and `Uv` are pulled out of it; the other language PMs stay in
`Other` (out of scope, see Non-Goals).

#### 3. Routing in `bucket_for`

```rust
match &fact.kind {
    // ...existing arms...

    InstallationMethod::Pip(_) | InstallationMethod::Uv(_) => Bucket::PipUvDirect,
    InstallationMethod::UvWithInstall(_) => Bucket::UvBootstrap,

    _ => Bucket::Other,
}
```

The OS-package-manager and Pnpm/Npm arms stay above this so default
OS PMs continue to win when applicable.

#### 4. Auto-append synthesis

Before deriving facts, `build_install_plan` post-processes the declared
methods:

```rust
fn synthesize_uv_bootstrap(
    declared: &[InstallationMethod],
) -> Option<InstallationMethod> {
    if declared.iter().any(|m| matches!(m, InstallationMethod::UvWithInstall(_))) {
        return None;
    }
    let pkg = declared
        .iter()
        .find_map(|m| match m {
            InstallationMethod::Uv(p) => Some(*p),
            _ => None,
        })
        .or_else(|| declared.iter().find_map(|m| match m {
            InstallationMethod::Pip(p) => Some(*p),
            _ => None,
        }))?;
    Some(InstallationMethod::UvWithInstall(pkg))
}
```

The synthesized variant is chained onto the declared slice before
fact derivation, so it flows through every downstream reason path
identically to a declared method.

#### 5. Eligibility in `derive_method_fact`

| Method | `eligible_without_priority` |
|---|---|
| `Uv(_)` | `host.lang_pkg_mgrs.is_installed(LanguagePackageManager::Uv)` |
| `Pip(_)` | `host.lang_pkg_mgrs.is_installed(LanguagePackageManager::Pip) && !host.lang_pkg_mgrs.is_installed(LanguagePackageManager::Uv)` |
| `UvWithInstall(_)` | Unix: `host.has_bash`. Native Windows (`os_type == Windows`): `true` (PowerShell is always present on supported Windows). |

The `&& !uv_installed` clause on `Pip` is the critical enforcement of
"uv wins over pip when both are installed." Because `Pip` and `Uv`
share the `PipUvDirect` bucket, at most one is ever eligible at a
time, so within-bucket order does not matter and the bucket selection
is unambiguous.

`UvWithInstall` deliberately does *not* gate on whether `uv` is
already installed. It is runnable in both states, and execution
decides at the last minute whether the bootstrap step fires.

#### 6. New blocking reasons

Two new variants of `InstallPlanReason`:

- `UvPreferredOverPip` — `Pip(_)` is blocked because `uv` is also
  installed and is the preferred Python tool installer. Reason text:
  `"uv is installed; uv is preferred over pip for Python tools"`.
- `BashNotAvailable` — `UvWithInstall(_)` cannot run on a Unix host
  without bash (the astral installer requires it). Reason text:
  `"bash is not available; cannot run the astral uv installer"`.

`PipUvDirect` selected reasons:

- `Uv(_)` selected: `"chosen — uv tool install (uv already on host)"`
- `Pip(_)` selected: `"chosen — pip install (pip already on host; uv absent)"`

`UvBootstrap` selected reason:

- `"chosen — uv tool install (bootstraps uv via astral.sh if absent; requires remote-script consent)"`

### Execution

In `sniff/lib/src/programs/installer.rs`:

#### 1. New branch in `execute_install` and `execute_versioned_install`

`UvWithInstall(pkg)` gets a dedicated branch. The existing `Pip(_)`
and `Uv(_)` branches are unchanged — they continue to run
`pip install <pkg>` and `uv tool install <pkg>` as today.

#### 2. Conditional two-step execution

```
Step 1 — bootstrap uv (only if `which uv` fails right now)
  Unix:    sh -c "curl -LsSf https://astral.sh/uv/install.sh | sh"
  Windows: powershell -ExecutionPolicy ByPass -c \
           "irm https://astral.sh/uv/install.ps1 | iex"

Step 2 — install the target program (always runs)
  Unversioned: <uv_path> tool install <pkg>
  Versioned:   <uv_path> tool install <pkg>@<version>
```

The step-1 skip is driven by a fresh `which::which("uv")` lookup
inside `execute_install`, not by the cached `HostCapabilities`. A
user who installed uv between plan-build and execute-time does not
trigger a redundant bootstrap.

#### 3. uv binary resolution

After bootstrap (or at the start of execution if bootstrap was
skipped), resolve the uv binary in this order:

1. Bare `uv` on `PATH` (handles "user already had uv" and "user
   added `~/.local/bin` to PATH").
2. `~/.local/bin/uv` (Unix) or `%USERPROFILE%\.local\bin\uv.exe`
   (Windows) — the astral installer's documented default location.
3. If both miss, return `SniffInstallationError::InstallationError`
   with a clear message that uv could not be located after bootstrap.

This ordering covers the case where uv was installed system-wide via
some other route (brew/apt) and lives at `/usr/local/bin/uv` or
similar.

#### 4. Consent check inside `InstallPlan::execute`

Extend the existing `RemoteBash` branch:

```rust
let needs_remote_consent = matches!(
    chosen.kind,
    InstallationMethod::RemoteBash(_) | InstallationMethod::UvWithInstall(_)
);
if needs_remote_consent && !opts.approve_remote_bash && !opts.dry_run {
    let url = match &chosen.kind {
        InstallationMethod::RemoteBash(u) => u.to_string(),
        InstallationMethod::UvWithInstall(_) => astral_installer_url().to_string(),
        _ => unreachable!(),
    };
    return Err(SniffInstallationError::RemoteBashConsentRequired {
        pkg: self.program.clone(),
        url,
    });
}
```

`astral_installer_url()` is a small platform-dispatched helper that
returns the Unix or Windows URL.

#### 5. `InstallResult.command` rendering

For dry-run and verbose CLI output, the `command` string contains
whichever steps will actually run, decided by the same runtime
`which uv` probe used by execution:

- When `uv` is absent (bootstrap will run):
  ```
  curl -LsSf 'https://astral.sh/uv/install.sh' | sh
  ~/.local/bin/uv tool install 'aider-chat'
  ```
- When `uv` is already on `PATH` (bootstrap will be skipped):
  ```
  uv tool install 'aider-chat'
  ```

No new fields on `InstallResult` — keeps the API surface unchanged.

#### 6. Versioned installs

Versioned installs work via `uv tool install pkg@version`. The
pre-existing `Pip(_)` versioned install (`pip install pkg==version`,
already in `installer.rs:438`) continues to work for the
"pip-installed-uv-absent" case via the new `PipUvDirect` selection.

### What stays untouched

- Program metadata in `sniff/lib/src/programs/enums/metadata.rs`. No
  `installation_methods` slice is edited. Auto-append handles
  existing and future Python-installable tools uniformly.
- CLI subcommands (`sniff <category> install …`). The consent gate
  already surfaces `RemoteBashConsentRequired`; users see the
  astral installer URL in the prompt and approve (or not) using the
  same flow they would for any other remote-bash install.
- `ProgramDetector` trait methods. All reachable via the existing
  `install_plan` / `install` / `install_version` entry points.
- Other language package managers (`Pnpm`, `Bun`, `Yarn`, `Poetry`,
  `Cpan`, `Cpanm`, `LuaRocks`, `VcPkg`, `Conan` lib, `Nuget`, `Hex`,
  `GoModules`, `Composer`, `SwiftPm`) and their `Bucket::Other`
  routing. Out of scope.

## Testing

### Unit tests — selection (`install_plan.rs`)

**Auto-append behavior:**

- `auto_append_synthesizes_uv_with_install_from_pip_only`: program
  declares `[Pip("conan")]` → plan options include
  `UvWithInstall("conan")` at the tail.
- `auto_append_synthesizes_uv_with_install_from_uv_only`: program
  declares `[Uv("kimi-cli")]` → synthesized
  `UvWithInstall("kimi-cli")`.
- `auto_append_prefers_uv_package_name_over_pip`: program declares
  both `[Pip("aider"), Uv("aider-chat")]` → synthesized
  `UvWithInstall("aider-chat")`.
- `auto_append_skipped_when_no_python_method`: program with only
  `[Brew("vim")]` → no synthesized variant.
- `auto_append_skipped_when_explicit_uv_with_install_present`: program
  declaring `[Pip("foo"), UvWithInstall("foo")]` → synthesis is a
  no-op.

**Selection rule (`uv > pip > bootstrap`):**

- `uv_wins_when_uv_installed_and_uv_declared`: program declares
  `[Pip("aider"), Uv("aider-chat")]`; host has uv installed → chosen
  is `Uv("aider-chat")` (in `PipUvDirect` bucket).
- `uv_wins_when_uv_installed_and_only_pip_declared`: program declares
  `[Pip("conan")]`; host has uv installed → chosen is synthesized
  `UvWithInstall("conan")` (`PipUvDirect` is empty of eligible facts;
  `UvBootstrap` wins). The `Pip` fact is blocked with reason
  `UvPreferredOverPip`.
- `pip_wins_when_pip_installed_uv_absent`: program declares
  `[Pip("conan")]`; host has pip installed but not uv → chosen is
  `Pip("conan")` (in `PipUvDirect` bucket). The synthesized
  `UvWithInstall` is present with reason `LowerPriorityAlternative`.
- `pip_wins_when_both_declared_and_pip_installed_uv_absent`: program
  declares `[Pip("aider"), Uv("aider-chat")]`; host has pip but not
  uv → chosen is `Pip("aider")`. The `Uv("aider-chat")` fact is
  blocked with reason `ManagerNotInstalled`.
- `bootstrap_wins_when_neither_installed`: program declares
  `[Pip("conan")]`; host has neither pip nor uv → chosen is
  synthesized `UvWithInstall("conan")` in `UvBootstrap`.
- `bootstrap_wins_when_only_uv_declared_and_only_pip_installed`:
  program declares `[Uv("kimi-cli")]`; host has pip but not uv →
  chosen is synthesized `UvWithInstall("kimi-cli")`. (Documents the
  edge case in row 5 of the scenarios table — we do not assume
  `pip install kimi-cli` would work without an explicit declaration.)

**Higher-priority methods still win:**

- `brew_wins_over_pip_uv_direct_on_macos`: program declares
  `[Brew("poetry"), Pip("poetry")]`; macOS host with brew → chosen
  is `Brew`; the `PipUvDirect` and `UvBootstrap` candidates are
  marked `LowerPriorityAlternative`.
- `cargo_wins_over_uv_bootstrap`: program declares
  `[Cargo("foo"), Pip("foo")]`; host has cargo but not pip/uv →
  chosen is `Cargo`. (Verifies `UvBootstrap` is strictly after
  `Cargo` in `bucket_order`.)
- `uv_bootstrap_is_truly_last`: program declares only `[Pip("foo")]`;
  host has runnable methods in every earlier bucket via siblings —
  `UvWithInstall` does not preempt anything in earlier buckets.

**Eligibility edges:**

- `uv_with_install_blocked_on_unix_without_bash`: `host.has_bash ==
  false` on Unix → `UvWithInstall` is ineligible with reason
  `BashNotAvailable`.
- `uv_with_install_eligible_on_native_windows`: fabricated Windows
  host → `UvWithInstall` is eligible regardless of `has_bash`.

### Unit tests — execution (`installer.rs`)

- `uv_with_install_dry_run_without_uv_renders_two_step_command`:
  dry-run on a host where `which uv` fails → `InstallResult.command`
  contains both the astral curl-sh line and the `uv tool install`
  line.
- `uv_with_install_dry_run_with_uv_renders_single_step_command`:
  dry-run on a host where `which uv` succeeds → `InstallResult.
  command` contains *only* the `uv tool install` line; no curl-sh.
- `uv_with_install_without_consent_returns_consent_error`: non-dry
  run with `approve_remote_bash: false` returns
  `RemoteBashConsentRequired`, and the `url` field is the astral
  installer URL (not the PyPI package). This must hold even when
  uv is already installed and the bootstrap would be skipped — the
  consent gate is plan-level, not execution-level.
- `uv_with_install_versioned_renders_at_version`: dry-run versioned
  install for `"aider-chat" @ "0.50.0"` → command contains
  `uv tool install 'aider-chat@0.50.0'`.
- `uv_with_install_windows_renders_powershell_command`: with a
  fabricated Windows host where `which uv` fails, dry-run command
  contains
  `powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"`.
- `uv_with_install_resolves_uv_path_via_path_first`: when `which uv`
  succeeds at a non-default location (e.g. `/opt/homebrew/bin/uv`),
  the install command uses that path, not `~/.local/bin/uv`.

### Integration test (`sniff/lib/tests/uv_with_install_plan.rs`)

A new test file using the fake `ProgramInfo` pattern from
`install_plan.rs::selection_tests`. It fabricates several
`HostCapabilities` permutations (uv installed; pip installed
without uv; neither installed; uv installed on Windows; bash
missing on Linux) and verifies end-to-end plan shape for a program
declaring only `[Pip("aider-chat")]`. This is the cross-cutting
guard that protects the full table of scenarios from regressing.

### Not tested

- Actually running the astral installer. It performs network I/O and
  persistent host mutation; `dry_run` verifies the command shape and
  that is sufficient for automated coverage.
- Live Windows PowerShell execution. The command-shape test covers
  rendering; live Windows behavior is validated manually.
- Live `uv tool install` against PyPI. Same reason — the command
  shape is verified, the network call is not.

## Open Questions

None. All design decisions were settled during the brainstorming
pass that produced this spec.

## Risks

1. **Astral default install path changes.** If astral moves `uv`
   out of `~/.local/bin/`, the post-bootstrap absolute-path
   resolution misses. *Mitigation:* the resolution order tries
   bare `uv` on `PATH` first, then the documented default location;
   a clear error fires only if both miss. Worst case is a one-line
   constant update in `installer.rs`.
2. **Consent reuse conflation.** A user who previously approved a
   remote-bash install for (say) `rustup` gets auto-opted into
   approving the astral uv bootstrap too. This was accepted for
   API simplicity. If users complain, we split the consent flag in
   a follow-up — the call sites are all inside
   `InstallPlan::execute`.
3. **Consent demanded even when bootstrap will be skipped.** Because
   the bootstrap decision is made at execute time but consent is
   plan-level, users on a host with uv already installed will see
   "approve remote script?" for an install that turns out to skip
   the remote script entirely. A more refined UX would do a
   plan-build-time `which uv` probe and skip the consent prompt
   accordingly, but that conflates plan semantics with host
   liveness checks and was deferred.
4. **Pulling `Pip` and `Uv` out of `Bucket::Other` may surprise the
   author of the install-improvements feature**, who deliberately
   left them out. This spec's narrow argument is that Python
   tooling is the largest concrete gap and that the new
   `PipUvDirect` bucket plus the `uv > pip` eligibility rule
   delivers the right semantics for this slice without committing
   to any particular policy for the other language PMs.
5. **Edge case: program declares only `Uv("kimi-cli")` and host has
   only pip.** We do not auto-synthesize a `Pip("kimi-cli")` from
   the `Uv` declaration; instead, `UvWithInstall` bootstraps uv.
   This is documented in row 5 of the scenarios table and tested
   by `bootstrap_wins_when_only_uv_declared_and_only_pip_installed`.
   If users hit this case in practice and want pip to win, the
   fix is a metadata edit on the affected program (add an explicit
   `Pip(_)` declaration) — not a behavior change in this spec.

## Delivery Order (informal)

This is a spec, not a plan, but the natural sequence is:

1. `InstallationMethod::UvWithInstall` variant + extensions to
   `package_name` / `manager_name` / `manager_binary` /
   exhaustive enum tests. No behavior change yet.
2. `Bucket::PipUvDirect` and `Bucket::UvBootstrap` additions to
   the `Bucket` enum + `bucket_order` update + routing in
   `bucket_for`. Existing tests should still pass.
3. Eligibility rules for `Pip(_)`, `Uv(_)`, and `UvWithInstall(_)`
   in `derive_method_fact`. Auto-append synthesis in
   `build_install_plan`. New blocking reasons. Selection tests.
4. Two-step execution in `installer.rs` with the runtime `which
   uv` probe and the binary resolution fallback. Dry-run rendering
   tests. Execution-shape tests.
5. Consent-flow extension in `InstallPlan::execute`. Consent test.
6. Integration test (`sniff/lib/tests/uv_with_install_plan.rs`).
7. Manual smoke test on macOS (with and without uv installed) and
   on a Linux container with only pip installed.

The detailed task breakdown belongs in a follow-up plan doc (see
`plan.md` sibling files in other feature folders for the convention).

[astral.sh/uv/install.sh]: https://astral.sh/uv/install.sh
