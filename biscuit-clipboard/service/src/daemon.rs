use std::fs;
use std::path::{Path, PathBuf};

use biscuit_clipboard::ClipboardError;
use biscuit_clipboard::config;
use fs4::fs_std::FileExt;

#[derive(Clone)]
pub struct DaemonFiles {
    runtime_dir: PathBuf,
}

impl DaemonFiles {
    pub fn new() -> Result<Self, ClipboardError> {
        let runtime_dir = config::runtime_dir()?;
        fs::create_dir_all(&runtime_dir)?;
        Ok(Self { runtime_dir })
    }

    pub fn with_runtime_dir(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        Self { runtime_dir: dir }
    }

    #[allow(dead_code)]
    pub fn runtime_dir(&self) -> &Path {
        &self.runtime_dir
    }

    pub fn pid_file(&self) -> PathBuf {
        self.runtime_dir.join(config::PID_FILENAME)
    }

    pub fn port_file(&self) -> PathBuf {
        self.runtime_dir.join(config::PORT_FILENAME)
    }

    pub fn write_pid(&self) -> Result<(), ClipboardError> {
        let pid = std::process::id();
        fs::write(self.pid_file(), pid.to_string())?;
        Ok(())
    }

    pub fn read_pid(&self) -> Option<u32> {
        fs::read_to_string(self.pid_file())
            .ok()
            .and_then(|s| s.trim().parse().ok())
    }

    pub fn write_port(&self, port: u16) -> Result<(), ClipboardError> {
        fs::write(self.port_file(), port.to_string())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn read_port(&self) -> Option<u16> {
        fs::read_to_string(self.port_file())
            .ok()
            .and_then(|s| s.trim().parse().ok())
    }

    pub fn is_already_running(&self) -> bool {
        if let Some(pid) = self.read_pid() {
            config::is_pid_alive(pid)
        } else {
            false
        }
    }

    pub fn cleanup(&self) -> Result<(), ClipboardError> {
        let _ = fs::remove_file(self.pid_file());
        let _ = fs::remove_file(self.port_file());
        Ok(())
    }

    pub fn acquire_lock(&self) -> Result<LockGuard, ClipboardError> {
        let lock_path = self.runtime_dir.join("clipper.lock");
        let file = fs::File::create(&lock_path)?;
        file.try_lock_exclusive().map_err(|e| {
            ClipboardError::Backend(format!("Another clipper instance is already running: {e}"))
        })?;

        Ok(LockGuard {
            file: Some(file),
            lock_path,
        })
    }
}

pub struct LockGuard {
    file: Option<fs::File>,
    lock_path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = file.unlock();
        }
        let _ = fs::remove_file(&self.lock_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_daemon_files_write_read_pid() {
        let dir = TempDir::new().unwrap();
        let daemon = DaemonFiles::with_runtime_dir(dir.path().to_path_buf());

        assert!(daemon.read_pid().is_none());
        daemon.write_pid().unwrap();
        let pid = daemon.read_pid().unwrap();
        assert_eq!(pid, std::process::id());
    }

    #[test]
    fn test_daemon_files_write_read_port() {
        let dir = TempDir::new().unwrap();
        let daemon = DaemonFiles::with_runtime_dir(dir.path().to_path_buf());

        assert!(daemon.read_port().is_none());
        daemon.write_port(9876).unwrap();
        assert_eq!(daemon.read_port().unwrap(), 9876);
    }

    #[test]
    fn test_daemon_files_cleanup() {
        let dir = TempDir::new().unwrap();
        let daemon = DaemonFiles::with_runtime_dir(dir.path().to_path_buf());

        daemon.write_pid().unwrap();
        daemon.write_port(1234).unwrap();
        assert!(daemon.pid_file().exists());
        assert!(daemon.port_file().exists());

        daemon.cleanup().unwrap();
        assert!(!daemon.pid_file().exists());
        assert!(!daemon.port_file().exists());
    }

    #[test]
    fn test_is_pid_alive_current_process() {
        assert!(config::is_pid_alive(std::process::id()));
    }

    #[test]
    fn test_is_pid_alive_nonexistent() {
        assert!(!config::is_pid_alive(999999999));
    }

    #[test]
    fn test_lock_file_created_and_cleaned_up() {
        let dir = TempDir::new().unwrap();
        let daemon = DaemonFiles::with_runtime_dir(dir.path().to_path_buf());

        let lock_path = daemon.runtime_dir.join("clipper.lock");

        let lock = daemon.acquire_lock().unwrap();
        assert!(lock_path.exists(), "Lock file should exist while held");

        drop(lock);
        assert!(!lock_path.exists(), "Lock file should be removed on drop");
    }

    #[test]
    fn test_is_already_running_with_alive_pid() {
        let dir = TempDir::new().unwrap();
        let daemon = DaemonFiles::with_runtime_dir(dir.path().to_path_buf());

        assert!(!daemon.is_already_running());

        daemon.write_pid().unwrap();
        assert!(daemon.is_already_running());
    }

    #[test]
    fn test_is_already_running_with_dead_pid() {
        let dir = TempDir::new().unwrap();
        let daemon = DaemonFiles::with_runtime_dir(dir.path().to_path_buf());

        fs::write(daemon.pid_file(), "999999999").unwrap();
        assert!(!daemon.is_already_running(), "Dead PID should not count as running");
    }

    #[test]
    fn test_cleanup_after_lock_release() {
        let dir = TempDir::new().unwrap();
        let daemon = DaemonFiles::with_runtime_dir(dir.path().to_path_buf());

        let _lock = daemon.acquire_lock().unwrap();
        daemon.write_pid().unwrap();
        daemon.write_port(5555).unwrap();

        assert!(daemon.pid_file().exists());
        assert!(daemon.port_file().exists());

        daemon.cleanup().unwrap();
        assert!(!daemon.pid_file().exists());
        assert!(!daemon.port_file().exists());
    }
}
