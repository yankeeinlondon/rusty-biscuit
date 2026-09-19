//! The dependency the two packages under test configure differently.

/// Whether this unit was compiled with the `extra` feature.
///
/// A `cfg!` rather than a `#[cfg]` item so both feature resolutions expose the
/// same API and only the answer differs: a consumer's test then observes the
/// feature graph it was built against rather than failing to compile.
pub const fn has_extra() -> bool {
    cfg!(feature = "extra")
}
