//! A counting global allocator for allocation measurements in benches.
//!
//! [`CountingAllocator`] wraps [`std::alloc::System`] and keeps process-wide
//! totals of allocations, bytes allocated, and live bytes, so work spread
//! across threads is counted. A global allocator can only be declared by the
//! final binary, so a bench target that measures allocations declares it
//! itself; libraries never do:
//!
//! ```ignore
//! #[global_allocator]
//! static ALLOCATOR: test_toolkit::alloc_count::CountingAllocator =
//!     test_toolkit::alloc_count::CountingAllocator;
//!
//! let before = test_toolkit::alloc_count::snapshot();
//! run_scenario();
//! let used = test_toolkit::alloc_count::snapshot().since(before);
//! ```
//!
//! Timing benches keep the system allocator: counting costs a few atomic
//! operations per allocation, which distorts short timings.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static BYTES_ALLOCATED: AtomicU64 = AtomicU64::new(0);
static LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
static PEAK_LIVE_BYTES: AtomicU64 = AtomicU64::new(0);

/// [`System`], counting what passes through it.
pub struct CountingAllocator;

fn note_alloc(size: usize) {
    let size = size as u64;
    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    BYTES_ALLOCATED.fetch_add(size, Ordering::Relaxed);
    let live = LIVE_BYTES.fetch_add(size, Ordering::Relaxed) + size;
    PEAK_LIVE_BYTES.fetch_max(live, Ordering::Relaxed);
}

fn note_dealloc(size: usize) {
    LIVE_BYTES.fetch_sub(size as u64, Ordering::Relaxed);
}

// SAFETY: every call is forwarded unchanged to `System`; the counters only
// observe sizes and never touch the memory.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded with the caller's layout.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            note_alloc(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded with the caller's layout.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            note_alloc(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: forwarded with the caller's pointer and layout.
        unsafe { System.dealloc(pointer, layout) };
        note_dealloc(layout.size());
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: forwarded with the caller's pointer, layout, and size.
        let moved = unsafe { System.realloc(pointer, layout, new_size) };
        if !moved.is_null() {
            // A reallocation counts as one allocation of the new size and
            // the release of the old one.
            note_dealloc(layout.size());
            note_alloc(new_size);
        }
        moved
    }
}

/// Allocation totals at one moment, or between two ([`Self::since`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllocationSnapshot {
    pub allocations: u64,
    pub bytes_allocated: u64,
    /// The most bytes live at once. In a [`Self::since`] delta: the peak
    /// reached after the earlier snapshot, above the live bytes at that
    /// snapshot (see [`reset_peak`]).
    pub peak_live_bytes: u64,
    live_bytes: u64,
}

impl AllocationSnapshot {
    /// The allocation work between `earlier` and this snapshot.
    pub fn since(self, earlier: AllocationSnapshot) -> AllocationSnapshot {
        AllocationSnapshot {
            allocations: self.allocations - earlier.allocations,
            bytes_allocated: self.bytes_allocated - earlier.bytes_allocated,
            peak_live_bytes: self.peak_live_bytes.saturating_sub(earlier.live_bytes),
            live_bytes: self.live_bytes.saturating_sub(earlier.live_bytes),
        }
    }
}

/// The current totals. All zero unless the binary declared
/// [`CountingAllocator`] as its global allocator.
pub fn snapshot() -> AllocationSnapshot {
    AllocationSnapshot {
        allocations: ALLOCATIONS.load(Ordering::Relaxed),
        bytes_allocated: BYTES_ALLOCATED.load(Ordering::Relaxed),
        peak_live_bytes: PEAK_LIVE_BYTES.load(Ordering::Relaxed),
        live_bytes: LIVE_BYTES.load(Ordering::Relaxed),
    }
}

/// Lowers the recorded peak to the current live bytes, so the next
/// [`snapshot`]'s peak describes only what follows. Call it just before the
/// snapshot that starts a measurement.
pub fn reset_peak() {
    PEAK_LIVE_BYTES.store(LIVE_BYTES.load(Ordering::Relaxed), Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_subtracts_counts_and_measures_peak_above_the_baseline() {
        let earlier = AllocationSnapshot { allocations: 3, bytes_allocated: 100, peak_live_bytes: 80, live_bytes: 50 };
        let later = AllocationSnapshot { allocations: 5, bytes_allocated: 400, peak_live_bytes: 350, live_bytes: 60 };
        let delta = later.since(earlier);
        assert_eq!((delta.allocations, delta.bytes_allocated, delta.peak_live_bytes), (2, 300, 300));
    }
}
