//! Rendering utilities for multiple output formats
//!
//! This module provides types that can render content to different
//! output formats including terminal (with ANSI escape sequences),
//! HTML, and Markdown.

pub mod image_ref;
pub mod link;
pub(crate) mod metadata_codec;
pub(crate) mod reference_parse;
pub mod stylesheet;

pub use image_ref::{
    FetchPriority, ImageDecoding, ImageLoading, ImageRef, ImageRefError, ReferrerPolicy,
};
pub use link::{Link, LinkError, LinkParseError, LinkTarget, LinkType};
pub use stylesheet::{
    CssColor, CssColorProp, CssCustomProp, CssIntegerProp, CssProp, CssRaw, CssSizing,
    CssSizingMulti, CssSizingMultiProp, CssSizingProp, CssStyle, CssUnit, CssValue, CssValueKind,
    StylesheetBlockError, StylesheetError, TerminalCss,
};
