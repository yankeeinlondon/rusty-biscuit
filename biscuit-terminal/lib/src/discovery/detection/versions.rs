//! Versions of the multiplexer or terminal hosting this process, read only
//! where the graphics route depends on them.

use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use super::route::ToolVersion;

/// How long `zellij --version` may take before the version counts as
/// unreadable.
const ZELLIJ_VERSION_DEADLINE: Duration = Duration::from_millis(500);

/// The installed Zellij's version, from `zellij --version`. Read once per
/// process; `None` when the command is missing, fails, prints no version, or
/// misses its deadline.
pub(crate) fn zellij_version() -> Option<ToolVersion> {
    static VERSION: OnceLock<Option<ToolVersion>> = OnceLock::new();
    *VERSION.get_or_init(|| {
        let output = run_with_deadline("zellij", &["--version"], ZELLIJ_VERSION_DEADLINE)?;
        ToolVersion::parse(&output)
    })
}

/// Runs `program` with `args` (no shell) and returns its stdout, or `None`
/// when it cannot start, exits unsuccessfully, or outlives `deadline`, in
/// which case it is killed.
pub(crate) fn run_with_deadline(program: &str, args: &[&str], deadline: Duration) -> Option<String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let output = child.wait_with_output().ok()?;
                return String::from_utf8(output.stdout).ok();
            }
            Ok(None) if started.elapsed() < deadline => {
                std::thread::sleep(Duration::from_millis(5));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// The file version of the `WindowsTerminal.exe` in this process's parent
/// chain. Read once per process; `None` off Windows, when no Windows
/// Terminal is found, or when its version resource cannot be read.
pub(crate) fn windows_terminal_version() -> Option<ToolVersion> {
    static VERSION: OnceLock<Option<ToolVersion>> = OnceLock::new();
    *VERSION.get_or_init(windows_terminal::version)
}

#[cfg(not(windows))]
mod windows_terminal {
    use super::ToolVersion;

    pub(super) fn version() -> Option<ToolVersion> {
        None
    }
}

/// Parent hops searched for Windows Terminal before giving up.
#[cfg(any(windows, test))]
const MAX_PARENT_HOPS: usize = 16;

/// The nearest ancestor of `start` whose image file name equals `image`
/// (ASCII case-insensitive), given a `pid -> (parent_pid, image name)`
/// process table. Stops after [`MAX_PARENT_HOPS`] hops, at a PID missing
/// from the table, or on a cycle (PIDs are reused, so a stale parent PID can
/// point back down the chain).
#[cfg(any(windows, test))]
fn find_ancestor(
    start: u32,
    processes: &std::collections::HashMap<u32, (u32, String)>,
    image: &str,
) -> Option<u32> {
    let mut visited = vec![start];
    let mut current = start;
    for _ in 0..MAX_PARENT_HOPS {
        let (parent, _) = processes.get(&current)?;
        if visited.contains(parent) {
            return None;
        }
        let (_, name) = processes.get(parent)?;
        if name.eq_ignore_ascii_case(image) {
            return Some(*parent);
        }
        visited.push(*parent);
        current = *parent;
    }
    None
}

/// `major.minor.build` from a `VS_FIXEDFILEINFO` file version; the fourth
/// (revision) part is dropped.
#[cfg(any(windows, test))]
const fn fixed_file_version(version_ms: u32, version_ls: u32) -> ToolVersion {
    ToolVersion::new(version_ms >> 16, version_ms & 0xFFFF, version_ls >> 16)
}

#[cfg(windows)]
mod windows_terminal {
    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::ptr::null_mut;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VS_FIXEDFILEINFO, VerQueryValueW,
    };
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };

    use super::{ToolVersion, find_ancestor, fixed_file_version};

    const WINDOWS_TERMINAL_IMAGE: &str = "WindowsTerminal.exe";
    /// `VS_FIXEDFILEINFO::dwSignature` of a well-formed version resource.
    const FIXED_FILE_INFO_SIGNATURE: u32 = 0xFEEF_04BD;
    /// Longest Win32 path, in UTF-16 units including the terminator.
    const MAX_IMAGE_PATH: usize = 32_768;

    pub(super) fn version() -> Option<ToolVersion> {
        let processes = process_table()?;
        // SAFETY: no preconditions.
        let current = unsafe { GetCurrentProcessId() };
        let terminal = find_ancestor(current, &processes, WINDOWS_TERMINAL_IMAGE)?;
        file_version(&image_path(terminal)?)
    }

    struct OwnedHandle(HANDLE);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: the handle is open and owned by this value alone.
            unsafe { CloseHandle(self.0) };
        }
    }

    /// Every running process as `pid -> (parent_pid, image file name)`.
    fn process_table() -> Option<HashMap<u32, (u32, String)>> {
        // SAFETY: no pointer arguments.
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot == INVALID_HANDLE_VALUE || snapshot.is_null() {
            return None;
        }
        let snapshot = OwnedHandle(snapshot);
        // SAFETY: PROCESSENTRY32W is plain data; all-zero is valid.
        let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
        entry.dwSize = u32::try_from(size_of::<PROCESSENTRY32W>()).ok()?;
        let mut table = HashMap::new();
        // SAFETY: `entry` is a writable PROCESSENTRY32W with `dwSize` set.
        let mut more = unsafe { Process32FirstW(snapshot.0, &mut entry) } != 0;
        while more {
            let name = &entry.szExeFile;
            let len = name.iter().position(|&unit| unit == 0).unwrap_or(name.len());
            table.insert(
                entry.th32ProcessID,
                (entry.th32ParentProcessID, String::from_utf16_lossy(&name[..len])),
            );
            // SAFETY: as above.
            more = unsafe { Process32NextW(snapshot.0, &mut entry) } != 0;
        }
        Some(table)
    }

    /// The NUL-terminated full path of `pid`'s executable.
    fn image_path(pid: u32) -> Option<Vec<u16>> {
        // SAFETY: no pointer arguments.
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if process.is_null() {
            return None;
        }
        let process = OwnedHandle(process);
        let mut path = vec![0u16; MAX_IMAGE_PATH];
        let mut len = u32::try_from(path.len()).ok()?;
        // SAFETY: `path` holds `len` writable units; `len` is updated in place.
        let ok = unsafe {
            QueryFullProcessImageNameW(process.0, PROCESS_NAME_WIN32, path.as_mut_ptr(), &mut len)
        };
        if ok == 0 {
            return None;
        }
        path.truncate(usize::try_from(len).ok()?);
        path.push(0);
        Some(path)
    }

    /// The file version in `path`'s version resource.
    fn file_version(path: &[u16]) -> Option<ToolVersion> {
        // SAFETY: `path` is NUL-terminated; the handle out-pointer may be null.
        let size = unsafe { GetFileVersionInfoSizeW(path.as_ptr(), null_mut()) };
        if size == 0 {
            return None;
        }
        let mut block = vec![0u8; usize::try_from(size).ok()?];
        // SAFETY: `block` holds `size` writable bytes.
        let ok = unsafe { GetFileVersionInfoW(path.as_ptr(), 0, size, block.as_mut_ptr().cast()) };
        if ok == 0 {
            return None;
        }
        let root = [u16::from(b'\\'), 0];
        let mut info: *mut c_void = null_mut();
        let mut info_len = 0u32;
        // SAFETY: `block` is a version block filled above; `info` points into it.
        let ok = unsafe { VerQueryValueW(block.as_ptr().cast(), root.as_ptr(), &mut info, &mut info_len) };
        if ok == 0 || info.is_null() || (info_len as usize) < size_of::<VS_FIXEDFILEINFO>() {
            return None;
        }
        // SAFETY: `info` points at `info_len` readable bytes inside `block`.
        let fixed = unsafe { std::ptr::read_unaligned(info.cast::<VS_FIXEDFILEINFO>()) };
        if fixed.dwSignature != FIXED_FILE_INFO_SIGNATURE {
            return None;
        }
        Some(fixed_file_version(fixed.dwFileVersionMS, fixed.dwFileVersionLS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_program_has_no_output() {
        assert_eq!(
            run_with_deadline(
                "biscuit-terminal-no-such-program",
                &[],
                Duration::from_millis(200)
            ),
            None
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_program_outliving_its_deadline_is_killed() {
        let started = Instant::now();

        let output = run_with_deadline("sleep", &["5"], Duration::from_millis(100));

        assert_eq!(output, None);
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "the deadline should kill the child, took {:?}",
            started.elapsed()
        );
    }

    fn table(entries: &[(u32, u32, &str)]) -> std::collections::HashMap<u32, (u32, String)> {
        entries
            .iter()
            .map(|&(pid, parent, name)| (pid, (parent, name.to_string())))
            .collect()
    }

    #[test]
    fn find_ancestor_matches_the_image_name_case_insensitively() {
        let processes = table(&[
            (10, 9, "pwsh.exe"),
            (9, 8, "OpenConsole.exe"),
            (8, 4, "windowsterminal.EXE"),
            (4, 0, "explorer.exe"),
        ]);

        assert_eq!(find_ancestor(10, &processes, "WindowsTerminal.exe"), Some(8));
    }

    #[test]
    fn find_ancestor_never_matches_the_starting_process() {
        let processes = table(&[(10, 4, "WindowsTerminal.exe"), (4, 0, "explorer.exe")]);

        assert_eq!(find_ancestor(10, &processes, "WindowsTerminal.exe"), None);
    }

    #[test]
    fn find_ancestor_stops_at_a_missing_parent() {
        let processes = table(&[(10, 9, "pwsh.exe")]);

        assert_eq!(find_ancestor(10, &processes, "WindowsTerminal.exe"), None);
    }

    #[test]
    fn find_ancestor_stops_on_a_cycle() {
        let processes = table(&[(10, 9, "pwsh.exe"), (9, 10, "cmd.exe")]);

        assert_eq!(find_ancestor(10, &processes, "WindowsTerminal.exe"), None);
    }

    #[test]
    fn find_ancestor_stops_after_the_hop_limit() {
        let hops = u32::try_from(MAX_PARENT_HOPS).expect("small constant");
        let mut entries: Vec<(u32, u32, &str)> = (1..=hops).map(|pid| (pid, pid + 1, "cmd.exe")).collect();
        entries.push((hops + 1, hops + 2, "cmd.exe"));
        entries.push((hops + 2, 0, "WindowsTerminal.exe"));
        let beyond = table(&entries);
        let within = table(&entries[1..]);

        assert_eq!(find_ancestor(1, &beyond, "WindowsTerminal.exe"), None);
        assert_eq!(find_ancestor(2, &within, "WindowsTerminal.exe"), Some(hops + 2));
    }

    #[test]
    fn fixed_file_version_reads_major_minor_build() {
        // Windows Terminal 1.22.10942.0
        let version = fixed_file_version((1 << 16) | 22, 10942 << 16);

        assert_eq!(version, ToolVersion::new(1, 22, 10942));
    }

    #[cfg(windows)]
    #[test]
    fn windows_terminal_version_does_not_panic() {
        // The build host may or may not run tests under Windows Terminal.
        let _ = windows_terminal::version();
    }
}
