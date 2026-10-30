//! The SimplifiedSchema language, independent of any Markdown host.
//!
//! The crate is being assembled from Darkmatter's schema modules; it currently
//! holds the shared foundations every schema module builds on:
//!
//! - [`numeric`]: admission of written numbers into finite `f64`s;
//! - [`span`]: byte spans over the exact supplied text;
//! - [`expression`]: passive `{{ … }}` syntax (scan, lexer, parser, AST),
//!   never evaluated;
//! - [`pending`]: the `{{!data:v1:…}}` literal-token codec.
//!
//! See the package's `docs/` for the crate boundary.

pub mod expression;
pub mod numeric;
pub mod pending;
pub mod span;
#[cfg(feature = "work-counters")]
pub mod work_counters;

/// Records one unit of `work_counters::Work`:
/// `note_work!(ExpressionParse)`. Expands to nothing without the
/// `work-counters` feature.
macro_rules! note_work {
    ($work:ident) => {
        #[cfg(feature = "work-counters")]
        $crate::work_counters::add($crate::work_counters::Work::$work, 1);
    };
}
pub(crate) use note_work;

pub use span::{SourceSpan, Spanned, line_col_of_offset, line_of_offset};
