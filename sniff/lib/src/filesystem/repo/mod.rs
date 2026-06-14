pub mod area;
pub mod cargo;
pub mod detection;
pub mod go;
pub mod identity;
pub(crate) mod manifest_index;
pub mod npm;
pub mod nx_turbo;
pub mod python;
pub mod types;

pub use area::{AreaError, detect_area};
pub use identity::{RepoIdentity, detect_repo_identity, detect_repo_identity_with_repo};
pub use types::{
    MonorepoTool, Package, PackageDiscoverySource, PackageEcosystem, RepoInfo, detect_repo,
    detect_repo_structure, detect_repo_with_inventory,
};

pub use crate::package::{DependencyEntry, DependencyKind};
