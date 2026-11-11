//! A temporary directory of `ci-push-test-tool` copies, their control file,
//! and their invocation log. See the tool's `//!` for the control format.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tempfile::TempDir;

pub struct Fixture {
    root: TempDir,
}

impl Fixture {
    /// Installs a copy of the tool under each of `names` and writes `tools`
    /// as the control file's behaviors.
    pub fn new(names: &[&str], tools: Value) -> Self {
        let fixture = Self {
            root: tempfile::tempdir().expect("create the fixture root"),
        };
        std::fs::create_dir_all(fixture.bin()).unwrap();
        std::fs::create_dir_all(fixture.work()).unwrap();
        let tool = biscuit_test_harness::bin_exe!("ci-push-test-tool");
        for name in names {
            std::fs::copy(&tool, fixture.tool(name))
                .unwrap_or_else(|error| panic!("copy {} as {name}: {error}", tool.display()));
        }
        fixture.configure(tools);
        fixture
    }

    /// Replaces the control file's behaviors (for behaviors that name paths
    /// inside the fixture).
    pub fn configure(&self, tools: Value) {
        let control = json!({ "log": self.log(), "tools": tools });
        std::fs::write(
            self.bin().join("ci-push-test-tool.json"),
            serde_json::to_vec_pretty(&control).unwrap(),
        )
        .unwrap();
    }

    pub fn bin(&self) -> PathBuf {
        self.root.path().join("bin")
    }

    /// A scratch directory for children's working directories and markers.
    pub fn work(&self) -> PathBuf {
        self.root.path().join("work")
    }

    pub fn tool(&self, name: &str) -> PathBuf {
        let file = if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_owned()
        };
        self.bin().join(file)
    }

    /// A `PATH` holding the fixture `bin` alone.
    pub fn path(&self) -> OsString {
        std::env::join_paths([self.bin()]).unwrap()
    }

    fn log(&self) -> PathBuf {
        self.root.path().join("invocations.jsonl")
    }

    /// Every recorded invocation, in order.
    pub fn invocations(&self) -> Vec<Value> {
        match std::fs::read_to_string(self.log()) {
            Ok(text) => text
                .lines()
                .map(|line| serde_json::from_str(line).expect("a JSON log line"))
                .collect(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => panic!("read the invocation log: {error}"),
        }
    }

    /// Forgets every recorded invocation.
    pub fn clear_log(&self) {
        let _ = std::fs::remove_file(self.log());
    }

    pub fn only_invocation(&self) -> Value {
        let mut invocations = self.invocations();
        assert_eq!(invocations.len(), 1, "{invocations:#?}");
        invocations.remove(0)
    }
}

/// Copies the tool into `dir` as `name` (`.exe` on Windows) beside its own
/// control file holding `tools`, with no invocation log: a stub placed where
/// another program looks for it (a bare remote's `hooks/`). Returns the copy.
pub fn install_tool(dir: &Path, name: &str, tools: Value) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    let copy = dir.join(file);
    let tool = biscuit_test_harness::bin_exe!("ci-push-test-tool");
    std::fs::copy(&tool, &copy)
        .unwrap_or_else(|error| panic!("copy {} as {name}: {error}", tool.display()));
    std::fs::write(
        dir.join("ci-push-test-tool.json"),
        serde_json::to_vec_pretty(&json!({ "tools": tools })).unwrap(),
    )
    .unwrap();
    copy
}

/// Polls `condition` until it holds, panicking with `what` after `limit`.
pub fn wait_until(limit: Duration, what: &str, mut condition: impl FnMut() -> bool) {
    let stop = Instant::now() + limit;
    while !condition() {
        assert!(
            Instant::now() < stop,
            "timed out after {limit:?} waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The pid a fixture child wrote to `file`, once the file exists.
pub fn read_pid(file: &Path) -> u32 {
    std::fs::read_to_string(file)
        .unwrap_or_else(|error| panic!("read {}: {error}", file.display()))
        .trim()
        .parse()
        .expect("a pid")
}

pub fn pid_of(invocation: &Value) -> u32 {
    invocation["pid"].as_u64().expect("a logged pid") as u32
}

/// Whether no process (not even an unreaped zombie) has this pid.
#[cfg(unix)]
pub fn process_gone(pid: u32) -> bool {
    // SAFETY: signal 0 only checks for existence.
    let alive = unsafe { libc::kill(pid as libc::pid_t, 0) } == 0;
    !alive && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

/// Whether the process with this pid has exited.
#[cfg(windows)]
pub fn process_gone(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };
    // SAFETY: the handle is closed on every path.
    unsafe {
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            return true;
        }
        let exited = WaitForSingleObject(handle, 0) == WAIT_OBJECT_0;
        CloseHandle(handle);
        exited
    }
}

/// Paths compared after resolving symlinks (`/var` is `/private/var` on
/// macOS).
pub fn same_path(left: &Path, right: &Path) -> bool {
    left.canonicalize().unwrap() == right.canonicalize().unwrap()
}
