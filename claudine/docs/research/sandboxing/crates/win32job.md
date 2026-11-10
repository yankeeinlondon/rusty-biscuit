---
prompt: |-
    Research the Rust `win32job` crate to help a developer decide whether
    and how to use it in the lifecycle and resource-control layer of a native Windows sandbox launcher.

    The starting point is safe Rust access to Windows job objects. Treat
    resource/lifetime management separately from filesystem, network, and
    token-based isolation; determine exactly which job controls the published
    crate exposes and which require direct Win32 integration.

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
    - https://docs.rs/win32job/
    - https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects

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
       - Explain job creation/opening, limit-info construction and updates,
         process assignment, queries, termination, and handle ownership.
       - Identify exposed memory, CPU/time, process-count, priority, UI, and
         kill-on-close controls; explicitly identify native controls not wrapped.
       - Explain descendant membership, nested jobs, breakaway, per-process versus
         aggregate limits, privileges, and whether assignment can be undone.
       - List Cargo features, defaults, Win32 binding dependencies, target
         requirements, and ownership/Send/Sync contracts that affect integration.

    3. Platform integration, compatibility, and failure behavior
       - Verify minimum native Windows versions for exposed APIs and limits,
         supported architectures, and nested-job behavior on current Windows.
       - Explain existing job membership in CI, services, shells, and parent
         launchers, and the errors or restrictions it creates.
       - Trace creation and assignment failures, handle rights, configuration
         validation, unsupported limits, and query/update error reporting.
       - Determine whether the crate offers process creation with atomic job
         membership or whether callers need suspended creation or a process
         attribute list to avoid running unassigned untrusted code.
       - Explain kill-on-last-handle-close, inherited/duplicated handles, child
         process handles versus job handles, and supervisor-death behavior.
       - Distinguish compilation from tested native Windows behavior and keep
         WSL2/Linux process control separate from Windows job objects.
       - Show how to refuse launch when required limits or membership cannot be
         established, and identify lower-level Win32 calls needed to do so.

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
       - Examine spawn-then-assign races, child breakaway, nested-job limits,
         shared or inherited job handles, and accidentally restricting the launcher.
       - Distinguish working-set limits from committed-memory limits, CPU rates
         from time budgets, and process termination from graceful cancellation.
       - State explicitly that jobs alone do not grant a filesystem allowlist,
         no-network policy, AppContainer identity, or malicious-code boundary.

    5. Rust examples and verification
       - Demonstrate assigning a dedicated child to a configured job, setting
         an actual supported resource limit, and killing the whole child tree.
       - Use safe launch ordering and explicit handle ownership; if direct Win32
         calls are needed for atomic membership, clearly identify that boundary.
       - Include assignment refusal and supervisor/last-handle termination probes.
         Never assign the research process or shared test runner to the job.
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
       Compare direct windows bindings and other job-object wrappers, then
       explain composition with AppContainer/restricted-token launchers such as
       rappct or sandboxrs-windows. Separate resource supervision from isolation.
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
hash: 423b4020fa93f346-a71879a5f2349ad8
last_updated: 2026-11-04
---

# win32job for a native Windows sandbox launcher

**Assessment:** `win32job` **2.0.3** is a small, usable building block for Windows process-tree lifetime management and a few scheduling/working-set controls. It is insufficient by itself for a sandbox launcher: important resource caps and secure process creation need direct Win32 calls, and access isolation needs a separate mechanism. Jobs alone provide **no filesystem allowlist, no no-network policy, no AppContainer identity, and no boundary for malicious code**. The crate's own description promises safe access to job objects, not a sandbox. [Published API](https://docs.rs/win32job/2.0.3/win32job/), [Microsoft job model](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

**Research date:** November 4, 2026. **Examined release:** stable, non-yanked `win32job = "=2.0.3"`, published May 15, 2025, 10:25:16 UTC. Evidence includes the downloaded crates.io archive, registry version metadata and index, versioned rustdoc, linked upstream history, and Microsoft documentation. Neighboring reports were not used as evidence. Examples below were cross-checked and executed on one native Windows rig; that is limited integration evidence, not a security certification.

## Identity, releases, and maintenance

| Item                       | Verified finding                                                                                                                                                                                   |
| -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Package                    | `win32job`; library import `win32job`; available from crates.io                                                                                                                                    |
| Repository                 | [`ohadravid/win32job-rs`](https://github.com/ohadravid/win32job-rs), matching the published manifest's homepage and repository                                                                     |
| Author and lead maintainer | Ohad Ravid (`ohadravid`), named in the package and authoring releases; contributions include Kharosx0 and danielpeng2. This does not establish an exclusive list of current publish-right holders. |
| License                    | **MIT OR Apache-2.0**, from the published manifest and both included license files; GitHub's single detected license is not the complete licensing expression                                      |
| Parent project             | Standalone repository under the author's account; no parent framework declared in the manifest. nextest is a consumer mentioned in upstream issues, not the owner.                                 |
| Release provenance         | Archive `.cargo_vcs_info.json` records commit `29913d5f6acd47a491f73819ff6d54b63fb851b2`                                                                                                           |

Sources: [published archive](https://static.crates.io/crates/win32job/win32job-2.0.3.crate), [package metadata](https://crates.io/api/v1/crates/win32job/versions), [registry index including yank flags](https://index.crates.io/wi/n3/win32job), [release commit](https://github.com/ohadravid/win32job-rs/commit/29913d5f6acd47a491f73819ff6d54b63fb851b2).

The 24-month window is November 4, 2024–November 4, 2026. It contains exactly three published releases:

| Version | Publication date (UTC) | Relevant change                                                            |
| ------- | ---------------------- | -------------------------------------------------------------------------- |
| 2.0.1   | January 23, 2025       | Added `limit_breakaway_ok`                                                 |
| 2.0.2   | February 21, 2025      | Added `limit_silent_breakaway_ok`                                          |
| 2.0.3   | May 15, 2025           | Added `From<JobError> for io::Error`; updated `windows` dependency to 0.61 |

The registry lists no later stable or prerelease versions and no releases in 2026 as of research. Dates above are crate publication dates, which differ by minutes from GitHub release timestamps. [Registry versions](https://crates.io/api/v1/crates/win32job/versions), [2.0.1 notes](https://github.com/ohadravid/win32job-rs/releases/tag/v2.0.1), [2.0.2 notes](https://github.com/ohadravid/win32job-rs/releases/tag/v2.0.2), [2.0.3 notes](https://github.com/ohadravid/win32job-rs/releases/tag/v2.0.3).

Older milestones matter for migration:

- 1.0.0 and 1.0.1 were published February 5, 2020; 1.0.2 on August 12, 2022 added `Send + Sync` for `Job`.
- **1.0.3, January 24, 2024, is yanked.** Its switch from `winapi` to `windows` changed public handle types and broke consumers despite being a patch release.
- 1.0.4 and 2.0.0 were published January 26, 2024. Both fix PID enumeration omitting the first PID and appending a spurious zero; 1.0.4 backports the fix to the older binding line. 2.0.0 deliberately exposes handles as `isize` and hides the native extended-limit structure. Code accessing that structure or passing old binding-specific handle types must migrate. The published stable API is on the 2.x line.

[Release history](https://github.com/ohadravid/win32job-rs/releases), [breaking-change issue #3](https://github.com/ohadravid/win32job-rs/issues/3), [API decision #4](https://github.com/ohadravid/win32job-rs/pull/4), [PID fix #5](https://github.com/ohadravid/win32job-rs/pull/5).

**Maintenance judgment:** small and intermittently maintained, with a narrow public surface rather than evidence of frequent feature development. The last default-branch commit observed was June 29, 2025, a documentation correction after 2.0.3. The repository is not archived. Open requests include more limits (#6), a child-handle assignment convenience (#12), and membership checking (#13). These are **requests, not released APIs**. The June documentation change is also absent from the published archive. About 17 months without a release does not prove abandonment, but adopters should budget for maintaining their Win32 integration. [Commit history](https://github.com/ohadravid/win32job-rs/commits/main/), [issues](https://github.com/ohadravid/win32job-rs/issues).

### Build contract

The release uses Rust edition 2021 but declares **no `rust-version` or MSRV policy**. Edition 2021's Rust 1.56 baseline is not a tested package MSRV. The examined resolution used `windows 0.61.3`, whose manifest declares Rust 1.74; transitive dependencies and lockfile resolution can raise the effective compiler requirement. Pin and test the complete dependency graph for an older compiler. [Published manifest](https://docs.rs/crate/win32job/2.0.3/source/Cargo.toml), [windows 0.61.3 manifest](https://docs.rs/crate/windows/0.61.3/source/Cargo.toml).

There are **no crate-defined Cargo features**, including no optional default feature set to disable. Dependencies are `thiserror ^1.0` and `windows ^0.61`; the latter enables `Win32_Foundation`, `Win32_Security`, `Win32_System_JobObjects`, `Win32_System_Threading`, and `Win32_System_ProcessStatus`. `rusty-fork ^0.3` is development-only. No bundled driver, C/C++ shim, or native build script is included. Windows system libraries supply the actual implementation. [Feature listing](https://docs.rs/crate/win32job/2.0.3/features), [archive manifest](https://static.crates.io/crates/win32job/win32job-2.0.3.crate).

Use a Windows-target dependency and gate calling code with `cfg(windows)`. The release imports Windows bindings unconditionally; it is not a Unix implementation. Its docs.rs default target is `x86_64-pc-windows-msvc`. The source does not impose an x86-only architecture gate: x86, x64, and ARM64 Windows are candidate targets where Rust and the bindings support them, but this research executed only x64 and makes no x86/ARM64 test claim. Pointer-sized handles, sizes, and affinity masks require architecture-aware conversion. Native MSVC builds need the Rust target, MSVC build tools and Windows SDK/linker; GNU builds need the corresponding Windows GNU toolchain for linking.

## What 2.0.3 actually wraps

A job is a kernel object with process membership and resource policy. The wrapper's complete operational surface is small:

| Operation                 | Published API and contract                                                                                                                                                                                                                                     |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Create                    | `Job::create()` creates an **anonymous** job with no members or limits. `create_with_limit_info(&info)` creates then sets limits; a failed set drops the new handle.                                                                                           |
| Open existing job         | **Absent**: no name/security-descriptor constructor, `open`, or `from_raw_handle` constructor. Native `CreateJobObjectW`/`OpenJobObjectW` and another owner are needed.                                                                                        |
| Construct/update limits   | `ExtendedLimitInfo::new()`/`Default` starts with zeroed information and no enabled flags. Mutating helpers return `&mut Self`; apply with `set_extended_limit_info(&info)`. `clear_limits()` clears flags, not every stored numeric field.                     |
| Assign                    | `assign_process(isize)` borrows a **process handle**, not a PID, and returns `Result<(), JobError>`. It neither closes that handle nor owns the process. `assign_current_process()` exists but is unsuitable for the launcher or a shared runner.              |
| Query                     | `query_extended_limit_info()` returns the opaque wrapper. `query_process_id_list()` returns `Vec<usize>` including processes in child jobs. There are no public native-field getters, accounting queries, completion-port methods, or `IsProcessInJob` helper. |
| Terminate                 | **No explicit method**. Use native `TerminateJobObject` through the borrowed job handle, or configure last-handle-close termination.                                                                                                                           |
| Borrow/transfer ownership | `handle(&self) -> isize` borrows; `into_handle(self) -> isize` consumes and suppresses closing. `Drop` calls `CloseHandle`, discarding any error. There is no `Clone` or standard `AsHandle`/`OwnedHandle` conversion.                                         |

These findings come from the **published archive**, rather than inferred native capabilities. [Job API](https://docs.rs/win32job/2.0.3/win32job/struct.Job.html), [limit API](https://docs.rs/win32job/2.0.3/win32job/struct.ExtendedLimitInfo.html), [versioned source archive](https://static.crates.io/crates/win32job/win32job-2.0.3.crate).

The public `utils` module also exposes `get_current_process()` (a borrowed pseudo-handle), `get_process_memory_info(isize)` (per-process counters), and `get_process_affinity_mask(isize)` (process/system masks). The latter two return `io::Error` directly. `ProcessMemoryCounters` includes working-set, page-fault, pool, pagefile and private-usage counters; it is telemetry, not a job accounting or enforcement API. Do not close the current-process pseudo-handle. [Published utilities](https://github.com/ohadravid/win32job-rs/blob/29913d5f6acd47a491f73819ff6d54b63fb851b2/src/utils.rs).

`Job` explicitly implements `Send` and `Sync`; `ExtendedLimitInfo` is also `Send + Sync`. Sharing through `Arc<Job>` is possible, but it does not serialize a query–modify–set transaction. Centralize policy updates or lock around the entire transaction so another thread cannot overwrite limits or clear kill-on-close. Never close a borrowed `handle()` or retain it beyond its owner; Windows can recycle the numeric handle. If using `into_handle()`, adopt it into exactly one RAII owner or close it exactly once. Casting between binding handle types changes representation only, not ownership. [Job trait contracts](https://docs.rs/win32job/2.0.3/win32job/struct.Job.html), [published `job.rs`](https://github.com/ohadravid/win32job-rs/blob/29913d5f6acd47a491f73819ff6d54b63fb851b2/src/job.rs).

### Capability matrix

| Control                             | Safe helper in 2.0.3                                  | Meaning / additional integration                                                                                                                                                                   |
| ----------------------------------- | ----------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Working set                         | `limit_working_memory(min, max)`                      | Per-process resident working-set bounds in bytes; **not committed memory**, not a tree-wide memory budget                                                                                          |
| Per-process committed memory        | None                                                  | Native `ProcessMemoryLimit` plus `JOB_OBJECT_LIMIT_PROCESS_MEMORY`                                                                                                                                 |
| Aggregate committed memory          | None                                                  | Native `JobMemoryLimit` plus `JOB_OBJECT_LIMIT_JOB_MEMORY`                                                                                                                                         |
| CPU affinity                        | `limit_affinity(usize)`                               | Processor mask, not a utilization percentage; validate system/group compatibility                                                                                                                  |
| Priority                            | `limit_priority_class(PriorityClass)`                 | `Normal`, `Idle`, `High`, `Realtime`, `BelowNormal`, `AboveNormal`                                                                                                                                 |
| Scheduling class                    | `limit_scheduling_class(u8)`                          | Native valid range 0–9; helper accepts any `u8` without checking                                                                                                                                   |
| CPU rate / weight / min-max         | None                                                  | Native `JobObjectCpuRateControlInformation`; hard cap throttles over scheduling intervals                                                                                                          |
| Per-process / per-job user CPU time | None                                                  | Native basic/extended fields and flags; 100-nanosecond user-time budgets, distinct from wall time                                                                                                  |
| Active process count                | None                                                  | Native `ActiveProcessLimit` and flag                                                                                                                                                               |
| UI restrictions                     | None                                                  | Native `JobObjectBasicUIRestrictions`; conflicts with nested-job composition                                                                                                                       |
| Kill on last job handle close       | `limit_kill_on_job_close()`                           | Kernel termination of members and nested descendants when the **last** job handle closes                                                                                                           |
| Breakaway permissions               | `limit_breakaway_ok()`, `limit_silent_breakaway_ok()` | These weaken descendant containment; leave both off for a managed tree                                                                                                                             |
| Other native facilities             | None                                                  | Completion ports, accounting/violation notifications, processor-group configuration, I/O limits, network bandwidth/DSCP controls, exception behavior, and membership tests need native integration |

The private inner `JOBOBJECT_EXTENDED_LIMIT_INFORMATION` cannot be extended by setting arbitrary public fields. Use a **separate native structure** with `QueryInformationJobObject`/`SetInformationJobObject` on `job.handle()`; do not transmute the wrapper. After native updates, a stale wrapper can overwrite those settings. [Released helpers/source](https://github.com/ohadravid/win32job-rs/blob/29913d5f6acd47a491f73819ff6d54b63fb851b2/src/limits.rs), [native information classes](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-setinformationjobobject).

Committed-memory caps reject a commit that would exceed the configured per-process or aggregate limit; they do not promise graceful recovery from allocator failure. Working-set restriction can instead increase paging and still permit a much larger allocation. [Extended memory fields](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_extended_limit_information).

CPU hard-rate control pauses job threads for the rest of a scheduling interval when their quota is exhausted. It is different from exhausting a user CPU-time allowance; neither is a wall-clock deadline. Relative CPU rates in nested jobs are measured against the parent's allocation. A launcher needs its own elapsed-time deadline and cancellation protocol. Native job termination is forced and cannot be postponed by members; allow graceful cancellation first only when the workload can be trusted to cooperate, then terminate the job. [CPU rate semantics](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_cpu_rate_control_information), [termination semantics](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-terminatejobobject).

## Membership, compatibility, and errors

### Descendants and nested jobs

Assignment cannot be undone for a live process. Normally `CreateProcess` descendants inherit membership; reparenting is not a way to leave it. Membership is not a universal audit of every process a workload can induce: a separate service or broker can create work outside the job, and Microsoft specifically notes the WMI `Win32_Process.Create` exception. The launcher must constrain access to such brokers separately. [Membership rules](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

Nested jobs arrived in **Windows 8 / Windows Server 2012**; Windows 7 and earlier cannot accept a process already in another job. On current Windows, an existing member can join another job only when a valid hierarchy can be formed; UI restrictions prevent that nesting. Ancestor limits remain effective and can be stricter than the launcher's settings. Create one empty inner job per workload and retain the outer job's restrictions; never assume nesting makes an inherited policy disappear. [Nested-job rules](https://learn.microsoft.com/en-us/windows/win32/procthread/nested-jobs).

CI workers, service hosts, IDEs, shells, and parent launchers **may** already use jobs; ordinary shell ancestry does not prove either presence or absence. Check the specific child with native `IsProcessInJob`, first with a null job to inspect any membership, then with the launcher's job to verify the required membership. The API needs query rights and returns a separate success status and membership Boolean. Do not treat a successful query returning false as success for containment. [Membership query](https://learn.microsoft.com/en-us/windows/win32/api/jobapi/nf-jobapi-isprocessinjob).

Do not add `CREATE_BREAKAWAY_FROM_JOB` merely to make CI launch succeed. It requires permission from the inherited job hierarchy and intentionally discards some inherited containment. Native breakaway and silent-breakaway permissions have different semantics; with neither enabled in the immediate job, descendants cannot request ordinary breakaway from it. [Basic-limit breakaway flags](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_basic_limit_information).

### Native version floor versus a supported deployment

| Facility                                                                   | Microsoft's native availability                               | Adoption consequence                                                                           |
| -------------------------------------------------------------------------- | ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Create, assign, query, set, terminate; basic and extended limits used here | API references list Windows XP / Server 2003 desktop minimums | An OS API's historical floor is **not** a promise that this Rust dependency graph supports XP. |
| Nested jobs                                                                | Windows 8 / Server 2012                                       | Required when composing with an existing job without breakaway                                 |
| CPU-rate control, not wrapped                                              | Windows 8 / Server 2012                                       | Can fail under Remote Desktop Services Dynamic Fair Share Scheduling                           |
| Job list at process creation, not wrapped                                  | Windows 10 / Server 2016                                      | Preferred modern launch path below; no silent fallback if required                             |

[Create requirements](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-createjobobjectw), [query requirements](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-queryinformationjobobject), [basic-limit requirements](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_basic_limit_information), [nested jobs](https://learn.microsoft.com/en-us/windows/win32/procthread/nested-jobs), [CPU rate requirements](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_cpu_rate_control_information), [process attributes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute).

**Recommendation:** define and test a supported Windows 10/11 or Server 2016+ deployment set, using a supported Rust toolchain. Do not advertise the historical native floor as the product floor. WSL2 Linux binaries use Linux process controls, not Windows jobs; launching a Windows `.exe` through interop is a separate native-Windows case, not confinement of Linux descendants.

### Failure behavior

| Stage          | What can fail / required response                                                                                                                                                      |
| -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Job creation   | `CreateFailed(io::Error)`; no job or process should be launched afterward                                                                                                              |
| Configuration  | `SetInfoFailed(io::Error)`; invalid sizes, ranges, flags, privileges, or unsupported information classes must refuse launch                                                            |
| Assignment     | `AssignFailed(io::Error)`; insufficient process rights, incompatible membership/UI restrictions, session mismatch, or a limit violation. Never resume a suspended child after refusal. |
| Query/readback | `GetInfoFailed(io::Error)`; preserve the job owner, terminate any created child, report failure rather than claiming policy was installed                                              |
| Drop           | Close failure is discarded by the wrapper; use explicit termination plus waits when shutdown success must be observable                                                                |

`JobError` is non-exhaustive. Its `Display` is a generic stage description, so log the contained `io::Error` (or convert with `io::Error::from`) as well. Binding errors converted to `io::Error` can retain **HRESULT-shaped numeric codes**: this research's refusal produced `-2147024891` (`0x80070005`, access denied), not the plain Win32 value 5. Do not rely on a blanket `raw_os_error() == Some(5)` test for wrapper errors. [Released error implementation](https://github.com/ohadravid/win32job-rs/blob/29913d5f6acd47a491f73819ff6d54b63fb851b2/src/error.rs).

Ordinary management of an accessible child does not inherently require an administrator account; access rights, token restrictions and particular privileged limits still matter. A newly created job handle has `JOB_OBJECT_ALL_ACCESS`. Assignment requires `JOB_OBJECT_ASSIGN_PROCESS` on the job and `PROCESS_SET_QUOTA | PROCESS_TERMINATE` on the process. Query, set and termination require their corresponding job rights. The wrapper does not enable privileges or validate scheduling, affinity or working-set arguments. Microsoft documents priority-class privilege requirements and extra privilege for scheduling class above 5. Validate configuration, then trust the actual OS call outcome, not builder success. Assignment itself can terminate a process when an active-process limit is exceeded; native time-limit rules can also terminate members. [Job rights](https://learn.microsoft.com/en-us/windows/win32/procthread/job-object-security-and-access-rights), [assignment rules](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject), [limit validation](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_basic_limit_information).

### Launch ordering and ownership

The crate creates **jobs, not processes**. A normal `Command::spawn()` followed by assignment lets the payload execute and create descendants before containment. Suspending an already running process does not undo that exposure or reliably stop every thread.

A caller can use `CreateProcessW(CREATE_SUSPENDED)`, retain both returned handles, assign with `Job::assign_process`, verify membership, and call `ResumeThread` only after policy succeeds. Any error requires `TerminateProcess` and waiting for the dedicated child. This prevents payload execution before assignment, but a supervisor crash **between creation and assignment** can leave an abandoned suspended process. For a lifecycle layer that must avoid that window, use `STARTUPINFOEXW` and **`PROC_THREAD_ATTRIBUTE_JOB_LIST`** with `EXTENDED_STARTUPINFO_PRESENT`. Windows assigns jobs before the initial thread runs. [Microsoft's explanation of both approaches](https://devblogs.microsoft.com/oldnewthing/20230209-00/?p=107812), [attribute API](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute).

The attribute's availability appeared in search-visible Microsoft reference content while one rendered view omitted its row. Microsoft's detailed engineering article independently describes the Windows 10 addition. This is an evidence-display inconsistency, not evidence of an absent feature. Neither source is a formal specification of every interrupted-`CreateProcessW` cleanup outcome; do not generalize the before-execution guarantee into a proof of all kernel crash interleavings.

Use one non-inheritable job handle owned by the supervisor. The crate creates jobs with null security attributes, so the returned handle is non-inheritable by default. If the supervisor dies, Windows closes its handles; **kill-on-close works only if this closes the last job handle**. An inherited/duplicated handle, another supervisor, or a leaked owner can keep the job alive. An unrestricted same-user attacker may also have ways to obtain handles to the supervisor; resource supervision is not protection against that identity. [Creation and inheritance contract](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-createjobobjectw).

Process and thread handles are separate: keeping a child **process** handle open for waiting does not keep the **job** handle open. Closing a process handle does not kill the process. Keeping a duplicated job handle open delays last-close termination. Avoid assigning the launcher itself: its cleanup could kill the launcher and obscure the error it was returning, as reported in closed upstream issue #7. [Upstream report](https://github.com/ohadravid/win32job-rs/issues/7), [process creation handle ownership](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw).

## Practical policy and material gaps

Useful workloads include test subprocesses, build workers and plugin helpers whose descendants must be cleaned up, and workloads needing a uniform working-set or scheduling policy. Resource limits reduce accidental resource exhaustion and enforce specific kernel budgets when installed correctly. They do not revoke the user's ability to access files, sockets, registry keys or IPC.

| Requirement or gap                                           | Classification                           | Consequence and mitigation                                                                                                                                                                        |
| ------------------------------------------------------------ | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Read-only input directories                                  | Another isolation mechanism              | Jobs cannot deny writes. Use an appropriate restricted identity/AppContainer with object ACLs or a stronger isolated environment; verify write, delete, rename and mapping behavior.              |
| Writable workspace, all other destinations denied            | Another isolation mechanism              | A working directory is not authority. Grant the intended objects to the restricted identity, including required executable/DLL/runtime dependencies, and test outside access.                     |
| No networking                                                | Another isolation mechanism              | Jobs do not block sockets. AppContainer capabilities/network isolation or a separately configured network enforcement layer is needed; environment variables and proxy settings are insufficient. |
| Only approved remote destinations                            | Another isolation mechanism              | A job network-rate cap is bandwidth policy, not destination filtering. Use an enforcing broker/proxy plus prevention of direct connections, or another suitable network policy layer.             |
| Missing commit, CPU-time/rate, process-count and UI controls | Native capabilities missing from crate   | Direct bindings can install these; configure/query before creation and reject unsupported requirements.                                                                                           |
| Crash between suspended creation and assignment              | Integration constraint                   | Prefer creation-time job lists; a Rust cleanup guard cannot run after abrupt supervisor death.                                                                                                    |
| Existing outer jobs and UI limits                            | OS nesting rules / deployment constraint | Probe the actual host and fail when the required hierarchy cannot form; do not silently shed policy.                                                                                              |
| Broker-created processes outside membership                  | OS membership scope plus isolation need  | Constrain access to WMI, services and other IPC brokers; membership alone cannot account for delegated work.                                                                                      |
| Shared/inherited job handle                                  | Ownership constraint                     | Prevent inheritance and unauthorized duplication; explicitly define shared-supervisor lifetime semantics.                                                                                         |
| Exact PID enumeration beyond a small job                     | Crate implementation limitation          | Fixed buffer, no resize/retry; use a native dynamic query or accounting/completion strategy.                                                                                                      |

For access isolation, install token/security capabilities **at creation**, not after arbitrary code starts. In a multithreaded launcher, blanket handle inheritance can expose another thread's temporary file, pipe or socket handles; use a deliberate `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` when inheritance is needed, and keep the job handle out. A pre-opened file or connected socket can carry authority past a later pathname/network check. Audit inherited handles, standard streams, environment, brokers and IPC along with the token. [Process attributes and inheritance](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute), [AppContainer access model](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation).

A read-only ACL grant does not cancel another grant of write authority. Review the restricted identity's effective access, including ambient AppContainer access, instead of assuming an added read grant creates a strict allowlist.

Path strings are not object identities: junctions, symlinks/reparse points, hard links, alternate path spellings and rename races matter to the **separate** filesystem policy. Likewise, executable search, DLL loading, interpreter module paths and writable runtime directories affect what code receives authority. Treat these as caller/isolation-layer validation requirements; `win32job` performs no path resolution, ACL design, handle sanitization or IPC mediation. Windows job UI restrictions, even when set natively, are not a substitute for those controls.

### Defects and advisory status

- The historical PID-query correctness bug is fixed in 1.0.4 and 2.0.0 and therefore in 2.0.3. The yanked 1.0.3 compatibility break is not a new 2.0.3 defect. [Fix](https://github.com/ohadravid/win32job-rs/pull/5), [yank context](https://github.com/ohadravid/win32job-rs/issues/3).
- **Still applicable:** published `query.rs` uses a native one-entry header plus 1,024 extra PID slots and has a TODO describing failures above roughly 1,024 processes. It does not grow the buffer on `ERROR_MORE_DATA`. Enumeration is also a changing snapshot, not proof no descendants can appear. Do not implement reliable tree termination by enumerating and individually killing PIDs. [Published query implementation](https://github.com/ohadravid/win32job-rs/blob/29913d5f6acd47a491f73819ff6d54b63fb851b2/src/query.rs).
- The open missing-limit request confirms the limited API, not a security patch waiting to be installed. No released membership helper exists despite issue #13. [Limit request](https://github.com/ohadravid/win32job-rs/issues/6), [membership proposal](https://github.com/ohadravid/win32job-rs/issues/13).
- A November 4, 2026 [OSV query](https://api.osv.dev/v1/query) for ecosystem `crates.io`, package `win32job`, version `2.0.3` returned `{}`; a RustSec site search found no matching advisory. This is a bounded negative lookup, **not** a complete dependency audit or evidence that the wrapper is secure. Repeat against the deployment lockfile and review upstream issues before adoption. [RustSec database](https://rustsec.org/advisories/).

## Complete Rust integration and probes

The following is one standalone program with five externally invoked probe modes. It launches only copies of itself, configures the **actually wrapped working-set limit**, and keeps the caller and observer outside the new job. It adds direct `windows-sys` calls for process creation, creation-time membership, native readback, membership checking, thread resumption, termination and waiting. This makes the unsafe integration boundary explicit.

**Scope:** demonstrates resource-policy installation and lifecycle behavior, not filesystem/network isolation. It uses the current executable's full path, a fixed trusted command line, no general argument quoting, no inherited handles for the payload, and `CREATE_NO_WINDOW`. It inherits the environment and current directory; a production sandbox must supply its own reviewed launch policy. The working-set readback verifies configuration, not a committed-memory cap or an adversarial memory-enforcement test.

Create a standalone directory with this `Cargo.toml`. The `[workspace]` keeps it independent if copied inside a monorepo. There are no `win32job` features to enable; direct bindings need the listed features. Commit a generated lockfile in a real application to freeze transitive resolution.

```toml
[package]
name = "win32job-probe"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]

[target.'cfg(windows)'.dependencies]
win32job = "=2.0.3"
windows-sys = { version = "=0.61.2", features = [
    "Win32_Foundation", "Win32_Security", "Win32_System_JobObjects",
    "Win32_System_Threading"
] }
```

Save this complete source as `src/main.rs`:

```rust
#[cfg(not(windows))]
fn main() { eprintln!("This probe requires native Windows."); std::process::exit(1); }

#[cfg(windows)]
mod native {
    use std::{ffi::OsStr, io::{self, BufRead, BufReader}, mem::{size_of, zeroed},
        os::windows::{ffi::OsStrExt, process::CommandExt}, process::{Command, Stdio}, ptr::{null, null_mut},
        thread, time::{Duration, Instant}};
    use win32job::{ExtendedLimitInfo, Job};
    use windows_sys::Win32::{Foundation::*, System::{JobObjects::*, Threading::*}};

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
    fn check(ok: i32) -> io::Result<()> {
        if ok == 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
    }
    fn wide(s: &OsStr) -> Vec<u16> { s.encode_wide().chain(Some(0)).collect() }
    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) { unsafe { CloseHandle(self.0); } }
    }
    fn open(pid: u32, rights: u32) -> io::Result<Handle> {
        let h = unsafe { OpenProcess(rights, 0, pid) };
        if h.is_null() { Err(io::Error::last_os_error()) } else { Ok(Handle(h)) }
    }
    fn ended(h: &Handle) -> Result<()> {
        match unsafe { WaitForSingleObject(h.0, 5000) } {
            WAIT_OBJECT_0 => Ok(()),
            WAIT_FAILED => Err(io::Error::last_os_error().into()),
            _ => Err("process did not exit within five seconds".into()),
        }
    }
    struct Child { process: Handle, thread: Handle, pid: u32 }
    impl Drop for Child {
        fn drop(&mut self) {
            // Also covers failures before membership is established.
            unsafe { TerminateProcess(self.process.0, 125); }
            let _ = ended(&self.process);
        }
    }
    struct Attributes { _storage: Vec<usize>, ptr: LPPROC_THREAD_ATTRIBUTE_LIST }
    impl Attributes {
        fn new() -> Result<Self> {
            let mut bytes = 0;
            let first = unsafe { InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut bytes) };
            if first != 0 || io::Error::last_os_error().raw_os_error() != Some(122) || bytes == 0 {
                return Err("unexpected attribute-list sizing result".into());
            }
            let mut storage = vec![0usize; bytes.div_ceil(size_of::<usize>())];
            let ptr = storage.as_mut_ptr().cast();
            check(unsafe { InitializeProcThreadAttributeList(ptr, 1, 0, &mut bytes) })?;
            Ok(Self { _storage: storage, ptr })
        }
    }
    impl Drop for Attributes {
        fn drop(&mut self) { unsafe { DeleteProcThreadAttributeList(self.ptr); } }
    }
    fn create(job: &Job, atomic: bool) -> Result<Child> {
        let exe = std::env::current_exe()?;
        let app = wide(exe.as_os_str());
        let mut command = wide(OsStr::new("probe --tree"));
        let jobs = [job.handle() as HANDLE];
        let attrs = Attributes::new()?;
        if atomic {
            check(unsafe { UpdateProcThreadAttribute(attrs.ptr, 0,
                PROC_THREAD_ATTRIBUTE_JOB_LIST as usize, jobs.as_ptr().cast(),
                size_of::<HANDLE>(), null_mut(), null()) })?;
        }
        let mut si: STARTUPINFOEXW = unsafe { zeroed() };
        si.StartupInfo.cb = if atomic { size_of::<STARTUPINFOEXW>() } else { size_of::<STARTUPINFOW>() } as u32;
        if atomic { si.lpAttributeList = attrs.ptr; }
        let mut pi: PROCESS_INFORMATION = unsafe { zeroed() };
        let flags = CREATE_SUSPENDED | CREATE_NO_WINDOW |
            if atomic { EXTENDED_STARTUPINFO_PRESENT } else { 0 };
        check(unsafe { CreateProcessW(app.as_ptr(), command.as_mut_ptr(), null(), null(),
            0, flags, null(), null(), &si.StartupInfo, &mut pi) })?;
        Ok(Child { process: Handle(pi.hProcess), thread: Handle(pi.hThread), pid: pi.dwProcessId })
    }
    fn membership(job: &Job, child: &Child) -> Result<()> {
        let mut yes = 0;
        check(unsafe { IsProcessInJob(child.process.0, job.handle() as HANDLE, &mut yes) })?;
        if yes == 0 { return Err("required job membership missing".into()); }
        Ok(())
    }
    fn configured() -> Result<Job> {
        let mut info = ExtendedLimitInfo::new();
        info.limit_working_memory(1024 * 1024, 64 * 1024 * 1024)
            .limit_kill_on_job_close();
        let job = Job::create_with_limit_info(&info)?;
        let mut actual: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
        check(unsafe { QueryInformationJobObject(job.handle() as HANDLE,
            JobObjectExtendedLimitInformation, (&mut actual as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32, null_mut()) })?;
        let b = actual.BasicLimitInformation;
        let required = JOB_OBJECT_LIMIT_WORKINGSET | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if b.LimitFlags & required != required ||
            b.LimitFlags & (JOB_OBJECT_LIMIT_BREAKAWAY_OK | JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK) != 0 ||
            b.MinimumWorkingSetSize != 1024 * 1024 || b.MaximumWorkingSetSize != 64 * 1024 * 1024 {
            return Err("required job configuration did not round-trip".into());
        }
        Ok(job)
    }
    fn observe() -> Result<()> {
        let mut supervisor = Command::new(std::env::current_exe()?).arg("death").creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::piped()).spawn()?;
        let result = (|| -> Result<()> {
            let mut line = String::new();
            BufReader::new(supervisor.stdout.take().ok_or("missing pipe")?).read_line(&mut line)?;
            let ids: Vec<u32> = line.strip_prefix("READY ").ok_or("no readiness record")?
                .split_whitespace().map(str::parse).collect::<std::result::Result<_, _>>()?;
            if ids.len() < 2 { return Err("tree was not established".into()); }
            let handles = ids.into_iter().map(|pid| open(pid, PROCESS_SYNCHRONIZE))
                .collect::<io::Result<Vec<_>>>()?;
            supervisor.kill()?; // Abrupt death: no Rust destructors in supervisor.
            supervisor.wait()?;
            for h in handles { ended(&h)?; }
            Ok(())
        })();
        let _ = supervisor.kill();
        let _ = supervisor.wait();
        result
    }
    pub fn run() -> Result<()> {
        let mode = std::env::args().nth(1).unwrap_or_else(|| "kill".into());
        if mode == "--tree" || mode == "--leaf" {
            if mode == "--tree" {
                let mut leaf = Command::new(std::env::current_exe()?).arg("--leaf").creation_flags(CREATE_NO_WINDOW)
                    .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
                thread::sleep(Duration::from_secs(30));
                leaf.kill()?;
                leaf.wait()?;
            } else { thread::sleep(Duration::from_secs(30)); }
            return Ok(());
        }
        if mode == "observe" { return observe(); }
        if !["kill", "close", "death", "assign", "refuse"].contains(&mode.as_str()) {
            return Err("expected kill, close, death, assign, refuse, or observe".into());
        }
        let job = configured()?;
        let child = create(&job, mode != "assign" && mode != "refuse")?;
        if mode == "refuse" {
            let query_only = open(child.pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
            return match job.assign_process(query_only.0 as isize) {
                Err(e) => {
                    let e: io::Error = e.into();
                    println!("assignment refused; child never resumed: {e:?}");
                    Ok(())
                }
                Ok(()) => Err("assignment unexpectedly accepted query-only handle".into()),
            };
        }
        if mode == "assign" { job.assign_process(child.process.0 as isize)?; }
        membership(&job, &child)?;
        if unsafe { ResumeThread(child.thread.0) } != 1 {
            return Err("unexpected initial suspend count; refusing continuation".into());
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        let ids = loop {
            let ids = job.query_process_id_list()?;
            if ids.len() >= 2 { break ids; }
            if Instant::now() >= deadline { return Err("descendant did not become a job member".into()); }
            thread::sleep(Duration::from_millis(20));
        };
        let handles = ids.iter().map(|pid| open(*pid as u32, PROCESS_SYNCHRONIZE))
            .collect::<io::Result<Vec<_>>>()?;
        if mode == "death" {
            use std::io::Write;
            println!("READY {}", ids.iter().map(ToString::to_string).collect::<Vec<_>>().join(" "));
            io::stdout().flush()?;
            thread::sleep(Duration::from_secs(20));
            return Err("observer did not kill supervisor before deadline".into());
        }
        if mode != "close" {
            check(unsafe { TerminateJobObject(job.handle() as HANDLE, 124) })?;
        }
        drop(job);
        for h in handles { ended(&h)?; }
        println!("{mode}: dedicated child and descendant exited");
        Ok(())
    }
}
#[cfg(windows)]
fn main() {
    if let Err(e) = native::run() { eprintln!("launch/probe refused: {e:?}"); std::process::exit(1); }
}
```

Build and run in a native Windows PowerShell session with Rust's Windows toolchain and build tools installed:

```powershell
cargo build --color=never
if ($LASTEXITCODE -ne 0) { throw "build failed" }
foreach ($mode in @("kill", "close", "assign", "refuse", "observe")) {
    & .\target\debug\win32job-probe.exe $mode
    if ($LASTEXITCODE -ne 0) { throw "probe failed: $mode" }
}
```

| Mode      | What it verifies                                                                                                                                                                                                                                                |
| --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `kill`    | Creation-time job membership; configuration and membership readback before resume; a real child creates a real descendant; native `TerminateJobObject` ends both.                                                                                               |
| `close`   | Same setup, then dropping the sole job owner ends both processes; retained process handles allow independent exit waits.                                                                                                                                        |
| `assign`  | Explicit `Job::assign_process` of a suspended dedicated child succeeds before resume; tree termination succeeds. **Has the pre-assignment supervisor-crash gap.**                                                                                               |
| `refuse`  | Passes a real query-only child handle to assignment, which lacks required rights. Child is never resumed and its guard terminates/reaps it. A reported refusal is success for this negative probe; ordinary launch errors still fail the program.               |
| `observe` | A separate observer opens handles for both workload members, abruptly kills the dedicated supervisor after readiness, and waits for both members to exit. The supervisor's destructors cannot supply the cleanup. It never assigns the observer to the new job. |

The `death`, `--tree`, and `--leaf` arguments are internal helpers. `death` waits at most 20 seconds for the observer, and workers expire after 30 seconds. Run these controlled probes under an external test deadline as well; OS process creation and pipe reads are not all cancellable by the program's polling deadline. On normal failure paths the child guard attempts direct termination and waits; destructor failures cannot be returned, so production cleanup should report explicit termination/wait failures and retain an outer watchdog when cleanup cannot be confirmed.

The normal modes use the native job list **and** suspended creation: atomic membership avoids the separate creation/assignment window, while suspension permits readback before execution. The legacy `assign`/`refuse` modes deliberately omit the job-list attribute to test the wrapper. There is no automatic fallback from atomic creation if the OS rejects the attribute or hierarchy. Native creation failure or missing required configuration prevents `ResumeThread`; do not interpret that as proof of access isolation.

### Validation performed

On November 4, 2026:

| Environment                                                                                                             | Result                                                                                                                                                    |
| ----------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| macOS 27.2, Apple Silicon; `rustc 1.98.1 (48a229cea 2026-09-01)`                                                        | `cargo check --target x86_64-pc-windows-gnu` passed for the exact manifest/source above. **Cross-compilation only.**                                      |
| Native Windows x64, reported OS version `10.0.26300.0`; `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `x86_64-pc-windows-msvc` | Built with the same lockfile and ran `kill`, `close`, `assign`, `refuse`, and `observe`; all returned success. The refusal reported HRESULT `0x80070005`. |

Resolution included `win32job 2.0.3`, `windows 0.61.3`, `windows-sys 0.61.2`, and `thiserror 1.0.69`. These observations establish this example's behavior on one rig, including descendant and supervisor-death termination after readiness. They do **not** establish x86/ARM64 behavior, every Windows build, every outer-job configuration, memory-pressure behavior, or supervisor death during the native creation call. No AppContainer, firewall, account ACL, or persistent host security settings were changed.

### Additional isolated acceptance tests before adoption

Use trusted fixtures in a disposable working directory and a supervisor outside the workload job. Give every probe a deadline and cleanup path. Never assign the research tool or a shared test runner itself.

| Test                                  | Allowed control and denied/failure assertion                                                                                                                                                                                                                         | Status here                                    |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| Required unsupported limit            | A valid configuration launches; malformed or unavailable native configuration returns an error before child resume. Do not retry with the limit removed.                                                                                                             | Design only                                    |
| Missing executable / resume failure   | Known helper launches; invalid full executable path fails with no running payload. Inject resume failure and verify child termination.                                                                                                                               | Design only                                    |
| Ordinary breakaway                    | Prove the fixture can create its helper normally; attempt `CREATE_BREAKAWAY_FROM_JOB` under the managed job and assert no uncontained helper survives. If an outer host job already forbids breakaway in the control, report **unavailable**, not a sandbox pass.    | Design only                                    |
| Handle sharing                        | Duplicate the job handle into a trusted holder, drop original and confirm worker remains; close duplicate and wait for worker exit. Keep the holder outside the job and under a timeout.                                                                             | Design only                                    |
| Committed-memory/process-count denial | Add native caps; below-cap work succeeds, above-cap commit or process creation fails. Verify membership and actual error/postcondition, not merely nonzero child exit. Keep allocation/process counts bounded.                                                       | Design only; not available via wrapper helpers |
| CPU budget/rate                       | With native controls, separately measure user-time termination and interval throttling; compare against a workload that can exceed the limit outside the job.                                                                                                        | Design only                                    |
| Outer-job compatibility               | Dedicated subprocess fixtures exercise compatible nesting and incompatible UI/other hierarchy conditions. Verify error causes and cleanup. Never alter the runner's own job.                                                                                         | Design only                                    |
| Read-only/workspace/outside access    | With jobs alone, a same-token helper should still write outside its working directory where the user can. With an isolation layer, prove input reads/workspace writes succeed and input writes/outside reads or writes fail, including descendants and path aliases. | Design only; jobs provide no denial            |
| No network/approved destination       | Prove the helper can connect to a controlled listener first; with the separate policy active, verify permitted endpoints work and prohibited endpoints, existing sockets, direct IP access and proxy bypass fail. Avoid public endpoints.                            | Design only; jobs provide no denial            |

For creation-crash stress, high process counts, or tests whose cleanup cannot be made reliable, use a disposable VM. A successful launch, a loader failure, a host-blocked control, or a missing PID is never proof that a forbidden operation was denied by the intended mechanism.

## Alternatives and complementary layers

Versions below identify the material inspected; the table is not a blanket endorsement of latest releases.

| Choice                                | Role and platform                                                                                                                | Integration and maintenance trade-off                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `win32job 2.0.3`                      | Windows job wrapper; resource/lifetime layer                                                                                     | Small safe surface, three 2025 releases, last release May 2025. Direct integration still required for launch and most budgets.                                                                                                                                                                                                                                                                                                                                                             |
| Direct `windows` / `windows-sys`      | Microsoft's competing Rust projections of Windows APIs, not sandbox launchers                                                    | Full native job information classes and process attributes; most flexibility and the most ownership/error-handling work. `windows` supplies typed wrappers/`Result`; `windows-sys` supplies raw declarations. Microsoft-maintained bindings avoid waiting for a narrow wrapper to expose a flag. [Binding project](https://github.com/microsoft/windows-rs), [windows-sys 0.61.2](https://docs.rs/crate/windows-sys/0.61.2).                                                               |
| `process-wrap 10.0.1`                 | Higher-level command lifecycle wrapper with std/Tokio frontends; Windows jobs and separate Unix process-group/session facilities | Successor to `command-group`; published September 23, 2026. Documented suspended-create/assign/resume ordering reduces application code, but does not by itself supply a creation-time job list or filesystem/network policy. Check lifecycle flags and frontend features. [Versioned API](https://docs.rs/process-wrap/10.0.1/process_wrap/), [release metadata](https://docs.rs/crate/process-wrap/10.0.1).                                                                              |
| `subc-jobobject 0.2.3`                | Another Windows job wrapper, focused on supervised subprocess trees                                                              | Supplies `spawn_contained` and suspended-child helpers; lowers assignment cleanup work. Its docs explicitly require suspended creation/assignment/resumption. Less broad ecosystem evidence was inspected; no independent execution here. Assess source, release history and ownership before choosing. [Versioned source/API](https://docs.rs/subc-jobobject/0.2.3/subc_jobobject/).                                                                                                      |
| `rappct 0.13.3`                       | Complementary Windows AppContainer/LPAC identity, capabilities, ACL and launch toolkit                                           | Documents optional job limits alongside AppContainer launch. MSRV 1.90. Greater access-policy coverage and more deployment setup than a job wrapper. Review released source: its rustdoc still labels some modules “skeleton,” so do not treat the overview as an audit. Avoid two independent owners fighting over resource configuration. [Versioned API](https://docs.rs/rappct/0.13.3/rappct/).                                                                                        |
| `sandboxrs-windows 0.1.1`             | Higher-level Windows policy/launcher layer using `rappct` fallback and an experimental modern backend                            | Published August 11, 2026, after 0.1.0 on August 8. Documents workspace/read-only rules, limits and capability probing. Its preferred modern API is explicitly experimental; reported AppContainer evaluations are project evidence, not reproduced here or a security guarantee. Higher policy abstraction can reduce integration cost but adds backend/policy/runtime compatibility questions. [Published package and evaluation claims](https://docs.rs/crate/sandboxrs-windows/0.1.1). |
| VM-backed execution / Windows Sandbox | Stronger boundary, not another job wrapper                                                                                       | Prefer a separate kernel/VM boundary when the threat model includes hostile native code attacking shared-kernel or same-user services. Higher deployment/startup cost; resources, filesystem sharing and networking still need explicit policy. [Windows Sandbox architecture](https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/windows-sandbox-architecture).                                                                |

AppContainer and restricted-token launchers address **identity and access checks**; jobs address **membership and resources**. They can compose in one process-creation path, with security attributes/capabilities and job membership established before untrusted execution. If a chosen launcher already owns a job, extend that integration or deliberately establish a valid nested hierarchy. Do not spawn through one layer and attach the other after the payload starts. [AppContainer model](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation), [process security attributes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute).

## Authoritative resources and remaining adoption questions

- [Published win32job 2.0.3 archive](https://static.crates.io/crates/win32job/win32job-2.0.3.crate), [versioned API](https://docs.rs/win32job/2.0.3/win32job/), [registry versions](https://crates.io/api/v1/crates/win32job/versions), [upstream releases](https://github.com/ohadravid/win32job-rs/releases).
- [Microsoft job overview](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects), [nested jobs](https://learn.microsoft.com/en-us/windows/win32/procthread/nested-jobs), [job access rights](https://learn.microsoft.com/en-us/windows/win32/procthread/job-object-security-and-access-rights).
- [Basic limits](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_basic_limit_information), [extended limits](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_extended_limit_information), [CPU rates](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_cpu_rate_control_information), [creation-time job assignment](https://devblogs.microsoft.com/oldnewthing/20230209-00/?p=107812).

Material unresolved questions for the adopting launcher:

1. Which Windows builds, architectures, outer-job hierarchies and compiler/dependency lockfiles are supported? The crate supplies no maintained compatibility matrix or explicit MSRV promise.
2. Are working-set and lifetime controls enough, or must the launcher own direct committed-memory, CPU and process-count configuration and telemetry? How will it serialize all updates?
3. Must supervisor death during process creation have a formally specified all-or-nothing cleanup contract? The cited public references establish assignment before execution, but this research does not prove every interrupted-call outcome.
4. Which identity/access layer enforces input, workspace, IPC and network policy, and how are inherited authority, executable/runtime access and aliases validated? Jobs cannot answer these questions.
5. Will the application accept upstream's bounded PID-query API or maintain its own dynamic queries and notification handling? Which native tests will prevent behavior from silently degrading on customer hosts?
