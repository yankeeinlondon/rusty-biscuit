use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sniff_lib::package::LanguagePackageManager;
use ai_pipeline::models::model_capability::ModelCapability;
use thiserror::Error;

/// The type of content in a research document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentType {
    /// High-level overview of the topic
    Overview,
    /// Comparison with similar libraries/tools
    SimilarLibraries,
    /// Libraries that integrate well with this topic
    IntegrationPartners,
    /// Common use cases and patterns
    UseCases,
    /// Version history and changes
    Changelog,
    /// Response to a custom user question
    CustomQuestion,
    /// In-depth technical reference
    DeepDive,
    /// Very short summary (1-2 sentences)
    Brief,
    /// Claude Code skill file
    Skill,
}

impl ContentType {
    /// Infer content type from filename.
    ///
    /// Returns `None` if the filename doesn't match a known pattern.
    pub fn from_filename(filename: &str) -> Option<Self> {
        let stem = Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename);

        match stem.to_lowercase().as_str() {
            "overview" => Some(Self::Overview),
            "similar_libraries" => Some(Self::SimilarLibraries),
            "integration_partners" => Some(Self::IntegrationPartners),
            "use_cases" => Some(Self::UseCases),
            "changelog" => Some(Self::Changelog),
            "deep_dive" => Some(Self::DeepDive),
            "brief" => Some(Self::Brief),
            "skill" => Some(Self::Skill),
            s if s.starts_with("question_") => Some(Self::CustomQuestion),
            _ => None,
        }
    }
}

/// The AI workflow/flow used to generate a document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Flow {
    /// Research workflow for underlying research documents
    Research,
    /// Synthesis workflow for final deliverables
    Synthesis,
    /// Manual creation (not AI-generated)
    Manual,
}

/// Error type for `TryFrom` conversions to `Document`.
#[derive(Debug, Error)]
pub enum DocumentConversionError {
    #[error("Invalid document path: {0}")]
    InvalidPath(String),
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum License {
    Proprietary,
    Mit,
    Bsd,
    Gplv2,
    Gplv3,
    Isc,
    Mpl1_1,
    Mpl2_0,
    AGpl,
    Apache2_0,
    Other(String)
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    /// The package manager used to install this library
    package_manager: LanguagePackageManager,
    /// The name of the package on the given package manager
    package_name: String,
    /// The optional feature flags offered by this package (where appropriate)
    #[serde(skip_serializing_if = "Option::is_none")]
    features: Option<Vec<String>>,
    /// The programming language this library is for
    language: String,
    /// The URL to the library on the package manager's site
    url: String,
    /// The URL to the repo of the library
    #[serde(skip_serializing_if = "Option::is_none")]
    repo: Option<String>,
    /// The URL to the documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    docs: Option<String>,
    /// The licenses this library is available under
    licenses: Vec<License>,
}

impl Library {
    /// Create a new Library instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        package_manager: LanguagePackageManager,
        package_name: String,
        language: String,
        url: String,
    ) -> Self {
        Self {
            package_manager,
            package_name,
            features: None,
            language,
            url,
            repo: None,
            docs: None,
            licenses: Vec::new(),
        }
    }

    /// Get the package manager.
    pub fn package_manager(&self) -> &LanguagePackageManager {
        &self.package_manager
    }

    /// Get the package name.
    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    /// Get the language.
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Get the URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Set the repository URL.
    pub fn set_repo(&mut self, repo: String) {
        self.repo = Some(repo);
    }

    /// Set the documentation URL.
    pub fn set_docs(&mut self, docs: String) {
        self.docs = Some(docs);
    }

    /// Set feature flags.
    pub fn set_features(&mut self, features: Vec<String>) {
        self.features = Some(features);
    }

    /// Add a license.
    pub fn add_license(&mut self, license: License) {
        self.licenses.push(license);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Software {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    company: Option<String>,
}

impl Software {
    /// Create a new Software instance.
    pub fn new(name: String) -> Self {
        Self { name, company: None }
    }

    /// Create a new Software instance with company.
    pub fn with_company(name: String, company: String) -> Self {
        Self {
            name,
            company: Some(company),
        }
    }

    /// Get the software name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the company name.
    pub fn company(&self) -> Option<&str> {
        self.company.as_deref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KindCategory {
    Library(Library),
    Software(Software),
    Person,
    SolutionArea,
    ProgrammingLanguage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Filename relative to research root
    filename: String,
    /// The content type of the document
    content_type: ContentType,
    /// The prompt used to generate the document
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
    /// The AI workflow used to generate the document
    #[serde(skip_serializing_if = "Option::is_none")]
    flow: Option<Flow>,
    /// The last updated date/time of the document
    last_updated: DateTime<Utc>,
    /// The date/time this document was first created
    created: DateTime<Utc>,
    /// The actual AI model used to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    /// The model capability this document requires to generate the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    model_capability: Option<ModelCapability>,
    /// xxHash of the document content
    content_hash: u64,
    /// xxHash of the interpolated prompt
    interpolated_hash: u64,
}

impl TryFrom<String> for Document {
    type Error = DocumentConversionError;

    fn try_from(_value: String) -> Result<Self, Self::Error> {
        // TODO: Parse document from string path or content
        todo!("Document::try_from(String) not yet implemented")
    }
}

impl TryFrom<&str> for Document {
    type Error = DocumentConversionError;

    fn try_from(_value: &str) -> Result<Self, Self::Error> {
        // TODO: Parse document from string path or content
        todo!("Document::try_from(&str) not yet implemented")
    }
}

impl TryFrom<&String> for Document {
    type Error = DocumentConversionError;

    fn try_from(_value: &String) -> Result<Self, Self::Error> {
        // TODO: Parse document from string path or content
        todo!("Document::try_from(&String) not yet implemented")
    }
}

impl Document {
    /// Create a new Document from a file path.
    ///
    /// The content type is inferred from the filename.
    pub fn new(filepath: &Path) -> Self {
        let filename = filepath
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let content_type = ContentType::from_filename(&filename)
            .unwrap_or(ContentType::CustomQuestion);

        let now = Utc::now();

        Self {
            filename,
            content_type,
            prompt: None,
            flow: None,
            last_updated: now,
            created: now,
            model: None,
            model_capability: None,
            content_hash: 0,
            interpolated_hash: 0,
        }
    }

    /// Create a Document with full metadata.
    #[allow(clippy::too_many_arguments)]
    pub fn with_metadata(
        filename: String,
        content_type: ContentType,
        prompt: Option<String>,
        flow: Option<Flow>,
        created: DateTime<Utc>,
        last_updated: DateTime<Utc>,
        model: Option<String>,
        model_capability: Option<ModelCapability>,
        content_hash: u64,
        interpolated_hash: u64,
    ) -> Self {
        Self {
            filename,
            content_type,
            prompt,
            flow,
            last_updated,
            created,
            model,
            model_capability,
            content_hash,
            interpolated_hash,
        }
    }

    /// Get the filename.
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Get the content type.
    pub fn content_type(&self) -> &ContentType {
        &self.content_type
    }

    /// Get the content hash.
    pub fn content_hash(&self) -> u64 {
        self.content_hash
    }

    /// Set the content hash.
    pub fn set_content_hash(&mut self, hash: u64) {
        self.content_hash = hash;
    }

    /// Update the last_updated timestamp to now.
    pub fn touch(&mut self) {
        self.last_updated = Utc::now();
    }
}

/// A research topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    /// The topic's name
    name: String,
    /// The kind category which the topic belongs to
    kind: KindCategory,
    /// The datetime at which this topic was first created
    created: DateTime<Utc>,
    /// The datetime the final deliverables for this topic were last produced
    last_updated: DateTime<Utc>,
    /// A very brief one sentence description of the topic
    brief: String,
    /// A summary of the topic in 1-2 paragraphs
    summary: String,
    /// A description of "when to use" this topic as a skill
    when_to_use: String,
    /// The core documents which make up this research topic (includes both
    /// underlying documents and final deliverables)
    documents: Vec<Document>,
    /// If this topic contains other skills they will be listed here
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    children: Vec<Topic>,
}


impl Topic {
    /// Create a new topic with the given name and kind.
    pub fn new(name: String, kind: KindCategory) -> Self {
        let now = Utc::now();
        Self {
            name,
            kind,
            created: now,
            last_updated: now,
            brief: String::new(),
            summary: String::new(),
            when_to_use: String::new(),
            documents: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Create a topic with full metadata.
    #[allow(clippy::too_many_arguments)]
    pub fn with_metadata(
        name: String,
        kind: KindCategory,
        created: DateTime<Utc>,
        last_updated: DateTime<Utc>,
        brief: String,
        summary: String,
        when_to_use: String,
        documents: Vec<Document>,
        children: Vec<Topic>,
    ) -> Self {
        Self {
            name,
            kind,
            created,
            last_updated,
            brief,
            summary,
            when_to_use,
            documents,
            children,
        }
    }

    /// Get the topic name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the kind category.
    pub fn kind(&self) -> &KindCategory {
        &self.kind
    }

    /// Get the created timestamp.
    pub fn created(&self) -> DateTime<Utc> {
        self.created
    }

    /// Get the last updated timestamp.
    pub fn last_updated(&self) -> DateTime<Utc> {
        self.last_updated
    }

    /// Get the brief description.
    pub fn brief(&self) -> &str {
        &self.brief
    }

    /// Set the brief description.
    pub fn set_brief(&mut self, brief: String) {
        self.brief = brief;
    }

    /// Get the summary.
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// Set the summary.
    pub fn set_summary(&mut self, summary: String) {
        self.summary = summary;
    }

    /// Get the when_to_use description.
    pub fn when_to_use(&self) -> &str {
        &self.when_to_use
    }

    /// Set the when_to_use description.
    pub fn set_when_to_use(&mut self, when_to_use: String) {
        self.when_to_use = when_to_use;
    }

    /// Get the documents.
    pub fn documents(&self) -> &[Document] {
        &self.documents
    }

    /// Get mutable access to documents.
    pub fn documents_mut(&mut self) -> &mut Vec<Document> {
        &mut self.documents
    }

    /// Add a document to this topic.
    pub fn add_document(&mut self, document: Document) {
        self.documents.push(document);
        self.last_updated = Utc::now();
    }

    /// Get the children topics.
    pub fn children(&self) -> &[Topic] {
        &self.children
    }

    /// Add a child topic.
    pub fn add_child(&mut self, child: Topic) {
        self.children.push(child);
    }

    /// Update the last_updated timestamp to now.
    pub fn touch(&mut self) {
        self.last_updated = Utc::now();
    }
}
