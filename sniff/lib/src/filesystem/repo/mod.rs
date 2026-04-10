pub mod detection;
pub mod types;

pub use types::{
    DependencyEntry, DependencyKind, MonorepoTool, Package, PackageDiscoverySource,
    PackageEcosystem, RepoInfo, detect_repo, detect_repo_structure, detect_repo_with_inventory,
};
