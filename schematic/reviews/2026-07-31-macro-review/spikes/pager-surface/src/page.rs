//! Stub envelope. The real response envelope (D7, D21) is designed in another
//! spike; this carries only what a pager needs to hand back.

use reqwest::header::HeaderMap;

#[derive(Debug)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub headers: HeaderMap,
    /// Present only when the strategy authored a total source and the
    /// response carried it.
    pub total: Option<u64>,
}

impl<T> Page<T> {
    pub fn items(&self) -> &[T] {
        &self.items
    }
}
