---
prompt: |-
    Research the Rust `nono` crate to help a developer decide whether
    and how to use it in a sandbox launcher supporting macOS and Linux.

    The starting point is capability-based sandboxing backed by Linux
    Landlock and macOS Seatbelt. Distinguish the nono library from its CLI,
    profiles, proxies, supervisors, and language bindings; verify which
    features actually belong to the published Rust dependency.

    Focus on the published crate's public API, enforcement guarantees,
    compatibility behavior, and practical integration. Explain native OS
    concepts where needed without turning this into a general sandbox survey.

    Evidence and scope:
    - State the research date and exact stable crate version examined.
    - Verify identity through published package metadata and its linked
      repository. If the named crate cannot be identified, report the search
      and uncertainty; do not invent APIs or silently research a substitute.
    - Use primary sources: version-specific API documentation, published
      source, release notes, repository history, and official documentation
      for the underlying operating-system mechanisms. Cite claims nearby.
    - Treat the starting links and neighboring research as leads. Independently
      verify relevant claims; do not treat neighboring reports as primary evidence.
    - Distinguish released functionality from unreleased changes, proposals,
      documentation claims, and your own recommendations. Explain conflicting
      or unavailable evidence. Do not infer security from a successful launch.

    Starting links:
    - https://docs.rs/nono/
    - ../os-support/landlock.md
    - ../os-support/seatbelt.md

    Cover the following topics:

    1. Identity, releases, and maintenance
       - Identify the package, repository, maintainers, license, and any parent
         project. Confirm it is published and usable as a dependency; identify
         relevant yanked or prerelease versions.
       - Verify the latest stable version and release date. Summarize releases
         from the past 24 months and older milestones needed to explain the API.
         Use dates to describe cadence rather than impressions.
       - Explain API stability, breaking changes, migration considerations,
         minimum supported Rust version, and important native dependencies.
         Do not assume major releases exist or equate pre-1.0 with unreliability.
       - Assess maintenance using observable evidence; label judgments as such.

    2. Public API and enforcement model
       - Explain capability sets, access modes, path validation, network modes,
         support queries, policy preparation, and sandbox application.
       - Describe defaults, grant composition, inheritance, irreversibility,
         thread scope, privileges, and the boundary between mechanism and policy.
       - Identify filesystem, networking, process/IPC, broker, and diagnostic
         capabilities actually exported by the library, separately from CLI UX.
       - List Cargo features, defaults, backend dependencies, companion packages,
         and any helpers needed for particular guarantees.

    3. Platform integration, compatibility, and failure behavior
       - Provide a macOS/Linux matrix of released controls, native enforcement,
         minimum OS/kernel requirements, privileges, and architectures.
       - Explain Landlock ABI detection, strict versus partial enforcement,
         unavailable Landlock, and any seccomp or namespace fallback. Determine
         whether automatic selection changes the effective security contract.
       - Verify Seatbelt API availability, deprecation, profile generation,
         escaping, signing/entitlements, and distribution requirements.
       - Trace unsupported capability requests, missing paths, backend errors,
         and partial setup. Can callers inspect and require effective controls?
       - Distinguish in-process application from a prepared child launch; analyze
         multithreaded callers and any allocation-free child-side enforcement.
       - Treat WSL2 as Linux subject to kernel capabilities. Verify native Windows
         support independently; do not infer it from CLI installation instructions.
       - Separate direct network enforcement from any proxy/broker that needs
         surrounding CLI or supervisor integration.

    4. Practical uses, limitations, and pitfalls
       - Describe supported uses and who they protect against. Distinguish
         accidental access reduction from confinement of malicious workloads.
       - Evaluate read-only inputs, a writable workspace, no networking, and
         access only to approved destinations. State exactly which parts the
         crate can enforce and which require another mechanism.
       - Examine enforcement ordering, multithreading, inherited descriptors or
         handles, existing sockets, descendants, executable/runtime access,
         filesystem aliases, and IPC paths where they affect claimed guarantees.
       - Classify material gaps as underlying OS limitations, native capabilities
         missing from the crate, integration/deployment constraints, or needs
         for another isolation mechanism. Explain consequences and mitigations.
       - Include documented defects or advisories material to the examined
         version, with their status and applicability. Absence of an advisory
         does not establish security.
       - Examine path aliases, runtime/system path grants, existing descriptors
         and sockets, Unix IPC, process inspection, and descendant inheritance.
       - For destination allowlists or credential protection, verify that direct
         egress, local endpoints, DNS, redirects, and proxy bypasses are covered.
       - Distinguish library guarantees from CLI defaults, rollback facilities,
         approval workflows, and marketing claims about impossible access.

    5. Rust examples and verification
       - Demonstrate a read-only-input/writable-workspace capability set,
         explicit network restrictions, support detection, and strict refusal.
       - Identify each example's effective guarantees on both backends, and show
         prepared child enforcement if the release supports that workflow.
       - State where a broker, CLI component, or separate mechanism is required
         instead of inventing equivalent controls in the library.
       - Include complete, focused Rust examples with Cargo configuration pinned
         to the examined release, required features, platform setup, and helpers.
       - Show meaningful error handling and fail-closed behavior for each
         example's stated requirements. Explain allowed and denied operations
         and identify guarantees requiring caller validation or another mechanism.
       - Compile-check when tooling permits. Label examples as compiled,
         executed, or unverified, naming the environment and toolchain tested.
         Compilation and cross-compilation do not establish enforcement.
       - Describe isolated child-process tests for allowed and denied operations,
         unavailable capabilities, and launch failures. Keep the supervisor/test
         runner unrestricted and avoid persistent host security changes; use a
         disposable environment when a probe cannot be safely cleaned up.

    6. Alternatives and complementary tools
       Compare procage, other macOS/Linux Rust sandbox wrappers, direct
       Landlock/Seatbelt integration, and stronger isolation where appropriate.
       Distinguish competing libraries from surrounding policy and proxy tooling.
       Distinguish competing bindings, higher-level wrappers, complementary
       mechanisms, and stronger boundaries. Compare platform coverage,
       maintenance, integration cost, and when another layer is needed.

    Presentation:
    - Begin with a concise suitability assessment and the main caveats.
    - Use well-formed CommonMark and capability/comparison tables where helpful.
      Use Mermaid only when a diagram clarifies policy or launch flow.
    - End with authoritative resource links and unresolved questions that
      materially affect adoption.
os: [macOS, Linux]
hash: b2a807f13757d1c8-a3595c80efc51c63
last_updated: 2026-11-04
---

# nono for a macOS and Linux sandbox launcher

**Assessment:** `nono` 0.81.0 is a usable Rust dependency for filesystem confinement and selected network/process controls on macOS and Linux. Adopt it as an enforcement component behind a launcher-owned policy and launch protocol. It is not a complete portable child-process sandbox: Linux defaults can leave UDP and Unix IPC available, destination restrictions need a serviced broker or network isolation, and macOS relies on deprecated Seatbelt entry points. Do not interpret `apply_auto()` succeeding as proof that the intended policy is fully enforced. [Published API][api], [Linux implementation][linux], [macOS implementation][mac]

**Research date: November 4, 2026. Examined stable release: 0.81.0, published October 29, 2026.** “Stable” here means a non-prerelease registry version, not a promise of a frozen API. The downloadable crates.io package, normalized manifest, and `.cargo_vcs_info.json` identify repository commit `b2d859deeb7a65f9ca5b334eb8726637ba76c866`, under `crates/nono`. This report uses that published source and versioned documentation; repository `main`, proposal documents, CLI instructions, and neighboring research are not substitutes for it. [Package metadata][pkg], [published manifest][cargo], [release][release]

## Identity, history, and maintenance

The current package is **nono**, Apache-2.0, authored in its manifest by **Luke Hinds**, within the **Nolabs nono** project at `nolabs-ai/nono`. The tagged maintainer council lists Luke Hinds, Aleksy Siek, Leonardo Zanivan, and James Carnegie. Earlier history uses `always-further/nono`; this is the project's earlier repository identity, not a second sandbox dependency. [Manifest][cargo], [maintainers][maintainers], [tagged changelog][changes]

There is a significant package-name trap: **nono 0.0.1**, published May 14, 2022 and now yanked, was Hisbaan Noorani's GPL-3.0 CLI for undoing commands at `hisbaan/nono`. Its published manifest does not describe this sandbox. Do not credit the current sandbox with maintenance since 2022. The registry index also marks **0.65.0** yanked; **0.65.1** is its subsequent patch release. Published prereleases include **0.6.0-alpha.3** and **0.44.0-alpha.1**. The index contains 97 version records at examination, with 0.81.0 the highest stable, unyanked version. [Old published manifest](https://docs.rs/crate/nono/0.0.1/source/Cargo.toml.orig), [registry index][index], [version history][pkg]

### Release cadence and migration

Within the 24-month window November 4, 2024–November 4, 2026, the sandbox's published history begins in February 2026; no 2024–2025 sandbox releases appear in the registry history. Repository release tags and crate publication dates are different events: repository v0.1.0 was released February 1, while the crate history dates 0.1.0 to February 16. Similarly, the crate lists 0.6.0 on February 17, while its GitHub release is February 27. Use registry dates when selecting a dependency. [Crate history][pkg], [repository release API](https://api.github.com/repos/nolabs-ai/nono/releases?per_page=100&page=2)

| Published period/version                  | Relevant development                                                                                                                                                |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| February–April 2026: 0.1.0 through 0.45.0 | Numerous minor releases; the modern capability API and surrounding CLI/binding system developed during this period. Old README examples are not a migration guide.  |
| May–June 2026: 0.46.0 through 0.66.0      | 0.55.0 (May 17) introduced explicit pathname Unix-socket capabilities and Linux supervisor mediation. 0.65.0 (June 23) is yanked; 0.65.1 was published that day.    |
| July 2026: 0.67.0 through 0.71.0          | Releases July 6, 14, 20, 29, and 31, with a 0.67.1 patch on July 6.                                                                                                 |
| 0.72.0, August 7                          | Added the explicit Linux static seccomp network baseline.                                                                                                           |
| 0.73.0, August 10; 0.74.0, August 19      | Proxy/security fixes and seccomp architecture guards; see advisory qualifications below.                                                                            |
| September 1, 9, 11, 16, 30                | 0.75.0–0.79.0; Unix-socket and profile-policy hardening continued.                                                                                                  |
| 0.80.0, October 13                        | CLI/proxy and policy fixes.                                                                                                                                         |
| **0.81.0, October 29**                    | Released opt-in Linux namespace primitives; detects Landlock through ABI 9 while intentionally enforcing at most ABI 6; blocks `io_uring` during AF_UNIX mediation. |

Dates and changes above are from the [published version list][pkg] and [tagged changelog][changes]. They describe frequent releases over nine months, not years of production maturity. **Maintenance judgment:** observable release activity, multiple named maintainers, Linux/macOS CI, and public vulnerability fixes are positive evidence of active maintenance. Repeated security corrections and stale examples mean adoption still needs source review and backend tests. Neither pre-1.0 numbering nor release frequency establishes security. [Tagged CI](https://github.com/nolabs-ai/nono/blob/v0.81.0/.github/workflows/ci.yml), [advisories][advisories]

The normalized 0.81.0 manifest declares **Rust 1.95**, edition **2024**. There is no 1.x stable API to migrate to. Cargo considers different `0.x` minor lines potentially incompatible; pin the examined version and review upgrades. In particular, older README calls such as `allow_read`, `allow_write`, `allow_execute`, or `Sandbox::apply` do not describe this release's public API. Use `allow_path`/`allow_file`, `AccessMode`, and the platform-specific apply methods. Linux `apply_auto` returns `Result<SeccompNetFallback>`; macOS returns `Result<()>`. Several backend methods are `cfg`-gated. [Manifest][cargo], [capability API][cap], [sandbox API][sandbox]

## What is actually in the Rust dependency?

The library calls itself a policy-free primitive, meaning the caller chooses grants rather than inheriting the CLI's profiles and protected-directory lists. That does **not** mean the backend has no built-in choices: its Seatbelt profile includes execution, Mach, shared-memory, DNS, and system-service rules. Review the generated-policy implementation. [Library exports][lib], [Seatbelt generator][mac]

| Layer                    | Shipped in `nono` 0.81.0                                                                                                                  | What remains outside a simple library call                                                                |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Filesystem capabilities  | `CapabilitySet`, `FsCapability`, `AccessMode`, sources, deduplication and coverage queries                                                | Selecting safe runtime/system paths; protecting secrets inside a broad grant; workload lifecycle          |
| Network mechanism        | `NetworkMode`, port grants, `block_dns`; Linux seccomp options and notification primitives                                                | HTTP proxy server, domain/endpoint policy enforcement, credential injection and notification event loop   |
| Process/IPC              | `SignalMode`, `ProcessInfoMode`, `IpcMode`, Unix-socket capabilities; Linux namespace primitives                                          | Portable process isolation, exhaustive IPC isolation, descendant cleanup                                  |
| Runtime mediation        | `supervisor` IPC/listener/request/approval types; Linux notification helpers; macOS extension issue/consume/release                       | Full supervisor, trusted approval decisions, safe file opening and FD injection, CLI interception/shims   |
| Diagnostics              | Structured denial/report types, state serialization, permission-query API, macOS denial logging switch                                    | A kernel-wide denial collector or proof that all denied operations were observed                          |
| Other exported utilities | `HostFilter`/`net_filter`, broker PATH sanitization, keystore, trust, audit, undo snapshots, scrubbing, manifests and resource-limit data | CLI rollback/session workflows, profile registry, resource supervisor/cgroup setup, proxy deployment      |
| Companion products       | Separate `nono-cli`, `nono-proxy`, C binding package and language integrations                                                            | Installing `nono` does not install their launch protocol or provide the Python SDK's `sandboxed_exec` API |

`SandboxState`, coverage queries and structured denial records describe declared policy or observations supplied by the caller; they do not read back a complete kernel-effective policy or certify absence of denials.

This inventory is verified against [0.81.0 exports][lib] and [backend exports][sandbox], rather than the project's feature marketing. The 0.81.0 changelog explicitly removes references to archived TypeScript and Go bindings; do not use an old SDK README as the Rust library contract. [Changelog][changes]

**Cargo features:** the only declared feature switch is `system-keyring`, enabled by default and enabling optional `keyring` 4 with its `v1` feature. Disabling default features removes that dependency, not the audit/trust/undo modules. Linux depends on `landlock` 0.4.7 and `nix` 0.31.3; macOS uses `nix` and direct `sandbox_init`/extension FFI through the OS. Both use `libc`; build-time schema generation pulls in `typify`, `syn`, and `prettyplease`. There is no `seccomp`, `namespace`, or “minimal sandbox” feature flag, and no external `libseccomp` dependency. Key-store helpers may need their referenced credential tools/services; those are unnecessary for basic confinement. [Full published dependency/feature manifest][cargo], [keystore source](https://docs.rs/crate/nono/0.81.0/source/src/keystore.rs)

## Capability and enforcement model

### Grants and validation

`CapabilitySet::new()` has **no filesystem grants**, **network allowed**, default signal/process-info modes `Isolated`, default IPC `SharedMemoryOnly`, DNS enabled, extensions disabled, and no resource ceilings. `allow_path` grants a directory tree; `allow_file` grants one non-directory node, which can include a device, FIFO, or socket inode. Filesystem access to a socket node is distinct from permission to connect to it. Directory grants compose additively: a writable parent defeats an intended read-only child. Ordinary filesystem capabilities are grants, not deny exceptions. [Capability implementation][cap]

`AccessMode::{Read, Write, ReadWrite}` is intentionally coarse. Linux **Read includes execute**; Write includes creation, deletion, linking/renaming and truncation rights available at the enforced ABI. ReadWrite is their union. A read-only input may therefore still contain runnable code. Linux `restrict_execute` can stack a further execute-only Landlock layer and requires full enforcement of that layer; it is not a portable execute allowlist. `allow_command`/`block_command` are CLI policy metadata, not kernel executable filtering. [Linux rights mapping][linux], [capability fields][cap]

Constructors canonicalize and validate node types, returning `PathNotFound`, `PathCanonicalization`, `ExpectedDirectory`, or `ExpectedFile`. Create the workspace before building capabilities; granting a missing future output file is not supported through `allow_file`. Grant its existing parent directory if file creation is intended. Both original and resolved paths are retained. Linux preparation opens paths and derives type/device permissions from the same opened FD. This improves rule-setup consistency but is not a general proof against all ancestor replacement or hostile filesystem races; canonicalization and later opening remain separate operations. Do not accept mutable attacker-controlled policy paths. [Constructors][cap], [opened-rule preparation][linux]

On macOS, path strings escape backslashes and quotes and reject control characters; socket-directory regexes have separate escaping. The generator accounts for original/canonical spellings, symlink targets and ancestor metadata, including aliases such as `/tmp` → `/private/tmp`. Raw `platform_rule` strings are validated but then inserted into the profile; Linux ignores them. Only trusted launcher policy should supply them. There is no public cross-platform prepared-profile/report object exposing the exact effective Seatbelt rules. [Seatbelt source][mac], [platform-rule validation][cap]

### Restriction is inherited, with important exceptions

Landlock restricts the **calling thread** and its future descendants, intersects with other security layers, and does not upgrade sibling threads already running. Nono caps enforcement at ABI 6 even when detection reports ABI 8 thread synchronization support. Its raw `landlock_restrict_self` and seccomp install use flags `0`, not a thread-synchronizing option. Do not apply in one Tokio worker and assume the whole application is confined. Use a fresh single-threaded helper or the prepared child path. [Linux source][linux], [kernel threading/inheritance rules][kernel]

Seatbelt application is process-wide and inherited across fork/exec. Nono's macOS apply path allocates a profile and calls `sandbox_init`, so it is unsuitable for an arbitrary multithreaded parent's allocation-sensitive `pre_exec` closure. A fresh helper should finish trusted setup, apply, then execute the workload. Calling `Sandbox::apply_auto` in the supervisor itself also confines that supervisor irreversibly. [macOS apply source][mac], [Rust pre-exec contract](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html#tymethod.pre_exec)

“No expansion” needs qualification. Fixed Landlock domains cannot be relaxed; a supervisor can instead open resources outside the domain and pass FDs into it. macOS `enable_extensions` deliberately adds rules for consumable sandbox extension tokens, with public `extension_issue_file`, `extension_consume`, and `extension_release` helpers. These can grant additional access after initial application. Keep extensions off for a fixed-policy design; authenticate and constrain any granting broker. [Backend exports][sandbox], [macOS extension implementation][mac]

## Platform and compatibility matrix

| Requirement                                 | Linux / WSL2                                                                                                                                                        | macOS                                                                                                                                                                        |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Basic filesystem confinement                | Landlock LSM, kernel 5.13 / ABI 1 minimum; kernel must actually enable it                                                                                           | Seatbelt via direct `sandbox_init` with a generated raw SBPL profile                                                                                                         |
| Truncation / cross-directory rename control | ABI 3 / Linux 6.2 for truncation; ABI 2 / 5.19 for `REFER`                                                                                                          | Filesystem rules in profile; different pathname semantics                                                                                                                    |
| TCP bind/connect ports                      | ABI 4 / Linux 6.7; port only, no address matching                                                                                                                   | General port-only API requests return `NetworkFilterUnsupported`; localhost proxy destination rules are supported                                                            |
| UDP/raw/non-IP network denial               | Explicit seccomp baseline required for a consistent restriction on newer Landlock kernels                                                                           | Generated restricted socket/network rules; use `block_dns()` to remove resolver exception                                                                                    |
| Signal / abstract Unix socket domain scopes | ABI 6 / Linux 6.12; see strict/degrading modes below                                                                                                                | Signal self/same-sandbox rules; pathname Unix sockets have explicit profile rules                                                                                            |
| Pathname Unix socket allowlist              | Exported capabilities alone do not enforce it; needs seccomp notification supervisor. Native ABI 9 control is **not used**                                          | Exact, direct-child directory, or recursive subtree socket grants are generated separately from file grants                                                                  |
| Process inspection                          | `ProcessInfoMode` is a no-op; `/proc` grants and native ptrace constraints are separate                                                                             | Self/same-sandbox inspection by default; `AllowAll` widens it                                                                                                                |
| Namespaces                                  | Released opt-in user + network/IPC/UTS primitives; not automatically entered by apply methods                                                                       | No equivalent nono namespace backend                                                                                                                                         |
| Privileges                                  | Normal unprivileged use with `no_new_privs`; seccomp/LSM/container policy can still refuse. Namespace use additionally needs permitted unprivileged user namespaces | No root required for ordinary self-application; existing sandbox/TCC/OS policy can impose further restrictions                                                               |
| OS minimum and distribution                 | Kernel version is insufficient without probing configuration and backing filesystem                                                                                 | README says 10.5+, but modern profile operations, Rust deployment target, and actual release testing determine practical minimum; 10.5 is not a validated deployment promise |
| Architectures                               | Source seccomp audit/syscall guards cover x86-64 and AArch64; do not assume every Linux Rust target works                                                           | Native Intel/Apple Silicon builds need separate deployment evidence; examples here tested only on Apple Silicon                                                              |
| Native Windows                              | `support_info().is_supported == false`; `apply_auto` is not defined for Windows in this release                                                                     | Not applicable                                                                                                                                                               |

Backend-specific facts come from [Linux][linux], [macOS][mac], [namespace source][ns], and [public conditional compilation][sandbox]. Kernel ABI introduction dates are independently documented by the [Rust Landlock ABI reference](https://docs.rs/landlock/0.4.7/landlock/enum.ABI.html). Windows installation instructions for a CLI or WSL distribution are not evidence of a native Windows enforcement backend. The crate-level prose says unsupported platforms return an error, but the actual `apply_auto` definitions are Linux/macOS-only: portable callers need compile-time guards. [Exports][sandbox]

### Linux: support, automatic selection, and strict refusal

`detect_abi` probes using hard-required feature sets, knows ABI 1–9, and exposes `DetectedAbi` feature queries. **Detected is not enforced:** `MAX_ENFORCED_ABI` is **V6**, and `enforced_abi()` clamps newer detections. Consequently ABI 9 pathname-socket restrictions, ABI 8 thread synchronization and newer native kernel controls do not become library guarantees merely because the host supports them. `DetectedAbi::new` can construct a value; it is not itself a host probe. Use `Sandbox::detect_abi`. [Detection and cap][linux]

| Apply choice                                          | Effective network contract                                                                                                                                                                                                                            |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `apply_landlock`                                      | Filesystem/process Landlock; requested network policy requires ABI 4+. Its network restrictions cover TCP only.                                                                                                                                       |
| `apply_auto` or `SeccompOpts::network_fallback()`     | Same Landlock rules; on ABI <4 installs full-block seccomp inline for a plain blocked policy, or returns a `ProxyOnly` fallback requiring later installation and servicing. On ABI ≥4 it does **not** automatically add the stronger socket baseline. |
| `apply_seccomp(..., SeccompOpts::network_baseline())` | Adds static denial of UDP, raw/non-IP socket families and `io_uring_setup` for restricted policies. Plain blocked policy denies new non-Unix sockets; TCP-port policies allow Internet TCP stream sockets and rely on Landlock for ports.             |
| `external_tcp()` / `external_tcp_with_baseline()`     | Explicitly delegates TCP filtering to the caller/deployment; the latter still adds a TCP-only socket baseline. Neither proves external filtering exists.                                                                                              |
| `apply_external()`                                    | A no-op marker. It does not apply filesystem or network isolation.                                                                                                                                                                                    |

These are [released implementation choices][linux], not proposed backends. Seccomp is a network complement/fallback, **not a replacement filesystem sandbox when Landlock is absent**. No automatic namespace fallback exists. `NamespaceSpec`, `probe_namespaces`, `enter_raw`, `write_id_maps`, and bridge-listener validation are released mechanisms requiring a synchronized parent/child protocol. They create a user namespace plus the requested network/IPC/UTS namespaces; they do not create a PID/mount namespace or private root filesystem. The network namespace has loopback only unless the caller adds connectivity. Probing performs real operations in a scratch child and can return `Unavailable`; sysctl inference alone is not the contract. [Namespace implementation][ns]

Nono hard-requires the selected handled filesystem/network rights initially, but masks grants against the chosen ABI and uses best-effort compatibility during parts of rule building. Ordinary apply logs `PartiallyEnforced` and returns success; `NotEnforced` returns an error. `SignalMode::Isolated` and default abstract-socket scoping silently omit those scopes below ABI 6. `AllowSameSandbox` explicitly **fails** below ABI 6. `ProcessInfoMode` is unenforced on Linux regardless of its value. There is no ordinary apply result carrying a complete dropped-rights/effective-controls report. `landlock_scope_policy` exposes requested/enforced signal and abstract-socket booleans, while `SeccompNetFallback` describes only fallback work. [Compatibility and scope code][linux]

**Recommendation:** declare required controls yourself. Require real ABI 6+ for this report's scoped-signal/abstract-IPC example, select `network_baseline`, reject unresolved notification work, validate supported filesystems, and test the exact permitted operations. If complete effective-rights accounting or an explicit full-enforcement-only result is mandatory, ordinary nono apply is insufficient: use the prepared exact-syscall path for a carefully specified policy or direct Landlock integration. Newer ABIs still require fresh integration analysis.

WSL2 follows the Linux path, subject to its kernel/LSM/seccomp configuration. The ordinary implementation warns about 9P-backed paths under `/mnt`, including Windows-host mounts, and recommends a native Linux filesystem. Its probe is heuristic and detection errors do not block startup; the prepared path does not supply a complete mount-safety audit. Keep workloads in the guest's native filesystem and independently validate mount behavior. An ABI probe does not certify every mount. [WSL/filesystem warning implementation][linux]

### macOS: availability, profiles, and distribution

The backend links direct `sandbox_init(profile, 0, errorbuf)` and frees returned errors. It does not shell out to `sandbox-exec`. `support_info()` simply reports true on macOS; it does not trial-compile/apply the intended profile or verify extensions, entitlements, or service availability. Syntax/profile errors become `SandboxInit`. The generator caps expanded port ranges at 16,384 ports to avoid a documented `sandbox_init` crash threshold. [Published implementation][mac]

Apple's **locally shipped** `sandbox_init(3)` and `sandbox-exec(1)` manuals on the test host mark both interfaces deprecated and direct app developers to App Sandbox. The `sandbox_init` manual also warns that legacy **named profiles** must not be used when building against SDK ≥27.0; nono passes a **raw profile**, not `SANDBOX_NAMED`. This is not a promise that raw SBPL or private extension APIs will remain compatible. The source comment calling `sandbox_init` “stable” is weaker evidence than Apple's deprecation notice. Apple's supported [App Sandbox documentation](https://developer.apple.com/documentation/security/app-sandbox) describes a different entitlement-based app deployment model.

The basic library path does not check code signatures or require a sandbox entitlement; that does not waive Developer ID signing/notarization or the constraints of an already sandboxed host application. Do not confuse calling Seatbelt with enabling `com.apple.security.app-sandbox`. Public SBPL documentation is incomplete, extension functions are private interfaces, and CLI DYLD-based interposition has additional hardened-runtime/SIP constraints outside the basic library call. Qualify distribution on the actual signed helper/app and supported OS releases. No App Store acceptance or notarized deployment was established here. [FFI and extension source][mac], [Apple App Sandbox](https://developer.apple.com/documentation/security/app-sandbox)

## Security boundaries, use cases, and material gaps

For a trusted build tool that might accidentally access unrelated files, narrow grants can reduce blast radius substantially. For a malicious native program, the boundary must also close inherited resources, IPC brokers, egress alternatives and later trust handoffs. Both backends share the host kernel; they do not defend against a kernel exploit or authorized destructive writes in the workspace. Snapshot/undo APIs can aid recovery but are not access control. [Kernel security scope][kernel], [undo module](https://docs.rs/nono/0.81.0/nono/undo/)

| Gap / pitfall                                                                                           | Classification and consequence                                                                                                                              | Mitigation                                                                                                                                                                                 |
| ------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Writable workspace contains credentials, `.git`, hooks, or configuration later used outside the sandbox | Policy/integration: a granted write can become a later unsandboxed execution or disclosure                                                                  | Separate disposable workspace, protect secrets outside all grants, review/export outputs before use; never treat rollback as prevention                                                    |
| Writable parent covers read-only input; hard links expose a protected file under another granted name   | Policy plus native alias semantics                                                                                                                          | Disjoint trees; audit aliases/hard links and mutable ancestors. Canonicalizing a path does not prove unique naming or immutability                                                         |
| Existing file descriptors, mapped files, sockets, shared memory or passed FDs                           | Native acquisition-time limitation and launch hygiene                                                                                                       | Start clean; close unneeded descriptors before confinement/exec, sanitize stdio, mappings and environment; deny unsafe broker/SCM_RIGHTS channels                                          |
| Executables and runtime/system trees                                                                    | Integration: no CLI runtime presets are added automatically; broad grants may expose local data, executables, devices or agents                             | Grant only measured executable/loader/library/config requirements; use a small trusted helper; avoid `/`, broad home or device grants                                                      |
| Linux pathname Unix sockets, D-Bus/systemd, Docker/SSH agents                                           | Native features missing from nono's ABI-6 Landlock mapping; an unsandboxed service can act for the workload                                                 | Service the AF_UNIX notification layer with a strict allowlist, hide endpoints with another layer, or use a suitably designed namespace/VM. A filesystem-only grant policy is insufficient |
| macOS Mach lookup and shared memory                                                                     | Backend baseline choices: general Mach service lookup and POSIX shared memory are allowed; some keychain service lookups are explicitly denied              | Review exposed services and trusted raw rules; use App Sandbox or a separate machine boundary for broader isolation needs                                                                  |
| Linux process inspection, POSIX/System V IPC and resource use                                           | Native coverage differences: process-info mode does nothing; default IPC label is not a Linux semaphore denial; resource ceilings are data for a supervisor | Restrict `/proc` exposure; add namespaces/seccomp/cgroup policy and trusted supervisor integration as requirements demand                                                                  |
| Existing sibling threads, partial setup, background descendants                                         | Integration: self-application is not a portable all-thread launch/cleanup mechanism                                                                         | Isolated helper/prepared child, fail before exec, kill/reap on any setup failure, and explicitly manage workload descendants                                                               |

The mappings and exceptions are established by [capability source][cap], [Linux code][linux], [Seatbelt profile][mac] and [kernel limitations][kernel]. Mitigations are recommendations, not hidden nono behavior.

Landlock rules follow kernel filesystem objects/hierarchies; Seatbelt rules are path filters. Neither should be modeled as a universal “secret inode cannot be reached” label. For example, a secret hard-linked into an allowed tree can remain accessible, and host-side renames/replacements can change path-based reasoning. Non-directory `allow_file` rules also need tests for tools that atomically replace a file: directory creation/rename authority is a different permission. Linux Write intentionally includes broad creation/removal/rename rights, and device write grants can add ioctl access. [Linux normalization][linux], [Seatbelt path filters][mac]

Descendants inherit restrictions when created through the restricted process, but a message to an **already unsandboxed** service can cause that service to create an unrestricted process. This is why blocking file reads alone does not protect credentials or stop `systemd-run --user`. Inherited sockets can also evade a filter that only prevents creating new sockets, including the static network baseline. Landlock and Seatbelt checks do not retroactively sanitize all existing resources. [Linux D-Bus escape advisory](https://github.com/nolabs-ai/nono/security/advisories/GHSA-27vp-2mmc-vmh3), [kernel limitations][kernel], [filter code][linux]

### No network versus approved destinations

For **read-only inputs + writable workspace + no new direct Internet networking**, the crate can supply a useful implementation if the input and workspace are disjoint, Linux uses the baseline, macOS uses `block_dns`, and the child starts without inherited network channels. “No communication of any kind” is a stronger requirement: AF_UNIX remains available to Linux's static block filter, POSIX shared memory exists, macOS has service IPC allowances, and stdout itself is a deliberate channel. An external service reachable through IPC can relay traffic. [Linux filter][linux], [macOS DNS and IPC rules][mac]

`block_network()` alone retains implicit macOS resolver access via `/var/run/mDNSResponder` and its `/private` alias. **`block_dns()` is necessary when DNS queries themselves must be forbidden.** It only removes those implicit macOS grants; it is not a Linux DNS firewall. On Linux ABI 4+, automatic Landlock TCP blocking can leave UDP DNS available unless the baseline or another network mechanism is selected. [DNS builder documentation][cap], [network implementations][linux], [macOS rules][mac]

For **approved domains or credentialed endpoints**, `HostFilter` is matching logic, not an egress enforcer, and `proxy_only(port)` does not start a proxy. On Linux ABI 4+, bare Landlock permits that **port on any address**; `network_baseline` still does not add destination-address matching. Below ABI 4, `apply_auto` can return `ProxyOnly`, which is a request for additional notification filtering, not a fully installed proxy boundary. A complete integration must install the prepared/proxy filter in the child, transfer the listener, authenticate/validate notifications and service them from the trusted supervisor. Alternatively, a network namespace can isolate the child to its own loopback with a deliberate bridge to a host proxy. [Proxy/filter API][linux], [namespace bridge][ns]

On macOS, proxy-only outbound is scoped to localhost and the chosen port. Supplying bind ports or localhost IPC port grants can reopen **all bind/inbound ports**, because this generator does not enforce the requested bind-port granularity. It is not a portable network-server allowlist. [Seatbelt generator][mac]

Before claiming destination or credential protection, validate all of these independently:

- IPv4/IPv6 direct sockets, UDP/QUIC, raw sockets, inherited connections, `io_uring`, local proxy alternatives and Unix/Mach endpoints cannot bypass the intended broker.
- Disallowed hostnames are rejected **before DNS resolution**; client-side resolver access is removed if DNS exfiltration is in scope.
- Redirects, CONNECT authority/ports, IDNs, trailing dots, DNS rebinding/private-IP resolution and every upstream request are checked at the actual forwarding point.
- Endpoint normalization agrees with the upstream service; credentials are inserted only for authorized host/method/path combinations and never exposed through errors, logs, returned environment or an alternate broker route.
- Proxy failure, listener loss and supervisor death terminate or leave the child denied; no “retry without proxy” path exists.

These are integration acceptance criteria, not guarantees conferred by `CapabilitySet`. The [published hostname/proxy advisories][advisories] show why matching logic, HTTP normalization, and kernel reachability must be reviewed together.

### Documented vulnerabilities and status

The project's public advisory API and release notes were checked on the research date. Several records use broad or contradictory version fields; those are reported rather than silently repaired. Absence from this table is not evidence that a code path is safe. [Advisory feed](https://api.github.com/repos/nolabs-ai/nono/security-advisories), [0.81.0 release][release]

| Advisory                                                                                                                                                                                           | Material finding and applicability to 0.81.0                                                                                                                                                                                                                                                                                 |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [GHSA-wh99-5qx8-cgh4](https://github.com/nolabs-ai/nono/security/advisories/GHSA-wh99-5qx8-cgh4)                                                                                                   | `apply_seccomp` previously behaved like automatic fallback and left non-TCP traffic available. Record says affected `>=0.67.0`, patched `<0.72.0`, which is internally inconsistent. Changelog adds explicit baseline in 0.72.0; 0.81.0 source contains it. Use that option; the old automatic behavior remains intentional. |
| [GHSA-p39g-3xw6-83jx](https://github.com/nolabs-ai/nono/security/advisories/GHSA-p39g-3xw6-83jx)                                                                                                   | `io_uring` bypassed pathname AF_UNIX notification mediation. Record says affected `>=0.55.0`, patched `<0.81.0` (again reversed-looking). 0.81.0 release and source explicitly add the syscall denial to that mediation filter. This does not make uninstalled mediation effective.                                          |
| [GHSA-vhq2-h2q7-8mmc](https://github.com/nolabs-ai/nono/security/advisories/GHSA-vhq2-h2q7-8mmc)                                                                                                   | Legacy x86 syscall ABIs bypassed seccomp; affected ≤0.73.0, patched 0.74.0. 0.81.0 includes architecture guards. Target architecture remains part of deployment validation.                                                                                                                                                  |
| [GHSA-6hww-cch7-pfrh](https://github.com/nolabs-ai/nono/security/advisories/GHSA-6hww-cch7-pfrh)                                                                                                   | Proxy bypass by reaching another host on the permitted port; affected ≤0.73.0, patched 0.74.0 in the surrounding product. Bare library port-only Landlock semantics still exist in 0.81.0; its caller must add destination mediation.                                                                                        |
| [GHSA-gcpc-cqvp-h5c8](https://github.com/nolabs-ai/nono/security/advisories/GHSA-gcpc-cqvp-h5c8), [GHSA-7q3j-vfmx-gc9g](https://github.com/nolabs-ai/nono/security/advisories/GHSA-7q3j-vfmx-gc9g) | Proxy DNS-before-policy tunnel and hostname normalization/denylist bypasses; records identify 0.74.0 fixes. Relevant when using `HostFilter` or companion proxy, not to offline file confinement alone.                                                                                                                      |
| [GHSA-8r33-hr9m-69wh](https://github.com/nolabs-ai/nono/security/advisories/GHSA-8r33-hr9m-69wh)                                                                                                   | Endpoint authorization/forwarding normalization mismatch. Record lists a ≤0.72.0 issue fixed in 0.73.0 **and a ≥0.73.0 issue with no patched version**. Do not claim 0.81.0 closes all credentialed endpoint bypasses without a maintainer resolution and upstream-specific regression tests.                                |
| [GHSA-p398-cgqf-c48x](https://github.com/nolabs-ai/nono/security/advisories/GHSA-p398-cgqf-c48x)                                                                                                   | Companion TLS proxy UTF-8 panic could prevent CLI audit finalization. Record's affected/patched fields conflict (`>=0.65.0` / `<0.81.0`); 0.81.0 release includes parsing fix. Durable audit completeness on supervisor death is a separate concern.                                                                         |
| [GHSA-q7m6-rr8w-vjff](https://github.com/nolabs-ai/nono/security/advisories/GHSA-q7m6-rr8w-vjff)                                                                                                   | macOS CLI recursive filesystem denies did not recursively deny socket connections; affected ≤0.78.0, patched 0.79.0. A filesystem deny is not intrinsically a socket deny.                                                                                                                                                   |
| [GHSA-27vp-2mmc-vmh3](https://github.com/nolabs-ai/nono/security/advisories/GHSA-27vp-2mmc-vmh3)                                                                                                   | Linux D-Bus escape in CLI <0.55.0; 0.55.0 added pathname mediation. Library users still need to install/service the mitigation themselves.                                                                                                                                                                                   |

Other September advisories concern CLI pack trust, Git-derived dynamic grants, and tool-sandbox shims. They are not executed by a minimal `Sandbox` call. One shim advisory, [GHSA-wjv5-93q3-xm73](https://github.com/nolabs-ai/nono/security/advisories/GHSA-wjv5-93q3-xm73), still names a commit and “unaffected version not yet released”; resolve that if adopting CLI tool mediation rather than claiming the whole 0.81.0 product is advisory-free. [Advisories][advisories]

## Rust integration examples

These examples deliberately implement a **limited, explicit contract**: disjoint read-only input/writable workspace; fixed grants; no newly created direct Internet sockets; no implicit macOS DNS; Linux ABI 6+ signal/abstract-socket scoping. They do **not** promise process-info isolation on Linux, denial of all IPC, revocation of inherited descriptors, or an exact dropped-rights audit. Use a trusted launcher with clean descriptors and environment. Missing directories, unsupported hosts, setup errors and child failures stop the operation.

Create a standalone package with the following files. No nono feature beyond the dependency itself is required. Keep the generated Cargo.lock with an application; the exact crate pin does not pin every transitive dependency. The dependency's declared MSRV was not independently tested on Rust 1.95.

### Cargo.toml


```toml
[package]
name = "nono-launch-example"
version = "0.1.0"
edition = "2024"
rust-version = "1.95"

[dependencies]
nono = { version = "=0.81.0", default-features = false }

[workspace]
```

### src/lib.rs: policy construction and in-process application

```rust
use nono::{AccessMode, CapabilitySet, Sandbox, SignalMode};
use std::{error::Error, io, path::Path};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub fn capabilities(input: &Path, work: &Path) -> Result<CapabilitySet> {
    let input = input.canonicalize()?;
    let work = work.canonicalize()?;
    if input.starts_with(&work) || work.starts_with(&input) {
        return Err(io::Error::other("input and workspace must be disjoint").into());
    }
    let support = Sandbox::support_info();
    if !support.is_supported {
        return Err(io::Error::other(support.details).into());
    }
    #[cfg(target_os = "linux")]
    {
        let abi = Sandbox::detect_abi()?;
        if !abi.has_truncate() || !abi.has_scoping() {
            return Err(io::Error::other("this policy requires Landlock ABI 6+").into());
        }
    }
    Ok(CapabilitySet::new()
        .allow_path(input, AccessMode::Read)?
        .allow_path(work, AccessMode::ReadWrite)?
        .set_signal_mode(SignalMode::AllowSameSandbox)
        .block_network()
        .block_dns())
}

pub fn apply(caps: &CapabilitySet) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use nono::sandbox::{SeccompNetFallback, SeccompOpts};
        let fallback = Sandbox::apply_seccomp(caps, SeccompOpts::network_baseline())?;
        if !matches!(fallback, SeccompNetFallback::None | SeccompNetFallback::BlockAll) {
            return Err(io::Error::other("unexpected supervisor requirement").into());
        }
    }
    #[cfg(target_os = "macos")]
    Sandbox::apply_auto(caps)?;
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return Err(io::Error::other("unsupported target").into());
    Ok(())
}
```

### src/bin/worker.rs: keep the supervisor unrestricted

```rust
use nono_launch_example::{Result, apply, capabilities};
use std::{env, fs, io, path::PathBuf, process::Command};

fn run() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let child_mode = args.first().is_some_and(|a| a == "--child");
    let paths = if child_mode { &args[1..] } else { &args[..] };
    if paths.len() != 2 {
        return Err(io::Error::other("usage: worker INPUT_DIR WORK_DIR").into());
    }
    let input = PathBuf::from(&paths[0]).canonicalize()?;
    let work = PathBuf::from(&paths[1]).canonicalize()?;
    if !child_mode {
        let status = Command::new(env::current_exe()?)
            .arg("--child").arg(input).arg(work)
            .env_clear().status()?;
        if !status.success() {
            return Err(io::Error::other(format!("worker failed: {status}")).into());
        }
        return Ok(());
    }
    let caps = capabilities(&input, &work)?;
    apply(&caps)?;
    let data = fs::read(input.join("probe.txt"))?;
    fs::write(work.join("result.txt"), data)?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("refusing or stopping workload: {error}");
        std::process::exit(126);
    }
}
```

### src/bin/prepared.rs: Linux prepared child enforcement

```rust
#[cfg(target_os = "linux")]
fn run() -> nono_launch_example::Result<()> {
    use nono::{AccessMode, Sandbox};
    use nono::sandbox::{SeccompNetFallback, SeccompOpts};
    use nono_launch_example::capabilities;
    use std::{env, io, os::unix::process::CommandExt, path::PathBuf, process::Command};

    let args: Vec<_> = env::args_os().skip(1).collect();
    let split = args.iter().position(|a| a == "--")
        .ok_or_else(|| io::Error::other("usage: prepared INPUT WORK [RUNTIME_DIR ...] -- ABS_PROGRAM [ARG ...]"))?;
    if split < 2 || args.len() <= split + 1 {
        return Err(io::Error::other("missing paths or command").into());
    }
    let mut caps = capabilities(&PathBuf::from(&args[0]), &PathBuf::from(&args[1]))?;
    for dir in &args[2..split] {
        caps = caps.allow_path(dir, AccessMode::Read)?;
    }
    let program = PathBuf::from(&args[split + 1]);
    if !program.is_absolute() {
        return Err(io::Error::other("program must be absolute").into());
    }
    let abi = Sandbox::detect_abi()?;
    let prepared = Sandbox::prepare_seccomp_with_abi(
        &caps, &abi, SeccompOpts::network_baseline(),
    )?;
    if !matches!(prepared.fallback(), SeccompNetFallback::None | SeccompNetFallback::BlockAll) {
        return Err(io::Error::other("notification supervisor required").into());
    }
    let mut command = Command::new(program);
    command.args(&args[split + 2..]).env_clear().current_dir(&args[1]);
    // Only the allocation-free apply runs between fork and exec.
    unsafe {
        command.pre_exec(move || {
            prepared.apply_raw().map_err(|e| io::Error::from_raw_os_error(e.errno()))
        });
    }
    let status = command.status()?;
    if !status.success() {
        return Err(io::Error::other(format!("workload failed: {status}")).into());
    }
    Ok(())
}

fn main() {
    #[cfg(target_os = "linux")]
    if let Err(error) = run() {
        eprintln!("refusing or stopping workload: {error}");
        std::process::exit(126);
    }
    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("prepared child API example requires Linux");
        std::process::exit(126);
    }
}
```

### Running and interpreting the examples

For `worker`, create two private, disjoint directories and place `probe.txt` in the input. Run:

```sh
cargo run --bin worker -- /absolute/input /absolute/workspace
```

The parent re-executes itself with a cleared environment. Only the helper applies confinement; its Rust code reads `input/probe.txt` and writes `workspace/result.txt`. It needs no runtime path grants because its code and loader are already running before confinement. This is an **in-process worker example**, not an arbitrary target launcher. The supervisor remains unrestricted and reports failure if helper setup or work fails. The helper is intentionally synchronous and single-threaded; do not initialize a multithreaded runtime before applying Linux policy.

The `prepared` example launches a separate executable on Linux. Supply audited **existing** runtime directories as read grants before `--`, then an absolute target and its arguments. For a conventional Linux filesystem, an illustrative invocation is:

```sh
cargo run --bin prepared -- /absolute/input /absolute/workspace \
  /usr /lib /lib64 /bin -- /usr/bin/cat /absolute/input/probe.txt
```

This command is not a universal runtime policy: some systems lack `/lib64`, use Nix stores, or need loader cache, locale/config files or other paths. A missing grant returns an error. Inventory the executable, interpreter, loader and shared objects for your supported deployment; add exact file grants in trusted policy where needed. Do not grant `/etc`, `$HOME`, or `/` merely to make a failed launch succeed. Environment clearing can also remove variables a tool needs; explicitly add only vetted values.

The Linux parent opens rule paths and prepares policy, then `Command::pre_exec` runs only `PreparedLandlockSandbox::apply_raw()`. This method creates/adds the prepared ruleset, sets `no_new_privs`, restricts the child and installs the static filter without heap allocation or libc coordination wrappers. Its error carries a stage and errno; the example uses only errno in the child to avoid allocation. `Command::spawn`/`status` reports setup or exec failure to the parent. Application can fail **after** irreversible Landlock restriction but before seccomp installation; returning from pre-exec prevents target execution. Never catch such an error and continue the target. [Prepared source][linux], [Rust CommandExt contract](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html#tymethod.pre_exec)

The preparation path uses exact raw syscalls for its already selected rights rather than the ordinary apply path's `RulesetStatus` handling. It still masks rights to the selected ABI, cannot add controls the library does not map, and does not solve hostile filesystem aliases, inherited resources, or caller-owned broker policy. `create_ruleset()` alone is not enforcement; callers using its split protocol must separately set `no_new_privs`, restrict the child and install the matching static filter. No equivalent allocation-free prepared macOS apply object is exported in 0.81.0. Use a fresh macOS helper and perform ordinary apply before the helper's final `exec`, with explicit runtime grants. [Prepared and platform API][sandbox], [raw implementation][linux], [macOS apply][mac]

A destination-allowlist example is deliberately omitted: neither `HostFilter` nor a `proxy_only` builder call is equivalent to deploying and enforcing a proxy. Use `nono-proxy` plus a carefully integrated supervisor/namespace bridge, or another egress mechanism; validate the advisory and normalization questions above first.

### Verification performed

On **macOS 27.2, Darwin 27.2.0, Apple Silicon**, using **rustc 1.98.1 (48a229cea, September 1, 2026)**:

- `cargo check --bins` passed for the complete example package on `aarch64-apple-darwin`.
- `cargo check --bins --target x86_64-unknown-linux-gnu` passed, including the actual Linux prepared implementation branch. This was **cross-checking**, not linking or running Linux binaries.
- The macOS `worker` executable was built and executed. Its sandboxed helper copied `probe.txt` into `result.txt` successfully. Missing-input and overlapping-input/workspace invocations both refused with exit status 126.
- A separate disposable child using the same capability builder/application verified `PermissionDenied` for creating a file in the input directory, reading a sibling sentinel file, connecting to an active parent-owned loopback TCP listener, binding a UDP socket, and connecting to an active ungranted Unix-socket listener. Listeners and sentinel data were created under a private temporary fixture; the parent remained unrestricted and the listeners were closed afterward.

These checks did not exercise DNS service IPC, descendants, malicious syscall sequences, extension tokens, signed distribution, Linux kernel enforcement, Windows, or WSL2. A successful compile or these particular denials do not establish a universal confinement guarantee. Neither persistent host security changes nor terminal/browser focus were needed.

### Acceptance tests before adoption

Keep the test runner outside the sandbox and start a fresh child per policy/probe. A denial test must assert the relevant error, not merely that a command exited nonzero; connect to an active listener so connection refusal is not mistaken for policy enforcement. Use private fixtures, bounded timeouts and output, explicit process cleanup and no real credentials.

| Test group         | Required observations                                                                                                                                                                                                                                                                        |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Allowed work       | Read input; create, update and atomically replace workspace files; execute real interpreter/runtime; fork/exec allowed descendants. Confirm failures are not missing-loader artifacts.                                                                                                       |
| File denials       | Read outside grants; create/write/truncate/delete input; rename across boundaries; hard-link/symlink aliases; original/canonical macOS spellings; parent rename/replacement under controlled fixtures.                                                                                       |
| Network            | Active IPv4/IPv6 TCP listeners, UDP DNS-shaped datagrams, raw/non-IP socket creation, `io_uring`, existing pre-opened sockets and FD passing. Test both automatic and explicit baseline to expose the contract difference.                                                                   |
| IPC and process    | Pathname and abstract Unix sockets, D-Bus-like fake broker, signal targets inside/outside the domain, process inspection, shared memory/semaphore behavior. Use controlled fake services, never real host Docker/systemd/keychain brokers for an escape probe.                               |
| Capability refusal | Missing/wrong-type path; macOS `allow_tcp_connect`/bind requests; Linux <6 with `AllowSameSandbox`; absent Landlock; seccomp installation denied; unavailable namespace creation; excessive macOS port ranges; malformed raw rule/profile. No target marker may be created on setup failure. |
| Child protocol     | Missing target, loader denial, pre-exec apply failure, broker listener loss, notification error, proxy death, supervisor death, descendant survival and timeout cleanup. No unsandboxed retry.                                                                                               |
| Deployment         | Each claimed OS/kernel/mount/architecture combination, WSL2 native guest disk versus host mount, and the actual signed macOS helper. Record selected enforcement options, detected/enforced ABI, mounts and observed behavior.                                                               |

To test unavailable Landlock or sysctl/namespace restrictions, use an already suitable disposable VM/container/kernel configuration. Do not disable a host LSM or change machine security settings persistently for a probe. A container can itself block Landlock/seccomp/user namespaces; report that as the tested deployment condition, not a claim about every Linux host. Linux enforcement and older-kernel refusal tests remain adoption work, not results obtained by cross-compilation.

## Alternatives and complementary tools

These comparisons separate Rust APIs from policy/supervision products. Alternative version observations are as of the research date; their claims were not independently enforcement-tested here.

| Option                                                        | Coverage / abstraction                                                                                                                                                                               | Maintenance evidence and integration tradeoff                                                                                                                                                                                                                                                                                                                                                                                                |
| ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **nono 0.81.0**                                               | macOS Seatbelt + Linux Landlock; mechanisms, utility modules, optional caller-integrated seccomp/namespace/supervisor protocol                                                                       | Releases February–October 2026 and public fixes. Good when owning policy and launch orchestration; audit backend differences and do not inherit CLI guarantees by assumption.                                                                                                                                                                                                                                                                |
| **procage 0.7.1**                                             | Competing higher-level Rust child launcher; macOS libsandbox, Linux Landlock/seccomp/namespaces; portable policy crate, capability gaps/explicit relaxations, lifecycle, receipts and optional proxy | Versioned package dates 0.7.1 to November 3, 2026: very recent release evidence, not long-term maturity. More ready-made launch/hygiene policy; potentially better fit when explicit capability gaps and child lifecycle are requirements. Still pre-1.0, with tier-specific limits to review. [Versioned API](https://docs.rs/procage/0.7.1/procage/), [package history](https://docs.rs/crate/procage/0.7.1)                               |
| **gaol 0.2.1**                                                | Older Rust abstraction over sandbox profiles and parent/child setup, with platform-specific implementations                                                                                          | Version 0.2.1 was published October 16, 2019 (previous listed release January 26, 2017). Historical alternative, not a drop-in current Landlock/Seatbelt contract; evaluate backend compatibility and current repository activity separately. [API](https://docs.rs/gaol/0.2.1/gaol/), [package history](https://docs.rs/crate/gaol/0.2.1)                                                                                                   |
| **procjail 0.2.0**                                            | Higher-level runtime process wrapper; documented strategies are Linux unshare, bubblewrap, firejail, then rlimits                                                                                    | Published July 5, 2026. Strategy fallback can change confinement radically; rlimits alone is not a security sandbox. Not established here as a two-backend macOS/Linux peer. [Versioned API](https://docs.rs/procjail/0.2.0/procjail/)                                                                                                                                                                                                       |
| **Direct `landlock` 0.4.7**                                   | Competing Linux binding; explicit ABI, compatibility levels, rights and enforcement status                                                                                                           | Official Landlock Rust project; exposes controls newer than nono's ABI-6 cap and more direct status handling. Requires your own policy, syscall/IPC complements and macOS backend. [API](https://docs.rs/landlock/0.4.7/landlock/), [upstream](https://github.com/landlock-lsm/rust-landlock)                                                                                                                                                |
| **Direct Seatbelt FFI / `sandbox-exec`**                      | macOS binding or subprocess integration with SBPL                                                                                                                                                    | Fewer library dependencies, but own escaping, profile evolution, failures and tests. `sandbox-exec` adds a deprecated executable dependency; direct FFI retains deprecated API/private-profile risk. Neither supplies a Linux backend. [Apple App Sandbox guidance](https://developer.apple.com/documentation/security/app-sandbox)                                                                                                          |
| **nono-cli / nono-proxy**                                     | Surrounding policy, profiles, approvals, credential/network brokers and lifecycle                                                                                                                    | Complements or alternative deployment to embedding, not additional guarantees automatically enabled by `nono`. Shares the project's security history and needs version-matched integration. [Repository architecture](https://github.com/nolabs-ai/nono/tree/v0.81.0/crates)                                                                                                                                                                 |
| **Linux bubblewrap/seccomp/namespaces; cgroups**              | Complementary syscall, namespace and resource mechanisms, or a separate Linux launcher                                                                                                               | More setup and deployment constraints; useful when pathname IPC, processes, private mounts or resource limits exceed nono's mapped controls. They share the host kernel. [bubblewrap upstream](https://github.com/containers/bubblewrap), [kernel seccomp documentation](https://docs.kernel.org/userspace-api/seccomp_filter.html), [cgroup v2](https://docs.kernel.org/admin-guide/cgroup-v2.html)                                         |
| **VM / microVM; WebAssembly runtime where the workload fits** | Stronger separate-kernel boundary, or a different execution/capability model                                                                                                                         | Higher operational cost or workload changes. Prefer a VM for hostile native workloads needing a boundary from host kernel/services; Wasm does not run arbitrary native commands unchanged. [Apple Virtualization](https://developer.apple.com/documentation/virtualization), [Firecracker design](https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md), [Wasmtime security](https://docs.wasmtime.dev/security.html) |

**Adoption recommendation:** prototype with the fixed offline policy first, require measured backend controls, and keep the supervisor outside the sandbox. Evaluate procage alongside nono if a ready-made child launcher and explicit policy-gap reporting are central requirements. Choose direct native integration if exact effective-rights reporting or newer Landlock controls outweigh the cost of owning both backends. Add a broker only after proving that all direct routes to the same authority are closed.

## Authoritative resources and unresolved adoption questions

Primary references:

- [Published nono 0.81.0 API][api], [published source tree](https://docs.rs/crate/nono/0.81.0/source/), [normalized Cargo manifest][cargo], [registry index][index].
- [Versioned Linux implementation][linux], [Seatbelt generator/FFI][mac], [namespace implementation][ns], [capability model][cap].
- [Tagged changelog][changes], [0.81.0 release][release], [maintainer council][maintainers], [public security advisories][advisories].
- [Official kernel Landlock documentation][kernel], [Landlock Rust ABI reference](https://docs.rs/landlock/0.4.7/landlock/enum.ABI.html), [seccomp documentation](https://docs.kernel.org/userspace-api/seccomp_filter.html).
- Apple's shipped `sandbox_init(3)`, `sandbox-exec(1)` and `sandbox(7)` manuals, plus [App Sandbox documentation](https://developer.apple.com/documentation/security/app-sandbox). The local manuals were inspected because the attempted archived Apple web-manual URL was unavailable; this report does not substitute an unofficial blog for Apple's API status.

Questions that materially affect adoption:

1. Will maintainers correct the contradictory affected/patched advisory fields, and provide a released resolution for the remaining endpoint-normalization range? Credentialed endpoint allowlists should wait for an answer plus upstream-specific tests.
2. Which exact OS releases, kernels, filesystems, architectures and signed macOS deployment configurations will the launcher support? The advertised minimums and `support_info` do not answer this.
3. Does the product require pathname IPC isolation, process inspection isolation, resource ceilings or all-thread confinement? If yes, decide the supervisor/namespace/direct-native additions explicitly; a bare apply call cannot supply them all.
4. Can the integration obtain sufficient effective-control reporting and fail-closed evidence despite ordinary apply accepting partial enforcement? If not, choose a different API/layer rather than translating success into a stronger claim.
5. Who owns proxy authorization, listener lifecycle, credential access and descriptor hygiene, and what happens when that component fails? The Rust dependency does not supply the surrounding CLI protocol automatically.

[api]: https://docs.rs/nono/0.81.0/nono/
[pkg]: https://docs.rs/crate/nono/0.81.0
[cargo]: https://docs.rs/crate/nono/0.81.0/source/Cargo.toml
[index]: https://index.crates.io/no/no/nono
[release]: https://github.com/nolabs-ai/nono/releases/tag/v0.81.0
[changes]: https://github.com/nolabs-ai/nono/blob/v0.81.0/CHANGELOG.md
[maintainers]: https://github.com/nolabs-ai/nono/blob/v0.81.0/MAINTAINERS.md
[lib]: https://docs.rs/crate/nono/0.81.0/source/src/lib.rs
[cap]: https://docs.rs/crate/nono/0.81.0/source/src/capability.rs
[sandbox]: https://docs.rs/crate/nono/0.81.0/source/src/sandbox/mod.rs
[linux]: https://docs.rs/crate/nono/0.81.0/source/src/sandbox/linux.rs
[mac]: https://docs.rs/crate/nono/0.81.0/source/src/sandbox/macos.rs
[ns]: https://docs.rs/crate/nono/0.81.0/source/src/sandbox/linux_namespace.rs
[kernel]: https://docs.kernel.org/userspace-api/landlock.html
[advisories]: https://github.com/nolabs-ai/nono/security/advisories
