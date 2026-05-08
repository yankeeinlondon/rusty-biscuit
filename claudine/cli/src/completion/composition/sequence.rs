//! Sequence partial handling for the composition completer.
//!
//! The sequence mode is handled by frontmatter filtering
//! (`frontmatter::valid_for_mode` with [`ComposeMode::Sequence`])
//! rather than by dedicated functions in this module. See [`super::compose`]
//! for the shared Word/Empty partial pipeline.
