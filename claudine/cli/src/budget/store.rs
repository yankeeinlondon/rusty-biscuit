//! Ledger persistence: an OS file lock per ledger, atomic replacement on
//! every write.
//!
//! The lock lives in a sibling `<ledger>.lock` file and is held for the life of
//! the [`LedgerFile`]. The OS releases it when the holder exits for any reason,
//! which is what lets the next holder detect a crash: a ledger that still says
//! `active` while its lock is free was abandoned.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};

use super::error::BudgetError;
use super::model::Ledger;

/// An exclusively locked ledger file.
pub(crate) struct LedgerFile {
    path: PathBuf,
    _lock: File,
    _exclusive: Option<File>,
}

impl LedgerFile {
    /// Write a new ledger, refusing to replace an existing one.
    pub(crate) fn create(path: &Path, ledger: &Ledger) -> Result<Self, BudgetError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
        }
        let lock = lock_file(&lock_path_for(path))?;
        if path.exists() {
            return Err(BudgetError::AlreadyExists {
                path: path.to_path_buf(),
            });
        }
        let file = Self {
            path: path.to_path_buf(),
            _lock: lock,
            _exclusive: None,
        };
        file.write(ledger)?;
        Ok(file)
    }

    /// Lock an existing ledger and load it.
    pub(crate) fn acquire(path: &Path) -> Result<(Self, Ledger), BudgetError> {
        if !path.is_file() {
            return Err(io_error(
                path,
                std::io::Error::new(std::io::ErrorKind::NotFound, "no such ledger; run `claudine budget init` first"),
            ));
        }
        let lock = lock_file(&lock_path_for(path))?;
        let ledger = read(path)?;
        Ok((
            Self {
                path: path.to_path_buf(),
                _lock: lock,
                _exclusive: None,
            },
            ledger,
        ))
    }

    /// Take the ledger's shared exclusive lock, if it names one.
    ///
    /// Runs of different platforms name the same lock file, so only one of
    /// them executes at a time.
    pub(crate) fn lock_exclusive(&mut self, ledger: &Ledger) -> Result<(), BudgetError> {
        let Some(configured) = ledger.exclusive_lock.as_deref() else {
            return Ok(());
        };
        let configured = Path::new(configured);
        let path = if configured.is_absolute() {
            configured.to_path_buf()
        } else {
            self.path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(configured)
        };
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
        }
        self._exclusive = Some(lock_file(&path)?);
        Ok(())
    }

    /// Atomically replace the ledger file with `ledger`.
    pub(crate) fn write(&self, ledger: &Ledger) -> Result<(), BudgetError> {
        let mut bytes = serde_json::to_vec_pretty(ledger).map_err(|source| BudgetError::Parse {
            path: self.path.clone(),
            source,
        })?;
        bytes.push(b'\n');
        claudine::config::atomic::atomic_write(&self.path, &bytes)
            .map_err(|source| io_error(&self.path, source))
    }
}

/// Read and validate a ledger without locking it.
///
/// Every write is an atomic replacement, so an unlocked reader sees one
/// complete version.
pub(crate) fn read(path: &Path) -> Result<Ledger, BudgetError> {
    let bytes = std::fs::read(path).map_err(|source| io_error(path, source))?;
    let ledger: Ledger = serde_json::from_slice(&bytes).map_err(|source| BudgetError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    ledger.validate()?;
    Ok(ledger)
}

fn lock_path_for(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".lock");
    path.with_file_name(name)
}

fn lock_file(path: &Path) -> Result<File, BudgetError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|source| io_error(path, source))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(BudgetError::Locked {
            path: path.to_path_buf(),
        }),
        Err(TryLockError::Error(source)) => Err(io_error(path, source)),
    }
}

fn io_error(path: &Path, source: std::io::Error) -> BudgetError {
    BudgetError::Io {
        path: path.to_path_buf(),
        source,
    }
}
