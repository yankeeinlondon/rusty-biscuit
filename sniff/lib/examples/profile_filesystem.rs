//! Filesystem-heavy profiling entry point.
//!
//! Targets the staged filesystem request against the current working
//! directory (expected to be the repo root when invoked from
//! `just profile`). Runs enough iterations that flamegraphs can
//! resolve git + shared-walk costs cleanly.

use sniff::filesystem::detect_filesystem_with_request;
use sniff::request::{FilesystemRequest, GitRequest, RepoRequest};
use std::env;
use std::path::PathBuf;

fn main() {
    let iterations: u32 = env::var("SNIFF_PROFILE_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let root: PathBuf = env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| env::current_dir().ok())
        .expect("failed to resolve profile root");

    let request = FilesystemRequest::new()
        .git(GitRequest::full())
        .repo(RepoRequest::full());

    println!(
        "profiling detect_filesystem_with_request({}) x{iterations}",
        root.display()
    );

    for _ in 0..iterations {
        let info = detect_filesystem_with_request(&root, &request).expect("filesystem detect failed");
        std::hint::black_box(info);
    }
}
