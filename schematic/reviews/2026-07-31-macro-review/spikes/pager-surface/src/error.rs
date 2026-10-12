use std::time::Duration;

/// Returned by `.page_size(n)` before any request exists.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("page size {requested} is outside the declared range {min}..={}", max.map_or("unbounded".to_owned(), |m| m.to_string()))]
pub struct PageSizeError {
    pub requested: u32,
    pub min: u32,
    pub max: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum PagerError {
    #[error(transparent)]
    PageSize(#[from] PageSizeError),

    /// D12: the URL's origin is neither the base origin nor a declared extra
    /// origin. Returned before anything is sent.
    #[error("refused to send to {url}: origin {origin} is not the base origin or a declared extra origin")]
    HostPolicy { url: String, origin: String },

    /// D13: the shared deadline is spent. No request is dispatched after this.
    #[error("deadline exceeded after {completed_pages} complete page(s) in {elapsed:?}")]
    DeadlineExceeded {
        completed_pages: usize,
        elapsed: Duration,
    },

    /// A continuation the pager has already followed came back again.
    #[error("pagination loop: continuation {continuation:?} repeated after {completed_pages} page(s)")]
    Stalled {
        continuation: String,
        completed_pages: usize,
    },

    #[error("invalid continuation: {0}")]
    InvalidContinuation(String),

    /// The provider said there is more but supplied no continuation.
    #[error("the response signalled more pages but carried no continuation")]
    MissingContinuation,

    #[error("HTTP {status}: {body}")]
    Http { status: u16, body: String },

    #[error("transport: {0}")]
    Transport(String),

    #[error("decode: {0}")]
    Decode(String),

    /// `next()` after an earlier error. The pager sends nothing more; an
    /// earlier failure never turns into an `Ok(None)` that reads as the end.
    #[error("the pager already failed: {previous}")]
    Failed { previous: String },

    /// Session close failed; carried separately so a caller can tell a
    /// leaked lease from a failed page.
    #[error("session close failed: {0}")]
    Close(Box<PagerError>),
}
