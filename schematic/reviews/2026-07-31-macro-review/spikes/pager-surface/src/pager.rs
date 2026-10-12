//! Encoding A (chosen): one generic `Pager<E>` (configuration) and one
//! generic `Pages<E>` (iteration) for every endpoint and every strategy kind.
//! Each knob is an inherent method with its own `where E: <Capability>`
//! bound, so an unauthored knob does not compile, and the error is E0277
//! carrying the capability trait's `on_unimplemented` text. Each method body
//! is a short forward to the non-generic engine.

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::time::Duration;

use futures::{FutureExt, Stream};

use crate::client::Client;
use crate::engine::{Engine, RawPage};
use crate::error::{PageSizeError, PagerError};
use crate::page::Page;
use crate::spec::{CursorLeg, HasPrev, KeepAliveKnob, Kind, PageNumbered, PageSizeKnob, Paginated, ResumeAfter, Sessioned};

/// Configuration before the first request. Knob methods exist only for
/// knobs the definition authored.
#[must_use = "a pager does nothing until `.pages()` is iterated"]
pub struct Pager<E> {
    engine: Engine,
    _endpoint: PhantomData<fn() -> E>,
}

/// An iteration in progress.
#[must_use = "dropping `Pages` for a sessioned endpoint leaks the server session until its lease expires; call `close()`"]
pub struct Pages<E> {
    engine: Engine,
    _endpoint: PhantomData<fn() -> E>,
}

fn decode<T: serde::de::DeserializeOwned>(raw: RawPage) -> Result<Page<T>, PagerError> {
    let items = raw
        .items
        .into_iter()
        .map(serde_json::from_value)
        .collect::<Result<_, _>>()
        .map_err(|e| PagerError::Decode(e.to_string()))?;
    Ok(Page {
        items,
        headers: raw.headers,
        total: raw.total,
    })
}

impl<E> Pager<E> {
    pub fn new(client: &Client) -> Self
    where
        E: Paginated,
    {
        Self {
            engine: Engine::new(client.clone(), E::SPEC, <E::Kind as Kind>::start(E::STRATEGY)),
            _endpoint: PhantomData,
        }
    }
}

impl<E: Paginated> Pager<E> {
    /// Stand-in for the typed request's ordinary parameters.
    pub fn query(mut self, name: &str, value: &str) -> Self {
        self.engine.query.push((name.to_owned(), value.to_owned()));
        self
    }

    /// D13: one budget for every request this pager sends.
    pub fn deadline(mut self, budget: Duration) -> Self {
        self.engine.deadline = Some(budget);
        self
    }

    pub fn pages(self) -> Pages<E> {
        Pages {
            engine: self.engine,
            _endpoint: PhantomData,
        }
    }

    /// Out-of-range sizes are refused here, before any request exists.
    pub fn page_size(mut self, n: u32) -> Result<Self, PageSizeError>
    where
        E: PageSizeKnob,
    {
        let spec = E::PAGE_SIZE;
        if n < spec.min || spec.max.is_some_and(|max| n > max) {
            return Err(PageSizeError {
                requested: n,
                min: spec.min,
                max: spec.max,
            });
        }
        self.engine.size = Some((spec.param, n));
        Ok(self)
    }

    /// Start at page `n`, in the strategy's own index base.
    pub fn page(mut self, n: u64) -> Self
    where
        E: PageNumbered,
    {
        self.engine.start_page(n);
        self
    }

    pub fn after(mut self, cursor: impl Into<String>) -> Self
    where
        E: ResumeAfter,
    {
        self.engine.start_cursor(cursor.into(), None);
        self
    }

    pub fn before(mut self, cursor: impl Into<String>) -> Self
    where
        E: ResumeAfter + HasPrev,
    {
        let leg: &'static CursorLeg = E::PREV;
        self.engine.start_cursor(cursor.into(), Some(leg));
        self
    }

    pub fn keep_alive(mut self, duration: &str) -> Self
    where
        E: KeepAliveKnob,
    {
        self.engine.set_keep_alive(duration.to_owned());
        self
    }

    /// Q4 option (c): the session is closed when `body` returns, returns an
    /// error, or panics. It is NOT closed if this future is dropped
    /// (cancelled) mid-flight: Rust has no async drop.
    pub async fn with_session<R>(self, body: impl AsyncFnOnce(&mut Pages<E>) -> Result<R, PagerError>) -> Result<R, PagerError>
    where
        E: Sessioned,
    {
        let mut pages = self.pages();
        let outcome = AssertUnwindSafe(body(&mut pages)).catch_unwind().await;
        let closed = pages.close().await;
        match outcome {
            Ok(Ok(value)) => closed.map(|()| value),
            Ok(Err(e)) => Err(e),
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
}

impl<E: Paginated> Pages<E> {
    pub async fn next(&mut self) -> Result<Option<Page<E::Item>>, PagerError> {
        self.engine.next_raw().await?.map(decode).transpose()
    }

    /// Flattens pages into items. Ends after the first error.
    pub fn items(self) -> impl Stream<Item = Result<E::Item, PagerError>> {
        futures::stream::unfold((self, VecDeque::new(), false), |(mut pages, mut buffer, failed)| async move {
            loop {
                if let Some(item) = buffer.pop_front() {
                    return Some((Ok(item), (pages, buffer, failed)));
                }
                if failed {
                    return None;
                }
                match pages.next().await {
                    Ok(Some(page)) => buffer.extend(page.items),
                    Ok(None) => return None,
                    Err(e) => return Some((Err(e), (pages, buffer, true))),
                }
            }
        })
    }
}

impl<E: Paginated> Pages<E> {
    pub async fn prev(&mut self) -> Result<Option<Page<E::Item>>, PagerError>
    where
        E: HasPrev,
    {
        self.engine.prev_raw(<E::Kind as Kind>::prev(E::PREV)).await?.map(decode).transpose()
    }

    pub async fn close(&mut self) -> Result<(), PagerError>
    where
        E: Sessioned,
    {
        self.engine.close().await
    }
}
