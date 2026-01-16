use rayon::prelude::*;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;
use which::which;

/// Finds a program by name in the system PATH.
///
/// Uses the `which` crate for cross-platform executable discovery,
/// handling Windows extensions (.exe, .cmd, etc.) automatically.
///
/// ## Returns
///
/// - `Some(PathBuf)` - The canonical path to the executable if found
/// - `None` - If the program is not found in PATH
///
/// ## Examples
///
/// ```
/// use sniff_lib::programs::find_program::find_program;
///
/// if let Some(path) = find_program("git") {
///     println!("git found at: {}", path.display());
/// }
/// ```
pub fn find_program<P: AsRef<OsStr>>(program: P) -> Option<PathBuf> {
    which(program).ok()
}

/// Checks for the existence of multiple programs in parallel.
/// Returns a HashMap where keys are program names and values are paths (if found).
pub fn find_programs_parallel(programs: &[&str]) -> HashMap<String, Option<PathBuf>> {
    programs
        .par_iter() // 1. Convert to a parallel iterator
        .map(|&prog| {
            // 2. Perform the check synchronously on a Rayon thread
            (prog.to_string(), which(prog).ok())
        })
        .collect() // 3. Collect results back into a HashMap
}
