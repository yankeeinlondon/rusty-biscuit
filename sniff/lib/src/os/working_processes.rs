//! Which running processes have their current directory inside a given
//! directory.
//!
//! An on-demand fact, absent from host inventory: one process-table refresh
//! that reads only names, ancestry, and current directories.

use std::path::{Path, PathBuf};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// A process whose current directory is inside the queried directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingProcess {
    pub pid: u32,
    /// The executable's short name as the OS reports it.
    pub name: String,
    pub cwd: PathBuf,
}

/// Lists the processes whose current directory is `dir` or below it.
///
/// The calling process and its ancestors are never listed, so a caller whose
/// own shell is about to leave `dir` does not find itself. A process whose
/// current directory the OS will not reveal (another user's, or one that
/// exited mid-scan) is skipped, so an empty result means "none visible", not
/// "none".
///
/// `dir` matches by path components in both its given spelling and its
/// canonical one, so a symlinked spelling such as macOS `/tmp` still finds
/// processes the OS reports under `/private/tmp`.
pub fn processes_working_in(dir: &Path) -> Vec<WorkingProcess> {
    let mut spellings = vec![std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf())];
    if let Ok(canonical) = dir.canonicalize()
        && !spellings.contains(&canonical)
    {
        spellings.push(canonical);
    }

    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cwd(UpdateKind::Always),
    );
    let excluded = self_and_ancestors(&system);

    let mut found: Vec<WorkingProcess> = system
        .processes()
        .iter()
        // A Linux thread appears as its own entry sharing the process's cwd.
        .filter(|(pid, process)| !excluded.contains(pid) && process.thread_kind().is_none())
        .filter_map(|(pid, process)| {
            let cwd = process.cwd()?;
            spellings
                .iter()
                .any(|spelling| cwd.starts_with(spelling))
                .then(|| WorkingProcess {
                    pid: pid.as_u32(),
                    name: process.name().to_string_lossy().into_owned(),
                    cwd: cwd.to_path_buf(),
                })
        })
        .collect();
    found.sort_by_key(|process| process.pid);
    found
}

fn self_and_ancestors(system: &System) -> Vec<Pid> {
    let mut chain = Vec::new();
    let mut next = Some(Pid::from_u32(std::process::id()));
    while let Some(pid) = next {
        if chain.contains(&pid) {
            break;
        }
        chain.push(pid);
        next = system.process(pid).and_then(sysinfo::Process::parent);
    }
    chain
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    struct Holder(Child);

    impl Drop for Holder {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    fn hold(dir: &Path) -> Holder {
        #[cfg(unix)]
        let command = Command::new("sleep").arg("60").current_dir(dir).stdout(Stdio::null()).spawn();
        #[cfg(windows)]
        let command = Command::new("ping")
            .args(["-n", "60", "127.0.0.1"])
            .current_dir(dir)
            .stdout(Stdio::null())
            .spawn();
        Holder(command.unwrap())
    }

    /// The OS may publish a new child's cwd a moment after spawn.
    fn find(dir: &Path, pid: u32) -> Option<WorkingProcess> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(found) = processes_working_in(dir).into_iter().find(|p| p.pid == pid) {
                return Some(found);
            }
            if Instant::now() > deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn a_process_in_a_subdirectory_is_found_with_its_name() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        std::fs::create_dir_all(&nested).unwrap();
        let holder = hold(&nested);

        let found = find(dir.path(), holder.0.id()).expect("the holder is listed");

        assert!(!found.name.is_empty());
        assert!(found.cwd.ends_with(Path::new("a").join("b")), "{found:?}");
    }

    #[test]
    fn a_sibling_directory_sharing_a_name_prefix_is_not_inside() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("feat");
        let sibling = dir.path().join("feat-other");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir_all(&sibling).unwrap();
        let holder = hold(&sibling);
        find(&sibling, holder.0.id()).expect("the holder is visible at all");

        assert!(processes_working_in(&target).iter().all(|p| p.pid != holder.0.id()));
    }

    #[test]
    fn the_caller_and_its_ancestors_are_never_listed() {
        let here = std::env::current_dir().unwrap();
        let me = std::process::id();
        assert!(processes_working_in(&here).iter().all(|p| p.pid != me));
    }
}
