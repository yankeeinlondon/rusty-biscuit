use serde::Serialize;
use std::hash::Hash;

/// The runnable trait ensures that type implementing
/// this trait can we executed with the `execute()`
/// function and that the function will return a
/// serializable value.
pub trait Runnable<R: Serialize + Hash + Eq> {
    fn execute(&self) -> R;
}
