//! The macro that gives a generated API its one user-facing client type.

/// Gives a generated API's client struct every client-wide setting and
/// send of [`Client`](crate::Client) as inherent methods.
///
/// `api_client!(Name, Tag)` expects `Name` to be a tuple struct wrapping
/// `schematic_runtime::Client<Tag>`, where `Tag` implements
/// [`Api`](crate::Api) and is each of the API's request types'
/// [`EndpointSpec::Api`](crate::EndpointSpec::Api). The struct is the only
/// name a caller writes for the API, and it accepts only that API's
/// requests. Every method is a one-line forward to the wrapped `Client`, so
/// the setter surface is written once, here, and no trait import is needed
/// to call any of them.
///
/// The invoking module adds what is specific to the API, such as its
/// constants, `definition()`, and a `new()`.
///
/// ## Examples
///
/// ```
/// #[derive(Clone, Debug)]
/// pub struct Example(schematic_runtime::Client<ExampleApi>);
///
/// pub enum ExampleApi {}
///
/// impl schematic_runtime::Api for ExampleApi {
///     const NAME: &'static str = "Example";
/// }
///
/// schematic_runtime::api_client!(Example, ExampleApi);
///
/// impl Example {
///     pub fn new() -> Self {
///         Self(schematic_runtime::Client::new(schematic_runtime::ApiDefinition::new(
///             "https://api.example.com",
///         )))
///     }
/// }
///
/// let client = Example::new().timeout(std::time::Duration::from_secs(5)).api_key("key");
/// assert!(client.build().is_err(), "the Example API accepts no key");
/// ```
#[macro_export]
macro_rules! api_client {
    ($name:ident, $tag:ty $(,)?) => {
        impl $name {
            /// See [`Client::base_url`](schematic_runtime::Client::base_url).
            #[must_use]
            pub fn base_url(self, url: impl Into<String>) -> Self {
                Self(self.0.base_url(url))
            }

            /// See [`Client::timeout`](schematic_runtime::Client::timeout).
            #[must_use]
            pub fn timeout(self, timeout: ::core::time::Duration) -> Self {
                Self(self.0.timeout(timeout))
            }

            /// See [`Client::connect_timeout`](schematic_runtime::Client::connect_timeout).
            #[must_use]
            pub fn connect_timeout(self, timeout: ::core::time::Duration) -> Self {
                Self(self.0.connect_timeout(timeout))
            }

            /// See [`Client::proxy`](schematic_runtime::Client::proxy).
            #[must_use]
            pub fn proxy(self, url: impl Into<String>) -> Self {
                Self(self.0.proxy(url))
            }

            /// See [`Client::add_root_certificate`](schematic_runtime::Client::add_root_certificate).
            #[must_use]
            pub fn add_root_certificate(self, certificate: impl Into<Vec<u8>>) -> Self {
                Self(self.0.add_root_certificate(certificate))
            }

            /// See [`Client::danger_accept_invalid_certificates`](schematic_runtime::Client::danger_accept_invalid_certificates).
            #[must_use]
            pub fn danger_accept_invalid_certificates(self) -> Self {
                Self(self.0.danger_accept_invalid_certificates())
            }

            /// See [`Client::user_agent`](schematic_runtime::Client::user_agent).
            #[must_use]
            pub fn user_agent(self, value: impl Into<String>) -> Self {
                Self(self.0.user_agent(value))
            }

            /// See [`Client::default_header`](schematic_runtime::Client::default_header).
            #[must_use]
            pub fn default_header(self, name: impl Into<String>, value: impl Into<String>) -> Self {
                Self(self.0.default_header(name, value))
            }

            /// See [`Client::transport`](schematic_runtime::Client::transport).
            #[must_use]
            pub fn transport(self, transport: &$crate::SharedTransport) -> Self {
                Self(self.0.transport(transport))
            }

            /// See [`Client::credentials`](schematic_runtime::Client::credentials).
            #[must_use]
            pub fn credentials(self, credential: $crate::Credential) -> Self {
                Self(self.0.credentials(credential))
            }

            /// See [`Client::api_key`](schematic_runtime::Client::api_key).
            #[must_use]
            pub fn api_key(self, key: impl Into<String>) -> Self {
                Self(self.0.api_key(key))
            }

            /// See [`Client::env_order`](schematic_runtime::Client::env_order).
            #[must_use]
            pub fn env_order<I, S>(self, vars: I) -> Self
            where
                I: IntoIterator<Item = S>,
                S: Into<String>,
            {
                Self(self.0.env_order(vars))
            }

            /// See [`Client::cross_origin`](schematic_runtime::Client::cross_origin).
            #[must_use]
            pub fn cross_origin(self, policy: $crate::OriginPolicy) -> Self {
                Self(self.0.cross_origin(policy))
            }

            /// See [`Client::pre_response_json`](schematic_runtime::Client::pre_response_json).
            #[must_use]
            pub fn pre_response_json<F>(self, hook: F) -> Self
            where
                F: Fn(&$crate::ResponseMeta, $crate::serde_json::Value) -> Result<$crate::serde_json::Value, $crate::HookError>
                    + Send
                    + Sync
                    + 'static,
            {
                Self(self.0.pre_response_json(hook))
            }

            /// See [`Client::mutate_response`](schematic_runtime::Client::mutate_response).
            #[must_use]
            pub fn mutate_response<R, F>(self, hook: F) -> Self
            where
                R: $crate::EndpointSpec<Api = $tag>,
                R::Decoder: $crate::SupportsHooks,
                F: Fn(&$crate::ResponseMeta, &mut R::Success) -> Result<(), $crate::HookError> + Send + Sync + 'static,
            {
                Self(self.0.mutate_response::<R, F>(hook))
            }

            /// See [`Client::build`](schematic_runtime::Client::build).
            ///
            /// ## Errors
            ///
            /// Returns the [`BuildError`](schematic_runtime::BuildError) of
            /// the first invalid setting.
            pub fn build(self) -> Result<Self, $crate::BuildError> {
                self.0.build().map(Self)
            }

            /// See [`Client::send`](schematic_runtime::Client::send).
            pub fn send<R>(
                &self,
                request: R,
                options: &$crate::CallOptions,
            ) -> impl ::core::future::Future<
                Output = Result<$crate::Response<R::Success, R::Failure>, $crate::TransportError>,
            >
            where
                R: $crate::EndpointSpec<Api = $tag>,
            {
                self.0.send(request, options)
            }

            /// See [`Client::send_raw`](schematic_runtime::Client::send_raw).
            pub fn send_raw<R>(
                &self,
                request: R,
                options: &$crate::CallOptions,
            ) -> impl ::core::future::Future<Output = Result<$crate::RawResponse, $crate::TransportError>>
            where
                R: $crate::EndpointSpec<Api = $tag>,
            {
                self.0.send_raw(request, options)
            }

            /// See [`Client::check_link`](schematic_runtime::Client::check_link).
            ///
            /// ## Errors
            ///
            /// Returns the [`TransportError`](schematic_runtime::TransportError)
            /// that [`Client::check_link`](schematic_runtime::Client::check_link) does.
            pub fn check_link<R>(&self, url: &$crate::Url) -> Result<(), $crate::TransportError>
            where
                R: $crate::EndpointSpec<Api = $tag>,
            {
                self.0.check_link::<R>(url)
            }
        }
    };
}
