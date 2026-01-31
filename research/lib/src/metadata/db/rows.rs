//! Row adapter types for database operations.
//!
//! This module provides row types that map directly to database tables,
//! along with conversion functions to/from domain types.
//!
//! ## Pattern
//!
//! SQLite stores all data as flat rows, but our domain types use Rust enums
//! with struct variants. The row adapter pattern bridges this gap:
//!
//! 1. **Row types** - `#[derive(sqlx::FromRow)]` structs that match table columns
//! 2. **Converters** - Functions to transform rows ↔ domain types
//! 3. **Discriminators** - TEXT columns that store enum variant names

use chrono::{DateTime, Utc};
use sniff_lib::package::LanguagePackageManager;
use unchained_ai::models::model_capability::ModelCapability;

use super::{DbError, DbResult, i64_to_u64, u64_to_i64};
use crate::metadata::{ContentType, Document, Flow, KindCategory, Library, License, Software, Topic};

// ============================================================================
// Enum TEXT conversions
// ============================================================================

/// Convert KindCategory to its discriminator string for the `kind` column.
pub fn kind_category_to_discriminator(kind: &KindCategory) -> &'static str {
    match kind {
        KindCategory::Library(_) => "Library",
        KindCategory::Software(_) => "Software",
        KindCategory::Person => "Person",
        KindCategory::SolutionArea => "SolutionArea",
        KindCategory::ProgrammingLanguage => "ProgrammingLanguage",
    }
}

/// Parse a discriminator string back to a KindCategory (without data).
/// Used only for simple enum variants; Library/Software need additional data.
pub fn discriminator_to_kind_category(s: &str) -> DbResult<KindCategory> {
    match s {
        "Person" => Ok(KindCategory::Person),
        "SolutionArea" => Ok(KindCategory::SolutionArea),
        "ProgrammingLanguage" => Ok(KindCategory::ProgrammingLanguage),
        // Library and Software need data from separate tables
        "Library" | "Software" => Err(DbError::InvalidEnumValue {
            type_name: "KindCategory".to_string(),
            value: format!("{s} requires data from details table"),
        }),
        other => Err(DbError::InvalidEnumValue {
            type_name: "KindCategory".to_string(),
            value: other.to_string(),
        }),
    }
}

/// Convert ContentType to TEXT for storage.
pub fn content_type_to_text(ct: &ContentType) -> &'static str {
    match ct {
        ContentType::Static => "Static",
        ContentType::Template => "Template",
        ContentType::Prompt => "Prompt",
        ContentType::KindDerived => "KindDerived",
        ContentType::Skill => "Skill",
        ContentType::DeepDive => "DeepDive",
    }
}

/// Parse ContentType from TEXT.
pub fn text_to_content_type(s: &str) -> DbResult<ContentType> {
    match s {
        "Static" => Ok(ContentType::Static),
        "Template" => Ok(ContentType::Template),
        "Prompt" => Ok(ContentType::Prompt),
        "KindDerived" => Ok(ContentType::KindDerived),
        "Skill" => Ok(ContentType::Skill),
        "DeepDive" => Ok(ContentType::DeepDive),
        other => Err(DbError::InvalidEnumValue {
            type_name: "ContentType".to_string(),
            value: other.to_string(),
        }),
    }
}

/// Convert Flow to TEXT for storage.
pub fn flow_to_text(flow: &Flow) -> &'static str {
    match flow {
        Flow::Research => "Research",
        Flow::Synthesis => "Synthesis",
        Flow::Manual => "Manual",
    }
}

/// Parse Flow from TEXT.
pub fn text_to_flow(s: &str) -> DbResult<Flow> {
    match s {
        "Research" => Ok(Flow::Research),
        "Synthesis" => Ok(Flow::Synthesis),
        "Manual" => Ok(Flow::Manual),
        other => Err(DbError::InvalidEnumValue {
            type_name: "Flow".to_string(),
            value: other.to_string(),
        }),
    }
}

/// Convert License to TEXT for JSON storage.
pub fn license_to_text(license: &License) -> String {
    match license {
        License::Proprietary => "Proprietary".to_string(),
        License::Mit => "Mit".to_string(),
        License::Bsd => "Bsd".to_string(),
        License::Gplv2 => "Gplv2".to_string(),
        License::Gplv3 => "Gplv3".to_string(),
        License::Isc => "Isc".to_string(),
        License::Mpl1_1 => "Mpl1_1".to_string(),
        License::Mpl2_0 => "Mpl2_0".to_string(),
        License::AGpl => "AGpl".to_string(),
        License::Apache2_0 => "Apache2_0".to_string(),
        License::Other(s) => format!("Other:{s}"),
    }
}

/// Parse License from TEXT.
pub fn text_to_license(s: &str) -> DbResult<License> {
    if let Some(custom) = s.strip_prefix("Other:") {
        return Ok(License::Other(custom.to_string()));
    }
    match s {
        "Proprietary" => Ok(License::Proprietary),
        "Mit" => Ok(License::Mit),
        "Bsd" => Ok(License::Bsd),
        "Gplv2" => Ok(License::Gplv2),
        "Gplv3" => Ok(License::Gplv3),
        "Isc" => Ok(License::Isc),
        "Mpl1_1" => Ok(License::Mpl1_1),
        "Mpl2_0" => Ok(License::Mpl2_0),
        "AGpl" => Ok(License::AGpl),
        "Apache2_0" => Ok(License::Apache2_0),
        other => Err(DbError::InvalidEnumValue {
            type_name: "License".to_string(),
            value: other.to_string(),
        }),
    }
}

/// Serialize Vec<License> to JSON TEXT.
pub fn licenses_to_json(licenses: &[License]) -> DbResult<String> {
    let strings: Vec<String> = licenses.iter().map(license_to_text).collect();
    Ok(serde_json::to_string(&strings)?)
}

/// Deserialize Vec<License> from JSON TEXT.
pub fn json_to_licenses(json: &str) -> DbResult<Vec<License>> {
    let strings: Vec<String> = serde_json::from_str(json)?;
    strings.iter().map(|s| text_to_license(s)).collect()
}

/// Convert LanguagePackageManager to TEXT for storage.
/// Uses serde_json to get the canonical variant name.
pub fn package_manager_to_text(pm: &LanguagePackageManager) -> String {
    // Serialize to get the canonical PascalCase name
    serde_json::to_string(pm)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string()
}

/// Parse LanguagePackageManager from TEXT.
/// Uses serde_json since LanguagePackageManager implements Deserialize.
pub fn text_to_package_manager(s: &str) -> DbResult<LanguagePackageManager> {
    // LanguagePackageManager serializes as a PascalCase string in JSON
    let json = format!("\"{s}\"");
    serde_json::from_str(&json).map_err(|_| DbError::InvalidEnumValue {
        type_name: "LanguagePackageManager".to_string(),
        value: s.to_string(),
    })
}

/// Convert ModelCapability to TEXT.
pub fn model_capability_to_text(mc: &ModelCapability) -> String {
    // Use serde to get the variant name
    serde_json::to_string(mc)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string()
}

/// Parse ModelCapability from TEXT.
pub fn text_to_model_capability(s: &str) -> DbResult<ModelCapability> {
    let json = format!("\"{s}\"");
    serde_json::from_str(&json).map_err(|_| DbError::InvalidEnumValue {
        type_name: "ModelCapability".to_string(),
        value: s.to_string(),
    })
}

// ============================================================================
// Row types
// ============================================================================

/// Row type for the `topics` table.
#[derive(Debug, sqlx::FromRow)]
pub struct TopicRow {
    pub name: String,
    pub kind: String,
    pub parent_topic_name: Option<String>,
    pub created: String,
    pub last_updated: String,
    pub brief: String,
    pub summary: String,
    pub when_to_use: String,
}

/// Row type for the `library_details` table.
#[derive(Debug, sqlx::FromRow)]
pub struct LibraryDetailsRow {
    pub topic_name: String,
    pub package_manager: String,
    pub package_name: String,
    pub features: Option<String>, // JSON array
    pub language: String,
    pub url: String,
    pub repo: Option<String>,
    pub docs: Option<String>,
    pub licenses: String, // JSON array
}

/// Row type for the `software_details` table.
#[derive(Debug, sqlx::FromRow)]
pub struct SoftwareDetailsRow {
    pub topic_name: String,
    pub name: String,
    pub company: Option<String>,
}

/// Row type for the `documents` table.
#[derive(Debug, sqlx::FromRow)]
pub struct DocumentRow {
    pub topic_name: String,
    pub filename: String,
    pub content_type: String,
    pub prompt: Option<String>,
    pub flow: Option<String>,
    pub created: String,
    pub last_updated: String,
    pub model: Option<String>,
    pub model_capability: Option<String>,
    pub content_hash: i64,
    pub interpolated_hash: i64,
}

// ============================================================================
// Row → Domain conversions
// ============================================================================

impl TopicRow {
    /// Convert to Topic with the provided KindCategory and children.
    ///
    /// Use this after fetching library/software details and children from DB.
    pub fn into_topic(
        self,
        kind: KindCategory,
        documents: Vec<Document>,
        children: Vec<Topic>,
    ) -> DbResult<Topic> {
        let created = DateTime::parse_from_rfc3339(&self.created)
            .map_err(|_| DbError::InvalidEnumValue {
                type_name: "DateTime".to_string(),
                value: self.created.clone(),
            })?
            .with_timezone(&Utc);

        let last_updated = DateTime::parse_from_rfc3339(&self.last_updated)
            .map_err(|_| DbError::InvalidEnumValue {
                type_name: "DateTime".to_string(),
                value: self.last_updated.clone(),
            })?
            .with_timezone(&Utc);

        Ok(Topic::with_metadata(
            self.name,
            kind,
            created,
            last_updated,
            self.brief,
            self.summary,
            self.when_to_use,
            documents,
            children,
        ))
    }
}

impl LibraryDetailsRow {
    /// Convert to Library domain type.
    pub fn into_library(self) -> DbResult<Library> {
        let package_manager = text_to_package_manager(&self.package_manager)?;
        let licenses = json_to_licenses(&self.licenses)?;
        let features: Option<Vec<String>> = self
            .features
            .as_ref()
            .map(|f| serde_json::from_str(f))
            .transpose()?;

        let mut lib = Library::new(
            package_manager,
            self.package_name,
            self.language,
            self.url,
        );

        if let Some(features) = features {
            lib.set_features(features);
        }
        if let Some(repo) = self.repo {
            lib.set_repo(repo);
        }
        if let Some(docs) = self.docs {
            lib.set_docs(docs);
        }
        for license in licenses {
            lib.add_license(license);
        }

        Ok(lib)
    }
}

impl SoftwareDetailsRow {
    /// Convert to Software domain type.
    pub fn into_software(self) -> Software {
        match self.company {
            Some(company) => Software::with_company(self.name, company),
            None => Software::new(self.name),
        }
    }
}

impl DocumentRow {
    /// Convert to Document domain type.
    pub fn into_document(self) -> DbResult<Document> {
        let content_type = text_to_content_type(&self.content_type)?;
        let flow = self.flow.as_ref().map(|f| text_to_flow(f)).transpose()?;
        let model_capability = self
            .model_capability
            .as_ref()
            .map(|mc| text_to_model_capability(mc))
            .transpose()?;

        let created = DateTime::parse_from_rfc3339(&self.created)
            .map_err(|_| DbError::InvalidEnumValue {
                type_name: "DateTime".to_string(),
                value: self.created.clone(),
            })?
            .with_timezone(&Utc);

        let last_updated = DateTime::parse_from_rfc3339(&self.last_updated)
            .map_err(|_| DbError::InvalidEnumValue {
                type_name: "DateTime".to_string(),
                value: self.last_updated.clone(),
            })?
            .with_timezone(&Utc);

        Ok(Document::with_metadata(
            self.filename,
            content_type,
            self.prompt,
            flow,
            created,
            last_updated,
            self.model,
            model_capability,
            i64_to_u64(self.content_hash),
            i64_to_u64(self.interpolated_hash),
        ))
    }
}

// ============================================================================
// Domain → Row conversions
// ============================================================================

/// Convert a Topic to TopicRow (base fields only).
pub fn topic_to_row(topic: &Topic, parent_name: Option<&str>) -> TopicRow {
    TopicRow {
        name: topic.name().to_string(),
        kind: kind_category_to_discriminator(topic.kind()).to_string(),
        parent_topic_name: parent_name.map(String::from),
        created: topic.created().to_rfc3339(),
        last_updated: topic.last_updated().to_rfc3339(),
        brief: topic.brief().to_string(),
        summary: topic.summary().to_string(),
        when_to_use: topic.when_to_use().to_string(),
    }
}

/// Extract LibraryDetailsRow from a Topic if it's a Library kind.
pub fn topic_to_library_row(topic: &Topic) -> Option<DbResult<LibraryDetailsRow>> {
    match topic.kind() {
        KindCategory::Library(lib) => {
            let licenses_result = licenses_to_json(&[]); // TODO: Need getter for licenses

        Some(licenses_result.map(|licenses| LibraryDetailsRow {
                topic_name: topic.name().to_string(),
                package_manager: package_manager_to_text(lib.package_manager()),
                package_name: lib.package_name().to_string(),
                features: None, // TODO: Need getter for features on Library
                language: lib.language().to_string(),
                url: lib.url().to_string(),
                repo: None,  // TODO: Need getter
                docs: None,  // TODO: Need getter
                licenses,
            }))
        }
        _ => None,
    }
}

/// Extract SoftwareDetailsRow from a Topic if it's a Software kind.
pub fn topic_to_software_row(topic: &Topic) -> Option<SoftwareDetailsRow> {
    match topic.kind() {
        KindCategory::Software(sw) => Some(SoftwareDetailsRow {
            topic_name: topic.name().to_string(),
            name: sw.name().to_string(),
            company: sw.company().map(String::from),
        }),
        _ => None,
    }
}

/// Convert a Document to DocumentRow.
pub fn document_to_row(doc: &Document, topic_name: &str) -> DocumentRow {
    DocumentRow {
        topic_name: topic_name.to_string(),
        filename: doc.filename().to_string(),
        content_type: content_type_to_text(doc.content_type()).to_string(),
        prompt: None, // TODO: Need getter for prompt
        flow: None,   // TODO: Need getter for flow
        created: Utc::now().to_rfc3339(), // TODO: Need getter for created
        last_updated: Utc::now().to_rfc3339(), // TODO: Need getter for last_updated
        model: None,  // TODO: Need getter
        model_capability: None, // TODO: Need getter
        content_hash: u64_to_i64(doc.content_hash()),
        interpolated_hash: 0, // TODO: Need getter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kind_category_discriminator_roundtrip() {
        // Simple variants
        assert_eq!(discriminator_to_kind_category("Person").unwrap(), KindCategory::Person);
        assert_eq!(discriminator_to_kind_category("SolutionArea").unwrap(), KindCategory::SolutionArea);
        assert_eq!(discriminator_to_kind_category("ProgrammingLanguage").unwrap(), KindCategory::ProgrammingLanguage);

        // Library/Software should error (need data)
        assert!(discriminator_to_kind_category("Library").is_err());
        assert!(discriminator_to_kind_category("Software").is_err());

        // Invalid
        assert!(discriminator_to_kind_category("Invalid").is_err());
    }

    #[test]
    fn test_content_type_roundtrip() {
        let types = [
            ContentType::Static,
            ContentType::Template,
            ContentType::Prompt,
            ContentType::KindDerived,
            ContentType::Skill,
            ContentType::DeepDive,
        ];

        for ct in types {
            let text = content_type_to_text(&ct);
            let parsed = text_to_content_type(text).unwrap();
            assert_eq!(parsed, ct);
        }
    }

    #[test]
    fn test_flow_roundtrip() {
        let flows = [Flow::Research, Flow::Synthesis, Flow::Manual];

        for flow in flows {
            let text = flow_to_text(&flow);
            let parsed = text_to_flow(text).unwrap();
            assert_eq!(parsed, flow);
        }
    }

    #[test]
    fn test_license_roundtrip() {
        let licenses = [
            License::Proprietary,
            License::Mit,
            License::Bsd,
            License::Gplv2,
            License::Gplv3,
            License::Isc,
            License::Mpl1_1,
            License::Mpl2_0,
            License::AGpl,
            License::Apache2_0,
            License::Other("Custom-1.0".to_string()),
        ];

        for license in licenses {
            let text = license_to_text(&license);
            let parsed = text_to_license(&text).unwrap();
            // Compare discriminants since Other contains data
            match (&license, &parsed) {
                (License::Other(a), License::Other(b)) => assert_eq!(a, b),
                _ => assert_eq!(std::mem::discriminant(&license), std::mem::discriminant(&parsed)),
            }
        }
    }

    #[test]
    fn test_licenses_json_roundtrip() {
        let licenses = vec![License::Mit, License::Apache2_0, License::Other("Custom".to_string())];
        let json = licenses_to_json(&licenses).unwrap();
        let parsed = json_to_licenses(&json).unwrap();
        assert_eq!(licenses.len(), parsed.len());
    }

    #[test]
    fn test_package_manager_roundtrip() {
        let managers = [
            LanguagePackageManager::Cargo,
            LanguagePackageManager::Npm,
            LanguagePackageManager::Pip,
            LanguagePackageManager::GoModules,
        ];

        for pm in managers {
            let text = package_manager_to_text(&pm);
            let parsed = text_to_package_manager(&text).unwrap();
            assert_eq!(pm, parsed);
        }
    }

    #[test]
    fn test_model_capability_roundtrip() {
        let capabilities = [
            ModelCapability::Fast,
            ModelCapability::Normal,
            ModelCapability::Smart,
            ModelCapability::FastCheap,
        ];

        for mc in capabilities {
            let text = model_capability_to_text(&mc);
            let parsed = text_to_model_capability(&text).unwrap();
            assert_eq!(mc, parsed);
        }
    }

    #[test]
    fn test_document_row_conversion() {
        let row = DocumentRow {
            topic_name: "test".to_string(),
            filename: "test.md".to_string(),
            content_type: "Static".to_string(),
            prompt: None,
            flow: Some("Research".to_string()),
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-02T00:00:00Z".to_string(),
            model: Some("gpt-4".to_string()),
            model_capability: Some("Normal".to_string()),
            content_hash: 12345,
            interpolated_hash: 0,
        };

        let doc = row.into_document().unwrap();
        assert_eq!(doc.filename(), "test.md");
        assert_eq!(*doc.content_type(), ContentType::Static);
        assert_eq!(doc.content_hash(), 12345);
    }

    #[test]
    fn test_software_details_row_conversion() {
        let row = SoftwareDetailsRow {
            topic_name: "vscode".to_string(),
            name: "Visual Studio Code".to_string(),
            company: Some("Microsoft".to_string()),
        };

        let software = row.into_software();
        assert_eq!(software.name(), "Visual Studio Code");
        assert_eq!(software.company(), Some("Microsoft"));
    }
}
