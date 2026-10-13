//! A call: one request bound to a client, with its per-call settings, sent
//! when it is awaited.
//!
//! [`Client::send`] returns a [`Call`]. Its per-call settings chain on it,
//! and awaiting it returns the endpoint's data or a [`CallError`].
//! [`Call::full`] turns it into a [`FullCall`], which awaits to the whole
//! [`Response`] envelope instead. Generated per-endpoint call types wrap a
//! [`Call`] and forward these methods, so both paths behave identically.

use std::fmt;
use std::future::{Future, IntoFuture};
use std::pin::Pin;

use crate::client::Client;
use crate::credentials::Credential;
use crate::deadline::Deadline;
use crate::endpoint::{Api, EndpointSpec};
use crate::error::{InvalidArgument, TransportError};
use crate::meta::{RequestMeta, ResponseMeta};
use crate::options::{CallAuth, CallOptions};
use crate::response::{ApiError, Outcome, Response};

/// The future a [`Call`] or [`FullCall`] becomes when awaited.
///
/// It is one heap pointer, so joining many calls in one future needs no
/// large stack, and it is `Send`, so a call can be awaited on a
/// multi-threaded runtime.
pub type CallFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A request bound to a client, sent when awaited.
///
/// Awaiting it returns `Ok` with the endpoint's data for a 2xx reply that
/// decoded, and otherwise a [`CallError`] that keeps every piece of evidence
/// the [`Response`] envelope or [`TransportError`] would have carried.
///
/// A call is `Clone` when its request is, so a call can be built once and
/// sent more than once; each await is a separate send.
#[must_use = "a call sends nothing until it is awaited"]
pub struct Call<'a, A: Api, R> {
    client: &'a Client<A>,
    request: R,
    options: CallOptions,
}

/// A call whose await returns the whole [`Response`] envelope: status,
/// headers, request id, and the outcome, for a reply of any status.
///
/// Created by [`Call::full`]. Its per-call settings are the same as
/// [`Call`]'s.
#[must_use = "a call sends nothing until it is awaited"]
pub struct FullCall<'a, A: Api, R> {
    call: Call<'a, A, R>,
}

impl<'a, A: Api, R: EndpointSpec<Api = A>> Call<'a, A, R> {
    pub(crate) fn new(client: &'a Client<A>, request: R) -> Self {
        Self {
            client,
            request,
            options: CallOptions::new(),
        }
    }

    /// Sends no credential from any source, and removes authentication
    /// headers, query values, and cookies from the request. Replaces a
    /// per-call [`credential`](Self::credential); the last one set wins.
    pub fn anonymous(mut self) -> Self {
        self.options.auth = CallAuth::Anonymous;
        self
    }

    /// Sends `credential` for this call, overriding the client's credential
    /// and the environment. Replaces [`anonymous`](Self::anonymous); the last
    /// one set wins.
    pub fn credential(mut self, credential: Credential) -> Self {
        self.options = self.options.credential(credential);
        self
    }

    /// Bounds this call by `deadline`, shared with its other holders.
    pub fn deadline(mut self, deadline: &Deadline) -> Self {
        self.options = self.options.deadline(deadline);
        self
    }

    /// Replaces the request with `f(request)`, keeping the per-call
    /// settings. Generated call types implement their optional-value setters
    /// with it.
    pub fn map_request(mut self, f: impl FnOnce(R) -> R) -> Self {
        self.request = f(self.request);
        self
    }

    /// Makes the await return the whole [`Response`] envelope rather than
    /// only the data. Chain it last: the per-call settings stay available on
    /// the result, but an endpoint's own setters do not.
    pub fn full(self) -> FullCall<'a, A, R> {
        FullCall { call: self }
    }

    /// Runs every check that can fail before network work, and returns the
    /// same call, ready to await.
    ///
    /// The checks run again when the call is awaited; `build` reports their
    /// failure early, without sending.
    ///
    /// ## Errors
    ///
    /// Returns [`InvalidArgument`] when the request cannot be encoded.
    pub fn build(self) -> Result<Self, InvalidArgument>
    where
        R: Clone,
    {
        self.request.clone().into_parts()?;
        Ok(self)
    }
}

impl<'a, A: Api, R: EndpointSpec<Api = A>> FullCall<'a, A, R> {
    /// See [`Call::anonymous`].
    pub fn anonymous(self) -> Self {
        Self {
            call: self.call.anonymous(),
        }
    }

    /// See [`Call::credential`].
    pub fn credential(self, credential: Credential) -> Self {
        Self {
            call: self.call.credential(credential),
        }
    }

    /// See [`Call::deadline`].
    pub fn deadline(self, deadline: &Deadline) -> Self {
        Self {
            call: self.call.deadline(deadline),
        }
    }

    /// See [`Call::build`].
    ///
    /// ## Errors
    ///
    /// Returns [`InvalidArgument`] when the request cannot be encoded.
    pub fn build(self) -> Result<Self, InvalidArgument>
    where
        R: Clone,
    {
        self.call.build().map(|call| Self { call })
    }
}

impl<'a, A: Api, R: EndpointSpec<Api = A>> IntoFuture for Call<'a, A, R> {
    type Output = Result<R::Success, CallError<R::Failure>>;
    type IntoFuture = CallFuture<'a, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let response = self.client.send_unboxed(self.request, &self.options).await?;
            match response.outcome {
                Outcome::Success(value) => Ok(value),
                Outcome::Failure(error) => Err(CallError::Api {
                    error,
                    meta: Box::new(response.meta),
                }),
            }
        })
    }
}

impl<'a, A: Api, R: EndpointSpec<Api = A>> IntoFuture for FullCall<'a, A, R> {
    type Output = Result<Response<R::Success, R::Failure>, TransportError>;
    type IntoFuture = CallFuture<'a, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        let Call {
            client,
            request,
            options,
        } = self.call;
        Box::pin(async move { client.send_unboxed(request, &options).await })
    }
}

impl<A: Api, R: Clone> Clone for Call<'_, A, R> {
    fn clone(&self) -> Self {
        Self {
            client: self.client,
            request: self.request.clone(),
            options: self.options.clone(),
        }
    }
}

impl<A: Api, R: Clone> Clone for FullCall<'_, A, R> {
    fn clone(&self) -> Self {
        Self {
            call: self.call.clone(),
        }
    }
}

impl<A: Api, R: fmt::Debug> fmt::Debug for Call<'_, A, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Call")
            .field("api", &A::NAME)
            .field("request", &self.request)
            .field("options", &self.options)
            .finish()
    }
}

impl<A: Api, R: fmt::Debug> fmt::Debug for FullCall<'_, A, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FullCall").field("call", &self.call).finish()
    }
}

/// Why an awaited [`Call`] returned no data.
///
/// It covers both ways a call can fail: no response head arrived
/// ([`Transport`](Self::Transport)), or a reply arrived that is an error or
/// could not be read ([`Api`](Self::Api)). Each variant keeps the evidence
/// the runtime recorded; [`request_meta`](Self::request_meta) reads the
/// request from either.
///
/// Its `Display` names the endpoint and, for a reply, its status and
/// request, and is free of credentials; [`source`](std::error::Error::source)
/// is the inner [`TransportError`] or [`ApiError`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CallError<E> {
    /// No response head arrived.
    Transport(TransportError),
    /// A response head arrived, and the reply is an error status, or its body
    /// was cut off, could not be decoded, or was refused by a response hook.
    Api {
        /// What went wrong, with the raw body.
        error: ApiError<E>,
        /// The response that carried it. Boxed so that a `Result` carrying
        /// this error stays small.
        meta: Box<ResponseMeta>,
    },
}

impl<E> CallError<E> {
    /// The redacted request: as assembled when no head arrived, or the hop
    /// that received the reply.
    pub fn request_meta(&self) -> &RequestMeta {
        match self {
            Self::Transport(error) => &error.request,
            Self::Api { meta, .. } => &meta.request,
        }
    }
}

impl<E> From<TransportError> for CallError<E> {
    fn from(error: TransportError) -> Self {
        Self::Transport(error)
    }
}

impl<E> fmt::Display for CallError<E> {
    // The inner error is the `source`, so its text is not repeated here.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => {
                write!(f, "{} call failed before a response arrived", error.request.endpoint_id)
            }
            Self::Api { meta, .. } => write!(f, "{} call failed: {meta}", meta.request.endpoint_id),
        }
    }
}

impl<E: fmt::Debug + 'static> std::error::Error for CallError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            Self::Api { error, .. } => Some(error),
        }
    }
}
