//! API-level cross-origin policy for redirects and followed next-page links.

/// Which origins a client may follow a redirect or next-page link to, and
/// whether credentials go with it.
///
/// An origin is the scheme, host, and effective port of a URL; the base
/// origin is the one in [`RestApi::base_url`](crate::RestApi::base_url).
///
/// - `SameOrigin` (the default) follows only to the base origin.
/// - `AllowOrigins` also follows to the listed origins, exactly as written,
///   and sends credentials to them. There is no wildcard or domain matching.
/// - `AllowWithoutCredentials` follows anywhere and strips credentials when
///   the target origin differs from the base.
/// - `DangerAllowAny` follows anywhere with credentials; the `danger_` name is
///   the explicit opt-in.
/// - `Deny` follows no redirect and no cross-origin link. A same-origin
///   next-page link still passes.
///
/// A caller may only tighten the definition's policy: to `Deny`,
/// `SameOrigin`, or a subset of an `AllowOrigins` list. Over a `Deny`
/// definition only `Deny` is accepted.
///
/// The runtime enforces the policy; the generator translates it into runtime
/// configuration.
///
/// ## Examples
///
/// ```
/// use schematic_define::CrossOrigin;
///
/// let policy = CrossOrigin::allow_origins(["https://uploads.example.com"]);
/// assert_eq!(policy.name(), "allow_origins");
/// assert_eq!(CrossOrigin::default(), CrossOrigin::same_origin());
/// ```
// Deliberately not `#[non_exhaustive]`, unlike this crate's other public
// enums: the generator translates every policy into runtime configuration, and
// an exhaustive match makes a new variant a compile error there instead of a
// wildcard arm that could emit a looser policy.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CrossOrigin {
    /// Follow only to the base origin.
    #[default]
    SameOrigin,
    /// Follow to the base origin and these exact origins, with credentials.
    AllowOrigins(Vec<String>),
    /// Follow anywhere; strip credentials when the origin differs from the base.
    AllowWithoutCredentials,
    /// Follow anywhere with credentials.
    DangerAllowAny,
    /// Follow no redirect and no cross-origin next-page link.
    Deny,
}

#[allow(missing_docs)]
impl CrossOrigin {
    #[must_use]
    pub fn same_origin() -> Self {
        Self::SameOrigin
    }

    #[must_use]
    pub fn allow_origins<I, S>(origins: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::AllowOrigins(origins.into_iter().map(Into::into).collect())
    }

    #[must_use]
    pub fn allow_without_credentials() -> Self {
        Self::AllowWithoutCredentials
    }

    #[must_use]
    pub fn danger_allow_any() -> Self {
        Self::DangerAllowAny
    }

    #[must_use]
    pub fn deny() -> Self {
        Self::Deny
    }

    /// The policy's snake_case name, matching its constructor.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::SameOrigin => "same_origin",
            Self::AllowOrigins(_) => "allow_origins",
            Self::AllowWithoutCredentials => "allow_without_credentials",
            Self::DangerAllowAny => "danger_allow_any",
            Self::Deny => "deny",
        }
    }
}
