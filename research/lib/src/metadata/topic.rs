use std::path::Path;

use serde::{Deserialize, Serialize};
use sniff::prelude::*;
use ai_pipeline::prelude::*;
use thiserror::Error;

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


pub struct Library {
    /// the Package Manager used to install this library
    package_manager: LibraryPackageManager,
    /// the name of the package on the given Package Manager
    package_name: String,
    /// the optional feature flags offered by this package (where appropriate)
    features: Option<Vec<String>>,
    /// the programming language this library is for
    language: String,
    /// The URL to the library on the package manager's site
    url: String,
    /// The URL to the repo of the library
    repo: Option<String>,
    docs: Option<String>,
    licenses: Vec<License>,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Software {
    name: String,
    company: Option<String>,

}


#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum KindCategory {
    Library(Library),
    Software(Software),
    Person,
    SolutionArea,
    ProgrammingLanguage,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Document {
    /// filename relative to research root
    filename: String,
    /// the content type of the document
    content_type: ContentType,
    /// the prompt used to generate the document
    prompt: Option<String>,
    /// the AI workflow used to generate the document
    flow: Option<Flow>,
    /// the last updated date/time of the document
    last_updated: DateTime,
    /// the date/time this document was first created
    created: DateTime,

    /// the actual AI model used to generate
    model: Option<String>,
    /// the model capability this document
    /// requires to generate the prompt
    model_capability: Option<ModelCapability>,

    content_hash: u64,
    interpolated_hash: u64
}

impl TryFrom<String> for Document {
    type Error = DocumentConversionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<&str> for Document {
    type Error = DocumentConversionError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<&String> for Document {
    type Error = DocumentConversionError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl Document {
    pub fn new(filepath: Path) -> Self {
        todo!()
    }
}

/// A research topic.
#[derive(Debug,Clone,Serialize)]
pub struct Topic {
    /// the Topic's name
    name: String,
    /// the kind category which the topic belongs to
    kind: KindCategory,
    /// the Datetime at which the this topic was first created
    created: DateTime,
    /// the Datetime the final deliverables for this topic were last produced
    last_updated: DateTime,
    /// a VERY brief one sentence description of the topic
    brief: String,
    /// a summary of the topic in 1-2 paragraphs
    summary: String,
    /// a description of "when to use" this topic as a skill
    when_to_use: String,

    documents: Vec<Document>
}


impl Topic {
    /// this is meant to return the details which are specific to the
    /// topic's `kind`
    fn details(self) {
        todo!()
    }
}
