//! Linux and WSL2 backend: open descriptors, working directories, and inotify
//! registrations read from `/proc`.
//!
//! Only thread-group leaders are listed (`readdir("/proc")` numeric entries);
//! `/proc/<tid>` also resolves for a thread ID, so no numeric path from any
//! other source is ever probed. Identity always comes from following the
//! `/proc/<pid>/fd/<n>` and `/proc/<pid>/cwd` magic links with `stat`, never
//! from their link text, which is descriptive for an unlinked file. An
//! inotify descriptor is recognized by its `fdinfo` registration lines and
//! its event queue is never read.

use super::backend::{
    Enumeration, InspectionContext, InspectionResult, MechanismInspection, MechanismSupport,
    ProcessCandidate, ProcessDetails, RawEvidence, Tally, UsageBackend,
};
use super::budget::Budget;
use super::identity::{self, FileIdentity};
use super::matching::ObservedObject;
use super::report::{
    Access, EvidenceKind, LimitationExample, LimitationKind, Limitations, Mechanism, WatchInfo,
};
use crate::performance::{self, counters};
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

const O_ACCMODE: u64 = 0o3;
const O_WRONLY: u64 = 0o1;
const O_RDWR: u64 = 0o2;
const O_PATH: u64 = 0o10000000;
const ESRCH: i32 = 3;

/// Reads usage from a `/proc` tree; `/proc` itself outside tests.
pub(crate) struct ProcBackend {
    proc_root: PathBuf,
    /// Identities of the root's ancestors, read on first need.
    ancestors: Option<HashSet<FileIdentity>>,
    /// Inode numbers present in the tree index, built on first need.
    tree_inodes: Option<HashSet<u64>>,
}

impl ProcBackend {
    #[cfg(target_os = "linux")]
    pub(crate) fn new() -> Self {
        Self::with_proc_root(PathBuf::from("/proc"))
    }

    pub(crate) fn with_proc_root(proc_root: PathBuf) -> Self {
        Self {
            proc_root,
            ancestors: None,
            tree_inodes: None,
        }
    }

    fn process_dir(&self, pid: u32) -> PathBuf {
        self.proc_root.join(pid.to_string())
    }
}

impl UsageBackend for ProcBackend {
    type Payload = ();

    fn mechanisms(&self) -> Vec<MechanismSupport> {
        vec![
            MechanismSupport::supported(Mechanism::OpenHandles),
            MechanismSupport::supported(Mechanism::WorkingDirectories),
            MechanismSupport::supported(Mechanism::Inotify),
            MechanismSupport::unsupported(Mechanism::Fanotify, super::backend::FANOTIFY_UNSUPPORTED),
            MechanismSupport::unsupported(Mechanism::Polling, super::backend::POLLING_UNSUPPORTED),
        ]
    }

    fn enumerate(&mut self, budget: &Budget) -> Result<Enumeration<()>, String> {
        let entries = fs::read_dir(&self.proc_root)
            .map_err(|e| format!("{} could not be listed: {e}", self.proc_root.display()))?;
        let mut processes = Vec::new();
        let mut complete = true;
        let mut limitations = Limitations::default();
        for entry in entries {
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
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    complete = false;
                    limitations.record(
                        LimitationKind::InspectionFailed,
                        "the process list could not be read completely",
                        1,
                        Some(LimitationExample::path(self.proc_root.as_path()).with_detail(error.to_string())),
                    );
                    break;
                }
            };
            let Some(pid) = parse_pid(&entry.file_name().into_vec()) else {
                continue;
            };
            let (creation_token, name) = match read_stat(&self.process_dir(pid)) {
                Ok(stat) => (Some(stat.start_ticks), Some(stat.comm)),
                // Exited between the listing and the read.
                Err(StatError::Gone) => continue,
                Err(StatError::Unreadable(detail)) => {
                    limitations.record(
                        LimitationKind::IdentityUncertain,
                        "the start time of some processes could not be read",
                        1,
                        Some(LimitationExample::pid(pid).with_detail(detail)),
                    );
                    (None, None)
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
                payload: (),
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
        candidate: &ProcessCandidate<()>,
        context: &InspectionContext<'_>,
    ) -> Vec<MechanismInspection> {
        let pid = candidate.pid;
        let dir = self.process_dir(pid);
        let mut handles = Tally::new(Mechanism::OpenHandles, pid);
        let mut inotify = Tally::new(Mechanism::Inotify, pid);
        let mut cwd = Tally::new(Mechanism::WorkingDirectories, pid);
        self.inspect_descriptors(&dir, context, &mut handles, &mut inotify);
        inspect_cwd(&dir, context, &mut cwd);

        // A changed or missing start time means the PID was reused or the
        // process exited during inspection, so nothing read can be attributed.
        if candidate.creation_token.is_some()
            && read_stat(&dir).ok().map(|s| s.start_ticks) != candidate.creation_token
        {
            return [handles, cwd, inotify]
                .into_iter()
                .map(Tally::vanished)
                .collect();
        }
        vec![handles.finish(), cwd.finish(), inotify.finish()]
    }

    fn creation_token(&mut self, pid: u32) -> Option<u64> {
        read_stat(&self.process_dir(pid)).ok().map(|s| s.start_ticks)
    }
}

impl ProcBackend {
    fn inspect_descriptors(
        &mut self,
        dir: &Path,
        context: &InspectionContext<'_>,
        handles: &mut Tally,
        inotify: &mut Tally,
    ) {
        let fd_dir = dir.join("fd");
        // The listing is collected and its directory handle closed before
        // any descriptor is read, so the querying process's own listing
        // handle has already gone when this process inspects itself.
        let descriptors = match list_descriptors(&fd_dir) {
            Ok(descriptors) => descriptors,
            Err(error) => {
                handles.whole(&error);
                inotify.whole(&error);
                return;
            }
        };
        for fd in descriptors {
            if context.budget.expired() {
                for tally in [&mut *handles, &mut *inotify] {
                    tally.problem(
                        LimitationKind::BudgetExhausted,
                        "the budget expired while a process's descriptors were being read",
                        None,
                    );
                }
                return;
            }
            performance::increment_counter(counters::QUERY_DESCRIPTOR_INSPECTIONS, 1);
            let link = fd_dir.join(fd.to_string());
            let metadata = match fs::metadata(&link) {
                Ok(metadata) => metadata,
                // Closed since the listing: no longer usage.
                Err(error) if gone(&error) => continue,
                Err(error) => {
                    handles.io_problem(&error, fd);
                    inotify.io_problem(&error, fd);
                    continue;
                }
            };
            handles.successes += 1;
            let identity = identity::of_metadata(&metadata);
            let object = ObservedObject {
                identity: Some(identity),
                path: None,
                deleted: false,
            };
            if context.match_object(&object).is_some() {
                inotify.successes += 1;
                matched_descriptor(dir, &link, fd, object, metadata.ino(), handles);
                continue;
            }
            let target = match fs::read_link(&link) {
                Ok(target) => target,
                Err(error) if gone(&error) => continue,
                Err(error) => {
                    inotify.io_problem(&error, fd);
                    continue;
                }
            };
            if !target.as_os_str().as_encoded_bytes().starts_with(b"anon_inode:") {
                inotify.successes += 1;
                continue;
            }
            self.inspect_anon_descriptor(dir, fd, context, inotify);
        }
    }

    /// Reads an anonymous-inode descriptor's `fdinfo` for inotify
    /// registrations. Other anonymous inodes (epoll, eventfd, ...) carry none.
    fn inspect_anon_descriptor(
        &mut self,
        dir: &Path,
        fd: u64,
        context: &InspectionContext<'_>,
        inotify: &mut Tally,
    ) {
        performance::increment_counter(counters::QUERY_WATCH_REGISTRATION_READS, 1);
        let text = match read_fdinfo(dir, fd) {
            Ok(text) => text,
            Err(FdinfoError::Io(error)) if gone(&error) => return,
            Err(FdinfoError::Io(error)) => return inotify.io_problem(&error, fd),
            Err(FdinfoError::NotText) => {
                return inotify.problem(
                    LimitationKind::MalformedRecord,
                    "some fdinfo records were not text",
                    Some(format!("fd {fd}")),
                );
            }
        };
        if text.is_empty() {
            return inotify.problem(
                LimitationKind::MalformedRecord,
                "some fdinfo records were empty",
                Some(format!("fd {fd}")),
            );
        }
        let parsed = parse_fdinfo(&text);
        for malformed in &parsed.malformed {
            inotify.problem(
                LimitationKind::MalformedRecord,
                "some inotify registration records were malformed",
                Some(format!("fd {fd} line {}: {}", malformed.line, malformed.reason)),
            );
        }
        for registration in parsed.registrations {
            inotify.successes += 1;
            let identity = FileIdentity::Unix {
                device: registration.device,
                inode: registration.inode,
            };
            let object = ObservedObject {
                identity: Some(identity),
                path: None,
                deleted: false,
            };
            if context.match_object(&object).is_some() {
                inotify.evidence.push(RawEvidence {
                    kind: EvidenceKind::WatchRegistration,
                    object,
                    descriptor: Some(fd),
                    watch: Some(WatchInfo {
                        watch_id: Some(registration.watch_id),
                        mask: Some(registration.mask),
                        recursive: Some(false),
                    }),
                    access: None,
                    event_only: None,
                });
            } else if self.ancestors(context).contains(&identity) {
                inotify.note(
                    LimitationKind::AncestorWatch,
                    "watches on an ancestor of the target are outside this query",
                    Some(format!("fd {fd} wd {}", registration.watch_id)),
                );
            } else if self.tree_inodes(context).contains(&registration.inode) {
                inotify.note(
                    LimitationKind::DeviceIdentityUnreliable,
                    "some watches name an in-scope inode on a device that did not match; \
                     filesystems whose stat device differs from the kernel's (btrfs \
                     subvolumes, overlayfs) cannot be compared reliably",
                    Some(format!("fd {fd} wd {}", registration.watch_id)),
                );
            }
        }
    }

    fn ancestors(&mut self, context: &InspectionContext<'_>) -> &HashSet<FileIdentity> {
        self.ancestors.get_or_insert_with(|| {
            context
                .root
                .resolved
                .ancestors()
                .skip(1)
                .filter_map(|ancestor| identity::of_path(ancestor, true).ok())
                .collect()
        })
    }

    fn tree_inodes(&mut self, context: &InspectionContext<'_>) -> &HashSet<u64> {
        self.tree_inodes.get_or_insert_with(|| {
            context
                .index
                .identities()
                .filter_map(|identity| match identity {
                    FileIdentity::Unix { inode, .. } => Some(*inode),
                    FileIdentity::Windows { .. } => None,
                })
                .collect()
        })
    }
}

/// Records an in-scope descriptor, after checking that the descriptor still
/// names the object `stat` saw.
fn matched_descriptor(
    dir: &Path,
    link: &Path,
    fd: u64,
    mut object: ObservedObject,
    inode: u64,
    handles: &mut Tally,
) {
    describe(link, &mut object);
    let mut access = None;
    // Access is optional detail; a descriptor that closed after `stat`
    // matched it, or an unreadable `fdinfo`, leaves the observation standing.
    if let Ok(text) = read_fdinfo(dir, fd) {
        let header = parse_fdinfo(&text);
        if header.inode.is_some_and(|fdinfo_inode| fdinfo_inode != inode) {
            return handles.problem(
                LimitationKind::DescriptorReplaced,
                "some descriptors were replaced while they were inspected",
                Some(format!("fd {fd}")),
            );
        }
        access = header.flags.map(access_from_flags);
    }
    handles.evidence.push(RawEvidence {
        kind: EvidenceKind::OpenHandle,
        object,
        descriptor: Some(fd),
        watch: None,
        access,
        event_only: None,
    });
}

/// Adds a magic link's text as the observed spelling. The text is
/// descriptive only: an unlinked object reads `<path> (deleted)`.
fn describe(link: &Path, object: &mut ObservedObject) {
    if let Ok(target) = fs::read_link(link) {
        let bytes = target.into_os_string().into_vec();
        object.deleted = bytes.ends_with(b" (deleted)");
        object.path = Some(PathBuf::from(OsString::from_vec(bytes)));
    }
}

fn inspect_cwd(dir: &Path, context: &InspectionContext<'_>, cwd: &mut Tally) {
    let link = dir.join("cwd");
    let metadata = match fs::metadata(&link) {
        Ok(metadata) => metadata,
        // A zombie or exiting process has no working directory; the start
        // time check after inspection tells an exit apart.
        Err(error) if gone(&error) => {
            cwd.successes += 1;
            return;
        }
        Err(error) => return cwd.whole(&error),
    };
    cwd.successes += 1;
    let mut object = ObservedObject {
        identity: Some(identity::of_metadata(&metadata)),
        path: None,
        deleted: false,
    };
    if context.match_object(&object).is_none() {
        return;
    }
    describe(&link, &mut object);
    cwd.evidence.push(RawEvidence {
        kind: EvidenceKind::WorkingDirectory,
        object,
        descriptor: None,
        watch: None,
        access: None,
        event_only: None,
    });
}

impl Tally {
    fn io_problem(&mut self, error: &io::Error, fd: u64) {
        if error.kind() == io::ErrorKind::PermissionDenied {
            self.problem(
                LimitationKind::PermissionDenied,
                "permission was denied for some descriptors",
                Some(format!("fd {fd}")),
            );
        } else {
            self.problem(
                LimitationKind::InspectionFailed,
                "some descriptors could not be read",
                Some(format!("fd {fd}: {error}")),
            );
        }
    }

    fn whole(&mut self, error: &io::Error) {
        self.whole = Some(if error.kind() == io::ErrorKind::PermissionDenied {
            InspectionResult::Denied
        } else if gone(error) {
            InspectionResult::Vanished
        } else {
            InspectionResult::Failed(error.to_string())
        });
    }
}

/// `ENOENT` or `ESRCH`: the descriptor closed or the process exited.
fn gone(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::NotFound || error.raw_os_error() == Some(ESRCH)
}

fn parse_pid(name: &[u8]) -> Option<u32> {
    if name.is_empty() || !name.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(name).ok()?.parse().ok()
}

fn list_descriptors(fd_dir: &Path) -> io::Result<Vec<u64>> {
    let mut descriptors = Vec::new();
    for entry in fs::read_dir(fd_dir)? {
        let entry = entry?;
        let name = entry.file_name().into_vec();
        if !name.is_empty() && name.iter().all(u8::is_ascii_digit)
            && let Some(fd) = std::str::from_utf8(&name).ok().and_then(|s| s.parse().ok())
        {
            descriptors.push(fd);
        }
    }
    Ok(descriptors)
}

enum FdinfoError {
    Io(io::Error),
    NotText,
}

fn read_fdinfo(dir: &Path, fd: u64) -> Result<String, FdinfoError> {
    let bytes = fs::read(dir.join("fdinfo").join(fd.to_string())).map_err(FdinfoError::Io)?;
    String::from_utf8(bytes).map_err(|_| FdinfoError::NotText)
}

fn access_from_flags(flags: u64) -> Access {
    if flags & O_PATH != 0 {
        return Access {
            read: false,
            write: false,
        };
    }
    match flags & O_ACCMODE {
        O_WRONLY => Access {
            read: false,
            write: true,
        },
        O_RDWR => Access {
            read: true,
            write: true,
        },
        _ => Access {
            read: true,
            write: false,
        },
    }
}

#[derive(Debug)]
struct Stat {
    comm: OsString,
    /// Field 22, clock ticks after boot: this lifetime's creation token.
    start_ticks: u64,
}

enum StatError {
    Gone,
    Unreadable(String),
}

fn read_stat(dir: &Path) -> Result<Stat, StatError> {
    let bytes = match fs::read(dir.join("stat")) {
        Ok(bytes) => bytes,
        Err(error) if gone(&error) => return Err(StatError::Gone),
        Err(error) => return Err(StatError::Unreadable(error.to_string())),
    };
    parse_stat(&bytes).map_err(StatError::Unreadable)
}

/// Parses `/proc/<pid>/stat`. `comm` may contain spaces and parentheses, so
/// fields are counted from the last `)`.
fn parse_stat(bytes: &[u8]) -> Result<Stat, String> {
    let open = bytes.iter().position(|&b| b == b'(').ok_or("stat has no `(`")?;
    let close = bytes.iter().rposition(|&b| b == b')').ok_or("stat has no `)`")?;
    if close < open {
        return Err("stat's `)` precedes its `(`".to_string());
    }
    let comm = OsString::from_vec(bytes[open + 1..close].to_vec());
    let rest = std::str::from_utf8(&bytes[close + 1..]).map_err(|_| "stat is not text")?;
    // Fields after `comm` start at field 3 (`state`); `starttime` is field 22.
    let start = rest
        .split_ascii_whitespace()
        .nth(22 - 3)
        .ok_or("stat has no start time")?;
    let start_ticks = parse_decimal(start).ok_or_else(|| format!("start time `{start}` is not a number"))?;
    Ok(Stat { comm, start_ticks })
}

/// One inotify watch registration from an `fdinfo` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Registration {
    pub(crate) watch_id: i64,
    pub(crate) inode: u64,
    /// Normalized to the `st_dev` encoding `stat` reports.
    pub(crate) device: u64,
    pub(crate) mask: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MalformedLine {
    /// One-based line number.
    pub(crate) line: usize,
    pub(crate) reason: String,
}

/// What `parse_fdinfo` found. `malformed` lists unusable lines of an inotify
/// record; valid sibling lines are kept in `registrations`.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Fdinfo {
    pub(crate) flags: Option<u64>,
    pub(crate) inode: Option<u64>,
    pub(crate) registrations: Vec<Registration>,
    pub(crate) malformed: Vec<MalformedLine>,
}

/// Parses `/proc/<pid>/fdinfo/<fd>` text.
///
/// A record is an inotify record when any line's first word is `inotify`.
/// Every such line must carry `wd` (decimal) and `ino`, `sdev`, `mask`
/// (hexadecimal) exactly once; unknown `key:value` fields are ignored. In an
/// inotify record, a line that is neither a registration nor a `key: value`
/// header is malformed. Malformed lines never discard their siblings.
pub(crate) fn parse_fdinfo(text: &str) -> Fdinfo {
    let mut fdinfo = Fdinfo::default();
    let is_inotify = text
        .lines()
        .any(|line| line.split_ascii_whitespace().next() == Some("inotify"));
    for (index, line) in text.lines().enumerate() {
        let first = line.split_ascii_whitespace().next();
        if first == Some("inotify") {
            match parse_registration(line) {
                Ok(registration) => fdinfo.registrations.push(registration),
                Err(reason) => fdinfo.malformed.push(MalformedLine {
                    line: index + 1,
                    reason,
                }),
            }
            continue;
        }
        match line.split_once(':') {
            Some(("flags", value)) => fdinfo.flags = parse_octal(value.trim()),
            Some(("ino", value)) => fdinfo.inode = parse_decimal(value.trim()),
            Some((key, _)) if !key.is_empty() && !key.contains(char::is_whitespace) => {}
            _ if is_inotify => fdinfo.malformed.push(MalformedLine {
                line: index + 1,
                reason: "the line is neither a registration nor a header field".to_string(),
            }),
            _ => {}
        }
    }
    fdinfo
}

fn required<'a>(value: Option<&'a str>, key: &str) -> Result<&'a str, String> {
    value.ok_or_else(|| format!("`{key}` is missing"))
}

fn parse_registration(line: &str) -> Result<Registration, String> {
    let mut watch_id = None;
    let mut inode = None;
    let mut device = None;
    let mut mask = None;
    for token in line.split_ascii_whitespace().skip(1) {
        let (key, value) = token
            .split_once(':')
            .ok_or_else(|| format!("`{token}` is not a key:value field"))?;
        if key.is_empty() {
            return Err(format!("`{token}` has no key"));
        }
        let slot = match key {
            "wd" => &mut watch_id,
            "ino" => &mut inode,
            "sdev" => &mut device,
            "mask" => &mut mask,
            _ => continue,
        };
        if slot.is_some() {
            return Err(format!("`{key}` appears more than once"));
        }
        *slot = Some(value);
    }
    let watch_id = required(watch_id, "wd")?;
    let watch_id = parse_decimal(watch_id)
        .and_then(|value| i64::try_from(value).ok())
        .ok_or_else(|| format!("`wd:{watch_id}` is not a decimal watch descriptor"))?;
    let hex = |key: &str, value: Option<&str>| {
        let value = required(value, key)?;
        parse_hex(value).ok_or_else(|| format!("`{key}:{value}` is not a 64-bit hexadecimal value"))
    };
    let inode = hex("ino", inode)?;
    let sdev = hex("sdev", device)?;
    let mask = hex("mask", mask)?;
    let device = st_dev_from_kernel(sdev)
        .ok_or_else(|| format!("`sdev:{sdev:x}` is not a 32-bit kernel device number"))?;
    Ok(Registration {
        watch_id,
        inode,
        device,
        mask,
    })
}

/// Converts the kernel-internal `dev_t` (`major << 20 | minor`) that inotify
/// `fdinfo` prints as `sdev` to the encoding `stat` reports as `st_dev`.
///
/// The two coincide only for major 0 with a minor below 256; `sdev:800011`
/// (8:17) is `st_dev` `0x811`.
pub(crate) fn st_dev_from_kernel(sdev: u64) -> Option<u64> {
    if sdev > u64::from(u32::MAX) {
        return None;
    }
    let major = sdev >> 20;
    let minor = sdev & 0xf_ffff;
    Some(((major & 0xfff) << 8) | (minor & 0xff) | ((minor & !0xff) << 12))
}

fn parse_decimal(value: &str) -> Option<u64> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn parse_hex(value: &str) -> Option<u64> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u64::from_str_radix(value, 16).ok()
}

fn parse_octal(value: &str) -> Option<u64> {
    if value.is_empty() || !value.bytes().all(|b| (b'0'..=b'7').contains(&b)) {
        return None;
    }
    u64::from_str_radix(value, 8).ok()
}
