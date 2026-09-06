//! Private, ordered playback handoff that survives the requesting process.

mod protocol;

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fs4::fs_std::FileExt as _;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::{AudioData, PlaybackError, Playa};

pub use protocol::{
    DetachedDucking, DetachedPlayback, JobId, JournalEntry, JournalOutcome,
    JournalSourceKind, JournalTransition, OsValue, PendingJob, PlaybackRouting, PreparingPayload,
    SpoolJob, SpoolSnapshot,
};
use protocol::{
    CAPABILITY_VERSION, DELEGATED_PLAY_VERSION, DelegatedReport, DelegatedRequest, JOURNAL_VERSION,
    JobEnvelope, JobState, SCHEMA_VERSION, protocol_error,
};

/// Scheduler process marker. Its value is the private spool root.
pub const SPOOL_WORKER_ENV: &str = "PLAYA_SPOOL_WORKER";
/// Playback-delegate marker. Its value is a private request-record path.
pub const DELEGATED_PLAY_WORKER_ENV: &str = "PLAYA_DELEGATED_PLAY_WORKER";

const JOURNAL_MAX_BYTES: u64 = 256 * 1024;
const PENDING_SUFFIX: &str = ".pending.json";
const IN_FLIGHT_SUFFIX: &str = ".in-flight.json";
const QUARANTINE_SUFFIX: &str = ".quarantine.json";
const STALE_PENDING: Duration = Duration::from_secs(10 * 60);
const NATIVE_DISABLED_MARKER: &str = "native-disabled";
/// Every delegate is a fresh process, so a tripped native breaker would
/// otherwise be forgotten and each later job would pay the full device-open
/// timeout before falling back. The memo expires because a device that stopped
/// answering may come back; without a TTL one bad open would pin the spool to
/// host players for good.
const NATIVE_DISABLED_MEMO_TTL: Duration = Duration::from_secs(10 * 60);

static WORKER_SEAM_INSTALLED: AtomicBool = AtomicBool::new(false);
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Install the host executable's worker entry seams before argument parsing.
///
/// A normal invocation records the seam and returns `None`. A worker invocation
/// performs its private task and returns the process exit code.
pub fn run_if_worker() -> Option<i32> {
    if let Some(root) = std::env::var_os(SPOOL_WORKER_ENV) {
        return Some(run_scheduler(Path::new(&root)).map_or_else(
            |error| {
                tracing::error!(%error, "detached Playa scheduler failed");
                1
            },
            |_| 0,
        ));
    }
    if let Some(request) = std::env::var_os(DELEGATED_PLAY_WORKER_ENV) {
        return Some(run_delegate(Path::new(&request)).map_or_else(
            |error| {
                tracing::error!(%error, "delegated Playa worker failed");
                1
            },
            |_| 0,
        ));
    }

    WORKER_SEAM_INSTALLED.store(true, Ordering::Release);
    None
}

/// Atomically publish one ready job and ensure a scheduler owns it.
pub fn enqueue(job: SpoolJob) -> Result<JobId, PlaybackError> {
    enqueue_state(JobState::Ready { job }, false)
}

/// Reserve an ordered slot while a producer prepares its final job.
pub fn reserve(preparing: PreparingPayload) -> Result<JobId, PlaybackError> {
    enqueue_state(
        JobState::Preparing {
            preparation: preparing,
        },
        false,
    )
}

/// Replace a reserved preparation slot with its ready payload without changing order.
pub fn publish_ready(job_id: &JobId, job: SpoolJob) -> Result<(), PlaybackError> {
    replace_preparing(job_id, JobState::Ready { job })
}

/// Mark a reserved preparation slot failed so the scheduler can advance.
pub fn publish_failed(job_id: &JobId, reason: impl Into<String>) -> Result<(), PlaybackError> {
    replace_preparing(
        job_id,
        JobState::Failed {
            reason: reason.into(),
        },
    )
}

/// Resolve the private record backing a reserved preparation slot.
pub fn preparation_record_path(job_id: &JobId) -> Result<PathBuf, PlaybackError> {
    let root = configured_spool_root()?;
    validate_private_root(&root)?;
    find_job_path(&root, job_id, PENDING_SUFFIX)?
        .ok_or_else(|| protocol_error("reserved job was not found"))
}

/// Read the producer-private payload from a validated preparation record.
pub fn read_preparation_record(
    path: &Path,
) -> Result<(JobId, serde_json::Map<String, serde_json::Value>), PlaybackError> {
    let root = configured_spool_root()?;
    validate_private_root(&root)?;
    let path = absolute_path(path.to_path_buf())?;
    if path.parent() != Some(root.as_path()) {
        return Err(protocol_error("preparation marker is outside the private spool"));
    }
    let envelope: JobEnvelope = read_secure_json(&path)?;
    match envelope.state {
        JobState::Preparing { preparation } => Ok((envelope.job_id, preparation.private_data)),
        _ => Err(protocol_error("preparation marker does not name a preparing slot")),
    }
}

/// Materialize audio bytes in Playa's shared content-addressed cache and enqueue them.
pub fn enqueue_bytes(
    bytes: &[u8],
    playback: DetachedPlayback,
) -> Result<JobId, PlaybackError> {
    if dry_run_enabled() {
        return Ok(JobId::dry_run());
    }
    let path = crate::playback::materialize_audio_bytes(bytes, playback.options.speed)?;
    enqueue(SpoolJob::PlayFile {
        path: OsValue::from(path.into_os_string()),
        playback,
        delete_after: false,
    })
}

/// Read redacted pending and journal projections for diagnostics.
pub fn snapshot() -> Result<SpoolSnapshot, PlaybackError> {
    let root = configured_spool_root()?;
    if !root.exists() {
        return Ok(SpoolSnapshot {
            root,
            ..SpoolSnapshot::default()
        });
    }
    validate_private_root(&root)?;

    let mut pending = Vec::new();
    for path in pending_paths(&root)? {
        let envelope: JobEnvelope = match read_secure_json(&path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let (state, source_kind) = match envelope.state {
            JobState::Preparing { .. } => ("preparing", JournalSourceKind::Preparation),
            JobState::Ready { ref job } => ("ready", source_kind(job)),
            JobState::Failed { .. } => ("failed", JournalSourceKind::Unknown),
        };
        pending.push(PendingJob {
            job_id: envelope.job_id,
            sequence: envelope.sequence,
            state,
            source_kind,
        });
    }

    let mut journal = Vec::new();
    for name in ["journal.jsonl.1", "journal.jsonl"] {
        let path = root.join(name);
        if !path.exists() {
            continue;
        }
        validate_regular_file(&path)?;
        let contents = fs::read_to_string(path)?;
        journal.extend(
            contents
                .lines()
                .filter_map(|line| serde_json::from_str::<JournalEntry>(line).ok()),
        );
    }
    journal.sort_by_key(|entry| entry.sequence);

    Ok(SpoolSnapshot {
        root,
        pending,
        journal,
    })
}

pub(crate) fn enqueue_data(
    audio: AudioData,
    playback: DetachedPlayback,
    builder_dry_run: bool,
) -> Result<JobId, PlaybackError> {
    if builder_dry_run || dry_run_enabled() {
        return Ok(JobId::dry_run());
    }
    match audio {
        AudioData::FilePath(path) => enqueue(SpoolJob::PlayFile {
            path: OsValue::from(path.into_os_string()),
            playback,
            delete_after: false,
        }),
        AudioData::Bytes(bytes) => enqueue_bytes(bytes.as_ref(), playback),
        AudioData::Url(_) => Err(protocol_error(
            "detached playback accepts local files or byte buffers, not URLs",
        )),
    }
}

fn enqueue_state(state: JobState, builder_dry_run: bool) -> Result<JobId, PlaybackError> {
    if builder_dry_run || dry_run_enabled() {
        return Ok(JobId::dry_run());
    }
    if !WORKER_SEAM_INSTALLED.load(Ordering::Acquire) {
        return Err(PlaybackError::NoDetachedWorker);
    }

    let root = configured_spool_root()?;
    enqueue_state_at(&root, state, spawn_scheduler)
}

fn enqueue_state_at<F>(root: &Path, state: JobState, spawn: F) -> Result<JobId, PlaybackError>
where
    F: FnOnce(&Path) -> Result<(), PlaybackError>,
{
    validate_state_before_io(&state, root)?;
    ensure_spool_root(root)?;
    let queue = open_lock(&root.join("queue.lock"))?;
    queue.lock_exclusive()?;

    let result = (|| {
        let sequence = allocate_sequence(root)?;
        let executable = absolute_current_exe()?;
        let fingerprint = fingerprint_file(&executable)?;
        let entropy = biscuit_hash::xx_hash(&format!(
            "{}:{}:{}",
            unix_millis(),
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let job_id = JobId::allocated(sequence, entropy);
        let envelope = JobEnvelope {
            schema_version: SCHEMA_VERSION,
            job_id: job_id.clone(),
            sequence,
            enqueued_at: chrono::Utc::now(),
            enqueuer_executable: OsValue::from(executable.into_os_string()),
            enqueuer_fingerprint: fingerprint,
            capability_version: CAPABILITY_VERSION,
            state: normalize_state(state, root)?,
        };
        let pending = pending_path(root, &envelope);
        atomic_write_json(&pending, &envelope)?;

        let worker = open_lock(&root.join("worker.lock"))?;
        match worker.try_lock_exclusive() {
            Ok(false) => Ok(job_id),
            Err(error) => Err(error.into()),
            Ok(true) => {
                FileExtUnlock::unlock_file(&worker)?;
                if let Err(error) = spawn(root) {
                    let failed = JobState::Failed {
                        reason: "worker_spawn_failed".to_string(),
                    };
                    let mut failed_envelope = envelope;
                    failed_envelope.state = failed;
                    atomic_replace_json(&pending, &failed_envelope)?;
                    append_journal(
                        root,
                        journal_failure(&failed_envelope, "worker_spawn_failed"),
                    )?;
                    let _ = fs::remove_file(&pending);
                    return Err(error);
                }
                Ok(job_id)
            }
        }
    })();

    let unlock = FileExtUnlock::unlock_file(&queue);
    match (result, unlock) {
        (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        (Ok(job_id), Ok(())) => Ok(job_id),
    }
}

fn replace_preparing(job_id: &JobId, state: JobState) -> Result<(), PlaybackError> {
    let root = configured_spool_root()?;
    validate_state_before_io(&state, &root)?;
    validate_private_root(&root)?;
    let queue = open_lock(&root.join("queue.lock"))?;
    queue.lock_exclusive()?;
    let result = (|| {
        let path = find_job_path(&root, job_id, PENDING_SUFFIX)?
            .ok_or_else(|| protocol_error("reserved job was not found"))?;
        let mut envelope: JobEnvelope = read_secure_json(&path)?;
        if !matches!(envelope.state, JobState::Preparing { .. }) {
            return Err(protocol_error("only a preparing job may be replaced"));
        }
        envelope.state = normalize_state(state, &root)?;
        atomic_replace_json(&path, &envelope)
    })();
    let unlock = FileExtUnlock::unlock_file(&queue);
    result.and(unlock)
}

fn configured_spool_root() -> Result<PathBuf, PlaybackError> {
    if let Some(root) = std::env::var_os("PLAYA_SPOOL_DIR") {
        let path = PathBuf::from(root);
        return absolute_path(path);
    }
    let user = sniff::os::current_user_id().map_err(|error| protocol_error(error.to_string()))?;
    Ok(default_spool_root(&std::env::temp_dir(), &user.to_string()))
}

fn default_spool_root(temp_dir: &Path, stable_user_id: &str) -> PathBuf {
    let hash = biscuit_hash::xx_hash(stable_user_id);
    temp_dir.join(format!("playa-spool-{hash:016x}")).join("v1")
}

fn ensure_spool_root(root: &Path) -> Result<(), PlaybackError> {
    if root.exists() {
        validate_directory(root)?;
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt as _;
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true).mode(0o700).create(root)?;
        }
        #[cfg(windows)]
        fs::create_dir_all(root)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
    }
    validate_private_root(root)?;
    fs::create_dir_all(root.join("requests"))?;
    fs::create_dir_all(root.join("files"))?;
    Ok(())
}

fn validate_directory(path: &Path) -> Result<(), PlaybackError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || is_reparse_point(&metadata) || !metadata.is_dir() {
        return Err(protocol_error(format!(
            "spool root is not a private regular directory: {}",
            path.display()
        )));
    }
    Ok(())
}

fn validate_private_root(path: &Path) -> Result<(), PlaybackError> {
    validate_directory(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let metadata = fs::metadata(path)?;
        if metadata.permissions().mode() & 0o777 != 0o700 {
            return Err(protocol_error("Unix spool root permissions must be 0700"));
        }
        let user = sniff::os::current_user_id().map_err(|error| protocol_error(error.to_string()))?;
        if let sniff::os::StableUserId::UnixUid(uid) = user
            && metadata.uid() != uid
        {
            return Err(protocol_error("Unix spool root belongs to another user"));
        }
    }
    Ok(())
}

fn validate_worker_root(path: &Path) -> Result<(), PlaybackError> {
    let configured = configured_spool_root()?;
    if path != configured {
        return Err(protocol_error("worker marker names an unexpected spool root"));
    }
    validate_private_root(path)
}

fn validate_regular_file(path: &Path) -> Result<(), PlaybackError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || is_reparse_point(&metadata) || !metadata.is_file() {
        return Err(protocol_error("spool record must be a regular non-linked file"));
    }
    Ok(())
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

fn validate_state_before_io(state: &JobState, root: &Path) -> Result<(), PlaybackError> {
    if let JobState::Ready {
        job:
            SpoolJob::PlayFile {
                path,
                delete_after: true,
                ..
            },
    } = state
    {
        let path = absolute_path(path.to_path_buf()?)?;
        if !path.starts_with(root.join("files")) {
            return Err(protocol_error(
                "delete_after is permitted only for spool-owned files",
            ));
        }
    }
    Ok(())
}

fn normalize_state(state: JobState, root: &Path) -> Result<JobState, PlaybackError> {
    match state {
        JobState::Ready {
            job:
                SpoolJob::PlayFile {
                    path,
                    playback,
                    delete_after,
                },
        } => {
            let absolute = absolute_path(path.to_path_buf()?)?;
            if delete_after && !absolute.starts_with(root.join("files")) {
                return Err(protocol_error(
                    "delete_after is permitted only for spool-owned files",
                ));
            }
            Ok(JobState::Ready {
                job: SpoolJob::PlayFile {
                    path: OsValue::from(absolute.into_os_string()),
                    playback,
                    delete_after,
                },
            })
        }
        JobState::Ready {
            job: SpoolJob::Command { program, args },
        } => {
            let original = program.to_path_buf()?;
            if !original.is_absolute() {
                return Err(protocol_error("command executable must be absolute"));
            }
            Ok(JobState::Ready {
                job: SpoolJob::Command {
                    program: OsValue::from(original.into_os_string()),
                    args,
                },
            })
        }
        other => Ok(other),
    }
}

fn absolute_path(path: PathBuf) -> Result<PathBuf, PlaybackError> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn absolute_current_exe() -> Result<PathBuf, PlaybackError> {
    let path = std::env::current_exe()?;
    if !path.is_absolute() {
        return Err(protocol_error("current executable path is not absolute"));
    }
    Ok(path)
}

/// An executable's on-disk identity: its size and modification time folded
/// into one value. Any rebuild or replacement changes the modification time,
/// and a stat costs microseconds where hashing a 100+ MB binary on every
/// publish made one lifecycle emission take seconds. The spool is private to
/// the user, so this guards against a binary swapped mid-flight, not against
/// an adversary.
fn fingerprint_file(path: &Path) -> Result<u64, PlaybackError> {
    validate_regular_file(path)?;
    let metadata = fs::metadata(path)?;
    let modified = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    Ok(biscuit_hash::xx_hash(&format!("{}:{modified}", metadata.len())))
}

fn open_lock(path: &Path) -> Result<File, PlaybackError> {
    if path.exists() {
        validate_regular_file(path)?;
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    validate_regular_file(path)?;
    Ok(file)
}

struct FileExtUnlock;

impl FileExtUnlock {
    fn unlock_file(file: &File) -> Result<(), PlaybackError> {
        fs4::fs_std::FileExt::unlock(file).map_err(Into::into)
    }
}

fn allocate_sequence(root: &Path) -> Result<u64, PlaybackError> {
    let path = root.join("sequence");
    if path.exists() {
        validate_regular_file(&path)?;
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    let previous = if text.trim().is_empty() {
        0
    } else {
        text.trim()
            .parse::<u64>()
            .map_err(|_| protocol_error("invalid durable sequence file"))?
    };
    let next = previous
        .checked_add(1)
        .ok_or_else(|| protocol_error("detached sequence exhausted"))?;
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    writeln!(file, "{next}")?;
    file.sync_all()?;
    Ok(next)
}

fn pending_path(root: &Path, envelope: &JobEnvelope) -> PathBuf {
    root.join(format!(
        "{:020}-{}{}",
        envelope.sequence, envelope.job_id, PENDING_SUFFIX
    ))
}

fn atomic_write_json(path: &Path, value: &impl Serialize) -> Result<(), PlaybackError> {
    if path.exists() {
        return Err(protocol_error("refusing to overwrite an existing spool record"));
    }
    let temp = unique_neighbor(path, "publish");
    let bytes = serde_json::to_vec(value).map_err(|error| protocol_error(error.to_string()))?;
    let result = (|| {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn atomic_replace_json(path: &Path, value: &impl Serialize) -> Result<(), PlaybackError> {
    validate_regular_file(path)?;
    let temp = unique_neighbor(path, "replace");
    let bytes = serde_json::to_vec(value).map_err(|error| protocol_error(error.to_string()))?;
    let result = (|| {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        replace_path(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(not(windows))]
fn replace_path(source: &Path, destination: &Path) -> Result<(), PlaybackError> {
    fs::rename(source, destination).map_err(Into::into)
}

#[cfg(windows)]
fn replace_path(source: &Path, destination: &Path) -> Result<(), PlaybackError> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let success = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if success == 0 {
        Err(std::io::Error::last_os_error().into())
    } else {
        Ok(())
    }
}

fn unique_neighbor(path: &Path, label: &str) -> PathBuf {
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!(
        ".{name}.{label}.{}.{}.tmp",
        std::process::id(),
        sequence
    ))
}

fn read_secure_json<T: DeserializeOwned>(path: &Path) -> Result<T, PlaybackError> {
    validate_regular_file(path)?;
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(|error| protocol_error(error.to_string()))
}

fn spawn_scheduler(root: &Path) -> Result<(), PlaybackError> {
    let executable = absolute_current_exe()?;
    let mut command = Command::new(executable);
    command
        .env_remove(DELEGATED_PLAY_WORKER_ENV)
        .env(SPOOL_WORKER_ENV, root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_detached_child(&mut command);
    command
        .spawn()
        .map(|_| ())
        .map_err(|source| PlaybackError::DetachedWorkerSpawn { source })
}

/// Configure a command so its child outlives the caller and shares neither its
/// process group nor the pipes a parent may be reading from.
///
/// Callers must still give the child `Stdio::null()` (or their own handles);
/// this only prevents the caller's inherited stdio from leaking into it.
pub fn configure_detached_child(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        stop_inheriting_std_handles();
        command.creation_flags(detached_creation_flags());
    }
}

/// Windows children inherit every inheritable handle in the parent, and the
/// stdio handles a parent handed this process stay inheritable even when the
/// child is given `Stdio::null()`. A caller capturing this process's output
/// would then wait for the detached worker to exit before its read returned,
/// which is the blocking behavior the spool exists to avoid. Clearing the flag
/// on this process's own copies is safe because `Stdio::inherit()` duplicates
/// the handle with inheritance re-enabled.
#[cfg(windows)]
fn stop_inheriting_std_handles() {
    use windows_sys::Win32::Foundation::{
        HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE, SetHandleInformation,
    };
    use windows_sys::Win32::System::Console::{
        GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };
    for id in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        // SAFETY: both calls only inspect or flag a handle this process owns;
        // a null or invalid handle is skipped rather than passed through.
        unsafe {
            let handle = GetStdHandle(id);
            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                continue;
            }
            SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0);
        }
    }
}

#[cfg(windows)]
const fn detached_creation_flags() -> u32 {
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW
}

fn run_scheduler(root: &Path) -> Result<(), PlaybackError> {
    let root = absolute_path(root.to_path_buf())?;
    validate_worker_root(&root)?;
    run_scheduler_with(&root, &mut |root, envelope, job| {
        delegate_job(root, envelope, job)
    })
}

fn run_scheduler_with<F>(root: &Path, delegate: &mut F) -> Result<(), PlaybackError>
where
    F: FnMut(&Path, &JobEnvelope, &SpoolJob) -> Result<JournalOutcome, PlaybackError>,
{
    let root = absolute_path(root.to_path_buf())?;
    validate_private_root(&root)?;
    let worker = open_lock(&root.join("worker.lock"))?;
    worker.lock_exclusive()?;
    quarantine_abandoned_in_flight(&root)?;

    loop {
        let Some(path) = next_pending(&root)? else {
            // The publication and final-empty paths use the same lock order:
            // queue first, then a worker-lock probe/release. A publisher can
            // therefore neither miss this worker nor commit behind its exit.
            let queue = open_lock(&root.join("queue.lock"))?;
            queue.lock_exclusive()?;
            if next_pending(&root)?.is_some() {
                FileExtUnlock::unlock_file(&queue)?;
                continue;
            }
            FileExtUnlock::unlock_file(&worker)?;
            FileExtUnlock::unlock_file(&queue)?;
            return Ok(());
        };
        process_pending(&root, &path, delegate)?;
    }
}

fn process_pending<F>(root: &Path, path: &Path, delegate: &mut F) -> Result<(), PlaybackError>
where
    F: FnMut(&Path, &JobEnvelope, &SpoolJob) -> Result<JournalOutcome, PlaybackError>,
{
    let header: serde_json::Value = match read_secure_json(path) {
        Ok(value) => value,
        Err(error) => {
            quarantine(root, path, None, "malformed_job")?;
            tracing::warn!(%error, "quarantined malformed detached job");
            return Ok(());
        }
    };
    if header.get("schema_version").and_then(serde_json::Value::as_u64)
        != Some(u64::from(SCHEMA_VERSION))
    {
        quarantine(root, path, header_identity(&header), "unsupported_schema")?;
        return Ok(());
    }
    let envelope = match JobEnvelope::deserialize(&header) {
        Ok(value) => value,
        Err(error) => {
            quarantine(root, path, header_identity(&header), "malformed_job")?;
            tracing::warn!(%error, "quarantined malformed detached job");
            return Ok(());
        }
    };
    let age = chrono::Utc::now().signed_duration_since(envelope.enqueued_at);
    if age > chrono::Duration::from_std(STALE_PENDING).unwrap_or_else(|_| chrono::Duration::minutes(10)) {
        append_journal(root, journal_failure(&envelope, "stale_pending"))?;
        fs::remove_file(path)?;
        return Ok(());
    }

    match &envelope.state {
        JobState::Preparing { preparation } => {
            let terminal_reason = if preparation.preparation_version
                != protocol::PREPARATION_VERSION
            {
                Some("preparation_incompatible")
            } else if chrono::Utc::now() >= preparation.deadline {
                Some("preparation_timed_out")
            } else {
                None
            };
            if let Some(reason) = terminal_reason {
                let queue = open_lock(&root.join("queue.lock"))?;
                queue.lock_exclusive()?;
                let latest: JobEnvelope = read_secure_json(path)?;
                if matches!(latest.state, JobState::Preparing { .. }) {
                    append_journal(root, journal_failure(&latest, reason))?;
                    fs::remove_file(path)?;
                }
                FileExtUnlock::unlock_file(&queue)?;
            } else {
                std::thread::sleep(Duration::from_millis(25));
            }
            Ok(())
        }
        JobState::Failed { .. } => {
            append_journal(root, journal_failure(&envelope, "preparation_failed"))?;
            fs::remove_file(path)?;
            Ok(())
        }
        JobState::Ready { job } => {
            let in_flight = path.with_file_name(
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .replace(PENDING_SUFFIX, IN_FLIGHT_SUFFIX),
            );
            fs::rename(path, &in_flight)?;
            let outcome = delegate(root, &envelope, job).unwrap_or_else(|error| {
                JournalOutcome::Failed {
                    reason: redacted_failure_reason(&error),
                }
            });
            append_journal(
                root,
                JournalEntry {
                    journal_version: JOURNAL_VERSION,
                    job_id: envelope.job_id.clone(),
                    sequence: envelope.sequence,
                    enqueued_at: envelope.enqueued_at,
                    finished_at: chrono::Utc::now(),
                    source_kind: source_kind(job),
                    transition: if matches!(outcome, JournalOutcome::Failed { .. }) {
                        JournalTransition::Failed
                    } else {
                        JournalTransition::Completed
                    },
                    outcome,
                },
            )?;
            if let SpoolJob::PlayFile {
                path,
                delete_after: true,
                ..
            } = job
                && let Ok(path) = path.to_path_buf()
                && path.starts_with(root.join("files"))
            {
                let _ = fs::remove_file(path);
            }
            fs::remove_file(in_flight)?;
            Ok(())
        }
    }
}

/// Resolve the enqueuer executable and prove it is the same file that
/// published the job.
fn validate_enqueuer_identity(envelope: &JobEnvelope) -> Result<(PathBuf, u64), PlaybackError> {
    let executable = envelope.enqueuer_executable.to_path_buf()?;
    if !executable.is_absolute()
        || fingerprint_file(&executable)? != envelope.enqueuer_fingerprint
    {
        return Err(protocol_error("enqueuer executable is missing or was replaced"));
    }
    Ok((executable, envelope.enqueuer_fingerprint))
}

fn delegate_job(
    root: &Path,
    envelope: &JobEnvelope,
    job: &SpoolJob,
) -> Result<JournalOutcome, PlaybackError> {
    if envelope.capability_version != CAPABILITY_VERSION {
        return Err(protocol_error("incompatible enqueuer capability version"));
    }
    let (executable, expected_fingerprint) = validate_enqueuer_identity(envelope)?;

    let request_dir = root.join("requests");
    let request_path = request_dir.join(format!("{}.request.json", envelope.job_id));
    let report_path = request_dir.join(format!("{}.report.json", envelope.job_id));
    let request = DelegatedRequest {
        schema_version: SCHEMA_VERSION,
        delegated_play_version: DELEGATED_PLAY_VERSION,
        job_id: envelope.job_id.clone(),
        sequence: envelope.sequence,
        delegate_executable: envelope.enqueuer_executable.clone(),
        delegate_fingerprint: expected_fingerprint,
        capability_version: envelope.capability_version,
        job: job.clone(),
        report_path: OsValue::from(report_path.clone().into_os_string()),
    };
    atomic_write_json(&request_path, &request)?;

    let status = Command::new(&executable)
        .env_remove(SPOOL_WORKER_ENV)
        .env(DELEGATED_PLAY_WORKER_ENV, &request_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = fs::remove_file(&request_path);
    let status = status.map_err(|error| protocol_error(format!("delegate launch failed: {error}")))?;
    if !status.success() {
        let _ = fs::remove_file(&report_path);
        return Err(protocol_error("delegate exited without a successful report"));
    }
    let report: DelegatedReport = read_secure_json(&report_path)?;
    let _ = fs::remove_file(&report_path);
    if report.schema_version != SCHEMA_VERSION
        || report.job_id != envelope.job_id
        || report.sequence != envelope.sequence
        || report.delegate.to_path_buf()? != executable
        || report.delegate_fingerprint != expected_fingerprint
    {
        return Err(protocol_error("delegated report identity or version mismatch"));
    }
    Ok(report.outcome)
}

fn run_delegate(request_path: &Path) -> Result<(), PlaybackError> {
    let request_path = absolute_path(request_path.to_path_buf())?;
    validate_regular_file(&request_path)?;
    let root = request_path
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| protocol_error("delegate request is outside a spool"))?;
    validate_worker_root(root)?;
    if request_path.parent() != Some(root.join("requests").as_path()) {
        return Err(protocol_error("delegate request is outside the request directory"));
    }
    let request: DelegatedRequest = read_secure_json(&request_path)?;
    if request.schema_version != SCHEMA_VERSION
        || request.delegated_play_version != DELEGATED_PLAY_VERSION
        || request.capability_version != CAPABILITY_VERSION
    {
        return Err(protocol_error("incompatible delegated request"));
    }
    let current = absolute_current_exe()?;
    if request.delegate_executable.to_path_buf()? != current
        || fingerprint_file(&current)? != request.delegate_fingerprint
    {
        return Err(protocol_error("delegated executable identity mismatch"));
    }
    let report_path = request.report_path.to_path_buf()?;
    if report_path.parent() != Some(root.join("requests").as_path()) || report_path.exists() {
        return Err(protocol_error("delegated report sidecar is not private and create-new"));
    }

    let force_host = native_disabled_memo_is_fresh(root);
    let outcome = execute_job(&request.job, force_host).unwrap_or_else(|error| {
        JournalOutcome::Failed {
            reason: redacted_failure_reason(&error),
        }
    });
    if native_breaker_tripped()
        && let Err(error) = record_native_disabled_memo(root)
    {
        tracing::warn!(%error, "could not record the native-disabled memo");
    }
    let report = DelegatedReport {
        schema_version: SCHEMA_VERSION,
        job_id: request.job_id,
        sequence: request.sequence,
        delegate: OsValue::from(current.into_os_string()),
        delegate_fingerprint: request.delegate_fingerprint,
        outcome,
    };
    atomic_write_json(&report_path, &report)
}

fn native_breaker_tripped() -> bool {
    #[cfg(feature = "sfx-native")]
    {
        !crate::native_audio::native_audio_available()
    }
    #[cfg(not(feature = "sfx-native"))]
    {
        false
    }
}

/// Reports whether a delegate must force host routing, removing a marker that
/// has outlived [`NATIVE_DISABLED_MEMO_TTL`] or is not a regular file.
fn native_disabled_memo_is_fresh(root: &Path) -> bool {
    let marker = root.join(NATIVE_DISABLED_MARKER);
    let Ok(metadata) = fs::symlink_metadata(&marker) else {
        return false;
    };
    let fresh = !metadata.file_type().is_symlink()
        && !is_reparse_point(&metadata)
        && metadata.is_file()
        && metadata
            .modified()
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age < NATIVE_DISABLED_MEMO_TTL);
    if !fresh {
        let _ = fs::remove_file(&marker);
    }
    fresh
}

fn record_native_disabled_memo(root: &Path) -> Result<(), PlaybackError> {
    let marker = root.join(NATIVE_DISABLED_MARKER);
    let temp = unique_neighbor(&marker, "memo");
    let result = (|| {
        let file = OpenOptions::new().write(true).create_new(true).open(&temp)?;
        file.sync_all()?;
        replace_path(&temp, &marker)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn execute_job(job: &SpoolJob, force_host: bool) -> Result<JournalOutcome, PlaybackError> {
    match job {
        SpoolJob::PlayFile { path, playback, .. } => {
            let path = path.to_path_buf()?;
            if !path.is_absolute() {
                return Err(protocol_error("delegated audio path is not absolute"));
            }
            validate_regular_file(&path)?;
            let mut player = Playa::from_path(path)
                .map_err(|error| protocol_error(format!("audio source rejected: {error}")))?
                .with_options(playback.options.clone());
            if force_host || playback.routing == PlaybackRouting::ForceHost {
                player = player.force_host();
            }
            #[cfg(feature = "audio-ducking")]
            let report = if let Some(ducking) = playback.ducking {
                let config = crate::ducking::DuckConfig::new(ducking.ramp_ms, ducking.floor_scalar)
                    .map_err(|error| protocol_error(error.to_string()))?;
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(player.with_ducked_audio(config).play_async_with_report())?
            } else {
                player.play_with_report()?
            };
            #[cfg(not(feature = "audio-ducking"))]
            let report = {
                if playback.ducking.is_some() {
                    return Err(protocol_error("delegate lacks audio-ducking capability"));
                }
                player.play_with_report()?
            };
            Ok(JournalOutcome::Playback { report })
        }
        SpoolJob::Command { program, args } => {
            let program = program.to_path_buf()?;
            if !program.is_absolute() {
                return Err(protocol_error("delegated command executable is not absolute"));
            }
            validate_regular_file(&program)?;
            let arguments = args
                .iter()
                .map(OsValue::to_os_string)
                .collect::<Result<Vec<OsString>, PlaybackError>>()?;
            let status = Command::new(program).args(arguments).status()?;
            Ok(JournalOutcome::CommandExit {
                code: status.code(),
                success: status.success(),
            })
        }
    }
}

fn pending_paths(root: &Path) -> Result<Vec<PathBuf>, PlaybackError> {
    let mut paths = fs::read_dir(root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(PENDING_SUFFIX))
        })
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn next_pending(root: &Path) -> Result<Option<PathBuf>, PlaybackError> {
    Ok(pending_paths(root)?.into_iter().next())
}

fn quarantine_abandoned_in_flight(root: &Path) -> Result<(), PlaybackError> {
    let paths = fs::read_dir(root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(IN_FLIGHT_SUFFIX))
        })
        .collect::<Vec<_>>();
    for path in paths {
        let header = read_secure_json::<serde_json::Value>(&path)
            .ok()
            .and_then(|value| header_identity(&value));
        quarantine(root, &path, header, "abandoned_in_flight")?;
    }
    Ok(())
}

fn quarantine(
    root: &Path,
    path: &Path,
    header: Option<RedactedHeader>,
    reason: &str,
) -> Result<(), PlaybackError> {
    validate_regular_file(path)?;
    let quarantine = path.with_file_name(
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .replace(PENDING_SUFFIX, QUARANTINE_SUFFIX)
            .replace(IN_FLIGHT_SUFFIX, QUARANTINE_SUFFIX),
    );
    fs::rename(path, quarantine)?;
    if let Some(header) = header {
        append_journal(root, journal_header_failure(&header, reason))?;
    }
    Ok(())
}

/// The journal-safe subset of an envelope. A record that fails the full v1
/// parse (missing required field, unknown payload) still journals from this.
struct RedactedHeader {
    job_id: JobId,
    sequence: u64,
    enqueued_at: chrono::DateTime<chrono::Utc>,
    source_kind: JournalSourceKind,
}

impl From<&JobEnvelope> for RedactedHeader {
    fn from(envelope: &JobEnvelope) -> Self {
        Self {
            job_id: envelope.job_id.clone(),
            sequence: envelope.sequence,
            enqueued_at: envelope.enqueued_at,
            source_kind: state_source_kind(&envelope.state),
        }
    }
}

fn header_identity(value: &serde_json::Value) -> Option<RedactedHeader> {
    #[derive(Deserialize)]
    struct Identity {
        job_id: JobId,
        sequence: u64,
        enqueued_at: chrono::DateTime<chrono::Utc>,
    }
    let identity = Identity::deserialize(value).ok()?;
    let source_kind = value
        .get("payload")
        .and_then(|payload| JobState::deserialize(payload).ok())
        .map_or(JournalSourceKind::Unknown, |state| state_source_kind(&state));
    Some(RedactedHeader {
        job_id: identity.job_id,
        sequence: identity.sequence,
        enqueued_at: identity.enqueued_at,
        source_kind,
    })
}

fn journal_failure(envelope: &JobEnvelope, reason: &str) -> JournalEntry {
    journal_header_failure(&RedactedHeader::from(envelope), reason)
}

fn journal_header_failure(header: &RedactedHeader, reason: &str) -> JournalEntry {
    JournalEntry {
        journal_version: JOURNAL_VERSION,
        job_id: header.job_id.clone(),
        sequence: header.sequence,
        enqueued_at: header.enqueued_at,
        finished_at: chrono::Utc::now(),
        source_kind: header.source_kind,
        transition: match reason {
            "stale_pending" => JournalTransition::Discarded,
            "abandoned_in_flight" | "unsupported_schema" | "malformed_job" => {
                JournalTransition::Quarantined
            }
            _ => JournalTransition::Failed,
        },
        outcome: JournalOutcome::Failed {
            reason: reason.to_string(),
        },
    }
}

fn state_source_kind(state: &JobState) -> JournalSourceKind {
    match state {
        JobState::Preparing { .. } => JournalSourceKind::Preparation,
        JobState::Ready { job } => source_kind(job),
        JobState::Failed { .. } => JournalSourceKind::Unknown,
    }
}

fn source_kind(job: &SpoolJob) -> JournalSourceKind {
    match job {
        SpoolJob::PlayFile { .. } => JournalSourceKind::File,
        SpoolJob::Command { .. } => JournalSourceKind::Command,
    }
}

fn redacted_failure_reason(error: &PlaybackError) -> String {
    match error {
        PlaybackError::NoDetachedWorker => "no_detached_worker",
        PlaybackError::DetachedWorkerSpawn { .. } => "worker_spawn_failed",
        PlaybackError::DetachedProtocol { .. } => "protocol_failure",
        _ => "playback_failed",
    }
    .to_string()
}

fn append_journal(root: &Path, entry: JournalEntry) -> Result<(), PlaybackError> {
    let path = root.join("journal.jsonl");
    let previous = root.join("journal.jsonl.1");
    let mut line = serde_json::to_vec(&entry).map_err(|error| protocol_error(error.to_string()))?;
    line.push(b'\n');
    if path.exists() {
        validate_regular_file(&path)?;
        let len = fs::metadata(&path)?.len();
        if len.saturating_add(line.len() as u64) > JOURNAL_MAX_BYTES {
            if previous.exists() {
                validate_regular_file(&previous)?;
                fs::remove_file(&previous)?;
            }
            fs::rename(&path, &previous)?;
        }
    }
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    file.write_all(&line)?;
    file.sync_all()?;
    Ok(())
}

fn find_job_path(
    root: &Path,
    job_id: &JobId,
    suffix: &str,
) -> Result<Option<PathBuf>, PlaybackError> {
    Ok(fs::read_dir(root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains(job_id.as_str()) && name.ends_with(suffix))
        }))
}

fn dry_run_enabled() -> bool {
    matches!(
        std::env::var("PLAYA_DRY_RUN").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}

pub(crate) fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
