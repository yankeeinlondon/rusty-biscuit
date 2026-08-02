#![allow(dead_code)]

use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness, strip_ansi};
use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Backend, Level, LevelDecision, decide_harness};

/// Gating decision for Level-2 WezTerm tests.
pub fn wezterm_decision() -> LevelDecision {
    decide_harness!(Level::L2, WezTermHarness::available(), Backend::WezTerm)
}

/// Process-wide shared WezTerm pane reused across every test in this file.
///
/// Spawning a fresh WezTerm window costs ≈2–3 s plus prompt readiness. Every
/// test in this file is `#[serial(level2_terminal)]`, so a single pane is
/// safe to share. [`SharedHarness`] wraps the `Mutex<Option<T>>` +
/// `libc::atexit` cleanup pattern so the pane is killed at process exit
/// instead of leaking into the `biscuit-bg` workspace (Rust does not run
/// `Drop` on `static` values).
pub static SHARED_HARNESS: SharedHarness<WezTermHarness> = SharedHarness::new();

/// Shared headless pane for tests that need deterministic geometry or need
/// `COLORFGBG` to remain authoritative because tmux does not answer OSC-11.
pub static SHARED_TMUX_HARNESS: SharedHarness<TmuxHarness> = SharedHarness::new();

/// Monotonic counter for sentinel uniqueness across tests in this binary.
static SENTINEL_COUNTER: AtomicU32 = AtomicU32::new(0);
static TMUX_SENTINEL_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Absolute path to the `md` binary built for *this* workspace, injected by
/// Cargo at compile time. Using it instead of a bare `md` keeps Level 2 tests
/// from silently passing against a stale `md` installed on the host `PATH`
/// (e.g. `~/.cargo/bin/md`) while the code under review still fails.
pub const MD_BIN: &str = env!("CARGO_BIN_EXE_md");

/// Absolute path to a `md` shim (under the system temp dir) that points at
/// [`MD_BIN`]. Tests invoke this instead of `MD_BIN` directly because the built
/// binary lives under `…/rusty-biscuit/…/target/debug/md`, whose path contains
/// the substring `rust`. Embedding that path in the shell command would put
/// `rust` into the captured command echo, where `find(|l| l.contains("rust"))`
/// anchors (the rust code-fence label) would match the echo instead of the
/// rendered code block. The shim lives under `/var/folders/…`, so the
/// visible command carries no `rust` while still running the freshly built
/// binary rather than whatever `md` happens to be installed on the host.
///
/// Using [`md_shim`] (rather than the bare host `PATH` `md`) is what lets the
/// Level 2 suite prove behavior of the code under review instead of whatever
/// stale `md` is installed on the developer's machine (review-2 finding #1).
///
/// The shim is created via [`link_or_copy`] so the suite stays robust on
/// hosts where symlink creation requires elevated privileges or Developer
/// Mode (notably Windows). See [`link_or_copy`] for the fallback ladder
/// (review-3 finding).
pub fn md_shim() -> &'static str {
    static SHIM: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SHIM.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("dm-md-shim-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create md shim dir");
        // On Windows the shim path needs the `.exe` suffix so the pane
        // resolves it as an executable. On Unix a bare `md` suffices.
        #[cfg(windows)]
        let link = dir.join("md.exe");
        #[cfg(not(windows))]
        let link = dir.join("md");
        // Idempotent across reruns within the same pid: replace any stale link.
        let _ = fs::remove_file(&link);
        link_or_copy(std::path::Path::new(MD_BIN), &link).expect("create md shim");
        // Fail fast if the shim does not actually point at the Cargo-built
        // binary. This catches environment-specific issues (broken link,
        // stale temp dir, permission problems) before the Level 2 pane
        // silently runs the wrong binary.
        assert_shim_resolves_to_built(&link);
        link.to_str().expect("shim path is valid UTF-8").to_string()
    })
}

/// Verifies that the shim at `link` resolves to [`MD_BIN`] (the
/// `CARGO_BIN_EXE_md` path baked in at compile time). Called once per
/// process from [`md_shim`]; failures abort the test binary so the
/// suite cannot silently pass against a host-installed `md`.
///
/// Uses [`is_same_binary`] so the check works regardless of whether the
/// shim was created as a symlink, a hard link, or a copy (review-3 finding).
pub fn assert_shim_resolves_to_built(link: &std::path::Path) {
    let built = std::path::Path::new(MD_BIN);
    if !is_same_binary(link, built) {
        panic!(
            "md shim {} did not resolve to the Cargo-built binary {};\
             tests would run against the wrong binary",
            link.display(),
            built.display(),
        );
    }
}

/// Cross-platform shim creation with a graceful fallback ladder.
///
/// Tries each strategy in order so the Level 2 suite stays robust across
/// host configurations where one method is unavailable (review-3 finding):
///
/// 1. **Symlink** — cleanest; preserves identity through `canonicalize`.
///    On Windows, file symlinks require Developer Mode or elevated
///    privileges.
/// 2. **Hard link** — no extra privileges required on Windows for
///    same-volume files; shares the same inode/file-index as the target
///    so identity comparison still works.
/// 3. **Copy** — always-works fallback for cross-volume temp directories
///    (e.g. Windows temp on `D:` and Cargo target on `C:`); produces a
///    separate inode so the integrity check uses content comparison.
pub fn link_or_copy(target: &std::path::Path, link: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    if std::os::unix::fs::symlink(target, link).is_ok() {
        return Ok(());
    }
    #[cfg(windows)]
    if std::os::windows::fs::symlink_file(target, link).is_ok() {
        return Ok(());
    }
    if std::fs::hard_link(target, link).is_ok() {
        return Ok(());
    }
    std::fs::copy(target, link).map(|_| ())
}

/// Returns `true` if `candidate` and `built` point at the same file or
/// at content-equivalent copies.
///
/// Fast path (Unix only): inode identity. Works for symlinks (after
/// `metadata` follows them) and for hard links.
///
/// Slow path: byte-for-byte content comparison. Handles the copy
/// fallback from [`link_or_copy`] where shim and target are different
/// inodes but identical content. This is the only path on Windows —
/// `MetadataExt::{volume_serial_number, file_index}` are the unstable
/// `windows_by_handle` feature, and `rust-toolchain.toml` pins stable, so
/// the equivalent identity check cannot be written here. Content
/// comparison returns the same answer; it is merely slower.
pub fn is_same_binary(candidate: &std::path::Path, built: &std::path::Path) -> bool {
    #[cfg(unix)]
    if let (Ok(ma), Ok(mb)) = (fs::metadata(candidate), fs::metadata(built)) {
        use std::os::unix::fs::MetadataExt;
        if ma.ino() == mb.ino() && ma.dev() == mb.dev() {
            return true;
        }
    }
    matches!(
        (fs::read(candidate), fs::read(built)),
        (Ok(a), Ok(b)) if a == b
    )
}

/// Maximum wall time we'll spend waiting for a single command's completion
/// sentinel to appear in the pane. Generous — most `md` invocations finish
/// in well under a second; this is a safety net for first-run cold builds.
const SENTINEL_TIMEOUT: Duration = Duration::from_secs(30);

fn wait_for_tmux_sentinel(
    harness: &mut TmuxHarness,
    sentinel: &str,
) -> Result<CapturedFrame, CapturedFrame> {
    let deadline = Instant::now() + SENTINEL_TIMEOUT;
    let mut last = CapturedFrame::from_raw(String::new());
    while Instant::now() < deadline {
        if let Ok(frame) = harness.capture() {
            if frame.plain.lines().any(|line| line.trim() == sentinel) {
                return Ok(frame);
            }
            last = frame;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(last)
}

fn run_tmux_command(
    harness: &mut TmuxHarness,
    command: &str,
    env: &[(&str, &str)],
) -> CapturedFrame {
    let id = TMUX_SENTINEL_COUNTER.fetch_add(1, Ordering::Relaxed);
    let start = format!("__DM_CODE_TMUX_START_{id}__");
    let end = format!("__DM_CODE_TMUX_DONE_{id}__");
    let sequence = format!("printf '\\n{start}\\n'; {command}; printf '\\n{end}\\n'");
    // Environment assignments from the harness prefix only one shell command;
    // this child keeps them active for the whole marked sequence.
    let wrapped = format!("sh -c {}", shell_quote(&sequence));
    harness
        .send_command_with_env(&wrapped, env)
        .expect("send_command_with_env failed");
    match wait_for_tmux_sentinel(harness, &end) {
        Ok(frame) => output_region(frame, &start, &end),
        Err(last) => panic!(
            "timed out waiting for sentinel {end} after {SENTINEL_TIMEOUT:?}. \
             last plain capture:\n{}",
            last.plain
        ),
    }
}

/// Narrows a shared-pane capture to the command's output.
///
/// Printed start and completion markers remain unambiguous when the echoed
/// command wraps through either marker text. The last standalone start before
/// completion wins, excluding prompts and earlier commands from substring and
/// color probes.
fn output_region(frame: CapturedFrame, start: &str, end: &str) -> CapturedFrame {
    let lines: Vec<&str> = frame.raw.split('\n').collect();
    let stripped: Vec<String> = lines.iter().map(|line| strip_ansi(line)).collect();

    let Some(end_index) = stripped.iter().rposition(|line| line.trim() == end) else {
        return frame;
    };
    let Some(start_index) = stripped[..end_index]
        .iter()
        .rposition(|line| line.trim() == start)
    else {
        return frame;
    };

    CapturedFrame::from_raw(lines[start_index + 1..end_index].join("\n"))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Runs the Cargo-built `md` in a headless tmux pane.
pub fn run_md_in_tmux(
    file_body: &str,
    shell_prefix: Option<&str>,
    extra_args: &str,
    env: &[(&str, &str)],
) -> CapturedFrame {
    let dir = tempdir().expect("create markdown fixture directory");
    let file_path = dir.path().join("layout.md");
    fs::write(&file_path, file_body).expect("write markdown fixture");

    let mut guard = SHARED_TMUX_HARNESS
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("tmux harness present");

    run_tmux_command(harness, "clear", &[]);
    // A shared pane may retain a fixture directory deleted by an earlier test.
    // Establishing this live directory also gives FileReference a valid CWD.
    let fixture_dir = shell_quote(&dir.path().to_string_lossy());
    let md_command = format!(
        "cd {fixture_dir} && {} layout.md {extra_args}",
        shell_quote(md_shim())
    );
    let command = shell_prefix
        .map(|prefix| format!("{prefix}; {md_command}"))
        .unwrap_or(md_command);
    run_tmux_command(harness, &command, env)
}

/// Polls the pane every 50 ms looking for `sentinel`, returning the final
/// captured frame once it appears. Returns the last attempted capture on
/// timeout so callers can include it in panic diagnostics.
pub fn wait_for_sentinel(
    harness: &mut WezTermHarness,
    sentinel: &str,
) -> Result<CapturedFrame, CapturedFrame> {
    let deadline = Instant::now() + SENTINEL_TIMEOUT;
    let mut last = CapturedFrame::from_raw(String::new());
    while Instant::now() < deadline {
        if let Ok(frame) = harness.capture() {
            // The sentinel also appears inline in the command echo
            // (e.g. `$ md ...; printf '\n__DM_DONE_0__\n'`). Only treat the
            // sentinel as completion when it appears on a line of its own,
            // which only happens after `printf` actually runs.
            if frame.plain.lines().any(|l| l.trim() == sentinel) {
                return Ok(frame);
            }
            last = frame;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(last)
}

/// Runs `cmd` in the shared pane, appending a unique completion sentinel so
/// we can detect when `cmd` has finished without depending on the user's
/// shell prompt format.
pub fn run_with_sentinel(harness: &mut WezTermHarness, cmd: &str) -> CapturedFrame {
    let id = SENTINEL_COUNTER.fetch_add(1, Ordering::Relaxed);
    let sentinel = format!("__DM_LVL2_DONE_{id}__");
    // `printf` is portable across bash/zsh and emits the sentinel on its own
    // line so it never visually fuses with `md`'s last row.
    let wrapped = format!("{cmd}; printf '\\n{sentinel}\\n'");
    harness
        .send_command_with_env(&wrapped, &[])
        .expect("send_command_with_env failed");
    match wait_for_sentinel(harness, &sentinel) {
        Ok(frame) => frame,
        Err(last) => panic!(
            "timed out waiting for sentinel {sentinel} after {SENTINEL_TIMEOUT:?}. \
             last plain capture:\n{}",
            last.plain
        ),
    }
}

/// Helper: write a markdown fixture, run `md` with the given flags inside the
/// shared harness pane, and return the captured frame.
///
/// Always invokes the Cargo-built `md` via [`md_shim`]; the bare host `PATH`
/// `md` is never used because doing so would let a stale installed binary
/// satisfy the suite while the code under review still fails (review-2
/// finding #1). The shim is also stored under a path that does not contain
/// the substring `rust`, so `find(|l| l.contains("rust"))` anchors (the rust
/// code-fence label) only match the rendered code block, not the command
/// echo.
pub fn run_md(file_body: &str, extra_args: &str) -> Option<(CapturedFrame, std::path::PathBuf)> {
    run_md_env(file_body, extra_args, &[])
}

/// Like [`run_md`] but injects inline environment assignments onto the `md`
/// invocation (e.g. `COLORFGBG` to force light/dark color-mode detection
/// deterministically).
pub fn run_md_env(
    file_body: &str,
    extra_args: &str,
    env: &[(&str, &str)],
) -> Option<(CapturedFrame, std::path::PathBuf)> {
    // `md_shim` is resolved lazily inside `run_md_env_bin` so a host
    // that would skip the Level 2 tier never pays for shim creation
    // (review-3 finding).
    run_md_env_bin(md_shim, file_body, extra_args, env)
}

/// Core of [`run_md_env`] parameterized over the `md` program to invoke.
///
/// `bin` is a closure that returns the binary path. The closure is called
/// only after the Level 2 gate has passed, so closure side effects (such
/// as [`md_shim`]'s filesystem work) are skipped on hosts that would skip
/// the Level 2 tier entirely. This matters on Windows where shim creation
/// may require elevated privileges or Developer Mode; gating first keeps
/// `just test-l2` from failing during setup when the real-terminal tier
/// would skip anyway (review-3 finding).
///
/// Tests should only pass a different `bin` closure when they need to
/// assert behavior that depends on the wrapper name itself (for example,
/// `run_md_after_shell_prefix` builds a `prefix md …` command where `md`
/// must resolve through the pane `PATH`-aware wrapper).
pub fn run_md_env_bin<F>(
    bin: F,
    file_body: &str,
    extra_args: &str,
    env: &[(&str, &str)],
) -> Option<(CapturedFrame, std::path::PathBuf)>
where
    F: FnOnce() -> &'static str,
{
    match wezterm_decision() {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return None;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    // Resolve the binary only after the gate has passed.
    let bin = bin();

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("layout.md");
    fs::write(&file_path, file_body).unwrap();

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();

    // Reset the visible region so the previous test's output does not bleed
    // into this capture. `clear` is portable across bash and zsh.
    run_with_sentinel(harness, "clear");

    let cmd = format!("{bin} {} {}", file_path.display(), extra_args);
    let frame = run_with_sentinel_env(harness, &cmd, env);
    // Keep tempdir alive past capture by returning its path.
    Some((frame, file_path))
}

/// Like [`run_with_sentinel`] but applies inline env assignments to `cmd`.
pub fn run_with_sentinel_env(
    harness: &mut WezTermHarness,
    cmd: &str,
    env: &[(&str, &str)],
) -> CapturedFrame {
    let id = SENTINEL_COUNTER.fetch_add(1, Ordering::Relaxed);
    let sentinel = format!("__DM_LVL2_DONE_{id}__");
    let wrapped = format!("{cmd}; printf '\\n{sentinel}\\n'");
    harness
        .send_command_with_env(&wrapped, env)
        .expect("send_command_with_env failed");
    match wait_for_sentinel(harness, &sentinel) {
        Ok(frame) => {
            // The sentinel only guarantees `md` finished; the pane grid may
            // still be settling (scroll, redraw). Settle and re-capture so
            // assertions see the final stable frame, not a transitional one.
            std::thread::sleep(Duration::from_millis(250));
            harness.capture().unwrap_or(frame)
        }
        Err(last) => panic!(
            "timed out waiting for sentinel {sentinel} after {SENTINEL_TIMEOUT:?}. \
             last plain capture:\n{}",
            last.plain
        ),
    }
}

/// Strip trailing whitespace from a line for visible-width comparisons.
pub fn rtrim(s: &str) -> &str {
    s.trim_end_matches([' ', '\t'])
}


pub fn run_md_after_shell_prefix(
    file_body: &str,
    prefix: &str,
    extra_args: &str,
) -> Option<(CapturedFrame, std::path::PathBuf)> {
    match wezterm_decision() {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return None;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("layout.md");
    fs::write(&file_path, file_body).unwrap();

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();

    run_with_sentinel(harness, "clear");

    // Use the Cargo-built `md` via the shim so prefix-wrapped invocations
    // also verify the code under review (review-2 finding #1).
    let cmd = format!("{prefix} {} {} {extra_args}", md_shim(), file_path.display());
    let frame = run_with_sentinel(harness, &cmd);
    Some((frame, file_path))
}

pub fn luma(r: u32, g: u32, b: u32) -> f32 {
    0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32
}

pub fn max_bg_luma_on_line(raw: &str, needle: &str) -> Option<f32> {
    let re = regex_lite_bg();
    for line in raw.lines().rev() {
        let plain = biscuit_test_harness::strip_ansi(line);
        if plain.contains(needle) {
            let mut best: Option<f32> = None;
            for (r, g, b) in re(line) {
                let l = luma(r, g, b);
                best = Some(best.map_or(l, |m: f32| m.max(l)));
            }
            return best;
        }
    }
    None
}

pub fn min_fg_luma_on_line(raw: &str, needle: &str) -> Option<f32> {
    let re = regex_lite_fg();
    for line in raw.lines().rev() {
        let plain = biscuit_test_harness::strip_ansi(line);
        if plain.contains(needle) {
            let mut best: Option<f32> = None;
            for (r, g, b) in re(line) {
                let l = luma(r, g, b);
                best = Some(best.map_or(l, |m: f32| m.min(l)));
            }
            return best;
        }
    }
    None
}

pub fn max_fg_luma_on_line(raw: &str, needle: &str) -> Option<f32> {
    let re = regex_lite_fg();
    for line in raw.lines().rev() {
        let plain = biscuit_test_harness::strip_ansi(line);
        if plain.contains(needle) {
            let mut best: Option<f32> = None;
            for (r, g, b) in re(line) {
                let l = luma(r, g, b);
                best = Some(best.map_or(l, |m: f32| m.max(l)));
            }
            return best;
        }
    }
    None
}

pub fn regex_lite_bg() -> impl Fn(&str) -> Vec<(u32, u32, u32)> {
    |line: &str| {
        let mut out = Vec::new();
        for chunk in line.split("\x1b[").skip(1) {
            let Some(mend) = chunk.find('m') else {
                continue;
            };
            let nums: Vec<u32> = chunk[..mend]
                .split([';', ':'])
                .filter_map(|s| s.parse::<u32>().ok())
                .collect();
            if nums.len() >= 5 && nums[0] == 48 && nums[1] == 2 {
                out.push((nums[2], nums[3], nums[4]));
            }
        }
        out
    }
}

pub fn regex_lite_fg() -> impl Fn(&str) -> Vec<(u32, u32, u32)> {
    |line: &str| {
        let mut out = Vec::new();
        for chunk in line.split("\x1b[").skip(1) {
            let Some(mend) = chunk.find('m') else {
                continue;
            };
            let nums: Vec<u32> = chunk[..mend]
                .split([';', ':'])
                .filter_map(|s| s.parse::<u32>().ok())
                .collect();
            if nums.len() >= 5 && nums[0] == 38 && nums[1] == 2 {
                out.push((nums[2], nums[3], nums[4]));
            }
        }
        out
    }
}

pub fn raw_line_is_blank(line: &str) -> bool {
    biscuit_test_harness::strip_ansi(line).trim().is_empty() && !line.contains("\x1b[48")
}

pub fn is_sentinel_line(line: &str) -> bool {
    let t = line.trim();
    t.strip_prefix("__DM_LVL2_DONE_")
        .and_then(|r| r.strip_suffix("__"))
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

pub fn rendered_region<'a>(lines: &'a [&'a str], head_needle: &str) -> (usize, usize) {
    let head = lines
        .iter()
        .position(|l| l.contains(head_needle))
        .unwrap_or(0);
    let sentinel = lines
        .iter()
        .rposition(|l| is_sentinel_line(l))
        .unwrap_or(lines.len());
    (head, sentinel)
}

pub fn foreground_at_text(raw: &str, needle: &str) -> Option<Option<(u8, u8, u8)>> {
    let target = raw.find(needle)?;
    let mut fg = None;
    let mut i = 0;
    while i < target {
        let rest = &raw[i..];
        if let Some(after_csi) = rest.strip_prefix("\x1b[")
            && let Some(end) = after_csi.find('m')
        {
            apply_sgr_foreground(&after_csi[..end], &mut fg);
            i += 2 + end + 1;
            continue;
        }
        i += rest.chars().next()?.len_utf8();
    }
    Some(fg)
}

fn apply_sgr_foreground(params: &str, fg: &mut Option<(u8, u8, u8)>) {
    if params.is_empty() {
        *fg = None;
        return;
    }

    for param in params.split(';') {
        match param {
            "0" | "39" => *fg = None,
            colon if colon.starts_with("38:2:") => {
                let values: Vec<u8> = colon
                    .split(':')
                    .filter_map(|part| part.parse::<u8>().ok())
                    .collect();
                if values.len() >= 5 {
                    *fg = Some((values[2], values[3], values[4]));
                }
            }
            _ => {}
        }
    }

    let mut semicolon = params.split(';');
    while let Some(param) = semicolon.next() {
        if param == "38" && semicolon.next() == Some("2") {
            let Some(r) = semicolon.next().and_then(|value| value.parse::<u8>().ok()) else {
                continue;
            };
            let Some(g) = semicolon.next().and_then(|value| value.parse::<u8>().ok()) else {
                continue;
            };
            let Some(b) = semicolon.next().and_then(|value| value.parse::<u8>().ok()) else {
                continue;
            };
            *fg = Some((r, g, b));
        }
    }
}

pub const CODE_DOC: &str = "\
# Heading\n\
\n\
Some prose line.\n\
\n\
```rust\n\
pub struct FooBar {\n\
    foo: String,\n\
}\n\
```\n\
";
