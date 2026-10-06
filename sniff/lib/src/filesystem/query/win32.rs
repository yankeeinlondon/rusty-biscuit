//! Native Windows backend: the modules each process has loaded, with process
//! identity read through a process handle the backend keeps open.
//!
//! The classification logic is generic over [`Source`], so tests can script
//! denials, exits, and undersized buffers on any host; the FFI source
//! ([`ffi::Win32`]) exists on Windows only. A module is observed by path
//! only, so the query looks its path up to decide whether it is in scope.
//!
//! Other processes' file handles are not inspected: reading the identity of
//! a duplicated handle can block for as long as its owner has synchronous
//! I/O pending on it, and no call cancels that wait. A working directory and
//! a `ReadDirectoryChangesW` subscription cannot be told apart from other
//! directory handles at all.

use super::backend::{
    Enumeration, InspectionContext, InspectionResult, MechanismInspection, MechanismSupport,
    ProcessCandidate, ProcessDetails, RawEvidence, Tally, UsageBackend,
};
use super::budget::Budget;
use super::matching::{self, ObservedObject};
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

    /// Every PID on the host. `Err` names why the list could not be read.
    fn pids(&mut self) -> Result<Vec<u32>, String>;
    fn open(&mut self, pid: u32) -> Result<Opened<Self::Process>, Fault>;
    fn creation_time(&mut self, process: &Self::Process) -> Result<u64, Fault>;
    fn exited(&mut self, process: &Self::Process) -> bool;
    /// At most `capacity` modules, and how many the process had loaded when
    /// the list was read.
    fn modules(&mut self, process: &Self::Process, capacity: usize) -> Result<(Vec<Module>, usize), Fault>;
    fn module_path(&mut self, process: &Self::Process, module: Module) -> Result<PathBuf, Fault>;
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
    /// Handles for processes whose inspection has not yet shown they hold
    /// nothing in scope. The rest stay open until the query returns, so
    /// enrichment reads the same lifetime.
    held: HashMap<u32, Held<S::Process>>,
}

impl<S: Source> ModuleBackend<S> {
    pub(crate) fn with_source(source: S) -> Self {
        Self {
            source,
            held: HashMap::new(),
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
            MechanismSupport::unsupported(OpenHandles, OPEN_HANDLES_UNSUPPORTED),
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
                if !modules.evidence.is_empty() {
                    self.held.insert(pid, held);
                }
            }
            Some(_) => {
                self.held.remove(&pid);
                modules.whole = Some(InspectionResult::Denied);
            }
            None => modules.whole = Some(InspectionResult::Failed("the process handle was not kept".to_string())),
        }
        vec![modules.finish()]
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
        let spellings = context.root.spellings();
        for module in modules {
            if context.budget.expired() {
                return tally.problem(
                    LimitationKind::BudgetExhausted,
                    "the budget expired while a process's modules were being read",
                    None,
                );
            }
            performance::increment_counter(counters::QUERY_MODULE_INSPECTIONS, 1);
            let path = match self.source.module_path(process, module) {
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
            // A text pre-filter only: the query's identity lookup decides, so
            // each module is looked up at most once.
            if !spellings.iter().any(|root| matching::may_contain(root, &path)) {
                continue;
            }
            tally.evidence.push(RawEvidence {
                kind: EvidenceKind::LoadedModule,
                object: ObservedObject {
                    identity: None,
                    path: Some(path),
                    deleted: false,
                },
                descriptor: None,
                watch: None,
                access: None,
                event_only: None,
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
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_ACCESS_RIGHTS,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
        QueryFullProcessImageNameW,
    };
    use windows::core::PWSTR;

    /// Longest path a module or image name can have.
    const MAX_PATH_UNITS: usize = 32_768;

    pub(crate) struct Win32;

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
