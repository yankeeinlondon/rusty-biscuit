//! The two response hooks a client can register, and the marker that limits
//! typed mutators to codecs that run them.
//!
//! - [`ClientBuilder::pre_response_json`](crate::ClientBuilder::pre_response_json)
//!   registers one client-global hook that reshapes a successful JSON body
//!   before it is decoded.
//! - [`ClientBuilder::mutate_response`](crate::ClientBuilder::mutate_response)
//!   registers a mutator for one endpoint, typed by its success type, that
//!   runs on the decoded value.
//!
//! For a 2xx reply to an endpoint whose codec implements [`SupportsHooks`],
//! the order is: parse the body as JSON, run the global hook, decode the
//! endpoint's success type, run the endpoint's mutator. Every other reply
//! skips both hooks: non-2xx statuses (including declared error bodies),
//! text, binary, and empty endpoints, and [raw sends](crate::Client::send_raw).
//!
//! A hook that returns [`HookError`] makes the send's outcome
//! [`ErrorKind::Hook`](crate::ErrorKind::Hook) with the original body bytes;
//! the hook's own error text is never kept.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::codec::Decode;
use crate::endpoint::EndpointSpec;
use crate::meta::ResponseMeta;

/// Marks a codec whose decoding runs the response hooks.
///
/// Only [`Json`](crate::Json) implements it here, so registering a mutator
/// for a text, binary, or empty endpoint does not compile. A codec that
/// implements it must override [`Decode::decode_hooked`] to apply the
/// [`HookStage`] it receives.
pub trait SupportsHooks: Decode {}

/// A hook's refusal of a response.
///
/// It deliberately carries no text: a hook's message could quote the body or
/// a credential, and the envelope's `Display` must stay safe to log. `?` on
/// any [`std::error::Error`] converts into it, discarding that error's text.
/// It does not implement `Error` itself, which is what allows that
/// conversion.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct HookError;

impl HookError {
    /// A refusal.
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Display for HookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("response hook rejected the body")
    }
}

impl<E: std::error::Error> From<E> for HookError {
    fn from(_: E) -> Self {
        Self
    }
}

type PreResponseJsonHook = dyn Fn(&ResponseMeta, serde_json::Value) -> Result<serde_json::Value, HookError> + Send + Sync;

/// A typed mutator with its type erased so mutators of different endpoints
/// share one map.
trait AnyMutator: Send + Sync {
    fn mutate(&self, meta: &ResponseMeta, value: &mut dyn Any) -> Result<(), HookError>;
}

struct TypedMutator<T, F> {
    hook: F,
    value: PhantomData<fn(&mut T)>,
}

impl<T, F> AnyMutator for TypedMutator<T, F>
where
    T: 'static,
    F: Fn(&ResponseMeta, &mut T) -> Result<(), HookError> + Send + Sync,
{
    fn mutate(&self, meta: &ResponseMeta, value: &mut dyn Any) -> Result<(), HookError> {
        // Registration fixes `T` to the endpoint's success type and the key
        // to that endpoint, so the downcast cannot fail; a failure is still
        // reported rather than skipped.
        let value = value.downcast_mut::<T>().ok_or(HookError)?;
        (self.hook)(meta, value)
    }
}

/// The hooks registered on one client.
#[derive(Clone, Default)]
pub(crate) struct Hooks {
    pre_response_json: Option<Arc<PreResponseJsonHook>>,
    /// Keyed by API and endpoint id: endpoint ids are unique only within an
    /// API.
    mutators: HashMap<(TypeId, &'static str), Arc<dyn AnyMutator>>,
}

impl Hooks {
    pub(crate) fn set_pre_response_json<F>(&mut self, hook: F)
    where
        F: Fn(&ResponseMeta, serde_json::Value) -> Result<serde_json::Value, HookError> + Send + Sync + 'static,
    {
        self.pre_response_json = Some(Arc::new(hook));
    }

    pub(crate) fn set_mutator<R, F>(&mut self, hook: F)
    where
        R: EndpointSpec,
        F: Fn(&ResponseMeta, &mut R::Success) -> Result<(), HookError> + Send + Sync + 'static,
    {
        let mutator = TypedMutator::<R::Success, F> {
            hook,
            value: PhantomData,
        };
        self.mutators.insert(key::<R>(), Arc::new(mutator));
    }

    /// The hooks that apply to a reply to `R` described by `meta`.
    pub(crate) fn stage<'a, R: EndpointSpec>(&'a self, meta: &'a ResponseMeta) -> HookStage<'a> {
        HookStage {
            meta,
            pre_response_json: self.pre_response_json.as_deref(),
            mutator: self.mutators.get(&key::<R>()).map(|mutator| &**mutator),
        }
    }
}

fn key<R: EndpointSpec>() -> (TypeId, &'static str) {
    (TypeId::of::<R::Api>(), R::ENDPOINT_ID)
}

/// The hooks that apply to one successful reply, handed to
/// [`Decode::decode_hooked`].
pub struct HookStage<'a> {
    meta: &'a ResponseMeta,
    pre_response_json: Option<&'a PreResponseJsonHook>,
    mutator: Option<&'a dyn AnyMutator>,
}

impl HookStage<'_> {
    /// Whether no hook applies, so a codec can decode directly.
    pub fn is_empty(&self) -> bool {
        self.pre_response_json.is_none() && self.mutator.is_none()
    }

    /// Whether a client-global JSON hook is registered.
    pub fn has_pre_response_json(&self) -> bool {
        self.pre_response_json.is_some()
    }

    /// Runs the client-global JSON hook on `value`, or returns it unchanged
    /// when none is registered.
    ///
    /// ## Errors
    ///
    /// Returns the hook's [`HookError`].
    pub fn pre_response_json(&self, value: serde_json::Value) -> Result<serde_json::Value, HookError> {
        match self.pre_response_json {
            Some(hook) => hook(self.meta, value),
            None => Ok(value),
        }
    }

    /// Runs the endpoint's mutator on the decoded `value`, if one is
    /// registered.
    ///
    /// ## Errors
    ///
    /// Returns the mutator's [`HookError`].
    pub fn mutate<T: 'static>(&self, value: &mut T) -> Result<(), HookError> {
        match self.mutator {
            Some(mutator) => mutator.mutate(self.meta, value),
            None => Ok(()),
        }
    }
}

impl fmt::Debug for HookStage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HookStage")
            .field("pre_response_json", &self.pre_response_json.is_some())
            .field("mutator", &self.mutator.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use http::{HeaderMap, Method, StatusCode};
    use url::Url;

    use super::*;
    use crate::codec::Json;
    use crate::endpoint::{Api, RequestParts};
    use crate::error::InvalidArgument;
    use crate::meta::{AuthMethod, RequestMeta};
    use crate::redact::SecretNames;
    use crate::response::NoDeclaredError;

    struct Alpha;
    struct Beta;

    impl Api for Alpha {
        const NAME: &'static str = "Alpha";
    }

    impl Api for Beta {
        const NAME: &'static str = "Beta";
    }

    /// The endpoint `list`, declared by API `A`.
    struct List<A>(PhantomData<fn() -> A>);

    impl<A: Api> EndpointSpec for List<A> {
        type Api = A;
        type Success = Vec<u32>;
        type Failure = NoDeclaredError;
        type Decoder = Json<Vec<u32>>;

        const ENDPOINT_ID: &'static str = "list";
        const METHOD: Method = Method::GET;

        fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
            Ok(RequestParts::new(Method::GET, "/list"))
        }
    }

    fn meta() -> ResponseMeta {
        let secrets = SecretNames::builtin();
        let url = Url::parse("https://api.example.invalid/list").expect("url");
        let request = RequestMeta::capture(&secrets, "list", Method::GET, &url, &HeaderMap::new(), AuthMethod::Anonymous);
        ResponseMeta::capture(&secrets, request, StatusCode::OK, &HeaderMap::new(), None, std::time::Duration::ZERO)
    }

    #[test]
    fn a_mutator_is_found_by_api_and_endpoint_not_by_endpoint_id_alone() {
        let mut hooks = Hooks::default();
        hooks.set_mutator::<List<Alpha>, _>(|_, values| {
            values.push(1);
            Ok(())
        });
        let meta = meta();

        let mut values: Vec<u32> = Vec::new();
        let alpha = hooks.stage::<List<Alpha>>(&meta);
        assert!(!alpha.is_empty());
        alpha.mutate(&mut values).expect("mutates");
        assert_eq!(values, [1]);

        let beta = hooks.stage::<List<Beta>>(&meta);
        assert!(beta.is_empty(), "Beta's `list` must not find Alpha's mutator");
        beta.mutate(&mut values).expect("nothing to run");
        assert_eq!(values, [1]);
    }

    #[test]
    fn a_mutator_given_a_value_of_another_type_refuses_rather_than_skipping() {
        let mut hooks = Hooks::default();
        hooks.set_mutator::<List<Alpha>, _>(|_, _| Ok(()));
        let meta = meta();
        let mut wrong = String::new();
        assert_eq!(hooks.stage::<List<Alpha>>(&meta).mutate(&mut wrong), Err(HookError));
    }
}
