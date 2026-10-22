//! Native Windows backend: file-side handle owners and loaded modules, with process
//! identity read through a process handle the backend keeps open.
//!
//! The classification logic is generic over [`Source`], so tests can script
//! denials, exits, and undersized buffers on any host; the FFI source
//! ([`ffi::Win32`]) exists on Windows only. A module is observed by path
//! only, so the backend looks each in-scope path up as it reads it and keeps
//! that match with the evidence; a module the budget stopped before its
//! lookup is a coverage gap, never evidence.
//!
//! Foreign file handles are never duplicated: reading the identity of
//! a duplicated handle can block for as long as its owner has synchronous
//! I/O pending on it, and no call cancels that wait. A working directory and
//! a `ReadDirectoryChangesW` subscription cannot be told apart from other
//! directory handles at all.

use super::backend::{
    Enumeration, InspectionContext, InspectionResult, MechanismInspection, MechanismSupport,
    ProcessCandidate, ProcessDetails, RawEvidence, Tally, UsageBackend,
};
use super::budget::Budget;
use super::matching::ObservedObject;
use super::report::{EvidenceKind, LimitationExample, LimitationKind, Limitations, Mechanism};
use crate::performance::{self, counters};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;

pub(crate) const OPEN_HANDLES_UNSUPPORTED: &str =
    "other processes' handles are not inspected in this build: reading a handle's identity can \
     block until its owner's pending I/O completes, and that wait cannot be cancelled";
pub(crate) const WORKING_DIRECTORIES_UNSUPPORTED: &str =
    "a working directory is indistinguishable from other directory handles without reading \
     another process's memory";
pub(crate) const DIRECTORY_CHANGE_SUBSCRIPTIONS_UNSUPPORTED: &str =
    "handle ownership does not identify a ReadDirectoryChangesW subscription";

const BUDGET_EXPIRED: &str = "the budget expired while a process's modules were being read";

/// Module-list capacity for the first read.
pub(crate) const DEFAULT_MODULE_CAPACITY: usize = 256;
/// Added to the reported count, which can grow between two reads.
const MODULE_HEADROOM: usize = 16;
/// A list reported larger than this is refused rather than allocated.
pub(crate) const MAX_MODULE_CAPACITY: usize = 1 << 16;
/// Reads before a list that keeps outgrowing its buffer is a failure.
const MODULE_LIST_ATTEMPTS: usize = 8;

/// FILETIME ticks (100 ns) between 1601-01-01 and the Unix epoch.
const FILETIME_UNIX_EPOCH: u64 = 116_444_736_000_000_000;

/// Why a native call returned nothing usable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Fault {
    /// `ERROR_ACCESS_DENIED`: a protected, elevated, or other-user process.
    Denied,
    /// The process exited (an invalid PID) or the module was unloaded.
    Gone,
    Failed(String),
}

/// A module's base address in its process; meaningful to that process only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Module(pub(crate) usize);

/// A process opened for one query.
#[derive(Debug)]
pub(crate) struct Opened<H> {
    pub(crate) process: H,
    /// Creation FILETIME: this lifetime's creation token.
    pub(crate) creation_time: u64,
    pub(crate) image: Option<PathBuf>,
    /// The handle carries `PROCESS_VM_READ`, which module listing needs.
    pub(crate) can_read_modules: bool,
}

/// The native calls the backend makes.
pub(crate) trait Source {
    /// An open process handle. While it is open the PID cannot be reused.
    type Process;

    /// Optional file-side reader; scripted module-only sources need none.
    fn path_reader(&self) -> Option<super::win32_paths::PathReader> { None }

    /// Every PID on the host. `Err` names why the list could not be read.
    fn pids(&mut self) -> Result<Vec<u32>, String>;
    fn open(&mut self, pid: u32) -> Result<Opened<Self::Process>, Fault>;
    fn creation_time(&mut self, process: &Self::Process) -> Result<u64, Fault>;
    fn exited(&mut self, process: &Self::Process) -> bool;
    /// At most `capacity` modules, and how many the process had loaded when
    /// the list was read.
    fn modules(&mut self, process: &Self::Process, capacity: usize) -> Result<(Vec<Module>, usize), Fault>;
    fn module_path(&mut self, process: &Self::Process, module: Module) -> Result<PathBuf, Fault>;
    /// The path of every module in the process's loader list, read in one
    /// pass and keyed by module: the strings [`Source::module_path`] reads
    /// with one pass per module. A module missing here is read through
    /// [`Source::module_path`]; `None` reads every module that way.
    fn loader_paths(&mut self, _process: &Self::Process) -> Option<HashMap<Module, PathBuf>> {
        None
    }
    /// Image path and account SID, read through the handle.
    fn details(&mut self, process: &Self::Process) -> Result<ProcessDetails, String>;
}

/// A process handle kept from enumeration through enrichment.
struct Held<H> {
    process: H,
    can_read_modules: bool,
}

pub(crate) struct ModuleBackend<S: Source> {
    source: S,
    /// Handles for enumerated processes and confirmed path holders. Those
    /// with evidence stay open until the query returns, so
    /// enrichment reads the same lifetime.
    held: HashMap<u32, Held<S::Process>>,
    path_holders: std::collections::HashSet<u32>,
}

impl<S: Source> ModuleBackend<S> {
    pub(crate) fn with_source(source: S) -> Self {
        Self {
            source,
            held: HashMap::new(),
            path_holders: std::collections::HashSet::new(),
        }
    }
}

#[cfg(windows)]
impl ModuleBackend<ffi::Win32> {
    pub(crate) fn new() -> Self {
        Self::with_source(ffi::Win32)
    }
}

/// Converts a creation FILETIME to a timestamp.
pub(crate) fn filetime_to_utc(filetime: u64) -> Option<DateTime<Utc>> {
    let since_epoch = filetime.checked_sub(FILETIME_UNIX_EPOCH)?;
    let seconds = i64::try_from(since_epoch / 10_000_000).ok()?;
    let nanos = (since_epoch % 10_000_000) as u32 * 100;
    DateTime::from_timestamp(seconds, nanos)
}

impl<S: Source> UsageBackend for ModuleBackend<S> {
    /// Why the process could not be opened; `None` when its handle is held.
    type Payload = Option<Fault>;

    fn mechanisms(&self) -> Vec<MechanismSupport> {
        use Mechanism::*;
        vec![
            if self.source.path_reader().is_some() {
                MechanismSupport::supported(OpenHandles)
            } else {
                MechanismSupport::unsupported(OpenHandles, OPEN_HANDLES_UNSUPPORTED)
            },
            MechanismSupport::supported(LoadedModules),
            MechanismSupport::unsupported(WorkingDirectories, WORKING_DIRECTORIES_UNSUPPORTED),
            MechanismSupport::unsupported(
                DirectoryChangeSubscriptions,
                DIRECTORY_CHANGE_SUBSCRIPTIONS_UNSUPPORTED,
            ),
            MechanismSupport::unsupported(Polling, super::backend::POLLING_UNSUPPORTED),
        ]
    }

    fn enumerate(&mut self, budget: &Budget) -> Result<Enumeration<Option<Fault>>, String> {
        let pids = self.source.pids()?;
        let mut processes = Vec::new();
        let mut complete = true;
        let mut limitations = Limitations::default();
        for pid in pids {
            if budget.expired() {
                complete = false;
                limitations.record(
                    LimitationKind::BudgetExhausted,
                    "the budget expired before every process was listed",
                    1,
                    None,
                );
                break;
            }
            let mut candidate = ProcessCandidate {
                pid,
                creation_token: None,
                retained_handle: false,
                details: ProcessDetails::default(),
                payload: None,
            };
            match self.source.open(pid) {
                Ok(opened) => {
                    candidate.creation_token = Some(opened.creation_time);
                    candidate.retained_handle = true;
                    candidate.details = ProcessDetails {
                        name: opened
                            .image
                            .as_deref()
                            .and_then(|image| image.file_name())
                            .map(ToOwned::to_owned),
                        executable: opened.image,
                        user: None,
                        start_time: filetime_to_utc(opened.creation_time),
                    };
                    self.held.insert(
                        pid,
                        Held {
                            process: opened.process,
                            can_read_modules: opened.can_read_modules,
                        },
                    );
                }
                // Exited between the listing and the open (or the System Idle
                // pseudo-process, which cannot be opened at all).
                Err(Fault::Gone) => continue,
                // Inspection reports the denial; with no handle it can read
                // nothing else either.
                Err(Fault::Denied) => candidate.payload = Some(Fault::Denied),
                Err(Fault::Failed(detail)) => {
                    limitations.record(
                        LimitationKind::IdentityUncertain,
                        "some processes could not be opened to read their creation time",
                        1,
                        Some(LimitationExample::pid(pid).with_detail(detail.clone())),
                    );
                    candidate.payload = Some(Fault::Failed(detail));
                }
            }
            processes.push(candidate);
        }
        Ok(Enumeration {
            processes,
            complete,
            limitations: limitations.into_vec(),
        })
    }

    fn inspect(
        &mut self,
        candidate: &ProcessCandidate<Option<Fault>>,
        context: &InspectionContext<'_>,
    ) -> Vec<MechanismInspection> {
        let pid = candidate.pid;
        let mut modules = Tally::new(Mechanism::LoadedModules, pid);
        if let Some(fault) = &candidate.payload {
            whole(&mut modules, fault.clone());
            return vec![modules.finish()];
        }
        match self.held.get(&pid) {
            Some(held) if held.can_read_modules => {
                let held = self.held.remove(&pid).expect("held handle");
                self.inspect_modules(&held.process, context, &mut modules);
                // A process holding nothing in scope needs no enrichment, so
                // its handle closes now rather than when the query returns.
                if !modules.evidence.is_empty() || self.path_holders.contains(&pid) {
                    self.held.insert(pid, held);
                }
            }
            Some(_) => {
                if !self.path_holders.contains(&pid) { self.held.remove(&pid); }
                modules.whole = Some(InspectionResult::Denied);
            }
            None => modules.whole = Some(InspectionResult::Failed("the process handle was not kept".to_string())),
        }
        vec![modules.finish()]
    }

    fn inspect_paths(
        &mut self,
        context: &InspectionContext<'_>,
        candidates: &[ProcessCandidate<Self::Payload>],
    ) -> Option<(Vec<super::process::Observation>, super::report::Coverage)> {
        let reader = self.source.path_reader()?;
        let entries = context.index.probe_entries(&context.root.resolved);
        let (mut observations, mut coverage) = super::win32_paths::scan(context, &entries, &reader);
        let mut uncertain = Vec::new();
        for observation in &mut observations {
            if let Some(candidate) = candidates.iter().find(|c| c.pid == observation.pid)
                && let Some(held) = self.held.get(&observation.pid)
                && !self.source.exited(&held.process)
            {
                observation.creation_token = candidate.creation_token;
                observation.retained_handle = candidate.retained_handle;
                observation.details = candidate.details.clone();
                self.path_holders.insert(observation.pid);
            } else if !context.budget.expired()
                && let Ok(opened) = self.source.open(observation.pid)
            {
                // A PID first seen after enumeration needs a second file-side
                // observation while its new process handle is held; otherwise
                // its creation time might describe a successor.
                let entry = entries.iter().find(|entry| observation.evidence.iter()
                    .any(|e| e.observed_path.as_ref().is_some_and(|p| p.as_os_str() == entry.path.as_os_str())));
                if let Some(entry) = entry
                    && let Ok((identity, pids)) = reader(entry, context.budget)
                    && identity == entry.identity && pids.contains(&observation.pid)
                    && !self.source.exited(&opened.process)
                {
                    observation.creation_token = Some(opened.creation_time);
                    observation.retained_handle = true;
                    observation.details = ProcessDetails {
                        name: opened.image.as_deref().and_then(|p| p.file_name()).map(ToOwned::to_owned),
                        executable: opened.image, user: None,
                        start_time: filetime_to_utc(opened.creation_time),
                    };
                    // Only the confirmed path belongs to this acquisition.
                    let (confirmed, unconfirmed): (Vec<_>, Vec<_>) = observation.evidence.drain(..)
                        .partition(|e| e.observed_path.as_ref()
                            .is_some_and(|p| p.as_os_str() == entry.path.as_os_str()));
                    observation.evidence = confirmed;
                    if !unconfirmed.is_empty() {
                        uncertain.push(super::process::Observation {
                            pid: observation.pid, creation_token: None, retained_handle: false,
                            details: ProcessDetails::default(), evidence: unconfirmed,
                        });
                    }
                    self.held.insert(observation.pid, Held {
                        process: opened.process, can_read_modules: opened.can_read_modules,
                    });
                    self.path_holders.insert(observation.pid);
                }
            }
            if observation.creation_token.is_none() {
                coverage.status = super::CoverageStatus::Partial;
                coverage.limitations.push(super::Limitation {
                    kind: LimitationKind::IdentityUncertain,
                    message: "a handle owner's process lifetime could not be associated reliably".to_string(),
                    count: 1, examples: vec![LimitationExample::pid(observation.pid)],
                });
            }
        }
        if !uncertain.is_empty() {
            coverage.status = super::CoverageStatus::Partial;
            coverage.limitations.push(super::Limitation {
                kind: LimitationKind::IdentityUncertain,
                message: "some earlier paths could not be associated with a newly opened holder's lifetime".to_string(),
                count: uncertain.len() as u64, examples: Vec::new(),
            });
            observations.extend(uncertain);
        }
        let mut limitations = Limitations::default();
        for limitation in coverage.limitations.drain(..) {
            limitations.merge(limitation);
        }
        coverage.limitations = limitations.into_vec();
        Some((observations, coverage))
    }

    /// Reads through the held handle, so it names the inspected lifetime:
    /// a PID cannot be reused while a handle to its process is open.
    fn details(&mut self, pid: u32) -> Result<ProcessDetails, String> {
        let held = self
            .held
            .get(&pid)
            .ok_or_else(|| "the process handle was not kept".to_string())?;
        self.source.details(&held.process)
    }

    fn creation_token(&mut self, pid: u32) -> Option<u64> {
        if let Some(held) = self.held.get(&pid) {
            return self.source.creation_time(&held.process).ok();
        }
        let opened = self.source.open(pid).ok()?;
        Some(opened.creation_time)
    }
}

impl<S: Source> ModuleBackend<S> {
    /// Lists modules, re-reading with the reported count while the list
    /// outgrows the buffer, so no loaded module is silently dropped.
    fn list_modules(&mut self, process: &S::Process) -> Result<Vec<Module>, Fault> {
        let mut capacity = DEFAULT_MODULE_CAPACITY;
        for _ in 0..MODULE_LIST_ATTEMPTS {
            let (mut modules, count) = self.source.modules(process, capacity)?;
            if count <= capacity {
                modules.truncate(count);
                return Ok(modules);
            }
            if count > MAX_MODULE_CAPACITY {
                return Err(Fault::Failed(format!(
                    "the process reported {count} modules, more than the {MAX_MODULE_CAPACITY} read"
                )));
            }
            capacity = count + MODULE_HEADROOM;
        }
        Err(Fault::Failed(format!(
            "the module list kept growing across {MODULE_LIST_ATTEMPTS} reads"
        )))
    }

    fn inspect_modules(&mut self, process: &S::Process, context: &InspectionContext<'_>, tally: &mut Tally) {
        let modules = match self.list_modules(process) {
            Ok(modules) => modules,
            Err(Fault::Failed(_)) if self.source.exited(process) => {
                tally.whole = Some(InspectionResult::Vanished);
                return;
            }
            Err(fault) => return whole(tally, fault),
        };
        let loader_paths = self.source.loader_paths(process);
        for module in modules {
            if context.budget.expired() {
                return tally.problem(LimitationKind::BudgetExhausted, BUDGET_EXPIRED, None);
            }
            performance::increment_counter(counters::QUERY_MODULE_INSPECTIONS, 1);
            let read = match loader_paths.as_ref().and_then(|paths| paths.get(&module)) {
                Some(path) => Ok(path.clone()),
                None => self.source.module_path(process, module),
            };
            let path = match read {
                Ok(path) => path,
                // Unloaded since the listing: no longer usage.
                Err(Fault::Gone) => continue,
                Err(Fault::Denied) => {
                    tally.problem(
                        LimitationKind::PermissionDenied,
                        "permission was denied for some modules",
                        Some(format!("module at {:#x}", module.0)),
                    );
                    continue;
                }
                Err(Fault::Failed(detail)) => {
                    if self.source.exited(process) {
                        tally.whole = Some(InspectionResult::Vanished);
                        return;
                    }
                    tally.problem(
                        LimitationKind::InspectionFailed,
                        "some module paths could not be read",
                        Some(format!("module at {:#x}: {detail}", module.0)),
                    );
                    continue;
                }
            };
            tally.successes += 1;
            let object = ObservedObject {
                identity: None,
                path: Some(path),
                deleted: false,
            };
            // Validated now, while the budget remains: the query makes no
            // lookup of its own, so a module kept unvalidated would be lost.
            let Some(found) = context.match_object(&object) else {
                if context.budget.exhausted() {
                    return tally.problem(LimitationKind::BudgetExhausted, BUDGET_EXPIRED, None);
                }
                continue;
            };
            tally.evidence.push(RawEvidence {
                kind: EvidenceKind::LoadedModule,
                object,
                descriptor: None,
                watch: None,
                access: None,
                event_only: None,
                matched: Some(found),
            });
        }
    }
}

/// A failure that prevented the whole mechanism for this process.
fn whole(tally: &mut Tally, fault: Fault) {
    tally.whole = Some(match fault {
        Fault::Denied => InspectionResult::Denied,
        Fault::Gone => InspectionResult::Vanished,
        Fault::Failed(detail) => InspectionResult::Failed(detail),
    });
}

#[cfg(windows)]
pub(crate) mod ffi {
    use super::{Fault, Module, Opened, ProcessDetails, Source};
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;
    use windows::Win32::Foundation::{
        CloseHandle, ERROR_ACCESS_DENIED, ERROR_INSUFFICIENT_BUFFER, ERROR_INVALID_PARAMETER,
        ERROR_PARTIAL_COPY, FILETIME, HANDLE, HMODULE, STILL_ACTIVE,
    };
    use windows::Win32::System::ProcessStatus::{
        EnumProcessModulesEx, EnumProcesses, GetModuleFileNameExW, LIST_MODULES_ALL,
    };
    use windows::Wdk::System::Threading::{NtQueryInformationProcess, ProcessBasicInformation};
    use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows::Win32::System::Threading::{PEB, PEB_LDR_DATA, PROCESS_BASIC_INFORMATION};
    use windows::Win32::System::WindowsProgramming::LDR_DATA_TABLE_ENTRY;
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_ACCESS_RIGHTS,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
        QueryFullProcessImageNameW,
    };
    use windows::core::PWSTR;

    /// Longest path a module or image name can have.
    const MAX_PATH_UNITS: usize = 32_768;

    pub(crate) struct Win32;


    /// `T` read whole from `address` in `process`.
    fn read<T: Copy + Default>(process: HANDLE, address: usize) -> Option<T> {
        let mut value = T::default();
        let mut read = 0usize;
        // SAFETY: `value` is a plain `repr(C)` struct of the size passed.
        unsafe {
            ReadProcessMemory(
                process,
                address as *const core::ffi::c_void,
                (&raw mut value).cast(),
                size_of::<T>(),
                Some(&mut read),
            )
        }
        .ok()?;
        (read == size_of::<T>()).then_some(value)
    }

    /// The `FullDllName` of every entry in `process`'s loader list, by
    /// `DllBase`: what `GetModuleFileNameExW` returns for that module, which
    /// walks the same list from its head for every single module (about
    /// 130 µs each on Windows; most of a usage query). Reads only the
    /// `winternl.h` fields (`PEB.Ldr`, `PEB_LDR_DATA.InMemoryOrderModuleList`,
    /// `LDR_DATA_TABLE_ENTRY`'s `InMemoryOrderLinks`, `DllBase`, and
    /// `FullDllName`). `None` on any failed read: a process still starting,
    /// one whose list changes during the walk, or one this build cannot read.
    /// A 32-bit (WOW64) process's own modules are not in this list; the
    /// caller reads those one at a time.
    #[cfg(target_pointer_width = "64")]
    fn loader_paths(process: HANDLE) -> Option<std::collections::HashMap<Module, PathBuf>> {
        let mut basic = PROCESS_BASIC_INFORMATION::default();
        let mut length = 0u32;
        // SAFETY: `basic` is the documented output of this class, of the size passed.
        let status = unsafe {
            NtQueryInformationProcess(
                process,
                ProcessBasicInformation,
                (&raw mut basic).cast(),
                size_of::<PROCESS_BASIC_INFORMATION>() as u32,
                &mut length,
            )
        };
        if status.is_err() || basic.PebBaseAddress.is_null() {
            return None;
        }
        let peb: PEB = read(process, basic.PebBaseAddress as usize)?;
        if peb.Ldr.is_null() {
            return None;
        }
        let loader: PEB_LDR_DATA = read(process, peb.Ldr as usize)?;
        let head = peb.Ldr as usize + std::mem::offset_of!(PEB_LDR_DATA, InMemoryOrderModuleList);
        let mut link = loader.InMemoryOrderModuleList.Flink as usize;
        let mut paths = std::collections::HashMap::new();
        while link != head {
            // A list that does not return to its head is changing under us.
            if paths.len() > super::MAX_MODULE_CAPACITY {
                return None;
            }
            let entry: LDR_DATA_TABLE_ENTRY =
                read(process, link.checked_sub(std::mem::offset_of!(LDR_DATA_TABLE_ENTRY, InMemoryOrderLinks))?)?;
            let name = entry.FullDllName;
            let units = usize::from(name.Length) / 2;
            if !name.Buffer.is_null() && units > 0 {
                let mut buffer = vec![0u16; units];
                let mut read = 0usize;
                // SAFETY: `buffer` holds `units` UTF-16 units, the size passed.
                unsafe {
                    ReadProcessMemory(
                        process,
                        name.Buffer.0 as *const core::ffi::c_void,
                        buffer.as_mut_ptr().cast(),
                        units * 2,
                        Some(&mut read),
                    )
                }
                .ok()?;
                if read != units * 2 {
                    return None;
                }
                paths.insert(Module(entry.DllBase as usize), PathBuf::from(OsString::from_wide(&buffer)));
            }
            link = entry.InMemoryOrderLinks.Flink as usize;
        }
        Some(paths)
    }

    #[cfg(not(target_pointer_width = "64"))]
    fn loader_paths(_process: HANDLE) -> Option<std::collections::HashMap<Module, PathBuf>> {
        None
    }

    /// Closes the handle when the backend drops it.
    pub(crate) struct ProcessHandle(HANDLE);

    impl Drop for ProcessHandle {
        fn drop(&mut self) {
            // SAFETY: the handle came from `OpenProcess` and is closed once.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    fn win32_code(error: &windows::core::Error) -> Option<u32> {
        super::super::identity::io_error(error.clone())
            .raw_os_error()
            .and_then(|code| u32::try_from(code).ok())
    }

    fn fault(error: windows::core::Error) -> Fault {
        match win32_code(&error) {
            Some(code) if code == ERROR_ACCESS_DENIED.0 => Fault::Denied,
            // `OpenProcess` reports a PID with no process as an invalid
            // parameter.
            Some(code) if code == ERROR_INVALID_PARAMETER.0 => Fault::Gone,
            Some(code) if code == ERROR_PARTIAL_COPY.0 => Fault::Failed(format!(
                "{} (the process is starting, protected, or of an architecture this build cannot read)",
                error.message()
            )),
            _ => Fault::Failed(error.message()),
        }
    }

    fn open_with(pid: u32, access: PROCESS_ACCESS_RIGHTS) -> Result<ProcessHandle, Fault> {
        // SAFETY: plain call; the returned handle is owned by `ProcessHandle`.
        unsafe { OpenProcess(access, false, pid) }
            .map(ProcessHandle)
            .map_err(fault)
    }

    fn creation_time(process: HANDLE) -> Result<u64, Fault> {
        let (mut creation, mut exit, mut kernel, mut user) =
            (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
        // SAFETY: four valid out-pointers.
        unsafe { GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) }
            .map_err(fault)?;
        Ok(u64::from(creation.dwHighDateTime) << 32 | u64::from(creation.dwLowDateTime))
    }

    fn image(process: HANDLE) -> Option<PathBuf> {
        let mut capacity = 512usize;
        loop {
            let mut buffer = vec![0u16; capacity];
            let mut size = capacity as u32;
            // SAFETY: `size` is the buffer's length in UTF-16 units.
            match unsafe {
                QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut size)
            } {
                Ok(()) => {
                    buffer.truncate(size as usize);
                    return Some(PathBuf::from(OsString::from_wide(&buffer)));
                }
                Err(error)
                    if win32_code(&error) == Some(ERROR_INSUFFICIENT_BUFFER.0)
                        && capacity < MAX_PATH_UNITS =>
                {
                    capacity = (capacity * 4).min(MAX_PATH_UNITS);
                }
                Err(_) => return None,
            }
        }
    }

    impl Source for Win32 {
        type Process = ProcessHandle;

        fn path_reader(&self) -> Option<super::super::win32_paths::PathReader> {
            Some(std::sync::Arc::new(super::super::win32_paths::ffi::holders))
        }

        fn pids(&mut self) -> Result<Vec<u32>, String> {
            let mut capacity = 1024usize;
            loop {
                let mut pids = vec![0u32; capacity];
                let bytes = u32::try_from(capacity * size_of::<u32>())
                    .map_err(|_| "the process list is too large to read".to_string())?;
                let mut written = 0u32;
                // SAFETY: the buffer is `bytes` writable bytes.
                unsafe { EnumProcesses(pids.as_mut_ptr(), bytes, &mut written) }
                    .map_err(|error| format!("the process list could not be read: {}", error.message()))?;
                let count = written as usize / size_of::<u32>();
                // A full buffer gives no sign whether more processes exist.
                if count < capacity {
                    pids.truncate(count);
                    return Ok(pids);
                }
                capacity *= 2;
            }
        }

        fn open(&mut self, pid: u32) -> Result<Opened<ProcessHandle>, Fault> {
            let (process, can_read_modules) =
                match open_with(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ) {
                    Ok(process) => (process, true),
                    // Identity may still be readable without memory access.
                    Err(Fault::Denied) => (open_with(pid, PROCESS_QUERY_LIMITED_INFORMATION)?, false),
                    Err(fault) => return Err(fault),
                };
            let creation_time = creation_time(process.0)?;
            Ok(Opened {
                image: image(process.0),
                process,
                creation_time,
                can_read_modules,
            })
        }

        fn creation_time(&mut self, process: &ProcessHandle) -> Result<u64, Fault> {
            creation_time(process.0)
        }

        fn exited(&mut self, process: &ProcessHandle) -> bool {
            let mut code = 0u32;
            // SAFETY: one valid out-pointer.
            unsafe { GetExitCodeProcess(process.0, &mut code) }
                .is_ok_and(|()| code != STILL_ACTIVE.0 as u32)
        }

        fn modules(&mut self, process: &ProcessHandle, capacity: usize) -> Result<(Vec<Module>, usize), Fault> {
            let mut modules = vec![HMODULE::default(); capacity];
            let bytes = u32::try_from(capacity * size_of::<HMODULE>())
                .map_err(|_| Fault::Failed("the module buffer is too large".to_string()))?;
            let mut needed = 0u32;
            // SAFETY: the buffer is `bytes` writable bytes.
            unsafe { EnumProcessModulesEx(process.0, modules.as_mut_ptr(), bytes, &mut needed, LIST_MODULES_ALL) }
                .map_err(fault)?;
            let count = needed as usize / size_of::<HMODULE>();
            modules.truncate(count.min(capacity));
            Ok((modules.into_iter().map(|module| Module(module.0 as usize)).collect(), count))
        }

        fn details(&mut self, process: &ProcessHandle) -> Result<ProcessDetails, String> {
            let user = crate::os::process_user_sid(process.0).map_err(|error| error.to_string())?;
            let executable = image(process.0);
            Ok(ProcessDetails {
                name: executable.as_deref().and_then(|e| e.file_name()).map(ToOwned::to_owned),
                executable,
                user: Some(user),
                start_time: creation_time(process.0).ok().and_then(super::filetime_to_utc),
            })
        }

        fn loader_paths(&mut self, process: &ProcessHandle) -> Option<std::collections::HashMap<Module, PathBuf>> {
            loader_paths(process.0)
        }

        fn module_path(&mut self, process: &ProcessHandle, module: Module) -> Result<PathBuf, Fault> {
            let mut capacity = 512usize;
            loop {
                let mut buffer = vec![0u16; capacity];
                // SAFETY: the slice carries its own length.
                let written = unsafe {
                    GetModuleFileNameExW(Some(process.0), Some(HMODULE(module.0 as *mut _)), &mut buffer)
                } as usize;
                if written == 0 {
                    return Err(fault(windows::core::Error::from_thread()));
                }
                // A result that fills the buffer may have been cut short.
                if written < capacity {
                    buffer.truncate(written);
                    return Ok(PathBuf::from(OsString::from_wide(&buffer)));
                }
                if capacity >= MAX_PATH_UNITS {
                    return Err(Fault::Failed("the module path is longer than any Windows path".to_string()));
                }
                capacity = (capacity * 4).min(MAX_PATH_UNITS);
            }
        }
    }
}
