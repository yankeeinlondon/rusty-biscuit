//! Inline-compose partial handling for the composition completer.
//!
//! The inline-compose mode is handled by frontmatter filtering
//! (`frontmatter::valid_for_mode` with [`ComposeMode::InlineCompose`])
//! rather than by dedicated functions in this module. See [`super::compose`]
//! for the shared Word/Empty partial pipeline.
