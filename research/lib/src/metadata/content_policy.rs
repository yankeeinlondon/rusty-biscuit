use serde::{Deserialize, Serialize};
use sniff_lib::package::LanguagePackageManager;

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct LibraryVariant {
    package_manager: LanguagePackageManager,
    package_name: String,
    version: String,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct SoftwareVariant {
    name: String,
    url: String,
    version: String
}


#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum ContentExpiry {
    /// expires after set number of days
    Days(u32),
    /// expires after set number of months
    Months(u32),
    /// expires after set number of years
    Years(u32),

    /// the `hash_content` property
    ContentHashConflict,

    /// the document had the `stale` frontmatter property set to **true**
    Flagged,

    /// When the software is updated the document becomes stale.
    SoftwareUpdate(SoftwareVariant),
    MajorLibraryUpdate(LibraryVariant),
    MinorLibraryUpdate(LibraryVariant),

    ModelArchived
}

/// A `ContentPolicy` is nothing more than
#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct ContentPolicy(Vec<ContentExpiry>);


