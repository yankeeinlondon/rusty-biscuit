#![allow(dead_code)]

pub(crate) mod completion;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_NONCE: AtomicU64 = AtomicU64::new(0);

pub struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    pub fn new() -> Self {
        Self::named("claudine-test")
    }

    pub fn named(prefix: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let unique = TEST_NONCE.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("{prefix}-{}-{nonce}-{unique}", process::id()));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    pub fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

pub fn write_json<T: serde::Serialize>(path: &Path, value: &T) {
    write(path, &serde_json::to_string_pretty(value).unwrap());
}

pub fn init_git_repo(path: &Path) -> bool {
    Command::new("git")
        .arg("init")
        .current_dir(path)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn augmented_path(fake_bin: &Path) -> std::ffi::OsString {
    let system_path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths: Vec<PathBuf> = vec![fake_bin.to_path_buf()];
    paths.extend(std::env::split_paths(&system_path));
    std::env::join_paths(paths).expect("join_paths")
}

pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for code in chars.by_ref() {
                    if ('@'..='~').contains(&code) {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(ch);
    }

    out
}

pub fn write_executable(path: &Path, content: &str) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        write(path, content);
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
    #[cfg(not(unix))]
    {
        write(path, content);
    }
}
