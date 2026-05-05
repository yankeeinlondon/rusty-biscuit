pub mod cargo;
pub mod detection;
pub mod go;
pub(crate) mod manifest_index;
pub mod npm;
pub mod nx_turbo;
pub mod python;
pub mod types;

pub use types::{
    MonorepoTool, Package, PackageDiscoverySource, PackageEcosystem, RepoInfo, detect_repo,
    detect_repo_structure, detect_repo_with_inventory,
};

pub use crate::package::{DependencyEntry, DependencyKind};
