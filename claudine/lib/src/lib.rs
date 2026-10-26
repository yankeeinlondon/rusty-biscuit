// Production builds may not construct or clear a child command directly; see
// `child_environment` and `clippy.toml`. Tests keep the plain constructors.
#![cfg_attr(not(test), deny(clippy::disallowed_methods))]
// `large-error-threshold` in `clippy.toml` is sized to `CompositionError` on
// Unix; a Windows `PathBuf` is 8 bytes wider, which lifts the enum to 144.
// Linux lint runs keep enforcing the threshold.
#![cfg_attr(windows, allow(clippy::result_large_err))]

pub mod actions;
pub mod badges;
pub mod child_environment;
pub mod composition;
pub mod config;
pub mod diagnostics;
pub mod dispatch;
pub mod error;
pub mod events;
pub mod harness;
pub mod hook_adapters;
pub mod interrupt;
pub mod invocation_context;
pub mod linking;
pub mod mcp;
pub mod messaging;
pub mod model_catalog;
mod npm_shim;
pub mod opencode_config;
mod path_semantics;
pub mod permissions;
pub mod protect;
pub mod provider;
pub mod provider_id;
pub mod provider_overlay;
pub mod render;
pub mod reporting;
pub mod runaway;
pub mod secrets;
pub mod signals;
pub mod steering;
pub mod stream;
pub mod system_prompt;
#[cfg(feature = "work-counters")]
pub mod work_counters;

/// Records units of [`Work`](crate::work_counters::Work): `note_work!(RuntimeSnapshot)` adds one,
/// `note_work!(RuntimeSnapshot, n)` adds `n`. Expands to nothing without the
/// `work-counters` feature, so `n` is not evaluated in production builds.
#[macro_export]
#[doc(hidden)]
macro_rules! note_work {
    ($work:ident) => {
        $crate::note_work!($work, 1)
    };
    ($work:ident, $count:expr) => {
        #[cfg(feature = "work-counters")]
        $crate::work_counters::add($crate::work_counters::Work::$work, ($count) as u64);
    };
}

#[cfg(test)]
pub(crate) mod test_support;

// The spike's crate-private scenarios measure allocations; a unit-test binary
// is a final binary, so it may declare the counting allocator.
#[cfg(all(test, feature = "work-counters"))]
#[global_allocator]
static SPIKE_ALLOCATOR: test_toolkit::alloc_count::CountingAllocator = test_toolkit::alloc_count::CountingAllocator;
#[cfg(all(test, feature = "work-counters"))]
mod pipeline_spike;
