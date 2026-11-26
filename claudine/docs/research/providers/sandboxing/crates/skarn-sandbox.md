---
prompt: |-
    Research the Rust `skarn-sandbox` crate to help a developer decide whether
    and how to use it in a sandbox launcher intended to run across operating systems.

    Research `skarn-sandbox` itself. The original placeholder named `rappct`; treat that
    as a copy-paste error, not evidence of an alias or dependency. Verify the exact
    package identity and the claim that it supports multiple operating systems.

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
         each OS. Separate standalone crate behavior from any larger Skarn
         project, companion packages, external launchers, or unreleased work.
       - Provide a per-OS capability matrix covering filesystem and networking
         restrictions, process/IPC controls, resource limits, and dependencies.
       - Explain the policy model and translation to native mechanisms,
         including default permissions, path semantics, and backend differences.
       - Trace unsupported requirements and runtime failures: determine whether
         policy is rejected, weakened, ignored, or replaced by a fallback.
       - Determine whether callers can inspect effective enforcement and require
         the same guarantees before launching workloads on different OSes.
       - Verify minimum OS versions, privileges, signing or packaging needs,
         helper-process requirements, and independent integration support.
         Distinguish native Windows from WSL2 and implemented support from plans.
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
hash: 1f3085e72888fa1d-e0d103400488e22f
last_updated: 2026-11-03
---

# skarn-sandbox: cross-platform launcher assessment

**Assessment:** `skarn-sandbox` is a real, published Rust library with implemented macOS, Linux, and native Windows backends. It is useful as a small source of native confinement primitives, but **version 1.0.1 is not sufficient by itself for a launcher promising equivalent filesystem isolation, complete network denial, and safe workload lifecycle management across those systems**. In particular, Linux `DenyAll` only handles TCP; macOS defaults to broad reads; Windows accumulates persistent filesystem grants under a shared identity. Its enforcement reports do not reliably identify these differences. Adoption for malicious workloads needs backend-specific validation and, for several requirements, changes to the implementation or another isolation layer. These conclusions come from the released backend implementations, rather than the names of their policy variants. [Published source](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/)

Research date: **November 3, 2026**. Examined release: **`skarn-sandbox` 1.0.1**, published **June 24, 2026, 09:43:49 UTC**, the latest stable, non-yanked release returned by the registry on that date. The package archive, including its normalized manifest and all backend source files, was inspected. Example compilation and behavioral evidence below are local macOS evidence only. [Registry metadata](https://crates.io/api/v1/crates/skarn-sandbox)

## Identity, release history, and maintenance

The Cargo package is `skarn-sandbox`; its Rust import is `skarn_sandbox`. Registry metadata links directly to **[Rani367/Skarn](https://github.com/Rani367/Skarn)**. The archive records repository revision `82d7caa37f280eef735f92b4154f991de5cec919`, subdirectory `crates/skarn-sandbox`. The listed author is “Skarn contributors”; the registry owner and publisher is **Rani367 (Rani)**. Licensing is **MIT OR Apache-2.0**. It is independently usable as a library dependency; installing the Skarn gateway is not required. There is no evidence here that `rappct` is its alias or dependency: that name was a placeholder error. [Owners](https://crates.io/api/v1/crates/skarn-sandbox/owners), [published manifest](https://docs.rs/crate/skarn-sandbox/1.0.1/source/Cargo.toml), [archive revision](https://docs.rs/crate/skarn-sandbox/1.0.1/source/.cargo_vcs_info.json)

Skarn is the parent MCP gateway project. Other packages provide compression, a QuickJS Code Mode engine, gateway services, and the `skarn` CLI. The engine's memory/time/tool-call limits and its brokered tool bridge are **not functionality supplied by `skarn-sandbox`**. Likewise, a Skarn gateway's decision to run a worker is separate from availability of this library's Windows launcher. [Repository architecture](https://github.com/Rani367/Skarn#architecture), [Code Mode API](https://docs.rs/skarn-codemode/1.0.1/skarn_codemode/)

### Releases in the preceding 24 months

The registry's entire version list contains two entries; neither is yanked or a prerelease. There is no published pre-1.0 migration path to describe. [Version metadata](https://crates.io/api/v1/crates/skarn-sandbox)

| Release | Publication date, UTC | What changed                                                                                                                                                                               |
| ------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1.0.0   | June 23, 2026         | First registry publication; already contains the three backends and current policy/report model.                                                                                           |
| 1.0.1   | June 24, 2026         | Repository release commit describes a version-only bump to refresh the parent README. Comparison of every Rust source file in the two published sandbox archives found identical contents. |

The [1.0.1 release commit](https://github.com/Rani367/Skarn/commit/b84e069fd8172d8614bdd75784e4de869bb4afae) explains the bump. [GitHub releases](https://github.com/Rani367/Skarn/releases) contain tags for those two releases but no substantive release-note bodies. Repository development predates publication: the sandbox path history includes Linux work on January 31, Windows work throughout February, and its latest listed path change on March 26, 2026. Those are development commits, not additional crate releases. [Sandbox history](https://github.com/Rani367/Skarn/commits/main/crates/skarn-sandbox)

The registry declares **Rust 1.95**, edition **2024**, for both releases. There are **no crate Cargo feature flags**, including no default feature set or opt-in backend switches. Target selection is automatic: `libc` on Unix; `landlock ^0.4` and `seccompiler ^0.5` on Linux; `windows ^0.62` with the necessary Win32 API features on Windows. Common dependencies include `skarn-common`, Serde/JSON, tracing, and thiserror. No bundled kernel module or native C build is declared. Native OS facilities still must be present and usable. [Manifest](https://docs.rs/crate/skarn-sandbox/1.0.1/source/Cargo.toml)

The package has reached 1.0, but no separate long-term API/MSRV compatibility policy was found. Public policy fields and string diagnostics matter to integrations: a dependency update can change translation even without changing your policy construction. Pin this release while evaluating, retain `Cargo.lock` to fix transitive versions, and review backend behavior before upgrading. There is no source-level API migration between 1.0.0 and 1.0.1. This is an integration recommendation, not an upstream compatibility promise.

**Maintenance judgment:** observable activity is concentrated in early 2026, with two publications one day apart and no subsequent release through the research date. The repository's latest default-branch commit returned by the API was June 24, 2026 (`96f08037…`, CLI completions). The project itself calls Windows its least exercised backend. Its CI configuration builds/tests all three OSes, but the published sandbox runtime test suite is `#![cfg(unix)]`; a Windows build job is not Windows confinement coverage. This supports treating it as a young project requiring adoption work, not a conclusion that it is abandoned or secure. [Commit history](https://github.com/Rani367/Skarn/commits/main), [project status](https://github.com/Rani367/Skarn#status), [release CI](https://github.com/Rani367/Skarn/blob/82d7caa37f280eef735f92b4154f991de5cec919/.github/workflows/ci.yml), [published runtime tests](https://docs.rs/crate/skarn-sandbox/1.0.1/source/tests/runtime.rs)

## Public API and launch model

### Policy construction

`Policy::builder()` returns `PolicyBuilder`. Its consuming methods are `read`, `read_write`, `exec`, `deny_read`, `workspace`, `net`, `allow_read_system`, `fail_closed`, and `build`. A `Policy` also exposes its fields publicly and supports Serde serialization. `workspace(dir)` adds read/write permission, appends `default_secret_paths()`, enables system reads, and selects `DenyAll`; it does not create a private workspace or validate isolation. `default_secret_paths()` uses `HOME`, returning an empty list if it is absent. It does not discover Windows known folders. [Published policy implementation](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/lib.rs)

| Default field                                         | Value                | Consequence                                                                                                     |
| ----------------------------------------------------- | -------------------- | --------------------------------------------------------------------------------------------------------------- |
| `fs_read`, `fs_read_write`, `fs_exec`, `fs_deny_read` | Empty vectors        | No explicit user grants or secret exclusions. Plain `Policy::default()` does not add secret paths.              |
| `net`                                                 | `NetPolicy::DenyAll` | Backend-dependent translation, detailed below.                                                                  |
| `allow_read_system`                                   | `true`               | Especially broad on macOS.                                                                                      |
| `fail_closed`                                         | `true`               | Rejects `NotEnforced` from self-application; does **not** reject partial enforcement or every lost requirement. |

Lists of filesystem grants accumulate. There is no portable “deny overrides allow” rule: Linux ignores the deny list, Windows ignores it, and macOS emits explicit SBPL denies and allows whose overlaps need native semantic testing. Keep grants disjoint from secrets and from read-only input trees. There is no API to request a set of required native rights or to validate the whole policy before applying it. [Policy dispatch](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/lib.rs), [backend source](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/)

### Unix execution

The application starts a **trusted, fresh, single-threaded worker**, which constructs the policy and calls `policy.apply_to_current_process()`. On success it checks the returned `RestrictionReport` before processing untrusted input or executing the target. The crate provides no portable `Sandbox::spawn` abstraction: the caller owns the worker executable, policy transport, descriptor/environment cleanup, target execution, reporting, and process-tree supervision. The included `skarn-sandbox-probe` binary is a test helper, not an installed general-purpose launch service. [Crate execution model](https://docs.rs/skarn-sandbox/1.0.1/skarn_sandbox/), [probe source](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/bin/probe.rs)

Restrictions survive subsequent execution and are inherited by future descendants. Linux enforcement targets the calling thread and its future descendants, not preexisting sibling threads; the seccompiler function used also applies to the calling thread. Do not self-restrict an async supervisor or apply this allocation-heavy code inside a multithreaded parent's `pre_exec` hook. Spawn a clean worker first. Restrictions are not undone by dropping `Policy`; a failure after one enforcement stage can leave that worker partially restricted, so terminate it rather than retrying unconfined. [Linux implementation](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/linux.rs), [seccompiler thread scope](https://docs.rs/seccompiler/0.5.0/seccompiler/fn.apply_filter.html), [kernel enforcement semantics](https://docs.kernel.org/userspace-api/landlock.html#inheritance)

### Windows execution

Self-application always returns `Error::SandboxUnsupported`. The Windows-only API instead is:

```text
spawn_appcontainer(&Policy, &skarn_common::CommandSpec) -> Result<SandboxChild>
SandboxChild::wait(&self) -> Result<i32>
SandboxChild::wait_with_output(self) -> Result<Captured>
Captured { stdout: Vec<u8>, stderr: Vec<u8>, code: i32 }
```

A direct caller needs a dependency on `skarn-common` to name/build `CommandSpec`. The parent creates or derives an AppContainer SID, modifies filesystem ACLs, prepares capability SIDs and stdio pipes, calls `CreateProcessW` with security capabilities, then creates and assigns a kill-on-close Job Object. `SandboxChild` owns process/thread/job/pipe handles; dropping it closes the job. `wait_with_output` drains stdout/stderr concurrently into unbounded vectors; `wait` does not drain output and can deadlock a chatty child. There is no timeout or output budget. [Windows implementation and exports](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/windows.rs)

This places trust in the supervisor and its IPC, policy validation, and pre-launch setup. A worker that can ask a trusted broker to fetch arbitrary files or URLs has the broker's effective authority regardless of its own network sandbox. Pipes carrying output can disclose secrets even with networking denied. “No network” is not a general information-flow guarantee.

## Platform capability matrix

These entries describe **released 1.0.1 code**, not the parent project's marketing or newer native APIs. [macOS source](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/macos.rs), [Linux source](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/linux.rs), [Windows source](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/windows.rs)

| Environment        | Filesystem                                                                                                                                                                                           | Networking                                                                                                                                                | Processes and IPC                                                                                                                           | Resource/lifecycle controls                                                   | Dependencies and deployment                                                                                                                                            |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| macOS              | Generated Seatbelt SBPL; read/write subtrees; explicit secret denies only when system reads are on. System-read default grants reads under `/`.                                                      | SBPL network rules for the four enum variants; no destination allowlist API.                                                                              | Allows fork, self-signals, all `mach-lookup`, sysctl reads, and file metadata reads. No caller-configurable IPC policy.                     | No CPU, memory, process-count, time, or output limits; no tree ownership API. | Direct deprecated `sandbox_init`/`sandbox_free_error` FFI through system libraries. Caller supplies worker. No crate-managed signing/entitlements/helper installation. |
| Linux              | Landlock handles filesystem rights through ABI 5; positive subtree grants. `fs_deny_read` unused. Read grants also permit execution.                                                                 | `DenyAll` handles TCP bind/connect with no grants; UDP and Unix IPC are outside this policy. Other variants do not install Landlock network restrictions. | Best-effort seccomp syscall denylist; no PID/IPC/user/mount namespaces, no configurable signals or Unix socket scope.                       | No rlimits/cgroups/time/output limits or tree ownership.                      | Enabled Landlock LSM and seccomp; unprivileged self-restriction. `landlock`, `seccompiler`, `libc`. Caller supplies worker.                                            |
| Native Windows     | AppContainer access checks plus **persistent** inheritable grants to program/read/read-write paths. Extra ambient ACL grants remain relevant. `fs_exec`, `fs_deny_read`, `allow_read_system` unused. | Capability SIDs: none, Internet client, or broader client/server capabilities. Loopback request becomes no network capabilities with warning.             | AppContainer token isolation plus job membership; inherited handles and shared identity are significant gaps. No LPAC toggle or IPC policy. | Job kill-on-close only, assigned after launch; no numeric resource limits.    | Win32 AppContainer/Job/ACL APIs; permission to create profile and edit each DACL. No package manifest or companion service required by this direct launch route.       |
| WSL2, Linux target | Same Linux code, subject to guest kernel/LSM support and mounted filesystem behavior.                                                                                                                | Same TCP-only restrictions.                                                                                                                               | Same Linux limits; host interoperability is a separate channel to assess.                                                                   | Same as Linux.                                                                | Not the native Windows backend; test within the actual guest.                                                                                                          |
| Other targets      | Unsupported fallback.                                                                                                                                                                                | None.                                                                                                                                                     | None.                                                                                                                                       | None.                                                                         | `Backend::None`, `NotEnforced`; default self-application rejects.                                                                                                      |

### Linux policy translation and version floor

The backend fixes its desired rights at **`ABI::V5`**, using `CompatLevel::BestEffort`. Landlock began in Linux 5.13; TCP rights arrived in ABI 4/Linux 6.7 and device ioctl control in ABI 5/Linux 6.10. Full enforcement of this requested rights set therefore requires ABI 5 or a corresponding backport, with Landlock enabled and its syscalls permitted. A newer kernel does not make this crate request newer restrictions. For example, it does not request later signal/abstract-socket scoping or UDP controls. Probe native capability, not just a kernel version string. [ABI reference](https://docs.rs/landlock/0.4.7/landlock/enum.ABI.html), [fixed ABI in released source](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/linux.rs)

The underlying Rust `restrict_self()` path normally sets `no_new_privs`, allowing unprivileged enforcement and preventing later execution from gaining privilege through set-user-ID/file capabilities. No root helper is requested by this backend. [Rust enforcement contract](https://docs.rs/landlock/0.4.7/landlock/struct.RulesetCreated.html#method.restrict_self)

It handles all ABI-5 filesystem rights, grants `AccessFs::from_read(V5)` to readable trees and that set plus `from_write(V5)` to writable trees. **`from_read` includes `Execute`**: `.read(input)` is not a no-execute input grant, and `.read_write(work)` permits execution too. An `fs_exec` list does not narrow those other grants. Default runtime trees are `/usr`, `/lib`, `/lib64`, `/bin`, `/sbin`, `/etc`, `/proc/self`; four standard device files also receive read/write access. Missing system paths are silently skipped. Any failure opening a user rule path is also skipped and described as “did not exist,” even if the actual cause was permission failure. [Backend translation](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/linux.rs), [Landlock read-right implementation](https://raw.githubusercontent.com/landlock-lsm/rust-landlock/v0.4.7/src/fs.rs)

Landlock grants concern filesystem objects/hierarchies, not durable textual path identities. Matching allows accumulate; an excluded child of a granted parent is not excluded by omission. This crate never reads `fs_deny_read` on Linux, so `.workspace(home).deny_read(home/.ssh)` still grants the secret tree. Existing file descriptors, filesystem aliases, mounts, metadata operations, and previously connected sockets need separate analysis; this is not a mount namespace or a complete read-only filesystem image. [Landlock filesystem limitations](https://docs.kernel.org/userspace-api/landlock.html#filesystem), [released backend](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/linux.rs)

The seccomp layer defaults to allowing syscalls and returns **`EPERM`**, rather than killing the process, for its listed operations: tracing/process-memory operations, mount/pivot/namespace changes, module operations, BPF, keyring calls, reboot, swap, and kexec. The module comment's “killing syscalls” wording does not match this return action. Networking syscalls, clone/fork, signals, and general IPC are not comprehensively blocked. Seccomp installation failure is only a report note. [Released filter](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/linux.rs), [native seccomp actions](https://docs.kernel.org/userspace-api/seccomp_filter.html)

### macOS policy translation and compatibility

The generator begins with deny-default, then grants baseline process/IPC/metadata operations. With `allow_read_system = true`, it emits `(allow file-read* (subpath "/"))`, followed by denies for the named secret paths. It does **not** deny the whole home directory, despite an inline comment claiming that it does. With `allow_read_system = false`, it emits only the explicit read grants, plus baseline metadata access and four device exceptions. This can support a much narrower worker, but arbitrary interpreters and dynamically loaded tools need carefully enumerated runtime access. The upstream security policy explicitly accepts coarse default read confidentiality for loader compatibility. [Generator](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/macos.rs), [upstream threat model](https://github.com/Rani367/Skarn/blob/82d7caa37f280eef735f92b4154f991de5cec919/SECURITY.md)

An empty `fs_exec` permits execution broadly. A nonempty one still grants execution in the hardcoded system executable directories, even when `allow_read_system` is false. `deny_read` emits no rules at all when system reads are false. Avoid relying on overlapping native deny/re-allow ordering: the source's assertion that a later workspace allow restores a denied tree is not itself evidence of SBPL precedence. The raw profile generator is inside a private module; callers do not get a public generated-profile inspection API. [macOS implementation](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/macos.rs)

Apple's installed SDK header identifies `sandbox_init` as introduced in **10.5**, deprecated in **10.8**, and “No longer supported.” Its documented flags contract reserves values other than `SANDBOX_NAMED`; this crate passes **0** with generated SBPL. The local tests show that route works on macOS 27.2, not that Apple promises its stability. The SDK's `sandbox_init(3)` page directs developers to App Sandbox and says restrictions are typically checked at resource acquisition. This research consulted the actual Apple-supplied SDK header and man page; Apple's older web man-page URL could not be retrieved. Local primary references: `/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk/usr/include/sandbox.h` and `usr/share/man/man3/sandbox_init.3` under that SDK. [Apple's current supported App Sandbox documentation](https://developer.apple.com/documentation/security/app-sandbox)

The crate declares no tested minimum macOS version, entitlement recipe, or signed-helper deployment contract. It directly applies a profile as the current user; it neither installs `sandbox-exec` nor uses an App Sandbox entitlement as its enforcement path. Sandboxing an already sandboxed process can fail according to the SDK header, and a fixed positive `backend_report()` does not test that condition. For an application distributed through the Mac App Store, Apple's App Sandbox requirements are a separate packaging concern, not satisfied by this crate's legacy FFI call. [Apple App Sandbox](https://developer.apple.com/documentation/security/app-sandbox), [apply/probe implementation](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/macos.rs)

### Windows policy translation and deployment hazards

AppContainer uses a restricted application identity and access checks; it is not a per-launch private filesystem. Ordinary AppContainers can access resources already granted to their identity/capabilities or broadly to application packages. Skarn does not request LPAC. It grants `FILE_GENERIC_READ` to input paths, read/execute to the program, and generic read/write/execute to writable paths. Generic write is not equivalent to every POSIX workspace operation, such as deleting any descendant. Protected child DACLs can also prevent recursive grants from behaving as expected. [Windows backend](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/windows.rs), [Microsoft AppContainer model](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation), [Microsoft launch guidance](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer)

Four source-level issues materially affect launcher adoption:

1. **Authority persists and accumulates.** Every launch uses the fixed profile name **`Skarn.Sandbox`**. `grant_access` adds inheritable ACEs to real host objects, with no rollback, ACL restoration, or profile deletion. A later “read-only” request cannot revoke an earlier writable grant. Concurrent or subsequent sandboxes under the same user can receive each other's historical authority. Dropping the child only closes handles and the job. This is a crate lifecycle/design gap, not a necessary property of AppContainer.
2. **A failed launch can leave a running child.** `CreateProcessW` is not suspended; Job Object creation, configuration, and assignment happen afterward. The child can run or spawn descendants before job attachment. A later error returns without a constructed `SandboxChild`, and the error path does not terminate the process or close all acquired handles. AppContainer confinement starts at creation, but job/tree ownership is not atomic with creation.
3. **Handle/environment inheritance is broad.** The launcher passes `bInheritHandles = true` without a `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`; other inheritable supervisor handles can leak. It passes a null environment block, inheriting the parent's environment, and selects the first writable policy path as working directory. A caller cannot configure these through `Policy`. Use of a dedicated clean supervisor reduces exposure but does not repair the library's handle policy.
4. **Partial setup is not reported precisely.** `GetNamedSecurityInfoW` errors are all treated as a skippable missing path and logged at debug level. Deny/exec/system-read requirements are ignored without a structured report. Profile creation errors of any kind trigger an attempt to derive the existing profile SID, obscuring the original cause. There are also manual-allocation/error-cleanup gaps; this is not a transactionally managed native resource lifecycle.

These are findings from the published implementation, not reported upstream advisories or experimentally demonstrated exploits. [Relevant source](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/windows.rs), [CreateProcess inheritance and scheduling contract](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw), [Job Object semantics](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)

The source claims Windows 8+ because AppContainer profiles begin there; Microsoft lists Windows 8/Server 2012 for profile creation. That does not establish the whole Rust 1.95/dependency/API deployment floor, especially for capability SID APIs and imported DLL symbols. No complete minimum-build test matrix is published. Validate a supported modern Windows build instead of advertising Windows 8 compatibility from the probe comment. Standard users can use AppContainer when authorized, but ACL edits require appropriate rights and can fail on other-owned paths or unsuitable filesystems. Executable DLLs, interpreters, and runtime data need their own access; granting just the executable does not provision those dependencies. [Profile creation requirements](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-createappcontainerprofile), [capability SID API](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-derivecapabilitysidsfromname)

## Networking: the enum is not a portable guarantee

| `NetPolicy`     | macOS generated policy                                                                                       | Linux 1.0.1                                                                                        | Windows 1.0.1                                                                                                                  |
| --------------- | ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `DenyAll`       | No network allow rule under deny-default.                                                                    | Handles TCP bind/connect only; UDP, existing connections, and other communication remain concerns. | No network capability SIDs; effective behavior also depends on native network isolation and existing exemptions/configuration. |
| `AllowLoopback` | Emits local/remote `localhost:*` network rules and loopback bind.                                            | Network **unrestricted**, with report note; not merely outbound.                                   | Treated as deny-all capabilities, with tracing warning; no per-launch report.                                                  |
| `AllowOutbound` | Allows `network-outbound` and loopback-local `network-inbound`; not a simple “no inbound binds” translation. | Network **unrestricted**, without a degradation note for this variant.                             | `internetClient`; not arbitrary approved hosts, private networks, or guaranteed loopback access.                               |
| `AllowAll`      | `network*` allowed.                                                                                          | Network unrestricted.                                                                              | Adds `internetClient`, `internetClientServer`, `privateNetworkClientServer`; native access checks still apply.                 |

These translations are directly visible in the [three released backends](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/). Microsoft's [network isolation documentation](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation#network-isolation) explains capability categories; those are not domain identities.

There is **no API for approved IP addresses, DNS names, URLs, or ports**. Even though native Landlock offers port rules, this wrapper exposes none. A port such as 443 would not establish a destination identity anyway. For approved-destination egress, use a separately confined proxy/broker that validates destinations and denies direct bypass; include DNS, redirects, IPv6, UDP/QUIC, local services, and inherited sockets in the design. Complete Linux offline operation requires another layer, such as an appropriately configured network namespace and descriptor cleanup. These are recommendations; neither proxy nor namespace is supplied here.

## Detection, failures, and fail-closed integration

`backend()` is a compile-target constant. `backend_report()` returns `RestrictionReport { backend, status, notes }` without applying the workload policy. Its limitations are significant:

- macOS and Windows probes return `FullyEnforced` unconditionally for their compiled platform. They do not launch a child, validate paths, check profile application, or inspect network exemptions.
- Linux probes only creation of an ABI-1 best-effort filesystem ruleset. They do not prove ABI-5 enforcement, networking, or seccomp installation. In particular, a best-effort constructor's success is not equivalent to a successful full-policy `restrict_self`.
- Unsupported targets return `Backend::None` and `NotEnforced`.

[Probe implementations](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/)

After Unix self-application, `Policy` rejects **only `NotEnforced`** when `fail_closed` is true. Linux maps the Landlock result to `FullyEnforced`, `PartiallyEnforced`, or `NotEnforced`; it does not lower that status for ignored deny lists, unrestricted loopback/outbound modes, skipped paths, or seccomp failure. macOS reports full after `sandbox_init` returns success. Windows `spawn_appcontainer` returns a child, not a `RestrictionReport`, and does not implement a `fail_closed` policy-validation gate. An error from spawn can occur after launch, as described above. [Dispatch](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/lib.rs), [Linux status mapping](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/linux.rs), [Windows launch](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/windows.rs)

`Error` is re-exported from `skarn-common` and is non-exhaustive. Relevant cases are `Sandbox(String)`, `SandboxUnsupported(String)`, plus I/O and configuration errors. It provides no structured missing-capability list or atomic launch/enforcement receipt. Treat every error as a refusal, reject partial statuses explicitly, and review notes conservatively for this pinned release. String matching is a temporary integration technique, not a stable capability API. [Error API](https://docs.rs/skarn-common/1.0.1/skarn_common/enum.Error.html)

**Can a caller require the same guarantees before launching on every OS?** Not using this API alone. Callers can reject known unrepresentable requirements before invoking it and inspect Unix application reports before executing the target, but cannot obtain a typed effective policy or Windows pre-execution confirmation. A production adapter needs a requirement-to-backend compatibility check, structured backend evidence, and an atomic confined launch path. Reject native Windows for a strict per-launch filesystem contract until persistent identity/ACL and lifecycle issues are addressed; reject Linux for total network denial unless a separate mechanism supplies it.

## Uses, threat boundaries, and gap classification

A reasonable use is reducing accidental filesystem damage from developer tools in a disposable workspace, or adding a kernel layer underneath an already restricted interpreter. The mechanisms can also reduce what malicious code can do, but the released wrapper's translations and lifecycle gaps prevent treating its default builder as a uniform malicious-workload boundary. The parent's security policy excludes kernel exploits, side channels, and hostile multitenancy; its QuickJS resource protections cannot be transferred to arbitrary native commands. [Upstream security policy](https://github.com/Rani367/Skarn/blob/82d7caa37f280eef735f92b4154f991de5cec919/SECURITY.md)

| Requirement or pitfall                                                      | Classification                                                   | Consequence and mitigation                                                                                                                                                                                                      |
| --------------------------------------------------------------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Read-only inputs plus writable workspace                                    | Backend translation and deployment                               | Use disjoint existing directories; disable macOS broad reads for confidentiality; avoid broad Linux ancestors; do not reuse Windows historical ACL authority. “Read-only” does not mean no execution or all metadata immutable. |
| Complete no-network policy                                                  | Missing crate coverage; underlying primitive scope               | Linux ABI-5 TCP rights leave UDP/IPC gaps; add an independent layer. macOS rules still need IPC/broker and inherited-resource review. Windows requires native configuration tests.                                              |
| Approved destinations only                                                  | Missing crate API; destination identity needs application policy | Use a narrow proxy/broker and prevent bypass, rather than treating capabilities or port rules as a host allowlist.                                                                                                              |
| Existing threads, descriptors, sockets, Mach ports, or handles              | Native scope and integration                                     | Apply early in a fresh worker; close unintended descriptors/handles; sanitize environment and dynamic-loader inputs before trusted helper startup. Do not assume application revokes acquired authority.                        |
| Runtime and subprocess compatibility                                        | Integration                                                      | Enumerate loaders, libraries, interpreters, modules, certificates, temp and cache needs. Explicitly provision them; avoid granting a whole home directory to make startup work.                                                 |
| Symlinks, hard links, bind mounts, firmlinks, junctions, and reparse points | Native object/path semantics plus integration                    | Canonicalization alone cannot freeze objects or exclude aliases. Control parent directories/topology and test actual access after resolution. A writable alias of an input defeats the intended read-only arrangement.          |
| CPU, memory, disk, process count, wall time, output exhaustion              | Native facilities missing from crate                             | Add rlimits/cgroups or Windows job limits and bounded supervisor output; use a VM/container design when stronger accounting is required.                                                                                        |
| Shared Windows profile and persistent ACLs                                  | Crate design                                                     | Another `Policy` cannot revoke them. Use a repaired backend with unique identities, narrowly scoped objects, lifecycle ownership and explicit cleanup, or a disposable OS image.                                                |
| Shared-kernel hostile multitenancy                                          | Need for stronger boundary                                       | Use a VM/microVM appropriate to the workload OS, with deliberate file/IPC/network integration.                                                                                                                                  |

`Policy::canonicalized()` has a documentation defect: its doc comment says missing paths are dropped, but the implementation retains the original path when canonicalization fails. Backends subsequently handle those paths differently. It also converts paths through lossy strings in several places. Validate existence, canonicalize with errors propagated, reject unsupported/nonrepresentable paths, and prevent path replacement during setup. This is caller validation, not a crate guarantee. [Actual canonicalization](https://docs.rs/crate/skarn-sandbox/1.0.1/source/src/lib.rs)

### Defects, advisories, and unreleased work

The public repository advisory API returned an empty list on the research date; a targeted RustSec search did not identify a `skarn-sandbox` advisory. This is a bounded search result, **not evidence of an audit or absence of vulnerabilities**, and is not a dependency-lockfile security assessment. [Repository advisories](https://github.com/Rani367/Skarn/security/advisories), [RustSec database](https://rustsec.org/advisories/)

The source findings above apply to both published versions because their Rust sources are identical. They should not be presented as assigned CVEs. Documentation/source conflicts include Linux loopback described as “outbound” instead of unrestricted; the macOS home-directory denial comment; the canonicalization comment; and the Linux claim that `fail_closed` rejects older-kernel degradation, whereas it only rejects `NotEnforced`. This report follows the implementation in each case.

**Open PR #7**, primarily about compression tests, also proposes adding seccomp `connect`/`bind` denials when Linux `DenyAll` is selected. Its patch was inspected separately. It is **unmerged/unreleased** as of the research date and does not change 1.0.1. Even that patch is not proof of complete offline confinement: unconnected UDP `sendto`, existing descriptors, syscall variants, and IPC require their own coverage. [PR discussion](https://github.com/Rani367/Skarn/pull/7), [primary PR file data](https://api.github.com/repos/Rani367/Skarn/pulls/7/files)

The parent project's Windows cross-process **Code Mode** worker is described as future work in its security policy; that is distinct from the already implemented standalone `spawn_appcontainer` function. Do not read either statement as implying the other is absent or complete. [Release security policy](https://github.com/Rani367/Skarn/blob/82d7caa37f280eef735f92b4154f991de5cec919/SECURITY.md)

## Rust example: a narrow worker with explicit refusal

The following complete program demonstrates shared policy construction and **Unix self-application**, then one filesystem or TCP operation. It does not launch an arbitrary external command or demonstrate parity on Windows. On macOS it supports the generated offline socket policy; on Linux it accepts only the explicitly narrower TCP-denial contract. On Windows and other backends it refuses before applying/launching anything because the strict per-run filesystem requirements cannot be established by this release. A misleading three-OS “same guarantees” launch example would hide that gap.

Create a standalone Cargo package with this `Cargo.toml` and the next block as `src/main.rs`:

```toml
[package]
name = "skarn-policy-probe"
version = "0.1.0"
edition = "2024"
rust-version = "1.95"

[dependencies]
skarn-sandbox = "=1.0.1"

[workspace]
```

No features or `.cargo/config.toml` target override are required. Build for the native target. `[workspace]` permits testing this as an independent scratch package inside a larger checkout. Commit the generated lockfile in a real integration. A Windows launcher would additionally need `skarn-common = "=1.0.1"` for `CommandSpec`; adding it does not resolve the Windows issues described above. [Published dependency configuration](https://docs.rs/crate/skarn-sandbox/1.0.1/source/Cargo.toml)

```rust
use skarn_sandbox::{Backend, NetPolicy, Policy, RestrictionStatus};
use std::{error::Error, io, path::PathBuf, time::Duration};

fn refuse(message: &str) -> io::Error {
    io::Error::other(message)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 6 {
        return Err(refuse("usage: probe INPUT_DIR WORK_DIR tcp-only|offline read|write|connect TARGET").into());
    }
    let input = PathBuf::from(&args[1]).canonicalize()?;
    let work = PathBuf::from(&args[2]).canonicalize()?;
    if !input.is_dir() || !work.is_dir()
        || input.starts_with(&work) || work.starts_with(&input)
    {
        return Err(refuse("input and workspace must be existing disjoint directories").into());
    }
    let policy = Policy::builder()
        .read(input)
        .read_write(work)
        .allow_read_system(false)
        .net(NetPolicy::DenyAll)
        .fail_closed(true)
        .build();
    let host = skarn_sandbox::backend_report();
    match (host.backend, args[3].as_str()) {
        (Backend::Seatbelt, "offline" | "tcp-only") => {}
        (Backend::Landlock, "tcp-only") => {}
        _ => return Err(refuse("requested guarantee unavailable with this released backend").into()),
    }
    if host.status != RestrictionStatus::FullyEnforced {
        return Err(refuse("backend preflight failed").into());
    }
    let report = policy.apply_to_current_process()?;
    if report.status != RestrictionStatus::FullyEnforced {
        return Err(refuse("partial enforcement; refusing workload").into());
    }
    if report.backend == Backend::Landlock
        && (report.notes.len() != 1 || report.notes[0] != "seccomp-bpf denylist applied")
    {
        return Err(refuse("missing seccomp or skipped paths; refusing workload").into());
    }
    let operation = match args[4].as_str() {
        "read" => std::fs::read(&args[5]).map(|_| ()),
        "write" => std::fs::write(&args[5], b"probe"),
        "connect" => {
            let address: std::net::SocketAddr = args[5].parse()?;
            std::net::TcpStream::connect_timeout(&address, Duration::from_secs(2))
                .map(|_| ())
        }
        _ => return Err(refuse("unknown operation").into()),
    };
    if let Err(error) = operation {
        eprintln!("operation failed: {error}");
        std::process::exit(10);
    }
    Ok(())
}
```

This worker must be entered through a fresh trusted process, before thread creation or untrusted callbacks. It deliberately uses already-loaded Rust code instead of `exec`, so disabling all system reads can work without provisioning a new executable's loader. The four device exceptions and metadata access still exist on macOS. Linux read grants include execution, and neither backend promises every metadata change is prevented. Disjoint canonical paths are a useful check, not proof against hard-link aliases, mount aliases, hostile parent-directory changes, or authority inherited through other channels. Use a supervisor-owned layout with no sensitive inherited resources.

The Linux note check is intentionally conservative and pinned to 1.0.1: exactly the seccomp-success note, no skipped-path or degradation notes. It does not repair ignored `fs_deny_read`, UDP, or later Landlock scopes; this policy supplies no deny list and explicitly limits its network contract to TCP. **Do not change `offline` to `tcp-only` automatically when a requirement cannot be met.** For total Linux offline isolation, provision and verify a separate mechanism before using an adapter that permits launch.

### Reproducing allowed and denied operations

The following Python driver runs each operation in a separate disposable worker. The parent retains its original privileges and deletes its private temporary tree afterward. Exit 10 means the operation was attempted and failed; exit 1 means argument/setup/enforcement refusal. The checks require a permission errno as well as a failure code, rather than mistaking a refused TCP port or failed sandbox installation for a policy denial.

Save as `verify.py`, then run:

```sh
cargo generate-lockfile
cargo build --locked
python3 verify.py target/debug/skarn-policy-probe offline
```

On Linux, use `tcp-only` only when that narrower contract is acceptable. On Windows, this example deliberately refuses; it does not exercise `spawn_appcontainer`.

```python
import pathlib
import socket
import subprocess
import sys
import tempfile

exe = str(pathlib.Path(sys.argv[1]).resolve())
contract = sys.argv[2]  # offline on macOS; tcp-only on Linux
with tempfile.TemporaryDirectory(prefix="skarn-proof-") as directory:
    root = pathlib.Path(directory)
    inputs, work = root / "inputs", root / "work"
    inputs.mkdir()
    work.mkdir()
    (inputs / "a").write_bytes(b"input")
    (root / "outside").write_bytes(b"outside")
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        listener.listen()
        address = f"127.0.0.1:{listener.getsockname()[1]}"
        with socket.create_connection(listener.getsockname(), timeout=2):
            pass
        control, _ = listener.accept()
        control.close()
        cases = [
            ("read", str(inputs / "a"), 0),
            ("write", str(work / "a"), 0),
            ("write", str(inputs / "a"), 10),
            ("read", str(root / "outside"), 10),
            ("write", str(root / "new"), 10),
            ("connect", address, 10),
        ]
        for operation, target, expected in cases:
            result = subprocess.run(
                [exe, str(inputs), str(work), contract, operation, target],
                capture_output=True, text=True, timeout=5,
                env={"PATH": "/usr/bin:/bin"},
            )
            print(operation, expected, result.returncode, result.stderr.strip())
            assert result.returncode == expected, result.stderr
            if expected == 10:
                assert "operation failed:" in result.stderr
                assert "(os error 1)" in result.stderr or "(os error 13)" in result.stderr
        assert (inputs / "a").read_bytes() == b"input"
        assert (work / "a").read_bytes() == b"probe"
        assert not (root / "new").exists()
    missing = subprocess.run(
        [exe, str(root / "missing"), str(work), contract, "write", str(work / "never")],
        capture_output=True, text=True, timeout=5,
        env={"PATH": "/usr/bin:/bin"},
    )
    assert missing.returncode == 1
    assert not (work / "never").exists()
```

**Verification performed:** `cargo check --locked` and `cargo build --locked` succeeded with **rustc/cargo 1.98.1 on macOS 27.2, native Darwin** against registry `skarn-sandbox 1.0.1`. The exact final worker and driver above executed successfully: two allowed operations, four denied operations with `EPERM`, input contents preserved, outside output absent, and missing-input setup refusal before any workload write. The TCP listener was reachable by the unsandboxed parent. No terminal/browser window was opened. The supervisor was never restricted, and the temporary filesystem fixture was removed. The pinned MSRV 1.95 was **not** tested. Linux, native Windows, WSL2, cross-compilation, arbitrary-program execution, descendant inheritance, and complete offline behavior across all protocols were **not tested in this research**. Those remain separate validation tasks; compilation does not establish enforcement.

Before adopting a launcher, extend child-process tests to cover:

- **Capability absence and degradation:** a Linux guest without Landlock, an ABI below 5, outer seccomp denial, and failure installing the extra filter. Require refusal for the intended contract, not a skipped test. Inject report values in adapter unit tests, then reproduce real kernel cases in disposable guests.
- **Network semantics:** controlled TCP/UDP IPv4/IPv6 endpoints, loopback and LAN, preconnected sockets, DNS, Unix sockets, and disallowed destinations while an unsandboxed control succeeds. Expect UDP confinement to be absent with released Linux code; this should fail an offline acceptance test.
- **Filesystem boundaries:** overwrite/truncate/delete/rename/create, denied parent and child trees, writable aliases of inputs, symlinks, hard links, mounts/reparse points, non-UTF-8 paths, missing paths, and descriptors opened before enforcement. Check actual data and object access, not just path string matches.
- **Startup and descendants:** loader/interpreter access, sanitized environment, inherited descriptor/handle enumeration, errors before target exec, failed target exec, grandchild confinement, timeouts, bounded output, and supervisor death. Use a handshake that confirms enforcement before the supervisor releases work; the crate does not provide it.
- **Windows lifecycle:** use a disposable VM snapshot, since this release edits ACLs and leaves a shared profile. Test read-only after an earlier writable grant, concurrent policies, protected DACL children, network exemptions, leaked inheritable handles, immediate child spawning before job assignment, and failures at every post-creation step. Confirm token identity and job membership with native APIs. Restore the image rather than leaving profile/ACL changes on a developer machine.

For repository-owned automated tests, follow the shared [test toolkit](../../../../tools/test-toolkit/README.md) and run the appropriate nextest/`just` tier. These scratch examples are one-shot child-process probes, not changes to the repository's test suite. Do not apply irreversible restrictions to a nextest runner or a shared test process.

## Alternatives and complementary mechanisms

The neighboring [Landlock](./landlock.md), [Seatbelt](./seatbelt.md), and [AppContainer](./app-container.md) reports supplied context and leads; their claims were checked against native documentation and source for this report. The neighboring `heel`, `rightkit-sandbox`, and unified-API pages were also inspected; placeholder prompts there do not establish functionality. No alternative below was substituted for research on `skarn-sandbox`.

| Option                                                                                       | Relationship and coverage                                                                                                                                                                                                                                                                                                                       | Maintenance evidence and integration cost                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`heel` 0.7.4**                                                                             | Competing higher-level Rust wrapper advertising macOS `sandbox-exec`, Linux Landlock/seccomp, and Windows AppContainer/jobs. Published docs describe separate read/write/execute grants, managed workspaces, resource settings, and proxy-based domain policies. They explicitly disclaim being a general sandbox for arbitrary untrusted code. | Registry publication: November 2, 2026; recent release activity is observable, but is not security assurance. More functionality and integration surface than Skarn; independently verify proxy bypass prevention and backend restrictions. This comparison inspected published documentation, not an enforcement audit. [Package source/README](https://docs.rs/crate/heel/0.7.4/source/README.md), [release metadata](https://crates.io/api/v1/crates/heel)                                              |
| **`rightkit-sandbox` 0.6.2**                                                                 | Competing Rust wrapper combining policy launch, process ownership, PTYs, and resource authority. Published docs describe scoped Windows ACL leases, suspended job-owned creation, and bounded output. Confined PTYs remain macOS-only; Linux deny-read is not enforced, while Windows rejects that requirement.                                 | Registry publication: November 1, 2026; several October releases show ongoing change. Broader dependency/API surface and more lifecycle management. Review its pinned backend implementation and tests; documented improvements are not independently verified guarantees here. [Published README](https://docs.rs/crate/rightkit-sandbox/0.6.2/source/README.md), [release metadata](https://crates.io/api/v1/crates/rightkit-sandbox)                                                                    |
| **Direct `landlock` + `seccompiler`; direct Seatbelt/App Sandbox; `windows` Win32 bindings** | Lower-level mechanisms/bindings, rather than a cross-platform policy wrapper. Can expose native capabilities omitted by Skarn: strict compatibility, newer Landlock scopes, transactional Windows setup, explicit jobs/handles/identities.                                                                                                      | More implementation and native expertise, but control over failure semantics. Native documentation and binding releases give a clearer capability contract; they do not supply a complete launcher. Apple still carries the legacy raw-profile compatibility risk. [Landlock](https://docs.rs/landlock/0.4.7/landlock/), [seccompiler](https://docs.rs/seccompiler/0.5.0/seccompiler/), [Win32 AppContainer launch](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer) |
| **Bubblewrap (`bwrap`)**                                                                     | Complementary Linux external launcher for mount, PID, IPC, user and network namespaces, with read-only bind mounts and optional seccomp. A new network namespace can close gaps left by TCP-only Landlock.                                                                                                                                      | External executable and user-namespace availability/configuration required. Its maintained primary README describes both the mechanism and security pitfalls; its arguments define the boundary. No native macOS/Windows backend. [Upstream](https://github.com/containers/bubblewrap)                                                                                                                                                                                                                     |
| **VMs / Windows Sandbox; Wasmtime for portable plugins**                                     | A VM supplies a separate kernel; Windows Sandbox runs native Windows software in a disposable VM. Wasmtime is a different workload model, using Wasm rather than arbitrary host executables.                                                                                                                                                    | Greater deployment/toolchain cost, with explicit host integrations still needing policy. Windows Sandbox enables networking by default. Wasmtime requires compatible modules and carefully designed imports. These are stronger/different boundaries, not drop-in Skarn policy implementations. [Windows Sandbox](https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/), [Wasmtime security](https://docs.wasmtime.dev/security.html)            |

**Recommendation:** use 1.0.1 only when its narrow, backend-specific controls match a documented requirement and you can validate them. For example, a controlled macOS worker with explicit reads and workspace writes, or a Linux worker requiring filesystem rights plus TCP denial, can be worth prototyping. Add resource supervision, descriptor hygiene, and broker validation in either case. For a reusable cross-platform launcher, direct per-OS adapters or a separately audited higher-level wrapper are a better starting point than promising parity from this report enum. Repair or replace the Windows backend before relying on strict per-launch isolation, and choose a VM when hostile workloads require a separate-kernel boundary.

## Authoritative resources and adoption questions

Useful primary references:

- [Registry identity, complete release list, dates, yanks, features and MSRV](https://crates.io/api/v1/crates/skarn-sandbox), [registry owners](https://crates.io/api/v1/crates/skarn-sandbox/owners).
- [Version 1.0.1 API](https://docs.rs/skarn-sandbox/1.0.1/skarn_sandbox/), [published source archive](https://docs.rs/crate/skarn-sandbox/1.0.1/source/), [pinned repository tree](https://github.com/Rani367/Skarn/tree/82d7caa37f280eef735f92b4154f991de5cec919).
- [Release security policy](https://github.com/Rani367/Skarn/blob/82d7caa37f280eef735f92b4154f991de5cec919/SECURITY.md), [releases](https://github.com/Rani367/Skarn/releases), [open network-related patch](https://github.com/Rani367/Skarn/pull/7).
- [Linux Landlock userspace contract](https://docs.kernel.org/userspace-api/landlock.html), [Landlock security model](https://docs.kernel.org/security/landlock.html), [seccomp contract](https://docs.kernel.org/userspace-api/seccomp_filter.html).
- [Apple App Sandbox](https://developer.apple.com/documentation/security/app-sandbox); for the deprecated raw-profile interface, inspect Apple's installed SDK `sandbox.h` and `sandbox_init(3)` as described above. Current App Sandbox documentation is not a specification of Skarn's generated SBPL.
- [Microsoft AppContainer isolation](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation), [AppContainer launch](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer), [CreateProcessW](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw), [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

Questions that materially affect adoption remain:

1. Will upstream provide typed per-control enforcement evidence, reject unsupported policy, and make `fail_closed` reject missing required controls, including partial enforcement and seccomp failure?
2. Will Windows receive per-run identities, reversible grants or private staged objects, explicit handle/environment control, and atomic job attachment with cleanup on every failure?
3. Which macOS releases and distribution configurations are supported for raw SBPL, and can the actual toolchain run under narrow read grants without falling back to whole-filesystem reads?
4. What independent security review and native Windows adversarial runtime coverage will accompany those guarantees? The research found no published audit establishing them.
5. Does the intended product require complete offline operation, destination identity restrictions, no-execute inputs, metadata immutability, or hostile multitenancy? If yes, its acceptance criteria exceed this release's common policy model and require additional mechanisms or a different design.
