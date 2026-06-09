//! Tunable constants for god-file detection.

/// Lower bound of the moderate risk band (effective SLOC).
pub const MODERATE_MIN_SLOC: usize = 400;

/// Lower bound of the high risk band (effective SLOC).
pub const HIGH_MIN_SLOC: usize = 1000;

/// Maximum symbol blocks listed per file.
pub const MAX_BLOCKS: usize = 8;

/// Floor below which a symbol is not listed as a block.
pub const MIN_BLOCK_SLOC: usize = 15;

/// Member count above which a container gets a call-out.
pub const MANY_MEMBERS_THRESHOLD: usize = 10;
