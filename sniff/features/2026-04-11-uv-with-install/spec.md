# UvWithInstall — Last-Resort Bootstrap for Python-Installable Tools

## Summary

Add a new `InstallationMethod::UvWithInstall(pkg)` variant to Sniff's program
installer. On a host where Python is on `PATH` but `uv` is not, this variant
is a last-resort fallback that bootstraps `uv` via the official
[astral.sh/uv/install.sh] script (or its PowerShell equivalent on Windows),
then runs `uv tool install <pkg>` to install the target program. It reuses
the existing `RemoteBashConsentRequired` consent gate so users who have not
approved remote install scripts are never surprised.

The variant is **auto-appended** by the plan builder: any program whose
metadata declares a `Pip(_)` or `Uv(_)` installation method automatically
gains a synthesized `UvWithInstall(_)` option at the tail of its install plan.
No program-metadata edits are required.

## Motivation

Today, a program whose only install methods are `Pip(_)` or `Uv(_)` cannot
be installed by `sniff <category> install <name>` at all — *regardless* of
whether pip or uv is present on the host. This is because the plan
builder's `bucket_for` helper routes both `Pip(_)` and `Uv(_)` to
`Bucket::Other`, which is deliberately excluded from `bucket_order`. The
deliberate exclusion (inherited from the install-improvements feature)
reflects a reluctance to pick unverified language package managers blindly,
and this spec does not relitigate that decision.

The gap affects real tools, including:

- AI CLIs: `aider`, `goose`, `kimi-cli`
- Build tools: `conan`, `poetry`
- TTS cluster: `gtts`, `coqui-tts`, `sherpa-onnx`, `kokoro-tts`, `mimic`,
  `mimic3`, `piper`

On a fresh Linux container with Python available but no language package
managers, every one of these currently resolves to `ManagerNotInstalled`.
On a host with pip *and* uv installed, they still fail with no chosen
method because of the `Bucket::Other` routing. `UvWithInstall` closes both
gaps with a single variant that is eligible whenever Python is present and
bash (or PowerShell) is available, and that bootstraps `uv` only if it is
not already installed.

## Goals

1. Every program with a `Pip(_)` or `Uv(_)` install method gains a
   runnable install path whenever Python is on the host, regardless of
   whether `uv` is already installed.
2. The fallback runs only when no higher-priority method is runnable — it
   must never preempt a working brew / apt / cargo / etc.
3. The fallback reuses the existing remote-bash consent flow; no new CLI
   flags, no new error variants the user has to learn.
4. The fallback works end-to-end on macOS and Linux (including WSL) in the
   first delivery; Windows support ships in the same change via PowerShell.
5. Program metadata (`installation_methods` slices in `metadata.rs`) does
   **not** need to be touched for existing programs to benefit.
6. When `uv` is already on the host, execution skips the bootstrap step
   and runs `uv tool install` directly; the bootstrap only fires when
   `uv` is absent.

## Non-Goals

- Detecting or managing the Python interpreter itself. Sniff will only check
  whether `python3`, `python`, or `py` is present on `PATH`; it will not
  install Python, probe version, or care about virtualenvs.
- Introducing a `PipWithInstall` variant. `uv` supersedes `pip` for tool
  installs (`uv tool install` targets the same PyPI packages and installs
  to `~/.local/bin` without requiring a venv), so a second variant adds
  complexity without covering any case `UvWithInstall` does not.
- Bootstrapping uv through any mechanism other than the astral installer.
  Alternatives (`pip install uv`, `brew install uv`, `cargo install uv`)
  are all covered by uv's existing `UV_INSTALL` metadata entry and would
  be selected through the normal plan if their prerequisites were present.
- Modifying the user's shell profile, `PATH`, or environment. The astral
  script writes `uv` to `~/.local/bin/`; Sniff calls that absolute path
  for the follow-up install and leaves environment configuration to the
  user.
- Version probing or version floors for Python. Q2 answer: PATH lookup
  only. False positives on the Windows Store Python stub are accepted as
  an execution-time failure rather than a selection-time concern.

## Design Decisions

The following decisions were reached during brainstorming and are binding
for the implementation:

| # | Decision | Notes |
|---|---|---|
| 1 | Only `UvWithInstall` — no `PipWithInstall`. | `uv tool install` subsumes `pip install --user` for CLI tools. |
| 2 | Python gate is PATH-only. | `which::which("python3").is_ok() \|\| which::which("python").is_ok() \|\| which::which("py").is_ok()`. No version probe, no interpreter spawn. |
| 3 | Bootstrap is hard-coded to the astral installer. | Unix: `sh -c "curl -LsSf https://astral.sh/uv/install.sh \| sh"`. Windows: `powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 \| iex"`. |
| 4 | Consent reuses `RemoteBashConsentRequired` + `approve_remote_bash`. | No new error variant, no new CLI flag. The `url` field reports the astral installer URL (not the PyPI package name). Consent is demanded even if the bootstrap step turns out to be skipped at execution time — the *plan* is built against a snapshot of host state and asks for worst-case consent. |
| 5 | Bucket placement is dead-last. | `UvBootstrap` is the final bucket in `bucket_order`, after `SudoNpm`. Only fires when nothing else is runnable. |
| 6 | Auto-append at plan-build time. | `build_install_plan` synthesizes a tail `UvWithInstall(pkg)` whenever declared methods include `Pip(_)` or `Uv(_)` and no explicit `UvWithInstall(_)` is already present. |
| 7 | `UvWithInstall` is a **superset of `Uv`**, not a sibling. | Eligibility does NOT require uv to be absent. When uv is already on the host, execution skips the bootstrap step and runs `uv tool install` directly. This is the minimal fix for the `Bucket::Other` gap noted in *Motivation*, without expanding scope to re-bucket plain `Pip(_)` / `Uv(_)` methods. |

## Architecture

### New enum variant

In `sniff/lib/src/programs/types.rs`:

```rust
pub enum InstallationMethod {
    // ...existing variants...

    /// Install `pkg` via `uv tool install`, bootstrapping uv from
    /// astral.sh/uv/install.sh (or install.ps1 on Windows) first if
    /// uv is not already on PATH. Runnable whenever Python is on
    /// PATH and bash (Unix) or PowerShell (Windows) is available.
    /// Uses the RemoteBash consent flow via the existing
    /// `approve_remote_bash` option — consent is demanded even when
    /// the bootstrap step will be skipped at execution time.
    UvWithInstall(&'static str),
}
```

The variant carries the PyPI package name (same shape as `Pip(_)` and
`Uv(_)`). The astral installer URL is hard-coded inside the installer
module — not stored on the variant — so there is one authoritative source
per platform.

### HostCapabilities delta

One new field in `sniff/lib/src/programs/host_capability.rs`:

```rust
pub struct HostCapabilities {
    // ...existing fields...
    pub python_on_path: bool,
}
```

Detection in `HostCapabilities::detect()`:

```rust
fn detect_python_on_path() -> bool {
    which::which("python3").is_ok()
        || which::which("python").is_ok()
        || which::which("py").is_ok()
}
```

`Default::default()` sets the field to `false`.

### Cache schema version bump

`CACHE_SCHEMA_VERSION` goes from `1` to `2`. Existing
`~/.sniff-programs.json` files will fail the version check inside
`load_host_capabilities_from` and be silently replaced on next detection.
No migration is needed.

### Plan-builder changes

In `sniff/lib/src/programs/install_plan.rs`:

1. **New bucket.** `Bucket::UvBootstrap` added to the `Bucket` enum and
   placed last in `bucket_order`:

   ```
   DefaultOsPm → VerifiedPnpm → NpmNoSudo → AltOsPm
     → RemoteBash → Cargo → SudoNpm → UvBootstrap
   ```

2. **Auto-append.** Before deriving facts, post-process the declared
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

3. **Routing.** `bucket_for(fact, host)` routes
   `InstallationMethod::UvWithInstall(_)` to `Bucket::UvBootstrap`.

4. **Eligibility.** `derive_method_fact` for `UvWithInstall` computes:

   ```
   eligible_without_priority =
        host.python_on_path
     && platform_bootstrap_available(host)
   ```

   where `platform_bootstrap_available` is:

   - **Unix** (`host.os_type != OsType::Windows`, which includes macOS,
     Linux, and WSL — sniff already detects WSL as Linux): requires
     `host.has_bash == true`. Bash is only needed on the bootstrap
     path; when uv is already installed, the bootstrap step is
     skipped and bash is not actually required. We still gate on
     bash at plan-build time because the plan is built against a
     pessimistic view of the host: if the user approves and runs
     the plan, and uv has been uninstalled in the interim, we want
     the plan to fail cleanly at selection rather than mid-execute.
   - **Native Windows** (`host.os_type == OsType::Windows`): always
     `true`. PowerShell is present on every supported Windows version,
     so no capability flag is needed.

   Notably, the eligibility rule does **not** depend on whether `uv`
   is currently installed. That's the core of the superset-of-`Uv`
   decision: the variant is runnable in both states, and the
   execution path decides at the last minute whether the bootstrap
   step fires.

5. **New blocking reason.** One new variant of `InstallPlanReason`:

   - `PythonNotOnPath` — Python interpreter missing from PATH.

   Surfaced through `blocking_reason_for` / `explain_blocking_reason`.
   Reason text examples:

   - Selected (uv absent):
     `"chosen — uv tool install (bootstraps uv via astral.sh; requires remote-script consent)"`
   - Selected (uv already present):
     `"chosen — uv tool install (uv already installed; bootstrap skipped at execute time; requires remote-script consent)"`
   - Python missing:
     `"python interpreter not on PATH — cannot use uv to install"`

   The "requires remote-script consent" suffix appears on the selected
   reason regardless of host state. This is intentional: the plan is a
   snapshot, and the user must approve the worst-case side effect even
   if the actual execution ends up skipping the bootstrap. An earlier
   iteration of this spec added an `UvAlreadyInstalledPreferred` reason;
   it was removed because `UvWithInstall` is now itself the direct path.

### Execution

In `sniff/lib/src/programs/installer.rs`:

1. `execute_install` and `execute_versioned_install` gain a branch for
   `UvWithInstall(pkg)`.

2. Consent check happens inside `InstallPlan::execute`, which already
   branches on `RemoteBash`. Extend the branch:

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

3. Execution sequence. Step 1 is conditional on host state **at
   execution time** (not plan-build time — re-check `which uv` just
   before running):

   ```
   Step 1 — bootstrap uv (only if uv is not already on PATH)
     Unix:    sh -c "curl -LsSf https://astral.sh/uv/install.sh | sh"
     Windows: powershell -ExecutionPolicy ByPass -c \
              "irm https://astral.sh/uv/install.ps1 | iex"

   Step 2 — install the target program with uv (always runs)
     Unversioned: <uv_path> tool install <pkg>
     Versioned:   <uv_path> tool install <pkg>@<version>
   ```

   The step-1 skip is driven by a fresh PATH lookup inside
   `execute_install`, not by the cached `HostCapabilities`, so a
   user who installed uv between plan-build and execute-time does
   not trigger a redundant bootstrap.

4. `uv` binary resolution after bootstrap:

   - Unix: `~/.local/bin/uv`
   - Windows: `%USERPROFILE%\.local\bin\uv.exe`

   These paths match the astral installer defaults as of April 2026.
   If the default path does not exist after a successful bootstrap
   execution, fall back to bare `uv` on `PATH`; if that also fails,
   return `SniffInstallationError::InstallationError` with a clear
   message that uv could not be located. This fallback covers future
   astral-default changes without requiring a Sniff release.

5. `InstallResult.command` rendering. For dry-run and verbose CLI
   output, the `command` string contains whichever steps will
   actually run:

   - When uv is absent (bootstrap required):
     ```
     curl -LsSf 'https://astral.sh/uv/install.sh' | sh
     ~/.local/bin/uv tool install 'aider-chat'
     ```
   - When uv is already on PATH (bootstrap skipped):
     ```
     uv tool install 'aider-chat'
     ```

   The dry-run renderer inspects the same runtime `which uv` probe
   as the execute path, so the rendered command is honest about
   what *would* run right now. No new fields on `InstallResult` —
   keeps the API surface unchanged.

6. Versioned installs *are* supported via `uv tool install pkg@version`.
   Unlike plain `RemoteBash` (which must error on versioned installs),
   `UvWithInstall` gracefully supports `install_version` end-to-end. This
   is a win for the affected programs: on hosts without pip/uv today,
   `install_version("aider-chat", "0.50.0")` returns an error; after this
   change, it works via the bootstrap path.

### What stays untouched

- Program metadata in `sniff/lib/src/programs/enums/metadata.rs`. No
  `installation_methods` slice is edited. Auto-append handles all
  existing and future Python-installable tools.
- CLI subcommands (`sniff <category> install …`). The consent gate
  already surfaces `RemoteBashConsentRequired`; users will see the
  astral installer URL in the prompt and approve (or not) using the
  same flow they would for any other remote-bash install.
- `ProgramDetector` trait methods. All reachable via the existing
  `install_plan` / `install` / `install_version` entry points.

## Testing

### Unit tests — selection (`install_plan.rs`)

- `uv_with_install_synthesized_when_pip_declared`: program declares
  only `Pip("aider-chat")`; Python on PATH; `has_bash == true` → plan
  options include a synthetic `UvWithInstall("aider-chat")` and it
  is chosen.
- `uv_with_install_synthesized_when_uv_declared`: program declares
  only `Uv("kimi-cli")` → same, package name taken from the `Uv`
  variant.
- `uv_with_install_prefers_uv_package_name_over_pip`: program
  declares both `Pip("aider")` and `Uv("aider-chat")` → synthesized
  variant uses `"aider-chat"` (the `Uv` name wins).
- `uv_with_install_blocked_when_python_not_on_path`: Python absent →
  reason is `PythonNotOnPath` and the option is not chosen.
- `uv_with_install_chosen_even_when_uv_already_installed`: uv present
  in `host.lang_pkg_mgrs`, program declares only `Pip("conan")` →
  `UvWithInstall("conan")` is synthesized and chosen. This verifies
  the superset-of-`Uv` behavior and guards against regressing into
  the `UvAlreadyInstalledPreferred` trap.
- `uv_with_install_not_chosen_when_higher_bucket_runnable`: program
  has `Brew("poetry")` and `Pip("poetry")`; host is macOS with brew →
  chosen is `Brew`; the synthesized `UvWithInstall` is present with
  reason `LowerPriorityAlternative`.
- `uv_with_install_is_last_bucket`: program has only `Pip("foo")`
  plus the synthesized `UvWithInstall`; host also has a runnable
  `Cargo` path for a sibling program — verify `UvWithInstall` does
  not preempt `Cargo` and lands in the final bucket.
- `uv_with_install_not_synthesized_when_no_python_method`: program
  with only `Brew("vim")` → no synthesized variant.
- `uv_with_install_not_double_synthesized`: program whose metadata
  already declares an explicit `UvWithInstall("foo")` → synthesis is
  a no-op.
- `uv_with_install_blocked_on_unix_without_bash`: Python on PATH,
  `has_bash == false` on Unix → option not chosen with a blocking
  reason describing the missing bash. (Even though bash is only
  strictly needed on the bootstrap path, we gate at plan-build time
  for worst-case planning as documented in the Eligibility section.)

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

### Unit tests — host capability (`host_capability.rs`)

- `default_python_on_path_is_false`: `HostCapabilities::default()
  .python_on_path == false`.
- `cache_schema_version_bump_invalidates_old`: write a cache envelope
  with `schema_version: 1`; `load_host_capabilities_from` returns
  `None`.

### Integration test (`sniff/lib/tests/uv_with_install_plan.rs`)

A new test file that uses the fake `ProgramInfo` pattern established by
`install_plan.rs`'s existing `selection_tests` module. It fabricates a
`HostCapabilities` with Python on PATH, `has_bash = true`, no installed
language package managers, and verifies end-to-end plan shape for a
program declaring only `Pip("aider-chat")`.

### Not tested

- Actually running the astral installer. It performs network I/O and
  persistent host mutation; `dry_run` verifies the command shape and
  that is sufficient for automated coverage.
- Live Windows PowerShell execution. The command-shape test covers
  rendering; live Windows behavior is validated manually.

## Open Questions

None. All design decisions were settled during the brainstorming pass
that produced this spec.

## Risks

1. **Astral default install path changes.** If astral moves `uv` out
   of `~/.local/bin/`, the post-bootstrap path resolution breaks.
   *Mitigation:* fallback to bare `uv` on `PATH` after a failed
   absolute-path lookup; a clear error if both fail. Worst case is a
   one-line constant update in `installer.rs`.
2. **Windows Store Python stub.** On Windows, `python` on `PATH` may
   be the Microsoft Store redirect that exits with a prompt rather
   than running code. Our PATH-only gate reports `python_on_path =
   true`, the plan picks `UvWithInstall`, and the user may hit the
   stub at execution time. Because the astral Windows installer does
   not actually require an existing Python interpreter to install uv
   itself (the installer bundles a Python runtime), this may turn
   out to be a non-issue in practice. If users hit it, we upgrade
   the gate to a version probe in a follow-up.
3. **Consent reuse conflation.** A user who previously approved a
   remote-bash install for (say) `rustup` gets auto-opted into
   approving the astral uv bootstrap too. This trade-off was
   accepted for API simplicity. If users complain, we split the
   consent flag in a follow-up — the call sites are all inside
   `InstallPlan::execute`.
4. **Consent is demanded even when the bootstrap would be skipped.**
   Because `UvWithInstall` is a superset of `Uv` and the bootstrap
   decision is made at execution time, the plan-level consent gate
   asks for worst-case approval even if `uv` is already installed.
   On a host with uv present, users will see "approve remote
   script?" for an install that turns out to skip the remote
   script entirely. This is mildly annoying but truthful — the
   *plan* really does declare a conditional remote-script side
   effect. A more refined UX would probe uv at plan-build time and
   skip the consent prompt accordingly, but that conflates plan
   semantics with host liveness checks and was deferred.

## Delivery Order (informal)

This is a spec, not a plan, but the natural sequence is:

1. `HostCapabilities::python_on_path` + cache schema bump + its tests.
2. `InstallationMethod::UvWithInstall` variant + its `package_name` /
   `manager_name` / `manager_binary` extensions + compile-time
   exhaustiveness tests.
3. `Bucket::UvBootstrap` + auto-append in `build_install_plan` +
   eligibility rule + selection tests.
4. Two-step execution in `installer.rs` + dry-run rendering +
   execution tests.
5. Consent-flow extension in `InstallPlan::execute` + consent test.
6. Integration test.
7. Manual smoke test on macOS and a Linux container with only Python
   present.

The detailed task breakdown belongs in a follow-up plan doc (see
`plan.md` sibling files in other feature folders for the convention).

[astral.sh/uv/install.sh]: https://astral.sh/uv/install.sh
