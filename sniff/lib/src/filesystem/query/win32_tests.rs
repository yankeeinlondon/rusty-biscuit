//! The Windows loaded-module backend, driven through the full query.
//!
//! A module is observed by path only, so on every host the backend's
//! classification runs against a scripted [`Source`] whose module paths name
//! real files in a temp tree. On Windows, spelling variants and controlled
//! children exercise the lookup and the live Win32 calls.

use super::backend::{
    InspectionContext, InspectionResult, MechanismInspection, ProcessCandidate, ProcessDetails,
    UsageBackend,
};
use super::budget::Budget;
use super::win32::{
    self, DEFAULT_MODULE_CAPACITY, Fault, MAX_MODULE_CAPACITY, Module, ModuleBackend, Opened, Source,
};
use super::*;
use crate::performance::counters;
use crate::performance::testing::measure;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;
use tempfile::TempDir;

/// Above every Windows, Linux, and macOS PID in practice, so enrichment
/// through `sysinfo` can never name a real host process.
const PID: u32 = 4_100_000_001;
const OTHER_PID: u32 = 4_100_000_002;
/// 2023-11-14T22:13:20Z as a FILETIME.
const CREATED: u64 = 133_444_736_000_000_000;
const USER: &str = "S-1-5-21-1-2-3-1001";

struct Scene {
    _temp: TempDir,
    base: PathBuf,
    root: PathBuf,
    sibling: PathBuf,
}

fn scene() -> Scene {
    let temp = TempDir::new().expect("tempdir");
    let base = dunce_canonical(temp.path());
    let root = base.join("app");
    let sibling = base.join("app-copy");
    std::fs::create_dir_all(root.join("bin")).unwrap();
    std::fs::create_dir_all(&sibling).unwrap();
    std::fs::write(root.join("bin").join("tool.exe"), "x").unwrap();
    std::fs::write(root.join("bin").join("helper.dll"), "x").unwrap();
    std::fs::write(sibling.join("tool.exe"), "y").unwrap();
    Scene {
        _temp: temp,
        base,
        root,
        sibling,
    }
}

/// The long, non-verbatim spelling a module list reports.
fn dunce_canonical(path: &Path) -> PathBuf {
    let canonical = path.canonicalize().expect("canonical path");
    #[cfg(windows)]
    {
        let text = canonical.to_string_lossy();
        if let Some(rest) = text.strip_prefix(r"\\?\")
            && !rest.starts_with("UNC")
        {
            return PathBuf::from(rest);
        }
    }
    canonical
}

/// One scripted process.
struct Scripted {
    open: Result<(Option<PathBuf>, bool), Fault>,
    /// Successive creation-time reads through the held handle; the last
    /// answer repeats.
    tokens: Vec<Result<u64, Fault>>,
    modules: Result<Vec<Result<PathBuf, Fault>>, Fault>,
    /// Every read reports one module more than it was asked for.
    growing: bool,
    exited: bool,
}

impl Scripted {
    fn new() -> Self {
        Self {
            open: Ok((None, true)),
            tokens: vec![Ok(CREATED)],
            modules: Ok(Vec::new()),
            growing: false,
            exited: false,
        }
    }

    fn image(mut self, image: &Path) -> Self {
        self.open = Ok((Some(image.to_path_buf()), true));
        self.module(image)
    }

    fn module(mut self, path: &Path) -> Self {
        self.modules.as_mut().expect("modules are scripted").push(Ok(path.to_path_buf()));
        self
    }

    fn module_fault(mut self, fault: Fault) -> Self {
        self.modules.as_mut().expect("modules are scripted").push(Err(fault));
        self
    }
}

#[derive(Default)]
struct Fake {
    pids: Option<Result<Vec<u32>, String>>,
    processes: BTreeMap<u32, Scripted>,
    opens: HashMap<u32, usize>,
    token_reads: HashMap<u32, usize>,
    /// Every capacity a module listing was asked for.
    capacities: Vec<usize>,
    /// PIDs whose handle was closed, in order.
    closed: Rc<RefCell<Vec<u32>>>,
}

/// Records its own close.
struct FakeHandle {
    pid: u32,
    closed: Rc<RefCell<Vec<u32>>>,
}

impl Drop for FakeHandle {
    fn drop(&mut self) {
        self.closed.borrow_mut().push(self.pid);
    }
}

impl Fake {
    fn with(mut self, pid: u32, process: Scripted) -> Self {
        self.processes.insert(pid, process);
        self
    }

    fn query(&mut self, path: &Path, options: &PathUsageOptions) -> PathUsageReport {
        let budget = Budget::start(Duration::from_secs(60)).unwrap();
        let mut backend = ModuleBackend::with_source(self);
        run(path, options, &mut backend, &budget, chrono::Utc::now()).expect("report")
    }

    fn process(&self, pid: u32) -> Result<&Scripted, Fault> {
        self.processes.get(&pid).ok_or(Fault::Gone)
    }
}

impl Source for &mut Fake {
    type Process = FakeHandle;

    fn pids(&mut self) -> Result<Vec<u32>, String> {
        match &self.pids {
            Some(pids) => pids.clone(),
            None => Ok(self.processes.keys().copied().collect()),
        }
    }

    fn open(&mut self, pid: u32) -> Result<Opened<FakeHandle>, Fault> {
        *self.opens.entry(pid).or_default() += 1;
        let (image, can_read_modules) = self.process(pid)?.open.clone()?;
        Ok(Opened {
            process: FakeHandle {
                pid,
                closed: Rc::clone(&self.closed),
            },
            creation_time: CREATED,
            image,
            can_read_modules,
        })
    }

    fn creation_time(&mut self, process: &FakeHandle) -> Result<u64, Fault> {
        let reads = self.token_reads.entry(process.pid).or_default();
        let index = *reads;
        *reads += 1;
        let tokens = &self.process(process.pid)?.tokens;
        tokens[index.min(tokens.len() - 1)].clone()
    }

    fn exited(&mut self, process: &FakeHandle) -> bool {
        self.process(process.pid).is_ok_and(|p| p.exited)
    }

    fn modules(&mut self, process: &FakeHandle, capacity: usize) -> Result<(Vec<Module>, usize), Fault> {
        self.capacities.push(capacity);
        let scripted = self.process(process.pid)?;
        let count = if scripted.growing {
            capacity + 1
        } else {
            scripted.modules.clone()?.len()
        };
        Ok(((0..count.min(capacity)).map(Module).collect(), count))
    }

    fn details(&mut self, process: &FakeHandle) -> Result<ProcessDetails, String> {
        let scripted = self.process(process.pid).map_err(|_| "gone".to_string())?;
        let image = scripted.open.clone().ok().and_then(|(image, _)| image);
        Ok(ProcessDetails {
            name: image.as_deref().and_then(Path::file_name).map(ToOwned::to_owned),
            executable: image,
            user: Some(USER.to_string()),
            start_time: win32::filetime_to_utc(CREATED),
        })
    }

    fn module_path(&mut self, process: &FakeHandle, module: Module) -> Result<PathBuf, Fault> {
        let modules = self.process(process.pid)?.modules.clone()?;
        modules.get(module.0).cloned().unwrap_or(Err(Fault::Gone))
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

fn paths(evidence: &Evidence) -> Vec<PathBuf> {
    evidence
        .matched_paths
        .iter()
        .map(|p| p.as_path().to_path_buf())
        .collect()
}

/// The spelling the tree index reports a path under: beneath the canonical
/// root, which is a verbatim `\\?\` path on Windows.
fn indexed(path: &Path) -> PathBuf {
    path.canonicalize().expect("canonical path")
}

fn module_paths(record: &ProcessRecord) -> Vec<PathBuf> {
    record.evidence.iter().flat_map(paths).collect()
}

#[test]
fn modules_loaded_from_the_tree_are_evidence_and_others_are_not() {
    let scene = scene();
    let root = &scene.root;
    let tool = root.join("bin").join("tool.exe");
    let helper = root.join("bin").join("helper.dll");
    let mut fake = Fake::default().with(
        PID,
        Scripted::new()
            .image(&tool)
            .module(&helper)
            // A sibling whose name shares the root's prefix, and the parent.
            .module(&scene.sibling.join("tool.exe"))
            .module(&scene.base.join("system.dll")),
    );
    let (report, counts) = measure(|| fake.query(root, &PathUsageOptions::default()));

    assert_eq!(report.outcome, Outcome::Usable);
    assert_eq!(report.processes.len(), 1);
    let record = &report.processes[0];
    assert_eq!(record.pid, PID);
    assert_eq!(record.creation_token, Some(CREATED));
    assert!(!record.identity_uncertain);
    assert_eq!(record.name.as_ref().map(|n| n.as_os_str().to_owned()), Some("tool.exe".into()));
    assert_eq!(record.executable.as_ref().map(|e| e.as_path()), Some(tool.as_path()));
    assert_eq!(record.start_time, win32::filetime_to_utc(CREATED));
    // Enrichment read the account through the handle held since enumeration.
    assert_eq!(record.user.as_deref(), Some(USER));
    assert!(report.limitations.is_empty(), "{:#?}", report.limitations);

    assert_eq!(module_paths(record), vec![indexed(&helper), indexed(&tool)]);
    for evidence in &record.evidence {
        assert_eq!(evidence.kind, EvidenceKind::LoadedModule);
        assert_eq!(evidence.mechanism, Mechanism::LoadedModules);
        assert_eq!(evidence.match_basis, MatchBasis::PathLookup);
        let observed = evidence.observed_path.as_ref().unwrap().as_path();
        assert_eq!(indexed(observed), paths(evidence)[0]);
        assert!(observed == tool || observed == helper, "{}", observed.display());
        assert!(evidence.identity.is_some());
        assert_eq!(evidence.access, None);
        assert_eq!(evidence.descriptor, None);
        assert_eq!(evidence.event_only, None);
    }

    let modules = coverage(&report, Mechanism::LoadedModules);
    assert_eq!(modules.status, CoverageStatus::Complete, "{modules:#?}");
    assert_eq!((modules.attempted, modules.succeeded), (Some(1), Some(1)));
    // Every module is read once; only the two inside the root are looked up.
    assert_eq!(counts.get(counters::QUERY_MODULE_INSPECTIONS), 4);
    assert_eq!(counts.get(counters::QUERY_PATH_LOOKUPS), 2);
    assert_eq!(counts.get(counters::QUERY_DESCRIPTOR_INSPECTIONS), 0);
}

#[test]
fn the_windows_mechanisms_are_fixed_with_reasons_for_each_unsupported_one() {
    let scene = scene();
    let mut fake = Fake::default().with(PID, Scripted::new());
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    let mechanisms: Vec<Mechanism> = report.coverage.iter().map(|c| c.mechanism).collect();
    assert_eq!(
        mechanisms,
        vec![
            Mechanism::ProcessEnumeration,
            Mechanism::TreeIdentity,
            Mechanism::OpenHandles,
            Mechanism::LoadedModules,
            Mechanism::WorkingDirectories,
            Mechanism::DirectoryChangeSubscriptions,
            Mechanism::Polling,
        ]
    );
    for (mechanism, reason) in [
        (Mechanism::OpenHandles, win32::OPEN_HANDLES_UNSUPPORTED),
        (Mechanism::WorkingDirectories, win32::WORKING_DIRECTORIES_UNSUPPORTED),
        (
            Mechanism::DirectoryChangeSubscriptions,
            win32::DIRECTORY_CHANGE_SUBSCRIPTIONS_UNSUPPORTED,
        ),
        (Mechanism::Polling, super::backend::POLLING_UNSUPPORTED),
    ] {
        let record = coverage(&report, mechanism);
        assert_eq!(record.status, CoverageStatus::Unsupported, "{mechanism:?}");
        assert_eq!(record.reason.as_deref(), Some(reason));
    }
    // A clean module inspection with nothing in scope is still usable.
    assert_eq!(coverage(&report, Mechanism::LoadedModules).status, CoverageStatus::Complete);
    assert_eq!(report.outcome, Outcome::Usable);
    assert!(report.processes.is_empty());
}

#[test]
fn a_module_list_larger_than_the_buffer_is_reread_in_full() {
    let scene = scene();
    let tool = scene.root.join("bin").join("tool.exe");
    let mut process = Scripted::new();
    for index in 0..DEFAULT_MODULE_CAPACITY + 40 {
        process = process.module(&scene.base.join(format!("m{index}.dll")));
    }
    // The only in-scope module sits beyond the first buffer.
    let mut fake = Fake::default().with(PID, process.module(&tool));
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    assert_eq!(module_paths(&report.processes[0]), vec![indexed(&tool)]);
    assert_eq!(fake.capacities.len(), 2, "{:?}", fake.capacities);
    assert_eq!(fake.capacities[0], DEFAULT_MODULE_CAPACITY);
    assert!(fake.capacities[1] > DEFAULT_MODULE_CAPACITY + 40);
    assert_eq!(coverage(&report, Mechanism::LoadedModules).status, CoverageStatus::Complete);
}

#[test]
fn a_module_list_that_never_fits_fails_instead_of_dropping_modules() {
    let scene = scene();
    let mut process = Scripted::new().module(&scene.root.join("bin").join("tool.exe"));
    process.growing = true;
    let mut fake = Fake::default().with(PID, process);
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    assert!(report.processes.is_empty());
    let modules = coverage(&report, Mechanism::LoadedModules);
    assert_eq!(modules.status, CoverageStatus::Failed, "{modules:#?}");
    assert_eq!(limitation_count(modules, LimitationKind::InspectionFailed), 1);
    assert_eq!(report.outcome, Outcome::Unavailable);
    assert!(fake.capacities.iter().all(|&c| c <= MAX_MODULE_CAPACITY));
}

#[test]
fn a_process_that_cannot_be_opened_is_a_visible_gap_not_an_empty_result() {
    let scene = scene();
    let tool = scene.root.join("bin").join("tool.exe");
    let mut denied = Scripted::new().module(&tool);
    denied.open = Err(Fault::Denied);
    let mut fake = Fake::default()
        .with(PID, Scripted::new().image(&tool))
        .with(OTHER_PID, denied);
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    // The PID list is whole; the denied process is a candidate without a token.
    let enumeration = coverage(&report, Mechanism::ProcessEnumeration);
    assert_eq!(enumeration.status, CoverageStatus::Complete);
    assert_eq!(enumeration.attempted, Some(2));
    let modules = coverage(&report, Mechanism::LoadedModules);
    assert_eq!(modules.status, CoverageStatus::Partial, "{modules:#?}");
    assert_eq!(limitation_count(modules, LimitationKind::PermissionDenied), 1);
    assert_eq!(report.outcome, Outcome::Usable);
    assert_eq!(report.processes.iter().map(|p| p.pid).collect::<Vec<_>>(), vec![PID]);
}

#[test]
fn only_denied_processes_make_the_outcome_unavailable() {
    let scene = scene();
    let mut denied = Scripted::new();
    denied.open = Err(Fault::Denied);
    let mut limited = Scripted::new();
    limited.open = Ok((None, false));
    let mut fake = Fake::default().with(PID, denied).with(OTHER_PID, limited);
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    let modules = coverage(&report, Mechanism::LoadedModules);
    assert_eq!(modules.status, CoverageStatus::Failed, "{modules:#?}");
    assert_eq!(limitation_count(modules, LimitationKind::PermissionDenied), 2);
    assert_eq!(report.outcome, Outcome::Unavailable);
}

#[test]
fn a_process_opened_without_memory_access_keeps_its_identity_and_denies_its_modules() {
    let scene = scene();
    let mut limited = Scripted::new().module(&scene.root.join("bin").join("tool.exe"));
    limited.open = Ok((Some(scene.root.join("bin").join("tool.exe")), false));
    let mut fake = Fake::default().with(PID, limited);
    let budget = Budget::start(Duration::from_secs(60)).unwrap();
    let (candidates, inspections) = inspect_directly(&scene, &mut fake, &budget);
    assert_eq!(candidates[0].creation_token, Some(CREATED));
    assert!(candidates[0].retained_handle);
    assert_eq!(inspections[0].result, InspectionResult::Denied);
    assert!(inspections[0].evidence.is_empty());
    // Its handle closes as soon as inspection shows it can contribute nothing.
    assert_eq!(*fake.closed.borrow(), vec![PID]);
    assert!(fake.capacities.is_empty());
}

#[test]
fn an_open_failure_is_an_identity_uncertain_enumeration_gap() {
    let scene = scene();
    let mut failing = Scripted::new();
    failing.open = Err(Fault::Failed("GetProcessTimes failed".to_string()));
    let mut fake = Fake::default().with(PID, failing);
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    let enumeration = coverage(&report, Mechanism::ProcessEnumeration);
    assert_eq!(enumeration.status, CoverageStatus::Partial);
    assert_eq!(limitation_count(enumeration, LimitationKind::IdentityUncertain), 1);
    let modules = coverage(&report, Mechanism::LoadedModules);
    assert_eq!(limitation_count(modules, LimitationKind::InspectionFailed), 1);
    let example = &modules
        .limitations
        .iter()
        .find(|l| l.kind == LimitationKind::InspectionFailed)
        .unwrap()
        .examples[0];
    assert_eq!(example.detail.as_deref(), Some("GetProcessTimes failed"));
}

#[test]
fn a_process_that_exits_before_it_is_opened_is_not_a_candidate() {
    let scene = scene();
    let mut fake = Fake::default().with(OTHER_PID, Scripted::new());
    // Listed, but gone by the time it is opened.
    fake.pids = Some(Ok(vec![PID, OTHER_PID]));
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    let enumeration = coverage(&report, Mechanism::ProcessEnumeration);
    assert_eq!(enumeration.status, CoverageStatus::Complete);
    assert_eq!(enumeration.attempted, Some(1));
    assert_eq!(coverage(&report, Mechanism::LoadedModules).status, CoverageStatus::Complete);
}

#[test]
fn a_failed_process_list_leaves_modules_not_attempted() {
    let scene = scene();
    let mut fake = Fake {
        pids: Some(Err("the process list could not be read: boom".to_string())),
        ..Fake::default()
    };
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    let enumeration = coverage(&report, Mechanism::ProcessEnumeration);
    assert_eq!(enumeration.status, CoverageStatus::Failed);
    assert_eq!(enumeration.reason.as_deref(), Some("the process list could not be read: boom"));
    assert_eq!(coverage(&report, Mechanism::LoadedModules).status, CoverageStatus::NotAttempted);
    assert_eq!(report.outcome, Outcome::Unavailable);
}

#[test]
fn module_read_faults_inside_a_listed_set_are_counted() {
    let scene = scene();
    let tool = scene.root.join("bin").join("tool.exe");
    let mut fake = Fake::default().with(
        PID,
        Scripted::new()
            .module(&tool)
            .module_fault(Fault::Denied)
            .module_fault(Fault::Failed("boom".to_string())),
    );
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    assert_eq!(module_paths(&report.processes[0]), vec![indexed(&tool)]);
    let modules = coverage(&report, Mechanism::LoadedModules);
    assert_eq!(modules.status, CoverageStatus::Partial, "{modules:#?}");
    assert_eq!(limitation_count(modules, LimitationKind::PermissionDenied), 1);
    assert_eq!(limitation_count(modules, LimitationKind::InspectionFailed), 1);
}

#[test]
fn a_module_unloaded_since_the_listing_leaves_coverage_complete() {
    let scene = scene();
    let mut fake = Fake::default().with(PID, Scripted::new().module_fault(Fault::Gone));
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    let modules = coverage(&report, Mechanism::LoadedModules);
    assert_eq!(modules.status, CoverageStatus::Complete, "{modules:#?}");
    assert!(modules.limitations.is_empty());
}

#[test]
fn a_module_failure_after_the_process_exits_is_a_disappearance_not_a_failure() {
    let scene = scene();
    let mut exited = Scripted::new();
    exited.modules = Err(Fault::Failed("Only part of a ReadProcessMemory request was completed".to_string()));
    exited.exited = true;
    let mut exited_mid_read = Scripted::new().module_fault(Fault::Failed("partial copy".to_string()));
    exited_mid_read.exited = true;
    let mut fake = Fake::default().with(PID, exited).with(OTHER_PID, exited_mid_read);
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    let modules = coverage(&report, Mechanism::LoadedModules);
    assert_eq!(limitation_count(modules, LimitationKind::ProcessDisappeared), 2, "{modules:#?}");
    assert_eq!(limitation_count(modules, LimitationKind::InspectionFailed), 0);

    // The same failure from a running process is an inspection failure.
    let mut running = Scripted::new();
    running.modules = Err(Fault::Failed("partial copy".to_string()));
    let mut fake = Fake::default().with(PID, running);
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    let modules = coverage(&report, Mechanism::LoadedModules);
    assert_eq!(modules.status, CoverageStatus::Failed);
    assert_eq!(limitation_count(modules, LimitationKind::InspectionFailed), 1);
}

#[test]
fn handles_close_after_inspection_unless_the_process_holds_something_in_scope() {
    let scene = scene();
    let tool = scene.root.join("bin").join("tool.exe");
    let mut fake = Fake::default()
        .with(PID, Scripted::new().image(&tool))
        .with(OTHER_PID, Scripted::new().module(&scene.sibling.join("tool.exe")));
    let budget = Budget::start(Duration::from_secs(60)).unwrap();
    let closed = Rc::clone(&fake.closed);
    {
        let mut backend = ModuleBackend::with_source(&mut fake);
        let report = run(&scene.root, &PathUsageOptions::default(), &mut backend, &budget, chrono::Utc::now())
            .expect("report");
        assert_eq!(report.processes.len(), 1);
        // The holder's handle outlives inspection and enrichment.
        assert_eq!(*closed.borrow(), vec![OTHER_PID]);
    }
    assert_eq!(*closed.borrow(), vec![OTHER_PID, PID]);
    // Enrichment proved the lifetime through the held handle, never by
    // opening the PID again.
    assert_eq!(fake.opens.get(&PID), Some(&1));
    assert!(fake.token_reads.get(&PID).is_some_and(|&n| n >= 1));
}

#[test]
fn a_changed_creation_time_at_enrichment_keeps_evidence_without_new_identity() {
    let scene = scene();
    let tool = scene.root.join("bin").join("tool.exe");
    let mut process = Scripted::new().module(&tool);
    process.open = Ok((None, true));
    process.tokens = vec![Ok(CREATED + 1)];
    let mut fake = Fake::default().with(PID, process);
    let report = fake.query(&scene.root, &PathUsageOptions::default());
    let record = &report.processes[0];
    assert_eq!(module_paths(record), vec![indexed(&tool)]);
    assert_eq!(record.name, None);
    assert_eq!(record.user, None);
    assert!(
        report
            .limitations
            .iter()
            .any(|l| l.kind == LimitationKind::ProcessDisappeared),
        "{:#?}",
        report.limitations
    );
}

#[test]
fn target_only_matches_a_module_that_is_the_target_file() {
    let scene = scene();
    let tool = scene.root.join("bin").join("tool.exe");
    let mut fake = Fake::default().with(
        PID,
        Scripted::new().image(&tool).module(&scene.root.join("bin").join("helper.dll")),
    );
    let report = fake.query(&tool, &PathUsageOptions::default().target_only());
    assert_eq!(module_paths(&report.processes[0]), vec![indexed(&tool)]);

    let report = fake.query(&scene.root, &PathUsageOptions::default().target_only());
    assert!(report.processes.is_empty(), "{:#?}", report.processes);
}

#[test]
fn a_budget_expiring_between_modules_keeps_what_was_read() {
    let scene = scene();
    let mut fake = Fake::default().with(
        PID,
        Scripted::new()
            .module(&scene.root.join("bin").join("tool.exe"))
            .module(&scene.root.join("bin").join("helper.dll")),
    );
    // The first per-module check passes; the second finds it expired.
    let budget = Budget::expiring_after_checks(1);
    let (_, inspections) = inspect_directly(&scene, &mut fake, &budget);
    assert_eq!(inspections[0].result, InspectionResult::Partial);
    assert_eq!(inspections[0].evidence.len(), 1);
    assert!(
        inspections[0]
            .limitations
            .iter()
            .any(|l| l.kind == LimitationKind::BudgetExhausted)
    );
}

#[test]
fn creation_filetimes_convert_to_start_times() {
    let start = win32::filetime_to_utc(CREATED).unwrap();
    assert_eq!(start.to_rfc3339(), "2023-11-14T22:13:20+00:00");
    let fraction = win32::filetime_to_utc(CREATED + 1).unwrap();
    assert_eq!(fraction.timestamp_subsec_nanos(), 100);
    // Before the Unix epoch, and the zero FILETIME, have no start time.
    assert_eq!(win32::filetime_to_utc(0), None);
}

/// Enumerates and inspects every scripted process directly.
fn inspect_directly(
    scene: &Scene,
    fake: &mut Fake,
    inspection_budget: &Budget,
) -> (Vec<ProcessCandidate<Option<Fault>>>, Vec<MechanismInspection>) {
    let budget = Budget::start(Duration::from_secs(60)).unwrap();
    let root = super::root::resolve(&scene.root, &budget).unwrap();
    let tree = super::tree::collect(&root, true, &budget);
    let mut backend = ModuleBackend::with_source(fake);
    let enumeration = backend.enumerate(&budget).unwrap();
    let context = InspectionContext {
        budget: inspection_budget,
        root: &root,
        index: &tree.index,
    };
    let inspections = enumeration
        .processes
        .iter()
        .flat_map(|candidate| backend.inspect(candidate, &context))
        .collect();
    (enumeration.processes, inspections)
}


/// Windows spellings of an in-tree module path, resolved by lookup.
#[cfg(windows)]
mod spellings {
    use super::*;
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    fn short_name(path: &Path) -> Option<PathBuf> {
        use windows::Win32::Storage::FileSystem::GetShortPathNameW;
        use windows::core::PCWSTR;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut buffer = vec![0u16; 1024];
        // SAFETY: a NUL-terminated input and a sized output slice.
        let written = unsafe { GetShortPathNameW(PCWSTR(wide.as_ptr()), Some(&mut buffer)) } as usize;
        (written > 0 && written < buffer.len())
            .then(|| PathBuf::from(OsString::from_wide(&buffer[..written])))
    }

    #[test]
    fn verbatim_case_variant_and_short_name_module_paths_match_the_long_path() {
        let scene = scene();
        let long_dir = scene.root.join("Long Directory Name");
        std::fs::create_dir_all(&long_dir).unwrap();
        let module = long_dir.join("Module With Long Name.dll");
        std::fs::write(&module, "x").unwrap();

        let mut spellings = vec![
            PathBuf::from(format!(r"\\?\{}", module.display())),
            PathBuf::from(module.to_string_lossy().to_uppercase()),
        ];
        // Volumes without 8.3 names (ReFS, or NTFS with them disabled) return
        // the long name; the other spellings still run.
        if let Some(short) = short_name(&module).filter(|short| *short != module) {
            spellings.push(short);
        }
        for spelling in spellings {
            let mut fake = Fake::default().with(PID, Scripted::new().module(&spelling));
            let report = fake.query(&scene.root, &PathUsageOptions::default());
            let record = report
                .processes
                .first()
                .unwrap_or_else(|| panic!("{} did not match", spelling.display()));
            assert_eq!(module_paths(record), vec![indexed(&module)], "{}", spelling.display());
            assert_eq!(
                record.evidence[0].observed_path.as_ref().map(|p| p.as_path()),
                Some(spelling.as_path())
            );
        }
    }

    #[test]
    fn a_case_sensitive_directory_matches_only_the_module_its_spelling_names() {
        let scene = scene();
        let directory = scene.root.join("cs");
        std::fs::create_dir(&directory).unwrap();
        let status = std::process::Command::new("fsutil.exe")
            .args(["file", "setCaseSensitiveInfo"])
            .arg(&directory)
            .arg("enable")
            .stdout(std::process::Stdio::null())
            .status()
            .expect("run fsutil.exe");
        // A skip would read as a pass, so a host that cannot enable
        // per-directory case sensitivity fails here instead.
        assert!(status.success(), "fsutil could not enable case sensitivity on {}", directory.display());
        let upper = directory.join("Module.dll");
        let lower = directory.join("module.dll");
        std::fs::write(&upper, "upper").unwrap();
        std::fs::write(&lower, "lower").unwrap();

        let mut fake = Fake::default().with(
            PID,
            Scripted::new()
                .module(&lower)
                // Names neither file in a case-sensitive directory.
                .module(&directory.join("MODULE.DLL")),
        );
        let report = fake.query(&scene.root, &PathUsageOptions::default());
        assert_eq!(module_paths(&report.processes[0]), vec![indexed(&lower)]);
        assert_ne!(indexed(&lower), indexed(&upper));
        assert_eq!(coverage(&report, Mechanism::LoadedModules).status, CoverageStatus::Complete);
    }

    #[test]
    fn a_module_path_with_an_unpaired_surrogate_matches_and_round_trips() {
        let scene = scene();
        let mut units: Vec<u16> = "lone-".encode_utf16().collect();
        units.push(0xD800);
        units.extend(".dll".encode_utf16());
        let module = scene.root.join("bin").join(OsString::from_wide(&units));
        std::fs::write(&module, "x").unwrap();

        let mut fake = Fake::default().with(PID, Scripted::new().module(&module));
        let report = fake.query(&scene.root, &PathUsageOptions::default());
        let record = &report.processes[0];
        assert_eq!(module_paths(record), vec![indexed(&module)]);
        let json = serde_json::to_string(&report).unwrap();
        let read: PathUsageReport = serde_json::from_str(&json).unwrap();
        assert_eq!(read, report);
        let observed = read.processes[0].evidence[0].observed_path.as_ref().unwrap();
        assert_eq!(observed.as_path(), module);
    }
}

/// Live Win32 calls on Windows.
#[cfg(windows)]
mod live {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::process::{Child, Command, Stdio};

    /// Sits with its executable loaded until its stdin closes.
    #[test]
    #[ignore = "subprocess fixture invoked by the live Win32 tests"]
    fn usage_child() {
        println!("ready");
        std::io::stdout().flush().unwrap();
        let mut sink = Vec::new();
        let _ = std::io::stdin().read_to_end(&mut sink);
    }

    /// Kills and reaps the child however the test ends.
    struct ChildGuard(Child);

    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// Runs a copy of this test executable from `image`, so the child's own
    /// executable is a module loaded from inside the tree.
    fn spawn_child(image: &Path) -> ChildGuard {
        let current = std::env::current_exe().expect("test executable");
        if std::fs::hard_link(&current, image).is_err() {
            std::fs::copy(&current, image).expect("copy the test executable");
        }
        let mut command = Command::new(image);
        command
            .args([
                "filesystem::query::win32_tests::live::usage_child",
                "--exact",
                "--ignored",
                "--nocapture",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = ChildGuard(command.spawn().expect("spawn the usage child"));
        let stdout = child.0.stdout.take().unwrap();
        for line in BufReader::new(stdout).lines() {
            if line.expect("child stdout") == "ready" {
                return child;
            }
        }
        panic!("the usage child exited before it was ready");
    }

    fn live_query(path: &Path) -> PathUsageReport {
        let options = PathUsageOptions::default().with_deadline(Duration::from_secs(120));
        query_path_usage(path, &options).expect("report")
    }

    #[test]
    fn a_controlled_child_is_found_by_its_loaded_executable() {
        let scene = scene();
        let image = scene.root.join("bin").join("usage-child.exe");
        let child = spawn_child(&image);
        let child_pid = child.0.id();

        let report = live_query(&scene.root);
        assert_eq!(report.outcome, Outcome::Usable);
        let records: Vec<&ProcessRecord> =
            report.processes.iter().filter(|p| p.pid == child_pid).collect();
        assert_eq!(records.len(), 1, "one record per process: {:#?}", report.processes);
        let record = records[0];
        assert!(record.creation_token.is_some());
        assert!(record.start_time.is_some());
        assert!(!record.identity_uncertain);
        assert_eq!(record.name.as_ref().map(|n| n.as_os_str().to_owned()), Some("usage-child.exe".into()));
        assert_eq!(module_paths(record), vec![indexed(&image)]);
        let evidence = &record.evidence[0];
        assert_eq!(evidence.kind, EvidenceKind::LoadedModule);
        assert_eq!(evidence.match_basis, MatchBasis::PathLookup);
        assert_eq!(evidence.identity, Some(super::super::identity::of_path(&image, true).unwrap()));
        drop(child);
    }

    #[test]
    fn the_shipped_windows_backend_inventories_loaded_modules() {
        let scene = scene();
        let report = live_query(&scene.root);
        assert_eq!(report.outcome, Outcome::Usable);
        let enumeration = coverage(&report, Mechanism::ProcessEnumeration);
        assert!(enumeration.attempted.is_some_and(|n| n > 1), "{enumeration:#?}");
        let modules = coverage(&report, Mechanism::LoadedModules);
        assert!(modules.succeeded.is_some_and(|n| n > 0), "{modules:#?}");
        // The System process's modules are never readable, so some gap is
        // always recorded rather than read as "no modules".
        assert_eq!(modules.status, CoverageStatus::Partial, "{modules:#?}");
        assert!(!modules.limitations.is_empty());
        for mechanism in [
            Mechanism::OpenHandles,
            Mechanism::WorkingDirectories,
            Mechanism::DirectoryChangeSubscriptions,
            Mechanism::Polling,
        ] {
            let record = coverage(&report, mechanism);
            assert_eq!(record.status, CoverageStatus::Unsupported, "{mechanism:?}");
            assert!(record.reason.as_deref().is_some_and(|r| !r.is_empty()));
        }
    }
}
