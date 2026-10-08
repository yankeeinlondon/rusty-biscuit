//! The Linux `/proc` backend, driven through the full query.
//!
//! On every Unix host the backend reads a synthetic `/proc` tree whose
//! `fd/<n>` and `cwd` entries are symlinks, which `stat` and `readlink` treat
//! like the kernel's magic links. On Linux, controlled children exercise the
//! live `/proc`.

use super::backend::{InspectionContext, UsageBackend};
use super::budget::Budget;
use super::identity;
use super::linux::{ProcBackend, st_dev_from_kernel};
use super::*;
use crate::performance::counters;
use crate::performance::testing::measure;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

/// A real inotify `fdinfo` captured on build-linux (kernel 7.0): a process
/// watching two directories and their parent.
const FIXTURE: &str = include_str!("fixtures/inotify-fdinfo.txt");

/// Above every Linux and macOS `pid_max`, so enrichment through `sysinfo`
/// can never name a real host process.
const PID: u32 = 4_100_000_001;
const START: u64 = 77_001;

struct Scene {
    _temp: TempDir,
    base: PathBuf,
    root: PathBuf,
    sibling: PathBuf,
    proc_root: PathBuf,
}

fn scene() -> Scene {
    let temp = TempDir::new().expect("tempdir");
    let base = temp.path().canonicalize().expect("canonical tempdir");
    let root = base.join("app");
    let sibling = base.join("app-copy");
    let proc_root = base.join("proc");
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::create_dir_all(&sibling).unwrap();
    std::fs::create_dir_all(&proc_root).unwrap();
    std::fs::write(root.join("file.txt"), "x").unwrap();
    std::fs::write(root.join("sub/b.txt"), "b").unwrap();
    std::fs::write(sibling.join("file.txt"), "y").unwrap();
    Scene {
        _temp: temp,
        base,
        root,
        sibling,
        proc_root,
    }
}

struct FakeProcess {
    dir: PathBuf,
}

impl Scene {
    fn process(&self, pid: u32, start: u64, comm: &str) -> FakeProcess {
        let dir = self.proc_root.join(pid.to_string());
        std::fs::create_dir_all(dir.join("fd")).unwrap();
        std::fs::create_dir_all(dir.join("fdinfo")).unwrap();
        let process = FakeProcess { dir };
        process.set_stat(&stat_line(pid, comm, start));
        process
    }

    fn backend(&self) -> ProcBackend {
        ProcBackend::with_proc_root(self.proc_root.clone())
    }

    fn query(&self, path: &Path, options: &PathUsageOptions) -> PathUsageReport {
        let budget = Budget::start(Duration::from_secs(60)).unwrap();
        run(path, options, &mut self.backend(), &budget, chrono::Utc::now()).expect("report")
    }
}

/// `/proc/<pid>/stat` with `starttime` as field 22.
fn stat_line(pid: u32, comm: &str, start: u64) -> String {
    format!("{pid} ({comm}) S{} {start} 0 0\n", " 0".repeat(18))
}

impl FakeProcess {
    fn set_stat(&self, text: &str) {
        std::fs::write(self.dir.join("stat"), text).unwrap();
    }

    /// Descriptor `fd` open on `target` with octal open `flags`.
    fn open(&self, fd: u64, target: &Path, flags: &str) {
        symlink(target, self.dir.join("fd").join(fd.to_string())).unwrap();
        let inode = std::fs::metadata(target).map(|m| m.ino()).unwrap_or(0);
        self.fdinfo(fd, &format!("pos:\t0\nflags:\t{flags}\nmnt_id:\t1\nino:\t{inode}\n"));
    }

    /// An anonymous-inode descriptor whose link text is `anon_inode:<kind>`.
    fn anon(&self, fd: u64, kind: &str, fdinfo: &str) {
        let name = format!("anon_inode:{kind}");
        std::fs::write(self.dir.join("fd").join(&name), "").unwrap();
        symlink(&name, self.dir.join("fd").join(fd.to_string())).unwrap();
        self.fdinfo(fd, fdinfo);
    }

    fn fdinfo(&self, fd: u64, text: &str) {
        std::fs::write(self.dir.join("fdinfo").join(fd.to_string()), text).unwrap();
    }

    fn cwd(&self, target: &Path) {
        symlink(target, self.dir.join("cwd")).unwrap();
    }
}

/// The inverse of [`st_dev_from_kernel`] for the host's `st_dev`, so a
/// synthetic registration can name a real tree object on any Unix host.
fn kernel_sdev(st_dev: u64) -> u64 {
    let major = ((st_dev >> 8) & 0xfff) | ((st_dev >> 32) & !0xfff);
    let minor = (st_dev & 0xff) | ((st_dev >> 12) & !0xff);
    assert!(major < 1 << 12 && minor < 1 << 20, "st_dev {st_dev:#x} has no kernel form");
    (major << 20) | minor
}

/// `ino:<hex> sdev:<hex>` naming `path`.
fn watch_fields(path: &Path) -> String {
    let metadata = std::fs::metadata(path).unwrap();
    format!("ino:{:x} sdev:{:x}", metadata.ino(), kernel_sdev(metadata.dev()))
}

fn registration(wd: i64, path: &Path) -> String {
    format!("inotify wd:{wd} {} mask:fff ignored_mask:0\n", watch_fields(path))
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

fn watch_ids(report: &PathUsageReport) -> Vec<i64> {
    let mut ids: Vec<i64> = evidence(report, EvidenceKind::WatchRegistration)
        .iter()
        .filter_map(|e| e.watch.as_ref().and_then(|w| w.watch_id))
        .collect();
    ids.sort();
    ids
}

#[test]
fn kernel_device_numbers_are_normalized_to_stat_encoding() {
    // Major 0 with a small minor: the only case where both encodings agree.
    assert_eq!(st_dev_from_kernel(0x2e), Some(0x2e));
    // /dev/sdb1 is 8:17: kernel `sdev:800011`, `st_dev` 0x811.
    assert_eq!(st_dev_from_kernel(0x0080_0011), Some(0x811));
    // A minor above 255 splits around the major.
    assert_eq!(st_dev_from_kernel((259 << 20) | 0x1_2345), Some(0x1231_0345));
    // Wider than the kernel's 32-bit dev_t.
    assert_eq!(st_dev_from_kernel(1 << 32), None);
    #[cfg(target_os = "linux")]
    for (major, minor) in [(0u32, 46u32), (8, 17), (259, 0x1_2345), (0xfff, 0xf_ffff)] {
        let sdev = (u64::from(major) << 20) | u64::from(minor);
        assert_eq!(st_dev_from_kernel(sdev), Some(libc::makedev(major, minor)));
    }
}

/// The fixture with its first two registrations aimed at the tree:
/// `wd:3` names `file.txt` and `wd:2` names the root directory. `wd:1` keeps
/// its captured, out-of-scope identity.
fn matrix_base(scene: &Scene) -> String {
    let base = FIXTURE
        .replace("ino:b84b5 sdev:2e", &watch_fields(&scene.root.join("file.txt")))
        .replace("ino:b84b8 sdev:2e", &watch_fields(&scene.root));
    assert_ne!(base, FIXTURE, "the fixture no longer has its captured registrations");
    base
}

/// Rewrites `field` on every registration line selected by `select`;
/// `None` removes the field.
fn edit_field(text: &str, select: &str, field: &str, value: Option<&str>) -> String {
    edit_lines(text, select, |line| {
        line.split(' ')
            .filter_map(|token| match token.split_once(':') {
                Some((key, _)) if key == field => value.map(|v| format!("{field}:{v}")),
                _ => Some(token.to_string()),
            })
            .collect::<Vec<_>>()
            .join(" ")
    })
}

fn edit_lines(text: &str, select: &str, edit: impl Fn(&str) -> String) -> String {
    text.lines()
        .map(|line| {
            if line.starts_with(select) {
                edit(line)
            } else {
                line.to_string()
            }
        })
        .map(|line| line + "\n")
        .collect()
}

/// Inserts an edited copy of the `wd:3` line before it.
fn repeat_wd3(text: &str, edit: impl Fn(&str) -> String) -> String {
    edit_lines(text, WD3, |line| format!("{}\n{line}", edit(line)))
}

const WD3: &str = "inotify wd:3 ";
const EVERY: &str = "inotify ";

struct Cell {
    name: &'static str,
    text: String,
    wd3: usize,
    wd2: usize,
    status: CoverageStatus,
    malformed: u64,
}

#[test]
fn the_fdinfo_reader_gives_every_malformed_shape_a_defined_outcome() {
    use CoverageStatus::*;
    let scene = scene();
    let base = matrix_base(&scene);
    let cell = |name, text: String, wd3, wd2, status, malformed| Cell {
        name,
        text,
        wd3,
        wd2,
        status,
        malformed,
    };
    let mut cells = vec![
        cell("control", base.clone(), 1, 1, Complete, 0),
        cell("unknown field", edit_lines(&base, WD3, |l| format!("{l} foo:1")), 1, 1, Complete, 0),
        cell("identical registration repeated", repeat_wd3(&base, |l| l.to_string()), 1, 1, Complete, 0),
        cell("same wd, different mask", repeat_wd3(&base, |l| l.replace("mask:fff", "mask:2")), 2, 1, Complete, 0),
        cell("garbage line", format!("{base}zzz\n"), 1, 1, Partial, 1),
        cell("trailing token without a colon", edit_lines(&base, WD3, |l| format!("{l} garbage")), 0, 1, Partial, 1),
        cell("duplicate ino, first", edit_lines(&base, WD3, |l| l.replace(" ino:", " ino:1 ino:")), 0, 1, Partial, 1),
        cell("duplicate ino, last", edit_lines(&base, WD3, |l| format!("{l} ino:1")), 0, 1, Partial, 1),
        cell("duplicate wd", edit_lines(&base, WD3, |l| format!("{l} wd:3")), 0, 1, Partial, 1),
        cell("ino wider than 64 bits", edit_field(&base, WD3, "ino", Some("1ffffffffffffffff")), 0, 1, Partial, 1),
        cell("sdev wider than dev_t", edit_field(&base, WD3, "sdev", Some("100000000")), 0, 1, Partial, 1),
        cell("signed wd", edit_field(&base, WD3, "wd", Some("+3")), 0, 1, Partial, 1),
        cell("wrong type, every line", edit_field(&base, EVERY, "ino", Some("zz")), 0, 0, Failed, 3),
        cell(
            "no registration lines",
            "pos:\t0\nflags:\t02004000\nmnt_id:\t19\nino:\t1070\n".to_string(),
            0,
            0,
            Complete,
            0,
        ),
        cell("zero-length record", String::new(), 0, 0, Failed, 1),
    ];
    for field in ["wd", "ino", "sdev", "mask"] {
        let (absent, empty, wrong) = match field {
            "wd" => ("wd absent", "wd empty", "wd not decimal"),
            "ino" => ("ino absent", "ino empty", "ino not hex"),
            "sdev" => ("sdev absent", "sdev empty", "sdev not hex"),
            _ => ("mask absent", "mask empty", "mask not hex"),
        };
        let bad = if field == "wd" { "x" } else { "zz" };
        cells.push(cell(absent, edit_field(&base, WD3, field, None), 0, 1, Partial, 1));
        cells.push(cell(empty, edit_field(&base, WD3, field, Some("")), 0, 1, Partial, 1));
        cells.push(cell(wrong, edit_field(&base, WD3, field, Some(bad)), 0, 1, Partial, 1));
    }

    for (index, cell) in cells.iter().enumerate() {
        let pid = PID + index as u32;
        scene.process(pid, START, "watcher").anon(9, "inotify", &cell.text);
        let report = scene.query(&scene.root, &PathUsageOptions::default());
        std::fs::remove_dir_all(scene.proc_root.join(pid.to_string())).unwrap();

        let ids = watch_ids(&report);
        let name = cell.name;
        assert_eq!(ids.iter().filter(|&&id| id == 3).count(), cell.wd3, "{name}: wd 3 in {ids:?}");
        assert_eq!(ids.iter().filter(|&&id| id == 2).count(), cell.wd2, "{name}: wd 2 in {ids:?}");
        let inotify = coverage(&report, Mechanism::Inotify);
        assert_eq!(inotify.status, cell.status, "{name}: {inotify:#?}");
        assert_eq!(
            limitation_count(inotify, LimitationKind::MalformedRecord),
            cell.malformed,
            "{name}: {inotify:#?}"
        );
        if cell.wd3 > 0 {
            let watch = evidence(&report, EvidenceKind::WatchRegistration)
                .into_iter()
                .find(|e| e.watch.as_ref().and_then(|w| w.watch_id) == Some(3))
                .unwrap();
            assert_eq!(paths(watch), vec![scene.root.join("file.txt")], "{name}");
            assert_eq!(watch.descriptor, Some(9), "{name}");
            assert_eq!(watch.observed_path, None, "{name}: a watch never gains a path");
            assert_eq!(watch.watch.as_ref().unwrap().recursive, Some(false), "{name}");
        }
        // A broken inotify record never hides the process's other mechanisms.
        assert_eq!(coverage(&report, Mechanism::OpenHandles).status, Complete, "{name}");
        assert_eq!(report.outcome, Outcome::Usable, "{name}");
    }
}

#[test]
fn a_watch_preserves_its_mask_descriptor_and_identity() {
    let scene = scene();
    let file = scene.root.join("file.txt");
    let text = format!("pos:\t0\nflags:\t02004000\n{}{}", registration(4, &file), registration(5, &scene.root));
    scene.process(PID, START, "watcher").anon(11, "inotify", &text);
    let report = scene.query(&scene.root, &PathUsageOptions::default());
    let watches = evidence(&report, EvidenceKind::WatchRegistration);
    assert_eq!(watches.len(), 2, "{watches:#?}");
    for watch in &watches {
        assert_eq!(watch.mechanism, Mechanism::Inotify);
        assert_eq!(watch.descriptor, Some(11));
        assert_eq!(watch.match_basis, MatchBasis::Identity);
        assert_eq!(watch.watch.as_ref().unwrap().mask, Some(0xfff));
    }
    let file_watch = watches.iter().find(|e| paths(e) == vec![file.clone()]).unwrap();
    assert_eq!(file_watch.identity, Some(identity::of_path(&file, true).unwrap()));
    assert_eq!(watch_ids(&report), vec![4, 5]);
}

#[test]
fn open_descriptors_and_the_cwd_are_matched_by_identity_with_access_flags() {
    let scene = scene();
    let process = scene.process(PID, START, "editor");
    let file = scene.root.join("file.txt");
    let b = scene.root.join("sub/b.txt");
    process.open(3, &file, "0100002");
    process.open(4, &b, "0100000");
    process.open(5, &file, "0100001");
    process.open(6, &scene.root, "010200000");
    process.open(7, &scene.sibling.join("file.txt"), "0100002");
    process.cwd(&scene.root.join("sub"));
    let report = scene.query(&scene.root, &PathUsageOptions::default());

    assert_eq!(report.processes.len(), 1);
    let record = &report.processes[0];
    assert_eq!(record.pid, PID);
    assert_eq!(record.creation_token, Some(START));
    assert_eq!(record.name.as_ref().map(|n| n.display().into_owned()), Some("editor".to_string()));
    let handle = |fd| {
        record
            .evidence
            .iter()
            .find(|e| e.kind == EvidenceKind::OpenHandle && e.descriptor == Some(fd))
            .unwrap_or_else(|| panic!("no fd {fd} in {:#?}", record.evidence))
    };
    let access = |read, write| Some(Access { read, write });
    assert_eq!(handle(3).access, access(true, true));
    assert_eq!(handle(4).access, access(true, false));
    assert_eq!(handle(5).access, access(false, true));
    assert_eq!(handle(6).access, access(false, false), "O_PATH reads and writes nothing");
    assert_eq!(paths(handle(4)), vec![b.clone()]);
    assert_eq!(handle(4).observed_path.as_ref().map(|p| p.as_path().to_path_buf()), Some(b));
    assert!(
        !record.evidence.iter().any(|e| e.descriptor == Some(7)),
        "a similarly prefixed sibling is outside the tree"
    );
    let cwd = evidence(&report, EvidenceKind::WorkingDirectory);
    assert_eq!(cwd.len(), 1);
    assert_eq!(paths(cwd[0]), vec![scene.root.join("sub")]);
    assert_eq!(cwd[0].mechanism, Mechanism::WorkingDirectories);
    for mechanism in [Mechanism::OpenHandles, Mechanism::WorkingDirectories, Mechanism::Inotify] {
        assert_eq!(coverage(&report, mechanism).status, CoverageStatus::Complete, "{mechanism:?}");
    }
    for mechanism in [Mechanism::Fanotify, Mechanism::Polling] {
        let record = coverage(&report, mechanism);
        assert_eq!(record.status, CoverageStatus::Unsupported);
        assert!(record.reason.as_deref().is_some_and(|r| !r.is_empty()));
    }
    assert_eq!(report.outcome, Outcome::Usable);
}

#[test]
fn a_hard_link_outside_the_tree_reports_the_in_scope_path_and_the_alias() {
    let scene = scene();
    let file = scene.root.join("file.txt");
    let alias = scene.base.join("alias.txt");
    std::fs::hard_link(&file, &alias).unwrap();
    scene.process(PID, START, "holder").open(3, &alias, "0100000");
    let report = scene.query(&scene.root, &PathUsageOptions::default());
    let handles = evidence(&report, EvidenceKind::OpenHandle);
    assert_eq!(handles.len(), 1);
    assert_eq!(paths(handles[0]), vec![file]);
    assert_eq!(handles[0].observed_path.as_ref().unwrap().as_path(), alias);
}

#[test]
fn a_watch_on_an_ancestor_is_reported_as_outside_the_query() {
    let scene = scene();
    let text = format!("pos:\t0\n{}{}", registration(1, &scene.base), registration(2, &scene.sibling));
    scene.process(PID, START, "watcher").anon(5, "inotify", &text);
    let report = scene.query(&scene.root, &PathUsageOptions::default());
    assert!(report.processes.is_empty(), "{:#?}", report.processes);
    let inotify = coverage(&report, Mechanism::Inotify);
    assert_eq!(limitation_count(inotify, LimitationKind::AncestorWatch), 1, "{inotify:#?}");
    let example = &inotify
        .limitations
        .iter()
        .find(|l| l.kind == LimitationKind::AncestorWatch)
        .unwrap()
        .examples[0];
    assert_eq!(example.pid, Some(PID));
    // The query's own scope was inspected completely; the sibling's watch is
    // simply out of scope.
    assert_eq!(inotify.status, CoverageStatus::Complete);
}

#[test]
fn target_only_keeps_a_watch_of_the_target_and_ignores_descendants() {
    let scene = scene();
    let text = format!(
        "pos:\t0\n{}{}",
        registration(1, &scene.root),
        registration(2, &scene.root.join("file.txt"))
    );
    let process = scene.process(PID, START, "watcher");
    process.anon(5, "inotify", &text);
    process.cwd(&scene.root.join("sub"));
    let report = scene.query(&scene.root, &PathUsageOptions::default().target_only());
    assert_eq!(watch_ids(&report), vec![1]);
    assert!(evidence(&report, EvidenceKind::WorkingDirectory).is_empty());
    let inotify = coverage(&report, Mechanism::Inotify);
    assert_eq!(inotify.status, CoverageStatus::Complete, "{inotify:#?}");
}

#[test]
fn a_watch_on_an_in_scope_inode_of_another_device_is_a_visible_limitation() {
    let scene = scene();
    let file = scene.root.join("file.txt");
    let metadata = std::fs::metadata(&file).unwrap();
    let other_device = kernel_sdev(metadata.dev()) ^ 1;
    let text = format!("inotify wd:1 ino:{:x} sdev:{other_device:x} mask:2\n", metadata.ino());
    scene.process(PID, START, "watcher").anon(5, "inotify", &text);
    let report = scene.query(&scene.root, &PathUsageOptions::default());
    assert!(report.processes.is_empty(), "never matched on inode alone");
    let inotify = coverage(&report, Mechanism::Inotify);
    assert_eq!(limitation_count(inotify, LimitationKind::DeviceIdentityUnreliable), 1);
    assert_eq!(inotify.status, CoverageStatus::Partial);
}

#[test]
fn other_anonymous_descriptors_are_read_but_carry_no_registrations() {
    let scene = scene();
    let process = scene.process(PID, START, "server");
    process.anon(3, "[eventfd]", "pos:\t0\nflags:\t02\nmnt_id:\t15\nino:\t1057\neventfd-count:  0\n");
    process.anon(4, "[eventpoll]", "pos:\t0\nflags:\t02\ntfd:        5 events:       19 data:                0  pos:0 ino:3 sdev:f\n");
    process.anon(5, "inotify", &registration(1, &scene.root));
    process.open(6, &scene.sibling.join("file.txt"), "0100000");
    let (report, counts) = measure(|| scene.query(&scene.root, &PathUsageOptions::default()));
    assert_eq!(watch_ids(&report), vec![1]);
    let inotify = coverage(&report, Mechanism::Inotify);
    assert_eq!(inotify.status, CoverageStatus::Complete, "{inotify:#?}");
    assert_eq!(counts.get(counters::QUERY_DESCRIPTOR_INSPECTIONS), 4);
    assert_eq!(counts.get(counters::QUERY_WATCH_REGISTRATION_READS), 3);
}

#[test]
fn a_descriptor_replaced_during_inspection_is_dropped_and_reported() {
    let scene = scene();
    let process = scene.process(PID, START, "racer");
    let file = scene.root.join("file.txt");
    process.open(3, &file, "0100000");
    let stale = std::fs::metadata(&file).unwrap().ino() + 1;
    process.fdinfo(3, &format!("pos:\t0\nflags:\t0100000\nmnt_id:\t1\nino:\t{stale}\n"));
    process.open(4, &scene.root.join("sub/b.txt"), "0100000");
    let report = scene.query(&scene.root, &PathUsageOptions::default());
    let handles = evidence(&report, EvidenceKind::OpenHandle);
    assert_eq!(handles.iter().map(|e| e.descriptor).collect::<Vec<_>>(), vec![Some(4)]);
    let coverage = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(limitation_count(coverage, LimitationKind::DescriptorReplaced), 1);
    assert_eq!(coverage.status, CoverageStatus::Partial);
}

#[test]
fn only_numeric_proc_entries_are_processes_and_comm_may_hold_parentheses() {
    let scene = scene();
    std::fs::create_dir(scene.proc_root.join("sys")).unwrap();
    std::fs::create_dir(scene.proc_root.join("123abc")).unwrap();
    symlink(scene.proc_root.join(PID.to_string()), scene.proc_root.join("self")).unwrap();
    scene.process(PID, START, "a) b (c").open(3, &scene.root.join("file.txt"), "0100000");
    let uncertain = scene.process(PID + 1, START, "x");
    uncertain.set_stat(&format!("{} (x) S 1 2\n", PID + 1));
    uncertain.open(3, &scene.root.join("file.txt"), "0100000");
    let report = scene.query(&scene.root, &PathUsageOptions::default());

    let enumeration = coverage(&report, Mechanism::ProcessEnumeration);
    assert_eq!(enumeration.attempted, Some(2), "{enumeration:#?}");
    assert_eq!(enumeration.status, CoverageStatus::Partial);
    assert_eq!(limitation_count(enumeration, LimitationKind::IdentityUncertain), 1);
    let pids: Vec<u32> = report.processes.iter().map(|p| p.pid).collect();
    assert_eq!(pids, vec![PID, PID + 1]);
    let named = &report.processes[0];
    assert_eq!(named.name.as_ref().unwrap().display(), "a) b (c");
    assert_eq!(named.creation_token, Some(START));
    let unknown = &report.processes[1];
    assert_eq!(unknown.creation_token, None);
    assert!(unknown.identity_uncertain);
}

/// Restores a mode-000 directory so the temp dir can be removed.
struct Unlock(PathBuf);

impl Drop for Unlock {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
    }
}

fn lock(path: &Path) -> Unlock {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).unwrap();
    Unlock(path.to_path_buf())
}

fn running_as_root() -> bool {
    // SAFETY: `geteuid` has no preconditions.
    unsafe { libc::geteuid() == 0 }
}

#[test]
fn a_denied_descriptor_table_is_a_visible_gap_not_an_empty_result() {
    if running_as_root() {
        eprintln!("skipping: root can read a mode-000 directory");
        return;
    }
    let scene = scene();
    let process = scene.process(PID, START, "private");
    process.open(3, &scene.root.join("file.txt"), "0100000");
    process.cwd(&scene.root);
    let _unlock = lock(&process.dir.join("fd"));
    let report = scene.query(&scene.root, &PathUsageOptions::default());
    for mechanism in [Mechanism::OpenHandles, Mechanism::Inotify] {
        let record = coverage(&report, mechanism);
        assert_eq!(record.status, CoverageStatus::Failed, "{mechanism:?}: {record:#?}");
        assert_eq!(record.attempted, Some(1));
        assert_eq!(record.succeeded, Some(0));
        assert_eq!(limitation_count(record, LimitationKind::PermissionDenied), 1);
    }
    // The readable cwd still produced evidence.
    assert_eq!(evidence(&report, EvidenceKind::WorkingDirectory).len(), 1);
    assert_eq!(report.outcome, Outcome::Usable);
}

#[test]
fn denied_descriptors_inside_a_listed_table_are_counted() {
    if running_as_root() {
        eprintln!("skipping: root can read a mode-000 directory");
        return;
    }
    let scene = scene();
    let locked = scene.base.join("locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::write(locked.join("inner"), "x").unwrap();

    let some = scene.process(PID, START, "some");
    some.open(3, &locked.join("inner"), "0100000");
    some.open(4, &scene.root.join("file.txt"), "0100000");
    let every = scene.process(PID + 1, START, "every");
    every.open(3, &locked.join("inner"), "0100000");
    every.open(4, &locked.join("inner"), "0100000");
    let _unlock = lock(&locked);
    let report = scene.query(&scene.root, &PathUsageOptions::default());

    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.attempted, Some(2));
    assert_eq!(handles.succeeded, Some(1), "the fully denied process is not a success");
    assert_eq!(handles.status, CoverageStatus::Partial);
    assert_eq!(
        limitation_count(handles, LimitationKind::PermissionDenied),
        3 + 1,
        "three denied descriptors and one denied process: {handles:#?}"
    );
    let found = evidence(&report, EvidenceKind::OpenHandle);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].descriptor, Some(4));
}

/// Inspects one fake process directly, so state can change between
/// enumeration and inspection.
fn inspect_directly(
    scene: &Scene,
    inspection_budget: &Budget,
    between: impl FnOnce(),
) -> Vec<super::backend::MechanismInspection> {
    let budget = Budget::start(Duration::from_secs(60)).unwrap();
    let root = super::root::resolve(&scene.root, &budget).unwrap();
    let tree = super::tree::collect(&root, true, &budget);
    let mut backend = scene.backend();
    let mut enumeration = backend.enumerate(&budget).unwrap();
    assert_eq!(enumeration.processes.len(), 1);
    let candidate = enumeration.processes.remove(0);
    between();
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
    let process = scene.process(PID, START, "first");
    process.open(3, &scene.root.join("file.txt"), "0100000");
    process.cwd(&scene.root);
    let budget = Budget::start(Duration::from_secs(60)).unwrap();
    let inspections = inspect_directly(&scene, &budget, || {
        process.set_stat(&stat_line(PID, "second", START + 1));
    });
    assert_eq!(inspections.len(), 3);
    for inspection in inspections {
        assert_eq!(inspection.result, super::backend::InspectionResult::Vanished);
        assert!(inspection.evidence.is_empty());
    }
}

#[test]
fn a_process_that_exits_before_inspection_has_vanished() {
    let scene = scene();
    let process = scene.process(PID, START, "brief");
    process.open(3, &scene.root.join("file.txt"), "0100000");
    let budget = Budget::start(Duration::from_secs(60)).unwrap();
    let inspections = inspect_directly(&scene, &budget, || {
        std::fs::remove_dir_all(&process.dir).unwrap();
    });
    for inspection in inspections {
        assert_eq!(inspection.result, super::backend::InspectionResult::Vanished);
    }
}

#[test]
fn a_budget_expiring_between_descriptors_keeps_what_was_read() {
    let scene = scene();
    let process = scene.process(PID, START, "busy");
    process.open(3, &scene.root.join("file.txt"), "0100000");
    process.open(4, &scene.root.join("sub/b.txt"), "0100000");
    // The first per-descriptor check passes; the second finds it expired.
    let budget = Budget::expiring_after_checks(1);
    let inspections = inspect_directly(&scene, &budget, || {});
    let handles = inspections
        .iter()
        .find(|i| i.mechanism == Mechanism::OpenHandles)
        .unwrap();
    assert_eq!(handles.result, super::backend::InspectionResult::Partial);
    assert_eq!(handles.evidence.len(), 1);
    assert!(
        handles
            .limitations
            .iter()
            .any(|l| l.kind == LimitationKind::BudgetExhausted)
    );
}

/// Live `/proc` on Linux and WSL2.
#[cfg(target_os = "linux")]
mod live {
    use super::*;
    use std::ffi::CString;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::ffi::OsStringExt;
    use std::process::{Child, Command, Stdio};

    const OPEN: &str = "SNIFF_QUERY_CHILD_OPEN";
    const CWD: &str = "SNIFF_QUERY_CHILD_CWD";
    const WATCH: &str = "SNIFF_QUERY_CHILD_WATCH";

    fn env_paths(name: &str) -> Vec<PathBuf> {
        std::env::var_os(name)
            .map(|value| std::env::split_paths(&value).collect())
            .unwrap_or_default()
    }

    /// Holds files open, sits in a directory, and keeps inotify watches until
    /// its stdin closes.
    #[test]
    #[ignore = "subprocess fixture invoked by the live /proc tests"]
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
        std::env::set_current_dir(std::env::var_os(CWD).expect("cwd")).unwrap();
        // SAFETY: plain syscall wrappers; every path is a valid C string.
        let inotify = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
        assert!(inotify >= 0, "inotify_init1: {}", std::io::Error::last_os_error());
        for path in env_paths(WATCH) {
            let c_path = CString::new(path.clone().into_os_string().into_vec()).unwrap();
            let wd = unsafe { libc::inotify_add_watch(inotify, c_path.as_ptr(), libc::IN_ALL_EVENTS) };
            assert!(wd >= 0, "watch {}: {}", path.display(), std::io::Error::last_os_error());
        }
        println!("ready {inotify}");
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
    /// carries its inotify descriptor number.
    fn spawn_child(open: &[PathBuf], cwd: &Path, watch: &[PathBuf]) -> (ChildGuard, u64) {
        let mut command = Command::new(std::env::current_exe().expect("test executable"));
        command
            .args([
                "filesystem::query::linux_tests::live::usage_child",
                "--exact",
                "--ignored",
                "--nocapture",
            ])
            .env(OPEN, std::env::join_paths(open).unwrap())
            .env(CWD, cwd)
            .env(WATCH, std::env::join_paths(watch).unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = ChildGuard(command.spawn().expect("spawn the usage child"));
        let stdout = child.0.stdout.take().unwrap();
        for line in BufReader::new(stdout).lines() {
            let line = line.expect("child stdout");
            if let Some(fd) = line.strip_prefix("ready ") {
                return (child, fd.trim().parse().expect("inotify fd"));
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

    #[test]
    fn a_controlled_child_is_found_by_its_descriptors_cwd_and_watches() {
        let scene = scene();
        let root = &scene.root;
        for dir in ["a", "b"] {
            std::fs::create_dir(root.join(dir)).unwrap();
        }
        std::fs::write(root.join("linked.txt"), "l").unwrap();
        std::fs::write(root.join("doomed.txt"), "d").unwrap();
        std::fs::hard_link(root.join("doomed.txt"), root.join("kept.txt")).unwrap();
        let alias = scene.base.join("alias.txt");
        std::fs::hard_link(root.join("linked.txt"), &alias).unwrap();
        let open = [
            root.join("file.txt"),
            alias.clone(),
            root.join("doomed.txt"),
            scene.sibling.join("file.txt"),
        ];
        let watch = [
            root.clone(),
            root.join("a"),
            root.join("b"),
            scene.base.clone(),
            scene.sibling.clone(),
        ];
        let (child, inotify_fd) = spawn_child(&open, &root.join("sub"), &watch);
        let child_pid = child.0.id();
        std::fs::remove_file(root.join("doomed.txt")).unwrap();

        let report = live_query(root, PathUsageOptions::default());
        assert_eq!(report.outcome, Outcome::Usable);
        let records: Vec<&ProcessRecord> =
            report.processes.iter().filter(|p| p.pid == child_pid).collect();
        assert_eq!(records.len(), 1, "one record per process: {records:#?}");
        let record = records[0];
        assert!(record.creation_token.is_some());
        assert!(!record.identity_uncertain);

        let handle = |path: &Path| {
            record
                .evidence
                .iter()
                .find(|e| e.kind == EvidenceKind::OpenHandle && paths(e) == vec![path.to_path_buf()])
                .unwrap_or_else(|| panic!("no handle on {} in {:#?}", path.display(), record.evidence))
        };
        let file = handle(&root.join("file.txt"));
        assert_eq!(file.access, Some(Access { read: true, write: true }));
        assert_eq!(file.identity, Some(unix_identity(&root.join("file.txt"))));
        let linked = handle(&root.join("linked.txt"));
        assert_eq!(linked.observed_path.as_ref().unwrap().as_path(), alias);
        let kept = handle(&root.join("kept.txt"));
        let observed = kept.observed_path.as_ref().unwrap().display().into_owned();
        assert!(observed.ends_with(" (deleted)"), "{observed}");

        let cwd: Vec<&Evidence> =
            record.evidence.iter().filter(|e| e.kind == EvidenceKind::WorkingDirectory).collect();
        assert_eq!(cwd.len(), 1);
        assert_eq!(paths(cwd[0]), vec![root.join("sub")]);

        let watches: Vec<&Evidence> =
            record.evidence.iter().filter(|e| e.kind == EvidenceKind::WatchRegistration).collect();
        let mut watched: Vec<PathBuf> = watches.iter().flat_map(|e| paths(e)).collect();
        watched.sort();
        assert_eq!(watched, vec![root.clone(), root.join("a"), root.join("b")]);
        let mut ids = Vec::new();
        for watch in &watches {
            let path = &paths(watch)[0];
            // The decoded `sdev` and `ino` name exactly the watched object.
            assert_eq!(watch.identity, Some(unix_identity(path)), "{}", path.display());
            assert_eq!(watch.descriptor, Some(inotify_fd));
            assert_eq!(watch.observed_path, None);
            let info = watch.watch.as_ref().unwrap();
            assert_eq!(info.recursive, Some(false));
            assert_eq!(info.mask.map(|m| m & u64::from(libc::IN_ALL_EVENTS)), Some(u64::from(libc::IN_ALL_EVENTS)));
            ids.push(info.watch_id.unwrap());
        }
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 3, "distinct registrations are not collapsed");

        let outside = |e: &&Evidence| paths(e).iter().any(|p| !p.starts_with(root));
        assert!(!record.evidence.iter().any(|e| outside(&e)), "{:#?}", record.evidence);
        let inotify = coverage(&report, Mechanism::Inotify);
        let ancestor = inotify
            .limitations
            .iter()
            .find(|l| l.kind == LimitationKind::AncestorWatch)
            .unwrap_or_else(|| panic!("no ancestor-watch limitation: {inotify:#?}"));
        assert!(ancestor.examples.iter().any(|e| e.pid == Some(child_pid)), "{ancestor:#?}");
        drop(child);
    }

    #[test]
    fn target_only_reports_the_target_watch_and_not_descendant_usage() {
        let scene = scene();
        let root = &scene.root;
        std::fs::create_dir(root.join("a")).unwrap();
        let (child, _) = spawn_child(
            &[root.join("file.txt")],
            &root.join("sub"),
            &[root.clone(), root.join("a")],
        );
        let child_pid = child.0.id();
        let report = live_query(root, PathUsageOptions::default().target_only());
        let record = report
            .processes
            .iter()
            .find(|p| p.pid == child_pid)
            .unwrap_or_else(|| panic!("child missing: {:#?}", report.processes));
        assert_eq!(record.evidence.len(), 1, "{:#?}", record.evidence);
        assert_eq!(record.evidence[0].kind, EvidenceKind::WatchRegistration);
        assert_eq!(paths(&record.evidence[0]), vec![root.clone()]);
        drop(child);
    }

    #[test]
    fn the_querying_process_is_listed_once_and_its_threads_are_not_processes() {
        let scene = scene();
        let file = scene.root.join("file.txt");
        let held = std::fs::File::open(&file).unwrap();
        let (tid_sender, tid_receiver) = std::sync::mpsc::channel();
        let (stop_sender, stop_receiver) = std::sync::mpsc::channel::<()>();
        let thread = std::thread::spawn(move || {
            let link = std::fs::read_link("/proc/thread-self").unwrap();
            let tid: u32 = link.file_name().unwrap().to_str().unwrap().parse().unwrap();
            tid_sender.send(tid).unwrap();
            let _ = stop_receiver.recv();
        });
        let tid = tid_receiver.recv().unwrap();
        let own = std::process::id();
        assert_ne!(tid, own);

        let report = live_query(&scene.root, PathUsageOptions::default());
        stop_sender.send(()).unwrap();
        thread.join().unwrap();
        drop(held);

        assert!(!report.processes.iter().any(|p| p.pid == tid), "a thread is not a process");
        let own_records: Vec<&ProcessRecord> =
            report.processes.iter().filter(|p| p.pid == own).collect();
        assert_eq!(own_records.len(), 1, "{:#?}", report.processes);
        // The caller's genuine usage is kept; the query's own listing and
        // walk handles are gone before it inspects itself.
        let evidence = &own_records[0].evidence;
        assert_eq!(evidence.len(), 1, "{evidence:#?}");
        assert_eq!(paths(&evidence[0]), vec![file]);
    }

    #[test]
    fn the_shipped_linux_backend_inventories_descriptors_cwd_and_inotify() {
        let scene = scene();
        let report = live_query(&scene.root, PathUsageOptions::default());
        assert_eq!(report.outcome, Outcome::Usable);
        let enumeration = coverage(&report, Mechanism::ProcessEnumeration);
        assert!(enumeration.attempted.is_some_and(|n| n > 0), "{enumeration:#?}");
        for mechanism in [Mechanism::OpenHandles, Mechanism::WorkingDirectories, Mechanism::Inotify] {
            let record = coverage(&report, mechanism);
            assert!(
                matches!(record.status, CoverageStatus::Complete | CoverageStatus::Partial),
                "{mechanism:?}: {record:#?}"
            );
            assert!(record.succeeded.is_some_and(|n| n > 0), "{mechanism:?}: {record:#?}");
        }
        for mechanism in [Mechanism::Fanotify, Mechanism::Polling] {
            assert_eq!(coverage(&report, mechanism).status, CoverageStatus::Unsupported);
        }
    }
}
