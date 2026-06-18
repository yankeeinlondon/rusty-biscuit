/// Package discovery and aggregation utilities.
pub mod tree_package;

pub use tree_package::{find_git_root, find_package_root, has_package_manifest};
