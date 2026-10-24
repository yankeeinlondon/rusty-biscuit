//! The embedded-font opt-in of the HTML static target: the face each
//! requested family is measured with and embedded as, decided from the face's
//! own `OS/2` `fsType` ([`permission`], [`faces`]); the code points and
//! weights each static SVG draws ([`glyphs`]); the subset of each face
//! ([`subset`]); and the page's `@font-face` rules with the comments that say
//! which face was embedded and why ([`embedding`], [`comment`]); and what a
//! rendered diagram hands its page, with the page's markup ([`page`]).

pub(crate) mod comment;
pub(crate) mod embedding;
pub(crate) mod faces;
pub(crate) mod glyphs;
pub(crate) mod page;
pub(crate) mod permission;
pub(crate) mod subset;

pub use embedding::FontEmbeddingError;
pub use page::{DiagramFont, PageFont, page_font};
pub use subset::SubsetError;
