---
prompt: |-
    Research the Rust `rightkit-sandbox` crate to help a developer decide whether
    and how to use it in a sandbox launcher intended to run across operating systems.

    The starting claim is that `rightkit-sandbox` provides sandboxing across multiple
    operating systems. Verify the exact package, its relationship to any larger Rightkit
    project, and which functionality is available in its published release.

    Focus on the published crate's public API, enforcement guarantees,
    compatibility behavior, and practical integration. Explain native OS
    concepts where needed without turning this into a general sandbox survey.

    Evidence and scope:
    - State the research date and exact stable crate version examined.
    - Verify identity through published package metadata and its linked
      repository. If the named crate cannot be identified, report the search
      and uncertainty; do not invent APIs or silently research a substitute.
    - Use primary sources: version-specific API documentation, release notes,
      published source, repository history, and official documentation for each underlying
      operating-system mechanism.
      Cite claims near the text they support.
    - Use neighboring sandboxing research documents for context and research leads.
      Independently verify relevant claims; do not treat those reports as primary
      evidence.
    - Distinguish released functionality from unreleased changes, proposals,
      documentation claims, and your own recommendations. If sources conflict,
      explain the uncertainty. Do not infer security guarantees from API names,
      a successful launch, or the absence of an error.

    Cover the following topics:

    1. Identity, releases, and maintenance
       - Identify the package, repository, maintainers, license, and any parent
         project. Confirm whether the package is published and usable as a
         dependency, and identify relevant yanked or prerelease versions.
       - Verify the latest stable version and release date. Summarize releases
         from the past 24 months and older milestones needed to explain today's
         API. Use dates to describe cadence rather than impressions.
       - Explain API stability, breaking changes, migration considerations,
         minimum supported Rust version, and important native dependencies.
         Do not assume major releases exist or equate pre-1.0 with unreliability.
       - Assess maintenance using observable evidence; label judgments as such.

    2. Public API and enforcement model
       - Explain the workflow from policy configuration to enforcement and
         workload execution, using the crate's actual public types and methods.
       - Explain defaults, allow/deny composition, enforcement scope, inheritance,
         reversibility, privilege requirements, and the trust boundary between
         supervisor, helpers, and workload where applicable.
       - Describe filesystem, networking, process/IPC, and resource controls
         actually exposed. Distinguish supported controls from absent ones.
       - List Cargo feature flags, defaults, and their effects on functionality,
         dependencies, and platform support. Explicitly say if none exist.

    3. Platform integration, compatibility, and failure behavior
       - Identify the released backend and native mechanism for each OS.
         Separate crate functionality from features supplied only by a parent
         application, workspace package, external binary, or unreleased code.
       - Provide a per-OS matrix of controls, minimum OS/runtime requirements,
         privileges, helper binaries, and signing or packaging requirements.
       - Explain whether the crate can be integrated independently, including
         configuration, runtime, process-management, or ecosystem coupling.
       - Trace a shared policy through backend selection, translation, and
         enforcement. Explain defaults, path semantics, unsupported requests,
         and whether any fallback can weaken the requested policy.
       - Determine how a caller inspects effective capabilities and refuses
         execution when required guarantees are unavailable on a target OS.
       - Distinguish implemented, experimental, planned, and unsupported
         targets. Treat native Windows and WSL2 as different environments.
       - Distinguish successful compilation from usable runtime enforcement.
         Explain capability detection and partial or unavailable support using
         the crate's actual errors, status values, or documented limitations.
       - Show how to require specific guarantees and refuse to launch when
         they cannot be enforced. If the API cannot do this, state that gap.

    4. Practical uses, limitations, and pitfalls
       - Describe supported uses and who they protect against. Distinguish
         accidental access reduction from confinement of malicious workloads.
       - Evaluate read-only inputs, a writable workspace, no networking, and
         access only to approved destinations. State exactly which parts the
         crate can enforce; distinguish paths from objects and network ports
         from destination identity where relevant.
       - Examine enforcement ordering, multithreading, inherited descriptors or
         handles, existing sockets, descendants, executable/runtime access,
         filesystem aliases, and IPC paths that affect the claimed guarantees.
       - Classify material gaps as underlying OS limitations, native capabilities
         missing from the crate, integration/deployment constraints, or needs
         for another isolation mechanism. Explain consequences and mitigations.
       - Include documented defects or advisories that materially affect the
         examined version, with their status and applicability. Do not equate
         an absence of published advisories with evidence of security.

    5. Rust examples and verification
       - Include complete, focused examples demonstrating a shared read-only-input and
         writable-workspace policy across the implemented backends,
         supported network restrictions, and capability/enforcement checks.
         If a requested control is unavailable, explain that instead of
         inventing an implementation.
       - Include Cargo configuration pinned to the examined release, required
         features, target configuration, and any setup or helper requirements.
       - Show meaningful error handling and fail-closed behavior for each
         example's stated requirements. Clearly identify any guarantee that
         needs caller-side validation or a separate mechanism.
       - Explain expected allowed and denied operations and remaining gaps.
         For multiple backends, identify what each example demonstrates on
         each platform instead of implying that compilation proves parity.
       - Compile-check when tooling permits. Label examples as compiled,
         executed, or unverified, naming the environment and version tested.
         Compilation and cross-compilation do not establish enforcement.
       - Describe isolated child-process tests for both allowed and denied
         operations, unavailable capabilities, and launch failures without
         restricting the test runner or leaving persistent security state.

    6. Alternatives and complementary tools
       - Compare a small, relevant selection of other cross-platform Rust sandbox
         wrappers, direct per-OS integration, and external sandbox launchers.
       - Distinguish competing bindings, higher-level wrappers, complementary
         mechanisms, and stronger isolation boundaries. Compare enforcement
         coverage, platform support, maintenance, and integration cost.
       - Explain when this crate is sufficient, when it needs another layer,
         and when a different approach better meets the stated requirements.

    Presentation:
    - Begin with a concise suitability assessment and the main caveats.
    - Use well-formed CommonMark and capability/comparison tables where helpful.
      Use Mermaid only when a diagram clarifies policy or launch flow.
    - End with authoritative resource links and unresolved questions that
      materially affect adoption.
hash: 8c75044ab9ecf5a3-1e2fb2a222a0923b
last_updated: 2026-11-03
---

# rightkit-sandbox: adoption research

**Assessment:** `rightkit-sandbox` is a real, independently consumable Rust crate with released macOS, Linux, and native Windows launch backends. It is a plausible starting point for a supervised tool launcher, provided the launcher admits policies against each backend's actual capabilities. It does **not** supply an equivalent, minimal filesystem and network boundary everywhere. Important exceptions include Linux ignoring `deny_read`, older Linux kernels lacking truncate protection, macOS retaining Unix-socket access and implicit system grants, and Windows modifying host ACLs and retaining AppContainer profiles. It is unsuitable by itself for a portable “only these objects, no communication, bounded resources” guarantee. These conclusions come from the published implementation, rather than the crate's broad fail-closed description. [Published source][source], [policy][policy], [Linux backend][linux], [Seatbelt backend][seatbelt], [Windows backend][windows].

**Research date:** November 3, 2026. **Exact stable release examined:** **0.6.2**, published **November 1, 2026, 12:02:30 UTC**. Registry metadata, the downloaded crates.io archive, and version-specific documentation/source links are the release baseline. Repository `main` is used only for explicitly identified maintenance or unreleased observations. [Registry metadata][registry], [release archive][archive].

## Identity, release history, and maintenance

The Cargo package is `rightkit-sandbox`; its Rust import is `rightkit_sandbox`. Its current metadata links to [`bogusyogi/rightkit`](https://github.com/bogusyogi/rightkit), whose README identifies RightKit as the shared Rust, npm, and Swift libraries for Right Suite apps. The crate lives at `crates/rightkit-sandbox`, with confinement code ported from CodeRight. It is neither the whole RightKit project nor a dependency on a running Right Suite application. Older package metadata, such as 0.2.4, linked to `bogusyogi/claude`; that historical URL should not be used to identify the current source home. [0.6.2 manifest][manifest], [RightKit README](https://github.com/bogusyogi/rightkit#readme), [0.2.4 manifest](https://docs.rs/crate/rightkit-sandbox/0.2.4/source/Cargo.toml.orig).

The registry owner is GitHub user **`bogusyogi`**, displayed as **Adrian D'souza**. This establishes publishing ownership, not an independently verified security team or review process. The published license is **MIT OR Apache-2.0**, and both license texts ship. [Registry owners](https://crates.io/api/v1/crates/rightkit-sandbox/owners), [manifest][manifest], [archive contents][source].

The registry lists 17 versions, all stable version strings, none yanked as of the research date. There are no published prereleases or pre-October milestones in that record. The entire publication history falls within the requested 24-month window, November 3, 2024–November 3, 2026. Dates below are UTC publication dates, which sometimes differ from changelog dates. [Registry version history][registry].

| Published date | Versions                 | Changes relevant to adoption                                                                                                                                                                                                                       |
| -------------- | ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2026-10-07     | 0.1.0                    | First published package.                                                                                                                                                                                                                           |
| 2026-10-08     | 0.2.0–0.2.4; 0.3.0–0.3.1 | Windows ancestor-ACL leases are documented in the early releases; 0.3.0 adds bounded output and `SandboxOutput::truncated`.                                                                                                                        |
| 2026-10-09     | 0.3.2–0.3.3              | Patch publications; the current changelog does not give an individual migration narrative for each.                                                                                                                                                |
| 2026-10-10     | 0.3.4                    | Introduces default-on `pty` feature and changes output-drain/reap ordering.                                                                                                                                                                        |
| 2026-10-14     | 0.3.5; 0.4.0             | Descendant-lifetime fixes, cross-session ancestor-lease tracking, profile cleanup API; dependency rename from `rightkit-process` to `rightkit-child-process`.                                                                                      |
| 2026-10-18     | 0.5.0                    | Adds caller-owned command confinement, policy knobs, deny-read API, reusable Windows profiles, interpreter pins, and piped stdin. Linux ABI 2/3 and seccomp architecture hardening change enforcement behavior. Changelog heading says October 17. |
| 2026-10-23     | 0.6.0; 0.6.1             | 0.6.0 refuses ineffective Windows deny-read policies; changes `with_grant`'s error type. 0.6.1 changes the specialized macOS browser-fixture Mach allowance.                                                                                       |
| 2026-11-01     | 0.6.2                    | Publishes optional `execution` contracts and local effect performer.                                                                                                                                                                               |

Release descriptions above are from the [packaged changelog][changelog] and [early published README](https://docs.rs/crate/rightkit-sandbox/0.2.4/source/README.md), reconciled with registry dates. The packaged changelog still labels **0.6.2 “unreleased,” 0.6.1 “unpublished staging,” and 0.3.0 “staged, unpublished.”** Those labels are stale: their archives are published. Conversely, repository `main` declares **0.6.4** and replaces `portable-pty` with `rightkit-pty`; neither 0.6.3 nor 0.6.4 appears in the registry record examined. Do not assign that PTY change to 0.6.2. [Current manifest, moving reference](https://github.com/bogusyogi/rightkit/blob/main/crates/rightkit-sandbox/Cargo.toml), [registry][registry].

There is no 1.x compatibility promise to rely on. Relevant concrete migrations are:

- 0.3.0 adds `SandboxOutput::truncated`, breaking complete output struct literals.
- 0.4.0 changes `terminate_tree_bounded` to return `rightkit_child_process::ReapOutcome`; update imports/dependencies that named the old package.
- 0.5.0 adds public policy fields and changes Linux behavior. Prefer builders or `..Default::default()` over exhaustive policy literals. Retest tools against the seccomp filter rather than assuming a source-compatible update preserves runtime behavior.
- 0.6.0 changes `AppContainerProfile::with_grant` to `Result<Self, WithGrantError>`; a failure returns the original profile in the error. Existing Windows deny-read configurations now fail with `Unsupported` rather than appearing to work. This is a security containment change, not implemented deny support. [Changelog][changelog], [policy][policy].

The manifest declares **Rust 1.98.0**, edition 2021. This is a declared minimum, not a minimum-version test performed here. Core dependencies are `rightkit-child-process ^0.5.7`, Unix `libc ^0.2`, and Windows `windows ^0.62`. The normalized registry manifest makes the RightKit dependency resolvable from crates.io despite the original workspace's sibling `path`. No RightKit checkout, broker, Tauri runtime, JSON configuration file, or async runtime is required for `Sandbox::spawn`. Windows binds system DLLs including userenv, advapi32, ole32, and kernel32. macOS uses system Seatbelt APIs; PTY confinement additionally uses `/usr/bin/sandbox-exec`. [Original and normalized manifests][source], [Windows source][windows], [Seatbelt source][seatbelt].

**Maintenance judgment:** publication is active but the public history is short: 17 releases from October 7 to November 1, followed by sandbox-directory commits on November 3. There is observable attention to concrete security and lifecycle failures, but publication volume and repository activity do not demonstrate correctness or external audit. The changelog repeatedly labels changes untested on particular native hosts, and its staging labels drift from the registry. Treat it as useful change evidence, not a certification. Pre-1.0 numbering alone is not evidence of unreliability. [Registry][registry], [sandbox commit history](https://github.com/bogusyogi/rightkit/commits/main/crates/rightkit-sandbox), [changelog][changelog].

## API, defaults, and the launch boundary

The ordinary workflow is:

1. Build a `SandboxPolicy` with `allow_read`, `allow_write`, `allow_exec`, `network`, and optional backend knobs. Build a `SandboxCommand` containing the executable, argument vector, cwd, and additional environment entries. There is no shell interpretation unless the caller explicitly selects a shell.
2. Construct a `ResourceAuthority`, normally shared across all launchers in one host, and `Sandbox::new(authority)`.
3. Call `Sandbox::spawn(&policy, command)` or `spawn_piped`. Validation and resource admission precede workload execution. Windows rejects denies even before ordinary validation and reservation.
4. Retain `SandboxedChild` for ownership of process/leases/reservation. Use `take_stdin`, `take_stdout`, `take_stderr`, `has_exited`, `try_wait`, `wait`, `terminate_tree`, or `terminate_tree_bounded` as appropriate. `wait_with_output_capped(timeout, cap)` supplies a deadline and bounded capture.
5. Inspect `SandboxOutput::{status, timed_out, truncated}`. Its `denial` is a text classification, not proof that confinement was complete. [Policy and command definitions][policy], [launch and output implementation][spawn], [resource authority][resources].

`SandboxPolicy::new()` defaults to `ConfinementMode::Strict`, empty path lists, `NetworkPolicy::Deny`, `exec_any = false`, no deny entries or named capabilities, `LandlockMode::Always`, `SeccompMode::Allowlist`, and `LoopbackPolicy::Blocked`. **Empty path lists do not mean no filesystem authority:** backends have implicit runtime/system allowances, described below. `allow_write` also allows reads; overlapping allow grants compose additively. A broader writable root overrides the practical read-only intent of a nested read grant. There is no portable “read-only exception inside writable parent” API. `deny_read` forbids reads **and writes** on macOS, is omitted on Linux, and causes refusal on Windows. [Policy source][policy], [confinement queries][confine].

`EnvPolicy` normally clears the environment, preserves `PATH`, `LANG`, `LC_ALL`, and `TERM`, then adds its `set` map. Command-specific environment entries win. On Windows the backend additionally installs platform variables, a container-reachable PATH, container-local temp/app-data paths, and Node `--preserve-symlinks --preserve-symlinks-main` options. Even an empty `keep` list is therefore not an empty Windows environment. The launcher remains responsible for excluding credentials, loader injection variables, agent sockets, and untrusted PATH resolution; prefer an absolute executable. [Policy][policy], [Unix environment application][spawn], [Windows environment builder][windows].

`confine_command(&mut std::process::Command, &SandboxPolicy)` is a separate synchronous API for Linux/macOS; a Tokio caller can pass `as_std_mut()`. It installs confinement in `pre_exec` and does **not** apply policy environment, cwd, stdio, process-group ownership, deadlines, or reservations. Windows returns `io::ErrorKind::Unsupported`; AppContainer setup must happen during creation. A preparation error must prevent the subsequent spawn, and `Command::spawn()` can still fail when the child applies the prepared restrictions. Registration order matters for other `pre_exec` hooks. Use `Sandbox::spawn` when its owned-child lifecycle fits. [Implementation][confine].

Windows additionally exposes `named_capability`, but validation permits only `lpacChromeInstallFiles` and `registryRead`; other names are rejected, and these entries have no backend effect off Windows. This is not arbitrary registry policy or a network-capability escape hatch. The archive also ships the `rightkit-sandbox-exec` binary: its Windows CLI accepts `--read-only`, `--writable`, `--cwd`, and `--env`, runs one command with networking denied, and reports setup failures with exit 125. On other targets the binary exits 2 as Windows-only. It is an optional external entry point, not a helper required by `Sandbox::spawn`. [Policy validation][policy], [binary source](https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/bin/rightkit-sandbox-exec.rs).

### Released Cargo features

| Feature     | Default                    | Effect in 0.6.2                                                                                                                                                                                                                           |
| ----------- | -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pty`       | On via `default = ["pty"]` | Adds optional `portable-pty ^0.9` and the top-level PTY session/input-lease API. Policy-constrained PTY launch works only on macOS. Disabling default features removes those items and that dependency.                                   |
| `execution` | Off                        | Adds `execution` module, optional `rightkit-crypto ^0.1.0`, `async-trait`, Serde/JSON, Tokio, Tokio-util, and `rightkit-child-process/tokio`. These are execution contracts and an async performer, not another native isolation backend. |

No other user-selectable features are declared in the [published manifest][manifest]. There is no per-OS feature to enable AppContainer or Landlock; target configuration selects their code.

**Do not confuse `execution` capability data with detected enforcement.** `ExecutionBackendRegistry`, `BackendCapabilities`, `NetworkProfile`, `SandboxProcessProof`, `ProcessLaunchAdapter`, and `CommandSandboxAdapter` are released public contracts. `doctor_backend` checks configuration and workspace existence; it does not probe native confinement. `LocalEffectPerformer` rejects required/profiled sandbox requests and required/profiled/host-filtered network requests with `EffectExecutionError::SandboxMissing`, and nondefault resource limits with `ResourceLimitsMissing`. Otherwise it launches a supervised **unconfined** Tokio command. `NetworkProfile::allow_hosts` and resource-limit fields therefore do not add those controls to `SandboxPolicy`. A host must implement/inject the appropriate adapter and validate its evidence. [0.6.2 execution source][execution].

## Backend and compatibility matrix

“Implemented” below means the release has an enforcement path, not that all OS versions, CPU architectures, filesystems, or host configurations have been proven usable.

| Environment                        | Released mechanisms and controls                                                                                                                                                                               | Requirements, privilege, and deployment                                                                                                                                                                                                                                                                                                                                                        | Important differences                                                                                                                                                                                                                   |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| macOS                              | Seatbelt SBPL via `sandbox_init` in a child `pre_exec`; filesystem read/write/exec, deny-read paths/globs, network choice. Public `seatbelt` module also exposes prepared/custom and browser-fixture profiles. | No root requirement or custom signing entitlement is imposed by the ordinary crate path. Needs a macOS release accepting the generated SBPL and its system profile imports. Crate declares no tested minimum macOS version. Sandboxed PTY requires `/usr/bin/sandbox-exec`.                                                                                                                    | `Strict` imports system `bsd.sb`, then adds its own grants. Metadata reads, many system/application paths and some devices remain available. `Deny` permits Unix sockets. `WriteOnly` is a different, default-allow mode.               |
| Linux x86_64                       | Landlock filesystem rules and, **only when network is denied**, seccomp. Default `Allowlist` or optional `NetworkOnly`.                                                                                        | Landlock enabled and usable, plus seccomp filter support. No root needed: child sets `PR_SET_NO_NEW_PRIVS` before applying restrictions. Landlock starts at mainline Linux 5.13/ABI 1; request ABI ≥3 for truncate protection, normally Linux 6.2+. No helper executable.                                                                                                                      | `deny_read` silently omitted. Handles filesystem rights through ABI 3 only, even on a newer ABI. Network `Allow` installs no seccomp filter. No namespace or cgroup isolation.                                                          |
| Linux aarch64                      | Same Landlock path; `NetworkOnly` seccomp implemented.                                                                                                                                                         | Same runtime checks; explicitly choose `SeccompMode::NetworkOnly` for a denied-network launch.                                                                                                                                                                                                                                                                                                 | Default denied-network `Allowlist` refuses this architecture. Selecting `NetworkOnly` deliberately gives up the broad syscall allowlist; it is not an automatic fallback.                                                               |
| Native Windows                     | AppContainer lowbox token/capability checks, temporary filesystem ACL grants, suspended creation then assignment to a kill-on-close Job Object with active-process limit 64.                                   | AppContainer profile API requires Windows 8/Server 2012 or later at the OS API level; this is not a tested crate minimum or Rust runtime support promise. Normal launches need a usable user profile and permission to edit relevant DACLs. Admin-owned ancestors may require administrator provisioning. No package identity/MSIX or special signing step is required by this implementation. | Read/exec grants share ACL semantics. Existing AppContainer-accessible objects and container storage are outside the explicit path list. `deny_read` and `WriteOnly` refused. Ordinary pipe-based launches work; sandboxed PTYs do not. |
| WSL2, Linux target                 | Linux backend inside the WSL guest; not AppContainer.                                                                                                                                                          | Probe the guest's Landlock ABI and seccomp support, including distro/kernel configuration. Do not infer support from the Windows host's version. Test guest filesystem and mounted Windows volumes separately.                                                                                                                                                                                 | No separate WSL backend or verified WSL2 enforcement result was established here. Launching native `.exe` files through WSL interop is outside the Linux-child guarantee evaluated here; disable/broker interop if it matters.          |
| Other OS/unsupported architectures | No additional confinement backend established in this release.                                                                                                                                                 | Refuse required confinement. Some types may compile without a usable runtime path.                                                                                                                                                                                                                                                                                                             | Do not interpret `cfg(unix)` process utilities or unconfined PTYs as another sandbox target.                                                                                                                                            |

Backend claims are from [Linux][linux], [Seatbelt][seatbelt], [Windows][windows], [PTY implementation][pty], and [published README][readme]. Native requirements and inheritance are explained by [Linux Landlock documentation][landlock-doc], [seccomp documentation][seccomp-doc], [Microsoft profile API][profile-api], and [Job Objects][job-doc]. WSL interop is documented by [Microsoft](https://learn.microsoft.com/en-us/windows/wsl/filesystems#run-windows-tools-from-linux).

Apple's supported **App Sandbox** is entitlement-based and required for Mac App Store distribution; that is a different integration surface from the crate's runtime custom Seatbelt profile. Using this crate does not configure App Sandbox entitlements, Hardened Runtime, TCC permissions, or notarization. The Apple SDK header also marks `sandbox_init` “No longer supported” (deprecated since macOS 10.8) and warns that the header may be removed. This was checked in the installed Apple macOS SDK at `usr/include/sandbox.h`; it is a compatibility risk despite the successful macOS probes below. Custom SBPL is not a stable high-level policy contract. [Apple App Sandbox](https://developer.apple.com/documentation/security/app-sandbox), [released Seatbelt implementation][seatbelt].

### How the same policy becomes different authority

For `allow_read(input) + allow_write(workspace) + network(Deny)`:

- **Linux:** parent prepares a Landlock ruleset and seccomp program; child sets no-new-privileges, restricts itself with that ruleset, then installs seccomp before exec. Rules additionally grant read/execute on `/usr`, `/bin`, `/sbin`, `/lib`, `/lib64`, `/etc`, `/proc`, `/dev`, and `/sys`. Read paths get file/directory reads, exec paths get those plus execute, and writable roots get all handled filesystem rights, **including execute**. Missing/unopenable grant paths are skipped by `add_rule`, granting nothing for them; this can produce a launch that succeeds while a requested input is unusable. Canonicalize/check existing inputs before launch. [Linux implementation][linux].
- **macOS:** parent resolves paths, including deepest existing ancestors for not-yet-created paths, and builds SBPL; child invokes `sandbox_init`. Strict system reads include `/Library`, `/Applications`, `/private/etc`, `/private/var/db`, `/dev`, and the usual executable/runtime trees. System execution includes `/Applications` and `/Library`; metadata is broadly readable. Workspace read/write does **not** itself add the explicit process-exec grant Linux's writable root gets. `bsd.sb` can contribute authority: `deny default` is not proof that imported explicit rules disappeared. Inspect the actual profile and installed imports when minimizing authority. [Seatbelt implementation][seatbelt].
- **Windows:** backend resolves the image and physical paths, creates/reuses a per-grant-set AppContainer identity, leases path/ancestor/image access, creates only the intended inherited I/O handles, starts suspended with `SECURITY_CAPABILITIES`, assigns the job, and resumes. A read grant adds read/execute, a write grant adds write/delete, and directory ACEs inherit. The executable and its required runtime/DLL paths need accessible objects; the implementation leases image access but does not make arbitrary tool installations work automatically. Windows can permit objects already accessible to the container/capability/ALL APPLICATION PACKAGES SIDs. [Windows implementation][windows], [Microsoft isolation model](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation).

These are additive restrictions under the caller's existing credentials and OS controls; they do not grant access forbidden by ordinary discretionary permissions or an outer sandbox. The supervisor and any broker/proxy remain outside and trusted. A workload must never control policy construction, grant approval, or pre-enforcement helper code.

### Detecting and refusing insufficient enforcement

The core crate exposes **`landlock_abi_version() -> Option<u32>`**, **`is_landlock_available() -> bool`**, and **`deny_read_enforced() -> bool`**. The last is a compile-target statement, true only on macOS; it is not a runtime Seatbelt probe. There is no unified effective-rights/status object returned by `Sandbox::spawn`, no portable required-feature set, and no equivalent Windows/macOS preflight proving a particular policy will apply. Landlock availability does not prove seccomp setup or execution will succeed. [Query implementation][confine], [spawn result][spawn].

For a launcher, make the following admission rules explicit:

| Requirement | Refusal rule or gap |
| --- | --- |
| Read-only inputs protected against truncation on Linux | Require `landlock_abi_version().is_some_and(|abi| abi >= 3)`; the crate otherwise accepts ABI 1/2. This checks one important right, not all filesystem mutations. |
| Secret deny subtree inside a broad grant | Before launch, reject a nonempty `policy.deny_read` if `!deny_read_enforced()`. Windows already refuses; **Linux does not**. Prefer separate input trees over granting a whole home directory. |
| Broad syscall filtering on Linux | Require x86_64 and `SeccompMode::Allowlist`. Do not quietly retry with `NetworkOnly`. |
| Every form of networking/IPC forbidden | Core `NetworkPolicy::Deny` is insufficient. Refuse this contract or supply separately verified isolation/broker restrictions. |
| Only approved Internet destinations | No shared destination allowlist in `SandboxPolicy`. Refuse that contract until a separately enforced egress path exists. `NetworkPolicy::Allow` is not the substitute. |
| Policy-constrained PTY | Require macOS and successful helper execution; Linux/Windows return `PtyError::Sandbox(SandboxError::Unsupported(_))`. Never retry with `None` policy: that is unconfined. |
| Required Windows loopback connectivity | After an `Allow` + `LoopbackPolicy::Exempt` launch, inspect `SandboxedChild::loopback_exempt()`; exemption can fail without failing spawn. This detects a narrower connectivity outcome, not a widened boundary. |

`SandboxError` distinguishes `InvalidPolicy(String)`, `Unsupported(String)`, `ResourceDenied(ResourceDenied)`, `Io(io::Error)`, and Windows `AncestorAccess { path, win32_error }`. Unix preparation/application failures can be wrapped as `Io`, including an inner `Unsupported` error kind; matching only the outer `Unsupported` variant misses these. **Any error must prevent workload execution through a fallback path.** `confine_command` reports `io::Error` directly. [Error enum][error], [launch implementation][spawn], [confinement implementation][confine].

A notable opt-in weakening is `LandlockMode::SkipWhenNoPaths`: with no read/write/exec paths it omits filesystem confinement. Coupled with `network(Allow)`, it can leave **neither Landlock nor seccomp** installed. Conversely, unsupported `WriteOnly`, unavailable Landlock with default `Always`, and unsupported seccomp architecture fail rather than automatically switching backends. macOS `profile_for`/`wrap_argv` use a deny-everything profile on deny planning failure; prefer their `try_` variants for an explicit construction error. [Linux source][linux], [Seatbelt source][seatbelt].

## Controls, uses, and material limitations

### Files, networking, processes, and resources

| Desired use                         | What 0.6.2 can do                                                                                                         | Boundary that remains                                                                                                                                                                                                              |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Read-only input, writable workspace | Shared allow policy; OS permission checks on subsequent operations.                                                       | No immutable snapshot, private root filesystem, object-copy isolation, or universal prevention of metadata changes. Inputs must not alias writable objects. Windows ACLs and implicit system grants complicate “only these paths.” |
| No direct IP networking             | macOS outbound rule; Linux syscall filtering; Windows absence of `internetClient`.                                        | Not equivalent to no communication: inherited channels, Unix IPC, available OS services, stdio, and trusted brokers need separate treatment.                                                                                       |
| Approved destination access         | Common API only chooses `Allow`/`Deny`.                                                                                   | No hostname, IP/CIDR, port, DNS, TLS identity, or proxy policy in `SandboxPolicy`. A TCP port is not a server identity.                                                                                                            |
| Child/descendant management         | `SandboxedChild` RAII, bounded termination/capture, Windows job containment.                                              | Unix process-group and ancestry enumeration are not a PID namespace or race-free adversarial containment.                                                                                                                          |
| CPU/memory/disk/process budgets     | Shared admission permits; Windows job cap 64; wall-clock/output bounds when requested.                                    | No shared kernel-enforced CPU, RAM, disk quota, or fork budget. Resource-authority slots count supervisor launches, not every process/thread created inside them.                                                                  |
| Process/IPC isolation               | Native mechanisms add their own constraints; macOS includes self-signal rules; Windows token/job and named-pipe controls. | No common signal, shared-memory, named-object, Unix-socket, device-ioctl, registry, or process-inspection policy. Linux newer Landlock rights/scopes are not exposed by this implementation.                                       |

Sources: [policy][policy], [resource implementation][resources], [spawn/output][spawn], [native implementations][source], [Job Objects][job-doc].

`ResourceProfile::Strict` means **deny every process-backed admission**, not “stronger sandbox policy.” `Practical` permits one aggregate execution group (per-kind ceilings 4 child/2 background); `Expanded` permits 16 aggregate (32/16 per kind). A new authority instance creates a separate pool. Reservations end with their RAII lifetime; they are not OS-wide accounting. [Resource authority][resources].

For destination filtering, use a trusted proxy plus a native boundary that makes direct bypass impossible, and validate name resolution, redirects, CONNECT, private destinations, and protocol scope in that proxy. `HTTP_PROXY` environment variables alone do not enforce anything against a hostile child. The release has a **macOS-only browser-fixture exception**: `seatbelt::BrowserFixturePolicy` and `fixture_profile_for`/`Prepared::for_browser_fixture` constrain a Chrome fixture to an IPv4 loopback proxy at a nonzero port and a disposable profile directory, with an additional loopback debugging-listener allowance. This is released specialized functionality, not common `SandboxPolicy` host filtering or a supplied proxy. Its Mach allowances and inherited launcher TCC authority deserve separate review. [Seatbelt source][seatbelt], [0.6.1 changelog][changelog].

### Ordering, inheritance, and trust

On Linux, applying Landlock and seccomp is irreversible for the restricted thread and inherited by its future children. Existing sibling threads are not retroactively confined. The crate's normal spawn applies restrictions in the new child before exec, leaving the supervisor unrestricted. On macOS the same child-before-exec pattern applies Seatbelt; do not call public `Prepared::apply()` in the application/test-runner process. On Windows the token is established at process creation, with job assignment before resume. Reusable profile grants persist while any profile/child holder needs them. [Linux kernel model][landlock-doc], [seccomp inheritance][seccomp-doc], [crate confinement][confine], [Windows creation][windows].

For multithreaded launchers, preparation in the parent avoids building policy data after `fork`. Caller-added `pre_exec` hooks must themselves be safe after fork; do not add allocations, locks, runtime calls, or untrusted work there. The normal confinement API does not retroactively restrict another already-running process. [Rust `CommandExt::pre_exec`](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html#tymethod.pre_exec), [crate implementation][confine].

Audit **inherited descriptors/handles**, not just path grants. Landlock does not revoke descriptors opened before restriction. In `NetworkOnly`, blocking new non-Unix sockets does not block `read`/`write` or other traffic over a supplied connected socket; even the broad allowlist admits ordinary descriptor I/O. Unix sockets can also deliver capabilities from a broker. macOS retains Unix-socket outbound authority; Windows explicitly limits inherited handles on its ordinary creation path, but intended stdin/stdout/stderr and additional pipe APIs are still channels. The crate's regular Unix launch is not an explicit inventory-and-close-all descriptor sweep. Use close-on-exec hygiene in the supervisor and audit intentionally passed endpoints. [Landlock descriptor caveats][landlock-man], [seccomp source][seccomp], [Seatbelt profile][seatbelt], [Windows handles][windows], [Unix spawn][spawn].

Confinement inheritance is distinct from **tree cleanup**. On Unix, `process_tree` snapshots parent relationships, caps enumeration at 4096 nodes, and signals groups/descendants with bounded waits. Children that change sessions, fork during discovery, or reparent cannot be treated as covered by a mathematically complete tree-kill guarantee just because the method says `terminate_tree`. Parent death and supervisor crash behavior also differ from Windows job-handle closure. For hostile fork-heavy programs, add a cgroup/PID namespace on Linux or another externally owned isolation boundary. [Process-tree implementation][tree], [spawn cleanup][spawn], [Job Object semantics][job-doc].

### Paths are not immutable objects

Canonicalization helps normal symlinks and macOS `/tmp` → `/private/tmp`, but it is a point-in-time observation. Landlock rules attach to kernel filesystem hierarchies; Seatbelt evaluates generated path rules; Windows changes DACLs on resolved objects and ancestors. Hard links, reparse points/junctions, mount aliases, replacement races, already-open objects, and concurrent host writers can defeat a caller's simplistic string-prefix model. No shared API pins a validated input snapshot against all these changes. Stage private input copies, use separate trees/filesystems when needed, and avoid granting roots the workload or another untrusted actor can replace during setup. [Linux implementation][linux], [Seatbelt path resolution][seatbelt], [Windows ancestor resolution][ancestors], [Landlock filesystem semantics][landlock-man].

A writable workspace is itself an output capability: malicious edits to build scripts, Git hooks, or tool/editor configuration can execute later outside confinement. Review outputs or promote them through an explicit trusted step. An input under an implicitly writable device path is not made read-only by `allow_read`; an input under a Windows object already granting the same AppContainer broader access needs an ACL audit. Canonical path separation alone does not exclude hard-linked objects. These are integration recommendations derived from the release's additive grant model, not additional checks the crate performs. [Policy][policy], [Seatbelt][seatbelt], [Windows grants][windows].

### Windows state and known defects

Windows ancestor traversal is more than ordinary Unix directory lookup. Strict one-shot `spawn` leases non-inheriting `FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | SYNCHRONIZE` rights on physical ancestors, which can expose directory names/metadata above the workspace. If the caller cannot edit an admin-owned ancestor, setup returns `AncestorAccess`; a deliberately provisioned ALL APPLICATION PACKAGES ancestor grant is an external administrative choice. **RightKit's managed-root broker bootstrap is outside this crate and not installed by depending on it.** Reusable profiles and pipe-pair spawns use best-effort ancestor leases: inaccessible ancestors may be skipped under specific checked conditions, relying on token bypass-traverse and setting `GIT_CEILING_DIRECTORIES`. This sacrifices upward-listing compatibility, not the requested child data grant. [Released README][readme], [ancestor implementation][ancestors].

ACL leases normally remove added grants when the final holder drops. Crashes can leave grants; profiles are deliberately reused and accumulate until `remove_unused_appcontainer_profiles()` is called. It returns `ProfileSweepReport`, retaining profiles with reasons such as `InUse`, `Busy`, `CouldNotProveUnused`, or `DeleteFailed`. Sweep stale ancestor ACEs **before** deleting unused profiles, because ownership recognition needs the registered profile/SID mapping. This is real-user persistent security state, requiring explicit deployment and recovery design. Loopback exemptions are separately machine state and can remain after a crashed holder. `pin_read_roots` is particularly broad on Windows: its vetted interpreter read/execute grant uses **ALL APPLICATION PACKAGES**, not just this launch's container. [README cleanup and pin sections][readme], [pins source][pins], [Windows lease implementation][windows].

Material documented defects and status:

- **Windows deny-read bypass, issue #887:** introduced deny support in 0.5.0 used DENY ACEs ineffective for the lowbox access-check path. The 0.6.0 containment, present in 0.6.2, rejects **every** deny entry before mutation, including nonexistent paths/globs/outside-grant paths. Old crash-left DENY ACEs are not cleaned by this release. The issue is marked **closed** as of November 3, but the examined release still has **no Windows deny implementation**; closure must not be read as that feature becoming available. [Issue #887](https://github.com/bogusyogi/rightkit/issues/887), [packaged security notes][changelog], [released refusal][windows].
- **Linux compat-syscall/ABI hardening:** 0.5.0 adds seccomp architecture checks and x32 rejection, plus Landlock `REFER`/`TRUNCATE` handling and file-versus-directory rights. Those fixes are present in 0.6.2. ABI 1/2 still lacks truncate enforcement by native design, and the crate accepts it unless the caller raises the minimum. [Changelog][changelog], [Linux][linux], [seccomp][seccomp].
- **Descendant output/lifetime and lease visibility bugs:** 0.3.4/0.3.5 fix reaping before pipe drain, descendants surviving apparently completed output, and ancestor sweeps missing leases in another logon session. Fixes are included in 0.6.2; older concurrently running launchers still have compatibility limits documented in that changelog. [Changelog][changelog].

A RustSec search for the exact crate and the repository's public security-advisory listing found no crate-specific advisory on the research date. That is a limited search result, **not** a security review, dependency audit, or proof that no vulnerabilities exist. The documented Windows bug matters regardless of advisory assignment. [RustSec database](https://rustsec.org/advisories/), [repository advisories](https://github.com/bogusyogi/rightkit/security/advisories).

### Classifying adoption gaps

| Class                                   | Material gap                                                                                                                                  | Consequence and practical response                                                                                                             |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Underlying native semantics             | Landlock ABI-dependent rights and existing-descriptor authority; additive Windows ACL/capability access; path rather than snapshot semantics. | Require sufficient ABI, close unintended channels, stage immutable inputs, and audit actual Windows object access.                             |
| Native capability absent from the crate | Newer Landlock TCP-port controls, IPC/signal scopes and ioctl rights; fine-grained shared network/IPC policy; CPU/memory/disk quotas.         | Integrate native APIs or another launcher. TCP-port restriction alone still does not identify approved hosts.                                  |
| Integration/deployment                  | Seatbelt profile compatibility, runtime/DLL access, Windows DACL permissions and crash cleanup, nonportable sandboxed PTYs.                   | Validate each shipped tool/runtime on each OS and define cleanup/recovery procedures.                                                          |
| Needs a different isolation boundary    | Hostile multi-tenant code, kernel exploit resistance, private filesystem/process view, strong denial of resource exhaustion.                  | Use a suitably configured VM or a sandboxed runtime; Linux namespaces/cgroups can add missing host-kernel controls but retain a shared kernel. |

**Suitability judgment:** useful for reducing accidental access and as one confinement layer around semi-trusted or untrusted tools, especially when the workload's runtime needs are known. Native checks also restrict malicious code, but the release's broad implicit allowances, IPC paths, Windows state handling, and uneven guarantees prevent recommending it as a complete portable hostile-workload boundary without additional analysis and testing.

## Rust integration and verification

### Shared launcher: read-only input, writable workspace, denied direct IP access

This is a complete small Cargo project. No feature is required for these APIs. Keep the launcher and probe executable outside the writable workspace, create both directories first, and use a known standalone executable or an explicitly provisioned runtime. Additional runtime/helper paths must be reviewed and granted; do not fix runtime failures by granting the user's entire home. [API definitions][policy], [spawn implementation][spawn].

```toml
[package]
name = "rightkit-launch-probe"
version = "0.1.0"
edition = "2021"
rust-version = "1.98"

[dependencies]
rightkit-sandbox = { version = "=0.6.2", default-features = false }
```

There is no `.cargo/config.toml` or OS feature setup required. Choose the ordinary native target on each host. The Windows compile-only check below used `--target x86_64-pc-windows-gnu`; do not deploy that fact as runtime evidence. Exact top-level pinning does not freeze transitive dependencies: retain/review `Cargo.lock`. The actual verification resolved `rightkit-child-process 0.5.9` under the release's `^0.5.7` requirement.

`src/main.rs`:

```rust
use rightkit_sandbox::{
    landlock_abi_version, EnvPolicy, NetworkPolicy, ResourceAuthority,
    ResourceProfile, Sandbox, SandboxCommand, SandboxPolicy, SeccompMode,
};
use std::{error::Error, io, path::PathBuf, time::Duration};

fn refuse(message: &str) -> io::Error {
    io::Error::other(message)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let input = PathBuf::from(args.next().ok_or_else(|| refuse("missing input directory"))?)
        .canonicalize()?;
    let workspace = PathBuf::from(args.next().ok_or_else(|| refuse("missing workspace"))?)
        .canonicalize()?;
    let program = PathBuf::from(args.next().ok_or_else(|| refuse("missing executable"))?)
        .canonicalize()?;
    if !input.is_dir() || !workspace.is_dir() || !program.is_file() {
        return Err(refuse("expected two existing directories and an executable file").into());
    }
    if input.starts_with(&workspace) || workspace.starts_with(&input)
        || program.starts_with(&workspace)
    {
        return Err(refuse("input and workspace must be disjoint; executable must be outside workspace").into());
    }
    if !cfg!(any(target_os = "macos", target_os = "linux", target_os = "windows")) {
        return Err(refuse("no released confinement backend for this target").into());
    }
    if cfg!(target_os = "linux") {
        if landlock_abi_version().unwrap_or(0) < 3 {
            return Err(refuse("Landlock ABI >= 3 required for truncate protection").into());
        }
        if !cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
            return Err(refuse("network-only seccomp unavailable for this architecture").into());
        }
    }
    let env = EnvPolicy {
        inherit: false,
        keep: vec![],
        set: Default::default(),
    };
    let policy = SandboxPolicy::new()
        .allow_read(&input)
        .allow_write(&workspace)
        .allow_exec(&program)
        .network(NetworkPolicy::Deny)
        .seccomp(SeccompMode::NetworkOnly)
        .env(env);
    let command = SandboxCommand::new(program.as_os_str())
        .args(args)
        .cwd(&workspace);
    let sandbox = Sandbox::new(ResourceAuthority::new(ResourceProfile::Practical));
    // Never retry with Command::spawn if confinement setup fails.
    let child = sandbox.spawn(&policy, command)?;
    let output = child.wait_with_output_capped(Duration::from_secs(30), 1024 * 1024)?;
    if output.timed_out || output.truncated {
        return Err(refuse("deadline exceeded or output incomplete").into());
    }
    if !output.status.is_some_and(|status| status.success()) {
        return Err(refuse(&format!("child failed; denial classification: {:?}", output.denial)).into());
    }
    Ok(())
}
```

The code refuses unsupported targets, insufficient Linux truncate protection, unsupported Linux network-filter architectures, any spawn error, timeout, incomplete output, or unsuccessful exit. It deliberately selects `NetworkOnly` so the same policy can be used on Linux arm64; it does **not** claim the default broad syscall filter. It leaves Landlock at `Always` and does not request deny holes. Windows DACL setup can still refuse launch; the example does not silently provision broad ancestor rights.

Expected scope by backend:

| Operation                                                            | macOS                                                       | Linux with ABI ≥3                                   | Native Windows                                                                                            |
| -------------------------------------------------------------------- | ----------------------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Read staged input                                                    | Allowed if native permissions permit.                       | Allowed.                                            | Allowed if grant/ancestor/image setup succeeds.                                                           |
| Create/modify workspace file                                         | Allowed.                                                    | Allowed.                                            | Allowed.                                                                                                  |
| Open input for writing                                               | Denied under the stated disjoint-tree/no-alias assumptions. | Denied under those assumptions.                     | Expected denied if no other effective write grant exists for the AppContainer; caller must validate ACLs. |
| Read an unrelated private user file                                  | Denied outside system/imported/other grants.                | Denied outside implicit system/granted hierarchies. | Expected denied if it is not already accessible through AppContainer/capability ACLs.                     |
| Start fresh TCP connection                                           | Expected denied by outbound policy.                         | New IP socket creation denied.                      | Expected denied without network capability.                                                               |
| Use an inherited socket/broker, mutate all metadata, or prove no IPC | Not established.                                            | Not established.                                    | Not established.                                                                                          |

The canonicalization checks are useful preparation, **not a hostile-filesystem proof**. They do not reject every overly broad input/workspace, hard link, reparse race, or implicit grant. Use privately staged trees and reviewed runtime objects. On Linux, `chmod` and other metadata operations are outside the handled filesystem rights; `NetworkOnly` permits many syscalls, so “read-only input” here describes content/open/truncate policy, not immutable ownership/permissions. A runtime requiring a home, temp directory, DLL directory, or helper executable needs explicit integration work. [Linux rights and filter][linux], [shared policy][policy].

### Extra requirements: refuse instead of pretending to support them

The following complete `src/lib.rs` is **application code**, using the crate's actual deny-enforcement query. Call it before `sandbox.spawn` if adding those requirements to the launcher. It intentionally refuses unsupported contracts; it does not implement a destination filter or a generic capability detector.

```rust
use rightkit_sandbox::{deny_read_enforced, SandboxPolicy};
use std::io;

// Application admission checks; this is not a crate-provided capability report.
pub fn admit_extra_requirements(
    policy: &SandboxPolicy,
    require_no_ipc: bool,
    approved_hosts: &[String],
) -> io::Result<()> {
    if !policy.deny_read.is_empty() && !deny_read_enforced() {
        return Err(io::Error::other("required deny-read carve-outs cannot be enforced"));
    }
    if require_no_ipc {
        return Err(io::Error::other("this integration cannot enforce absence of all IPC"));
    }
    if !approved_hosts.is_empty() {
        return Err(io::Error::other("this integration has no enforced destination filter"));
    }
    Ok(())
}
```

For example, `admit_extra_requirements(&policy, false, &["api.example.com".into()])?` refuses **on every OS** in this integration. With `policy.clone().deny_read(secret)`, it refuses on Linux/Windows; macOS still has to successfully apply the policy at spawn. A successful call with no extra requirements is not evidence that the host enforces the baseline policy. [Deny-enforcement query][confine], [network policy][policy].

### Isolated child probes

This complete `src/bin/io_probe.rs` provides deterministic workload operations without a shell. It is intentionally **not confined when run directly**: the parent launches it through the preceding sandbox for restricted checks and directly only for isolated positive controls.

```rust
use std::{fs, io, net::{SocketAddr, TcpStream}, path::PathBuf, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let mode = args.next().ok_or_else(|| io::Error::other("missing mode"))?;
    let target = args.next().ok_or_else(|| io::Error::other("missing target"))?;
    match mode.to_str() {
        Some("read") => { fs::read(PathBuf::from(target))?; }
        Some("write") => { fs::write(PathBuf::from(target), b"probe")?; }
        Some("truncate") => {
            fs::OpenOptions::new().write(true).open(PathBuf::from(target))?.set_len(0)?;
        }
        Some("tcp") => {
            let addr: SocketAddr = target.to_str()
                .ok_or_else(|| io::Error::other("address must be UTF-8"))?.parse()?;
            TcpStream::connect_timeout(&addr, Duration::from_secs(2))?;
        }
        _ => return Err(io::Error::other("expected read, write, truncate, or tcp").into()),
    }
    Ok(())
}
```

Example invocation, with absolute paths chosen by the parent:

```text
rightkit-launch-probe INPUT_DIR WORKSPACE_DIR /absolute/path/io_probe read INPUT_DIR/data
rightkit-launch-probe INPUT_DIR WORKSPACE_DIR /absolute/path/io_probe write WORKSPACE_DIR/new
rightkit-launch-probe INPUT_DIR WORKSPACE_DIR /absolute/path/io_probe write INPUT_DIR/new
rightkit-launch-probe INPUT_DIR WORKSPACE_DIR /absolute/path/io_probe tcp 127.0.0.1:PORT
```

The first two should return success; the latter two should fail under the policy assumptions. A refused network operation need not produce a recognized `DenialSignature`. The `truncate` mode opens for writing before `set_len`; it tests a practical content-modification attempt, **not** the ABI-1/2 `O_RDONLY | O_TRUNC` bypass in isolation. A Linux ABI regression test should additionally issue that exact operation through a tiny native helper and confirm the file remains unchanged. [Native truncate semantics][landlock-man].

**Verification performed for this report:** macOS **27.2**, kernel **27.2.0**, **aarch64-apple-darwin**, Rust/Cargo **1.98.1**, pinned `rightkit-sandbox 0.6.2`, resolved `rightkit-child-process 0.5.9`. All three Rust snippets were compile-checked as one isolated project with `cargo check --all-targets`; the same project passed `cargo check --all-targets --target x86_64-pc-windows-gnu`. Native binaries were also built. No monorepo crate implementation or test runner was sandboxed.

| macOS child-process probe actually run               | Observed outcome                                              |
| ---------------------------------------------------- | ------------------------------------------------------------- |
| Read `inputs/data` through `io_probe`                | Success.                                                      |
| Create `workspace/new`                               | Success.                                                      |
| Create `inputs/new`                                  | Nonzero; file remained absent.                                |
| Truncate `inputs/data`                               | Nonzero; original content remained intact.                    |
| Read a sibling private file outside both roots       | Nonzero.                                                      |
| Connect to a live parent-owned loopback TCP listener | Direct unsandboxed helper succeeded; sandboxed helper failed. |
| Missing executable passed to launcher                | Nonzero before workload launch, from canonicalization.        |

Additional `/bin/cat`, `/usr/bin/touch`, and `/usr/bin/curl` child probes produced the expected corresponding outcomes. The live-loopback control returned an HTTP response outside confinement and failed inside it. These are narrow functional observations, not penetration testing or proof about all IPC. The guard snippet was compiled but its full refusal matrix was not executed. Linux/WSL2 execution, native Windows execution, sandboxed PTY, the browser fixture, and `execution` feature behavior were **not executed** for this report. Windows cross-compilation establishes only compile compatibility for that target and dependency resolution.

For a repeatable adoption suite, use fresh **child processes** and disposable data:

1. Create sibling private input/workspace/outside trees; retain original contents in the supervisor. Run positive and negative probes separately so an early denial cannot hide an untested operation. Check both exit/result and filesystem effects. Include input reads, workspace creation/deletion, unrelated reads, input overwrite/truncate, nested overlap, symlinks/hard links, and runtime/helper execution.
2. Bind an ephemeral parent-owned listener and prove the unsandboxed probe can connect before checking denial. Separately test IPv4/IPv6, UDP, Unix sockets, inherited connected descriptors, and any supplied proxy/broker. A connection failure to an absent listener proves nothing about enforcement.
3. On Linux, test an environment without usable Landlock and ABI 1/2 if supported by the product's deployment matrix; assert refusal under the launcher's ABI-3 admission rule. Test seccomp installation refusal and aarch64 default-allowlist refusal. Never interpret a skipped unavailable-host test as passing confinement evidence.
4. Exercise failures **inside the launcher API**, not only canonicalization: missing executable with a valid policy, invalid relative grant/cwd, `ResourceProfile::Strict`, denied Windows ancestor DACL, unavailable PTY confinement, Windows `deny_read`, and the caller guard for Linux denies. Use a controlled marker workload and verify it never ran.
5. Give a child/grandchild a readiness handshake before checking timeout/drop cleanup. Include one descendant holding output open and another redirecting output; verify termination by identity, not just by a recycled PID. Retain an outside process as a negative control for overbroad cleanup.
6. On Windows, prefer a disposable VM/user profile and test-owned grant trees. Inventory profiles/ACEs/exemptions first, wait for children to end, release pins/profile holders, sweep owned stale ancestors, then remove unused owned profiles. Confirm state restoration; a temporary directory alone does not contain profile/ACL/exemption state. Crash-recovery tests belong in that disposable environment.

For tests added to this monorepo, use its L2 gating/Test Toolkit and nextest recipes; keep terminal/browser launches unfocused. These recommendations are a verification plan, not tests claimed to have run here. The crate's own Windows e2e tests may skip real-user-store mutations unless `GITHUB_ACTIONS=true` or `RIGHTKIT_TEST_REAL_USER_STORES=1`; read the test body and verify execution rather than accepting its exit code as coverage. [Published tests](https://docs.rs/crate/rightkit-sandbox/0.6.2/source/tests/), [test gating notes][changelog].

## Alternatives and complementary layers

Neighboring [Seatbelt](seatbelt.md), [Landlock](landlock.md), [Landlock crate](landlock-crate.md), and [AppContainer](app-container.md) reports supplied leads about native mechanisms, compatibility, and inherited authority. Other neighboring wrapper documents were prompt-only at review time. None is primary evidence for the findings above; the relevant crate/native claims were checked independently.

This is a small selection for the launcher decision, not an exhaustive sandbox survey. Alternative versions and publication dates below were checked against their registries on the research date.

| Choice                                        | Relationship and coverage                                                                                                                                                                                                                                     | Maintenance evidence and integration trade-off                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`rightkit-sandbox 0.6.2`**                  | Higher-level Rust child launcher across macOS/Linux/native Windows, plus lifecycle, output and admission management.                                                                                                                                          | November 1, 2026 publication; short, active history. Low initial integration cost, but native capability admission and Windows ACL recovery remain your responsibility. [Registry][registry], [source][source].                                                                                                                                                                                                                                                                                                                                                                                                            |
| **`nono 0.81.0`**                             | Competing Rust capability wrapper around Landlock/Seatbelt. Released API has `CapabilitySet`, `Sandbox::support_info`, and sandbox application primitives. Native Windows is not supplied by those backends; README's “Windows (WSL2)” means Linux execution. | October 29, 2026 publication; a relevant actively publishing alternative. More explicit capability configuration may fit a fail-closed launcher, but inspect its exact release and lifecycle model rather than assuming drop-in `SandboxedChild` equivalence. [Registry](https://crates.io/api/v1/crates/nono), [versioned API](https://docs.rs/nono/0.81.0/nono/), [repository](https://github.com/nolabs-ai/nono).                                                                                                                                                                                                       |
| **`gaol 0.2.1`**                              | Older competing cross-platform Rust application-sandbox abstraction; multiprocess profile model and per-operation support query.                                                                                                                              | Latest registry release October 16, **2019**. README explicitly describes limited security review. That publication age warrants a current build/backend audit before adoption; it alone does not prove all repository development stopped. [Registry](https://crates.io/api/v1/crates/gaol), [project README](https://github.com/servo/gaol#readme).                                                                                                                                                                                                                                                                      |
| **Direct native integration**                 | `landlock 0.4.7` on Linux, a reviewed Seatbelt profile on macOS, Windows bindings for AppContainer/Jobs/ACLs. Bindings are building blocks, not a shared launcher.                                                                                            | `landlock` release July 27, 2026. More control over compatibility levels, handled rights and status; highest policy/lifecycle/packaging burden. Use when exact required rights matter more than sharing every builder. [Registry](https://crates.io/api/v1/crates/landlock), [versioned API](https://docs.rs/landlock/0.4.7/landlock/), [native references below](#authoritative-resources-and-open-adoption-questions).                                                                                                                                                                                                   |
| **External `srt` / bubblewrap**               | `srt` is a higher-level process wrapper with proxy filtering, using Seatbelt/macOS and bubblewrap/Linux. Bubblewrap itself is a Linux namespace/filesystem launcher, useful as an added layer or alternative backend.                                         | Maintained upstream repositories have current implementation/release records; `srt` labels itself a Beta Research Preview. External processes, installation and proxy lifecycle add integration cost. Neither a proxy alone nor an arbitrary bubblewrap invocation guarantees isolation: review the generated boundary. This row does not claim native-Windows parity for either tool. [srt README](https://github.com/anthropics/sandbox-runtime#readme), [srt releases](https://github.com/anthropics/sandbox-runtime/releases), [bubblewrap security model](https://github.com/containers/bubblewrap#sandbox-security). |
| **VM / Windows Sandbox / restricted runtime** | Stronger or different boundary: guest-kernel separation for a VM; WebAssembly/WASI instead restricts workloads to a runtime interface. Not interchangeable with running arbitrary host binaries.                                                              | Greater deployment/startup/runtime compatibility cost. Prefer a maintained VM boundary for hostile multi-tenant native code; supply only reviewed mounts/devices/network paths. Windows Sandbox is explicitly virtualization-based and disposable. [Microsoft Windows Sandbox](https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/), [Wasmtime security model](https://docs.wasmtime.dev/security.html).                                                                                                                                                        |

For a known toolchain with staged inputs and an ordinary “reduce file access and deny fresh Internet connections” requirement, this crate can be sufficient **after** per-platform validation. For approved-host access, add a non-bypassable proxy/native egress layer. For precise Linux rights or exact macOS IPC policy, direct integration may be simpler than working around the common builder. For strict object isolation, adversarial resource control, or hostile native tenants, choose a stronger boundary and use this crate only where it adds useful supervision or defense in depth. These are adoption recommendations, not guarantees published by the crate.

## Authoritative resources and open adoption questions

The primary release anchors are the [crates.io metadata][registry], [owner record](https://crates.io/api/v1/crates/rightkit-sandbox/owners), [0.6.2 archive][archive], [version-specific API documentation](https://docs.rs/rightkit-sandbox/0.6.2/rightkit_sandbox/), [published source][source], [packaged changelog][changelog], and [linked RightKit repository](https://github.com/bogusyogi/rightkit). The archive is authoritative where a search result's cached `latest` or repository staging label disagrees with published functionality.

Native references: [Linux Landlock userspace API][landlock-doc], [Landlock manual and ABI table][landlock-man], [seccomp filter model][seccomp-doc], [Apple App Sandbox](https://developer.apple.com/documentation/security/app-sandbox), [Microsoft AppContainer isolation](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation), [AppContainer profile creation and minimum API requirements][profile-api], and [Job Object lifetime/descendant behavior][job-doc].

Resolve these material questions before adopting:

- Which exact OS builds, Linux ABIs/architectures, WSL guest kernels/filesystems, tool runtimes and PTY modes will the product support? The crate does not publish a complete verified deployment floor.
- Does the product require denied secret subtrees, immutable input objects, zero IPC, or approved destinations? Which separate enforceable boundary covers each missing requirement, and does admission refuse before launch when it is unavailable?
- On native Windows, who owns ancestor provisioning, concurrent ACL updates, stale grant recovery, profile collection, and loopback-exemption cleanup? Can this be validated in a disposable user/VM with crash tests?
- Is the implicit system/`bsd.sb`/AppContainer ambient authority acceptable for the specific threat model, including available brokers and launcher TCC rights? The probes here do not answer that globally.
- What evidence supports adversarial descendant cleanup and handle hygiene on each shipped OS? Is a cgroup, VM, or independently owned supervisor needed?
- How will updates be reviewed against the published archive when staged repository code and release notes diverge? In particular, re-evaluate native Windows deny-read support only after a future **published implementation** demonstrates it; issue closure is insufficient.

[registry]: https://crates.io/api/v1/crates/rightkit-sandbox
[archive]: https://crates.io/api/v1/crates/rightkit-sandbox/0.6.2/download
[source]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/
[manifest]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/Cargo.toml.orig
[readme]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/README.md
[changelog]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/CHANGELOG.md
[policy]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/policy.rs
[confine]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/confine.rs
[error]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/error.rs
[spawn]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/spawn.rs
[resources]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/resources.rs
[linux]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/linux.rs
[seccomp]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/seccomp.rs
[seatbelt]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/seatbelt.rs
[windows]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/windows_appcontainer.rs
[ancestors]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/windows_ancestors.rs
[pins]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/pins.rs
[pty]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/pty.rs
[tree]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/process_tree.rs
[execution]: https://docs.rs/crate/rightkit-sandbox/0.6.2/source/src/execution/mod.rs
[landlock-doc]: https://docs.kernel.org/userspace-api/landlock.html
[landlock-man]: https://man7.org/linux/man-pages/man7/landlock.7.html
[seccomp-doc]: https://docs.kernel.org/userspace-api/seccomp_filter.html
[profile-api]: https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-createappcontainerprofile
[job-doc]: https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects
