//! Testing utilities for terminal output verification.
//!
//! This module provides utilities for testing terminal output, including ANSI
//! escape sequence handling and snapshot testing patterns.
//!
//! ## Testing Strategies
//!
//! ### Text-Based Verification
//!
//! Use `strip_ansi_codes()` to remove ANSI escape sequences for plain-text
//! assertions when you don't care about colors/formatting:
//!
//! ```rust
//! use darkmatter_lib::testing::strip_ansi_codes;
//!
//! let output = "\x1b[31mError:\x1b[0m Something went wrong";
//! assert_eq!(strip_ansi_codes(output), "Error: Something went wrong");
//! ```
//!
//! ### Color/Formatting Verification
//!
//! Use `TestTerminal` to verify specific ANSI codes are present:
//!
//! ```rust
//! use darkmatter_lib::testing::TestTerminal;
//!
//! let mut terminal = TestTerminal::new();
//! terminal.run(|term| {
//!     term.push_str("\x1b[31mError\x1b[0m");
//! });
//! terminal.assert_has_color("\x1b[31m"); // Red foreground
//! ```

pub mod terminal;

pub use terminal::{TestTerminal, strip_ansi_codes};
