//! The macOS `libproc` backend, driven through the full query.
//!
//! On every Unix host the backend's classification logic runs against a
//! scripted [`Source`] whose vnodes carry real `(dev, ino)` identities from a
//! temp tree. On macOS, controlled children exercise the live `libproc` calls.

use super::backend::{InspectionContext, InspectionResult, MechanismInspection, UsageBackend};
use super::budget::Budget;
use super::macos::{Descriptor, Fault, LibprocBackend, OpenVnode, Source, Task, Vnode};
use super::*;
use crate::performance::counters;
use crate::performance::testing::measure;
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

/// Above every Linux and macOS `pid_max`, so enrichment through `sysinfo`
/// can never name a real host process.
const PID: u32 = 4_100_000_001;
const OTHER_PID: u32 = 4_100_000_002;
const START: u64 = 1_700_000_000_000_001;

const FREAD: u32 = 0x1;
const FWRITE: u32 = 0x2;
const O_EVTONLY: u32 = 0x8000;

struct Scene {
    _temp: TempDir,
    base: PathBuf,
    root: PathBuf,
    sibling: PathBuf,
}

fn scene() -> Scene {
    let temp = TempDir::new().expect("tempdir");
    let base = temp.path().canonicalize().expect("canonical tempdir");
    let root = base.join("app");
    let sibling = base.join("app-copy");
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::create_dir_all(&sibling).unwrap();
    std::fs::write(root.join("file.txt"), "x").unwrap();
    std::fs::write(root.join("sub/b.txt"), "b").unwrap();
    std::fs::write(sibling.join("file.txt"), "y").unwrap();
    Scene {
        _temp: temp,
        base,
        root,
        sibling,
    }
}

/// A vnode with `path`'s real identity, as the kernel would describe it.
fn vnode(path: &Path) -> Vnode {
    let metadata = std::fs::metadata(path).unwrap();
    Vnode {
        device: metadata.dev(),
        inode: metadata.ino(),
        links: metadata.nlink() as u16,
        path: path.as_os_str().as_bytes().to_vec(),
    }
}

fn task(start: u64) -> Result<Task, Fault> {
    Ok(Task {
        start_micros: start,
        name: OsString::from("scripted"),
        table_size: 8,
    })
}

/// One scripted process. `tasks` answers successive task reads (enumeration,
/// then each start-time check); the last answer repeats.
struct Scripted {
    tasks: Vec<Result<Task, Fault>>,
    descriptors: Result<Vec<Descriptor>, Fault>,
    vnodes: HashMap<i32, Result<OpenVnode, Fault>>,
    cwd: Result<Option<Vnode>, Fault>,
}

impl Scripted {
    fn new(start: u64) -> Self {
        Self {
            tasks: vec![task(start)],
            descriptors: Ok(Vec::new()),
            vnodes: HashMap::new(),
            cwd: Ok(None),
        }
    }

    fn open(mut self, fd: i32, path: &Path, flags: u32) -> Self {
        self.push(fd, true);
        self.vnodes.insert(
            fd,
            Ok(OpenVnode {
                flags,
                vnode: vnode(path),
            }),
        );
        self
    }

    fn fault(mut self, fd: i32, fault: Fault) -> Self {
        self.push(fd, true);
        self.vnodes.insert(fd, Err(fault));
        self
    }

    /// A socket, pipe, or kqueue descriptor.
    fn other(mut self, fd: i32) -> Self {
        self.push(fd, false);
        self
    }

    fn cwd(mut self, path: &Path) -> Self {
        self.cwd = Ok(Some(vnode(path)));
        self
    }

    fn push(&mut self, fd: i32, vnode: bool) {
        self.descriptors
            .as_mut()
            .expect("descriptors are scripted")
            .push(Descriptor { fd, vnode });
    }
}

#[derive(Default)]
struct Fake {
    pids: Option<Result<Vec<u32>, String>>,
    processes: BTreeMap<u32, Scripted>,
    task_reads: HashMap<u32, usize>,
    /// Every capacity a descriptor listing was asked for.
    capacities: Vec<usize>,
}

impl Fake {
    fn with(mut self, pid: u32, process: Scripted) -> Self {
        self.processes.insert(pid, process);
        self
    }

    fn query(&mut self, path: &Path, options: &PathUsageOptions) -> PathUsageReport {
        let budget = Budget::start(Duration::from_secs(60)).unwrap();
        self.query_with(path, options, &budget)
    }

    fn query_with(&mut self, path: &Path, options: &PathUsageOptions, budget: &Budget) -> PathUsageReport {
        let mut backend = LibprocBackend::with_source(self);
        run(path, options, &mut backend, budget, chrono::Utc::now()).expect("report")
    }

    fn process(&self, pid: u32) -> Result<&Scripted, Fault> {
        self.processes.get(&pid).ok_or(Fault::Gone)
    }
}

impl Source for &mut Fake {
    fn pids(&mut self) -> Result<Vec<u32>, String> {
        match &self.pids {
            Some(pids) => pids.clone(),
            None => Ok(self.processes.keys().copied().collect()),
        }
    }

    fn task(&mut self, pid: u32) -> Result<Task, Fault> {
        let reads = self.task_reads.entry(pid).or_default();
        let index = *reads;
        *reads += 1;
        let tasks = &self.processes.get(&pid).ok_or(Fault::Gone)?.tasks;
        tasks[index.min(tasks.len() - 1)].clone()
    }

    fn descriptors(&mut self, pid: u32, capacity: usize) -> Result<Vec<Descriptor>, Fault> {
        self.capacities.push(capacity);
        let mut descriptors = self.process(pid)?.descriptors.clone()?;
        descriptors.truncate(capacity);
        Ok(descriptors)
    }

    fn descriptor_vnode(&mut self, pid: u32, fd: i32) -> Result<OpenVnode, Fault> {
        self.process(pid)?.vnodes.get(&fd).cloned().unwrap_or(Err(Fault::Gone))
    }

    fn cwd(&mut self, pid: u32) -> Result<Option<Vnode>, Fault> {
        self.process(pid)?.cwd.clone()
    }
}

fn coverage(report: &PathUsageReport, mechanism: Mechanism) -> &Coverage {
    report
        .coverage
        .iter()
        .find(|c| c.mechanism == mechanism)
        .unwrap_or_else(|| panic!("no {mechanism:?} coverage in {:#?}", report.coverage))
}

fn limitation_count(coverage: &Coverage, kind: LimitationKind) -> u64 {
    coverage
        .limitations
        .iter()
        .filter(|l| l.kind == kind)
        .map(|l| l.count)
        .sum()
}

fn evidence(report: &PathUsageReport, kind: EvidenceKind) -> Vec<&Evidence> {
    report
        .processes
        .iter()
        .flat_map(|p| &p.evidence)
        .filter(|e| e.kind == kind)
        .collect()
}

fn paths(evidence: &Evidence) -> Vec<PathBuf> {
    evidence
        .matched_paths
        .iter()
        .map(|p| p.as_path().to_path_buf())
        .collect()
}

fn find<'a>(found: &[&'a Evidence], path: &Path) -> &'a Evidence {
    found
        .iter()
        .find(|e| paths(e) == vec![path.to_path_buf()])
        .unwrap_or_else(|| panic!("no evidence on {} in {found:#?}", path.display()))
}

#[test]
fn open_vnodes_and_the_cwd_are_matched_by_identity_with_access_and_event_only() {
    let scene = scene();
    let root = &scene.root;
    let mut fake = Fake::default().with(
        PID,
        Scripted::new(START)
            .other(0)
            .open(3, &root.join("file.txt"), FREAD | FWRITE)
            .open(4, &root.join("sub/b.txt"), FWRITE)
            .open(5, root, O_EVTONLY | FREAD)
            .open(6, &scene.sibling.join("file.txt"), FREAD)
            .open(7, &scene.base, O_EVTONLY | FREAD)
            .cwd(&root.join("sub")),
    );
    let (report, counts) = measure(|| fake.query(root, &PathUsageOptions::default()));
    assert_eq!(report.outcome, Outcome::Usable);
    assert_eq!(report.processes.len(), 1);
    assert_eq!(report.processes[0].creation_token, Some(START));

    let handles = evidence(&report, EvidenceKind::OpenHandle);
    // The sibling `app-copy` and the ancestor are not in scope.
    assert_eq!(handles.len(), 3, "{handles:#?}");
    let file = find(&handles, &root.join("file.txt"));
    assert_eq!(file.descriptor, Some(3));
    assert_eq!(file.access, Some(Access { read: true, write: true }));
    assert_eq!(file.event_only, Some(false));
    assert_eq!(file.observed_path.as_ref().unwrap().as_path(), root.join("file.txt"));
    let written = find(&handles, &root.join("sub/b.txt"));
    assert_eq!(written.access, Some(Access { read: false, write: true }));
    let watched = find(&handles, root);
    assert_eq!(watched.kind, EvidenceKind::OpenHandle, "an event-only open stays a handle");
    assert_eq!(watched.event_only, Some(true));
    assert_eq!(watched.access, Some(Access { read: true, write: false }));

    let cwd = evidence(&report, EvidenceKind::WorkingDirectory);
    assert_eq!(cwd.len(), 1);
    assert_eq!(paths(cwd[0]), vec![root.join("sub")]);
    assert_eq!(cwd[0].descriptor, None);

    for mechanism in [Mechanism::OpenHandles, Mechanism::WorkingDirectories] {
        let record = coverage(&report, mechanism);
        assert_eq!(record.status, CoverageStatus::Complete, "{record:#?}");
        assert_eq!(record.succeeded, Some(1));
    }
    for mechanism in [Mechanism::Fsevents, Mechanism::Polling] {
        let record = coverage(&report, mechanism);
        assert_eq!(record.status, CoverageStatus::Unsupported);
        assert!(record.reason.as_deref().is_some_and(|r| !r.is_empty()));
    }
    assert_eq!(
        coverage(&report, Mechanism::Fsevents).reason.as_deref(),
        Some(super::macos::FSEVENTS_UNSUPPORTED)
    );
    // Every listed descriptor counts once, including the non-vnode one.
    assert_eq!(counts.get(counters::QUERY_DESCRIPTOR_INSPECTIONS), 6);
}

#[test]
fn target_only_ignores_descendant_handles_and_cwd() {
    let scene = scene();
    let root = &scene.root;
    let mut fake = Fake::default().with(
        PID,
        Scripted::new(START)
            .open(3, &root.join("file.txt"), FREAD)
            .open(4, root, O_EVTONLY)
            .cwd(&root.join("sub")),
    );
    let report = fake.query(root, &PathUsageOptions::default().target_only());
    let found: Vec<&Evidence> = report.processes.iter().flat_map(|p| &p.evidence).collect();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(paths(found[0]), vec![root.clone()]);
    assert_eq!(found[0].event_only, Some(true));
    assert_eq!(coverage(&report, Mechanism::OpenHandles).scope, CoverageScope::Target);
}

#[test]
fn a_hard_link_outside_the_tree_reports_the_in_scope_path_and_the_alias() {
    let scene = scene();
    let alias = scene.base.join("alias.txt");
    std::fs::hard_link(scene.root.join("file.txt"), &alias).unwrap();
    // The kernel names the vnode by the spelling it was opened through.
    let mut fake = Fake::default().with(PID, Scripted::new(START).open(3, &alias, FREAD));
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    let handles = evidence(&report, EvidenceKind::OpenHandle);
    assert_eq!(handles.len(), 1);
    assert_eq!(paths(handles[0]), vec![scene.root.join("file.txt")]);
    assert_eq!(handles[0].observed_path.as_ref().unwrap().as_path(), alias);
}

#[test]
fn an_unlinked_file_is_matched_by_identity_and_not_by_its_former_path() {
    let scene = scene();
    std::fs::write(scene.root.join("doomed.txt"), "d").unwrap();
    std::fs::hard_link(scene.root.join("doomed.txt"), scene.root.join("kept.txt")).unwrap();
    let mut doomed = vnode(&scene.root.join("doomed.txt"));
    std::fs::remove_file(scene.root.join("doomed.txt")).unwrap();
    doomed.links = 1;
    // A vnode with no links left whose former path is in the tree: identity
    // is the only basis, and it names nothing in scope.
    let mut gone = vnode(&scene.root.join("kept.txt"));
    gone.inode = u64::MAX;
    gone.links = 0;
    gone.path = scene.root.join("file.txt").as_os_str().as_bytes().to_vec();
    let mut process = Scripted::new(START);
    for (fd, vnode) in [(3, doomed), (4, gone)] {
        process.push(fd, true);
        process.vnodes.insert(fd, Ok(OpenVnode { flags: FREAD, vnode }));
    }
    let mut fake = Fake::default().with(PID, process);
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    let handles = evidence(&report, EvidenceKind::OpenHandle);
    assert_eq!(handles.len(), 1, "{handles:#?}");
    assert_eq!(handles[0].descriptor, Some(3));
    assert_eq!(paths(handles[0]), vec![scene.root.join("kept.txt")]);
}

#[test]
fn a_denied_process_is_a_visible_gap_not_an_empty_result() {
    let scene = scene();
    let denied = Scripted {
        tasks: vec![Err(Fault::Denied)],
        descriptors: Err(Fault::Denied),
        vnodes: HashMap::new(),
        cwd: Err(Fault::Denied),
    };
    let mut fake = Fake::default()
        .with(PID, Scripted::new(START).open(3, &scene.root.join("file.txt"), FREAD))
        .with(OTHER_PID, denied);
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    assert_eq!(report.outcome, Outcome::Usable);
    // The process list itself is whole; only its detail is denied.
    let enumeration = coverage(&report, Mechanism::ProcessEnumeration);
    assert_eq!(enumeration.status, CoverageStatus::Complete, "{enumeration:#?}");
    assert_eq!(enumeration.attempted, Some(2));
    for mechanism in [Mechanism::OpenHandles, Mechanism::WorkingDirectories] {
        let record = coverage(&report, mechanism);
        assert_eq!(record.status, CoverageStatus::Partial, "{record:#?}");
        assert_eq!(record.attempted, Some(2));
        assert_eq!(record.succeeded, Some(1));
        assert_eq!(limitation_count(record, LimitationKind::PermissionDenied), 1);
        let example = &record.limitations[0].examples[0];
        assert_eq!(example.pid, Some(OTHER_PID));
    }
    assert_eq!(evidence(&report, EvidenceKind::OpenHandle).len(), 1);
}

#[test]
fn only_denied_processes_make_the_outcome_unavailable() {
    let scene = scene();
    let denied = Scripted {
        tasks: vec![Err(Fault::Denied)],
        descriptors: Err(Fault::Denied),
        vnodes: HashMap::new(),
        cwd: Err(Fault::Denied),
    };
    let mut fake = Fake::default().with(OTHER_PID, denied);
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    assert_eq!(report.outcome, Outcome::Unavailable);
    assert_eq!(coverage(&report, Mechanism::OpenHandles).status, CoverageStatus::Failed);
}

#[test]
fn denied_and_failed_descriptors_inside_a_listed_table_are_counted() {
    let scene = scene();
    let mut fake = Fake::default().with(
        PID,
        Scripted::new(START)
            .open(3, &scene.root.join("file.txt"), FREAD)
            .fault(4, Fault::Denied)
            .fault(5, Fault::Failed("Bad message".to_string()))
            // Closed between the listing and the read: no longer usage.
            .fault(6, Fault::Gone),
    );
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.status, CoverageStatus::Partial, "{handles:#?}");
    assert_eq!(limitation_count(handles, LimitationKind::PermissionDenied), 1);
    assert_eq!(limitation_count(handles, LimitationKind::InspectionFailed), 1);
    let failed = handles
        .limitations
        .iter()
        .find(|l| l.kind == LimitationKind::InspectionFailed)
        .unwrap();
    assert_eq!(failed.examples[0].detail.as_deref(), Some("fd 5: Bad message"));
    let found = evidence(&report, EvidenceKind::OpenHandle);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].descriptor, Some(3));
}

#[test]
fn a_closed_descriptor_alone_leaves_coverage_complete() {
    let scene = scene();
    let mut fake = Fake::default().with(PID, Scripted::new(START).fault(3, Fault::Gone));
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.status, CoverageStatus::Complete, "{handles:#?}");
    assert!(handles.limitations.is_empty());
}

#[test]
fn a_full_descriptor_list_is_reread_with_a_larger_buffer() {
    let scene = scene();
    let mut process = Scripted::new(START);
    for fd in 0..39 {
        process = process.other(fd);
    }
    // The only in-scope descriptor is past the first two buffer sizes.
    process = process.open(39, &scene.root.join("file.txt"), FREAD);
    process.tasks = vec![Ok(Task {
        start_micros: START,
        name: OsString::from("busy"),
        table_size: 2,
    })];
    let mut fake = Fake::default().with(PID, process);
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    // `pbi_nfiles` + 16 headroom, then doubled while a listing fills it.
    assert_eq!(fake.capacities, vec![18, 36, 72]);
    let found = evidence(&report, EvidenceKind::OpenHandle);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].descriptor, Some(39));
    assert_eq!(coverage(&report, Mechanism::OpenHandles).status, CoverageStatus::Complete);
}

#[test]
fn an_unreadable_table_size_starts_from_the_default_capacity() {
    let scene = scene();
    let mut process = Scripted::new(START).open(3, &scene.root.join("file.txt"), FREAD);
    process.tasks = vec![Err(Fault::Failed("short read".to_string()))];
    let mut fake = Fake::default().with(PID, process);
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    assert_eq!(fake.capacities, vec![256]);
    // No token: enumeration names the gap and the record is kept apart.
    let enumeration = coverage(&report, Mechanism::ProcessEnumeration);
    assert_eq!(enumeration.status, CoverageStatus::Partial);
    assert_eq!(limitation_count(enumeration, LimitationKind::IdentityUncertain), 1);
    assert_eq!(report.processes.len(), 1);
    assert!(report.processes[0].identity_uncertain);
    assert_eq!(evidence(&report, EvidenceKind::OpenHandle).len(), 1);
}

#[test]
fn a_failed_process_list_fails_enumeration_and_attempts_nothing() {
    let scene = scene();
    let mut fake = Fake {
        pids: Some(Err("the process list could not be read: Operation not permitted".to_string())),
        ..Fake::default()
    };
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    assert_eq!(report.outcome, Outcome::Unavailable);
    let enumeration = coverage(&report, Mechanism::ProcessEnumeration);
    assert_eq!(enumeration.status, CoverageStatus::Failed);
    assert!(enumeration.reason.as_deref().unwrap().contains("Operation not permitted"));
    for mechanism in [Mechanism::OpenHandles, Mechanism::WorkingDirectories] {
        assert_eq!(coverage(&report, mechanism).status, CoverageStatus::NotAttempted);
    }
}

#[test]
fn a_process_that_exits_after_listing_is_not_a_candidate() {
    let scene = scene();
    let mut fake = Fake {
        pids: Some(Ok(vec![PID, OTHER_PID])),
        ..Fake::default()
    }
    .with(PID, Scripted::new(START));
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    let enumeration = coverage(&report, Mechanism::ProcessEnumeration);
    assert_eq!(enumeration.status, CoverageStatus::Complete);
    assert_eq!(enumeration.attempted, Some(1));
}

/// Inspects the one scripted process directly.
fn inspect_directly(scene: &Scene, fake: &mut Fake, inspection_budget: &Budget) -> Vec<MechanismInspection> {
    let budget = Budget::start(Duration::from_secs(60)).unwrap();
    let root = super::root::resolve(&scene.root, &budget).unwrap();
    let tree = super::tree::collect(&root, true, &budget);
    let mut backend = LibprocBackend::with_source(fake);
    let mut enumeration = backend.enumerate(&budget).unwrap();
    assert_eq!(enumeration.processes.len(), 1);
    let candidate = enumeration.processes.remove(0);
    let context = InspectionContext {
        budget: inspection_budget,
        root: &root,
        index: &tree.index,
    };
    backend.inspect(&candidate, &context)
}

#[test]
fn a_reused_pid_discards_everything_read_during_inspection() {
    let scene = scene();
    let mut process = Scripted::new(START)
        .open(3, &scene.root.join("file.txt"), FREAD)
        .cwd(&scene.root);
    process.tasks = vec![task(START), task(START + 1)];
    let mut fake = Fake::default().with(PID, process);
    let budget = Budget::start(Duration::from_secs(60)).unwrap();
    let inspections = inspect_directly(&scene, &mut fake, &budget);
    assert_eq!(inspections.len(), 2);
    for inspection in inspections {
        assert_eq!(inspection.result, InspectionResult::Vanished);
        assert!(inspection.evidence.is_empty());
    }
}

#[test]
fn a_process_that_exits_during_inspection_has_vanished() {
    let scene = scene();
    let mut process = Scripted::new(START).cwd(&scene.root);
    process.descriptors = Err(Fault::Gone);
    process.cwd = Err(Fault::Gone);
    process.tasks = vec![task(START), Err(Fault::Gone)];
    let mut fake = Fake::default().with(PID, process);
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    assert!(report.processes.is_empty());
    for mechanism in [Mechanism::OpenHandles, Mechanism::WorkingDirectories] {
        let record = coverage(&report, mechanism);
        assert_eq!(limitation_count(record, LimitationKind::ProcessDisappeared), 1, "{record:#?}");
    }
}

#[test]
fn a_budget_expiring_between_descriptors_keeps_what_was_read() {
    let scene = scene();
    let mut fake = Fake::default().with(
        PID,
        Scripted::new(START)
            .open(3, &scene.root.join("file.txt"), FREAD)
            .open(4, &scene.root.join("sub/b.txt"), FREAD),
    );
    // The first per-descriptor check passes; the second finds it expired.
    let budget = Budget::expiring_after_checks(1);
    let inspections = inspect_directly(&scene, &mut fake, &budget);
    let handles = inspections
        .iter()
        .find(|i| i.mechanism == Mechanism::OpenHandles)
        .unwrap();
    assert_eq!(handles.result, InspectionResult::Partial);
    assert_eq!(handles.evidence.len(), 1);
    assert!(
        handles
            .limitations
            .iter()
            .any(|l| l.kind == LimitationKind::BudgetExhausted)
    );
}

/// Live `libproc` on macOS.
#[cfg(target_os = "macos")]
mod live {
    use super::*;
    use std::ffi::CString;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::process::{Child, Command, Stdio};

    const OPEN: &str = "SNIFF_QUERY_CHILD_OPEN";
    const EVENT_ONLY: &str = "SNIFF_QUERY_CHILD_EVENT_ONLY";
    const CWD: &str = "SNIFF_QUERY_CHILD_CWD";

    fn env_paths(name: &str) -> Vec<PathBuf> {
        std::env::var_os(name)
            .map(|value| std::env::split_paths(&value).collect())
            .unwrap_or_default()
    }

    /// Holds files open read-write, holds `O_EVTONLY` descriptors, and sits
    /// in a directory until its stdin closes.
    #[test]
    #[ignore = "subprocess fixture invoked by the live libproc tests"]
    fn usage_child() {
        let held: Vec<std::fs::File> = env_paths(OPEN)
            .iter()
            .map(|path| {
                std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(path)
                    .unwrap_or_else(|e| panic!("open {}: {e}", path.display()))
            })
            .collect();
        let mut event_only = Vec::new();
        for path in env_paths(EVENT_ONLY) {
            let c_path = CString::new(path.as_os_str().as_bytes()).unwrap();
            // SAFETY: a plain syscall wrapper on a valid C string.
            let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_EVTONLY) };
            assert!(fd >= 0, "O_EVTONLY {}: {}", path.display(), std::io::Error::last_os_error());
            event_only.push(fd);
        }
        std::env::set_current_dir(std::env::var_os(CWD).expect("cwd")).unwrap();
        println!("ready {}", event_only.iter().map(i32::to_string).collect::<Vec<_>>().join(","));
        std::io::stdout().flush().unwrap();
        let mut sink = Vec::new();
        let _ = std::io::stdin().read_to_end(&mut sink);
        drop(held);
    }

    /// Kills and reaps the child however the test ends.
    struct ChildGuard(Child);

    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// Starts [`usage_child`] and waits for its readiness line, which
    /// carries its `O_EVTONLY` descriptor numbers.
    fn spawn_child(open: &[PathBuf], event_only: &[PathBuf], cwd: &Path) -> (ChildGuard, Vec<u64>) {
        let mut command = Command::new(std::env::current_exe().expect("test executable"));
        command
            .args([
                "filesystem::query::macos_tests::live::usage_child",
                "--exact",
                "--ignored",
                "--nocapture",
            ])
            .env(OPEN, std::env::join_paths(open).unwrap())
            .env(EVENT_ONLY, std::env::join_paths(event_only).unwrap())
            .env(CWD, cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = ChildGuard(command.spawn().expect("spawn the usage child"));
        let stdout = child.0.stdout.take().unwrap();
        for line in BufReader::new(stdout).lines() {
            let line = line.expect("child stdout");
            if let Some(fds) = line.strip_prefix("ready ") {
                let fds = fds
                    .split(',')
                    .filter(|fd| !fd.is_empty())
                    .map(|fd| fd.trim().parse().expect("descriptor"))
                    .collect();
                return (child, fds);
            }
        }
        panic!("the usage child exited before it was ready");
    }

    fn live_query(path: &Path, options: PathUsageOptions) -> PathUsageReport {
        query_path_usage(path, &options.with_deadline(Duration::from_secs(120))).expect("report")
    }

    fn unix_identity(path: &Path) -> FileIdentity {
        let metadata = std::fs::metadata(path).unwrap();
        FileIdentity::Unix {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }

    /// A scene under `/var/tmp`, which macOS links to `/private/var/tmp`,
    /// returned with the `/var` spelling of its root.
    fn aliased_scene() -> (Scene, PathBuf) {
        let temp = TempDir::new_in("/var/tmp").expect("tempdir under /var/tmp");
        let base = temp.path().canonicalize().expect("canonical tempdir");
        assert!(base.starts_with("/private/var/tmp"), "{}", base.display());
        let root = base.join("app");
        let sibling = base.join("app-copy");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(&sibling).unwrap();
        std::fs::write(root.join("file.txt"), "x").unwrap();
        std::fs::write(sibling.join("file.txt"), "y").unwrap();
        let alias = Path::new("/").join(root.strip_prefix("/private").unwrap());
        let scene = Scene {
            _temp: temp,
            base,
            root,
            sibling,
        };
        (scene, alias)
    }

    #[test]
    fn a_controlled_child_is_found_by_its_descriptors_and_cwd_through_the_var_alias() {
        let (scene, alias) = aliased_scene();
        let root = &scene.root;
        let open = [root.join("file.txt"), scene.sibling.join("file.txt")];
        let event_only = [root.clone(), scene.base.clone()];
        let (child, event_fds) = spawn_child(&open, &event_only, &root.join("sub"));
        let child_pid = child.0.id();

        let report = live_query(&alias, PathUsageOptions::default());
        assert_eq!(report.target.requested.as_path(), alias);
        assert_eq!(report.target.resolved.as_path(), root.as_path());
        assert_eq!(report.outcome, Outcome::Usable);
        let records: Vec<&ProcessRecord> =
            report.processes.iter().filter(|p| p.pid == child_pid).collect();
        assert_eq!(records.len(), 1, "one record per process: {:#?}", report.processes);
        let record = records[0];
        assert!(record.creation_token.is_some());
        assert!(!record.identity_uncertain);

        let handles: Vec<&Evidence> =
            record.evidence.iter().filter(|e| e.kind == EvidenceKind::OpenHandle).collect();
        let file = find(&handles, &root.join("file.txt"));
        // The vnode's device is compared in the encoding `std` reports.
        assert_eq!(file.identity, Some(unix_identity(&root.join("file.txt"))));
        assert_eq!(file.access, Some(Access { read: true, write: true }));
        assert_eq!(file.event_only, Some(false));
        // The kernel reports the canonical `/private/var` spelling.
        assert_eq!(file.observed_path.as_ref().unwrap().as_path(), root.join("file.txt"));
        let watched = find(&handles, root);
        assert_eq!(watched.event_only, Some(true));
        assert_eq!(watched.descriptor, Some(event_fds[0]));
        assert_eq!(watched.identity, Some(unix_identity(root)));

        let cwd: Vec<&Evidence> =
            record.evidence.iter().filter(|e| e.kind == EvidenceKind::WorkingDirectory).collect();
        assert_eq!(cwd.len(), 1);
        assert_eq!(paths(cwd[0]), vec![root.join("sub")]);

        // The sibling file and the event-only ancestor are out of scope.
        assert_eq!(record.evidence.len(), 3, "{:#?}", record.evidence);
        assert_eq!(coverage(&report, Mechanism::Fsevents).status, CoverageStatus::Unsupported);
        drop(child);
    }

    #[test]
    fn target_only_reports_the_target_and_not_descendant_usage() {
        let (scene, _) = aliased_scene();
        let root = &scene.root;
        let (child, _) = spawn_child(&[root.join("file.txt")], std::slice::from_ref(root), &root.join("sub"));
        let child_pid = child.0.id();
        let report = live_query(root, PathUsageOptions::default().target_only());
        let record = report
            .processes
            .iter()
            .find(|p| p.pid == child_pid)
            .unwrap_or_else(|| panic!("child missing: {:#?}", report.processes));
        assert_eq!(record.evidence.len(), 1, "{:#?}", record.evidence);
        assert_eq!(paths(&record.evidence[0]), vec![root.clone()]);
        assert_eq!(record.evidence[0].event_only, Some(true));
        drop(child);
    }

    #[test]
    fn the_querying_process_keeps_its_genuine_usage_only() {
        let scene = scene();
        let file = scene.root.join("file.txt");
        let held = std::fs::File::open(&file).unwrap();
        let report = live_query(&scene.root, PathUsageOptions::default());
        drop(held);
        let own = std::process::id();
        let own_records: Vec<&ProcessRecord> =
            report.processes.iter().filter(|p| p.pid == own).collect();
        assert_eq!(own_records.len(), 1, "{:#?}", report.processes);
        // The query's own tree walk has closed before it inspects itself.
        let evidence = &own_records[0].evidence;
        assert_eq!(evidence.len(), 1, "{evidence:#?}");
        assert_eq!(paths(&evidence[0]), vec![file]);
        assert_eq!(evidence[0].access, Some(Access { read: true, write: false }));
    }

    #[test]
    fn the_shipped_macos_backend_inventories_descriptors_and_cwd() {
        let scene = scene();
        let report = live_query(&scene.root, PathUsageOptions::default());
        assert_eq!(report.outcome, Outcome::Usable);
        let enumeration = coverage(&report, Mechanism::ProcessEnumeration);
        assert!(enumeration.attempted.is_some_and(|n| n > 1), "{enumeration:#?}");
        for mechanism in [Mechanism::OpenHandles, Mechanism::WorkingDirectories] {
            let record = coverage(&report, mechanism);
            assert!(
                matches!(record.status, CoverageStatus::Complete | CoverageStatus::Partial),
                "{mechanism:?}: {record:#?}"
            );
            assert!(record.succeeded.is_some_and(|n| n > 0), "{mechanism:?}: {record:#?}");
        }
        for mechanism in [Mechanism::Fsevents, Mechanism::Polling] {
            assert_eq!(coverage(&report, mechanism).status, CoverageStatus::Unsupported);
        }
        let mechanisms: Vec<Mechanism> = report.coverage.iter().map(|c| c.mechanism).collect();
        assert_eq!(
            mechanisms,
            vec![
                Mechanism::ProcessEnumeration,
                Mechanism::TreeIdentity,
                Mechanism::OpenHandles,
                Mechanism::WorkingDirectories,
                Mechanism::Fsevents,
                Mechanism::Polling,
            ]
        );
        // Another user's processes (launchd, PID 1) are denied, never silently
        // read as "no matches".
        if !running_as_root() {
            let handles = coverage(&report, Mechanism::OpenHandles);
            assert_eq!(handles.status, CoverageStatus::Partial, "{handles:#?}");
            let denied = handles
                .limitations
                .iter()
                .find(|l| l.kind == LimitationKind::PermissionDenied)
                .unwrap_or_else(|| panic!("no denial recorded: {handles:#?}"));
            assert!(denied.count > 0);
        }
    }

    fn running_as_root() -> bool {
        // SAFETY: `geteuid` has no preconditions.
        unsafe { libc::geteuid() == 0 }
    }
}
