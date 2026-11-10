---
prompt: |-
    Research the Rust `sandboxrs-windows` crate to help a developer decide whether
    and how to use it in a native Windows sandbox launcher.

    The starting point is a no-admin Windows process sandbox with a
    Command-like API. Verify the exact published package, implemented
    backends, backend selection, and dependency on any wider sandboxrs
    project rather than inferring guarantees from its name.

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
    - https://docs.rs/sandboxrs-windows/
    - ../os-support/app-container.md

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
       - Explain sandbox building, validated read-only/read-write authority,
         command configuration, stream defaults, child handles, and output.
       - Identify the native mechanisms used by each released backend, such as
         tokens, AppContainer, ACLs, job objects, or alternate-user isolation;
         do not assume all of these participate.
       - Describe default access, network policy, process/IPC controls, resource
         limits, descendant authority, and the supervisor/workload boundary.
       - List Cargo features, defaults, native dependencies, companion packages,
         privilege requirements, and external tools or helpers.

    3. Platform integration, compatibility, and failure behavior
       - Verify minimum native Windows versions and editions, architectures,
         supported backends, and no-admin claims using source and native docs.
       - Trace backend probes and automatic selection. Determine whether a
         failed probe or launch changes backend or weakens requested controls,
         and whether the caller can pin a backend and inspect effective policy.
       - Examine token/profile creation, filesystem grant preparation, suspended
         creation, job assignment, handle inheritance, and resume ordering.
       - Determine whether ACLs or identities persist, how concurrent sandboxes
         interact, and how partial setup and cleanup errors are reported.
       - Explain executable lookup, working directory, environment construction,
         command-line quoting, pipes/PTY support, cancellation, and output limits.
       - Verify capability reporting and refusal paths for unsupported requests;
         distinguish build success from usable native Windows enforcement.
       - Keep WSL2/Linux behavior separate from native Windows support.

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
       - Examine inherited token rights and handles, broad ACL grants, shared
         identities, reparse points, hard links, and filesystem changes between
         validation and use.
       - Investigate network loopback exemptions, local IPC, process breakaway,
         nested jobs, descendant cleanup, and supervisor termination.
       - Identify host security changes that tests or failed launches could leave
         behind, and explain how to avoid or reliably restore them.

    5. Rust examples and verification
       - Demonstrate a native Windows child with read-only inputs, a writable
         workspace, explicit backend requirements, and bounded lifetime.
       - Show backend probing, unavailable-control refusal, allowed/denied file
         operations, and network restrictions only where actually supported.
       - Include child-tree cleanup and error handling for setup, creation, job
         assignment, execution, and teardown as applicable to the real API.
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
       Compare rappct, higher-level Windows sandbox wrappers, win32job as a
       complement for lifecycle/resource management, and direct windows bindings.
       Explain when a separate-kernel option such as Windows Sandbox is needed.
       Distinguish competing bindings, higher-level wrappers, complementary
       mechanisms, and stronger boundaries. Compare platform coverage,
       maintenance, integration cost, and when another layer is needed.

    Presentation:
    - Begin with a concise suitability assessment and the main caveats.
    - Use well-formed CommonMark and capability/comparison tables where helpful.
      Use Mermaid only when a diagram clarifies policy or launch flow.
    - End with authoritative resource links and unresolved questions that
      materially affect adoption.
os: [Windows]
hash: e50b9486ae4fb237-6ed9a95b58fd0708
last_updated: 2026-11-04
---

# sandboxrs-windows: native Windows launcher assessment

**Research date: November 4, 2026. Examined release: `sandboxrs-windows` 0.1.1, published August 11, 2026.**

`sandboxrs-windows` is a real, independently published MIT-licensed Windows launcher with a small, synchronous, Command-like Rust API. It is worth prototyping for a controlled worker in a disposable Windows environment. **I would not adopt unmodified 0.1.1 as the enforcement layer for a strict malicious-code launcher.** Source inspection finds backend-selection, modern-policy serialization, overlapping filesystem-policy, persistence, timeout, and error-reporting gaps that matter to that decision. These are source-derived findings, not reproduced escapes. The experimental backend is an in-process Windows security API, not the separate-kernel Windows Sandbox VM. [Published package](https://docs.rs/crate/sandboxrs-windows/0.1.1), [published source archive](https://static.crates.io/crates/sandboxrs-windows/sandboxrs-windows-0.1.1.crate).

The useful starting policy is separate read-only inputs plus a private writable workspace, under a standard-user supervisor. The crate supplies AppContainer launch plumbing and Job Objects. It does **not** supply destination network allowlists, LPAC, a filesystem overlay, explicit ACL rollback, or an effective-policy attestation API. A successful probe or child exit is insufficient evidence of confinement. See the source findings below before running even `Sandbox::probe()` on a development machine.

## Identity, releases, and maintenance

The package name is `sandboxrs-windows`; the Rust import is `sandboxrs_windows`. Its manifest links directly to **TarunKurella/sandboxrs-windows**. The registry owner endpoint identifies one owner, `TarunKurella` (display name Tarun), matching the repository owner. There is no published workspace dependency on a parent `sandboxrs` crate. The repository's deferred roadmap describes a possible future cross-platform facade; that is a proposal, not a dependency or current platform guarantee. [Manifest](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/Cargo.toml.orig), [registry owners](https://crates.io/api/v1/crates/sandboxrs-windows/owners), [repository](https://github.com/TarunKurella/sandboxrs-windows), [pinned roadmap](https://github.com/TarunKurella/sandboxrs-windows/blob/81e14e5567fab1b88236e0b89c6741e8aa5f8f58/FUTUREPLAN.md).

The sparse registry index lists exactly these releases in the preceding 24 months, November 4, 2024–November 4, 2026; no earlier release, prerelease, or yanked version appears. Both are usable Cargo dependencies. “Stable” here means a version without a prerelease suffix, not a security certification. [Registry index, including publication timestamps and yank status](https://index.crates.io/sa/nd/sandboxrs-windows).

| Release | Publication, UTC    | API and behavior milestones                                                                                                           |
| ------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| 0.1.0   | 2026-08-08 15:14:02 | Initial reusable sandbox, filesystem plan, experimental API and AppContainer backends, jobs, CLI, and evaluation suite.               |
| 0.1.1   | 2026-08-11 05:17:05 | AppContainer named-pipe stdio fixes, parent endpoint closure for EOF, `output()` defaults, unique identities even when labels repeat. |

The release changelog describes a patch update, with no migration guide or removed public API. Existing callers must nevertheless account for changed output defaults and labels no longer intentionally sharing profiles. Keep explicit stream settings and do not use a label as persistent identity. The manifest declares Rust **1.90**, edition 2021; the native `rappct` dependency also declares 1.90. There is no 1.x release. Pre-1.0 alone says little about reliability; the implementation findings below are the stronger adoption evidence. [Release changelog](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/CHANGELOG.md), [builder](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/builder.rs).

Observable maintenance: GitHub reports repository creation on August 8, 2026, last push August 11, and an unarchived repository. The latest recorded commit is `81e14e5567fab1b88236e0b89c6741e8aa5f8f58`, August 11, fixing pipe direction. The repository has a `v0.1.1` tag but its releases API returned no GitHub Release objects; the published changelog is the release narrative. The all-state issues API returned an empty list. **Assessment:** two releases three days apart followed by roughly twelve weeks without a recorded push is a short maintenance history, not proof of abandonment or sustained support. [Repository metadata](https://api.github.com/repos/TarunKurella/sandboxrs-windows), [history](https://api.github.com/repos/TarunKurella/sandboxrs-windows/commits?per_page=5), [tags](https://api.github.com/repos/TarunKurella/sandboxrs-windows/tags), [releases](https://api.github.com/repos/TarunKurella/sandboxrs-windows/releases), [issues](https://api.github.com/repos/TarunKurella/sandboxrs-windows/issues?state=all).

### Dependencies and deployment

There are **no Cargo feature flags**, including no default feature set to select a backend. Common dependencies are `flatbuffers ^24`, `serde ^1` with derive, and `serde_json ^1`; Windows adds `rappct ^0.13.3` and `windows-sys ^0.59`. The published manifest also declares the `sandboxrs` CLI and `sandboxrs-test-attacker` helper binaries. Those are not required companion services, and depending on the library does not install a helper executable. Use an application lockfile: pinning the top-level crate alone does not freeze transitive versions. [Published manifest](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/Cargo.toml.orig).

Native dependencies are Windows APIs, including Userenv profile functions, security/ACL APIs, process attributes, pipes, and Job Objects. The modern backend dynamically loads `processmodel.dll` from System32; it does not require a new SDK header at compile time. No daemon, driver, Docker, WSL, Hyper-V feature, alternate account, or machine-wide firewall installation participates in these launch paths. Standard-user launch is the intended model, but changing an object's DACL requires appropriate access: “no admin” does not mean the caller can grant access to arbitrary protected files. Never elevate the launcher simply to make recursive grants succeed. [AppContainer implementation](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/appcontainer/), [dynamic FFI](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/modern/ffi.rs), [rappct profile implementation](https://docs.rs/crate/rappct/0.13.3/source/src/profile.rs).

## Public API and actual authority

`Sandbox::builder(workspace)` makes the workspace read-write. Builder methods add `read_only`, `read_write`, `timeout`, `max_memory`, `max_processes`, `preferred_backend`, and a diagnostic `identity`. `build()` compiles paths, selects a backend, validates it, and prepares AppContainer state if selected. One immutable `Sandbox` can launch multiple commands sharing that policy and identity. It is a security grouping, not a fresh identity for every command. [Builder](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/builder.rs), [Sandbox](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/sandbox.rs).

Path validation rejects empty, relative, NUL-containing, and parent-component roots, trims trailing separators, merges equivalent roots, rejects conflicting duplicate access, and sorts more-specific paths first. **It does not canonicalize roots, bind file IDs, resolve aliases, or prevent a path changing after validation.** The private plan's `Hidden` default means “no grant in this plan,” not a universal denial of everything outside it. [Filesystem compiler](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/filesystem.rs).

| Requirement                   | Released mechanism                                                                                      | Important limit                                                                                                             |
| ----------------------------- | ------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Read-only inputs              | AppContainer package-SID read/execute ACL grants; modern backend serializes read-only roots             | Additive grants do not revoke preexisting access. Keep RO and RW trees disjoint; nested RO exceptions are unsafe to assume. |
| Writable workspace            | Package-SID `FILE_ALL_ACCESS`; modern backend serializes RW roots                                       | Real host writes, not a copy-on-write view. Full access includes security/delete rights, not merely writing contents.       |
| No ordinary worker networking | AppContainer launch has zero capability SIDs; modern spec enables AppContainer and adds no capabilities | No explicit network policy, endpoint allowlist, or effective-firewall inspection. Validate network behavior separately.     |
| Process-tree lifecycle        | Unnamed Job Object, kill on last job-handle close; no breakaway flags granted                           | Depends on successful assignment and Windows job rules; cleanup errors are suppressed in some paths.                        |
| Memory/process limits         | Active process count and `JOB_OBJECT_LIMIT_PROCESS_MEMORY`                                              | Memory is **per process**, contrary to the builder's process-tree wording; no aggregate job-memory limit.                   |
| Stronger token/IPC/UI policy  | Regular AppContainer OS restrictions                                                                    | No LPAC, token privilege configuration, desktop policy, Win32k lockdown, or general IPC allowlist.                          |
| CPU, disk, output quotas      | No corresponding public control                                                                         | Another layer is necessary.                                                                                                 |

Sources: [ACL compiler](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/appcontainer/compile.rs), [AppContainer spawn](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/appcontainer/spawn.rs), [modern spec](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/modern/spec.rs), [job implementation](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/job.rs).

An AppContainer token carries the user's identity plus a package SID and optional capabilities. Access checks consider the user and AppContainer portions; granting the package SID cannot create rights the user lacks. Regular AppContainer also has access to resources made available to ordinary application packages. It is not LPAC, which removes that broad baseline. The crate neither creates a separate user nor explicitly calls `CreateRestrictedToken`; the OS creates the AppContainer security context from `SECURITY_CAPABILITIES`. The unrestricted supervisor remains trusted and retains its original authority. [Microsoft launch model](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer), [Microsoft isolation overview](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation).

### Commands, child handles, and output

`SandboxCommand` accepts `arg`, `args`, `env`, `env_remove`, `env_clear`, `current_dir`, `stdin`, `stdout`, and `stderr`. It exposes synchronous `spawn()` and `output()`. Use this crate's `Stdio::{inherit,null,piped}`, not `std::process::Stdio`. `spawn()` defaults to inherited streams; `output()` defaults to null stdin and piped stdout/stderr; explicit choices override defaults. Internally, spawning takes the stream options out of the command, so reusing a command does not preserve previously set stream modes. [Command source](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/command.rs), [public exports](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/lib.rs).

On Windows, `SandboxChild` exposes optional stdin/stdout/stderr wrappers implementing `Write` or `Read`, plus `id`, `backend`, `try_wait`, `wait`, `kill`, and `wait_with_output`. `SandboxOutput` contains root exit status, stdout/stderr byte vectors, backend, and duration. There is no public raw process/job/token handle, policy snapshot, async API, PTY/ConPTY, or output-size limit. `wait_with_output()` drains both pipes on threads but accumulates everything in memory, ignores read errors, and substitutes empty data after reader panic. It can wait for EOF from descendants after the root exits. A root exit therefore does not imply output collection or the entire tree has completed. [Child](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/child.rs), [output](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/output.rs).

For hostile output, explicitly use null streams or implement bounded draining and cancellation outside `output()`. Drain concurrently to avoid pipe backpressure. Close piped stdin when finished: `wait_with_output(&mut self)` does not itself take and close stdin. Do not forward raw child terminal control sequences to a privileged UI.

## Compatibility, probes, and launch ordering

### Supported Windows environments

The project advertises Windows 11 and Windows Server 2025. Its repository evaluation records describe AppContainer runs on Server 2025 build 26100 and Windows 11 ARM64 build 26200, under a standard user. These are upstream reports, not runs reproduced for this research; the modern API was reported present but disabled. The reported developer-tool compatibility was limited, including initialization failures for some `cmd.exe` and Python invocations. Launch failure is not successful isolation. [Pinned evaluation documentation](https://github.com/TarunKurella/sandboxrs-windows/blob/81e14e5567fab1b88236e0b89c6741e8aa5f8f58/evals/README.md), [pinned results directory](https://github.com/TarunKurella/sandboxrs-windows/tree/81e14e5567fab1b88236e0b89c6741e8aa5f8f58/evals/results).

AppContainer profile APIs themselves date to Windows 8 / Server 2012. That native minimum does not establish that this crate, its dependencies, and its probing strategy work there. Microsoft lists Windows 11 experimental support for the modern API, without a reliable generally enabled build/edition matrix. There is no crate edition gate, nor a Windows Home exclusion derived from the VM product. Treat Windows 10, older Server releases, x86, and other untested target combinations as unverified rather than supported by inference. On 32-bit Windows, requesting `max_memory` is explicitly refused during job creation. [CreateAppContainerProfile requirements](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-createappcontainerprofile), [modern native API requirements](https://learn.microsoft.com/en-us/windows/win32/secauthz/createprocessinsandbox), [job source](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/job.rs).

Non-Windows builds expose much of the API but `build()`/spawn cannot provide enforcement: `UnsupportedPlatform` is returned, and probes report unavailable. Native Linux in WSL2 follows that path. A Windows `.exe` launched through WSL interoperability is a native Windows workload whose actual Windows setup must still be verified. [Backend dispatch](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/mod.rs).

### Selection is not a reliable pin in 0.1.1

`Sandbox::probe()` returns entries with `backend`, `export_present`, `usable`, and human-readable `detail`; `available_backends()` filters usable entries. Results are cached per backend in `OnceLock`, despite public wording suggesting live probing. Changes to environment, helper deployment, or OS configuration after the first call are not reprobed in that process. This is not a capability matrix for network, LPAC, or individual filesystem semantics. [Sandbox diagnostics](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/sandbox.rs), [modern probe](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/modern/mod.rs), [AppContainer probe cache](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/appcontainer/mod.rs).

Automatic selection resolves the experimental create export from System32, queries `Experimental_QuerySandboxSupport` if present, otherwise makes an invalid-argument feature check, then runs a minimal launch/outside-write probe. AppContainer is also probed; the first usable entry wins. A failed modern probe can select AppContainer under Auto. Once selected, a build-policy or command-launch failure is returned, with no retry on another backend and no unrestricted `std::process::Command` fallback. [Selection code](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/mod.rs), [modern probe implementation](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/modern/probe.rs).

**Source finding:** `preferred_backend(AppContainer)` still leaves the modern entry first, so a usable modern backend wins. `preferred_backend(WindowsSandboxApi)` returns early if modern is unavailable, but when modern is usable it still probes AppContainer. If AppContainer is unavailable while modern is usable, the AppContainer preference can still select modern. Check `sandbox.backend()` immediately after build and refuse mismatches **before any workload launch**. This protects workload selection; it does not prevent probes or setup on an unwanted backend. True side-effect-free pinning requires an upstream fix or a reviewed fork. [Exact control flow](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/mod.rs).

### Probing has real effects

AppContainer probing chooses `SANDBOXRS_ATTACKER` if set, otherwise copies and executes `current_exe()` in a temporary workspace. With the helper variable it runs `sleep 1`, then `write <outside-file>`. Without it, it sets **the parent process environment** variable `SANDBOXRS_PROBE_SELF=1`, launches the copied host with no arguments, and only checks its exit status: the outside-write attempt is skipped. A general embedding application has no obligation to understand that protocol; it may recurse, hang, or initialize services. There is no probe timeout. The diagnostic claiming outside-write denial overstates the no-helper branch. [Published AppContainer probe](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/appcontainer/probe.rs).

Supply a trusted, self-contained helper implementing that exact protocol before process startup. Do not take its path from untrusted job configuration. Probe in a dedicated supervised process, before starting host threads, because the no-helper branch mutates global environment and probes can wait indefinitely. Even a backend preference does not avoid all other probes. Both the probe and normal AppContainer setup invoke the ACL preparation described below; neither is a harmless availability query.

### Native launch sequence

For AppContainer, build creates/derives a profile through `rappct`, then prepares filesystem grants. Each launch constructs a package SID with **zero capabilities**, a `STARTUPINFOEXW` containing both security capabilities and an explicit standard-handle list, and calls `CreateProcessW` with `CREATE_SUSPENDED`, `CREATE_NO_WINDOW`, and inheritance enabled for that list. It then creates/configures a Job Object, assigns the still-suspended child, and resumes its primary thread. Job creation, assignment, or resume failure attempts termination/reaping and returns an error. No workload instruction is intentionally allowed before job assignment. `rappct`'s process launcher is deliberately bypassed: the source records an attribute/handle-list defect causing error 87. That comment is the wrapper author's defect report, not an independently reproduced diagnosis here. [AppContainer spawn source](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/appcontainer/spawn.rs).

The modern backend builds a private FlatBuffer specification with schema version `0.1.0`, AppContainer enabled, and RO/RW paths. It does not set network, LPAC, UI, Win32k, explicit deny, or least-privilege options. It calls `Experimental_CreateProcessInSandbox`, creates suspended, assigns its own lifecycle job, then resumes. **On error 50 with a custom environment it retries without that environment**, potentially restoring parent secrets despite `env_clear()`/removals. This is a real semantic downgrade within one backend, although it does not switch backend. Strict environment requirements should refuse this backend in 0.1.1. [Modern specification](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/modern/spec.rs), [modern launch](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/modern/spawn.rs).

**Host-reproduced modern serialization defect:** the released `spec.rs` passes `0`, `1`, `7`, and `8` as FlatBuffers slot offsets. FlatBuffers expects vtable **byte offsets**, not field ordinals: the corresponding offsets are `4`, `6`, `18`, and `20`. The misplaced writes overlap the vtable header and do not encode the intended schema fields. A macOS-only harness imported the unmodified published `filesystem.rs` and `spec.rs`, with `flatbuffers = "=24.12.23"`, and serialized one writable `/workspace` root. It produced a 72-byte buffer, root at byte 20, vtable at byte 11, and vtable bytes `[12, 11, 0, 0, 0, 0, 0, 4, 0]`; field 0 at vtable offset 4 was zero and the encoded vtable length exceeded the buffer. This executes serialization only, with no profile, ACL, or process launch. It confirms malformed encoding in that case, not the exact error an enabled Windows engine would return. The release's identifier-only test cannot establish schema validity. Treat the modern backend as blocked pending a serializer fix and real schema-verifier/native tests. [Published serializer and tests](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/modern/spec.rs), [FlatBuffers 24.12.23 builder](https://docs.rs/crate/flatbuffers/24.12.23/source/src/builder.rs), [field-index conversion](https://docs.rs/crate/flatbuffers/24.12.23/source/src/vtable.rs).

There are additional source-level modern-stdio concerns: the pipe endpoints selected as child stdin/stdout have the opposite direction to the corrected AppContainer implementation, and the native call sets `inheritHandles=FALSE` while preparing inheritable handles. Microsoft explicitly disallows inheritance for this API. Do not assume modern piping works because AppContainer piping was repaired. Validate modern launch, policy serialization, stdio, and environment together on an actually enabled host. [Modern stdio source](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/modern/spawn.rs), [native inheritance contract](https://learn.microsoft.com/en-us/windows/win32/secauthz/createprocessinsandbox).

### Executables, directories, and environment

AppContainer resolves programs using the **supervisor's PATH**, checking bare names and `.exe`, `.com`, `.bat`, `.cmd`; it does not use the command's modified PATH or PATHEXT. Finding a batch file does not make `CreateProcessW` a shell interpreter. Modern launch passes a null application name and lets Windows parse/resolve the command line. Prefer an absolute trusted executable and explicit arguments; invoking `cmd /c` adds shell-language parsing that the crate's CRT-style backslash/quote escaping does not secure. Strings use lossy Unicode conversion in several paths and lack comprehensive embedded-NUL validation. [AppContainer command construction](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/appcontainer/spawn.rs), [modern construction](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/modern/spawn.rs).

The workspace grant is not the working directory: unset `current_dir` reaches Windows as null and inherits the supervisor's directory. Set it explicitly to the workspace. Environment defaults inherit the supervisor's variables, potentially including credentials. AppContainer `env_clear()` preserves/reconstructs hidden drive-current-directory entries and has a conditional fallback list of essential parent variables; it is not literally an empty environment. Its hidden-entry parser splits at the initial equals sign, an additional reason to inspect actual child variables rather than trusting the comment. Use a launcher process started with a deliberately minimal environment, plus explicit `SystemRoot`, workspace TEMP/TMP, and workload variables. Never log launch errors indiscriminately: AppContainer creation failures include the first six environment entries in the error message. [Environment and failure diagnostics](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/appcontainer/spawn.rs).

## Security-relevant gaps and their consequences

### Filesystem grants are changes to host security

AppContainer setup walks each root and adds a package-SID allow ACE to its existing DACL using `rappct::acl::grant_to_package`. Directory ACEs inherit to subdirectories/files. RO uses generic read plus execute; RW uses full access. It then attempts recursive read/execute grants to `SystemRoot`, `ProgramFiles`, `ProgramFiles(x86)`, and common-program-files roots taken from the **parent environment**. These are broad extra roots beyond the caller's explicit input list, and recursion can be expensive. Baseline errors are ignored. Errors below a root are also ignored by `grant_recursive`, even for explicit user policy, contrary to the comment saying explicit rules fail closed. A build can succeed while some intended readable inputs remain inaccessible. [Grant traversal](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/appcontainer/compile.rs), [rappct DACL implementation](https://docs.rs/crate/rappct/0.13.3/source/src/acl.rs).

The private plan advertises “more specific wins,” but the AppContainer compiler adds grants, not deny ACEs or revocations. A read-only child below a writable parent can receive the parent's full-access grant during later recursive traversal; sorting the rules does not solve additive authority. Parent delete-child rights also matter to replacement/deletion semantics. Use disjoint roots, test deletion and rename as well as writes, and reject policies needing negative exceptions. AppContainer validation currently does not refuse those overlaps. This is a **missing crate enforcement/refusal**, not evidence that Windows cannot represent stronger policy. [Filesystem plan](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/filesystem.rs), [grant compiler](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/appcontainer/compile.rs).

Dropping AppContainer state best-effort deletes its profile; neither normal drop nor partial-setup failure restores DACLs. A later root failure leaves earlier grants behind. Profile deletion does not remove ACEs written on unrelated filesystem objects by this code. Unique 0.1.1 identities prevent accidental label-based sharing but leave SID-specific entries and potentially failed profile deletions. `Sandbox` exposes neither its generated identity/SID nor fallible teardown. Commands sharing one `Sandbox` share grants and IPC identity; they should belong to one trust domain. [State cleanup](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/appcontainer/mod.rs), [identity generation](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/builder.rs), [rappct profile deletion](https://docs.rs/crate/rappct/0.13.3/source/src/profile.rs).

Concurrent grants use a read/merge/write DACL sequence without a shared transaction or lock. Concurrent sandbox builds or another ACL editor can lose updates. Blindly restoring old DACL snapshots can similarly erase legitimate concurrent changes. The reliable test cleanup for this release is **discarding a disposable VM/snapshot**, with no writable host shares. A production design needs audited SID-specific grant tracking, serialization, rollback on partial failure, crash recovery, and observable teardown. Merely deleting a temporary workspace cannot restore ACEs on baseline roots. Do not run `doctor`, probe, or failure tests on a valuable host assuming they are read-only.

### Aliases, reparse points, and changing files

The crate validates path spelling rather than object identity. Traversal uses `symlink_metadata` but then calls path-based ACL APIs; it does not open roots with a no-follow, handle-based grant protocol or reject every Windows reparse tag. Hard links share file security, so a link inside a granted tree can grant access to the same object through another name. Reparse points or mutable ancestors can change where path-based setup operates. A hostile workload can also modify its writable tree while subsequent commands reuse the sandbox. These are **Windows object/namespace properties compounded by missing crate validation and race defenses**. [Normalization](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/filesystem.rs), [path-based recursion](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/appcontainer/compile.rs), [Microsoft hard-link behavior](https://learn.microsoft.com/en-us/windows/win32/fileio/hard-links-and-junctions).

Recommendation: stage independently copied inputs and runtime binaries in a trusted, private layout; exclude reparse points and external hard links; keep grant targets and ancestors under trusted control during setup; make input trees disjoint from every writable ancestor. Canonicalization alone cannot close a validation/use race. For an exact accessible-object contract, use a reviewed handle-based provisioning layer or a VM rather than relying on this crate's private path plan.

### Networking, local IPC, tokens, and handles

Zero capabilities is a useful AppContainer starting point for blocking ordinary Internet/private-network sockets. It is not an explicit crate-level “deny all communication” contract. There is no `deny_network`, capability setter, loopback-exemption audit, destination allowlist, or broker API. Local pipes, COM/RPC, registry resources, shared memory, and same-identity communication depend on OS access rules. AppContainer IPC restrictions are not equivalent to disabling all IPC. Existing privileged handles can carry already-authorized operations, so file ACL checks on new opens are insufficient to assess them. [Capability count in launch](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/appcontainer/spawn.rs), [Microsoft IPC guidance](https://learn.microsoft.com/en-us/windows/apps/develop/communication/interprocess-communication).

AppContainer's explicit handle list is a valuable protection against arbitrary inheritable secret handles and existing sockets. However, **inherited standard handles are intentional authority**, not harmless labels: a secret file wired as stdin or a sensitive file wired as stdout passes that authority deliberately. The implementation marks inherited host standard handles inheritable without restoring their flags. Other concurrent process-launch code that broadly inherits handles can interact with these changes. Prefer null or private pipes, serialize sensitive launches, inspect token state under the deployed parent context, and use a standard-user broker with no impersonation or ambient secrets. No public token audit confirms privileges, integrity, or effective package capabilities after launch. [Handle setup and security attributes](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/backend/appcontainer/spawn.rs).

Do not add loopback exemptions to make a test pass. They change host policy and can expose local services; the crate does not create, record, or restore them. Test host loopback, machine-address access, IPv4/IPv6, TCP/UDP, DNS-mediated effects, and enterprise firewall/VPN configurations separately. For approved destinations only, keep the worker offline and provide narrowly authorized broker requests, or add independently enforced firewall/proxy policy. HTTP proxy environment variables alone cannot constrain a malicious binary. These are **additional mechanisms**, not missing builder flags that can be emulated securely with environment settings. [Microsoft network-isolation troubleshooting](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/troubleshooting-uwp-firewall), [network capability definitions](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/app-capability-declarations).

### Jobs, timeouts, and termination

The job enables kill-on-close and optionally active-process count and **per-process committed-memory** limits. It enables neither breakaway nor silent breakaway. Normally descendants stay in the job; nested-job compatibility and existing outer job limits still apply. A creation/assignment failure is an error, not permission to resume outside the job. The API does not disable descendant creation; `max_processes(1)` can prevent it at the cost of workload compatibility. Jobs manage lifecycle/resources, not filesystem or network permissions. [Job code](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/job.rs), [Microsoft job rules](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects), [nested jobs](https://learn.microsoft.com/en-us/windows/win32/procthread/nested-jobs).

`timeout()` is implemented by a polling deadline created **when `wait()` starts**. It is not a watchdog from spawn, and `try_wait()` does not enforce it. On timeout, `wait()` kills the tree and returns root status, not `SandboxError::Timeout` (the enum exists but this path does not emit it). Worse, `terminate_tree()` discards `job.terminate()` errors and returns success, then `wait_until_exit()` can wait indefinitely. `kill()` therefore cannot establish successful tree termination solely from `Ok(())`. The 0.1.1 child code, rather than the timeout/error documentation, determines these semantics. [Child implementation](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/child.rs), [error enum](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/src/error.rs).

Child drop attempts termination and closes the job through ownership. Supervisor termination should close its noninherited job handle, causing kill-on-close if no other job-handle reference exists. It cannot run profile/ACL cleanup after a crash. Normal root exit does not itself close the child's job; drop it promptly to kill remaining descendants. A strict launcher needs an external watchdog and independent observation of tree disappearance, plus a fallible explicit teardown design. This is a **crate lifecycle/error-reporting gap** layered over genuine OS job containment.

### Defects, advisories, and assurance

The published changelog identifies fixed 0.1.0 defects in piped stdio, endpoint ownership, output defaults, and reused identity labels; they are reasons to choose 0.1.1 over 0.1.0. The source-level issues described here remain present in the examined 0.1.1 files and latest observed repository state; they are not claimed to have upstream advisory IDs or confirmed exploitability on every host. The repository issue query and targeted searches for `sandboxrs-windows` advisories/vulnerabilities found no published advisory for this package. That is a limited search result, not a full dependency audit or proof of safety. [Changelog](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/CHANGELOG.md), [issue tracker](https://github.com/TarunKurella/sandboxrs-windows/issues), [RustSec advisory database](https://rustsec.org/advisories/).

Upstream evaluations include useful positive-control and adversarial fixture designs. They do not establish every backend on every Windows build: required behavior must actually execute, and a launch failure, native job restriction, or unavailable modern API must remain an error/unsupported result. LPAC, restricted-token fallback, network configuration, and a formal capability-requirements API appear in the deferred roadmap, not the released public surface. [Pinned future plans](https://github.com/TarunKurella/sandboxrs-windows/blob/81e14e5567fab1b88236e0b89c6741e8aa5f8f58/FUTUREPLAN.md).

## Focused Rust integration example

This example demonstrates the **real 0.1.1 API** with disjoint inputs/workspace, an AppContainer backend postcondition, explicit null streams, allowed/denied file operations, and a supervisor polling deadline. It refuses requirements it cannot implement before any probe. It is an experiment for a disposable Windows VM, **not a complete strict sandbox launcher**: there is no reliable ACL rollback, bounded setup/probe time, or fallible tree teardown in the public API. An outer watchdog must bound the whole experiment and discard the VM afterward.

`Cargo.toml`:

```toml
[package]
name = "sandboxrs-review-example"
version = "0.0.0"
edition = "2021"
rust-version = "1.90"

[dependencies]
sandboxrs-windows = "=0.1.1"
```

`src/main.rs` (one executable; worker/probe modes run before supervisor setup):

```rust
use std::{env, error::Error, fs, io, path::Path, process, thread};
use std::time::{Duration, Instant};
use sandboxrs_windows::{BackendKind, BackendPreference, Sandbox, Stdio};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn require_denial<T>(result: io::Result<T>) -> Result<()> {
    match result {
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => Ok(()),
        Err(e) => Err(format!("test did not establish access denial: {e}").into()),
        Ok(_) => Err("forbidden operation succeeded".into()),
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("sleep") if args.len() == 2 => {
            thread::sleep(Duration::from_secs(args[1].parse()?));
            return Ok(());
        }
        Some("write") if args.len() == 2 => {
            fs::write(&args[1], b"probe")?;
            return Ok(());
        }
        Some("worker") if args.len() == 4 => {
            let inputs = Path::new(&args[1]);
            let workspace = Path::new(&args[2]);
            let secret = Path::new(&args[3]);
            if fs::read(inputs.join("input.txt"))? != b"input" {
                return Err("unexpected input".into());
            }
            fs::write(workspace.join("result.txt"), b"result")?;
            require_denial(fs::OpenOptions::new().append(true)
                .open(inputs.join("input.txt")))?;
            require_denial(fs::read(secret.join("secret.txt")))?;
            require_denial(fs::write(secret.join("forbidden.txt"), b"bad"))?;
            return Ok(());
        }
        Some("--require-offline") | Some("--require-destination-allowlist") => {
            return Err("0.1.1 cannot attest this requirement; no sandbox launched".into());
        }
        _ => {}
    }
    if !cfg!(windows) {
        return Err("native Windows is required".into());
    }
    if args.len() != 3 {
        return Err("usage: example ABS_INPUTS ABS_WORKSPACE ABS_SECRET".into());
    }
    let inputs = Path::new(&args[0]);
    let workspace = Path::new(&args[1]);
    let secret = Path::new(&args[2]);
    if ![inputs, workspace, secret].iter().all(|p| p.is_absolute() && p.is_dir()) {
        return Err("all fixture roots must be existing absolute directories".into());
    }
    let exe = env::current_exe()?;
    let helper = env::var_os("SANDBOXRS_ATTACKER")
        .ok_or("set SANDBOXRS_ATTACKER to this trusted executable before startup")?;
    if fs::canonicalize(helper)? != fs::canonicalize(&exe)? {
        return Err("unexpected probe helper".into());
    }
    let report = Sandbox::probe();
    if !report.entries.iter().any(|e| e.backend == BackendKind::AppContainer && e.usable) {
        return Err(format!("AppContainer probe unavailable: {report:?}").into());
    }
    let sandbox = Sandbox::builder(workspace)
        .read_only(inputs)
        .read_only(exe.parent().ok_or("executable directory missing")?)
        .preferred_backend(BackendPreference::AppContainer)
        .max_processes(8)
        .max_memory(128 * 1024 * 1024)
        .timeout(Duration::from_secs(5))
        .build()?;
    if sandbox.backend() != BackendKind::AppContainer {
        return Err("required AppContainer backend was not selected".into());
    }
    let system_root = env::var_os("SystemRoot").ok_or("SystemRoot missing")?;
    let mut command = sandbox.command(&exe);
    command.arg("worker").arg(inputs).arg(workspace).arg(secret)
        .current_dir(workspace)
        .env_clear().env("SystemRoot", system_root)
        .env("TEMP", workspace).env("TMP", workspace)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = command.spawn()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let cancellation = child.kill();
            drop(child);
            return Err(format!(
                "deadline expired; kill returned {cancellation:?}; verify tree externally"
            ).into());
        }
        thread::sleep(Duration::from_millis(20));
    };
    drop(child);
    if !status.success() {
        return Err(format!("worker failed: {status}; inspect isolated fixture").into());
    }
    if fs::read(workspace.join("result.txt"))? != b"result"
        || fs::read(inputs.join("input.txt"))? != b"input"
        || secret.join("forbidden.txt").exists()
    {
        return Err("filesystem postconditions failed".into());
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        process::exit(1);
    }
}
```

Platform setup: use a fresh standard-user VM snapshot and build a native Windows executable with the appropriate MSVC toolchain/build tools. Put the executable/runtime DLLs, input directory, workspace, and secret directory in **four separate sibling trees** owned by that user. Do not nest inputs/runtime below the workspace, add junctions, or use host-mounted secrets. Create `input.txt` containing exactly `input`, `secret.txt` containing a test secret, and leave `forbidden.txt` absent. Before setup, prove the supervisor can read/write the test secret and open the input for append, then restore fixtures. This keeps failed negative controls from being mistaken for enforcement.

Set `$env:SANDBOXRS_ATTACKER` to the absolute path of the built example in the PowerShell process **before launching it**, then invoke the example with the three absolute fixture paths. The probe copies this executable and uses its `sleep`/`write` modes. The worker inherits no caller standard handles and receives no normal parent environment secrets through the command's AppContainer path. The example deliberately makes no network guarantee; `--require-offline` and `--require-destination-allowlist` are refusal demonstrations. The comparison of canonical paths is only a deployment sanity check, not a defense against a malicious actor replacing the executable or roots.

Setup errors propagate from `build()`, creation/job-assignment/resume errors from `spawn()`, and worker operation failures through its exit status. On later supervisor errors, `SandboxChild` drop attempts tree cleanup. The crate does not expose enough information to certify teardown or ACL restoration; the outer watchdog must independently verify process disappearance. Null stdio bounds captured-output memory but sacrifices worker diagnostics. A production broker should collect bounded structured diagnostics over a carefully authorized channel.

**Verification status:** the exact Cargo configuration and Rust example above passed `cargo check --target x86_64-pc-windows-gnu` on macOS 27.2 (Apple Silicon), using `rustc 1.98.1 (48a229cea 2026-09-01)`. Resolution selected `sandboxrs-windows 0.1.1`, `rappct 0.13.3`, `windows-sys 0.59.0`, and `flatbuffers 24.12.23`. This was a cross-target type check, not linking or native execution, and did not test the declared Rust 1.90 minimum. No sandbox probe or workload was executed; compilation does not validate Windows enforcement. The separate serialization harness described above executed on the macOS host with the same Rust toolchain; it called no Windows security APIs.

## Native verification plan

Run a dedicated supervisor executable under an unrestricted standard user, not in the unit-test runner itself. The supervisor owns the sandboxed worker and fixtures. A separate unrestricted watchdog owns the supervisor lifetime and records process IDs/postconditions. Keep all windows hidden and avoid desktop focus changes. Use one fresh disposable Windows environment per security test batch, because probes and partial failures can leave system-root ACEs. Do not expose writable host folders, clipboard credentials, or production network credentials to that VM.

| Test                 | Positive control                                        | Required sandbox observation                                                                                                                                                            |
| -------------------- | ------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Inputs/workspace     | Native read, append, delete, create all work            | Input reads and workspace mutations work; input append/delete/rename fail with access denial and unchanged contents.                                                                    |
| Excluded files       | Native read/write works                                 | Read/write denied; no outside artifact. An absent executable or startup crash is an error.                                                                                              |
| Policy/refusal       | Backend availability known in separate process          | Wrong backend rejected before worker launch; unavailable required control rejected before probing. Test modern enabled and disabled hosts separately.                                   |
| Alias attacks        | Native junction/hard-link setup demonstrably works      | No access or grant propagation to excluded objects; include root swaps during setup and after build. Inspect DACLs, not only file contents.                                             |
| Handles/IPC          | Native secret handle and socket transfer works          | Unlisted inheritable handles unavailable; explicit stdio grants understood; unrelated named pipes/RPC and cross-sandbox IPC tested.                                                     |
| Descendants          | Native child/grandchild creation and target writes work | Allowed child writes succeed; forbidden writes fail; kill, timeout, root exit/drop, and abrupt supervisor death leave no owned descendants.                                             |
| Jobs/resources       | Native breakaway and allocations work                   | Test breakaway, detached and suspended child modes, nested job constraints, active-process count, per-process memory; do not credit an outer host job's native refusal to this sandbox. |
| Network              | Native controlled endpoints reachable                   | Test external/private/loopback TCP and UDP, IPv4/IPv6, DNS, existing sockets and broker requests. Attribute access-denied evidence separately from timeouts/unreachable servers.        |
| Failures/persistence | Snapshot ACL/profile state before setup                 | Inject missing helper, invalid executable/cwd, protected grant, job assignment/resume/cleanup failures, concurrent builds; no weakened retry and all leftovers recorded. Discard VM.    |

Use native token introspection in the worker or a trusted external inspector to record AppContainer SID, capabilities, integrity, and privileges. Observe job membership and child-tree disappearance independently; the public backend label alone is not attestation. Treat `export_present` as one loader fact and `usable` as the result of a narrow cached probe, never proof of all requested controls.

## Alternatives and complementary mechanisms

These choices solve different portions of the launcher. None should be selected just because its name contains “sandbox.”

| Choice                    | Relationship and coverage                                                                                                                                                 | Maintenance evidence and integration cost                                                                                                                                                                     | When another layer is needed                                                                                                                                                                         |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rappct` 0.13.3           | Competing lower-level native Windows toolkit; already used for profiles/ACLs here. Exposes capabilities, LPAC options, token introspection, and optional network helpers. | Latest index entry October 23, 2025; broader policy plumbing belongs to caller. The wrapper source specifically bypasses its launch helper due to a reported defect. Reproduce/fix that path before adoption. | Job supervision, transactional grants, destination policy, and deployment tests still need design.                                                                                                   |
| `sandboxrs-windows` 0.1.1 | Higher-level native Windows command wrapper with two backends.                                                                                                            | Two August 2026 releases; smallest basic API integration, but current gaps require a reviewed fork or upstream fixes for strict requirements.                                                                 | LPAC, exact policy/refusal, ACL lifecycle, strong networking and observable teardown.                                                                                                                |
| `firehazard`              | Typed low-level Windows sandbox primitives; a comparison point for building a custom wrapper, not a drop-in Command facade.                                               | Published docs identify `0.0.0-2022-09-10` and explicitly warn about maturity/security assurance. High policy/ownership cost; old publication is observable, not a complete repository-health assessment.     | Supply your own complete launcher and review/tests; no inferred assurance from typed bindings.                                                                                                       |
| `win32job` 2.0.3          | **Complement**, Windows Job Object wrapper for process lifecycle/resource limits. No token, ACL, or network boundary.                                                     | Latest index entry May 15, 2025. Lower job-plumbing cost; this crate already has a private job, with no public handle for easy augmentation.                                                                  | Use in a custom launcher or outer supervisor; do not assume assigning an already-running worker closes the preassignment race.                                                                       |
| `windows` / `windows-sys` | Microsoft-maintained competing binding styles: typed projections versus lower-level APIs. Native Windows enforcement; no assembled sandbox policy.                        | Lowest-level control, highest review burden for unsafe ownership, ACL transactions, attributes, job ordering, and errors.                                                                                     | Appropriate when the application needs precise LPAC/handles/jobs/WFP policy that the convenience wrapper cannot expose.                                                                              |
| Windows Sandbox VM        | Stronger, separate-kernel native Windows execution environment; not this crate's experimental backend.                                                                    | Microsoft OS feature; requires supported editions and virtualization setup, guest provisioning, and external orchestration.                                                                                   | Prefer when the risk includes hostile native binaries/kernel attack surface or when grant/policy complexity exceeds a defensible process sandbox. Configure networking and host mappings explicitly. |

Sources: [rappct API](https://docs.rs/rappct/0.13.3/rappct/), [rappct release index](https://index.crates.io/ra/pp/rappct), [firehazard warnings](https://docs.rs/firehazard/0.0.0-2022-09-10/firehazard/), [win32job API](https://docs.rs/win32job/2.0.3/win32job/), [win32job release index](https://index.crates.io/wi/n3/win32job), [Microsoft Rust bindings](https://github.com/microsoft/windows-rs), [Windows Sandbox](https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/).

Windows Sandbox is unavailable on Home and runs a separate kernel; networking is enabled by default. Its feature/edition requirements do not apply to ordinary AppContainer launches. A VM still needs restrictive clipboard, network, and mapped-folder configuration: a writable host mapping grants real host write authority. [Microsoft VM overview](https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/), [configuration reference](https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/windows-sandbox-configure-using-wsb-file).

## Authoritative resources and adoption questions

Start with the [exact 0.1.1 package source](https://docs.rs/crate/sandboxrs-windows/0.1.1/source/), [versioned Rust API](https://docs.rs/sandboxrs-windows/0.1.1/sandboxrs_windows/), [downloadable publication](https://static.crates.io/crates/sandboxrs-windows/sandboxrs-windows-0.1.1.crate), [release index](https://index.crates.io/sa/nd/sandboxrs-windows), and [pinned repository](https://github.com/TarunKurella/sandboxrs-windows/tree/81e14e5567fab1b88236e0b89c6741e8aa5f8f58). For underlying contracts, use Microsoft's [AppContainer launch guide](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer), [experimental API reference](https://learn.microsoft.com/en-us/windows/win32/secauthz/createprocessinsandbox), [process creation reference](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw), and [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects). Some docs.rs pages were unavailable through the browsing service; the versioned archive was downloaded and inspected directly rather than substituting repository main for released code.

Questions that materially affect adoption remain:

- Will upstream fix modern-spec serialization, true backend pinning, overlapping-policy refusal, and implicit environment fallback, with regression tests?
- Which exact Windows builds/architectures actually enable the modern API, and do its serialized policy and stdio pass the same behavioral contract?
- How will the launcher track and restore every ACL/profile change, including probes, partial setup, concurrency, and supervisor crashes?
- Does the deployed runtime launch with the necessary RO grants under a standard user, and do hostile aliases, inherited authority, IPC, and network tests pass on the actual enterprise configuration?
- Can a reviewed integration provide independently verified deadlines and tree teardown, bounded diagnostics, and a measurable no-network or destination-only contract?

Until those are answered, use the crate as a prototype/reference for native Windows process reduction, and choose an audited custom launcher or disposable VM for requirements its released API cannot reliably establish.
