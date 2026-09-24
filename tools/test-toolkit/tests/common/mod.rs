//! Helpers shared by more than one test binary in this package. Each binary
//! compiles this module separately, so a helper one binary does not call is
//! dead code there rather than a defect.
#![allow(dead_code)]

#[cfg(unix)]
pub mod kache;
