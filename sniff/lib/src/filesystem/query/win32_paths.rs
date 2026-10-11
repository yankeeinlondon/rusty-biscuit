//! File-side handle discovery over the existing tree walk. Scoped workers
//! share the request budget and close their file handles before returning.
use super::backend::{InspectionContext, ProcessDetails};
use super::budget::Budget;
use super::identity::FileIdentity;
use super::process::Observation;
use super::report::{Coverage, CoverageScope, CoverageStatus, Evidence, EvidenceKind,
    LimitationExample, LimitationKind, Limitations, MatchBasis, Mechanism, Truncation};
use super::tree::ProbeEntry;
use crate::performance::{self, counters};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) type PathReader = Arc<dyn Fn(&ProbeEntry, &Budget) -> Result<(FileIdentity, Vec<u32>), PathFailure> + Send + Sync>;
type PathResult = Result<(FileIdentity, Vec<u32>), PathFailure>;

#[derive(Debug)]
pub(crate) struct PathFailure {
    kind: LimitationKind,
    detail: String,
}

impl From<String> for PathFailure {
    fn from(detail: String) -> Self {
        Self { kind: LimitationKind::InspectionFailed, detail }
    }
}

#[derive(Default)]
struct Results {
    attempted: u64,
    succeeded: u64,
    omitted: u64,
    total: u64,
    limitations: Limitations,
    processes: BTreeMap<u32, Observation>,
}

impl Results {
    fn record(&mut self, entry: &ProbeEntry, result: PathResult, context: &InspectionContext<'_>) {
        self.attempted += 1;
        let (identity, pids) = match result {
            Ok(result) => result,
            Err(failure) => {
                self.limitations.record(failure.kind,
                    "some paths could not be queried for handle owners", 1,
                    Some(LimitationExample::path(entry.path.as_path()).with_detail(failure.detail)));
                return;
            }
        };
        if identity != entry.identity {
            self.limitations.record(LimitationKind::DescriptorReplaced,
                "some paths were replaced after the tree walk", 1,
                Some(LimitationExample::path(entry.path.as_path())));
            return;
        }
        self.succeeded += 1;
        let mut matched_paths = context.index.paths(&identity).expect("indexed entry").to_vec();
        matched_paths.sort();
        for pid in pids {
            if self.total >= super::MAX_EVIDENCE_TOTAL {
                self.omitted += 1;
                continue;
            }
            let observation = self.processes.entry(pid).or_insert_with(|| Observation {
                pid, creation_token: None, retained_handle: false,
                details: ProcessDetails::default(), evidence: Vec::new(),
            });
            if observation.evidence.len() as u64 >= super::MAX_EVIDENCE_PER_PROCESS {
                self.omitted += 1;
                continue;
            }
            self.total += 1;
            observation.evidence.push(Evidence {
                kind: EvidenceKind::OpenHandle, mechanism: Mechanism::OpenHandles,
                matched_paths: matched_paths.iter().map(|p| p.as_path().into()).collect(),
                observed_path: Some(entry.path.as_path().into()),
                match_basis: MatchBasis::Identity, identity: Some(identity),
                descriptor: None, watch: None, access: None, event_only: None,
            });
        }
    }
}

fn ordered_entries<'a>(context: &InspectionContext<'a>) -> Vec<&'a ProbeEntry> {
    let mut entries: Vec<_> = context.index.entries.iter().collect();
    let mut ignores = Vec::new();
    for entry in &entries {
        if context.budget.expired() { break; }
        if entry.path.file_name().is_some_and(|name| name == ".gitignore") {
            let directory = entry.path.parent().expect("tree entry parent");
            let mut builder = ignore::gitignore::GitignoreBuilder::new(directory);
            if builder.add(&entry.path).is_none() && let Ok(matcher) = builder.build() {
                ignores.push((directory.to_path_buf(), matcher));
            }
        }
    }
    ignores.sort_by_key(|(path, _)| path.components().count());
    entries.sort_by_cached_key(|entry| {
        let mut ignored = false;
        for (directory, matcher) in &ignores {
            if entry.path.starts_with(directory) {
                let matched = matcher.matched_path_or_any_parents(&entry.path, entry.directory);
                if matched.is_ignore() { ignored = true; }
                if matched.is_whitelist() { ignored = false; }
            }
        }
        let generated = entry.path.strip_prefix(&context.root.resolved).unwrap_or(&entry.path)
            .components().any(|c| c.as_os_str() == "target" || c.as_os_str() == "node_modules");
        (ignored || generated, !entry.directory, entry.path.clone())
    });
    entries
}

pub(crate) fn scan(context: &InspectionContext<'_>, reader: &PathReader) -> (Vec<Observation>, Coverage) {
    let entries = ordered_entries(context);
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Results::default());
    let collector = performance::current_collector();
    let workers = std::thread::available_parallelism().map_or(1, usize::from).min(8).min(entries.len());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                let _collector = performance::pooled_worker(collector.as_ref());
                loop {
                    if context.budget.expired() { break; }
                    let position = next.fetch_add(1, Ordering::Relaxed);
                    let Some(entry) = entries.get(position) else { break };
                    performance::increment_counter(counters::QUERY_PATH_HOLDER_INSPECTIONS, 1);
                    let result = reader(entry, context.budget);
                    results.lock().expect("path results").record(entry, result, context);
                }
            });
        }
    });
    let Results { attempted, succeeded, omitted, mut limitations, processes, .. } =
        results.into_inner().expect("joined path workers");
    if attempted < entries.len() as u64 {
        limitations.record(LimitationKind::BudgetExhausted,
            "the budget expired before every path was queried", entries.len() as u64 - attempted, None);
    }
    if omitted > 0 {
        limitations.record(LimitationKind::Truncated, "evidence beyond the retention cap was omitted", omitted, None);
    }
    let status = if attempted == 0 { CoverageStatus::NotAttempted }
        else if succeeded == 0 { CoverageStatus::Failed }
        else if limitations.is_empty() { CoverageStatus::Complete } else { CoverageStatus::Partial };
    let coverage = Coverage {
        mechanism: Mechanism::OpenHandles,
        scope: if context.root.kind == super::TargetKind::Directory { CoverageScope::TargetTree } else { CoverageScope::Target },
        status, attempted: Some(attempted), succeeded: Some(succeeded),
        reason: (attempted == 0).then(|| "the budget expired before path inspection started".to_string()),
        truncation: (omitted > 0).then_some(Truncation {
            per_process_limit: super::MAX_EVIDENCE_PER_PROCESS, total_limit: super::MAX_EVIDENCE_TOTAL, omitted,
        }),
        limitations: limitations.into_vec(),
    };
    (processes.into_values().filter(|o| !o.evidence.is_empty()).collect(), coverage)
}

#[cfg(any(windows, test))]
fn read_owners(
    budget: &Budget,
    mut query: impl FnMut(&mut [usize]) -> (i32, usize),
) -> Result<Vec<u32>, PathFailure> {
    let mut buffer = vec![0usize; 4096 / size_of::<usize>()];
    loop {
        if budget.expired() {
            return Err(PathFailure { kind: LimitationKind::BudgetExhausted,
                detail: "budget exhausted before the next native query".to_string() });
        }
        let (result, written) = query(&mut buffer);
        if result as u32 == 0xc0000004 || result as u32 == 0x80000005 {
            if buffer.len() * size_of::<usize>() >= 16 * 1024 * 1024 {
                return Err("handle-owner list exceeded 16 MiB".to_string().into());
            }
            buffer.resize(buffer.len() * 2, 0);
            continue;
        }
        if result < 0 {
            return Err(PathFailure {
                kind: match result as u32 {
                    0xc0000056 => LimitationKind::DeletePending,
                    0xc0000022 => LimitationKind::PermissionDenied,
                    _ => LimitationKind::InspectionFailed,
                },
                detail: if result as u32 == 0xc0000056 {
                    "STATUS_DELETE_PENDING: holders of a delete-pending entry cannot be seen".to_string()
                } else { format!("NtQueryInformationFile failed: NTSTATUS {:#010x}", result as u32) },
            });
        }
        // SAFETY: initialized usize storage may be viewed as bytes over its allocation.
        let bytes = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), buffer.len() * size_of::<usize>()) };
        return decode(bytes, written).map_err(Into::into);
    }
}

/// Count followed by pointer-sized PIDs, aligned to the native pointer width.
#[cfg(any(windows, test))]
fn decode(bytes: &[u8], written: usize) -> Result<Vec<u32>, String> {
    let width = size_of::<usize>();
    if written < width || written > bytes.len() { return Err("invalid handle-owner response length".to_string()); }
    let count = u32::from_ne_bytes(bytes[..4].try_into().expect("count bytes")) as usize;
    if count > (written - width) / width { return Err("truncated handle-owner PID list".to_string()); }
    bytes[width..width + count * width].chunks_exact(width)
        .map(|pid| u32::try_from(usize::from_ne_bytes(pid.try_into().expect("PID bytes")))
            .map_err(|_| "handle-owner PID exceeds u32".to_string())).collect()
}

#[cfg(windows)]
pub(crate) mod ffi {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Storage::FileSystem::{CreateFileW, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, FileIdInfo, GetFileInformationByHandleEx, OPEN_EXISTING};
    use windows::core::PCWSTR;

    #[repr(C)]
    struct IoStatusBlock { status: usize, information: usize }
    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtQueryInformationFile(handle: HANDLE, status: *mut IoStatusBlock,
            buffer: *mut std::ffi::c_void, length: u32, class: u32) -> i32;
    }
    struct FileHandle(HANDLE);
    impl Drop for FileHandle {
        fn drop(&mut self) {
            // SAFETY: this query owns the handle and closes it exactly once.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    fn native_path(path: &std::path::Path) -> Vec<u16> {
        let path = path.as_os_str().encode_wide().collect::<Vec<_>>();
        let mut wide: Vec<u16> = if path.starts_with(&[92, 92, 63, 92]) { path }
            else if path.starts_with(&[92, 92]) {
                "\\\\?\\UNC\\".encode_utf16().chain(path.into_iter().skip(2)).collect()
            } else { "\\\\?\\".encode_utf16().chain(path).collect() };
        wide.push(0);
        wide
    }

    pub(crate) fn holders(entry: &ProbeEntry, budget: &Budget) -> Result<(FileIdentity, Vec<u32>), PathFailure> {
        let wide = native_path(&entry.path);
        // SAFETY: terminated native path, attribute-only access, and full sharing.
        // OPEN_REPARSE_POINT prevents a replaced entry from following a link outside the tree.
        let handle = unsafe { CreateFileW(PCWSTR(wide.as_ptr()), FILE_READ_ATTRIBUTES.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, None, OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, None) }
            .map(FileHandle).map_err(|e| {
                let error = super::super::identity::io_error(e);
                PathFailure {
                    kind: match error.raw_os_error() {
                        Some(5) => LimitationKind::PermissionDenied,
                        Some(303) => LimitationKind::DeletePending,
                        _ => LimitationKind::InspectionFailed,
                    },
                    detail: if error.raw_os_error() == Some(303) {
                        "STATUS_DELETE_PENDING: holders of a delete-pending entry cannot be seen".to_string()
                    } else { format!("open: {error}") },
                }
            })?;
        let mut info = FILE_ID_INFO::default();
        // SAFETY: a correctly sized writable FILE_ID_INFO and an owned file handle.
        unsafe { GetFileInformationByHandleEx(handle.0, FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast(), size_of::<FILE_ID_INFO>() as u32) }
            .map_err(|e| PathFailure::from(e.message()))?;
        let identity = FileIdentity::Windows { volume_serial: info.VolumeSerialNumber,
            file_id: u128::from_le_bytes(info.FileId.Identifier) };
        read_owners(budget, |buffer| {
            let mut status = IoStatusBlock { status: 0, information: 0 };
            // SAFETY: pointer-aligned writable buffer and class 47's IO_STATUS_BLOCK ABI.
            let result = unsafe { NtQueryInformationFile(handle.0, &mut status,
                buffer.as_mut_ptr().cast(), size_of_val(buffer) as u32, 47) };
            (result, status.information)
        }).map(|pids| (identity, pids))
    }

    #[cfg(test)]
    #[test]
    fn native_paths_preserve_unc_verbatim_and_unpaired_surrogate_spellings() {
        use std::os::windows::ffi::OsStringExt;
        for (path, expected) in [
            (r"C:\work\file", r"\\?\C:\work\file"),
            (r"\\server\share\file", r"\\?\UNC\server\share\file"),
            (r"\\?\UNC\server\share\file", r"\\?\UNC\server\share\file"),
        ] {
            assert_eq!(native_path(std::path::Path::new(path)),
                expected.encode_utf16().chain(Some(0)).collect::<Vec<_>>());
        }
        let mut units: Vec<_> = r"C:\work\".encode_utf16().collect();
        units.push(0xd800);
        let path = std::path::PathBuf::from(std::ffi::OsString::from_wide(&units));
        let expected: Vec<_> = "\\\\?\\".encode_utf16().chain(units).chain(Some(0)).collect();
        assert_eq!(native_path(&path), expected);
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_owner_response_rejects_invalid_lengths_counts_and_pid_overflow() {
        let width = size_of::<usize>();
        let mut bytes = vec![0; width * 2];
        bytes[..4].copy_from_slice(&1u32.to_ne_bytes());
        bytes[width..].copy_from_slice(&42usize.to_ne_bytes());
        assert_eq!(decode(&bytes, bytes.len()).unwrap(), vec![42]);
        for written in [0, width - 1, width, bytes.len() + 1] {
            assert!(decode(&bytes, written).is_err());
        }
        bytes[..4].copy_from_slice(&u32::MAX.to_ne_bytes());
        assert!(decode(&bytes, bytes.len()).is_err());
        bytes[..4].copy_from_slice(&1u32.to_ne_bytes());
        if width > 4 {
            bytes[width..].copy_from_slice(&usize::MAX.to_ne_bytes());
            assert!(decode(&bytes, bytes.len()).is_err());
        }
        bytes[..4].fill(0);
        assert!(decode(&bytes, width).unwrap().is_empty());
    }

    #[test]
    fn native_buffer_growth_retries_both_statuses_and_stops_when_the_budget_expires() {
        let budget = Budget::start(std::time::Duration::from_secs(60)).unwrap();
        let mut capacities = Vec::new();
        let pids = read_owners(&budget, |buffer| {
            capacities.push(size_of_val(buffer));
            match capacities.len() {
                1 => (0xc0000004u32 as i32, 0),
                2 => (0x80000005u32 as i32, 0),
                _ => { buffer[0] = 1; buffer[1] = 42; (0, size_of::<usize>() * 2) }
            }
        }).unwrap();
        assert_eq!(pids, vec![42]);
        assert_eq!(capacities, vec![4096, 8192, 16384]);
        let mut calls = 0;
        let failure = read_owners(&budget, |_| {
            calls += 1;
            budget.expire();
            (0xc0000004u32 as i32, 0)
        }).unwrap_err();
        assert_eq!(calls, 1);
        assert_eq!(failure.kind, LimitationKind::BudgetExhausted);
    }

    #[test]
    fn native_denial_delete_pending_and_malformed_success_are_distinct_gaps() {
        let budget = Budget::start(std::time::Duration::from_secs(60)).unwrap();
        for (status, kind) in [(0xc0000022u32, LimitationKind::PermissionDenied),
            (0xc0000056, LimitationKind::DeletePending), (0xc0000001, LimitationKind::InspectionFailed)]
        {
            let failure = read_owners(&budget, |_| (status as i32, 0)).unwrap_err();
            assert_eq!(failure.kind, kind);
        }
        let malformed = read_owners(&budget, |buffer| {
            buffer[0] = 100;
            (0, size_of::<usize>())
        }).unwrap_err();
        assert_eq!(malformed.kind, LimitationKind::InspectionFailed);
    }

    #[test]
    fn directories_precede_files_and_ignored_entries_are_scheduled_last_without_exclusion() {
        let temp = tempfile::tempdir().unwrap();
        let root_path = temp.path().join("tree");
        std::fs::create_dir(&root_path).unwrap();
        for directory in ["src", "target", "node_modules", "ignored"] {
            std::fs::create_dir(root_path.join(directory)).unwrap();
        }
        std::fs::write(root_path.join(".gitignore"), "ignored/\n").unwrap();
        std::fs::write(root_path.join("file.txt"), "").unwrap();
        let budget = Budget::start(std::time::Duration::from_secs(60)).unwrap();
        let root = super::super::root::resolve(&root_path, &budget).unwrap();
        let tree = super::super::tree::collect(&root, true, &budget);
        let context = InspectionContext { root: &root, index: &tree.index, budget: &budget };
        let entries = ordered_entries(&context);
        let names: Vec<_> = entries.iter().map(|e| e.path.file_name().unwrap().to_string_lossy()).collect();
        assert_eq!(names, vec!["tree", "src", ".gitignore", "file.txt", "ignored", "node_modules", "target"]);
        assert_eq!(entries.len(), tree.index.entries.len());
    }

}
