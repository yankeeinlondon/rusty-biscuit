//! Metadata types and migration for research outputs.
//!
//! This module provides:
//! - [`ResearchDetails`] - Type-specific details for different research kinds
//! - [`MetadataV0`] - Legacy metadata format for migration
//! - [`migration`] - Automatic v0 -> v1 migration utilities
//!
//! ## Schema Evolution
//!
//! The metadata schema uses versioning to support backward compatibility:
//! - **v0**: Original format with `library_info` field
//! - **v1**: New format with `details: ResearchDetails` enum
//!
//! Migration happens automatically during [`crate::ResearchMetadata::load()`].

pub mod content_policy;
pub mod db;
pub mod inventory;
pub mod migration;
pub mod migration_v2;
pub mod topic;
pub mod types;
pub mod v0;

pub use types::{
    ApiDetails, AppDetails, CliDetails, CloudProviderDetails, CompanyCategoryDetails,
    CompanyDetails, LibraryDetails, NewsDetails, PeopleDetails, PersonDetails, PlaceDetails,
    ProductDetails, ResearchDetails, SkillSetDetails, SolutionSpaceDetails, StandardDetails,
};
pub use v0::MetadataV0;

pub use content_policy::{ContentExpiry, ContentPolicy};
pub use db::{DbError, DbPool, DbResult, ResearchInventoryDb, init_memory_pool, init_pool, run_migrations};
pub use inventory::{InventoryError, ResearchInventory};
pub use topic::{ContentType, Document, DocumentConversionError, Flow, KindCategory, Library, License, Software, Topic};
