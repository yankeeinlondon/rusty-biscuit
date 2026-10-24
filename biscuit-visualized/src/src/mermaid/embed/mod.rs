//! The embedded-font opt-in of the HTML static target: the face each
//! requested family is measured with and embedded as, decided from the face's
//! own `OS/2` `fsType` ([`permission`], [`faces`]); the code points and
//! weights each static SVG draws ([`glyphs`]); the subset of each face
//! ([`subset`]); and the page's `@font-face` rules with the comments that say
//! which face was embedded and why ([`embedding`], [`comment`]).

pub(crate) mod comment;
pub(crate) mod embedding;
pub(crate) mod faces;
pub(crate) mod glyphs;
pub(crate) mod permission;
pub(crate) mod subset;
