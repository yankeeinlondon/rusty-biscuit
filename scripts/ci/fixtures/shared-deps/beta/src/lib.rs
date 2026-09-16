//! The package that takes `shared-deps-divergent` without its `extra` feature.

/// What this package's own feature graph resolved for its divergent dependency.
pub const fn divergent_has_extra() -> bool {
    shared_deps_divergent::has_extra()
}

/// The identically configured dependency, read so it is genuinely linked.
pub const fn common_mark() -> &'static str {
    shared_deps_common::MARK
}
