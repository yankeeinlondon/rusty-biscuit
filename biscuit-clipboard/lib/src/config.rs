use std::path::PathBuf;

use crate::error::ClipboardError;

pub const DEFAULT_PORT: u16 = 17530;
pub const CLIP_PORT_ENV: &str = "CLIP_PORT";
pub const PID_FILENAME: &str = "clipper.pid";
pub const PORT_FILENAME: &str = "clipper.port";

pub fn configured_port() -> u16 {
    std::env::var(CLIP_PORT_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

pub fn runtime_dir() -> Result<PathBuf, ClipboardError> {
    let base = dirs::runtime_dir()
        .or_else(dirs::cache_dir)
        .ok_or_else(|| {
            ClipboardError::Backend("Could not determine runtime directory".to_string())
        })?;
    Ok(base.join("biscuit-clipboard"))
}

pub fn read_port_file() -> Option<u16> {
    let path = runtime_dir().ok()?.join(PORT_FILENAME);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

pub fn read_pid_file() -> Option<u32> {
    let path = runtime_dir().ok()?.join(PID_FILENAME);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

pub fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configured_port_default() {
        unsafe { std::env::remove_var(CLIP_PORT_ENV) };
        assert_eq!(configured_port(), DEFAULT_PORT);
    }

    #[test]
    fn test_runtime_dir_is_valid() {
        let dir = runtime_dir();
        assert!(dir.is_ok());
        assert!(dir.unwrap().to_string_lossy().contains("biscuit-clipboard"));
    }

    #[test]
    fn test_read_port_file_missing() {
        let result = read_port_file();
        assert!(result.is_none());
    }

    #[test]
    fn test_read_pid_file_missing() {
        let result = read_pid_file();
        assert!(result.is_none());
    }

    #[test]
    fn test_is_pid_alive_current() {
        assert!(is_pid_alive(std::process::id()));
    }

    #[test]
    fn test_is_pid_alive_nonexistent() {
        assert!(!is_pid_alive(999999999));
    }
}
