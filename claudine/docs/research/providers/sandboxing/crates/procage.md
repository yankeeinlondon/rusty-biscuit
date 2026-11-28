---
prompt: |-
    Research the Rust `procage` crate to help a developer decide whether
    and how to use it in a sandbox launcher supporting macOS and Linux.

    The starting point is a child-process sandbox using macOS Seatbelt and
    Linux Landlock, seccomp, and namespaces. Verify the published backends,
    policy crate, capability tiers, and lifecycle behavior rather than
    assuming a portable policy yields equivalent enforcement.

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
    - https://docs.rs/procage/
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
       - Explain portable policy construction and backend lowering, cage
         creation, per-run grants, command spawning, child ownership, and wait.
       - Verify the relationship to procage-policy and distinguish public
         re-exports, companion packages, and backend-specific APIs.
       - Describe default authority, policy composition, inheritance, privileges,
         descriptor/environment hygiene, and the supervisor/workload boundary.
       - List Cargo features, defaults, async integration, native dependencies,
         and any helper or executable-replacement workflows.

    3. Platform integration, compatibility, and failure behavior
       - Map released macOS and Linux backends, including any Linux tiers, to
         required kernels, ABIs, namespace access, and enforcement mechanisms.
       - Explain unsupported-policy errors, approximations, capability gaps,
         explicit relaxations, and any host-fitting API. Determine whether the
         caller must opt into each weakening and can inspect the final policy.
       - Verify macOS native API availability, deprecation, signing/entitlements,
         profile escaping, and helper/tool requirements.
       - Trace setup and launch ordering, post-fork restrictions, threading and
         async constraints, cancellation, and partial initialization failures.
       - Distinguish execution in an existing process from a supervised child.
         Verify descendant termination, process-group/session escape handling,
         handle-drop behavior, and supervisor-death guarantees per backend.
       - Investigate network proxy integration, direct-egress prevention, and
         whether diagnostics or receipts demonstrate enforcement completeness.
       - Address Linux containers and WSL2 separately from native Windows;
         compilation or a policy abstraction does not establish backend support.

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
       - Examine default policy bases, nested protect/hide rules, path aliases,
         preopened descriptors, Unix sockets, inherited environments, and
         toolchain/runtime access that may broaden intended authority.
       - Investigate descendant cleanup races, output handling, denial-report
         completeness, and whether an audit is advisory or an enforcement gate.

    5. Rust examples and verification
       - Demonstrate portable read-only inputs and a writable workspace,
         explicit networking requirements, and child lifetime management.
       - Show strict rejection of unsupported policy, and separately demonstrate
         inspecting any explicitly accepted relaxation without making it a default.
       - Include async launch or executable replacement when supported, stating
         which lifecycle guarantees differ from ordinary child spawning.
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
       Compare nono, other Rust process-sandbox wrappers, direct per-OS
       integration, and external launchers such as Bubblewrap. Compare policy
       expressiveness, explicit degradation, lifecycle ownership, and deployment.
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
hash: 05285c406b860607-351fecf5b14b2609
last_updated: 2026-11-04
---

# procage for a macOS and Linux sandbox launcher

**Assessment:** `procage` is a plausible embedded launcher for same-user, untrusted child processes on macOS and Linux, with unusually explicit policy-gap reporting and run ownership. Adopt it only with a host qualification suite and a deliberate degradation policy. A writable workspace is **not equally enforceable across its backends**: Linux T1 relies on accepted snapshots for some hard denies; T2 cannot protect nested paths inside a writable tree. macOS uses private Apple interfaces and rejects allows on case-sensitive volumes. Neither a successful launch nor a clean violation record proves complete confinement. These conclusions follow from the released [lowering contract](https://docs.rs/crate/procage-policy/0.7.1/source/src/lowering.md), [backend source](https://docs.rs/crate/procage/0.7.1/source/src/lib.rs), and [threat model](https://github.com/hupe1980/procage/blob/v0.7.1/crates/procage/THREAT_MODEL.md).

Research date: **November 4, 2026**. Exact stable release examined: **procage 0.7.1**, with **procage-policy 0.7.1** and **procage-proxy 0.7.1**. “Stable” here means a non-prerelease version in the registry; it is not a 1.0 API or a security certification. Recommendations below are this report's judgments, not upstream guarantees.

## Identity, release history, and adoption risk

The [published registry metadata](https://crates.io/api/v1/crates/procage) identifies the package as `procage`, repository [`hupe1980/procage`](https://github.com/hupe1980/procage), author `hupe1980`, license **MIT OR Apache-2.0**, edition 2024, and declared MSRV **Rust 1.99**. The [registry owner endpoint](https://crates.io/api/v1/crates/procage/owners) lists one owner, **Frank Hübner (`hupe1980`)**. There is no identified parent organization or required parent application: the repository is a workspace containing the runtime, policy and proxy crates, test infrastructure, and Python/Node bindings. Those bindings are companion products, not needed for Rust integration. The [published manifest](https://docs.rs/crate/procage/0.7.1/source/Cargo.toml) and downloadable `.crate` establish that this is a real dependency, not the similarly named npm reservation.

The latest stable version at the research cutoff was **0.7.1**, published **2026-11-03 at 22:41:43 UTC**. All 13 registry versions below were non-yanked, with no semver prerelease identifiers. The entire published history falls inside the requested 24-month window, November 4, 2024–November 4, 2026; no older published milestone was found. Dates are registry publication dates, with changes cross-checked against the [versioned changelog](https://github.com/hupe1980/procage/blob/v0.7.1/CHANGELOG.md).

| Publication date (UTC) | Versions     | Relevant milestone                                                                                                                                                               |
| ---------------------- | ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2026-10-27             | 0.0.1        | Name reservation; no Rust implementation.                                                                                                                                        |
| 2026-11-01             | 0.1.0        | First implementation: both OS backends, T1/T2, policy lowering, hard denies, fitting, hygiene, reports, exec helpers, frozen bases/presets.                                      |
| 2026-11-02             | 0.2.0        | Service/container handoff and process-introspection protections; default hard-link refusal; receipts, audits, explain/diff. Introduced explicit consent for Linux `/proc` reach. |
| 2026-11-02             | 0.3.0        | Tokio, integrated proxy, Landlock rights through ABI 11; fixed cross-directory move/link handling and passed CLOEXEC descriptors.                                                |
| 2026-11-02             | 0.3.1, 0.3.2 | Linux run lifetime permits descendant process groups but refuses new sessions; adds exec-session workflow.                                                                       |
| 2026-11-02             | 0.4.0        | `Network::Ip`, excluding general Unix-socket access; `Network` becomes non-exhaustive.                                                                                           |
| 2026-11-03             | 0.4.1        | Security fixes for raced Linux mount targets and macOS case-sensitive-volume allows. T1 now requires modern descriptor-based mount syscalls.                                     |
| 2026-11-03             | 0.5.0        | New PTYs, policy suggestions; fixes macOS cleanup racing another run's child.                                                                                                    |
| 2026-11-03             | 0.6.0, 0.6.1 | Python bindings; wheel packaging correction.                                                                                                                                     |
| 2026-11-03             | 0.7.0, 0.7.1 | Node bindings; npm publication correction in 0.7.1, with no code change from 0.7.0 for that fix.                                                                                 |

This is eight days from reservation to the research date, and roughly two days of implemented releases. That is evidence of concentrated activity, not evidence of long-term maintenance or maturity. The [repository history](https://github.com/hupe1980/procage/commits/main/) showed a November 3 follow-up recording the release after the published commit; do not treat arbitrary `main` content as 0.7.1. Registry trusted-publishing metadata associates 0.7.1 with commit [`0ea56cdd53b92bf6c304c7475e91e590d807b72e`](https://github.com/hupe1980/procage/commit/0ea56cdd53b92bf6c304c7475e91e590d807b72e).

Upstream explicitly permits breaking changes between pre-1.0 minor versions. Pin and review upgrades: 0.2 changed formerly accepted policies into failures, 0.4 changed exhaustive enum matching, and 0.4.1 tightened host requirements and path identity. A renamed/replaced protect/hide target can require rebuilding the cage. Pre-1.0 alone says nothing about reliability; the short evidence window and security-sensitive recent changes are the adoption concerns. The [security policy at v0.7.1](https://github.com/hupe1980/procage/blob/v0.7.1/SECURITY.md) says only the latest minor receives fixes, without backports, but its table still says “0.4.x.” That conflict needs maintainer clarification; it does not establish maintained 0.4 or 0.7 support indefinitely.

## Public API: intent, plans, and actual execution

`procage::policy` publicly re-exports the **whole `procage-policy` crate**. That crate is a separate `no_std` + `alloc`, unsafe-free policy/data/lowering library; using it alone confines nothing. `procage::native` aliases `procage::macos` or `procage::linux` at compile time. The shared API is intentionally a subset: backend errors and plans remain different concrete types. `procage::proxy` is a feature-gated re-export of **procage-proxy**, not a built-in hostname rule in the kernel. See the published [exports](https://docs.rs/crate/procage/0.7.1/source/src/lib.rs) and [policy API](https://docs.rs/crate/procage-policy/0.7.1/source/src/lib.rs).

The integration sequence is:

1. Build `Policy`, filesystem rules, named `Grant` declarations, network/process/IPC/environment/limits/TTY settings, and optionally a versioned base or preset.
2. Optionally call `lower(&policy, Target, &LowerOptions)` for plan-only compatibility analysis. This does not probe the real host.
3. Call `Cage::new`, or `Cage::with_options`, to probe/resolve/lower/prepare the native backend. Unsupported requirements return errors. `Cage::fit` is an explicitly configured alternative that changes only named unsupported constructs and returns the changes.
4. Bind declared grant names to existing paths with `Grants::new().bind(name, path)`. Bindings are per run, not amendments to every later run's authority.
5. Pass a fresh `std::process::Command` to `spawn`, or Tokio command to `spawn_async`. Preserve the returned procage child handle, drain I/O, wait for the direct child, and explicitly end the run.

`Cage` is reusable and cloneable; its `plan()` exposes the lowered representation. macOS exposes SBPL, parameters, grant slots, degradations and waivers; Linux exposes the Landlock, seccomp, mount/namespace and hygiene plan. `Cage::explain`, policy `explain::diff`, and `procage::suggest` help explain authority, but are models of enforcement, not kernel attestations. Suggestions return a proposed policy and a `weakens()` indication; applying one is a caller decision. Sources: [macOS Cage](https://docs.rs/crate/procage/0.7.1/source/src/macos/mod.rs), [Linux Cage](https://docs.rs/crate/procage/0.7.1/source/src/linux/mod.rs), and [suggestions](https://docs.rs/crate/procage/0.7.1/source/src/suggest.rs).

### Default authority and composition

`Policy::default()` has no base, filesystem access, networking, process creation, signals, IPC, inherited environment allowlist, or controlling terminal. It is not a usable general-purpose shell policy by itself. `Access::Write` includes read; `Access::Exec` includes read. `Exec::Any` executes wherever readable; it does not grant all filesystem reads. `Protect` removes writes inside broader allows; `Hide` removes access. Rules are not a first-match ACL: macOS lowering emits default deny, allows, inner denies, then hard denies, with ordering normalized. An allow cannot simply override a protect/hide or a hard deny. Linux's additive Landlock grants cannot express all such subtraction. See [policy definitions](https://docs.rs/crate/procage-policy/0.7.1/source/src/policy.rs) and [lowering implementation](https://docs.rs/crate/procage-policy/0.7.1/source/src/lower/macos.rs).

Every policy includes named hard-deny classes unless explicitly present in `Policy::waive`: credentials/secrets, trust handoff, persistence, process introspection, service handoff, container engines, privilege escalation, terminal injection, and platform-specific kernel/namespace/socket restrictions. They are a curated catalog, not recognition of every possible secret or execution sink. On Linux they can themselves become gaps when a broad rule reaches the protected names. A writable workspace therefore does not imply that `.git/config`, hooks, shell startup files, or agent configuration within it are writable. Conversely, accepting a T1 snapshot means newly created matching paths are not necessarily protected. [Class lists and presets](https://docs.rs/crate/procage-policy/0.7.1/source/src/presets.rs), [Linux class handling](https://docs.rs/crate/procage/0.7.1/source/src/linux/classes.rs).

Bases and presets broaden authority materially:

| Choice                           | Authority or consequence                                                                                                                                                                                       |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Base::None`                     | Only explicit rules; caller must supply executable, loader, libraries, configuration, devices and metadata access.                                                                                             |
| V1                               | System code and data. Linux includes read/exec of `/usr`, `/bin`, `/sbin`, library trees and read of `/etc`; macOS includes system code, `/private/etc`, standard devices, sysctls and selected Mach services. |
| V2 / V3                          | macOS adds developer-tool access including Command Line Tools, Xcode and substantial Homebrew/`/usr/local` trees. V3 suppresses benign denial reports and adds no authority over V2. Linux V2/V3 equal V1.     |
| `minimal::v3()`                  | Base and preset environment allowlist, without workspace/process additions.                                                                                                                                    |
| `agent_readonly::v3(workspace)`  | Workspace read, fork, self/child signals, private-temp grant. ABI <6 cannot enforce its signal scope.                                                                                                          |
| `agent_workspace::v3(workspace)` | Adds workspace write **and execution**, subject to hard denies and Linux gaps.                                                                                                                                 |
| `build::v3(workspace)`           | Also loopback and workspace Unix sockets; Linux rejects loopback in this release.                                                                                                                              |

These are frozen versioned policies, not a moving “safe defaults” promise. A broad code tree may contain user-installed code or data; inspect it before treating it as trusted. Home-based Rust/toolchain caches need additional grants and explicit environment settings; avoid solving tool startup by granting the entire home or `/`. macOS git commonly needs `GIT_CONFIG_GLOBAL=/dev/null`; prepare repositories outside the cage if hooks/config must stay protected, and use `gc.autoDetach=false` when Linux forbids daemonization. [Base profiles and runtime guidance](https://docs.rs/crate/procage-policy/0.7.1/source/src/base.rs).

### Cargo features and native requirements

| Package / feature              | Released behavior                                                                                                                                                              |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `procage`, no default features | Synchronous spawn, exec, policy re-export, reports/audits/receipts. Uses libc plus Rust serialization/digest dependencies; no libseccomp or external launcher dependency.      |
| `tokio`                        | Adds Tokio process/runtime/sync integration and futures-core streams. Caller needs an active runtime and I/O driver; absent runtime gives `SpawnError::NoRuntime` before fork. |
| `proxy`                        | Adds `procage-proxy`, per-run HTTP/CONNECT/SOCKS5 proxy integration and allowlist refusals.                                                                                    |
| `procage-policy`               | No default feature set; optional `serde` and `unstable-operations`. Runtime procage enables `serde`. Avoid unstable raw operations without reviewing their backend effect.     |

There is no runtime helper binary shipped or installed by the Rust crate. Linux uses syscalls directly; macOS dynamically loads `/usr/lib/libsandbox.1.dylib`. macOS diagnostic reporting may invoke the OS `log` tool and need access the account lacks; this is distinct from enforcement and launch. The runtime and companion manifests declare Rust 1.99. [Runtime manifest](https://docs.rs/crate/procage/0.7.1/source/Cargo.toml), [policy manifest](https://docs.rs/crate/procage-policy/0.7.1/source/Cargo.toml), [native macOS interface](https://docs.rs/crate/procage/0.7.1/source/src/macos/sys.rs), [report collection](https://docs.rs/crate/procage/0.7.1/source/src/macos/violations.rs).

## Platform compatibility and explicit degradation

Seatbelt is Apple's kernel sandbox, with operations filtered by a compiled profile. Landlock restricts the applying thread and future descendants using kernel object hierarchies; rules stack with other access controls. Seccomp filters syscall numbers and scalar arguments, not arbitrary pointed-to path strings. Namespaces isolate selected kernel resource views. These are complementary mechanisms, and seccomp alone is not a complete sandbox. [Official Landlock documentation](https://docs.kernel.org/userspace-api/landlock.html), [official seccomp documentation](https://docs.kernel.org/userspace-api/seccomp_filter.html), [Linux namespaces manual](https://man7.org/linux/man-pages/man7/namespaces.7.html).

| Released backend | Actual mechanisms / prerequisites                                                                                                                            | Important differences                                                                                                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| macOS            | Private libsandbox compilation, `__mac_syscall` application, sandbox checks/extensions. Unsandboxed caller; compatible private symbols and profile behavior. | No `sandbox-exec`. Path allow rules on case-sensitive volumes are refused. Kernel-level profile inheritance; descendant cleanup uses userspace marker queries.                             |
| Linux T1         | Landlock ABI ≥1; working unprivileged user, mount, IPC, UTS namespaces; network namespace when required; seccomp; required mount APIs.                       | Read-only bind mounts for protect, covers for hide, private IPC resources. No PID namespace/private `/proc`. Some hard denies and socket covers are snapshots, requiring explicit consent. |
| Linux T2         | Landlock ABI ≥1 and seccomp, without T1 namespace/mount isolation.                                                                                           | No protect inside writable trees, no hide inside writable trees, no integrated proxy, no loopback-only mode. Some read-only hiding can use accepted piecewise snapshots.                   |
| Other targets    | Policy data may compile.                                                                                                                                     | No native Windows launcher backend or `native` alias there; no Windows enforcement claim.                                                                                                  |

Linux probes cache the actual ABI, errata and namespace result **per process**. `CageOptions::with_tier` can require a tier; `with_landlock_abi` caps the effective ABI for compatibility testing. It selects the strongest available tier, not the weaker tier that happens to accept a policy. A T1 construction failure is not an instruction to retry unsandboxed. T1's descriptor-anchored mount setup requires `openat2`, `open_tree`, `fsmount`, `mount_setattr`, and `move_mount`: the “Linux 5.13” Landlock minimum alone is insufficient to promise T1. The syscall mappings explicitly target x86-64 and AArch64; other architectures need independent build/runtime qualification. [Probe](https://docs.rs/crate/procage/0.7.1/source/src/linux/probe.rs), [syscalls and mount setup](https://docs.rs/crate/procage/0.7.1/source/src/linux/sys.rs), [seccomp architecture mapping](https://docs.rs/crate/procage/0.7.1/source/src/linux/seccomp.rs).

### Linux ABI-dependent enforcement

The following versions are upstream's ABI introduction mapping, not a substitute for the running-kernel probe. Distributions may backport features. Procage 0.7.1 uses rights through **ABI 11**, even if the host reports newer capabilities. Current kernel documentation discusses newer facilities, which are not automatically exposed by this release.

| ABI / upstream kernel | Consequence for this launcher                                                                                                                                                                                     |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1 / 5.13              | Minimum on both tiers. No Landlock means refusal. Cross-directory move/link behavior remains restricted before ABI 2.                                                                                             |
| 3 / 6.2               | Truncate right. Below it procage installs stricter seccomp guards: pathname `truncate` refused, read-only `O_TRUNC` refused, `openat2` returns ENOSYS. Use writable FDs for truncation even within allowed paths. |
| 4 / 6.7               | TCP bind/connect port rights. Before it a caller-supplied TCP socket can evade a no-network intent; new IP socket creation is still refused. Proxy integration needs ABI 4+.                                      |
| 6 / 6.12              | Signal scope and abstract Unix-socket scope. Self/descendant-only signals are unsupported below it; `Signals::SameUser` is an explicit, significant weakening.                                                    |
| 9 / 7.1               | Pathname Unix-socket access rights: exact allow paths on both tiers. Before it T1 masks discovered sockets with consent; T2 cannot provide the same allowlist.                                                    |
| 10 / 7.2              | UDP rights. Before it caller-supplied UDP sockets can still send under a no-network policy.                                                                                                                       |

The [released Linux backend](https://docs.rs/crate/procage/0.7.1/source/src/linux/mod.rs) and [versioned policy table](https://docs.rs/crate/procage-policy/0.7.1/source/src/lowering.md) give these distinctions. Do not pass existing sockets when claiming no networking; syscall creation restrictions are not revocation of every inherited endpoint.

### What requires caller consent

`lower` returns `LowerError::Gaps` with named `CapabilityGap`s and kinds. Under default `LowerOptions`, approximations are rejected too. A Linux literal-file rule binds an inode at spawn rather than permanently following a replaced pathname; that needs named approximation consent. Globs are unsupported on Linux, not silently expanded. A raw Seatbelt layer on Linux, or a raw Linux layer on macOS, is ignored only with explicit acceptance of that cross-backend approximation. [Lowering options and gap types](https://docs.rs/crate/procage-policy/0.7.1/source/src/lower/mod.rs).

There are two separate consent paths:

- **Approximation:** `LowerOptions::allow(Construct::...)` accepts a named imperfect implementation. `accept_spawn_snapshot()` accepts a whole documented group, including hard-deny, hiding and service/socket snapshots; it is broader than one isolated permission. The plan records degradations.
- **Changed requirement:** `Cage::fit(&policy, &options, &relax)` clones the policy and accepts only named unsupported constructs: omit protect/hide, waive a named class, broaden signals, remove Unix sockets, or remove loopback networking. Removing networking is stricter; omitting protection or broadening signals is weaker. It returns `(Cage, Vec<Relaxed>)`, each with the gap, exact change and what would lift it. Unaccepted gaps remain errors. It does not automatically accept approximations.

Consent is by **construct/class**, potentially covering multiple rules, not a separate interactive approval for every path. Log and review every returned change. `plan()` exposes the effective backend plan; `fit` does not return a public getter for the fully rewritten `Policy`. For that, apply the public `Relax::apply` to a clone while examining gaps, or retain policy-inclusive receipts from actual runs. Never reconstruct authority solely from a “sandboxed” boolean. [Fit implementation](https://docs.rs/crate/procage-policy/0.7.1/source/src/lower/fit.rs), [runtime fit loop](https://docs.rs/crate/procage/0.7.1/source/src/linux/mod.rs).

### macOS deployment details

Avoiding deprecated `sandbox-exec` avoids an executable dependency, not Apple's private-API risk. Procage resolves `sandbox_compile_string`, parameter/extension functions, `sandbox_check` and `__mac_syscall` before fork. Missing symbols, incompatible behavior, nesting and compile limits fail rather than choose an unsandboxed fallback. The [published interface source](https://docs.rs/crate/procage/0.7.1/source/src/macos/sys.rs) is the evidence for these private calls; Apple offers no supported compatibility guarantee for them. The deprecated-tool characterization is corroborated by upstream's documentation; an authoritative Apple deprecation date for these exact private symbols was not established. Avoiding an old public entry point does not make the replacement private ABI supported. Apple's documented [App Sandbox](https://developer.apple.com/documentation/security/app-sandbox) uses app entitlements and is a different deployment model. No special signing entitlement, Apple approval or root requirement is imposed by procage's child launcher; normal application signing/notarization requirements remain deployment concerns. Enabling App Sandbox on the supervisor is not a way to make private nested sandboxing reliable.

Paths are resolved for symlink/firmlink aliases (`/tmp` versus `/private/tmp`, data-volume spellings); SBPL strings and glob-derived regexes are escaped. It grants some link/ancestor metadata needed for lookup. Use typed paths/rules instead of concatenating profile text. Escaping does not solve case folding: 0.7.1 rejects allow rules, Unix-socket allows, raw allows and grant bindings on case-sensitive macOS volumes because Seatbelt can also match distinct case variants. This is a deployment blocker, not an optional compatibility warning. [Canonicalization](https://docs.rs/crate/procage/0.7.1/source/src/macos/canon.rs), [SBPL escaping](https://docs.rs/crate/procage-policy/0.7.1/source/src/sbpl.rs), [0.4.1 security fix](https://github.com/hupe1980/procage/blob/v0.7.1/CHANGELOG.md#041---2026-11-03).

Upstream reports CI lanes for macOS 26/27 and Ubuntu 24.04/26.04 and kernel-matrix testing. The released threat model still says bases were measured on macOS 26.6.2 and makes no certainty claim for other releases. Treat those as different evidence levels: a green lane can cover particular operations without validating every service, toolchain and filesystem on a new OS. macOS 15 is explicitly untested in the changelog. The root README's “no PTY” and open R-08/R-25 statements are stale relative to the 0.7.1 source/threat model; the newer implementation has PTYs and mitigations for the documented variants. This report follows the published code, without treating its mitigation labels as an independent audit.

### Containers, WSL2, and Windows

A Linux container shares its host kernel; a high container distro version cannot add missing Landlock rights. Docker seccomp/AppArmor restrictions may block user namespaces or a seccomp listener, yielding T2 or reporting unavailability. Test the actual image, host kernel, mounts, runtime UID and outer security policy; do not make the container privileged merely to pass a demonstration. [Runtime probe/error behavior](https://docs.rs/crate/procage/0.7.1/source/src/linux/probe.rs), [reporting compatibility](https://docs.rs/crate/procage/0.7.1/source/src/linux/violations.rs).

WSL2 has a real Linux kernel, so procage takes the Linux backend, contingent on that kernel's configuration/ABI and namespace permissions. Native Windows is explicitly outside procage's threat model. WSL2's Windows-executable interop and mounted Windows filesystems need separate escape/alias tests; kernel compatibility alone does not establish confinement of a Windows executable reached through interop. Prefer Linux filesystem workspaces and deny interop paths/endpoints for a hostile-workload deployment. These WSL recommendations are inferences, not an upstream validated backend promise. [Microsoft's WSL architecture comparison](https://learn.microsoft.com/en-us/windows/wsl/compare-versions), [Windows/Linux interop](https://learn.microsoft.com/en-us/windows/dev-environment/wsl-interop), [procage target gating](https://docs.rs/crate/procage/0.7.1/source/src/lib.rs).

## Launch ordering and lifecycle guarantees

The ordinary spawn boundary leaves the supervisor unrestricted. Preparing paths, profiles, filters, descriptors and allocations happens before fork. Procage installs its child sequence on the command; after fork it uses preallocated state and syscall-oriented native functions. Use fresh commands without unrelated `pre_exec` hooks: caller-supplied hooks are additional trusted code in a sensitive interval. Multithreaded supervisors and Tokio are supported by spawn; this does not authorize arbitrary allocation, locking or callbacks in the post-fork child. [macOS child operations](https://docs.rs/crate/procage/0.7.1/source/src/macos/sys.rs), [Linux child operations](https://docs.rs/crate/procage/0.7.1/source/src/linux/sys.rs).

| Backend | Before target execution                                                                                                                                                                                                                                                                       |
| ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| macOS   | Establish session/TTY and limits, mark undesignated descriptors close-on-exec, apply compiled profile, verify sandboxing, consume per-run extension grants, then exec. Failure names `Step`; target does not run.                                                                             |
| Linux   | Establish session/TTY; set/check parent-death signal; create namespaces/mounts and UID/GID maps for T1; drop capabilities; set limits and CLOEXEC; `no_new_privs`; Landlock; seccomp/listener handoff; verify seccomp and no-new-privileges; exec. Failure names `Step`; target does not run. |

Parent-side guards own temporary resources and launch-abandon cleanup. Filesystem/mount races can return `MountMoved`; do not retry through a weaker path. Successful setup only proves these setup checks succeeded, not that the requested policy covers every attack route. [Linux launch preparation](https://docs.rs/crate/procage/0.7.1/source/src/linux/spawn.rs), [macOS launch preparation](https://docs.rs/crate/procage/0.7.1/source/src/macos/spawn.rs).

### Handle ownership is part of the boundary

`Child` dereferences to `std::process::Child`; `AsyncChild` dereferences to Tokio's child. `wait()` waits for **the direct child only**. A run ends on `kill_all`, handle drop, or completion operations such as `receipt()`/`audit()` that end an exited child's run. `kill()` through Deref kills only the direct child. `detach()` relinquishes lifetime control and leaves private temp storage behind. Keep the procage handle even while separately draining its taken stdout/stderr handles. [Synchronous Linux ownership](https://docs.rs/crate/procage/0.7.1/source/src/linux/child.rs), [macOS ownership](https://docs.rs/crate/procage/0.7.1/source/src/macos/child.rs).

| Situation                               | Linux                                                                                                                                                                                                           | macOS                                                                                                                                                      |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ordinary spawn, default `Lifetime::Run` | Child leads a session; seccomp denies descendant `setsid`, while permitted process groups stay in that session. Cleanup scans/kills session and descendants. Inherited TTY instead needs process-group locking. | Descendants may change session/group. A profile marker inherited through fork/exec lets supervisor find double-forked descendants after direct-child exit. |
| `Lifetime::Tree`                        | A double-forked descendant in another session can escape cleanup.                                                                                                                                               | Still uses marker discovery for spawned runs.                                                                                                              |
| Explicit cleanup                        | `kill_all()` returns I/O errors, but session scanning is bounded and its internal kill routine does not establish a verified zero-survivor result.                                                              | Marker scan/kill is bounded to 64 rounds and returns an error with survivors. PID reuse between query and kill remains a documented race.                  |
| Drop / cancellation                     | Drop attempts cleanup and removes private temp storage; errors are discarded.                                                                                                                                   | Same, including discarded marker-cleanup errors.                                                                                                           |
| Abrupt supervisor death                 | Direct child has `PR_SET_PDEATHSIG(SIGKILL)` plus parent check; **not a whole-descendant death guarantee**.                                                                                                     | No equivalent kernel parent-death guarantee in the released launcher; marker cleanup needs the live supervisor.                                            |

The last distinction is important: Linux clears the parent-death setting in children created by `fork`, and it refers to the creating parent **thread**, not necessarily the lifetime of the supervisor process. Procage does not add cgroup ownership or a PID-namespace init that guarantees all descendants die on supervisor SIGKILL. This is an inference from [the release's parent-death setup](https://docs.rs/crate/procage/0.7.1/source/src/linux/sys.rs) and the [official PR_SET_PDEATHSIG semantics](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html), qualifying broader upstream “children die with parent” wording.

Even ordinary cleanup is userspace process discovery: Linux reads `/proc`, macOS queries private marker behavior, and both race process exits/forks. Linux's bounded session scan and ignored per-kill outcomes merit hostile fork-churn testing; `Ok(())` is not an attested empty process tree. Use an independently owned service/cgroup or stronger container/VM lifecycle when survivor-free cleanup after crashes is a hard requirement. [Linux cleanup implementation](https://docs.rs/crate/procage/0.7.1/source/src/linux/child.rs), [macOS marker implementation](https://docs.rs/crate/procage/0.7.1/source/src/macos/lifetime.rs).

Tokio spawn itself is synchronous preparation plus spawn, not an async future that can be canceled mid-setup. Canceling a future that merely borrows a retained child's `wait()` does not drop that child; the owner must end it. Dropping the actual `AsyncChild` invokes cleanup, possibly blocking briefly. `kill_all().await` awaits reaping but performs some final bookkeeping on the calling task. Keep ownership through timeouts and runtime shutdown, and explicitly observe cleanup errors. [Async implementation](https://docs.rs/crate/procage/0.7.1/source/src/async_child.rs).

### Executable replacement is a different contract

`Cage::exec` and `exec_with_fds` sandbox **the calling process** and replace its executable; they return a `SpawnError` only on failure. There is no child handle or supervisor inside that process. Use only in a dedicated launcher helper whose external owner handles waiting, pipes and termination. T1 exec rejects a multithreaded process; call early before runtimes/workers. Validation failures can precede mutation, but apply/exec failures can leave partial confinement: print an error and exit, never return to normal supervisor duties. The auto-managed `tmpdir` grant and `Tty::NewPty` are refused because this workflow lacks their owning handle. [Linux exec contract](https://docs.rs/crate/procage/0.7.1/source/src/linux/mod.rs), [macOS exec contract](https://docs.rs/crate/procage/0.7.1/source/src/macos/mod.rs).

Under Linux `Lifetime::Run`, plain exec disallows `setpgid` as well as `setsid`, permitting external process-group termination. `exec_session` permits descendant groups within its session; call backend `end_session(pid)` **before reaping** the helper. Do not start that helper with `process_group(0)` unless it already leads a session; the API rejects that incompatible state. On macOS exec/exec-session descendants can create other sessions that external group/session kills miss; ordinary `spawn` is needed for procage's marker-owned cleanup. Keep any caller-built helper outside all writable grants: helper replacement is the caller's responsibility. [Complete upstream helper](https://github.com/hupe1980/procage/blob/v0.7.1/crates/procage/examples/exec_helper.rs).

## Networking, hygiene, audits, and material limitations

### Approved destinations require a broker and direct-egress blocking

`Network::None` denies new network sockets (Linux still permits socketpairs); `Network::Ip` permits arbitrary IP endpoints, including local daemons, not approved destinations. `Network::All` additionally permits broad Unix-socket access. `Loopback` is implemented on macOS but is a Linux gap; a TCP port restriction cannot distinguish loopback from a remote address. Do not equate proxy environment variables with enforcement. [Network lowering](https://docs.rs/crate/procage-policy/0.7.1/source/src/lower/linux.rs).

With feature `proxy`, combine `Network::Proxy` with `CageOptions::with_proxy(Proxy::new(Allowlist::new(...)?))`. macOS narrows outbound networking to the run's loopback proxy ports; Linux T1 creates a network namespace whose only reachable endpoints are its own proxy listeners and additionally constrains port access through Landlock. T2 rejects proxy-only policy. Standard upper/lowercase proxy variables are injected and `NO_PROXY` is removed; clients that ignore proxies fail rather than receiving direct egress. On macOS external proxy ports can also be represented without the integrated helper, making broker ownership the caller's problem. Linux requires the integrated proxy for this workflow. [Proxy integration](https://docs.rs/crate/procage/0.7.1/source/src/proxied.rs), [runtime requirements](https://docs.rs/crate/procage/0.7.1/source/src/linux/mod.rs).

The companion proxy supports HTTP absolute requests, HTTP CONNECT, and unauthenticated SOCKS5 CONNECT. It parses one canonical host value for check and connect, checks resolved addresses, and needs explicit ranges for nonpublic addresses such as loopback/private/link-local/metadata endpoints. It relays bytes without TLS interception. An allowed service can still be an exfiltration sink or serve arbitrary content; hostname permission does not constrain URL paths, tenant identity, protocol contents or downstream relay behavior. Set ports explicitly. Its serving path converts startup failure into an absent proxy run; kernel egress restrictions remain, but launch success alone does not establish proxy readiness. [Proxy crate](https://docs.rs/crate/procage-proxy/0.7.1/source/src/lib.rs), [canonical host parser](https://docs.rs/crate/procage-proxy/0.7.1/source/src/host.rs), [integration's `serve` handling](https://docs.rs/crate/procage/0.7.1/source/src/proxied.rs).

### Security gaps and who must close them

| Area / classification                                            | Consequence                                                                                                                                                                                | Practical mitigation                                                                                                                                             |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Nested filesystem denies — OS model and backend capability       | Additive Landlock alone cannot subtract protect/hide inside a writable parent. T1 mounts add subtraction, but class-path snapshots miss future names.                                      | Require T1 plus explicit snapshot acceptance, isolate outputs from future trusted execution, or reject. Do not waive a class just to get T2 working.             |
| Aliases and preopened FDs — OS capability semantics              | Symlinks/firmlinks are canonicalized, but existing descriptors may retain authority independent of pathname rules. Hard links create additional names.                                     | No ambient passed descriptors; keep inputs/workspace disjoint and host-controlled during setup. Retain default link refusal. Test aliases and mount boundaries.  |
| Hard-link audit — integration boundary                           | Default `LinkAudit::Refuse` gates spawn; `Report`/`Off` weaken it. Walks do not cross devices or follow links, can be incomplete, and some large/privacy-protected home data are excluded. | Inspect completeness and scope; avoid treating a clean scan as a proof that every alias on the host was found. Exempt only immutable protected stores knowingly. |
| Unix sockets/service handoff — policy coverage and OS capability | D-Bus, systemd or Docker/Podman can run code outside the sandbox; known lists do not cover every custom engine socket or local network service.                                            | Prefer `None`/`Ip` without Unix sockets, require exact ABI where needed, deny custom endpoints. Never pass engine/agent sockets casually.                        |
| Environment and runtime — integration                            | Default is empty allowlist, but explicit `Command::env` entries win even for secrets. Passthrough removes only recognized names; arbitrary names may leak secrets.                         | `env_clear` and an explicit small map, trusted PATH/HOME/cache roots, no authentication sockets or loader injection variables.                                   |
| Privileges — mechanisms                                          | Linux drops capabilities and always sets no-new-privileges; macOS hard denies privilege-changing operations/exec. This is not a different VM or user identity.                             | Run as an unprivileged user and retain privilege hard denies. Do not use a privileged launcher to compensate for host incompatibility.                           |
| Output and PTYs — another mediation layer                        | Child stdout/stderr/PTY bytes may include terminal escape sequences, huge output, or content later trusted by tools. Backpressure can stall the child; descendants keep pipes open.        | Drain concurrently with byte limits, deadlines and terminal sanitization; after child exit end descendants before waiting indefinitely for EOF.                  |
| Persistence/trust handoff — harness responsibility               | Build scripts, generated source and new project-specific config can be run later by an unrestricted host tool, despite sandbox-time path restrictions.                                     | Quarantine/review artifacts; run later builds/tools under a sandbox too. Handoff audit is advisory, not a release gate unless the launcher makes it one.         |
| Resource exhaustion and kernel bugs — stronger boundary          | Optional rlimits are not complete per-run CPU/memory/process quotas, and same-kernel isolation cannot protect against kernel vulnerabilities.                                              | Add cgroups/service limits where available; use a container with deliberate policy or microVM for stronger boundaries and crash-resistant lifecycle.             |

FD hygiene closes undesignated descriptors numbered ≥3 at exec, retaining standard streams and explicitly passed `BorrowedFd`s. That means **stdio itself can be authority**: a socket or writable secret file used as stdin/stdout is not magically made harmless. Existing TCP/UDP sockets are especially important on older Landlock ABIs. Explicit inherited FDs also can carry directories, IPC, or shared memory. The environment implementation depends on a checked interpretation of Rust `Command` debug output to detect `env_clear`; passthrough fails if that interpretation is unavailable. [Environment implementation](https://docs.rs/crate/procage/0.7.1/source/src/hygiene.rs), [command hygiene](https://docs.rs/crate/procage/0.7.1/source/src/unix.rs), [audits and limits](https://docs.rs/crate/procage/0.7.1/source/src/audit.rs).

### Diagnostics are not enforcement receipts

| Output             | What it means                                                                                                                                                   | What it does not establish                                                                                                         |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| macOS violations   | Best-effort unified-log observations, merged by repeated denial; availability and fencing/completeness are exposed. V3 intentionally suppresses routine probes. | Every kernel denial was delivered, or an empty record means no denial happened.                                                    |
| Linux violations   | Refused calls from procage's seccomp filter, answered EPERM by its supervisor thread when listener reporting works.                                             | Landlock filesystem/network/signal denials, mount-cover denials, or calls denied first by an outer container filter were reported. |
| `Child::receipt()` | Unsigned record of policy digest, backend/tier/kernel/OS, approximations, relaxations, waivers, exit and audit/report summaries; optional full policy paths.    | Attestation, tamper resistance, exhaustive enforcement, successful containment, or safe output.                                    |
| Handoff audit      | Before/after observations of suspicious files, links and repository metadata in writable trees; completeness matters.                                           | Prevention of those changes or proof all trust sinks were identified.                                                              |

Reporting unavailable is intentionally compatible with continued **enforcement**. A launcher that needs audit availability must separately gate on coverage/completeness. Linux listener absence does not permit denied syscalls. Treat target/path strings in diagnostics as untrusted text. Receipts belong outside all writable grants, and `with_policy_in_receipt` may disclose paths. [Violation types](https://docs.rs/crate/procage/0.7.1/source/src/violation.rs), [Linux reporting](https://docs.rs/crate/procage/0.7.1/source/src/linux/violations.rs), [macOS reporting](https://docs.rs/crate/procage/0.7.1/source/src/macos/violations.rs), [receipt format](https://docs.rs/crate/procage/0.7.1/source/src/receipt.rs).

### Known defects, fixes, and evidence conflicts

Material published fixes are the 0.2 service/container handoff, hard-link and process-introspection changes; 0.3 descriptor and move/link fixes; 0.4.1 mount-target race and case-sensitive-path corrections; and 0.5 concurrent macOS cleanup correction. These fixes are included in 0.7.1. They warrant regression probes even on a newer patch release. R-07 remains a documented harness responsibility; R-26 helper replacement is marked planned but procage installs no helper, so it applies to caller-built helpers. Kernel and OS-service bugs are out of scope. [Versioned changelog](https://github.com/hupe1980/procage/blob/v0.7.1/CHANGELOG.md), [released escape catalog](https://github.com/hupe1980/procage/blob/v0.7.1/crates/procage/THREAT_MODEL.md).

The [public GitHub advisory endpoint](https://api.github.com/repos/hupe1980/procage/security-advisories) returned an empty list at this research cutoff. A RustSec search did not identify a procage-specific advisory; this was not an exhaustive dependency audit. Private reports may exist, and no advisory is not evidence of security. The security policy requests private reports with exact OS, version, policy and reproducer. The repository's canaries and kernel-facts suites are useful observable maintenance evidence, but their passing claims were not independently reproduced across the advertised matrix here.

## Complete Rust examples and verification

These files form one small Cargo package, separate from the monorepo workspace. They are intentionally conservative: no unsandboxed retry, no class waivers, no passed FDs, null standard streams, and explicit cleanup. They demonstrate APIs, not a production output collector or a security proof. All examples target macOS/Linux; gate this integration out of a native Windows build.

Use existing **disjoint** input and workspace directories controlled by the launcher. Put a newline-terminated `input.txt` in the input directory. Neither may contain sensitive data or untrusted setup-time mutations. macOS requires a case-insensitive volume for these allows, compatible libsandbox, and an unsandboxed supervisor. Linux needs a usable tier; examples with a writable tree may reject T2 and reject T1 until snapshot consent is separately supplied. No external helper installation is required. The proxy demo additionally needs `/usr/bin/curl` with its runtime files allowed.

### Cargo.toml and shared policy builder

The direct policy/proxy pins constrain companion versions too; procage's own manifest uses compatible-version requirements rather than exact pins. Retain and review Cargo.lock for transitive reproducibility. The no-feature synchronous path can omit the Tokio/proxy features and dependencies; both are enabled here so every listed binary compiles together.

```toml
[package]
name = "procage-launch-examples"
version = "0.0.0"
edition = "2024"
rust-version = "1.99"

[workspace]

[dependencies]
procage = { version = "=0.7.1", default-features = false, features = ["tokio", "proxy"] }
procage-policy = "=0.7.1"
procage-proxy = "=0.7.1"
tokio = { version = "1.47", default-features = false, features = ["rt", "process", "time", "net"] }
```

`src/lib.rs` builds the shared intent. The writable workspace is static policy; the read-only input uses a per-run named grant. `Base::V3` grants runtime/system access beyond the input; “read-only inputs” does not mean “the only readable bytes on the host.” No explicit workspace exec grant is added, but interpreters can still read and run scripts from readable paths; an exec restriction is not a code-provenance boundary.

```rust
use std::{error::Error, io, os::unix::ffi::OsStrExt, path::PathBuf};
use procage::native::Grants;
use procage::policy::{AbsPath, Access, Base, FsRule, FsTarget, Grant, GrantName, Network, Policy};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub fn setup() -> Result<(Policy, Grants, PathBuf, PathBuf)> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err(io::Error::other("usage: PROGRAM INPUT_DIR WORKSPACE_DIR").into());
    }
    let input = std::fs::canonicalize(&args[0])?;
    let workspace = std::fs::canonicalize(&args[1])?;
    if !input.is_dir() || !workspace.is_dir()
        || input.starts_with(&workspace) || workspace.starts_with(&input)
    {
        return Err(io::Error::other("use existing, disjoint directories").into());
    }
    let mut policy = Policy::default();
    policy.base = Base::V3;
    policy.net = Network::None;
    policy.process.fork = true;
    policy.fs.push(FsRule::new(Access::Write, FsTarget::Subpath(
        AbsPath::new(workspace.as_os_str().as_bytes().to_vec())?
    )));
    policy.grants.push(Grant { name: GrantName::new("inputs")?, access: Access::Read });
    let grants = Grants::new().bind("inputs", &input);
    Ok((policy, grants, input, workspace))
}
```

### Strict no-network run: src/bin/strict.rs

This requires the policy **as written**. On Linux a writable workspace reaches trust-handoff names, so strict T1 may return an approximation gap and T2 an unsupported gap. That refusal is the correct outcome for this example's requirements. On a qualifying macOS host, the shell reads the input and writes `result.txt` in the workspace. New network sockets and writes through the input path are not allowed. To demonstrate the denied operations, use the isolated tests below rather than interpreting the shell's exit as proof.

```rust
use std::process::{Command, Stdio};
use procage::native::Cage;
use procage_launch_examples::{Result, setup};

fn main() -> Result<()> {
    let (policy, grants, input, workspace) = setup()?;
    let cage = Cage::new(&policy)?;
    eprintln!("plan: {:#?}", cage.plan());
    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c")
        .arg("IFS= read -r line < \"$1\" && printf '%s\\n' \"$line\" > result.txt")
        .arg("worker").arg(input.join("input.txt"))
        .current_dir(&workspace).env_clear().env("TMPDIR", &workspace)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = cage.spawn(&mut cmd, &grants)?;
    let waited = child.wait();
    child.kill_all()?;
    let status = waited?;
    if !status.success() {
        return Err(std::io::Error::other(format!("worker failed: {status}")).into());
    }
    Ok(())
}
```

`waited` is saved before `kill_all` so cleanup runs even if waiting returns an error. Cleanup failure fails the program. Standard stderr is null to avoid exposing untrusted terminal control bytes in this focused example; a real launcher should capture and sanitize it. `Drop` remains the fallback on earlier errors. This example has no wall-clock timeout; the async example supplies one.

### Explicit snapshot acceptance: src/bin/snapshot.rs

This is **inspection only**, a separate integration decision. It accepts exactly `HdTrustHandoff` approximation, not all spawn snapshots and not a waiver. On Linux T1 that permits masks of existing trust-handoff paths; new matching files can remain writable and must never flow unchecked into later trusted execution. T2 still fails. macOS may need no such approximation. After reviewing this result, a launcher could use this same constructor in the strict run only if that weaker requirement is acceptable to its product. Additional overlapping home/class requirements still fail.

```rust
use procage::native::{Cage, CageOptions};
use procage::policy::{Construct, LowerOptions};
use procage_launch_examples::{Result, setup};

fn main() -> Result<()> {
    let (policy, _, _, _) = setup()?;
    let lower = LowerOptions::default().allow(Construct::HdTrustHandoff);
    let cage = Cage::with_options(&policy, &CageOptions::default().with_lower(lower))?;
    for change in &cage.plan().degradations {
        if change.construct != Construct::HdTrustHandoff {
            return Err(std::io::Error::other("unexpected approximation").into());
        }
        eprintln!("accepted approximation: {change:?}");
    }
    if !cage.plan().waivers.is_empty() {
        return Err(std::io::Error::other("unexpected hard-deny waiver").into());
    }
    eprintln!("effective plan: {:#?}", cage.plan());
    Ok(())
}
```

### Host fitting: src/bin/fit.rs

This independent example asks for loopback, but expressly permits **removing** it when unsupported. It prints every change and the effective native plan and launches nothing. Linux should report the no-loopback relaxation; macOS can keep loopback. This is a stricter fit, not authority broadening. The same API's `leave_out_protect`, `leave_out_hide`, `waive(class)` and `signals_same_user` can weaken isolation: review the returned `Relaxed` records and do not enable them as generic fallback defaults. Approximation consent remains separate.

```rust
use procage::native::{Cage, CageOptions};
use procage::policy::{Network, Policy, Ports, Relax};
use procage_launch_examples::Result;

fn main() -> Result<()> {
    let mut policy = Policy::default();
    policy.net = Network::Loopback(Ports::Any);
    let (cage, changes) = Cage::fit(
        &policy, &CageOptions::default(), &Relax::new().no_loopback()
    )?;
    for change in &changes {
        eprintln!("accepted change: {change}");
    }
    eprintln!("effective plan: {:#?}", cage.plan());
    // Inspection only. No workload is launched under a changed requirement.
    Ok(())
}
```

### Approved-destination async run: src/bin/proxy_async.rs

This requires proxy-only networking to `example.com:443` and a writable workspace. It deliberately retains strict filesystem requirements, so a Linux hard-deny gap still fails before launch. A Linux deployment may separately accept the one snapshot from the prior example after review; it must also require T1 and ABI ≥4 for the proxy. On macOS the integrated proxy picks fresh ports; Linux uses the declared namespace port. The program obtains no direct IP egress by changing its environment. HTTPS response contents and anything the approved service relays remain outside hostname-policy guarantees.

```rust
use std::{io, process::Stdio, time::Duration};
use procage::native::{Cage, CageOptions};
use procage::policy::{Network, Port};
use procage::proxy::{Allowlist, Proxy};
use procage_launch_examples::{Result, setup};

fn main() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all().build()?;
    runtime.block_on(run())
}

async fn run() -> Result<()> {
    let (mut policy, grants, _, workspace) = setup()?;
    policy.net = Network::Proxy { http: Some(Port::new(18080)?), socks: None };
    let proxy = Proxy::new(Allowlist::new(["example.com:443"])?);
    let options = CageOptions::default().with_proxy(proxy);
    let cage = Cage::with_options(&policy, &options)?;
    let mut cmd = tokio::process::Command::new("/usr/bin/curl");
    cmd.args(["--fail", "--silent", "--show-error", "--max-time", "10",
              "--output", "response", "https://example.com/"])
        .current_dir(&workspace).env_clear()
        .env("TMPDIR", &workspace)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = cage.spawn_async(&mut cmd, &grants)?;
    let waited = tokio::time::timeout(Duration::from_secs(15), child.wait()).await;
    child.kill_all().await?;
    let status = waited.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "worker timed out"))??;
    if !status.success() {
        return Err(io::Error::other(format!("curl failed: {status}")).into());
    }
    Ok(())
}
```

The timeout cancels only the borrowed wait future; the code retains the `AsyncChild` and calls `kill_all().await` before returning either success or failure. No executable-replacement example is needed to obtain async launch: ordinary async spawning retains the stronger procage-owned child lifecycle. Use the dedicated upstream helper linked earlier if integration requires exec instead, and adopt its different owner contract.

### Verification performed

On **macOS 27.2, Darwin 27.2.0, AArch64**, detected with `sniff os`, all four binaries above were **compile-checked** from registry dependencies using **rustc 1.101.0-nightly (f9ceb4a4d, 2026-10-31)** and Cargo's nightly toolchain:

```sh
cargo +nightly check --manifest-path Cargo.toml --bins --color=never
```

Installed stable was **rustc 1.98.1 (48a229cea, 2026-09-01)**, below the declared MSRV; stable 1.99 compatibility was **not** tested. Compilation succeeded without modifying published source or bypassing MSRV. This is compile evidence for the examples on macOS, not cross-platform enforcement evidence.

The strict example was also **attempted** using disposable fixtures within this worktree. It returned `SpawnError::CaseSensitive` for the workspace on `/Volumes/coding/...`; no workload ran and no result file was produced. That verifies a real fail-closed preparation path on this host. It does **not** verify successful Seatbelt enforcement. The proxy, fitting and snapshot examples were compile-checked only; no outbound network request or successful sandboxed workload execution was performed. Linux/container/WSL2 enforcement tests remain unperformed in this research session.

### Acceptance tests before deployment

Run each probe as its own sandboxed child under an unrestricted supervisor/test runner, with a control proving the operation succeeds when expressly allowed. Use disposable input/output trees, a test-only canary secret, local test endpoints, bounded output and hard deadlines. Do not sandbox the test runner or alter persistent sysctls, LSM configuration or host mounts.

| Test                                | Required assertion                                                                                                                                                                                                                   |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Read-only input and writable output | Input read and output creation succeed; input overwrite/truncate/delete, sibling-directory reads and outside writes fail. Verify data, not only child exit.                                                                          |
| Runtime access                      | Loader, interpreter, compiler, certificate and cache requirements work only through intended grants/base. Explicitly inspect extra readable trees.                                                                                   |
| Strict incompatibility              | Synthetic `lower` tests reject T2 nested protect/hide, Linux globs/loopback and old-ABI signal scope; runtime failure leaves a sentinel executable unexecuted.                                                                       |
| Accepted compatibility              | Check exact degradation/relaxation records, plan and waivers; create matching trust-handoff names after launch to measure snapshot limits.                                                                                           |
| Descriptor/environment hygiene      | Open a canary FD in the supervisor, ensure absent without handoff; inspect the child's environment for non-allowlisted values. Repeat with socket stdio/passed FDs only in disposable tests of the documented exception.             |
| Network none / proxy                | Test IPv4, IPv6, TCP, UDP, direct DNS, Unix/abstract sockets, loopback daemons and other-run ports. Under proxy, approved host succeeds; unapproved hosts, private DNS results, malformed host spellings and direct routes fail.     |
| Aliases and races                   | Symlinks, hard links, case variants, firmlinks and mount-target replacement; replaced target must fail with `MountMoved` rather than cover the wrong object.                                                                         |
| Ownership                           | Direct child exits leaving a grandchild, double fork, new process groups, attempted setsid, concurrent runs, wait cancellation, handle drop and explicit kill. Record survivor identities/start times, avoiding killing reused PIDs. |
| Failure and reporting               | Missing program, invalid binding, unavailable namespace/ABI, report collector unavailable and failed exec. Assert target did not run; distinguish report gaps from enforcement gaps.                                                 |
| Host trust handoff                  | Check audit completeness, newly created hooks/config/links, output sanitization and bounded pipe draining after direct-child exit. No generated artifact is executed unrestricted during testing.                                    |

Fork churn, supervisor SIGKILL, incomplete process discovery, descriptor-transfer daemons and WSL interop escape attempts belong in a disposable VM/container with an independently controlled cleanup boundary. macOS marker PID-reuse testing particularly should not run alongside important same-user processes. A unit test of policy strings or a successful cross-compile cannot replace these kernel probes. Upstream's [escape tests](https://github.com/hupe1980/procage/tree/v0.7.1/crates/procage/tests) and [kernel-facts crate](https://github.com/hupe1980/procage/tree/v0.7.1/crates/procage-kernel-facts) are primary starting points, not a reason to skip product-specific tests.

## Alternatives and complementary layers

The comparison is about integration shape, not a substitute security audit of every alternative. Versions below are those visible in their API documentation during research; check their own release histories before adopting.

| Option                                                         | Coverage and policy                                                                                                                                            | Degradation and lifecycle                                                                                                                                                                                                                                          | Deployment / when to choose                                                                                                                                         |
| -------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **procage 0.7.1**                                              | Embedded macOS Seatbelt + Linux T1/T2; portable intent, raw layers, hard-deny classes, typed lowering.                                                         | Named approximations/fit; owned sync/async child; inspectable plan and unsigned receipts. Crash cleanup caveats above.                                                                                                                                             | No launcher binary/root; Rust 1.99, private Apple ABI, short public history. Good when embedding and explicit rejection are central.                                |
| **nono 0.81.0**                                                | Rust capability library uses Landlock/Seatbelt; its higher-level CLI adds agent/tool/credential policy and proxy workflows. “Windows” coverage refers to WSL2. | Library `Sandbox::apply_auto(&caps)` irreversibly confines the caller; it is not a procage-owned `Child`. CLI supervision is a separate integration choice. Compare explicit scope options and support information rather than assuming the same gap/fit protocol. | More product features and configuration. Useful for agent-specific credential mediation/recovery; keep its self-sandbox call out of the unrestricted supervisor.    |
| **birdcage 0.8.1**                                             | Rust process-sandbox wrapper for macOS/Linux with `Birdcage`, `Exception`, `Sandbox` and its own process command.                                              | Simpler exception-oriented API; procage's tier/gap/fit protocol is not an interchangeable interface. Validate descriptor and descendant ownership separately.                                                                                                      | Alternative embedded wrapper when its smaller API meets requirements; compare actual source/tests for target kernels before choosing.                               |
| **Direct landlock 0.4.7 + seccompiler 0.5.0 + namespace code** | Linux bindings/filter compiler; maximum control over per-kernel policy. Separate native macOS implementation required.                                         | Landlock's compatibility API includes best-effort and hard requirements; enforce a deliberate strict mode/status check. No complete cross-platform child supervisor supplied by these bindings.                                                                    | More engineering and audit cost, but can expose native capabilities procage does not. They are competing lower-level building blocks, not all procage dependencies. |
| **Bubblewrap**                                                 | Linux external namespace/filesystem launcher, including PID namespace/private root construction; caller builds command-line policy.                            | Caller owns policy correctness; explicit namespace/session/death options differ from procage's handle contract. No portable Seatbelt lowering.                                                                                                                     | System package/executable dependency and user-namespace availability. Useful when filesystem layout and PID isolation matter more than a shared Rust API.           |
| **Service/cgroup, container, microVM**                         | Complementary quotas/lifecycle; a VM adds a separate kernel boundary.                                                                                          | Can provide independent crash cleanup and stronger resource separation; still need safe I/O and filesystem/network policy.                                                                                                                                         | Choose another layer when same-kernel escape or orphan survival is unacceptable. A default container is not automatically an equivalent sandbox.                    |

Primary evidence: [nono README and current workflows](https://github.com/nolabs-ai/nono), [nono 0.81.0 Rust API](https://docs.rs/nono/0.81.0/nono/), [birdcage API](https://docs.rs/birdcage/0.8.1/birdcage/), [Landlock bindings and compatibility](https://docs.rs/landlock/0.4.7/landlock/), [seccompiler API](https://docs.rs/seccompiler/0.5.0/seccompiler/), [Bubblewrap security model](https://github.com/containers/bubblewrap/blob/main/README.md), [Bubblewrap options](https://github.com/containers/bubblewrap/blob/main/bwrap.xml).

Maintenance evidence differs: procage's relevant Rust implementation was published November 1–3, 2026; [nono 0.81.0 was published October 29, 2026](https://crates.io/api/v1/crates/nono), while [birdcage 0.8.1 was published April 19, 2024](https://crates.io/api/v1/crates/birdcage), with no later registry release at the cutoff. Birdcage's longer release gap is an adoption question, not proof its repository is abandoned. Bubblewrap documents longstanding Flatpak use and distribution packaging. Those observations do not establish comparative audit quality. Avoid choosing on version numbers or apparent release frequency alone. For a small Rust launcher, compare procage/birdcage first; for agent-specific secrets/proxy workflows, evaluate nono; for Linux PID/filesystem isolation, evaluate Bubblewrap or direct namespaces; for an unacceptable kernel-escape risk, move the workload into a VM.

## Authoritative resources and adoption questions

- [Registry identity and full release metadata](https://crates.io/api/v1/crates/procage), [owners](https://crates.io/api/v1/crates/procage/owners), and [exact published archive](https://static.crates.io/crates/procage/procage-0.7.1.crate).
- [procage 0.7.1 API](https://docs.rs/procage/0.7.1/procage/), [published source](https://docs.rs/crate/procage/0.7.1/source/), [policy API](https://docs.rs/procage-policy/0.7.1/procage_policy/), and [proxy API](https://docs.rs/procage-proxy/0.7.1/procage_proxy/). Initial browser fetches of the versioned docs failed; direct HTTP retrieval and published archive inspection succeeded. The examples were checked against downloaded registry code, not guessed from missing pages.
- [Versioned changelog](https://github.com/hupe1980/procage/blob/v0.7.1/CHANGELOG.md), [threat model](https://github.com/hupe1980/procage/blob/v0.7.1/crates/procage/THREAT_MODEL.md), and [security policy](https://github.com/hupe1980/procage/blob/v0.7.1/SECURITY.md).
- [Official Landlock mechanism/limitations](https://docs.kernel.org/userspace-api/landlock.html), [seccomp](https://docs.kernel.org/userspace-api/seccomp_filter.html), [namespaces](https://man7.org/linux/man-pages/man7/namespaces.7.html), [parent-death signals](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html), and [Apple's supported App Sandbox model](https://developer.apple.com/documentation/security/app-sandbox).

Unresolved questions that materially affect adoption:

1. Will the maintainer confirm the supported Rust minor and current macOS builds, correcting the stale security table and root README? What response/backport policy is actually available for this fast-changing release line?
2. Can the product accept T1 trust-handoff snapshots and reject T2 writable-workspace policies, or does it need an output-only staging design/stronger isolation layer? Which exact relaxations are acceptable requirements changes?
3. Is deployment possible on case-insensitive macOS storage with an unsandboxed supervisor, given the observed refusal on this worktree and private-ABI risk?
4. What owns crash cleanup, including supervisor SIGKILL and descendant churn, independently of procage's bounded scans and drop behavior?
5. Which Linux kernels, container profiles and WSL2 configurations pass the product's real allow/deny/alias/network tests, including Windows interop? No general support claim closes that evidence gap.
6. Which runtime/toolchain trees, standard streams, descriptors, Unix services and approved destinations are genuinely required, and can the caller keep all resulting artifacts and diagnostics untrusted after the run?
