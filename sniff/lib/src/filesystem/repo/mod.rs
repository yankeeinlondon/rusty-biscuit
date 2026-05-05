pub mod detection;
pub mod types;

pub use types::{
    MonorepoTool, Package, PackageDiscoverySource, PackageEcosystem, RepoInfo, detect_repo,
    detect_repo_structure, detect_repo_with_inventory,
};

pub use crate::package::{DependencyEntry, DependencyKind};
