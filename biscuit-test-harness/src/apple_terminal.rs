//! Terminal.app-driven [`TerminalHarness`] (macOS).
//!
//! Drives Apple's bundled Terminal.app via `osascript` so Level-2 tests
//! can verify rendering against the real lower-capability display path
//! (no images, no OSC8, single-underline only).
//!
//! ## Capture limitation
//!
//! Terminal.app exposes only the **plain visible text** of a tab via
//! its scripting interface. There is no way to retrieve the underlying
//! ANSI/SGR byte stream. As a result, [`CapturedFrame::raw`] and
//! [`CapturedFrame::plain`] are populated with the same string, and
//! callers cannot directly assert "no OSC8 bytes were emitted" — they
//! can only assert "no literal escape garbage is visible".
//!
//! Level-1 PTY tests cover the byte-level negative assertions; this
//! harness covers the real-display visibility assertions.
//!
//! ## Skip semantics
//!
//! [`AppleTerminalHarness::available`] returns `false` when:
//!
//! - The host is not macOS.
//! - `CI` has a truthy value (Terminal.app cannot be exercised in CI).
//! - `osascript` cannot resolve the `Terminal` application bundle.
//!
//! Tests should early-return with
//! [`crate::skip_with_reason`](super::skip_with_reason) when this
//! returns `false`.
//!
//! ## Concurrency
//!
//! Terminal.app exposes one global application state to AppleScript;
//! spawning multiple harnesses in parallel races on window indices and
//! focus. Level-2 tests that use this harness must serialize via
//! `serial_test::serial(level2_terminal)` or equivalent.
//!
//! ## AppleScript escape contract
//!
//! [`applescript_escape`] handles only the characters that have a
//! defined representation inside an AppleScript double-quoted literal:
//! backslash (`\\`), double-quote (`"`), newline (`\n`), and tab
//! (`\t`). Bytes outside printable UTF-8 plus LF (0x0A) and HT (0x09)
//! are not escaped; CR (0x0D), NUL (0x00), BEL (0x07), ESC (0x1B), and
//! the Unicode line / paragraph separators (U+2028 / U+2029) will
//! produce an AppleScript syntax error and are rejected via
//! `debug_assert!` in debug builds. Release builds remain best-effort
//! so that an unexpected byte does not crash the harness.

#![allow(dead_code)]

use std::env;
use std::fs::{self, OpenOptions};
use std::io;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Once, OnceLock};
use std::time::Duration;

use super::{
    CLEANUP_TIMEOUT, CapturedFrame, QUERY_TIMEOUT, TerminalHarness, current_process_id,
    pid_from_tag, process_is_alive, run_with_timeout,
};

const WINDOW_TITLE_PREFIX: &str = "biscuit-test-terminal-";
static CLEANUP_ONCE: Once = Once::new();

/// Set when a stale-window cleanup closed a window or quit Terminal.app, so
/// the next [`AppleTerminalHarness::spawn_shell`] lets those asynchronous
/// closes land before it spawns. A spawn with nothing closed skips the wait.
static CLEANUP_CLOSED_WINDOWS: AtomicBool = AtomicBool::new(false);

fn ci_value_is_truthy(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

/// Open-window count above which [`sweep_legacy_apple_terminal_windows`]
/// self-heals without the opt-in env var. The shared-broker model keeps
/// only one harness window open at a time, so a pile-up well past this is
/// almost certainly leaked test windows; the narrow
/// [`looks_like_harness_window`] predicate still protects a developer's real
/// windows during the sweep. Counterpart to WezTerm's
/// `LEGACY_BACKGROUND_PANE_LIMIT` (which can be higher because its panes are
/// quarantined in an isolated workspace).
const LEGACY_WINDOW_LIMIT: usize = 16;

/// Basename of the title-independent window-id registry.
///
/// One JSONL row per harness-created window lives under `${TMPDIR:-/tmp}`
/// so the reaper can identify leaked windows by id even after the login
/// shell overwrites their `custom title` (Pitfall 2, Layer 1).
const REGISTRY_FILE_NAME: &str = "biscuit-test-terminal-registry.jsonl";

/// Basename of the sidecar lock guarding the registry read-modify-write.
///
/// Held for registry mutations that can race with a rewrite. A crashed
/// process's lock is reclaimed once it is older than
/// [`REGISTRY_LOCK_STALE`] (Pitfall 2, Layer 1).
const REGISTRY_LOCK_FILE_NAME: &str = "biscuit-test-terminal-registry.lock";

/// A registry lock older than this is presumed abandoned by a crashed
/// reaper and may be stolen. Generous relative to a prune's sub-second
/// cost so we never steal a lock a live reaper still holds.
const REGISTRY_LOCK_STALE: Duration = Duration::from_secs(30);

/// One window the harness created, recorded so a later process can reap
/// it after the owning test exits — independent of the window's title.
///
/// `seq` is a per-process monotonic counter that disambiguates rows when
/// `owner_pid` collides after pid reuse; it carries no Terminal meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RegistryEntry {
    pub window_id: i64,
    pub owner_pid: u32,
    pub seq: u64,
}

impl RegistryEntry {
    /// Serializes to a single-line JSON object (no trailing newline).
    fn to_json_line(self) -> String {
        serde_json::json!({
            "window_id": self.window_id,
            "owner_pid": self.owner_pid,
            "seq": self.seq,
        })
        .to_string()
    }

    /// Parses one registry line, returning `None` for blank or malformed
    /// rows so a single corrupt append never poisons the whole file.
    fn from_json_line(line: &str) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
        Some(Self {
            window_id: value.get("window_id")?.as_i64()?,
            owner_pid: u32::try_from(value.get("owner_pid")?.as_u64()?).ok()?,
            seq: value.get("seq")?.as_u64()?,
        })
    }
}

/// Directory holding the registry and its lock: `${TMPDIR:-/tmp}`.
fn registry_dir() -> PathBuf {
    env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// Path to the on-disk window registry: `${TMPDIR:-/tmp}/{REGISTRY_FILE_NAME}`.
fn registry_path() -> PathBuf {
    registry_dir().join(REGISTRY_FILE_NAME)
}

/// Sidecar lock path for a specific registry file.
fn registry_lock_path_for(path: &Path) -> PathBuf {
    path.with_file_name(REGISTRY_LOCK_FILE_NAME)
}

/// Next per-process registry sequence number.
fn next_registry_seq() -> u64 {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

/// Records `window_id` in the registry as owned by the owning process (see
/// [`super::owner_process_id`]).
///
/// Call only for windows the harness genuinely created (`owned == true`):
/// registering a reused window we don't own would later authorize the
/// reaper to close it. Best-effort — registry I/O never fails a spawn.
fn register_window(window_id: i64) {
    let entry = RegistryEntry {
        window_id,
        owner_pid: super::owner_process_id(),
        seq: next_registry_seq(),
    };
    append_registry_entry(&registry_path(), entry);
}

/// Removes every row for `window_id` from the registry (best-effort).
///
/// Called on the clean close path so the common case leaves no residue;
/// orphaned rows from killed runs are pruned by the reaper instead.
fn unregister_window(window_id: i64) {
    remove_registry_entry(&registry_path(), window_id);
}

/// Appends one row to `path` while holding the registry sidecar lock.
///
/// The file is still opened with `O_APPEND`, but the lock is what prevents
/// concurrent registry rewrites from dropping a freshly appended row. Errors
/// are swallowed: registry upkeep must not fail a spawn.
fn append_registry_entry(path: &Path, entry: RegistryEntry) {
    let Some(_lock) = RegistryLock::acquire_with_retry(&registry_lock_path_for(path)) else {
        return;
    };
    let mut line = entry.to_json_line();
    line.push('\n');
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(line.as_bytes()));
}

/// Reads and parses every well-formed row from `path`.
///
/// A missing file yields an empty vector; malformed lines are skipped.
fn read_registry(path: &Path) -> Vec<RegistryEntry> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };
    contents
        .lines()
        .filter_map(RegistryEntry::from_json_line)
        .collect()
}

/// Rewrites `path` dropping every row whose `window_id` matches, while
/// preserving rows for other windows. Best-effort: a read or write error
/// leaves the file untouched.
fn remove_registry_entry(path: &Path, window_id: i64) {
    let Some(_lock) = RegistryLock::acquire_with_retry(&registry_lock_path_for(path)) else {
        return;
    };
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };
    let mut out = String::with_capacity(contents.len());
    for line in contents.lines() {
        let drop = RegistryEntry::from_json_line(line).is_some_and(|e| e.window_id == window_id);
        if drop || line.trim().is_empty() {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    let _ = fs::write(path, out);
}

/// Best-effort exclusive lock around the registry read-modify-write.
///
/// Created with `O_CREAT|O_EXCL` so only one holder wins; the file is
/// removed on [`Drop`]. A lock left behind by a crashed reaper is stolen
/// once it ages past [`REGISTRY_LOCK_STALE`].
struct RegistryLock {
    path: PathBuf,
}

impl RegistryLock {
    /// Tries to take the lock at `path`.
    ///
    /// Returns `None` when another reaper holds a fresh lock — the caller
    /// then skips the prune (best-effort). A stale lock is reclaimed.
    fn acquire(path: &Path) -> Option<Self> {
        match OpenOptions::new().create_new(true).write(true).open(path) {
            Ok(_) => Some(Self { path: path.to_path_buf() }),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                if !lock_is_stale(path) {
                    return None;
                }
                // Reclaim a lock abandoned by a crashed reaper.
                let _ = fs::remove_file(path);
                OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(path)
                    .ok()
                    .map(|_| Self { path: path.to_path_buf() })
            }
            Err(_) => None,
        }
    }

    /// Tries briefly to take the lock at `path`.
    ///
    /// Spawn/drop cleanup is best-effort, but the lock is normally held
    /// for only one small file rewrite. A short retry avoids dropping a
    /// registry update during ordinary contention.
    fn acquire_with_retry(path: &Path) -> Option<Self> {
        for attempt in 0..100 {
            if let Some(lock) = Self::acquire(path) {
                return Some(lock);
            }
            if attempt < 99 {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        None
    }
}

impl Drop for RegistryLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Whether the lock at `path` is old enough to presume abandoned. A file
/// whose mtime cannot be read is treated as *not* stale so we never steal
/// a lock we cannot reason about.
fn lock_is_stale(path: &Path) -> bool {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|mtime| mtime.elapsed().ok())
        .is_some_and(|age| age >= REGISTRY_LOCK_STALE)
}

/// Window ids that should be closed: those whose owning test process is
/// gone (and is not us). Pure so the dead-owner policy is unit-testable
/// without shelling out.
fn reap_candidates(entries: &[RegistryEntry], me: u32, alive: impl Fn(u32) -> bool) -> Vec<i64> {
    entries
        .iter()
        .filter(|entry| entry.owner_pid != me && !alive(entry.owner_pid))
        .map(|entry| entry.window_id)
        .collect()
}

/// Registry rows worth keeping after a prune: the owning process is still
/// alive **and** the window still exists. Pure for the same reason as
/// [`reap_candidates`].
fn entries_to_keep(
    entries: &[RegistryEntry],
    existing_window_ids: &[i64],
    alive: impl Fn(u32) -> bool,
) -> Vec<RegistryEntry> {
    entries
        .iter()
        .copied()
        .filter(|entry| alive(entry.owner_pid) && existing_window_ids.contains(&entry.window_id))
        .collect()
}

/// Harness that drives Terminal.app via `osascript`.
///
/// On `spawn_shell` opens a fresh window running a login shell with
/// `PATH` augmented to find the workspace's CLI binaries, captures the
/// AppleScript window id, and restores keyboard focus to whichever
/// application was frontmost before the spawn so the developer can
/// keep working without an animated minimize-to-Dock effect. On `Drop`
/// the window is closed without saving.
///
/// ## Why we no longer miniaturize
///
/// Earlier versions of this harness called
/// `set miniaturized of front window to true` to keep the spawned
/// window out of the developer's way. Miniaturize triggers macOS's
/// genie / scale animation which is slow, distracting, and pushes the
/// window into the Dock where it is easy to click by accident. Instead
/// the window is opened with `open -g`, which leaves Terminal.app in the
/// background, and the frontmost application is restored if it changed
/// anyway: the spawned Terminal.app window remains
/// part of normal window-manager z-order but sits behind whatever the
/// developer was working in, with no animation and no risk of stray
/// keystrokes being typed into the test window.
/// Environment variable read by [`AppleTerminalHarness::shared_or_spawn`]
/// to attach to a Terminal.app window that was pre-spawned by an outer
/// process (e.g. the `_test_l2` recipe via `biscuit-harness-broker`).
pub const SHARED_WINDOW_ENV: &str = "BISCUIT_SHARED_APPLE_TERMINAL_WINDOW_ID";

pub struct AppleTerminalHarness {
    window_id: Option<i64>,
    window_tag: Option<String>,
    preserve_capabilities: bool,
    /// When `true`, [`Drop`] closes the Terminal.app window. When
    /// `false` (set by [`AppleTerminalHarness::attach`]) the window
    /// is left alone.
    owned: bool,
}

impl AppleTerminalHarness {
    /// Returns a fresh harness. Call
    /// [`spawn_shell`](TerminalHarness::spawn_shell) to actually open a
    /// Terminal.app window.
    pub fn new() -> Self {
        Self {
            window_id: None,
            window_tag: None,
            preserve_capabilities: false,
            owned: true,
        }
    }

    /// Returns a harness that references an existing Terminal.app
    /// window by id without taking ownership of its lifecycle.
    /// [`Drop`] is a no-op.
    pub fn attach(window_id: i64) -> Self {
        Self {
            window_id: Some(window_id),
            window_tag: None,
            preserve_capabilities: false,
            owned: false,
        }
    }

    /// If [`SHARED_WINDOW_ENV`] is set, returns an
    /// [`attach`](Self::attach)-style harness pointing at the
    /// pre-spawned window. Otherwise spawns a fresh window (owned).
    ///
    /// ## Errors
    ///
    /// Propagates whatever [`spawn_shell`](TerminalHarness::spawn_shell)
    /// returns when no shared window id is available.
    pub fn shared_or_spawn() -> io::Result<Self> {
        Self::shared_or_else(|| {
            let mut h = Self::new();
            h.spawn_shell()?;
            Ok(h)
        })
    }

    /// Like [`shared_or_spawn`](Self::shared_or_spawn), but lets the
    /// caller supply a customised spawn closure for the fallback case
    /// — used by Prose-degradation tests that need
    /// [`preserve_capabilities`](Self::preserve_capabilities)`(true)`.
    ///
    /// The shared (attached) path always returns a vanilla
    /// [`attach`](Self::attach) handle; this is fine because the
    /// `biscuit-harness-broker` spawn already configures the shared
    /// window with `preserve_capabilities(true)`.
    ///
    /// ## Errors
    ///
    /// Returns whatever `spawn` returns when no shared window id is
    /// available.
    pub fn shared_or_else<F>(spawn: F) -> io::Result<Self>
    where
        F: FnOnce() -> io::Result<Self>,
    {
        if let Ok(id) = env::var(SHARED_WINDOW_ENV) {
            let trimmed = id.trim();
            if let Ok(parsed) = trimmed.parse::<i64>() {
                return Ok(Self::attach(parsed));
            }
        }
        spawn()
    }

    /// Suppresses the `FORCE_COLOR=1 CLICOLOR_FORCE=1` exports the
    /// harness otherwise injects into the spawned shell, so capability
    /// detection runs against Terminal.app's natural profile.
    ///
    /// Use this for tests that exercise `Prose` graceful-degradation
    /// paths — Apple Terminal does not implement OSC8 hyperlinks or
    /// double underline, and forcing color flips `bt` into the
    /// `Terminal::new_forced` path which unconditionally enables both,
    /// defeating the very degradation being tested.
    ///
    /// Image / color tests should leave this at the default (`false`).
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use biscuit_test_harness::apple_terminal::AppleTerminalHarness;
    ///
    /// let harness = AppleTerminalHarness::new().preserve_capabilities(true);
    /// ```
    pub fn preserve_capabilities(mut self, yes: bool) -> Self {
        self.preserve_capabilities = yes;
        self
    }

    /// Returns `true` when this harness can be used:
    /// macOS, not in CI, and `osascript` can address Terminal.app.
    ///
    /// The `osascript` probe runs once per process; a test's gate and its
    /// spawn both ask.
    pub fn available() -> bool {
        static TERMINAL_ADDRESSABLE: OnceLock<bool> = OnceLock::new();
        if !cfg!(target_os = "macos") {
            return false;
        }
        if ci_value_is_truthy(env::var("CI").ok().as_deref()) {
            return false;
        }
        *TERMINAL_ADDRESSABLE.get_or_init(|| {
            Command::new("osascript")
                .args(["-e", "id of application \"Terminal\""])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        })
    }

    /// Returns the spawned (or attached) window id, or `None` when no
    /// pane has been associated yet.
    ///
    /// Used by `biscuit-harness-broker spawn apple-terminal` to surface
    /// the freshly spawned window id to the `_test_l2` recipe.
    pub fn spawned_window_id(&self) -> Option<i64> {
        self.window_id
    }

    /// Borrows the spawned window id, panicking with a clear message
    /// when no window has been spawned yet.
    fn window_id(&self) -> i64 {
        self.window_id
            .expect("AppleTerminalHarness::spawn_shell must be called before send_text/capture")
    }

    /// Closes the spawned window without saving its session.
    ///
    /// Best-effort: errors are surfaced via `eprintln!` so a stuck
    /// window in CI is diagnosable, but they are not propagated up.
    fn close_window(&mut self) {
        self.window_tag = None;
        if let Some(id) = self.window_id.take() {
            // Drop our registry row first so the clean close path leaves
            // no residue for the reaper to reconcile.
            unregister_window(id);
            let script = format!(
                "tell application \"Terminal\" to close (every window whose id is {id}) saving no",
            );
            if let Err(err) = Self::run_script_with_timeout(&script, CLEANUP_TIMEOUT) {
                eprintln!("warning: failed to close Terminal.app window {id}: {err}");
                return;
            }
            // Terminal.app processes the close asynchronously; wait until
            // the window actually disappears so the next test doesn't race
            // on a stale window ID.
            let check = format!(
                "tell application \"Terminal\" to return (count of (every window whose id is {id})) as text"
            );
            for attempt in 0..31 {
                if attempt > 0 {
                    std::thread::sleep(Duration::from_millis(100));
                }
                if Self::run_script_with_timeout(&check, Duration::from_secs(2))
                    .is_ok_and(|count| count == "0")
                {
                    break;
                }
            }
        }
    }

    /// Runs an AppleScript `script` and returns its trimmed result, which
    /// must be text (a list result reads as empty).
    ///
    /// ## Errors
    ///
    /// Returns `io::Error` when the script cannot be run, fails, or does
    /// not finish within [`QUERY_TIMEOUT`], with the AppleScript error
    /// embedded in the message.
    fn run_script(script: &str) -> io::Result<String> {
        Self::run_script_with_timeout(script, QUERY_TIMEOUT)
    }

    /// [`run_script`](Self::run_script) with a caller-chosen `timeout`.
    ///
    /// Scripts run in this process's [`osa_helper`]; a one-shot
    /// `osascript -e` runs one only when the helper cannot take it.
    fn run_script_with_timeout(script: &str, timeout: Duration) -> io::Result<String> {
        if let Some(result) = osa_helper::run(script, timeout) {
            return result.map(|out| out.trim().to_string());
        }
        let mut cmd = Command::new("osascript");
        cmd.args(["-e", script]);
        let out = run_with_timeout(&mut cmd, timeout)?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "osascript failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

/// A long-lived `osascript` that runs the harness's AppleScript.
///
/// Every `osascript` process spends about 100 ms loading Terminal.app's
/// scripting dictionary at its first `tell`; a process that stays up pays
/// that once, and each later script takes 10–20 ms. A test makes a dozen
/// such calls, so the helper saves over a second per test, more on a loaded
/// host.
///
/// The helper is a JXA loop that reads one escaped AppleScript source per
/// line from stdin, runs it with `NSAppleScript` inside an AppleScript
/// `try`, and answers one line on stdout: `OK <text>` or `ERR <message>`.
/// (JXA crashes when `executeAndReturnError` is handed an error out-pointer,
/// hence the `try`.) It exits when its stdin closes, which happens when this
/// process exits, so it never outlives the test that started it, and its
/// stderr is discarded so it holds none of the test's output pipes.
mod osa_helper {
    use std::io::{self, BufRead as _, BufReader, Write as _};
    use std::process::{Child, ChildStdin, Command, Stdio};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, PoisonError, mpsc};
    use std::time::Duration;

    /// Requests arrive ASCII-only (see [`encode_request`]), so the loop can
    /// decode stdin chunk by chunk without splitting a UTF-8 sequence.
    /// `availableData.length` reaches JXA as a string, hence `Number(…)`.
    const SCRIPT: &str = r#"ObjC.import('Foundation');
const input = $.NSFileHandle.fileHandleWithStandardInput;
const output = $.NSFileHandle.fileHandleWithStandardOutput;
const MARK = '__biscuit_osa_error__';
let buffer = '';
function readLine() {
  while (buffer.indexOf('\n') < 0) {
    const data = input.availableData;
    if (Number(data.length) === 0) return null;
    buffer += $.NSString.alloc.initWithDataEncoding(data, $.NSASCIIStringEncoding).js;
  }
  const end = buffer.indexOf('\n');
  const line = buffer.slice(0, end);
  buffer = buffer.slice(end + 1);
  return line;
}
function decode(s) {
  return s.replace(/\\(u\{([0-9a-f]+)\}|.)/g, (m, e, hex) =>
    hex ? String.fromCodePoint(parseInt(hex, 16)) : e === 'n' ? '\n' : e === 'r' ? '\r' : e);
}
function encode(s) {
  return s.replace(/\\/g, '\\\\').replace(/\n/g, '\\n').replace(/\r/g, '\\r');
}
function reply(status, text) {
  output.writeData($(status + ' ' + encode(text) + '\n').dataUsingEncoding($.NSUTF8StringEncoding));
}
for (;;) {
  const line = readLine();
  if (line === null) break;
  const source = 'try\n' + decode(line) + '\non error errorText number errorNumber\nreturn "' + MARK + '" & errorText & " (" & errorNumber & ")"\nend try';
  const result = $.NSAppleScript.alloc.initWithSource($(source)).executeAndReturnError(null);
  if (result.isNil()) { reply('ERR', 'the script did not compile'); continue; }
  const value = result.stringValue;
  const text = value.isNil() ? '' : value.js;
  if (text.startsWith(MARK)) reply('ERR', text.slice(MARK.length)); else reply('OK', text);
}"#;

    struct Helper {
        child: Child,
        stdin: ChildStdin,
        replies: mpsc::Receiver<String>,
    }

    static HELPER: Mutex<Option<Helper>> = Mutex::new(None);
    static UNAVAILABLE: AtomicBool = AtomicBool::new(false);

    /// Runs `script` in the helper, starting it on first use.
    ///
    /// Returns `None` when the script was never delivered (no helper could
    /// be started, or writing to it failed), so the caller can run it with a
    /// one-shot `osascript`. Once delivered the script may have run, so a
    /// timeout or a helper that died answers `Some(Err(…))` rather than
    /// inviting a second run; the helper is then stopped and the next call
    /// starts a fresh one.
    pub(super) fn run(script: &str, timeout: Duration) -> Option<io::Result<String>> {
        if UNAVAILABLE.load(Ordering::Relaxed) {
            return None;
        }
        let mut slot = HELPER.lock().unwrap_or_else(PoisonError::into_inner);
        if slot.is_none() {
            match spawn() {
                Ok(helper) => *slot = Some(helper),
                Err(_) => {
                    UNAVAILABLE.store(true, Ordering::Relaxed);
                    return None;
                }
            }
        }
        let helper = slot.as_mut()?;
        let request = encode_request(script);
        let delivered = helper
            .stdin
            .write_all(request.as_bytes())
            .and_then(|()| helper.stdin.flush());
        if delivered.is_err() {
            stop(slot.take());
            return None;
        }
        match helper.replies.recv_timeout(timeout) {
            Ok(reply) => Some(parse_reply(&reply)),
            Err(err) => {
                stop(slot.take());
                Some(Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("osascript helper gave no answer: {err}"),
                )))
            }
        }
    }

    fn spawn() -> io::Result<Helper> {
        let mut child = Command::new("osascript")
            .args(["-l", "JavaScript", "-e", SCRIPT])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other("osascript helper has no pipes"));
        };
        let (sender, replies) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        Ok(Helper {
            child,
            stdin,
            replies,
        })
    }

    fn stop(helper: Option<Helper>) {
        if let Some(mut helper) = helper {
            let _ = helper.child.kill();
            let _ = helper.child.wait();
        }
    }

    /// Encodes `script` as one ASCII request line: backslash, CR, and LF
    /// are escaped, and every non-ASCII character becomes `\u{hex}`.
    pub(super) fn encode_request(script: &str) -> String {
        let mut line = String::with_capacity(script.len() + 1);
        for ch in script.chars() {
            match ch {
                '\\' => line.push_str("\\\\"),
                '\n' => line.push_str("\\n"),
                '\r' => line.push_str("\\r"),
                c if c.is_ascii() => line.push(c),
                c => line.push_str(&format!("\\u{{{:x}}}", u32::from(c))),
            }
        }
        line.push('\n');
        line
    }

    /// Parses one helper reply line into the script's text or its error.
    pub(super) fn parse_reply(reply: &str) -> io::Result<String> {
        if let Some(text) = reply.strip_prefix("OK ") {
            Ok(decode_reply_text(text))
        } else if let Some(message) = reply.strip_prefix("ERR ") {
            Err(io::Error::other(format!(
                "osascript failed: {}",
                decode_reply_text(message)
            )))
        } else {
            Err(io::Error::other(format!(
                "malformed osascript helper reply {reply:?}"
            )))
        }
    }

    /// Reverses the helper's escaping of backslash, CR, and LF.
    fn decode_reply_text(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut chars = text.chars();
        while let Some(ch) = chars.next() {
            if ch != '\\' {
                out.push(ch);
                continue;
            }
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        }
        out
    }
}

/// Removes stale Terminal.app windows created by earlier
/// [`AppleTerminalHarness`] instances whose owning test process no longer
/// exists.
///
/// Runs three independent passes, all safe with a developer's own windows
/// open:
///
/// 1. **Registry pass (primary).** Reads the title-independent window-id
///    registry and closes every recorded window whose owning process is
///    dead — but only when the window *still looks like an idle harness
///    login shell* ([`looks_like_harness_window`]), so a window id that
///    Terminal recycled to host real work is never closed. The registry is
///    then pruned under a lock. This is what catches windows whose
///    `custom title` was overwritten by the login shell (Pitfall 2).
/// 2. **Title pass (secondary).** The legacy scan for windows still
///    carrying a `biscuit-test-terminal-<pid>-<seq>` custom title with a
///    dead `<pid>`. Cheap, and catches windows that kept their tag.
/// 3. **Self-healing sweep ([`sweep_legacy_apple_terminal_windows`]).**
///    Catches leaks the registry cannot see (macOS-restored windows,
///    pre-registry leaks, leaks recorded under a different `$TMPDIR`). Runs
///    when the open-window count exceeds [`LEGACY_WINDOW_LIMIT`] or the
///    opt-in env var is set, mirroring the WezTerm sweep.
///
/// Before those passes, [`quit_terminal_if_only_disposable`] handles the case
/// `close` cannot: invisible zero-tab **husks**. `close … saving no` does not
/// destroy a Terminal.app window on current macOS — it leaves a `visible
/// false` husk that no further `close` removes, and the only reliable
/// recovery is to quit the app. Zero-tab windows are closed next
/// ([`close_tabless_windows`]).
///
/// Every pass reads one shared [`snapshot_windows`] enumeration, because this
/// runs once per test process and is that process's first AppleScript call,
/// the one that pays to load Terminal.app's scripting dictionary; the sweep
/// re-enumerates only when an earlier pass closed something.
pub fn cleanup_stale_apple_terminal_windows() {
    if !AppleTerminalHarness::available() {
        return;
    }
    let Some(snapshot) = snapshot_windows() else {
        return;
    };
    let ids: Vec<i64> = snapshot.windows.iter().map(|w| w.id).collect();
    // Husk recovery first: if the window list has degenerated into only
    // leaked harness windows and unclosceable husks, quitting Terminal is the
    // one thing that clears husks. It relaunches clean on the next spawn, so
    // the per-window passes below have nothing left to do.
    if quit_terminal_if_only_disposable(&ids) {
        return;
    }
    // Only a window with no tabs, or one whose id cannot be read, gives
    // the object-based tabless pass anything to close.
    if snapshot.window_count > snapshot.windows.len()
        || snapshot.windows.iter().any(|w| w.tab_count == 0)
    {
        close_tabless_windows();
    }
    reap_registered_windows();
    reap_titled_windows(&snapshot.windows);
    delete_stale_harness_settings(&snapshot.harness_settings);
    let sweep_ids = if CLEANUP_CLOSED_WINDOWS.load(Ordering::Relaxed) {
        existing_window_ids()
    } else {
        Some(ids)
    };
    if let Some(sweep_ids) = sweep_ids {
        sweep_legacy_apple_terminal_windows(&sweep_ids);
    }
}

/// Terminal.app's windows as the cleanup passes see them.
struct WindowsSnapshot {
    /// Every window Terminal.app reports, including any whose id cannot be
    /// read and is therefore missing from `windows`.
    window_count: usize,
    windows: Vec<WindowSnapshot>,
    /// Names of settings sets carrying the harness tag, which a spawn
    /// imports and deletes again; one still present outlived its spawn.
    harness_settings: Vec<String>,
}

/// One Terminal.app window as the cleanup passes see it.
struct WindowSnapshot {
    id: i64,
    tab_count: usize,
    /// Invisible without being miniaturized: what `close … saving no` leaves
    /// behind, which no further `close` removes.
    husk: bool,
    /// Empty when the window has no custom title or it cannot be read.
    custom_title: String,
}

/// Enumerates every Terminal.app window whose id can be read, with its tab
/// count and custom title, and every harness-tagged settings set, in one
/// `osascript` call. Returns `None` when the enumeration script itself
/// fails.
fn snapshot_windows() -> Option<WindowsSnapshot> {
    // `tab` inside `tell application "Terminal"` names Terminal's tab
    // class, not the character, so the separator is bound outside it.
    let script = format!(
        r#"set sep to (character id 9)
        set out to ""
        tell application "Terminal"
            set out to ((count of windows) as text) & linefeed
            repeat with w in windows
                try
                    set wid to (id of w) as text
                    set tabCount to (count of tabs of w) as text
                    set husk to ((visible of w) is false and (miniaturized of w) is false) as text
                    set t to ""
                    try
                        set t to custom title of w
                    end try
                    set out to out & wid & sep & tabCount & sep & husk & sep & t & linefeed
                end try
            end repeat
            repeat with s in settings sets
                try
                    set n to name of s
                    if n starts with "{prefix}" then set out to out & "settings" & sep & n & linefeed
                end try
            end repeat
        end tell
        return out"#,
        prefix = WINDOW_TITLE_PREFIX,
    );
    let stdout = AppleTerminalHarness::run_script(&script).ok()?;
    let mut lines = stdout.lines();
    let window_count = lines.next()?.trim().parse().ok()?;
    let mut snapshot = WindowsSnapshot {
        window_count,
        windows: Vec::new(),
        harness_settings: Vec::new(),
    };
    for line in lines {
        if let Some(name) = line.strip_prefix("settings\t") {
            snapshot.harness_settings.push(name.to_string());
        } else if let Some(window) = parse_window_snapshot(line) {
            snapshot.windows.push(window);
        }
    }
    Some(snapshot)
}

/// Parses one `id<TAB>tab-count<TAB>husk<TAB>custom-title` line of
/// [`snapshot_windows`] output.
fn parse_window_snapshot(line: &str) -> Option<WindowSnapshot> {
    let mut fields = line.splitn(4, '\t');
    let id = fields.next()?.trim().parse().ok()?;
    let tab_count = fields.next()?.trim().parse().ok()?;
    let husk = fields.next()?.trim() == "true";
    let custom_title = fields.next().unwrap_or("").to_string();
    Some(WindowSnapshot {
        id,
        tab_count,
        husk,
        custom_title,
    })
}

/// Deletes harness settings sets whose owning process is dead.
///
/// A spawn imports a settings set named after its window tag and deletes it
/// again as soon as the window exists; a set is left behind only when the
/// spawning process died in between. A live owner's set may belong to a
/// spawn in progress, so it is left alone.
fn delete_stale_harness_settings(names: &[String]) {
    let me = current_process_id();
    let stale: Vec<&String> = names
        .iter()
        .filter(|name| {
            pid_from_tag(name, WINDOW_TITLE_PREFIX)
                .is_some_and(|pid| pid != me && !process_is_alive(pid))
        })
        .collect();
    if stale.is_empty() {
        return;
    }
    let mut script = String::from("tell application \"Terminal\"\n");
    for name in stale {
        script.push_str(&format!(
            "try\ndelete (every settings set whose name is \"{}\")\nend try\n",
            applescript_escape(name)
        ));
    }
    script.push_str("end tell");
    let _ = AppleTerminalHarness::run_script(&script);
}

/// Quits Terminal.app when every open window is **disposable** and the count
/// has piled up past [`LEGACY_WINDOW_LIMIT`] (or the opt-in sweep env var is
/// set). This is the only recovery for invisible zero-tab husks, which
/// `close` cannot remove.
///
/// A window is disposable when it is an invisible husk
/// ([`window_is_invisible`]) or an idle harness-signature window
/// ([`looks_like_harness_window`]). If **any** window is neither — i.e. a
/// developer's real working window — this refuses to quit and returns
/// `false`, leaving the gentler per-window passes to run. Returns `true` only
/// when it actually quit Terminal.
fn quit_terminal_if_only_disposable(ids: &[i64]) -> bool {
    let forced = env::var("BISCUIT_TEST_HARNESS_SWEEP_LEGACY_APPLE").as_deref() == Ok("1");
    if !forced && ids.len() <= LEGACY_WINDOW_LIMIT {
        return false;
    }
    if ids
        .iter()
        .any(|&id| !window_is_invisible(id) && !looks_like_harness_window(id))
    {
        return false;
    }
    quit_terminal();
    true
}

/// Whether the window with `window_id` is gone or `visible false` — the state
/// a husk left behind by `close … saving no` presents. A window that cannot
/// be enumerated is treated as gone (disposable).
fn window_is_invisible(window_id: i64) -> bool {
    let script = format!(
        r#"tell application "Terminal"
            set matches to (every window whose id is {id})
            if (count of matches) is 0 then return "gone"
            if visible of (first window whose id is {id}) is false then return "hidden"
        end tell
        return "visible""#,
        id = window_id,
    );
    matches!(
        AppleTerminalHarness::run_script(&script).as_deref(),
        Ok("hidden") | Ok("gone")
    )
}

/// Quits Terminal.app and waits for the process to actually exit so the next
/// `do script` relaunches a clean instance rather than racing a quitting one.
/// Best-effort: idle login shells and husks do not trip Terminal's
/// running-process quit prompt, so `quit` returns without blocking.
fn quit_terminal() {
    let _ = AppleTerminalHarness::run_script_with_timeout(
        "tell application \"Terminal\" to quit",
        CLEANUP_TIMEOUT,
    );
    CLEANUP_CLOSED_WINDOWS.store(true, Ordering::Relaxed);
    for _ in 0..30 {
        std::thread::sleep(Duration::from_millis(100));
        let running = Command::new("pgrep")
            .args(["-x", "Terminal"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !running {
            break;
        }
    }
}

/// Closes Terminal.app windows that have no tabs.
///
/// A zero-tab window cannot contain user work or a harness shell, but Terminal's
/// AppleScript `front window` lookup can still resolve to it. Leaving that stale
/// object around misdirects any script that falls back to `front window`, which
/// then fails at `selected tab`.
fn close_tabless_windows() {
    let script = r#"tell application "Terminal"
            set before to count of windows
            repeat with w in windows
                try
                    if (count of tabs of w) is 0 then close w saving no
                end try
            end repeat
            return (before - (count of windows)) as text
        end tell"#;
    // Only a close that removed a window gives a following spawn anything
    // to wait for; a pass that found nothing to close (a window whose id
    // cannot be read is usually a close-confirmation sheet) closes nothing.
    let closed = AppleTerminalHarness::run_script(script)
        .ok()
        .and_then(|out| out.trim().parse::<i64>().ok());
    if closed != Some(0) {
        CLEANUP_CLOSED_WINDOWS.store(true, Ordering::Relaxed);
    }
}

/// Registry pass: close dead-owner windows that still look idle, then
/// prune the registry. See [`cleanup_stale_apple_terminal_windows`].
fn reap_registered_windows() {
    let path = registry_path();
    let entries = read_registry(&path);
    if entries.is_empty() {
        return;
    }
    let me = current_process_id();
    for window_id in reap_candidates(&entries, me, process_is_alive) {
        // Window-id reuse guard: only close a recorded id that still
        // presents as an idle login shell. A recycled id now running real
        // work fails the predicate and is left untouched.
        if looks_like_harness_window(window_id) {
            close_window_by_id(window_id);
        }
    }
    prune_registry(&path);
}

/// Rewrites the registry keeping only rows whose window still exists and
/// whose owner is still alive, under a sidecar lock so concurrent reapers
/// cannot corrupt it. Best-effort: a lock-contention or osascript failure
/// skips the rewrite rather than risk dropping live rows.
fn prune_registry(path: &Path) {
    let Some(existing) = existing_window_ids() else {
        // Couldn't enumerate windows; dropping rows now could lose live
        // ones. Leave the registry for the next reaper.
        return;
    };
    let Some(_lock) = RegistryLock::acquire(&registry_lock_path_for(path)) else {
        return;
    };
    // Re-read under the lock so we rewrite against the current contents.
    let kept = entries_to_keep(&read_registry(path), &existing, process_is_alive);
    let mut out = String::with_capacity(kept.len() * 48);
    for entry in kept {
        out.push_str(&entry.to_json_line());
        out.push('\n');
    }
    let _ = fs::write(path, out);
}

/// Title pass: close windows whose `custom title` still carries a
/// dead owner's `biscuit-test-terminal-<pid>-<seq>` tag.
fn reap_titled_windows(windows: &[WindowSnapshot]) {
    for window in windows {
        // A tabless window was already closed by the tabless pass, and a
        // husk keeps its tag but cannot be closed again; only
        // `quit_terminal_if_only_disposable` clears husks.
        if window.tab_count == 0
            || window.husk
            || !window.custom_title.starts_with(WINDOW_TITLE_PREFIX)
        {
            continue;
        }
        let Some(pid) = pid_from_tag(&window.custom_title, WINDOW_TITLE_PREFIX) else {
            continue;
        };
        if pid == current_process_id() || process_is_alive(pid) {
            continue;
        }
        close_window_by_id(window.id);
    }
}

/// Returns the ids of every currently open Terminal.app window, or `None`
/// when the enumeration script fails (so callers can distinguish "no
/// windows" from "couldn't ask").
fn existing_window_ids() -> Option<Vec<i64>> {
    let script = r#"set out to ""
        tell application "Terminal"
            repeat with w in windows
                set out to out & ((id of w) as text) & linefeed
            end repeat
        end tell
        return out"#;
    let stdout = AppleTerminalHarness::run_script(script).ok()?;
    Some(
        stdout
            .lines()
            .filter_map(|line| line.trim().parse::<i64>().ok())
            .collect(),
    )
}

/// Whether the window with `window_id` still presents as an idle harness
/// login shell: it exists, no tab is `busy`, every tab process is a bare
/// login shell (no foreground program), geometry is the default 80×24, and
/// the title is empty or the generic "Terminal".
///
/// Gates every registry-driven close so a window id that Terminal recycled
/// to host real work — or one a developer is actively using — is never
/// closed. Any osascript failure is treated as "not a harness window"
/// (returns `false`), erring toward leaving windows open.
///
/// ## Why a shell allowlist, not [`super::detect_shell`]
///
/// The harness execs the shell [`super::detect_shell`] picked, but a
/// Terminal.app window reports the *login* shell macOS started it with — the user's
/// `UserShell` (commonly `zsh`, shown as `-zsh`), not whatever
/// `detect_shell()` (which prefers `bash`) selected. Matching only the
/// detected shell therefore never recognized a real leaked window, so they
/// accumulated and slowed every full-window osascript scan. We instead
/// accept any common interactive login shell (with or without the leading
/// `-` login marker). The narrow blast radius is preserved by the other
/// conjuncts: not busy, default 80×24 geometry, empty/"Terminal" title.
///
/// This predicate is also used by the opt-in legacy sweep
/// ([`sweep_legacy_apple_terminal_windows`]).
fn looks_like_harness_window(window_id: i64) -> bool {
    // `set w to item 1 of (every window whose id …)` then `processes of t`
    // produces a nested element-reference chain that fails `p as text` with
    // AppleScript error -1700. Use a direct `first window whose id …`
    // reference and materialize `processes of t` into a list variable first,
    // which coerces cleanly. (This bug previously made the predicate error
    // on *every* window, so the reaper never closed anything.)
    let script = format!(
        r#"tell application "Terminal"
            set matches to (every window whose id is {id})
            if (count of matches) is 0 then return "missing"
            set w to (first window whose id is {id})
            repeat with t in tabs of w
                if busy of t is true then return "busy"
                set procList to processes of t
                repeat with p in procList
                    set pname to (p as text)
                    if pname starts with "-" and (length of pname) > 1 then set pname to (text 2 thru -1 of pname)
                    if pname is not "login" and pname is not "bash" and pname is not "zsh" and pname is not "sh" and pname is not "dash" and pname is not "fish" then return "busy"
                end repeat
                set cols to number of columns of t
                set rows to number of rows of t
                if cols is not 80 or rows is not 24 then return "geometry"
            end repeat
            set ttitle to custom title of w
            if ttitle is not "" and ttitle is not "Terminal" and ttitle does not start with "{prefix}" then return "title"
        end tell
        return "idle""#,
        id = window_id,
        prefix = WINDOW_TITLE_PREFIX,
    );
    matches!(AppleTerminalHarness::run_script(&script).as_deref(), Ok("idle"))
}

/// Sweep for Terminal.app windows that the registry pass cannot see —
/// macOS-restored windows (new ids), pre-registry leaks, and windows leaked
/// under a different `$TMPDIR` (so their registry rows live in another file).
///
/// ## Trigger
///
/// Mirrors the self-healing WezTerm sweep ([`super::wezterm`]'s
/// `cleanup_stale_wezterm_panes`): runs when **either** the opt-in env var
/// is set **or** the open-window count exceeds [`LEGACY_WINDOW_LIMIT`].
/// WezTerm can sweep unconditionally inside its isolated `biscuit-bg`
/// workspace; Terminal.app has no workspace isolation, so the count gate is
/// what keeps a developer's handful of real windows safe while still letting
/// an abnormal pile-up of leaked test windows self-heal without an env var.
///
/// ```bash
/// BISCUIT_TEST_HARNESS_SWEEP_LEGACY_APPLE=1 cargo test …
/// ```
///
/// Every open window is then tested with [`looks_like_harness_window`] and
/// only matches are closed: the predicate requires default 80×24 geometry,
/// an empty/"Terminal"/harness-tag title, and only idle login-shell
/// processes — a match a developer's working window effectively never has.
fn sweep_legacy_apple_terminal_windows(ids: &[i64]) {
    let forced = env::var("BISCUIT_TEST_HARNESS_SWEEP_LEGACY_APPLE").as_deref() == Ok("1");
    if !forced && ids.len() <= LEGACY_WINDOW_LIMIT {
        return;
    }
    for &id in ids {
        if looks_like_harness_window(id) {
            close_window_by_id(id);
        }
    }
}

impl Default for AppleTerminalHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AppleTerminalHarness {
    fn drop(&mut self) {
        if self.owned {
            self.close_window();
        }
    }
}

/// Closes the Terminal.app window with the given id via AppleScript.
/// Used by `biscuit-harness-broker kill` to tear down shared windows
/// whose `AppleTerminalHarness` was leaked in a different process.
/// Best-effort: any error is swallowed.
pub fn close_window_by_id(window_id: i64) {
    let script = format!(
        "tell application \"Terminal\" to close (every window whose id is {window_id}) saving no",
    );
    let _ = AppleTerminalHarness::run_script_with_timeout(&script, CLEANUP_TIMEOUT);
    CLEANUP_CLOSED_WINDOWS.store(true, Ordering::Relaxed);
}

impl TerminalHarness for AppleTerminalHarness {
    /// Opens a fresh Terminal.app window running a login shell and
    /// captures its AppleScript window id.
    ///
    /// Focus handling: the window is opened with `open -g`, which leaves
    /// Terminal.app in the background, and the frontmost application is
    /// brought back to the front if the spawn displaced it anyway, so the
    /// spawned window sits behind the developer's current work without
    /// being miniaturized. There is no Dock animation and the test window
    /// cannot accidentally receive keystrokes meant for the developer's
    /// foreground app.
    ///
    /// The window runs the harness's two-stage login shell directly, as
    /// its shell, through a transient settings file rather than typing it
    /// into the profile's own shell; the interactive stage runs with rc
    /// files suppressed — see
    /// [`configure_login_shell`](super::configure_login_shell).
    ///
    /// Cargo's target binary directory is prepended to `PATH` so CLI
    /// binaries (`bt`, `question`) resolve without an absolute path.
    /// Color-forcing env vars (`FORCE_COLOR`, `CLICOLOR_FORCE`) are set
    /// by default so SGR output in captures is deterministic —
    /// Terminal.app's plain-text capture strips SGR anyway, but the
    /// variables propagate to `bt` and other child processes that gate
    /// output on TTY heuristics. Tests exercising graceful-degradation
    /// paths (OSC8 hyperlink fallback, double-underline degradation)
    /// should opt out via
    /// [`AppleTerminalHarness::preserve_capabilities`] so `bt` runs its
    /// natural detection against Terminal.app's actual capability
    /// profile instead of the unconditionally-capable
    /// `Terminal::new_forced` path. `TERM` and `COLORTERM` are always
    /// exported (when not already inherited) — they fix the TERM family
    /// for deterministic capture rather than forcing the color profile.
    fn spawn_shell(&mut self) -> io::Result<()> {
        if !Self::available() {
            return Err(io::Error::other("Terminal.app via osascript not available"));
        }
        CLEANUP_ONCE.call_once(cleanup_stale_apple_terminal_windows);
        // Let Terminal.app settle after a cleanup close: AppleScript window
        // operations are asynchronous inside Terminal.app, and spawning
        // immediately after a close can race on stale window lists. A
        // harness `Drop` already waits for its own window to disappear, so
        // only a cleanup close needs this.
        if CLEANUP_CLOSED_WINDOWS.swap(false, Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(300));
        }
        let shell = super::detect_shell();
        let window_tag = unique_window_tag();
        let mut shell_cmd = String::new();

        let bin_dir = super::cargo_bin_dir("bt").or_else(|| super::cargo_bin_dir("question"));
        if let Some(bin_dir) = &bin_dir {
            shell_cmd.push_str("BISCUIT_TEST_BIN_DIR=");
            shell_cmd.push_str(&shell_quote(&bin_dir.to_string_lossy()));
            shell_cmd.push(' ');
        }
        if !self.preserve_capabilities {
            // Force-color env vars are gated because they flip `bt`'s
            // `detect_terminal_honoring_force_color` into the
            // `Terminal::new_forced` profile, which unconditionally
            // enables `osc_link_support` and `supports_italic` — that
            // collapses the very Prose graceful-degradation paths that
            // Level-2 capability tests are designed to exercise.
            shell_cmd.push_str("FORCE_COLOR=1 CLICOLOR_FORCE=1 ");
        }
        // TERM / COLORTERM are NOT gated by preserve_capabilities. They
        // pin the TERM family Apple Terminal already advertises in
        // normal use (a truecolor `xterm-256color`-class TERM); dropping
        // them would regress every existing image / color test that
        // depends on a deterministic TERM value, and they do not by
        // themselves trigger the forced-capability path that
        // FORCE_COLOR / CLICOLOR_FORCE do.
        if env::var_os("TERM").is_none() {
            shell_cmd.push_str("TERM=xterm-256color ");
        }
        if env::var_os("COLORTERM").is_none() {
            shell_cmd.push_str("COLORTERM=truecolor ");
        }
        shell_cmd.push_str(&super::login_shell_command_line(&shell, bin_dir.is_some()));

        let launch = LaunchFiles::write(&window_tag, &shell, &shell_cmd)?;

        // ## Why a `.terminal` file instead of `do script`
        //
        // `do script` with no target window starts the *profile's* shell —
        // the developer's interactive login shell, rc files included — and
        // only then types the harness command into it. That startup cost
        // 1.5–2.5 s per window on a development Mac and owes the harness
        // nothing. Opening a settings file whose `CommandString` runs the
        // launcher script (`RunCommandAsShell`) makes Terminal.app run it
        // as the window's shell instead, still under `login -pf` and with
        // Terminal's own environment (`TERM_PROGRAM`, `TERM`, `LANG`); see
        // [`LaunchFiles::write`] for why the command names the shell.
        //
        // Opening the file imports it as a settings set named after the
        // window tag. The spawn deletes that set as soon as the window
        // exists (the window keeps running and falls back to the default
        // settings), so the developer's profile list is left as it was;
        // [`cleanup_stale_apple_terminal_windows`] deletes any set a crashed
        // run left behind. No existing profile, and never the default, is
        // touched.
        //
        // A new window is always created, so unlike `do script` the spawn
        // can never adopt a pre-existing window; the window is identified by
        // its settings-set name, not by diffing window lists.
        //
        // ## Focus
        //
        // `open -g` opens the file without bringing Terminal.app forward,
        // and nothing here calls `activate` — spawning is focus-free, a hard
        // invariant: the harness runs while the developer works. As a guard,
        // the frontmost application is snapshotted first and restored if it
        // changed anyway.
        //
        // The snapshot uses StandardAdditions' `path to frontmost
        // application`, not System Events: a System Events `whose frontmost`
        // query cost 2.1 s per spawn on a development Mac (and times out
        // outright when System Events is wedged), while `path to` answers
        // in milliseconds. System Events is consulted only when the
        // frontmost application actually changed.
        //
        // The restore step uses `System Events` to re-frontmost the
        // captured process by *bundle identifier* rather than
        // `tell application prevApp to activate`. Resolving an application
        // by name goes through LaunchServices, and a process name that
        // matches no installed `.app` bundle (e.g. `wezterm-gui`, many
        // Electron helpers) pops a "Choose Application — Where is X?"
        // dialog that blocks the test until the developer dismisses it.
        let script = format!(
            r#"set prevApp to ""
            try
                set prevApp to (path to frontmost application as text)
            end try
            do shell script "/usr/bin/open -g -a Terminal " & quoted form of "{settings_path}"
            set winId to 0
            repeat 200 times
                tell application "Terminal"
                    repeat with w in windows
                        try
                            if (name of current settings of selected tab of w) is "{window_tag}" then set winId to (id of w)
                        end try
                    end repeat
                end tell
                if winId is not 0 then exit repeat
                delay 0.05
            end repeat
            tell application "Terminal"
                if winId is not 0 then
                    try
                        set custom title of (first window whose id is winId) to "{window_tag}"
                    end try
                end if
                try
                    delete (every settings set whose name is "{window_tag}")
                end try
            end tell
            if prevApp is not "" then
                try
                    if (path to frontmost application as text) is not prevApp then
                        set prevBundleId to id of application prevApp
                        tell application "System Events" to set frontmost of (first process whose bundle identifier is prevBundleId) to true
                    end if
                end try
            end if
            return winId as text"#,
            settings_path = applescript_escape(&launch.settings.to_string_lossy()),
            window_tag = applescript_escape(&window_tag),
        );
        let stdout = Self::run_script(&script)?;
        let id: i64 = stdout
            .trim()
            .parse()
            .map_err(|e| io::Error::other(format!("unparseable window id {stdout:?}: {e}")))?;
        if id == 0 {
            return Err(io::Error::other(format!(
                "Terminal.app did not open the harness window {window_tag} within 10 s"
            )));
        }
        // The window's shell now owns the launcher script, which deletes
        // itself once it runs.
        launch.keep_script();
        self.window_id = Some(id);
        self.window_tag = Some(window_tag);
        register_window(id);

        // Poll for the shell prompt instead of sleeping a fixed
        // 800 ms — `wait_for_prompt` returns early once `$`/`#`/`%`
        // appears on the last line, with a 5 s ceiling. Once the inner
        // shell's prompt is visible it is reading the tab's input, and
        // `do script … in selected tab` queues later lines on that input
        // in order, so no settle sleep follows.
        super::wait_for_prompt(self)?;

        // When preserve_capabilities is true we must ensure no
        // FORCE_COLOR / CLICOLOR_FORCE leak through from the user's
        // login profile (e.g. .bash_profile).  Unset them now so subsequent
        // `bt` invocations see a clean environment. Every later line
        // queues behind this one on the tab's input, so there is no need
        // to wait for it to finish.
        if self.preserve_capabilities {
            self.send_text(b"unset FORCE_COLOR CLICOLOR_FORCE\n")?;
        }

        Ok(())
    }

    /// Sends a single command line into the spawned tab.
    ///
    /// Bytes are interpreted as UTF-8 and any trailing newline is
    /// stripped before being handed to AppleScript's `do script "…" in
    /// selected tab` — `do script` runs the string as a shell command
    /// regardless of whether it ends in a newline, and a literal `\n`
    /// confuses the AppleScript escape layer.
    ///
    /// ## Errors
    ///
    /// Returns `io::Error` when the bytes are not valid UTF-8 or
    /// `osascript` returns a non-zero exit status.
    fn send_text(&mut self, bytes: &[u8]) -> io::Result<()> {
        let id = self.window_id();
        let s = std::str::from_utf8(bytes)
            .map_err(|e| io::Error::other(format!("send_text non-utf8: {e}")))?;
        let cmd = s.trim_end_matches(['\r', '\n']);
        if cmd.is_empty() {
            return Ok(());
        }
        let script = format!(
            r#"tell application "Terminal"
                do script "{cmd}" in selected tab of (first window whose id is {id})
            end tell"#,
            cmd = applescript_escape(cmd),
            id = id,
        );
        Self::run_script(&script)?;
        Ok(())
    }

    /// Returns the visible plain-text contents of the spawned tab.
    ///
    /// Terminal.app does not expose ANSI/SGR bytes; both `raw` and
    /// `plain` fields of the returned [`CapturedFrame`] hold the same
    /// string.
    fn capture(&mut self) -> io::Result<CapturedFrame> {
        let id = self.window_id();
        let script = format!(
            r#"tell application "Terminal"
                return contents of selected tab of (first window whose id is {id})
            end tell"#,
        );
        let text = Self::run_script(&script)?;
        Ok(CapturedFrame {
            raw: text.clone(),
            plain: text,
        })
    }

    /// Sleeps long enough for `do script` to dispatch and Terminal.app
    /// to redraw. Terminal.app's AppleScript layer is noticeably slower
    /// than `wezterm cli` / `kitty @`, so we wait a bit longer than the
    /// 200 ms default.
    fn settle(&self) {
        std::thread::sleep(Duration::from_millis(400));
    }
}

/// Escapes a string so it can be embedded inside an AppleScript
/// double-quoted literal.
///
/// AppleScript string literals support only `\"` and `\\` as escapes
/// inside `"..."`. Newlines and tabs are not legal inside the literal
/// at all; we replace them with a `" & linefeed & "` /
/// `" & tab & "` concatenation so the resulting AppleScript still
/// parses.
///
/// ## Byte contract
///
/// Bytes outside printable UTF-8 plus LF (0x0A) and HT (0x09) are not
/// escaped. CR (0x0D), NUL (0x00), BEL (0x07), ESC (0x1B), and the
/// Unicode line / paragraph separators (U+2028 / U+2029) will produce
/// an AppleScript syntax error and are rejected via `debug_assert!` in
/// debug builds. Release builds pass the bytes through unmodified to
/// preserve best-effort behaviour.
///
/// ## Allocation
///
/// The output buffer is pre-allocated based on input shape: line or
/// tab characters expand to ~17 bytes apiece (`" & linefeed & "`), so
/// inputs containing either receive `s.len() * 2` capacity. Plain
/// inputs use the existing `s.len() + 8` budget.
///
/// ## Panics
///
/// In debug builds, panics via `debug_assert!` when `s` contains any
/// byte forbidden by the contract above.
fn applescript_escape(s: &str) -> String {
    let needs_concat = s.contains(['\n', '\t']);
    let mut out = if needs_concat {
        String::with_capacity(s.len() * 2)
    } else {
        String::with_capacity(s.len() + 8)
    };
    for ch in s.chars() {
        debug_assert!(
            is_applescript_safe(ch),
            "applescript_escape: forbidden character U+{:04X} would break AppleScript parse",
            ch as u32,
        );
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\" & linefeed & \""),
            '\t' => out.push_str("\" & tab & \""),
            _ => out.push(ch),
        }
    }
    out
}

/// Returns `true` when `ch` can be safely embedded in an AppleScript
/// double-quoted literal (after [`applescript_escape`] handles the
/// special-case characters `\\`, `"`, `\n`, `\t`).
///
/// Forbidden characters: CR (0x0D), NUL (0x00), BEL (0x07), ESC
/// (0x1B), and the Unicode line / paragraph separators (U+2028 /
/// U+2029). Each of these triggers a parse error in `osascript`.
fn is_applescript_safe(ch: char) -> bool {
    !matches!(
        ch,
        '\r' | '\u{0000}' | '\u{0007}' | '\u{001B}' | '\u{2028}' | '\u{2029}'
    )
}

fn unique_window_tag() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    // Owner first (what the reaper checks), then the spawning process so
    // test processes sharing an owner under `_test_l2` never reuse a tag.
    format!(
        "{WINDOW_TITLE_PREFIX}{}-{}-{n}",
        super::owner_process_id(),
        current_process_id()
    )
}

/// The two files a spawn hands Terminal.app: a settings file that names the
/// launcher script as its shell, and the launcher script itself.
///
/// Dropping the value deletes the settings file, which Terminal.app has
/// imported by the time the window exists. The launcher script is deleted
/// too unless [`keep_script`](Self::keep_script) was called: once a window
/// has started it, the script unlinks itself, and deleting it earlier would
/// race the shell that is about to read it.
struct LaunchFiles {
    settings: PathBuf,
    script: PathBuf,
    keep_script: bool,
}

impl LaunchFiles {
    /// Writes the launcher script running `shell_cmd` and the settings file
    /// that runs it with `/bin/{shell}`, both named after `window_tag`.
    ///
    /// The settings file names `/bin/{shell}`, not the script, as the
    /// window's command: Terminal.app counts the tab as idle only while its
    /// process carries the command's name, and the shell the script `exec`s
    /// is `{shell}`. A tab running anything else reports `busy`, which makes
    /// closing it raise a confirmation sheet and makes the stale-window
    /// reaper skip it.
    ///
    /// ## Errors
    ///
    /// Returns the `io::Error` of a failed write.
    fn write(window_tag: &str, shell: &str, shell_cmd: &str) -> io::Result<Self> {
        // Terminal.app splits `CommandString` on whitespace, so the script
        // path must not contain any.
        let mut dir = registry_dir();
        if dir.to_string_lossy().chars().any(char::is_whitespace) {
            dir = PathBuf::from("/tmp");
        }
        let files = Self {
            settings: dir.join(format!("{window_tag}.terminal")),
            script: dir.join(format!("{window_tag}.sh")),
            keep_script: false,
        };
        fs::write(&files.script, format!("rm -f \"$0\"\n{shell_cmd}\n"))?;
        let command = format!("/bin/{shell} {}", files.script.to_string_lossy());
        fs::write(
            &files.settings,
            terminal_settings_plist(window_tag, &command),
        )?;
        Ok(files)
    }

    fn keep_script(mut self) {
        self.keep_script = true;
    }
}

impl Drop for LaunchFiles {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.settings);
        if !self.keep_script {
            let _ = fs::remove_file(&self.script);
        }
    }
}

/// A Terminal.app settings (`.terminal`) document named `name` that runs
/// `command` as the window's shell in place of the profile's login shell.
fn terminal_settings_plist(name: &str, command: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>name</key>
	<string>{name}</string>
	<key>type</key>
	<string>Window Settings</string>
	<key>ProfileCurrentVersion</key>
	<real>2.07</real>
	<key>CommandString</key>
	<string>{command}</string>
	<key>RunCommandAsShell</key>
	<true/>
</dict>
</plist>
"#,
        name = xml_escape(name),
        command = xml_escape(command),
    )
}

/// Escapes the characters XML text content cannot hold literally.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Wraps `s` in single quotes and escapes embedded single quotes using
/// the POSIX `'\''` trick, matching the convention used by
/// [`crate::TerminalHarness::send_command_with_env`].
fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserve_capabilities_default_is_false() {
        let h = AppleTerminalHarness::new();
        assert!(!h.preserve_capabilities);
    }

    #[test]
    fn preserve_capabilities_setter_toggles() {
        let h = AppleTerminalHarness::new().preserve_capabilities(true);
        assert!(h.preserve_capabilities);
    }

    #[test]
    fn applescript_escape_double_quote() {
        assert_eq!(applescript_escape(r#"a"b"#), r#"a\"b"#);
    }

    #[test]
    fn applescript_escape_backslash() {
        assert_eq!(applescript_escape(r"a\b"), r"a\\b");
    }

    #[test]
    fn applescript_escape_newline_concat() {
        assert_eq!(applescript_escape("a\nb"), "a\" & linefeed & \"b");
    }

    #[test]
    fn applescript_escape_tab_concat() {
        assert_eq!(applescript_escape("a\tb"), "a\" & tab & \"b");
    }

    #[test]
    fn applescript_escape_plain_passthrough() {
        assert_eq!(applescript_escape("hello world"), "hello world");
    }

    #[test]
    fn applescript_escape_combined() {
        // The shell command we'd send for the OSC8 fixture must round-trip
        // through the escape layer without losing its inner quotes.
        let s = r#"bt prose "<a href=\"https://example.com\">click here</a>""#;
        let got = applescript_escape(s);
        assert!(got.contains(r#"\""#));
        assert!(!got.contains('\n'));
    }

    /// The `do script` payload crosses two quoting layers — POSIX single
    /// quotes around the `-c` script, then AppleScript's own escaping — and
    /// has to survive both intact.
    #[test]
    fn login_shell_command_line_survives_the_applescript_escape() {
        let line = super::super::login_shell_command_line("bash", true);
        let escaped = applescript_escape(&format!("BISCUIT_TEST_BIN_DIR='/b' {line}"));
        assert!(
            escaped.contains(r#"-c 'unset ENV BASH_ENV; export PATH=\"$BISCUIT_TEST_BIN_DIR:$PATH\"; unset BISCUIT_TEST_BIN_DIR; exec \"$0\" --norc -i' bash"#),
            "{escaped}"
        );
        assert!(!escaped.contains('\n'), "a newline would break the literal");
    }

    #[test]
    fn shell_quote_simple() {
        assert_eq!(shell_quote("/usr/local/bin"), "'/usr/local/bin'");
    }

    #[test]
    fn shell_quote_embedded_single_quote() {
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn registry_entry_json_round_trips() {
        let entry = RegistryEntry {
            window_id: 4_815_162_342,
            owner_pid: 90_210,
            seq: 7,
        };
        let parsed = RegistryEntry::from_json_line(&entry.to_json_line());
        assert_eq!(parsed, Some(entry));
    }

    #[test]
    fn registry_from_json_line_rejects_malformed() {
        assert_eq!(RegistryEntry::from_json_line(""), None);
        assert_eq!(RegistryEntry::from_json_line("   "), None);
        assert_eq!(RegistryEntry::from_json_line("not json"), None);
        // Missing `seq` field.
        assert_eq!(
            RegistryEntry::from_json_line(r#"{"window_id":1,"owner_pid":2}"#),
            None
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn registry_path_uses_tmpdir_or_tmp() {
        let path = registry_path();
        assert!(path.ends_with(REGISTRY_FILE_NAME));
        assert!(path.is_absolute());
    }

    #[test]
    fn append_then_read_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(REGISTRY_FILE_NAME);
        let entries = [
            RegistryEntry { window_id: 10, owner_pid: 1, seq: 0 },
            RegistryEntry { window_id: 20, owner_pid: 1, seq: 1 },
            RegistryEntry { window_id: 30, owner_pid: 2, seq: 2 },
        ];
        for entry in entries {
            append_registry_entry(&path, entry);
        }
        assert_eq!(read_registry(&path), entries.to_vec());
    }

    #[test]
    fn read_registry_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(REGISTRY_FILE_NAME);
        assert!(read_registry(&path).is_empty());
    }

    #[test]
    fn read_registry_skips_malformed_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(REGISTRY_FILE_NAME);
        let valid = RegistryEntry { window_id: 42, owner_pid: 7, seq: 0 };
        let mut content = String::new();
        content.push_str("garbage line\n");
        content.push_str(&valid.to_json_line());
        content.push('\n');
        content.push('\n');
        content.push_str(r#"{"window_id":99}"#);
        content.push('\n');
        std::fs::write(&path, content).unwrap();
        assert_eq!(read_registry(&path), vec![valid]);
    }

    #[test]
    fn remove_registry_entry_drops_only_matching_window() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(REGISTRY_FILE_NAME);
        let keep_a = RegistryEntry { window_id: 10, owner_pid: 1, seq: 0 };
        let drop_it = RegistryEntry { window_id: 20, owner_pid: 1, seq: 1 };
        let keep_b = RegistryEntry { window_id: 30, owner_pid: 2, seq: 2 };
        for entry in [keep_a, drop_it, keep_b] {
            append_registry_entry(&path, entry);
        }
        remove_registry_entry(&path, 20);
        assert_eq!(read_registry(&path), vec![keep_a, keep_b]);
    }

    #[test]
    fn remove_registry_entry_missing_file_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(REGISTRY_FILE_NAME);
        // Must not create the file or panic.
        remove_registry_entry(&path, 1);
        assert!(!path.exists());
    }

    /// Concurrent appends from many threads must each land as a whole,
    /// well-formed line — no interleaving or lost rows.
    #[test]
    fn concurrent_appends_preserve_every_row() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(REGISTRY_FILE_NAME);
        const THREADS: i64 = 16;
        const PER_THREAD: i64 = 25;
        std::thread::scope(|scope| {
            for thread_idx in 0..THREADS {
                let path = path.clone();
                scope.spawn(move || {
                    for n in 0..PER_THREAD {
                        let window_id = thread_idx * PER_THREAD + n;
                        append_registry_entry(
                            &path,
                            RegistryEntry {
                                window_id,
                                owner_pid: thread_idx as u32,
                                seq: n as u64,
                            },
                        );
                    }
                });
            }
        });
        let entries = read_registry(&path);
        assert_eq!(entries.len() as i64, THREADS * PER_THREAD);
        let mut ids: Vec<i64> = entries.iter().map(|e| e.window_id).collect();
        ids.sort_unstable();
        let expected: Vec<i64> = (0..THREADS * PER_THREAD).collect();
        assert_eq!(ids, expected);
    }

    #[test]
    fn reap_candidates_selects_dead_owners_excluding_self() {
        let me = 100;
        let entries = [
            RegistryEntry { window_id: 1, owner_pid: me, seq: 0 }, // self → skip
            RegistryEntry { window_id: 2, owner_pid: 200, seq: 1 }, // dead → reap
            RegistryEntry { window_id: 3, owner_pid: 300, seq: 2 }, // alive → skip
            RegistryEntry { window_id: 4, owner_pid: 400, seq: 3 }, // dead → reap
        ];
        let alive = |pid: u32| pid == me || pid == 300;
        assert_eq!(reap_candidates(&entries, me, alive), vec![2, 4]);
    }

    #[test]
    fn entries_to_keep_requires_alive_owner_and_live_window() {
        let entries = [
            RegistryEntry { window_id: 10, owner_pid: 1, seq: 0 }, // alive + exists → keep
            RegistryEntry { window_id: 20, owner_pid: 2, seq: 1 }, // dead owner → drop
            RegistryEntry { window_id: 30, owner_pid: 1, seq: 2 }, // alive but window gone → drop
        ];
        let existing = [10, 99];
        let alive = |pid: u32| pid == 1;
        assert_eq!(
            entries_to_keep(&entries, &existing, alive),
            vec![entries[0]],
        );
    }

    #[test]
    fn registry_lock_is_exclusive_then_released_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(REGISTRY_LOCK_FILE_NAME);

        let lock = RegistryLock::acquire(&path).expect("first acquire succeeds");
        assert!(path.exists());
        // A second acquire while the fresh lock is held must fail.
        assert!(RegistryLock::acquire(&path).is_none());

        drop(lock);
        assert!(!path.exists(), "drop removes the lock file");
        // Now it can be re-acquired.
        assert!(RegistryLock::acquire(&path).is_some());
    }

    #[test]
    fn fresh_lock_is_not_stale() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(REGISTRY_LOCK_FILE_NAME);
        std::fs::write(&path, b"").unwrap();
        assert!(!lock_is_stale(&path));
    }

    #[test]
    fn unique_window_tag_includes_harness_prefix_and_pid() {
        let tag = unique_window_tag();
        assert!(tag.starts_with(WINDOW_TITLE_PREFIX));
        assert_eq!(pid_from_tag(&tag, WINDOW_TITLE_PREFIX), Some(crate::owner_process_id()));
        assert!(tag.contains(&format!("-{}-", current_process_id())), "{tag}");
        assert_ne!(tag, unique_window_tag());
    }

    /// On non-macOS hosts `available()` must be false unconditionally so
    /// that Linux / CI environments skip these tests cleanly without
    /// shelling out to a missing `osascript`.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn available_is_false_off_macos() {
        assert!(!AppleTerminalHarness::available());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "forbidden character")]
    fn applescript_escape_rejects_cr() {
        let _ = applescript_escape("a\rb");
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "forbidden character")]
    fn applescript_escape_rejects_nul() {
        let _ = applescript_escape("a\u{0000}b");
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "forbidden character")]
    fn applescript_escape_rejects_esc() {
        let _ = applescript_escape("a\u{001B}b");
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "forbidden character")]
    fn applescript_escape_unicode_line_separator_panics() {
        let _ = applescript_escape("a\u{2028}b");
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "forbidden character")]
    fn applescript_escape_unicode_paragraph_separator_panics() {
        let _ = applescript_escape("a\u{2029}b");
    }

    /// Smoke test that the multi-line allocation policy still produces
    /// a coherent output for a 1024-char line/tab-heavy input.
    #[test]
    fn applescript_escape_preallocates_for_multiline() {
        // 1024 chars, half newlines + half tabs interleaved with text.
        let mut input = String::with_capacity(1024);
        for i in 0..256 {
            input.push('a');
            input.push('\n');
            input.push('b');
            input.push('\t');
            // Suppress unused-warning loops.
            let _ = i;
        }
        assert_eq!(input.len(), 1024);
        let got = applescript_escape(&input);
        // Every newline becomes `" & linefeed & "` (16 chars),
        // every tab becomes `" & tab & "` (11 chars), every plain
        // `a` / `b` stays one char.
        let linefeed_repl = "\" & linefeed & \"";
        let tab_repl = "\" & tab & \"";
        let expected_len = 256 * (1 + linefeed_repl.len() + 1 + tab_repl.len());
        assert_eq!(got.len(), expected_len);
        assert!(got.contains(linefeed_repl));
        assert!(got.contains(tab_repl));
    }

    /// Compile-time check: `wait_for_prompt` is the canonical
    /// shell-readiness path used by [`AppleTerminalHarness::spawn_shell`].
    /// This helper is never invoked at runtime; its sole purpose is to
    /// fail compilation if the call signature drifts.
    #[allow(dead_code)]
    fn _wait_for_prompt_path_typecheck(harness: &mut AppleTerminalHarness) {
        let _ = super::super::wait_for_prompt(harness);
    }

    /// CI providers use more than one truthy spelling; GitHub Actions uses
    /// `true`, while local test scripts commonly use `1`.
    #[test]
    fn ci_truthy_values_disable_apple_terminal() {
        for value in ["1", "true", "TRUE", "yes", "on"] {
            assert!(ci_value_is_truthy(Some(value)), "{value}");
        }
        for value in ["", "0", "false", "no", "off"] {
            assert!(!ci_value_is_truthy(Some(value)), "{value}");
        }
        assert!(!ci_value_is_truthy(None));
    }

    /// A request is one ASCII line, whatever the script contains, so the
    /// helper can decode stdin chunk by chunk.
    #[test]
    fn osa_request_is_one_ascii_line() {
        let line = osa_helper::encode_request("say \"a\\b\"\nem—dash 💻\r");
        assert_eq!(line, "say \"a\\\\b\"\\nem\\u{2014}dash \\u{1f4bb}\\r\n");
        assert!(line.is_ascii());
        assert_eq!(line.matches('\n').count(), 1);
    }

    #[test]
    fn osa_reply_decodes_text_and_errors() {
        assert_eq!(
            osa_helper::parse_reply("OK a\\nb\\\\c\\r").unwrap(),
            "a\nb\\c\r"
        );
        assert_eq!(osa_helper::parse_reply("OK ").unwrap(), "");
        let err = osa_helper::parse_reply("ERR boom (42)").unwrap_err();
        assert_eq!(err.to_string(), "osascript failed: boom (42)");
        assert!(osa_helper::parse_reply("garbage").is_err());
    }

    /// Phase-5 static verification: the spawn AppleScript must never call
    /// `activate` or emit a `keystroke` — both would steal foreground focus
    /// from the developer. The harness is focus-free by design: `open -g`
    /// opens the window without activation, and focus is restored to the
    /// previous frontmost process via `set frontmost … to true` (not
    /// `activate`). This test encodes that invariant so a future edit cannot
    /// accidentally regress it.
    #[test]
    fn spawn_script_is_focus_free() {
        let src = include_str!("apple_terminal.rs");
        // Scan raw string literals that contain AppleScript, stopping before
        // this test function so the assertion text itself is not scanned.
        let mut in_raw = false;
        let mut raw_delim = String::new();
        let mut is_terminal_script = false;
        let mut violations = Vec::new();
        for (i, line) in src.lines().enumerate() {
            // Stop before we reach this test — avoids self-scanning the
            // assertion text below.
            if line.contains("fn spawn_script_is_focus_free") {
                break;
            }
            if !in_raw {
                if let Some(pos) = line.find("r\"") {
                    let after_r = &line[pos + 2..];
                    let hashes = after_r.chars().take_while(|c| *c == '#').count();
                    raw_delim = format!("\"{}", "#".repeat(hashes));
                    in_raw = true;
                    is_terminal_script = false;
                }
            } else {
                if line.contains(&raw_delim) {
                    in_raw = false;
                } else {
                    if line.contains("tell application \"Terminal\"") {
                        is_terminal_script = true;
                    }
                    if is_terminal_script {
                        if line.contains("activate") {
                            violations.push(format!("line {}: contains 'activate'", i + 1));
                        }
                        if line.contains("keystroke") {
                            violations.push(format!("line {}: contains 'keystroke'", i + 1));
                        }
                    }
                }
            }
        }
        assert!(
            violations.is_empty(),
            "spawn script violates focus-free invariant:\n{}",
            violations.join("\n")
        );
    }
}
