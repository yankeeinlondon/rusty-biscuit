//! Process-wide reuse of values derived from repository files.
//!
//! One generator run reads the same files many times: ten provider passes
//! resolve every topic sidecar, and the signals, vocabulary, and steering
//! passes reload documents a provider pass already validated. A [`FileMemo`]
//! computes a value once per key and returns it again while every file it was
//! derived from still holds the same bytes, so an edit between two calls in
//! one process (a test doctoring a fixture, `generate` writing an output) is
//! always seen.
//!
//! Callers of one key are serialized: a provider thread that asks for a topic
//! schema another thread is resolving waits for that result instead of
//! resolving it again.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// A memo of `V` values keyed by a caller-built string.
pub(crate) struct FileMemo<V> {
    slots: Mutex<HashMap<String, Slot<V>>>,
}

/// One key's entry, locked on its own so callers of other keys never wait.
type Slot<V> = Arc<Mutex<Option<Entry<V>>>>;

struct Entry<V> {
    /// Each input file with the bytes it held after the value was computed;
    /// `None` records an unreadable file.
    inputs: Vec<(PathBuf, Option<Vec<u8>>)>,
    value: V,
}

impl<V: Clone> FileMemo<V> {
    pub(crate) fn new() -> Self {
        Self {
            slots: Mutex::new(HashMap::new()),
        }
    }

    /// Returns the value for `key`, computing it when absent or when any file
    /// it was computed from has changed since.
    ///
    /// `compute` returns the value and the files it was derived from. Inputs
    /// are read back after `compute` returns, so a file rewritten while it runs
    /// can pair the old value with the new bytes; the generator never edits
    /// its inputs while it reads them. Errors are returned and never stored.
    pub(crate) fn get_or_try_insert<E>(
        &self,
        key: String,
        compute: impl FnOnce() -> Result<(V, Vec<PathBuf>), E>,
    ) -> Result<V, E> {
        let slot = Arc::clone(lock(&self.slots).entry(key).or_default());
        let mut slot = lock(&slot);
        if let Some(entry) = slot.as_ref()
            && entry.inputs.iter().all(|(path, bytes)| read(path) == *bytes)
        {
            return Ok(entry.value.clone());
        }
        let (value, inputs) = compute()?;
        let inputs = inputs
            .into_iter()
            .map(|path| {
                let bytes = read(&path);
                (path, bytes)
            })
            .collect();
        *slot = Some(Entry {
            inputs,
            value: value.clone(),
        });
        Ok(value)
    }
}

fn read(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

/// A panic in one caller's computation leaves its slot empty, never half
/// written, so a poisoned lock is still safe to use.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reuses_a_value_until_an_input_changes() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.txt");
        std::fs::write(&input, "one").unwrap();
        let memo = FileMemo::<String>::new();
        let mut computed = 0;
        let get = |computed: &mut i32| {
            memo.get_or_try_insert::<std::io::Error>("key".into(), || {
                *computed += 1;
                Ok((std::fs::read_to_string(&input)?, vec![input.clone()]))
            })
            .unwrap()
        };

        assert_eq!(get(&mut computed), "one");
        assert_eq!(get(&mut computed), "one");
        assert_eq!(computed, 1, "an unchanged input reuses the value");

        std::fs::write(&input, "two").unwrap();
        assert_eq!(get(&mut computed), "two");
        assert_eq!(computed, 2, "a changed input recomputes the value");
    }

    #[test]
    fn an_error_is_not_stored() {
        let memo = FileMemo::<u8>::new();
        let failed: Result<u8, &str> = memo.get_or_try_insert("key".into(), || Err("boom"));
        assert_eq!(failed, Err("boom"));
        let value: Result<u8, &str> = memo.get_or_try_insert("key".into(), || Ok((7, Vec::new())));
        assert_eq!(value, Ok(7));
    }
}
