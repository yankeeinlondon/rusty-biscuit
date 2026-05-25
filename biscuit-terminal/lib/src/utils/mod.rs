pub mod block_constraint;
pub mod color;
pub mod escape_codes;
pub mod layout;
pub mod multiplex;
pub mod styling;
pub mod term_color;
pub mod text;
pub mod word_wrap;
pub mod wrap_policy;

// Re-export unicode-width traits so downstream crates don't need a direct dependency
pub use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
