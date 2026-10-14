//! Pages, pager errors, and the incomplete result of `collect_all`.
//!
//! ## What is safe to log
//!
//! The `Display` text of every type here names the endpoint, the page, the
//! response status, and the failure, and never a cursor value, a session
//! handle, a credential, or a followed link: a request URL can carry a cursor
//! in its query, so it is not printed either. The retained
//! [`ResponseMeta`] is redacted as the runtime redacts every send, and raw
//! response bodies are kept as evidence with the envelope's documented
//! limits (see [`crate::response`]).

use std::fmt;

use crate::error::{TransportError, TransportErrorKind};
use crate::meta::ResponseMeta;
use crate::response::{ApiError, ErrorKind};

/// One page of a list endpoint.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Page<T> {
    /// The page's items, in document order, after any inclusive-boundary
    /// duplicates were dropped.
    pub items: Vec<T>,
    /// The total the page reported, in the unit the definition declares,
    /// when the strategy authors one.
    pub total: Option<u64>,
    /// The page's position in this traversal, from 1. The same number is in
    /// the request metadata's `page`.
    pub number: u32,
    /// The response that carried the page.
    pub meta: ResponseMeta,
}

/// Which continuation value was unusable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContinuationProblem {
    /// A cursor part is not a usable value.
    Cursor,
    /// A next or previous link is not a valid URL, or a `Link` header does
    /// not parse.
    Link,
    /// The response names more than one distinct link of the wanted
    /// relation.
    AmbiguousLink,
    /// The authored "more" signal has the wrong type.
    MoreSignal,
    /// The total is negative, not an integer, or not a number.
    Total,
}

impl fmt::Display for ContinuationProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Cursor => "a cursor value is not usable",
            Self::Link => "the page link is not a valid URL",
            Self::AmbiguousLink => "the response names more than one distinct page link",
            Self::MoreSignal => "the end-of-list signal has the wrong type",
            Self::Total => "the total is not a non-negative integer",
        })
    }
}

/// Why a pager stopped with an error.
///
/// Every variant that saw a response keeps its [`ResponseMeta`]; the
/// transport variants keep the [`TransportError`] with its request metadata.
/// A pager that has returned an error returns
/// [`AlreadyFailed`](Self::AlreadyFailed) from then on, without a request.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PagerError<E> {
    /// No response head arrived: a refused argument (such as a page size
    /// outside the declared bounds), a refused cross-origin link, or a
    /// connection failure.
    Transport(TransportError),
    /// A response arrived and is an error status, or its body was cut off,
    /// is not JSON, or does not hold the authored collection of decodable
    /// items. No item of that page is delivered.
    Api {
        error: ApiError<E>,
        /// The response that carried it.
        meta: Box<ResponseMeta>,
    },
    /// The page said more follows, but the continuation is missing, or only
    /// some parts of a composite cursor are present.
    MissingContinuation { meta: Box<ResponseMeta> },
    /// A continuation, link, end signal, or total is present but unusable.
    InvalidContinuation {
        problem: ContinuationProblem,
        meta: Box<ResponseMeta>,
    },
    /// A continuation this traversal already sent in the same direction
    /// came back: the provider is looping.
    RepeatedContinuation { meta: Box<ResponseMeta> },
    /// Every record of a page was already delivered at an inclusive cursor
    /// boundary.
    BoundaryNoProgress { meta: Box<ResponseMeta> },
    /// A page-number or offset page was empty while its end signal said more
    /// follows.
    EmptyPageClaimsMore { meta: Box<ResponseMeta> },
    /// The shared deadline expired before the next page's response head.
    DeadlineExceeded {
        /// Pages received before the deadline expired.
        completed_pages: u32,
        /// The transport error, whose kind reports the deadline's elapsed
        /// time.
        error: TransportError,
    },
    /// The pager already returned an error of this kind; no request was
    /// sent.
    AlreadyFailed { first: PagerErrorKind },
}

/// The variant of a [`PagerError`], without its evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PagerErrorKind {
    Transport,
    Api,
    MissingContinuation,
    InvalidContinuation,
    RepeatedContinuation,
    BoundaryNoProgress,
    EmptyPageClaimsMore,
    DeadlineExceeded,
}

impl<E> PagerError<E> {
    /// The variant, without its evidence.
    pub fn kind(&self) -> PagerErrorKind {
        match self {
            Self::Transport(_) => PagerErrorKind::Transport,
            Self::Api { .. } => PagerErrorKind::Api,
            Self::MissingContinuation { .. } => PagerErrorKind::MissingContinuation,
            Self::InvalidContinuation { .. } => PagerErrorKind::InvalidContinuation,
            Self::RepeatedContinuation { .. } => PagerErrorKind::RepeatedContinuation,
            Self::BoundaryNoProgress { .. } => PagerErrorKind::BoundaryNoProgress,
            Self::EmptyPageClaimsMore { .. } => PagerErrorKind::EmptyPageClaimsMore,
            Self::DeadlineExceeded { .. } => PagerErrorKind::DeadlineExceeded,
            Self::AlreadyFailed { first } => *first,
        }
    }

    /// The response the error is about, when one arrived.
    pub fn response_meta(&self) -> Option<&ResponseMeta> {
        match self {
            Self::Api { meta, .. }
            | Self::MissingContinuation { meta }
            | Self::InvalidContinuation { meta, .. }
            | Self::RepeatedContinuation { meta }
            | Self::BoundaryNoProgress { meta }
            | Self::EmptyPageClaimsMore { meta } => Some(meta),
            Self::Transport(_) | Self::DeadlineExceeded { .. } | Self::AlreadyFailed { .. } => None,
        }
    }

    /// The reason `collect_all` reports for this error.
    pub fn incomplete_reason(&self) -> IncompleteReason {
        match self {
            Self::Transport(error) => match error.kind {
                TransportErrorKind::CrossOriginRejected { .. } => IncompleteReason::CrossOriginRejected,
                TransportErrorKind::InvalidArgument(_) => IncompleteReason::InvalidArgument,
                TransportErrorKind::DeadlineExceeded { .. } => IncompleteReason::DeadlineExceeded,
                _ => IncompleteReason::TransportError,
            },
            // A redirect hop that failed before a response is a transport
            // failure that kept its earlier 3xx as evidence.
            Self::Api {
                error:
                    ApiError {
                        kind: ErrorKind::RedirectFailed { .. },
                        ..
                    },
                ..
            } => IncompleteReason::TransportError,
            Self::Api { .. } => IncompleteReason::HttpError,
            Self::MissingContinuation { .. } | Self::InvalidContinuation { .. } => {
                IncompleteReason::InvalidContinuation
            }
            Self::RepeatedContinuation { .. } | Self::BoundaryNoProgress { .. } | Self::EmptyPageClaimsMore { .. } => {
                IncompleteReason::RepeatedContinuation
            }
            Self::DeadlineExceeded { .. } => IncompleteReason::DeadlineExceeded,
            Self::AlreadyFailed { first } => match first {
                PagerErrorKind::Transport => IncompleteReason::TransportError,
                PagerErrorKind::Api => IncompleteReason::HttpError,
                PagerErrorKind::MissingContinuation | PagerErrorKind::InvalidContinuation => {
                    IncompleteReason::InvalidContinuation
                }
                PagerErrorKind::RepeatedContinuation
                | PagerErrorKind::BoundaryNoProgress
                | PagerErrorKind::EmptyPageClaimsMore => IncompleteReason::RepeatedContinuation,
                PagerErrorKind::DeadlineExceeded => IncompleteReason::DeadlineExceeded,
            },
        }
    }
}

/// `endpoint page N` for a response, without its URL.
fn about(meta: &ResponseMeta) -> String {
    match meta.request.page {
        Some(page) => format!("{} page {page} ({})", meta.request.endpoint_id, meta.status),
        None => format!("{} ({})", meta.request.endpoint_id, meta.status),
    }
}

impl<E> fmt::Display for PagerError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => match error.request.page {
                Some(page) => write!(f, "{} page {page} failed before a response arrived", error.request.endpoint_id),
                None => write!(f, "{} page failed before a response arrived", error.request.endpoint_id),
            },
            Self::Api { meta, .. } => write!(f, "{} failed", about(meta)),
            Self::MissingContinuation { meta } => {
                write!(f, "{} says more pages follow but has no continuation", about(meta))
            }
            Self::InvalidContinuation { problem, meta } => write!(f, "{}: {problem}", about(meta)),
            Self::RepeatedContinuation { meta } => {
                write!(f, "{} repeats a continuation already followed", about(meta))
            }
            Self::BoundaryNoProgress { meta } => {
                write!(f, "{} delivered no record past the cursor boundary", about(meta))
            }
            Self::EmptyPageClaimsMore { meta } => write!(f, "{} is empty but says more pages follow", about(meta)),
            Self::DeadlineExceeded {
                completed_pages,
                error,
            } => write!(
                f,
                "{} deadline exceeded after {completed_pages} page(s)",
                error.request.endpoint_id
            ),
            Self::AlreadyFailed { first } => write!(f, "the pager already failed ({first:?})"),
        }
    }
}

impl<E: fmt::Debug + 'static> std::error::Error for PagerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(error) | Self::DeadlineExceeded { error, .. } => Some(error),
            Self::Api { error, .. } => Some(error),
            _ => None,
        }
    }
}

/// Why `collect_all` returned no complete list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IncompleteReason {
    /// The page limit was reached before the provider signaled the end.
    PageLimitReached,
    /// A continuation repeated, or a page made no progress.
    RepeatedContinuation,
    /// A page's response is an error status, was cut off, was refused at a
    /// redirect, or is malformed.
    HttpError,
    /// The shared deadline expired.
    DeadlineExceeded,
    /// A followed link's origin was refused.
    CrossOriginRejected,
    /// A continuation was missing where one was required, or unusable.
    InvalidContinuation,
    /// An argument (page size, start position, page limit) was refused
    /// before sending.
    InvalidArgument,
    /// Any other failure before a response arrived, including a redirect
    /// hop that failed.
    TransportError,
}

impl fmt::Display for IncompleteReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::PageLimitReached => "page limit reached",
            Self::RepeatedContinuation => "no progress",
            Self::HttpError => "a page response failed",
            Self::DeadlineExceeded => "deadline exceeded",
            Self::CrossOriginRejected => "a page link's origin was refused",
            Self::InvalidContinuation => "missing or invalid continuation",
            Self::InvalidArgument => "invalid argument",
            Self::TransportError => "a page request failed before a response",
        })
    }
}

/// `collect_all` stopped before the provider signaled the end of the list.
///
/// `items` holds every item of every page that decoded, including a page
/// whose continuation then proved unusable; a failed or malformed page
/// contributes none.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Incomplete<T, E> {
    pub reason: IncompleteReason,
    /// The items gathered before the stop.
    pub items: Vec<T>,
    /// Page requests attempted, including the one that failed.
    pub pages: u32,
    /// The error that stopped collection; `None` for
    /// [`IncompleteReason::PageLimitReached`].
    pub error: Option<PagerError<E>>,
}

impl<T, E> fmt::Display for Incomplete<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "list incomplete after {} page request(s) and {} item(s): {}",
            self.pages,
            self.items.len(),
            self.reason
        )
    }
}

impl<T: fmt::Debug, E: fmt::Debug + 'static> std::error::Error for Incomplete<T, E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.error.as_ref().map(|error| error as &(dyn std::error::Error + 'static))
    }
}
