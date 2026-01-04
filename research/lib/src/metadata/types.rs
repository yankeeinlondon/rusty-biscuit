//! Type-specific details for research outputs.
//!
//! This module defines the [`ResearchDetails`] enum and its associated detail structs.
//! Each research kind has its own detail struct that holds type-specific metadata.
//!
//! ## Design Notes
//!
//! - All detail structs use `{}` (empty braced) rather than unit-like (`;`) because:
//!   - Unit structs serialize as `null` in JSON
//!   - Empty braced structs serialize as `{}` which is more extensible
//! - The `#[non_exhaustive]` attribute signals that this enum will grow over time
//! - All structs derive `Default` for easy construction

use serde::{Deserialize, Serialize};

/// Type-specific details for research outputs.
///
/// This enum is tagged with `"type"` in JSON serialization, producing output like:
/// ```json
/// {
///     "type": "Library",
///     "package_manager": "crates.io",
///     ...
/// }
/// ```
///
/// ## Examples
///
/// ```
/// use research_lib::metadata::ResearchDetails;
///
/// let details = ResearchDetails::Api(Default::default());
/// let json = serde_json::to_string(&details).unwrap();
/// assert!(json.contains("\"type\":\"Api\""));
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ResearchDetails {
    /// Details for library/package research
    Library(LibraryDetails),
    /// Details for solution space research (comparing approaches)
    SolutionSpace(SolutionSpaceDetails),
    /// Details for CLI tool research
    Cli(CliDetails),
    /// Details for application research
    App(AppDetails),
    /// Details for cloud provider service research
    CloudProvider(CloudProviderDetails),
    /// Details for standard/specification research
    Standard(StandardDetails),
    /// Details for API research
    Api(ApiDetails),
    /// Details for individual person research
    Person(PersonDetails),
    /// Details for group/team research
    People(PeopleDetails),
    /// Details for location research
    Place(PlaceDetails),
    /// Details for product research
    Product(ProductDetails),
    /// Details for company research
    Company(CompanyDetails),
    /// Details for company category/industry research
    CompanyCategory(CompanyCategoryDetails),
    /// Details for news/current events research
    News(NewsDetails),
    /// Details for skill set research
    SkillSet(SkillSetDetails),
}

impl ResearchDetails {
    /// Returns the type name as a string (matches the serde tag).
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Library(_) => "Library",
            Self::SolutionSpace(_) => "SolutionSpace",
            Self::Cli(_) => "Cli",
            Self::App(_) => "App",
            Self::CloudProvider(_) => "CloudProvider",
            Self::Standard(_) => "Standard",
            Self::Api(_) => "Api",
            Self::Person(_) => "Person",
            Self::People(_) => "People",
            Self::Place(_) => "Place",
            Self::Product(_) => "Product",
            Self::Company(_) => "Company",
            Self::CompanyCategory(_) => "CompanyCategory",
            Self::News(_) => "News",
            Self::SkillSet(_) => "SkillSet",
        }
    }
}

/// Details for library/package research.
///
/// Contains metadata about the library's source, language, and location.
/// Fields are optional to support libraries where some information may be unavailable.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct LibraryDetails {
    /// The package manager (e.g., "crates.io", "npm", "PyPI")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<String>,
    /// The programming language (e.g., "Rust", "JavaScript", "Python")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// URL to the package on the package manager
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// URL to the source repository (e.g., GitHub, GitLab)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

/// Details for solution space research.
///
/// Used when researching a problem space and comparing different approaches,
/// rather than focusing on a specific library or tool.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SolutionSpaceDetails {}

/// Details for CLI tool research.
///
/// Used when researching command-line interface tools that may not be
/// distributed as traditional packages.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CliDetails {}

/// Details for application research.
///
/// Used when researching desktop, mobile, or web applications.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AppDetails {}

/// Details for cloud provider service research.
///
/// Used when researching cloud services (AWS, GCP, Azure, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CloudProviderDetails {}

/// Details for standard/specification research.
///
/// Used when researching technical standards, protocols, or specifications.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StandardDetails {}

/// Details for API research.
///
/// Used when researching public APIs (REST, GraphQL, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ApiDetails {}

/// Details for individual person research.
///
/// Used when researching a notable individual.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PersonDetails {}

/// Details for group/team research.
///
/// Used when researching teams, organizations, or groups of people.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PeopleDetails {}

/// Details for location research.
///
/// Used when researching physical locations, regions, or venues.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PlaceDetails {}

/// Details for product research.
///
/// Used when researching commercial products.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ProductDetails {}

/// Details for company research.
///
/// Used when researching specific companies or organizations.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CompanyDetails {}

/// Details for company category/industry research.
///
/// Used when researching industry sectors or categories of companies.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CompanyCategoryDetails {}

/// Details for news/current events research.
///
/// Used when researching recent events or news topics.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct NewsDetails {}

/// Details for skill set research.
///
/// Used when researching a collection of related skills or competencies.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SkillSetDetails {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_details_serialization() {
        let details = LibraryDetails {
            package_manager: Some("crates.io".to_string()),
            language: Some("Rust".to_string()),
            url: Some("https://crates.io/crates/serde".to_string()),
            repository: Some("https://github.com/serde-rs/serde".to_string()),
        };

        let json = serde_json::to_string(&details).unwrap();
        assert!(json.contains("crates.io"));
        assert!(json.contains("Rust"));

        let roundtrip: LibraryDetails = serde_json::from_str(&json).unwrap();
        assert_eq!(details, roundtrip);
    }

    #[test]
    fn test_library_details_skip_none() {
        let details = LibraryDetails {
            package_manager: Some("npm".to_string()),
            language: None,
            url: None,
            repository: None,
        };

        let json = serde_json::to_string(&details).unwrap();
        assert!(json.contains("npm"));
        assert!(!json.contains("language"));
        assert!(!json.contains("url"));
        assert!(!json.contains("repository"));
    }

    #[test]
    fn test_research_details_library_serialization() {
        let details = ResearchDetails::Library(LibraryDetails {
            package_manager: Some("crates.io".to_string()),
            language: Some("Rust".to_string()),
            url: None,
            repository: None,
        });

        let json = serde_json::to_string(&details).unwrap();
        assert!(json.contains("\"type\":\"Library\""));
        assert!(json.contains("crates.io"));

        let roundtrip: ResearchDetails = serde_json::from_str(&json).unwrap();
        assert_eq!(details, roundtrip);
    }

    #[test]
    fn test_research_details_api_serialization() {
        let details = ResearchDetails::Api(ApiDetails::default());

        let json = serde_json::to_string(&details).unwrap();
        // Empty braced structs serialize as {} not null
        assert!(json.contains("\"type\":\"Api\""));

        let roundtrip: ResearchDetails = serde_json::from_str(&json).unwrap();
        assert_eq!(details, roundtrip);
    }

    #[test]
    fn test_empty_struct_serialization() {
        // Verify empty braced structs serialize correctly (not as null)
        let api = ApiDetails::default();
        let json = serde_json::to_string(&api).unwrap();
        assert_eq!(json, "{}");

        let cli = CliDetails::default();
        let json = serde_json::to_string(&cli).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_type_name() {
        assert_eq!(
            ResearchDetails::Library(Default::default()).type_name(),
            "Library"
        );
        assert_eq!(ResearchDetails::Api(Default::default()).type_name(), "Api");
        assert_eq!(
            ResearchDetails::SolutionSpace(Default::default()).type_name(),
            "SolutionSpace"
        );
        assert_eq!(ResearchDetails::Cli(Default::default()).type_name(), "Cli");
        assert_eq!(ResearchDetails::App(Default::default()).type_name(), "App");
        assert_eq!(
            ResearchDetails::CloudProvider(Default::default()).type_name(),
            "CloudProvider"
        );
        assert_eq!(
            ResearchDetails::Standard(Default::default()).type_name(),
            "Standard"
        );
        assert_eq!(
            ResearchDetails::Person(Default::default()).type_name(),
            "Person"
        );
        assert_eq!(
            ResearchDetails::People(Default::default()).type_name(),
            "People"
        );
        assert_eq!(
            ResearchDetails::Place(Default::default()).type_name(),
            "Place"
        );
        assert_eq!(
            ResearchDetails::Product(Default::default()).type_name(),
            "Product"
        );
        assert_eq!(
            ResearchDetails::Company(Default::default()).type_name(),
            "Company"
        );
        assert_eq!(
            ResearchDetails::CompanyCategory(Default::default()).type_name(),
            "CompanyCategory"
        );
        assert_eq!(
            ResearchDetails::News(Default::default()).type_name(),
            "News"
        );
        assert_eq!(
            ResearchDetails::SkillSet(Default::default()).type_name(),
            "SkillSet"
        );
    }

    #[test]
    fn test_all_variants_roundtrip() {
        let variants = vec![
            ResearchDetails::Library(Default::default()),
            ResearchDetails::SolutionSpace(Default::default()),
            ResearchDetails::Cli(Default::default()),
            ResearchDetails::App(Default::default()),
            ResearchDetails::CloudProvider(Default::default()),
            ResearchDetails::Standard(Default::default()),
            ResearchDetails::Api(Default::default()),
            ResearchDetails::Person(Default::default()),
            ResearchDetails::People(Default::default()),
            ResearchDetails::Place(Default::default()),
            ResearchDetails::Product(Default::default()),
            ResearchDetails::Company(Default::default()),
            ResearchDetails::CompanyCategory(Default::default()),
            ResearchDetails::News(Default::default()),
            ResearchDetails::SkillSet(Default::default()),
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let roundtrip: ResearchDetails = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, roundtrip, "Failed for {}", variant.type_name());
        }
    }
}
