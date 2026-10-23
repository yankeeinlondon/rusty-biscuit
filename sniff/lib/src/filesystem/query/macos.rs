//! macOS backend: open vnode descriptors and working directories read through
//! `libproc` (`proc_pidinfo`/`proc_pidfdinfo`).
//!
//! The classification logic is generic over [`Source`], so tests can script
//! denials, races, and undersized buffers; the FFI source ([`ffi::Libproc`])
//! exists on macOS only. Identity is the vnode's `(dev, ino)`; the path the
//! kernel reports is descriptive only.
//!
//! `proc_listpidspath` is not used as a candidate source: it matches a single
//! vnode, never a tree, so a full descriptor and cwd scan is needed anyway.
//! FSEvents subscriptions have no supported systemwide inventory.

use super::backend::{
    Enumeration, InspectionContext, InspectionResult, LinkError, MechanismInspection,
    MechanismSupport, ProcessCandidate, ProcessDetails, ProcessLink, RawEvidence, Tally,
    UsageBackend,
};
use super::budget::Budget;
use super::identity::FileIdentity;
use super::matching::ObservedObject;
use super::report::{
    Access, EvidenceKind, LimitationExample, LimitationKind, Limitations, Mechanism,
};
use crate::performance::{self, counters};
use std::collections::HashMap;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

pub(crate) const FSEVENTS_UNSUPPORTED: &str =
    "macOS offers no supported systemwide inventory of FSEvents subscriptions";

/// Kernel open-file flags (`fi_openflags`), not `O_*` open flags, except
/// that `O_EVTONLY` shares its value with the kernel flag.
const FREAD: u32 = 0x1;
const FWRITE: u32 = 0x2;
const O_EVTONLY: u32 = 0x8000;

/// Descriptor-list capacity when the table size is unknown.
const DEFAULT_DESCRIPTOR_CAPACITY: usize = 256;
/// Added to `pbi_nfiles`, which can grow between the two calls.
const DESCRIPTOR_HEADROOM: usize = 16;
/// Above every `kern.maxfilesperproc` default; a list this full is refused
/// rather than grown without bound.
const MAX_DESCRIPTOR_CAPACITY: usize = 1 << 20;

/// Why a `libproc` call returned nothing usable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Fault {
    /// `EPERM`/`EACCES`: typically another user's process.
    Denied,
    /// `ESRCH`, `EBADF`, `ENOENT`: the process exited or the descriptor
    /// closed.
    Gone,
    Failed(String),
}

/// One process's BSD info, read in one call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Task {
    /// Start time in microseconds since the epoch: this lifetime's creation
    /// token.
    pub(crate) start_micros: u64,
    pub(crate) name: OsString,
    /// Descriptor-table capacity (`pbi_nfiles`), not the open count.
    pub(crate) table_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Descriptor {
    pub(crate) fd: i32,
    /// `PROX_FDTYPE_VNODE`; sockets, pipes, and kqueues carry no path.
    pub(crate) vnode: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Vnode {
    /// In the encoding `std::os::unix::fs::MetadataExt::dev` reports.
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) links: u16,
    /// The kernel's last-known path; empty when it has none.
    pub(crate) path: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenVnode {
    /// `fi_openflags`.
    pub(crate) flags: u32,
    pub(crate) vnode: Vnode,
}

/// One process's parent and names, read with calls that need no same-user
/// permission, so a parent owned by root (such as `login`) is still read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawLink {
    /// Start time in microseconds since the epoch, as in [`Task::start_micros`].
    pub(crate) start_micros: u64,
    pub(crate) parent_pid: u32,
    /// The kernel's short name, at most 16 bytes.
    pub(crate) comm: OsString,
    pub(crate) executable: Option<PathBuf>,
}

/// The name keys of a bundle's `Info.plist`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BundleInfo {
    pub(crate) name: Option<String>,
    pub(crate) display_name: Option<String>,
}

/// The nearest `*.app` directory enclosing `executable`.
pub(crate) fn enclosing_bundle(executable: &Path) -> Option<&Path> {
    executable
        .ancestors()
        .skip(1)
        .find(|dir| dir.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("app")))
}

/// `CFBundleName`, else `CFBundleDisplayName`, else the bundle directory's
/// stem: Finder's name for an app whose `Info.plist` names none.
pub(crate) fn bundle_name(bundle: &Path, info: Option<&BundleInfo>) -> Option<OsString> {
    info.and_then(|info| {
        [&info.name, &info.display_name]
            .into_iter()
            .flatten()
            .find(|name| !name.trim().is_empty())
    })
    .map(OsString::from)
    .or_else(|| bundle.file_stem().map(ToOwned::to_owned))
}

/// The `libproc` calls the backend makes.
pub(crate) trait Source {
    /// Every PID on the host. `Err` names why the list could not be read.
    fn pids(&mut self) -> Result<Vec<u32>, String>;
    fn task(&mut self, pid: u32) -> Result<Task, Fault>;
    /// At most `capacity` descriptors; a full list may have been cut short.
    fn descriptors(&mut self, pid: u32, capacity: usize) -> Result<Vec<Descriptor>, Fault>;
    fn descriptor_vnode(&mut self, pid: u32, fd: i32) -> Result<OpenVnode, Fault>;
    /// `None` when the process has no current directory (the kernel task).
    fn cwd(&mut self, pid: u32) -> Result<Option<Vnode>, Fault>;
    /// Every field from one lifetime: a start time that changed during the
    /// read is [`LinkError::Gone`].
    fn link(&mut self, _pid: u32) -> Result<RawLink, LinkError> {
        Err(LinkError::Unsupported)
    }
    /// `None` when the bundle has no readable `Info.plist`.
    fn bundle_info(&mut self, _bundle: &Path) -> Option<BundleInfo> {
        None
    }
}

pub(crate) struct LibprocBackend<S> {
    source: S,
    /// App names by bundle, read once per query: many processes share one.
    bundles: HashMap<PathBuf, Option<OsString>>,
}

impl<S: Source> LibprocBackend<S> {
    pub(crate) fn with_source(source: S) -> Self {
        Self {
            source,
            bundles: HashMap::new(),
        }
    }

    fn app_name(&mut self, executable: &Path) -> Option<OsString> {
        let bundle = enclosing_bundle(executable)?;
        if let Some(name) = self.bundles.get(bundle) {
            return name.clone();
        }
        let info = self.source.bundle_info(bundle);
        let name = bundle_name(bundle, info.as_ref());
        self.bundles.insert(bundle.to_path_buf(), name.clone());
        name
    }
}

#[cfg(target_os = "macos")]
impl LibprocBackend<ffi::Libproc> {
    pub(crate) fn new() -> Self {
        Self::with_source(ffi::Libproc)
    }
}

impl<S: Source> UsageBackend for LibprocBackend<S> {
    /// The descriptor-table capacity, when the task info was readable.
    type Payload = Option<u32>;

    fn mechanisms(&self) -> Vec<MechanismSupport> {
        vec![
            MechanismSupport::supported(Mechanism::OpenHandles),
            MechanismSupport::supported(Mechanism::WorkingDirectories),
            MechanismSupport::unsupported(Mechanism::Fsevents, FSEVENTS_UNSUPPORTED),
            MechanismSupport::unsupported(Mechanism::Polling, super::backend::POLLING_UNSUPPORTED),
        ]
    }

    fn enumerate(&mut self, budget: &Budget) -> Result<Enumeration<Option<u32>>, String> {
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
            let (creation_token, name, table_size) = match self.source.task(pid) {
                Ok(task) => (Some(task.start_micros), Some(task.name), Some(task.table_size)),
                // Exited between the listing and the read.
                Err(Fault::Gone) => continue,
                // The same check denies its descriptors and cwd, so it can
                // contribute no evidence; inspection reports the denial.
                Err(Fault::Denied) => (None, None, None),
                Err(Fault::Failed(detail)) => {
                    limitations.record(
                        LimitationKind::IdentityUncertain,
                        "the start time of some processes could not be read",
                        1,
                        Some(LimitationExample::pid(pid).with_detail(detail)),
                    );
                    (None, None, None)
                }
            };
            processes.push(ProcessCandidate {
                pid,
                creation_token,
                retained_handle: false,
                details: ProcessDetails {
                    name,
                    ..ProcessDetails::default()
                },
                payload: table_size,
            });
        }
        Ok(Enumeration {
            processes,
            complete,
            limitations: limitations.into_vec(),
        })
    }

    fn inspect(
        &mut self,
        candidate: &ProcessCandidate<Option<u32>>,
        context: &InspectionContext<'_>,
    ) -> Vec<MechanismInspection> {
        let pid = candidate.pid;
        let mut handles = Tally::new(Mechanism::OpenHandles, pid);
        let mut cwd = Tally::new(Mechanism::WorkingDirectories, pid);
        self.inspect_descriptors(candidate, context, &mut handles);
        let cwd_started = !context.budget.expired();
        if cwd_started {
            self.inspect_cwd(pid, context, &mut cwd);
        }

        // A changed or missing start time means the PID was reused or the
        // process exited during inspection, so nothing read can be attributed.
        // The check runs even after the budget expired: it is what makes the
        // evidence already read attributable.
        if candidate.creation_token.is_some() && self.creation_token(pid) != candidate.creation_token
        {
            return [handles, cwd].into_iter().map(Tally::vanished).collect();
        }
        let cwd = if cwd_started { cwd.finish() } else { cwd.not_attempted() };
        vec![handles.finish(), cwd]
    }

    fn creation_token(&mut self, pid: u32) -> Option<u64> {
        self.source.task(pid).ok().map(|task| task.start_micros)
    }

    fn link(&mut self, pid: u32) -> Result<ProcessLink, LinkError> {
        let raw = self.source.link(pid)?;
        let app_name = raw.executable.as_deref().and_then(|exe| self.app_name(exe));
        // The executable's file name is not truncated to 16 bytes.
        let name = raw
            .executable
            .as_deref()
            .and_then(Path::file_name)
            .map(ToOwned::to_owned)
            .or_else(|| (!raw.comm.is_empty()).then_some(raw.comm));
        Ok(ProcessLink {
            order: raw.start_micros,
            parent_pid: raw.parent_pid,
            name,
            app_name,
        })
    }
}

impl<S: Source> LibprocBackend<S> {
    /// Lists descriptors, growing the buffer while a call fills it: a full
    /// buffer gives no sign whether more descriptors exist.
    fn list_descriptors(&mut self, pid: u32, table_size: Option<u32>) -> Result<Vec<Descriptor>, Fault> {
        let mut capacity = table_size
            .map_or(DEFAULT_DESCRIPTOR_CAPACITY, |n| n as usize + DESCRIPTOR_HEADROOM)
            .min(MAX_DESCRIPTOR_CAPACITY);
        loop {
            let descriptors = self.source.descriptors(pid, capacity)?;
            if descriptors.len() < capacity {
                return Ok(descriptors);
            }
            if capacity >= MAX_DESCRIPTOR_CAPACITY {
                return Err(Fault::Failed(format!(
                    "the descriptor list did not fit in {MAX_DESCRIPTOR_CAPACITY} entries"
                )));
            }
            capacity = (capacity * 2).min(MAX_DESCRIPTOR_CAPACITY);
        }
    }

    fn inspect_descriptors(
        &mut self,
        candidate: &ProcessCandidate<Option<u32>>,
        context: &InspectionContext<'_>,
        handles: &mut Tally,
    ) {
        let pid = candidate.pid;
        let descriptors = match self.list_descriptors(pid, candidate.payload) {
            Ok(descriptors) => descriptors,
            Err(fault) => return whole(handles, fault),
        };
        for descriptor in descriptors {
            if context.budget.expired() {
                return handles.problem(
                    LimitationKind::BudgetExhausted,
                    "the budget expired while a process's descriptors were being read",
                    None,
                );
            }
            performance::increment_counter(counters::QUERY_DESCRIPTOR_INSPECTIONS, 1);
            if !descriptor.vnode {
                handles.successes += 1;
                continue;
            }
            let fd = descriptor.fd;
            let open = match self.source.descriptor_vnode(pid, fd) {
                Ok(open) => open,
                // Closed since the listing: no longer usage.
                Err(Fault::Gone) => continue,
                Err(Fault::Denied) => {
                    handles.problem(
                        LimitationKind::PermissionDenied,
                        "permission was denied for some descriptors",
                        Some(format!("fd {fd}")),
                    );
                    continue;
                }
                Err(Fault::Failed(detail)) => {
                    handles.problem(
                        LimitationKind::InspectionFailed,
                        "some descriptors could not be read",
                        Some(format!("fd {fd}: {detail}")),
                    );
                    continue;
                }
            };
            handles.successes += 1;
            let object = observed(open.vnode);
            if context.match_object(&object).is_none() {
                continue;
            }
            handles.evidence.push(RawEvidence {
                kind: EvidenceKind::OpenHandle,
                object,
                descriptor: u64::try_from(fd).ok(),
                watch: None,
                access: Some(Access {
                    read: open.flags & FREAD != 0,
                    write: open.flags & FWRITE != 0,
                }),
                event_only: Some(open.flags & O_EVTONLY != 0),
                matched: None,
            });
        }
    }

    fn inspect_cwd(&mut self, pid: u32, context: &InspectionContext<'_>, cwd: &mut Tally) {
        let vnode = match self.source.cwd(pid) {
            Ok(Some(vnode)) => vnode,
            Ok(None) => {
                cwd.successes += 1;
                return;
            }
            // An exiting process has no working directory; the start time
            // check after inspection tells an exit apart.
            Err(Fault::Gone) => {
                cwd.successes += 1;
                return;
            }
            Err(fault) => return whole(cwd, fault),
        };
        cwd.successes += 1;
        let object = observed(vnode);
        if context.match_object(&object).is_some() {
            cwd.evidence.push(RawEvidence {
                kind: EvidenceKind::WorkingDirectory,
                object,
                descriptor: None,
                watch: None,
                access: None,
                event_only: None,
                matched: None,
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

/// The kernel's path is the observed spelling; an object with no remaining
/// links has been unlinked, so its path is a former name.
fn observed(vnode: Vnode) -> ObservedObject {
    ObservedObject {
        identity: Some(FileIdentity::Unix {
            device: vnode.device,
            inode: vnode.inode,
        }),
        deleted: vnode.links == 0,
        path: (!vnode.path.is_empty()).then(|| PathBuf::from(OsString::from_vec(vnode.path))),
    }
}

#[cfg(target_os = "macos")]
pub(crate) mod ffi {
    use super::{BundleInfo, Descriptor, Fault, OpenVnode, RawLink, Source, Task, Vnode};
    use crate::filesystem::query::backend::LinkError;
    use std::ffi::{OsString, c_char, c_int, c_void};
    use std::io;
    use std::mem::{MaybeUninit, size_of};
    use std::os::unix::ffi::OsStringExt;
    use std::path::{Path, PathBuf};

    /// `sys/proc_info.h` items the `libc` crate does not define.
    const PROC_PIDFDVNODEPATHINFO: c_int = 2;
    const PROX_FDTYPE_VNODE: u32 = 1;
    const PROC_PIDT_SHORTBSDINFO: c_int = 13;
    const MAXCOMLEN: usize = 16;

    /// `struct proc_bsdshortinfo`. Unlike `proc_bsdinfo`, the kernel serves
    /// it for another user's process.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ProcBsdShortInfo {
        pbsi_pid: u32,
        pbsi_ppid: u32,
        pbsi_pgid: u32,
        pbsi_status: u32,
        pbsi_comm: [c_char; MAXCOMLEN],
        pbsi_flags: u32,
        pbsi_uid: u32,
        pbsi_gid: u32,
        pbsi_ruid: u32,
        pbsi_rgid: u32,
        pbsi_svuid: u32,
        pbsi_svgid: u32,
        pbsi_rfu: u32,
    }

    /// Start time in microseconds from `sysctl` `kern.proc.pid.<pid>`, which,
    /// unlike `proc_bsdinfo`, needs no same-user permission. Only the
    /// leading `p_starttime` timeval of `struct kinfo_proc` is read, so the
    /// rest of that large structure is never declared here.
    fn start_micros(pid: u32) -> Result<u64, Fault> {
        let mut name = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_PID, pid_arg(pid)?];
        // `u64` storage keeps the leading timeval aligned.
        let mut buffer = [0u64; 128];
        let mut length = size_of::<[u64; 128]>();
        // SAFETY: `buffer` is `length` writable bytes; `name` has 4 entries.
        let status = unsafe {
            libc::sysctl(name.as_mut_ptr(), 4, buffer.as_mut_ptr().cast(), &mut length, std::ptr::null_mut(), 0)
        };
        if status != 0 {
            return Err(Fault::Failed(io::Error::last_os_error().to_string()));
        }
        // A PID with no process returns success and no data.
        if length < 16 {
            return Err(Fault::Gone);
        }
        let seconds = buffer[0];
        let micros = buffer[1] & 0xFFFF_FFFF;
        Ok(seconds.saturating_mul(1_000_000).saturating_add(micros))
    }

    fn executable(pid: u32) -> Option<PathBuf> {
        let mut buffer = vec![0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
        // SAFETY: the buffer is `PROC_PIDPATHINFO_MAXSIZE` writable bytes.
        let written = unsafe {
            libc::proc_pidpath(pid_arg(pid).ok()?, buffer.as_mut_ptr().cast(), buffer.len() as u32)
        };
        if written <= 0 {
            return None;
        }
        buffer.truncate(written as usize);
        Some(PathBuf::from(OsString::from_vec(buffer)))
    }

    fn link_error(fault: Fault) -> LinkError {
        match fault {
            Fault::Gone => LinkError::Gone,
            Fault::Denied => LinkError::Unreadable("permission was denied".to_string()),
            Fault::Failed(detail) => LinkError::Unreadable(detail),
        }
    }

    fn info_string(dictionary: &plist::Dictionary, key: &str) -> Option<String> {
        dictionary.get(key)?.as_string().map(ToOwned::to_owned)
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ProcFileInfo {
        fi_openflags: u32,
        fi_status: u32,
        fi_offset: libc::off_t,
        fi_type: i32,
        fi_guardflags: u32,
    }

    /// `struct vnode_fdinfowithpath`.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct VnodeFdInfoWithPath {
        pfi: ProcFileInfo,
        pvip: libc::vnode_info_path,
    }

    pub(crate) struct Libproc;

    fn clear_errno() {
        // SAFETY: `__error` returns this thread's errno slot.
        unsafe { *libc::__error() = 0 };
    }

    /// `libproc` returns 0 (not -1) on failure and sets `errno`, which
    /// [`clear_errno`] reset before the call.
    fn fault() -> Fault {
        let error = io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::EPERM | libc::EACCES) => Fault::Denied,
            Some(libc::ESRCH | libc::EBADF | libc::ENOENT) => Fault::Gone,
            Some(0) | None => Fault::Failed("libproc returned no data".to_string()),
            Some(_) => Fault::Failed(error.to_string()),
        }
    }

    fn pid_arg(pid: u32) -> Result<c_int, Fault> {
        c_int::try_from(pid).map_err(|_| Fault::Gone)
    }

    /// Calls a fixed-size `proc_pidinfo` flavor.
    fn pidinfo<T: Copy>(pid: u32, flavor: c_int) -> Result<T, Fault> {
        let pid = pid_arg(pid)?;
        let size = size_of::<T>() as c_int;
        let mut value = MaybeUninit::<T>::zeroed();
        clear_errno();
        // SAFETY: the buffer is `size` writable bytes.
        let written = unsafe { libc::proc_pidinfo(pid, flavor, 0, value.as_mut_ptr().cast(), size) };
        if written <= 0 {
            return Err(fault());
        }
        if written != size {
            return Err(Fault::Failed(format!("libproc wrote {written} of {size} bytes")));
        }
        // SAFETY: the kernel filled all `size` bytes of a plain-data struct.
        Ok(unsafe { value.assume_init() })
    }

    fn c_bytes(chars: &[c_char]) -> Vec<u8> {
        chars
            .iter()
            .map(|&c| c as u8)
            .take_while(|&b| b != 0)
            .collect()
    }

    fn vnode(info: &libc::vnode_info_path) -> Vnode {
        let stat = &info.vip_vi.vi_stat;
        Vnode {
            // `std` widens the signed `st_dev` with sign extension; `vst_dev`
            // is the same 32 bits declared unsigned.
            device: stat.vst_dev as i32 as u64,
            inode: stat.vst_ino,
            links: stat.vst_nlink,
            path: c_bytes(info.vip_path.as_flattened()),
        }
    }

    impl Source for Libproc {
        fn pids(&mut self) -> Result<Vec<u32>, String> {
            // SAFETY: a null buffer asks for the current count.
            let estimate = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
            if estimate <= 0 {
                return Err(format!("the process list could not be read: {}", io::Error::last_os_error()));
            }
            let mut capacity = estimate as usize + 64;
            loop {
                let mut pids: Vec<libc::pid_t> = vec![0; capacity];
                let bytes = c_int::try_from(capacity * size_of::<libc::pid_t>())
                    .map_err(|_| "the process list is too large to read".to_string())?;
                // SAFETY: the buffer is `bytes` writable bytes.
                let count = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast::<c_void>(), bytes) };
                if count < 0 {
                    return Err(format!("the process list could not be read: {}", io::Error::last_os_error()));
                }
                // A full buffer may have cut the list short.
                if (count as usize) < capacity {
                    pids.truncate(count as usize);
                    return Ok(pids.into_iter().filter_map(|pid| u32::try_from(pid).ok()).collect());
                }
                capacity *= 2;
            }
        }

        fn task(&mut self, pid: u32) -> Result<Task, Fault> {
            let info: libc::proc_bsdinfo = pidinfo(pid, libc::PROC_PIDTBSDINFO)?;
            let mut name = c_bytes(&info.pbi_name);
            if name.is_empty() {
                name = c_bytes(&info.pbi_comm);
            }
            Ok(Task {
                start_micros: info
                    .pbi_start_tvsec
                    .saturating_mul(1_000_000)
                    .saturating_add(info.pbi_start_tvusec),
                name: OsString::from_vec(name),
                table_size: info.pbi_nfiles,
            })
        }

        fn descriptors(&mut self, pid: u32, capacity: usize) -> Result<Vec<Descriptor>, Fault> {
            let pid = pid_arg(pid)?;
            let entry = size_of::<libc::proc_fdinfo>();
            let bytes = c_int::try_from(capacity * entry)
                .map_err(|_| Fault::Failed("the descriptor buffer is too large".to_string()))?;
            let mut buffer: Vec<libc::proc_fdinfo> = Vec::with_capacity(capacity);
            clear_errno();
            // SAFETY: the buffer has room for `capacity` entries (`bytes` bytes).
            let written = unsafe {
                libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, buffer.as_mut_ptr().cast(), bytes)
            };
            if written < 0 || (written == 0 && io::Error::last_os_error().raw_os_error() != Some(0)) {
                return Err(fault());
            }
            let count = (written as usize / entry).min(capacity);
            // SAFETY: the kernel initialized `count` whole entries.
            unsafe { buffer.set_len(count) };
            Ok(buffer
                .into_iter()
                .map(|info| Descriptor {
                    fd: info.proc_fd,
                    vnode: info.proc_fdtype == PROX_FDTYPE_VNODE,
                })
                .collect())
        }

        fn descriptor_vnode(&mut self, pid: u32, fd: i32) -> Result<OpenVnode, Fault> {
            let pid = pid_arg(pid)?;
            let size = size_of::<VnodeFdInfoWithPath>() as c_int;
            let mut info = MaybeUninit::<VnodeFdInfoWithPath>::zeroed();
            clear_errno();
            // SAFETY: the buffer is `size` writable bytes.
            let written = unsafe {
                libc::proc_pidfdinfo(pid, fd, PROC_PIDFDVNODEPATHINFO, info.as_mut_ptr().cast(), size)
            };
            if written <= 0 {
                return Err(fault());
            }
            if written != size {
                return Err(Fault::Failed(format!("libproc wrote {written} of {size} bytes")));
            }
            // SAFETY: the kernel filled the whole plain-data struct.
            let info = unsafe { info.assume_init() };
            Ok(OpenVnode {
                flags: info.pfi.fi_openflags,
                vnode: vnode(&info.pvip),
            })
        }

        fn cwd(&mut self, pid: u32) -> Result<Option<Vnode>, Fault> {
            let info: libc::proc_vnodepathinfo = pidinfo(pid, libc::PROC_PIDVNODEPATHINFO)?;
            let cwd = vnode(&info.pvi_cdir);
            Ok((cwd.inode != 0 || !cwd.path.is_empty()).then_some(cwd))
        }

        fn link(&mut self, pid: u32) -> Result<RawLink, LinkError> {
            let start = start_micros(pid).map_err(link_error)?;
            let info: ProcBsdShortInfo = pidinfo(pid, PROC_PIDT_SHORTBSDINFO).map_err(link_error)?;
            let executable = executable(pid);
            // An unchanged start time proves the PID named one lifetime
            // throughout.
            if start_micros(pid).map_err(link_error)? != start {
                return Err(LinkError::Gone);
            }
            Ok(RawLink {
                start_micros: start,
                parent_pid: info.pbsi_ppid,
                comm: OsString::from_vec(c_bytes(&info.pbsi_comm)),
                executable,
            })
        }

        fn bundle_info(&mut self, bundle: &Path) -> Option<BundleInfo> {
            // `plist` reads both the XML and the binary form.
            let value = plist::Value::from_file(bundle.join("Contents").join("Info.plist")).ok()?;
            let dictionary = value.as_dictionary()?;
            Some(BundleInfo {
                name: info_string(dictionary, "CFBundleName"),
                display_name: info_string(dictionary, "CFBundleDisplayName"),
            })
        }
    }
}
