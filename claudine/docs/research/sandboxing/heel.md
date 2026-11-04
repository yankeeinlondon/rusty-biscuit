---
prompt: |-
    Research the Rust `heel` crate to help a developer decide whether
    and how to use it in a sandbox launcher intended to run across operating systems.

    The starting claim is that `heel` provides sandboxing across multiple operating
    systems. Verify its identity, supported targets, and released backends rather than
    assuming cross-platform parity.

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
       - Identify the released backend and native enforcement mechanism for
         each supported OS. Distinguish implemented, experimental, planned,
         and unsupported targets, including native Windows versus WSL2.
       - Provide a per-OS matrix of policy controls, minimum OS/runtime
         requirements, privileges, external tools, and distribution constraints.
       - Explain how a shared policy maps to each backend, including default
         permissions, path semantics, and any differences in rule composition.
       - Trace unsupported policy requests through backend selection and
         launch: are they rejected, weakened, ignored, or delegated?
       - Determine whether callers can require equivalent guarantees across
         OSes and inspect the effective policy before launching a workload.
       - Investigate backend fallback, no-op implementations, helper processes,
         and multithreaded launcher constraints. Show how missing dependencies
         or backend failures affect whether the workload starts.
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
hash: 0fd7c8a3d7afc3c2-6f7b59f0adc835c2
last_updated: 2026-11-03
---

# `heel`: native sandboxing for a portable Rust launcher

**Assessment:** `heel` is a real, published Rust dependency with released macOS, Linux, and native Windows implementations. **Version 0.7.4 is not sufficient by itself for equivalent confinement of malicious native workloads across those systems.** In particular, Windows destination filtering can be bypassed, Linux grants have execution and metadata-mutation gaps, and Unix execution paths have cleanup gaps. It is a candidate for reducing accidental access by controlled workloads, or as one layer behind a stronger boundary, after platform-specific validation. The upstream README itself excludes general arbitrary-untrusted-code sandboxing from its intended use. LLM-generated code must still be treated as potentially malicious; its origin does not reduce the authority of a bypass. [Published README](https://docs.rs/crate/heel/0.7.4/source/README.md), [Windows network defect](https://github.com/May-Agent/heel/issues/152), [Linux permission defects](https://github.com/May-Agent/heel/issues/151), [metadata defect](https://github.com/May-Agent/heel/issues/153)

Research date: **November 3, 2026**. Exact stable release examined: **`heel` 0.7.4**, published **November 2, 2026, 15:52:05 UTC**, not yanked. The registry's sparse index and its Git-backed copy agree. Source inspection used the downloaded published `.crate`, whose `.cargo_vcs_info.json` identifies commit `9fc026c776e4b541f114f0cff988d8272fbaa235`. Unless explicitly labeled otherwise, implementation statements below concern that archive, not today's `dev` branch. [Registry record](https://index.crates.io/he/el/heel), [registry mirror](https://github.com/rust-lang/crates.io-index/blob/master/he/el/heel), [published source](https://docs.rs/crate/heel/0.7.4/source/), [release commit](https://github.com/May-Agent/heel/tree/9fc026c776e4b541f114f0cff988d8272fbaa235)

## Identity, releases, and maintenance

The manifest names package and library `heel`, edition 2024, dual **MIT OR Apache-2.0** licensing, and repository `https://github.com/lexoliu/heel`. That repository now redirects to **`May-Agent/heel`**; GitHub's live repository API reports organization `May-Agent`, default branch `dev`, and `archived: false`. This is a repository transfer, not evidence of a different crate. The workspace contains the library, its `heel` executable, and Node bindings; no larger parent project is established by the package metadata. `rappct` was formerly a dependency, not the identity of this crate. [Manifest](https://docs.rs/crate/heel/0.7.4/source/Cargo.toml.orig), [live repository metadata](https://api.github.com/repos/lexoliu/heel), [contributors](https://api.github.com/repos/May-Agent/heel/contributors)

The observable primary human maintainer is **Lexo Liu (`lexoliu`)**, the historical repository owner and dominant contributor. The crates.io owners API returned HTTP 403 during this research, so the current registry publisher/owner roster could not be independently verified. Do not equate repository control with proof of registry publishing authority. The dependency is usable: Cargo downloaded 0.7.4 from the registry and both examples below passed `cargo check` against it.

### Publication history

The following covers the entire 24-month window, November 3, 2024–November 3, 2026. There are **16 indexed releases**, all ordinary stable-version identifiers, no indexed prereleases, and no indexed version before 0.1.0. Only 0.1.0 and 0.1.1 are yanked. Dates below are registry publication dates in UTC; changes come from the release's changelog. A changelog comparison mentioning 0.0.1 does not establish a published release. [Registry history](https://index.crates.io/he/el/heel), [versioned changelog](https://docs.rs/crate/heel/0.7.4/source/CHANGELOG.md)

| Version(s)   | Publication date | Relevant milestone                                                                                                                                                  |
| ------------ | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.1.0, 0.1.1 | 2026-03-30       | Both yanked. Initial library/CLI integration and early native-backend work. The changelog dates 0.1.0 March 25; the registry records publication March 30.          |
| 0.2.0        | 2026-08-26       | Breaking core/IPC/enforcement rework; Linux thread compatibility and write/execute fixes; network auditing.                                                         |
| 0.3.0        | 2026-09-13       | Gate and Windows executable-location fixes.                                                                                                                         |
| 0.3.1        | 2026-10-11       | Windows ACL, pipe, long-path and directory-grant fixes; network audit ordering; scheduling priority.                                                                |
| 0.4.0, 0.4.1 | 2026-10-12       | Direct Win32 backend replaces `rappct`; IPC builder typestate and portable PATH API; job assignment at creation. Patch fixes dependency pinning and release assets. |
| 0.4.2        | 2026-10-16       | Quiet IPC reader closure.                                                                                                                                           |
| 0.5.0        | 2026-10-18       | Fallible IPC handlers and verbatim argv; Windows effective-rights checks and grant cleanup.                                                                         |
| 0.5.1        | 2026-10-22       | Stable Windows identities/persistent grants; ACL serialization and private-temp fixes.                                                                              |
| 0.6.0        | 2026-10-29       | Linux credential/persistence denylist broker; Windows rejects fully writable filesystem settings.                                                                   |
| 0.7.0, 0.7.1 | 2026-10-29       | Breaking typed broker-precondition errors; separate never-loopback-exempt Windows identity for deny-all.                                                            |
| 0.7.2        | 2026-10-29       | Linux opt-in process-tree supervision; resolved-path/file-identity checks and no-follow grants.                                                                     |
| 0.7.3        | 2026-11-01       | macOS credential protection anchored to real home locations.                                                                                                        |
| 0.7.4        | 2026-11-02       | Opt-in executable working directory, including macOS executable mappings.                                                                                           |

Observable cadence is a March publication, an almost five-month gap to August, then increasingly frequent September–November releases, including four on October 29. GitHub showed development activity on November 3. **Judgment:** active development, but rapidly changing enforcement and many unresolved correctness reports make adoption require more validation than a stable facade suggests. This judgment is based on releases, source changes, and issues, not downloads or a pre-1.0 label. [Commit history](https://api.github.com/repos/May-Agent/heel/commits?per_page=5), [open issues](https://api.github.com/repos/May-Agent/heel/issues?state=open&per_page=100)

There is no 1.x release and no manifest `rust-version`/published MSRV promise. Edition 2024 implies at least Rust 1.85 for language support, but **that is not a tested minimum for the dependency graph**. This report compiled with Rust/Cargo 1.98.1. Pin the crate and retain a tested lockfile; reevaluate dependency updates. Important target dependencies are `landlock` 0.4.7 and `seccompiler` 0.5 on Linux, `windows` 0.62 on Windows, and `pty-process` 0.5 on macOS. The Linux filter builder does not require an external `libseccomp` launcher/library; the mechanisms still require kernel support. No Docker daemon is required. [Published manifest](https://docs.rs/crate/heel/0.7.4/source/Cargo.toml), [Rust edition requirements](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)

Migration requires reading source signatures, not copying old README snippets: construction is now async, `SandboxConfigBuilder::build()` returns a config directly, IPC handlers return `Result`, and IPC configuration changes the builder's type. Path grants now require `ResolvedPath`; that signature change landed in the 0.7.2 patch line, illustrating why a patch upgrade still deserves review. Replace raw grant strings with `resolve_paths`/`PathRequest` or verified `ResolvedPath::new`. Unix-only `DEFAULT_SANDBOX_PATH` has the portable alternative `default_sandbox_path()`. [Changelog](https://docs.rs/crate/heel/0.7.4/source/CHANGELOG.md), [path-resolution history](https://api.github.com/repos/May-Agent/heel/commits?path=src/resolve.rs&per_page=8), [public exports](https://docs.rs/crate/heel/0.7.4/source/src/lib.rs)

### Cargo features

| Feature   | Default?                             | Actual effect                                                                                                                                                                                                  |
| --------- | ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `python`  | Yes                                  | Enables `which`-based Python discovery. Python configuration/types remain present without it; automatic discovery returns no interpreter, so callers needing Python must arrange an explicit interpreter/venv. |
| `which`   | Implicit optional-dependency feature | Enables the optional dependency; does not itself enable code guarded by `feature = "python"`. Windows separately needs `which` for executable lookup.                                                          |
| `default` | `['python']`                         | No backend selection or security-strength switch.                                                                                                                                                              |

There are no Cargo features selecting Linux, Seatbelt, AppContainer, networking, or IPC. Backends are selected by target `cfg`; disabling default features does not disable confinement or remove all CLI/proxy dependencies. [Manifest](https://docs.rs/crate/heel/0.7.4/source/Cargo.toml), [Python discovery](https://docs.rs/crate/heel/0.7.4/source/src/python/venv.rs)

## Public API and policy-to-launch workflow

1. Resolve caller paths once with `resolve_paths([(PathRequest, tag), …])`. A writable request tells the resolver where a previous sandbox might have planted links. `ResolvedPath::new` checks an already-resolved, existing, absolute path without link components; it is not a general canonicalizer.
2. Build `SandboxConfig<N>` through `SandboxConfig::builder()`. Choose `NetworkPolicy`, `SecurityConfig`, filesystem grants, optional limits, identity, Python environment, and IPC.
3. Await `Sandbox::with_config(config)` or `with_config_and_executor(config, executor)`. Setup creates/validates the work directory, prepares optional host-side services and the native backend, and returns an error if required setup fails.
4. Call `sandbox.command(program)` and its consuming builder methods (`arg`, `args`, `env`, `current_dir`, `stdin`, `stdout`, `stderr`), then `spawn`, `output`, or `status`. `spawn` returns `Child` with `wait`, `wait_with_output`, `kill`, `try_wait`, and stream accessors. Check both the Rust result and the child's exit status.

Sources: [resolution](https://docs.rs/crate/heel/0.7.4/source/src/resolve.rs), [configuration](https://docs.rs/crate/heel/0.7.4/source/src/config.rs), [construction](https://docs.rs/crate/heel/0.7.4/source/src/sandbox.rs), [command](https://docs.rs/crate/heel/0.7.4/source/src/command.rs), [child](https://docs.rs/crate/heel/0.7.4/source/src/platform/child.rs).

The default is `DenyAll`, `filesystem_strict(true)`, `writable_file_system(false)`, strict security toggles, no grants, no host read/write denylist, no resource limits, no IPC, and no Linux supervised-spawn helper. GPU and NPU access are **on** in the strict security preset; general hardware access and TCC prompts are off. A generated work directory is writable, normally non-executable, and removed on drop; a caller-supplied `working_dir` is preserved. `keep_working_dir` preserves a generated directory. **0.7.4 allows `executable_working_dir(true)`**, contrary to older prose saying no configuration can make it executable. [Defaults and methods](https://docs.rs/crate/heel/0.7.4/source/src/config.rs), [security defaults](https://docs.rs/crate/heel/0.7.4/source/src/security.rs), [work-directory lifecycle](https://docs.rs/crate/heel/0.7.4/source/src/workdir.rs)

`Access::READ`, `WRITE`, and `EXEC` are positive grants; WRITE and EXEC imply read, and repeated grants union their access. Directory grants cover descendants. They do not subtract permissions from a broader grant. On macOS the generated profile applies baseline rules, protections, explicit grants, then optional host-protection denylists; exceptions for explicitly named credential stores differ from broad parent-directory grants. Linux normally composes positive Landlock grants; optional denylists invoke a broker and carved rulesets. Windows grants ACL rights to a package/capability SID, alongside rights already available to AppContainers. These are not equivalent rule algebras. [Access model](https://docs.rs/crate/heel/0.7.4/source/src/grant.rs), [macOS profile](https://docs.rs/crate/heel/0.7.4/source/templates/sandbox.txt), [Linux rules](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/landlock_rules.rs), [Windows grants](https://docs.rs/crate/heel/0.7.4/source/src/platform/windows/container.rs)

The supervisor, network proxy, IPC handlers, Python installation, and Linux broker are trusted host-side code. Python environment creation/package installation happens before workload confinement; package installation is not made safe by later launching Python inside heel. IPC deliberately grants host authority: `IpcRouter` registers `IpcCommand` handlers and generated executable shims; Unix uses local sockets and Windows named pipes. `external_ipc(TcpIpcEndpoint)` relays through the host, even for an otherwise offline workload. Do not add such a route to a policy that promises no external effects. Validate handler paths, arguments, authorization, and quotas separately. [Sandbox initialization](https://docs.rs/crate/heel/0.7.4/source/src/sandbox.rs), [IPC](https://docs.rs/crate/heel/0.7.4/source/src/ipc/mod.rs), [Python setup](https://docs.rs/crate/heel/0.7.4/source/src/python/venv.rs)

There is no public portable syscall policy, PID namespace, mount namespace, chroot, cgroup, registry-rule builder, arbitrary IPC allowlist, or comparable whole-machine isolation configuration. `ResourceLimits` exposes memory bytes, CPU seconds, file-size bytes, and process count; `ProcessPriority` exposes CPU niceness and I/O priority. Scheduling priority is not a resource quota. PTY-based `run_interactive` is macOS-only. [Public surface](https://docs.rs/crate/heel/0.7.4/source/src/lib.rs), [configuration](https://docs.rs/crate/heel/0.7.4/source/src/config.rs), [priority](https://docs.rs/crate/heel/0.7.4/source/src/priority.rs)

## Released platform behavior

Seatbelt applies kernel access checks to a macOS process using an SBPL profile. Landlock is a Linux security module through which an unprivileged thread restricts itself and future descendants; permissions accumulate within a layer and stacked layers can only restrict further. Seccomp filters system calls; ordinary BPF cannot safely inspect pointed-to path strings. AppContainer launches a Windows process with a restricted application identity and capabilities; ACLs govern object access. A Job Object owns processes and applies selected limits but is not a filesystem sandbox. [Apple App Sandbox overview](https://developer.apple.com/documentation/security/app-sandbox), [Linux Landlock contract](https://docs.kernel.org/userspace-api/landlock.html), [seccomp contract](https://docs.kernel.org/userspace-api/seccomp_filter.html), [Microsoft AppContainer](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation), [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)

| Target                | Released backend and runtime floor                                                                                                | Filesystem and IPC                                                                                                                                                 | Network boundary                                                                                                                            | Privileges, tools, distribution                                                                                                                                                                                                                                                    |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| macOS                 | `/usr/bin/sandbox-exec -p SBPL`; constructor requires macOS 10.15+                                                                | Broad runtime reads narrowed by protected locations and explicit grants; workspace writes; baseline Mach services, POSIX IPC, process creation and signals allowed | Deny outbound by default; enabled networking permits the local proxy endpoint; profile also permits loopback server behavior                | No root for normal self-restriction. Requires `/usr/bin/sw_vers` and `/usr/bin/sandbox-exec`; PTY is a separate macOS facility. SBPL compatibility is a deployment responsibility.                                                                                                 |
| Linux                 | Landlock ABI 4, seccomp BPF, optional seccomp-notification broker; kernel 6.7+ version gate; x86-64/aarch64 filter implementation | Positive object/tree grants plus baseline runtime paths; optional read/write denylists through broker; Unix sockets remain possible                                | DenyAll blocks new Internet stream/datagram/raw sockets; proxied mode allows TCP and brokers `connect` to the proxy address and port        | No root for baseline Landlock/seccomp, but kernel configuration/outer policy must permit them. Proxy/denylists require seccomp user notification and Yama scope ≤1. `/proc` and its kernel-release entry are required.                                                             |
| Native Windows        | Direct Win32 AppContainer + attributed `CreateProcessW` + Job Object; documented Windows 10+                                      | ACL grants, existing package permissions and private AppContainer storage; named-pipe IPC; directory access has an open defect                                     | DenyAll omits `internetClient` and loopback exemption. Proxied mode grants Internet access and loopback exemption, permitting direct bypass | User must be able to create profiles and change granted-object DACLs. Loopback exemption uses machine-wide firewall configuration APIs and can encounter access-denied/policy restrictions; do not promise an unelevated filtered-network deployment. No external `rappct` helper. |
| WSL2                  | Linux backend in the guest                                                                                                        | Guest Linux path/object semantics; mounted Windows filesystems need separate testing                                                                               | Guest kernel and broker requirements, not native Windows AppContainer                                                                       | Older/custom WSL kernels or disabled Landlock fail. Host Windows version alone establishes nothing.                                                                                                                                                                                |
| Other OS/architecture | No released usable backend established                                                                                            | No portable emulation                                                                                                                                              | No fallback guarantee                                                                                                                       | `NativeBackend` exists only for the three named OSes; an `UnsupportedPlatform` branch does not establish that every other target even compiles.                                                                                                                                    |

Architecture qualification: the Linux filter explicitly supports x86-64 and AArch64; other architectures return `UnsupportedPlatform`. The changelog records native Windows ARM64 assets, but 0.7.4 drops the x86-64 macOS CI lane. That is reduced test evidence, not proof that the macOS source cannot compile on Intel. No broader target-support promise was verified.

Matrix evidence: [macOS implementation](https://docs.rs/crate/heel/0.7.4/source/src/platform/macos/mod.rs), [profile](https://docs.rs/crate/heel/0.7.4/source/templates/sandbox.txt), [Linux implementation](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/mod.rs), [seccomp architecture and sockets](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/seccomp_filter.rs), [broker preconditions](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/broker/mod.rs), [Windows launch](https://docs.rs/crate/heel/0.7.4/source/src/platform/windows/launch.rs), [Windows loopback setup](https://docs.rs/crate/heel/0.7.4/source/src/platform/windows/loopback.rs), [Microsoft firewall API](https://learn.microsoft.com/en-us/windows/win32/api/netfw/nf-netfw-networkisolationsetappcontainerconfig).

These backends are released implementations, not merely plans. Old indexed 0.1.1 rustdoc still labels Linux/Windows planned; it does not describe 0.7.4. Conversely, a stale comment in 0.7.4's internal `SpawnRequest` says Windows has no backend: the actual `NativeBackend` alias and Windows launch implementation contradict it. The code determines the finding here. [Versioned library source](https://docs.rs/crate/heel/0.7.4/source/src/lib.rs), [backend selection](https://docs.rs/crate/heel/0.7.4/source/src/platform/mod.rs)

The Apple-supplied local manual `/usr/share/man/man1/sandbox-exec.1` was inspected: it marks the command deprecated and directs developers to App Sandbox. Apple's archived web manual URLs returned 404 during research. Apple documents entitlement-based [App Sandbox](https://developer.apple.com/documentation/security/app-sandbox); that is not a drop-in SBPL launcher replacement. No removal date or durable SBPL compatibility promise was verified. Heel's 10.15 version gate is not proof of successful enforcement on every later macOS release.

### Shared policy differences and unsupported requests

| Request                                                        | macOS                                                   | Linux                                                                                           | Windows                                                                            |
| -------------------------------------------------------------- | ------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Read-only input content                                        | Explicit read grant; avoid overlapping write grants     | Landlock read grant restricts content writes, but metadata and unintended execution remain gaps | ACL read grant, subject to ambient package permissions and directory-access defect |
| Writable workspace                                             | Dedicated directory; optional executable-workdir escape | Dedicated directory; additional writable grants unintentionally executable                      | Dedicated directory plus private profile temp/storage                              |
| Only the declared inputs are readable                          | Not provided: broad system/runtime baseline             | Not provided: `/etc`, `/proc`, `/sys`, `/run` and runtime/device baselines remain               | Not provided: resources available to all packages and AppContainer storage remain  |
| Fully writable filesystem                                      | Accepted, loses core write confinement                  | Accepted, broader Landlock grants                                                               | Rejected with `UnsupportedSetting`                                                 |
| `ReadDenylist::credentials()` / `WriteDenylist::persistence()` | Profile restrictions                                    | Broker + Landlock; known bypasses below                                                         | Rejected with `UnsupportedSetting`                                                 |
| CPU/process/file-size limits                                   | Installed through Unix rlimits, with native semantics   | Installed through Unix rlimits, with native semantics                                           | Silently unused in this release                                                    |
| Equivalent effective-policy report                             | None                                                    | None                                                                                            | None                                                                               |

These are source-derived results, not claims that similar method names imply parity. `SecurityConfig` is also not a portable guarantee schema: several macOS credential/browser/keychain/NPU concepts are not implemented as independent Linux/Windows controls. In Linux ordinary mode protection often means absence of a positive grant; a broad grant can reopen protected data. Optional host denylists are distinct settings, absent by default. [Configuration](https://docs.rs/crate/heel/0.7.4/source/src/config.rs), [Linux planning](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/landlock_rules.rs), [Windows rejection](https://docs.rs/crate/heel/0.7.4/source/src/platform/mod.rs), [Linux credential follow-up](https://github.com/May-Agent/heel/issues/139)

Unix resource limits are inherited, but are **not aggregate tree quotas**: `RLIMIT_AS` and `RLIMIT_CPU` are process limits, `RLIMIT_FSIZE` limits an individual file, and `RLIMIT_NPROC` has per-user and privilege semantics. On Windows only `max_memory_bytes` reaches a per-process job memory limit; the other three values are dropped. Reject those requirements in caller code before construction. The README's whole-tree resource statement is too broad. [Unix mapping](https://docs.rs/crate/heel/0.7.4/source/src/platform/rlimit.rs), [Windows mapping](https://docs.rs/crate/heel/0.7.4/source/src/platform/windows/mod.rs), [open limit defect](https://github.com/May-Agent/heel/issues/156), [Linux rlimit definitions](https://man7.org/linux/man-pages/man2/getrlimit.2.html)

### Failure behavior and effective-policy visibility

Backend selection is compile-time and has no observed alternate backend or unsandboxed retry. Linux creates a hard-requirement ABI-4 ruleset at construction and checks `RulesetStatus::FullyEnforced` when restricting the child. Failure prevents `exec`. There is no best-effort ABI downgrade. Broker setup errors include `PtraceScopeTooStrict { scope, max }` and `SeccompNotifyUnavailable`; do not automatically lower a host sysctl to make a launch work. Other relevant errors are `UnsupportedPlatformVersion`, `UnsupportedSetting`, `NotEnforced`, `InvalidProfile`, `InitFailed`, `Path`, `GrantRedirected`, and `Io`. The `Error` enum is non-exhaustive. [Linux status checks](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/landlock_rules.rs), [error definitions](https://docs.rs/crate/heel/0.7.4/source/src/error.rs)

On macOS construction checks the OS and generates the profile, but does not launch `sandbox-exec` to validate it. A missing executable yields an I/O error at spawn; a rejected profile can instead produce a successfully spawned launcher that exits nonzero without starting the workload. `Ok(Child)` alone proves neither target execution nor confinement. Windows profile/ACL/launch preparation errors abort through `InitFailed`/I/O/path errors; there is no observed plain `CreateProcess` fallback that discards AppContainer attributes. [macOS launch path](https://docs.rs/crate/heel/0.7.4/source/src/platform/macos/mod.rs), [Windows preparation](https://docs.rs/crate/heel/0.7.4/source/src/platform/windows/mod.rs)

`Sandbox::config()` exposes the retained policy-independent configuration; `proxy_url()` reports an endpoint and `ipc_endpoint()` an IPC endpoint. **None is an attestation of effective restrictions.** There is no public per-platform capabilities object, effective SBPL/ACL/Landlock report, portable enforcement status, or `require_equivalent_guarantees` operation. Profile generation exists inside a private platform module; it is not an exported portable inspection API. `record_denials()` provides macOS log-based records or Linux broker refusals and rejects Windows; it is not exhaustive Linux kernel-denial reporting and silence is not evidence of permission denial. [Sandbox accessors](https://docs.rs/crate/heel/0.7.4/source/src/sandbox.rs), [denial recording](https://docs.rs/crate/heel/0.7.4/source/src/denial.rs), [module visibility](https://docs.rs/crate/heel/0.7.4/source/src/lib.rs)

For strict requirements, maintain a caller-owned requirement check **before** constructing or launching anything. With 0.7.4, refuse native Windows for adversarial destination filtering, refuse Linux for a promise that all host mutations are confined to writable grants, and refuse Unix `output()` when guaranteed cancellation/tree cleanup is required. No runtime flag repairs these gaps. If those are product requirements across all OSes, refuse the launch or select another independently validated isolation layer.

## Networking in detail

`NetworkPolicy::check(&DomainRequest)` returns an async boolean. `DomainRequest` supplies normalized `host()` and `port()`. `DenyAll` sets the associated `DENIES_ALL` constant, so no proxy starts. `AllowAll`, `AllowList`, `CustomPolicy`, and `Audited<N>` use an HTTP/HTTPS proxy outside the sandbox; an empty allowlist still uses that proxy. `AllowList` checks hostnames but **does not restrict ports**. A custom policy is needed for host-and-port rules. `*.example.com` matches subdomains, not the bare domain; exact names are lowercased and trailing dots removed. [Policy source](https://docs.rs/crate/heel/0.7.4/source/src/network/policy.rs)

This is an HTTP proxy and CONNECT tunnel, not transparent support for arbitrary protocols, UDP, QUIC, DNS packets, or every application's networking stack. Applications must use the injected proxy variables or speak proxy protocol. HTTPS remains end-to-end through CONNECT: the proxy checks the requested authority, not certificate identity, application authorization, URL paths, or encrypted requests inside the tunnel. A hostname/port rule therefore is not proof of the remote service's identity. Trust and credential verification remain the TLS client's job. [Proxy implementation](https://docs.rs/crate/heel/0.7.4/source/src/network/proxy.rs), [environment injection](https://docs.rs/crate/heel/0.7.4/source/src/command.rs)

Landlock ABI 4 permits TCP by **port**, not destination address. Heel's Linux proxied mode adds a seccomp-notification broker that checks the loopback IP as well, so it is inaccurate to describe this release as only a proxy-port Landlock rule. Windows, however, adds `internetClient` and a loopback exemption without a matching destination restriction; bypassing proxy variables bypasses the policy and its audit log. `Audited<N>` records proxy decisions, not direct connections or every network attempt. [Linux broker](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/broker/mod.rs), [broker network checks](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/broker/net.rs), [Windows defect](https://github.com/May-Agent/heel/issues/152), [audit implementation](https://docs.rs/crate/heel/0.7.4/source/src/network/audit.rs)

The proxy's local-address check rejects some special addresses, but not all host interface addresses or IPv4-mapped IPv6 spellings. Do not rely on its documented “cannot reach host services” promise. `DenyAll` is a useful new-Internet-connection restriction, not proof of no communication: inherited connected sockets, approved IPC handlers, Unix sockets, and allowed Mach services need separate analysis. [Proxy address classification](https://docs.rs/crate/heel/0.7.4/source/src/network/proxy.rs), [open local-address defect](https://github.com/May-Agent/heel/issues/154)

## Material defects and integration pitfalls

The following are public issue reports, **open on the research date**, not formal CVEs. The central implementation gaps were cross-checked against 0.7.4 source; issue proposals and acceptance criteria are not shipped fixes.

| Finding                                                                                   | Version applicability and consequence                                                                                                                                                 | Classification / mitigation                                                                                                                                        |
| ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [#151: Linux READ/WRITE also execute](https://github.com/May-Agent/heel/issues/151)       | `landlock_access` starts with `AccessFs::from_read`, which includes Execute in Landlock 0.4.7. A writable explicit grant can run written binaries despite the advertised separation.  | Crate mapping defect. Refuse a no-execute guarantee until fixed and regression-tested; interpreter access already permits execution of readable scripts.           |
| [#153: Linux metadata writes escape grants](https://github.com/May-Agent/heel/issues/153) | Ordinary mode does not mediate chmod/chown/xattr/time changes, and broker checks do not fully authorize them against writable grants. User-owned files outside grants can be damaged. | Landlock capability limitation plus missing crate mediation. A read-only content grant is not immutable metadata; use another isolation boundary for hostile code. |
| [#150: read-denylist bypass](https://github.com/May-Agent/heel/issues/150)                | Broker `O_RDONLY\|O_CREAT` handling can return readable descriptors for existing denied credentials.                                                                                  | Crate broker defect. Do not expose secrets and rely on this denylist to hide them.                                                                                 |
| [#152: Windows proxy bypass](https://github.com/May-Agent/heel/issues/152)                | Present when networking enables Internet capability; arbitrary direct destinations escape allowlist/custom policy/audit.                                                              | Missing Windows destination enforcement. Deny networking or add a separately enforced network boundary.                                                            |
| [#154: host-local address check](https://github.com/May-Agent/heel/issues/154)            | Proxy's special-address checks omit host interface addresses and mapped IPv6 normalization.                                                                                           | Crate proxy defect. Do not depend on it for host-service isolation.                                                                                                |
| [#155: Unix `output()`/`status()` cleanup](https://github.com/May-Agent/heel/issues/155)  | Backend execute paths bypass tracked spawn groups; cancellation and background descendants can survive.                                                                               | Lifecycle defect. Prefer tracked `spawn`, await/kill explicitly, and test descendants; ordinary process groups still do not own deliberate escapees.               |
| [#156: Windows dropped limits](https://github.com/May-Agent/heel/issues/156)              | CPU-time, process-count, file-size settings do not reach the job.                                                                                                                     | Crate omission. Reject these requirements; use direct job limits or another layer.                                                                                 |
| [#105: host process signaling](https://github.com/May-Agent/heel/issues/105)              | macOS profile broadly allows signals; Linux does not scope them to this sandbox. Same-user host processes can be attacked.                                                            | Missing process isolation policy. Separate account/VM or independent restriction where this matters.                                                               |
| [#131: Windows directory access](https://github.com/May-Agent/heel/issues/131)            | Reported inability to list/enter granted directories despite file access; no verified released fix.                                                                                   | Runtime compatibility defect. Test actual directory traversal, not merely file reads.                                                                              |
| [#123: Windows exemption lifetime](https://github.com/May-Agent/heel/issues/123)          | Overlapping proxied launches share a SID; first guard drop can remove exemption needed by remaining launch.                                                                           | Crate lifetime defect. Avoid overlapping proxied launches; not a fix for #152.                                                                                     |
| [#158: IPC connection exhaustion](https://github.com/May-Agent/heel/issues/158)           | Blocking frame reads lack deadlines and connection caps, consuming shared blocking-pool threads.                                                                                      | Broker availability defect. Omit IPC or impose an independently bounded service boundary.                                                                          |

Source anchors for the table: [Linux grants](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/landlock_rules.rs), [Landlock read mask](https://docs.rs/landlock/0.4.7/landlock/enum.AccessFs.html#method.from_read), [broker operations](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/broker/ops.rs), [Unix backend](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/mod.rs), [Windows limits](https://docs.rs/crate/heel/0.7.4/source/src/platform/windows/launch.rs), [IPC server](https://docs.rs/crate/heel/0.7.4/source/src/ipc/server.rs).

Other reports need more restrained interpretation. [#24](https://github.com/May-Agent/heel/issues/24) reports CONNECT/WebSocket handshake failures and proposes a pipelined-byte-loss explanation; that diagnosis is explicitly unconfirmed. [#10](https://github.com/May-Agent/heel/issues/10) reports an undriven default executor, but 0.7.4 uses `SmolGlobal` initialization and includes a regression test for running spawned tasks, so its original code-path explanation is not evidence that this release still has that cause. A November 3 development commit adds the macOS trust-evaluation agent; it is **after** the examined release and cannot be counted as a 0.7.4 TLS compatibility fix. [Released executor path](https://docs.rs/crate/heel/0.7.4/source/src/sandbox.rs), [development history](https://api.github.com/repos/May-Agent/heel/commits?per_page=5)

The public repository security-advisories API returned an empty list; no heel-specific formal advisory was identified in this investigation. This is neither a dependency audit nor evidence that the crate is secure. The open defects above matter regardless of advisory issuance. [Public advisories API](https://api.github.com/repos/May-Agent/heel/security-advisories)

### Ordering, inherited authority, and deployment

- **Threads and irreversibility:** heel confines the workload, not the supervisor. Linux prepares rules/filters before forking, then applies limits/priority, Landlock, broker filter, and seccomp before exec. Broker launcher threads have their own confined Landlock domain. This design avoids constructing allocating policies in a multithreaded post-fork child. Do not move enforcement into arbitrary async worker threads or add allocating `pre_exec` callbacks. Descendants inherit restrictions; they cannot “drop” their Landlock or seccomp layer. [Linux ordering](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/mod.rs), [broker launcher](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/broker/mod.rs), [kernel inheritance](https://docs.kernel.org/userspace-api/landlock.html)
- **Descriptors and handles:** restrictions do not revoke previously acquired authority. Unix non-CLOEXEC descriptors and already-connected sockets are not proven sanitized by heel. Windows uses an explicit standard-handle inheritance list, but those handles are still authority you intentionally pass. Prefer null input and controlled pipes; do not inherit terminals or broker/control sockets casually. Windows 0.7.4 selects one stdio mode from stdout for all three streams, another mismatch with the independent public setters. [Command configuration](https://docs.rs/crate/heel/0.7.4/source/src/command.rs), [Windows stdio](https://docs.rs/crate/heel/0.7.4/source/src/platform/windows/mod.rs), [Windows handle list](https://docs.rs/crate/heel/0.7.4/source/src/platform/windows/launch.rs)
- **Paths versus objects:** Landlock rules hold object references and can follow the object through a rename; macOS SBPL uses path matching; Windows ACL changes apply to objects and inherited entries. `resolve_paths` and no-follow checks address planted-link redirection, not universal filesystem snapshot isolation. Keep input trees disjoint from writable trees, control their parents, and test hard links, symlinks, mounts and Windows junctions. Trusted host processes can still mutate shared input. [Resolver](https://docs.rs/crate/heel/0.7.4/source/src/resolve.rs), [Linux object anchors](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/landlock_rules.rs), [Windows ACLs](https://docs.rs/crate/heel/0.7.4/source/src/platform/windows/acl.rs)
- **Runtime access:** grant the executable, loader, libraries, certificates, interpreter and necessary toolchain data, not merely source files. The library adds platform baselines, not a hermetic runtime. An explicit execute grant to an interpreter means readable code can be interpreted even when direct execution is denied. Windows resolves/grants a program at launch; avoid attacker-controlled executable search paths. [Windows program grants](https://docs.rs/crate/heel/0.7.4/source/src/platform/windows/mod.rs), [Linux baselines](https://docs.rs/crate/heel/0.7.4/source/src/platform/linux/landlock_rules.rs)
- **Lifecycle ownership:** ordinary Unix spawn uses process groups; daemonization/group escape needs stronger ownership. Linux `supervised_spawn(true)` plus `Command::spawn_supervised` adds a separate heel helper and returns tree reports; it is opt-in and Linux-only. Its error variants include `SupervisorNotConfigured`, `SupervisedSpawnFailed`, `SupervisorLost`, and `SupervisorFailed`; the last two explicitly permit uncertainty about remaining processes. It is not a replacement for cgroup isolation. Windows jobs kill members on close. [Supervisor API](https://docs.rs/crate/heel/0.7.4/source/src/supervisor/mod.rs), [helper configuration](https://docs.rs/crate/heel/0.7.4/source/src/config.rs), [errors](https://docs.rs/crate/heel/0.7.4/source/src/error.rs)
- **Host state:** Windows profile creation, ACL grants, named persistent identities and firewall exemptions modify host state. RAII cleanup cannot promise crash-proof rollback. Persistent `Identity::prepare()` grants intentionally outlive an individual sandbox. Use a dedicated account/disposable machine for adversarial integration tests and inspect cleanup. [Identity](https://docs.rs/crate/heel/0.7.4/source/src/identity.rs), [Windows container lifecycle](https://docs.rs/crate/heel/0.7.4/source/src/platform/windows/container.rs)

## Complete Rust examples and verification

These examples show **configuration and limited behavioral probes**, not a repaired hostile-code boundary. A product requiring “no host mutation outside workspace, no signaling host processes, complete process-tree ownership, and approved destinations only on every OS” must reject heel 0.7.4 before launching its workload. There is no honest example satisfying that combined requirement using this release alone.

Use a new standalone Cargo project. Save the first Rust program as `src/main.rs` and the second as `src/bin/network.rs`:

```toml
[package]
name = "heel-policy-probe"
version = "0.1.0"
edition = "2024"

[dependencies]
heel = { version = "=0.7.4", default-features = false }
smol = "=2.0.2"
```

No backend feature or target-specific Cargo section is necessary for these examples. Build natively on each intended OS. Keep `Cargo.lock`, use `--locked` thereafter, and supply the native requirements in the matrix. Neither example configures IPC or Linux supervised spawn, so neither needs the heel executable helper. Those features require installing a matching trusted binary, normally with `cargo install heel --version 0.7.4 --locked`, and arranging `heel_binary(...)`/helper discovery; embedding the library alone does not install it. [Helper lookup](https://docs.rs/crate/heel/0.7.4/source/src/sandbox.rs), [helper settings](https://docs.rs/crate/heel/0.7.4/source/src/config.rs)

### Shared read-only input, generated writable workspace, and DenyAll

Run this only with a **disposable regular input file** and a trusted binary. It launches itself as a probe, reads the input, copies it into the sandbox's generated workspace, requires a permission-denied error opening the input for writing, and rejects a successful direct connection to a live host loopback listener. It does not test all socket families or metadata writes. There is no shell interpolation.

```rust
use heel::{Access, PathRequest, Sandbox, SandboxConfig, StdioConfig, resolve_paths};
use std::{error::Error, fs::OpenOptions, path::PathBuf, time::Duration};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--probe") {
        let input = args.get(2).ok_or("missing input")?;
        let address = args.get(3).ok_or("missing address")?.parse()?;
        let bytes = std::fs::read(input)?;
        std::fs::write("result.bin", bytes)?;
        match OpenOptions::new().write(true).open(input) {
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {}
            Err(e) => return Err(e.into()),
            Ok(_) => return Err("input unexpectedly writable".into()),
        }
        if std::net::TcpStream::connect_timeout(&address, Duration::from_secs(2)).is_ok() {
            return Err("unexpected direct loopback connection".into());
        }
        return Ok(());
    }
    if args.len() != 2 {
        return Err("usage: heel-policy-probe ABSOLUTE_INPUT_FILE".into());
    }
    smol::block_on(run(PathBuf::from(&args[1])))
}

async fn run(input: PathBuf) -> Result<()> {
    // Resolve caller spellings once; the input must be a disposable fixture.
    let paths = resolve_paths([
        (PathRequest::readable(input), Access::READ),
        (PathRequest::readable(std::env::current_exe()?), Access::EXEC),
    ])?;
    let input = paths[0].0.as_path().to_str().ok_or("non-UTF-8 input")?.to_owned();
    // Confirm the host can write it, so a later denial is a meaningful control.
    drop(OpenOptions::new().write(true).open(&input)?);
    let executable = paths[1].0.as_path().to_str().ok_or("non-UTF-8 executable")?.to_owned();
    let mut builder = SandboxConfig::builder().filesystem_strict(true);
    for (path, access) in paths {
        builder = builder.grant(path, access);
    }
    // DenyAll is the default; no IPC and no network proxy are configured.
    let sandbox = Sandbox::with_config(builder.build()).await?;
    if sandbox.proxy_url().is_some() || !sandbox.config().network_deny_all() {
        return Err("offline configuration was not retained".into());
    }
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?.to_string();
    let command = sandbox.command(executable)
        .args(["--probe", &input, &address])
        .stdin(StdioConfig::Null)
        .stdout(StdioConfig::Null)
        .stderr(StdioConfig::Inherit);
    // Use the tracked spawn path, avoiding the Unix output() cleanup defect.
    let mut child = command.spawn().await?;
    if !child.wait().await?.success() {
        return Err("policy probe failed; refuse to run a workload".into());
    }
    if std::fs::read(sandbox.working_dir().join("result.bin"))? != std::fs::read(input)? {
        return Err("workspace output mismatch".into());
    }
    // This smoke probe intentionally launches no untrusted workload afterward.
    Ok(())
}
```

The `?` paths stop on setup and spawn errors; a failed child/probe stops the parent. A missing runtime/loader, denied executable, unavailable Linux ABI or Windows ACL failure is a failure, not a reason to run normally. The direct-connect check only proves no successful connection in that probe; its returned error is not independently attributed to a security mechanism. The parent deliberately keeps its listener alive so a refused connection is not simply a closed-port test.

| OS      | What this example can demonstrate if it passes                                                                                         | What it cannot establish                                                                            |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| macOS   | Actual child ran; input content readable and not opened writable; generated workspace writable; that direct loopback connection failed | Project-only reads, complete IPC/process isolation, portable quotas, or descendant cleanup          |
| Linux   | Same operations under ABI-4 Landlock/seccomp; default DenyAll requires no broker                                                       | Metadata immutability or correct no-execute semantics for explicit grants; the known defects remain |
| Windows | Same selected operations under AppContainer, with no Internet capability; no filtered-proxy setup needed                               | Arbitrary directory usability, total absence of ambient package access, or destination filtering    |

A writable input already permitted by another grant/baseline should fail the probe. This is useful feedback, not automatic policy repair. The test opens for writing without truncation so that an unexpected grant does not overwrite the fixture. The input path and executable path must be representable as UTF-8 because this command API takes strings. Use controlled ASCII test paths initially and test real deployment paths separately.

### Host-and-port policy, refusing Windows

This separate program permits proxy requests only for `example.com:443`. It requires `/usr/bin/curl`, uses an explicit timeout, writes the response in the workspace, and fails on setup or command failure. It refuses Windows **before sandbox construction** because that backend cannot enforce the stated destination rule in 0.7.4. Replace the hostname with an application-owned test endpoint before adoption.

```rust
use heel::{DomainRequest, NetworkPolicy, Sandbox, SandboxConfig, StdioConfig};
use std::error::Error;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
struct ApiOnly;
impl NetworkPolicy for ApiOnly {
    async fn check(&self, request: &DomainRequest) -> bool {
        request.host() == "example.com" && request.port() == 443
    }
}

fn main() -> Result<()> {
    smol::block_on(run())
}

async fn run() -> Result<()> {
    if !cfg!(any(target_os = "macos", target_os = "linux")) {
        return Err("heel 0.7.4 cannot enforce this destination policy here".into());
    }
    let sandbox = Sandbox::with_config(
        SandboxConfig::builder().network(ApiOnly).build(),
    ).await?;
    if sandbox.proxy_url().is_none() {
        return Err("required proxy was not configured".into());
    }
    // Requires curl at this exact trusted system path; no PATH fallback.
    let command = sandbox.command("/usr/bin/curl")
        .args(["--fail", "--silent", "--show-error", "--max-time", "10",
                  "--output", "response.txt", "https://example.com/"])
        .stdin(StdioConfig::Null)
        .stdout(StdioConfig::Null)
        .stderr(StdioConfig::Inherit);
    let mut child = command.spawn().await?;
    if !child.wait().await?.success() {
        return Err("network probe failed; refuse to proceed".into());
    }
    Ok(())
}
```

On macOS this exercises the SBPL proxy endpoint permission. On Linux it exercises the connect broker as well as Landlock/seccomp; Yama or notification restrictions must produce errors rather than an unsandboxed retry. This example is not IP pinning, service authentication, protection from allowed-service exfiltration, or enforcement of HTTPS inside an opaque CONNECT stream. Curl validates TLS normally; the sandbox policy only checks the requested host and port. It also does not close the proxy's host-local-address defect. For a malicious client, test direct connections to denied destinations independently; the successful curl call alone proves only the permitted path worked.

### What was actually verified

Both programs above passed **`cargo check --bins --locked`** using the published registry `heel = 0.7.4`, `default-features = false`, and smol 2.0.2, on **macOS 27.2**, with **rustc 1.98.1** and **Cargo 1.98.1**. Host identity was obtained with `sniff os --json`. This was a native macOS compile check, not cross-compilation. **Neither program was executed, and no Linux, WSL2, or native Windows enforcement test was performed for this report.** The checks establish API compatibility for that dependency resolution, not enforcement, runtime compatibility, or MSRV.

### Isolated verification plan before deployment

Use a fresh child launcher for each case, with an outer watchdog and disposable fixtures. Do not call irreversible restriction APIs in the test runner. Run without terminal/browser focus changes. Windows ACL/exemption tests belong in a disposable account/VM with recorded pre/post state, not on sensitive user directories.

| Test group         | Allowed control                                                                       | Denied/failure control and required observation                                                                                                                                                                                                      |
| ------------------ | ------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Files              | Read fixture; write/create/rename workspace files; list and enter granted directories | Read sibling secret, write/truncate/delete input, chmod/xattr outside grants, execute freshly written payload. Assert expected denial and unchanged fixture content/metadata. Known 0.7.4 gaps should be visible failures.                           |
| Network            | Approved proxy host/port with controlled server                                       | Denied host, denied port, direct IPv4/IPv6, UDP/QUIC, host interface address, mapped IPv6, preconnected socket. Observe server-side connection counts as well as client errors.                                                                      |
| Aliases and races  | Trusted resolved input/tool paths                                                     | Symlink/junction replacement between resolution and construction, hard links and overlapping grants; require `GrantRedirected` or demonstrated denial, never silently broaden access.                                                                |
| Capability absence | Fully configured test machine                                                         | Older/disabled Landlock, blocked seccomp notification, Yama 2/3, inaccessible ACL, missing helper, invalid SBPL; a workload start-marker must stay absent. Use isolated machines rather than changing the development host's security configuration. |
| Lifecycle          | Child plus grandchild under tracked spawn/supervisor/job                              | Cancellation, background child, group escape, helper death, overlapping Windows proxy launches; verify no surviving test process and restored temporary host state.                                                                                  |
| Native limits      | Workloads below each required threshold                                               | Exceed one threshold at a time. Reject unsupported limits up front; distinguish process limits from aggregate tree quotas.                                                                                                                           |

A probe can reveal inadequate enforcement; passing a finite probe suite is not proof that all malicious programs are confined. Record OS/kernel, architecture, crate/lockfile, enabled settings, setup errors, child exit statuses and server-side observations. WSL2 evidence validates only its guest Linux backend. Tests involving existing sockets or inherited handles should deliberately create those capabilities in the parent so they measure the real trust boundary.

## Alternatives and complementary layers

The neighboring [Landlock](landlock.md), [Seatbelt](seatbelt.md), and [AppContainer](app-container.md) reports supplied context and leads. The neighboring wrapper documents are mostly research prompts rather than completed evidence. Claims in this report were checked against upstream sources independently.

| Approach                                                                                                                                                 | Relationship and coverage                                                                                                          | Maintenance evidence and integration trade-off                                                                                                                                                                                                                        |
| -------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`skarn-sandbox` 1.0.1](https://docs.rs/skarn-sandbox/1.0.1/skarn_sandbox/)                                                                              | Competing Rust wrapper advertising Seatbelt, Landlock/seccomp and AppContainer/job backends; uses a worker-first confinement model | Published versioned API and backend design are inspectable. Separate source/defect review is required; 1.x numbering does not prove stronger enforcement. More explicit worker integration than heel's convenient command facade.                                     |
| [`rightkit-sandbox` 0.4.0](https://docs.rs/rightkit-sandbox/0.4.0/rightkit_sandbox/)                                                                     | Competing higher-level wrapper with resource authority, child ownership and PTY facilities; documents the three native mechanisms  | Published 0.4.0 API documents fail-closed unsupported confinement and narrower PTY support. `spawn_supervised` explicitly has no filesystem/network confinement. Evaluate its actual backend paths independently rather than treating supervision as sandboxing.      |
| Direct [`landlock` 0.4.7](https://docs.rs/landlock/0.4.7/landlock/), [`seccompiler`](https://docs.rs/seccompiler/0.5.0/seccompiler/), Win32, or Seatbelt | Per-OS bindings/integration, not one portable sandbox                                                                              | Versioned bindings and OS contracts make requested rights visible. Higher engineering cost: helper lifecycle, strict compatibility, ACL rollback, network mediation, and test matrix are yours. Can expose capabilities heel omits, but cannot remove OS limitations. |
| [`bubblewrap`](https://github.com/containers/bubblewrap) / [`nsjail`](https://github.com/google/nsjail)                                                  | External Linux launchers; namespaces and filesystem views, with additional policies/limits                                         | Public upstream implementations/configuration are inspectable. Adds executable deployment and kernel/runtime prerequisites. Neither is a native macOS/Windows backend; bwrap's boundary depends on its arguments. Useful Linux complement or replacement.             |
| [Wasmtime/WASI](https://docs.wasmtime.dev/security.html)                                                                                                 | Different execution boundary for code that can target Wasm and explicit host interfaces                                            | Documented security model; audit and constrain host imports. More portable authority control for plugins, but cannot run arbitrary existing native CLI binaries unchanged.                                                                                            |
| Dedicated OS account or VM/container boundary                                                                                                            | Complementary identity separation or stronger kernel boundary                                                                      | Operational ownership, image updates and resource policy replace some embedding convenience. Linux containers share a kernel; a VM separates guest kernel attack surface. Native hardware/tool access and filesystem integration cost more.                           |

The alternatives table compares documented designs, not security certification or a complete maintenance ranking. The two competing Rust wrappers need their own release, defect, and enforcement review before selection. Shared native mechanisms do not make their guarantees equivalent to heel or to each other.

**Recommendation:** heel can be useful for a controlled data-processing child with explicitly accepted baseline access and limitations, especially offline, after real target tests. Add another layer for secrets, adversarial code, aggregate resource accounting, strong process isolation, or precise egress. Choose a different boundary if equivalent hostile-workload guarantees across OSes are nonnegotiable; 0.7.4 has no portable fail-closed capability contract that can supply them.

## Authoritative resources and unresolved adoption questions

- [Registry release history](https://index.crates.io/he/el/heel), [published 0.7.4 source](https://docs.rs/crate/heel/0.7.4/source/), [immutable release commit](https://github.com/May-Agent/heel/tree/9fc026c776e4b541f114f0cff988d8272fbaa235), and [version-specific API documentation endpoint](https://docs.rs/heel/0.7.4/heel/). The browser could not retrieve that rustdoc endpoint during research; exported APIs were checked in the published source and by compilation instead.
- [Release changelog](https://docs.rs/crate/heel/0.7.4/source/CHANGELOG.md), [current issue tracker](https://github.com/May-Agent/heel/issues), and [public advisories](https://api.github.com/repos/May-Agent/heel/security-advisories).
- [Linux Landlock contract](https://docs.kernel.org/userspace-api/landlock.html), [seccomp contract](https://docs.kernel.org/userspace-api/seccomp_filter.html), [Apple App Sandbox](https://developer.apple.com/documentation/security/app-sandbox), [Microsoft AppContainer](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation), and [Windows Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects). Current kernel documentation describes newer ABIs too; heel 0.7.4 requests ABI 4, not every capability documented there.

Before adoption, resolve these material questions:

1. Which released version fixes the confirmed Linux mapping/broker and Windows network/limit gaps, and what real-platform negative tests cover them?
2. What are the actual publisher roster, supported MSRV, sustained native-platform support commitments, and security-reporting process after the repository transfer?
3. Can the product accept the system-read/IPC/hardware baseline and same-user signaling exposure, or does it require a stronger boundary?
4. Can supported Windows installations perform the needed ACL/profile/firewall operations without changing the product's privilege model, and are grant/exemption lifetimes safe under crashes and concurrent runs?
5. Which real agent/tool runtimes work with this release's proxy and macOS profile, including TLS and WebSocket transports, and when will post-release compatibility fixes be published?
