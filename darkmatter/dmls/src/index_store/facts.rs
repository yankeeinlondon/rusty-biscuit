//! The stored per-file facts: the subset of a [`DocumentIndex`] that the
//! references, rename, and workspace-symbol requests need.
//!
//! **Row invariant:** a row's facts depend only on `(path, bytes,
//! FACTS_VERSION)`. Nothing resolved against configuration or a
//! `FileResolutionContext` is stored; a link keeps its raw target and is
//! resolved at query time, in the temporary graph.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::graph::{DocumentIndex, HeadingFact, LinkFact, TransclusionFact, WikiLinkFact};

use super::keys::candidate_keys;

/// One Markdown file's stored facts.
///
/// [`FileFacts::from_index`] and [`FileFacts::into_index`] are inverse on the
/// stored fields: `into_index` leaves the unstored ones (`schema_uses`,
/// `file_uses`, `variable_uses`, `frontmatter_keys`) empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFacts {
    /// The file's path as the file list gives it (absolute, canonical).
    pub path: PathBuf,
    /// Raw-bytes xxHash, the same value as [`DocumentIndex::content_hash`].
    pub hash: u64,
    pub headings: Vec<HeadingFact>,
    /// Each link's [`LinkFact::target`] is derived from its raw target, so a
    /// row stores only the raw target.
    pub links: Vec<LinkFact>,
    pub wiki_links: Vec<WikiLinkFact>,
    pub transclusions: Vec<TransclusionFact>,
    /// The candidate keys of every outgoing link, wiki link, and
    /// transclusion (see [`super::keys`]).
    pub candidate_keys: BTreeSet<String>,
}

impl FileFacts {
    /// The stored facts of `index`, with its candidate keys.
    pub fn from_index(index: &DocumentIndex) -> Self {
        Self {
            path: index.path.clone(),
            hash: index.content_hash,
            headings: index.headings.clone(),
            links: index.links.clone(),
            wiki_links: index.wiki_links.clone(),
            transclusions: index.transclusions.clone(),
            candidate_keys: candidate_keys(index),
        }
    }

    /// A [`DocumentIndex`] carrying these facts, whose `content_hash` is the
    /// row hash and whose unstored fields are empty.
    pub fn into_index(self) -> DocumentIndex {
        DocumentIndex {
            path: self.path,
            content_hash: self.hash,
            headings: self.headings,
            links: self.links,
            wiki_links: self.wiki_links,
            transclusions: self.transclusions,
            schema_uses: Vec::new(),
            file_uses: Vec::new(),
            variable_uses: Vec::new(),
            frontmatter_keys: Vec::new(),
        }
    }
}
