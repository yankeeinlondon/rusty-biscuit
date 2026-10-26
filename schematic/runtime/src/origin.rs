//! The cross-origin policy that governs which redirects a client follows and
//! which absolute URLs (such as next-page links) it may request, and whether
//! the credential travels with them.
//!
//! An origin is exact: scheme, host, and effective port. `https://a.test` and
//! `https://a.test:443` are one origin; `http://a.test`, `https://a.test:8443`,
//! and `https://api.a.test` are each a different one. There is no domain
//! matching.
//!
//! | Policy | Redirect to base | Redirect elsewhere | Link to base | Link elsewhere |
//! | --- | --- | --- | --- | --- |
//! | `same_origin` | with credentials | refused | allowed | refused |
//! | `allow_origins` | with credentials | with credentials if listed, else refused | allowed | with credentials if listed, else refused |
//! | `allow_without_credentials` | with credentials | without credentials | allowed | without credentials |
//! | `danger_allow_any` | with credentials | with credentials | allowed | with credentials |
//! | `deny` | refused | refused | allowed | refused |
//!
//! Whether the credential goes with a request is decided by comparing its
//! target with the client's *base* origin, never with the previous hop: under
//! `allow_without_credentials`, a chain A → B → B stays anonymous after the
//! first hop, and B → A carries the credential again.
//!
//! `deny` follows no redirect at all, not even one to the base origin, but a
//! same-origin link is still allowed so that paging keeps working.
//!
//! A target whose scheme is not `http` or `https`, or that carries user
//! information, is refused under every policy.
//!
//! An endpoint's authored policy replaces the API default. A caller may only
//! tighten each effective policy: `deny` over anything,
//! `same_origin` over anything but `deny`, and an `allow_origins` list that is
//! a subset of the API's own `allow_origins` list. Any other request is
//! [`BuildError::OriginPolicyNotPermitted`].

use url::{Origin, Url};

use crate::error::BuildError;

/// Runtime form of an API's or endpoint's cross-origin policy.
///
/// Generated code translates the definition's policy into this value; a
/// caller may pass a tighter one to the client builder.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum OriginPolicy {
    /// Only the base origin.
    #[default]
    SameOrigin,
    /// The base origin and the listed origins, all with credentials. Each
    /// entry must be a bare HTTP(S) origin such as `https://b.test`.
    AllowOrigins(Vec<String>),
    /// Any origin, but the credential only goes to the base origin.
    AllowWithoutCredentials,
    /// Any origin, with credentials.
    DangerAllowAny,
    /// No redirects; links only to the base origin.
    Deny,
}

impl OriginPolicy {
    /// The `same_origin` policy.
    pub fn same_origin() -> Self {
        Self::SameOrigin
    }

    /// The `allow_origins` policy over `origins`. Entries are validated when
    /// the client is built.
    pub fn allow_origins<I, S>(origins: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::AllowOrigins(origins.into_iter().map(Into::into).collect())
    }

    /// The `allow_without_credentials` policy.
    pub fn allow_without_credentials() -> Self {
        Self::AllowWithoutCredentials
    }

    /// The `danger_allow_any` policy.
    pub fn danger_allow_any() -> Self {
        Self::DangerAllowAny
    }

    /// The `deny` policy.
    pub fn deny() -> Self {
        Self::Deny
    }

    /// The policy's name as written in an API definition, used in error
    /// text.
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

/// Whether a request to a target may start, and whether the credential goes
/// with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    Refuse,
    Follow { credentials: bool },
}

/// A validated policy bound to the client's base origin.
#[derive(Debug, Clone)]
pub(crate) struct OriginRules {
    base: Origin,
    kind: Kind,
}

#[derive(Debug, Clone)]
enum Kind {
    SameOrigin,
    AllowOrigins(Vec<Origin>),
    AllowWithoutCredentials,
    DangerAllowAny,
    Deny,
}

impl OriginRules {
    /// Validates `definition`, and `requested` (the caller's tighten-only
    /// override) if given, for a client whose base URL is `base`. The
    /// effective policy is `requested` when given, else `definition`.
    ///
    /// ## Errors
    ///
    /// [`BuildError::InvalidAllowedOrigin`] when either policy lists an entry
    /// that is not a bare HTTP(S) origin; this is checked first.
    /// [`BuildError::OriginPolicyNotPermitted`] when `requested` is not a
    /// tightening of `definition`.
    pub(crate) fn new(base: &Url, definition: &OriginPolicy, requested: Option<&OriginPolicy>) -> Result<Self, BuildError> {
        let definition_kind = Kind::parse(definition)?;
        let kind = match requested {
            None => definition_kind,
            Some(requested) => {
                let requested_kind = Kind::parse(requested)?;
                if !requested_kind.tightens(&definition_kind) {
                    return Err(BuildError::OriginPolicyNotPermitted {
                        definition: definition.name(),
                        requested: requested.name(),
                    });
                }
                requested_kind
            }
        };
        Ok(Self {
            base: base.origin(),
            kind,
        })
    }

    /// Whether a redirect to `target` may be followed, and whether the
    /// credential goes with it.
    pub(crate) fn redirect(&self, target: &Url) -> Decision {
        if matches!(self.kind, Kind::Deny) {
            return Decision::Refuse;
        }
        self.decide(target)
    }

    /// Whether a request to `target` (a followed next-page link or an
    /// absolute request URL) may start, and whether the credential goes with
    /// it. A same-origin target is allowed under every policy.
    pub(crate) fn link(&self, target: &Url) -> Decision {
        self.decide(target)
    }

    fn decide(&self, target: &Url) -> Decision {
        if !matches!(target.scheme(), "http" | "https") || !target.username().is_empty() || target.password().is_some()
        {
            return Decision::Refuse;
        }
        let origin = target.origin();
        if origin == self.base {
            return Decision::Follow { credentials: true };
        }
        match &self.kind {
            Kind::SameOrigin | Kind::Deny => Decision::Refuse,
            Kind::AllowOrigins(allowed) if allowed.contains(&origin) => Decision::Follow { credentials: true },
            Kind::AllowOrigins(_) => Decision::Refuse,
            Kind::AllowWithoutCredentials => Decision::Follow { credentials: false },
            Kind::DangerAllowAny => Decision::Follow { credentials: true },
        }
    }
}

impl Kind {
    fn parse(policy: &OriginPolicy) -> Result<Self, BuildError> {
        Ok(match policy {
            OriginPolicy::SameOrigin => Self::SameOrigin,
            OriginPolicy::AllowOrigins(entries) => Self::AllowOrigins(
                entries
                    .iter()
                    .map(|entry| parse_allowed_origin(entry))
                    .collect::<Result<_, _>>()?,
            ),
            OriginPolicy::AllowWithoutCredentials => Self::AllowWithoutCredentials,
            OriginPolicy::DangerAllowAny => Self::DangerAllowAny,
            OriginPolicy::Deny => Self::Deny,
        })
    }

    /// Whether `self`, requested by a caller, is permitted over `definition`.
    fn tightens(&self, definition: &Self) -> bool {
        match (definition, self) {
            (_, Self::Deny) => true,
            (Self::Deny, _) => false,
            (_, Self::SameOrigin) => true,
            (Self::AllowOrigins(listed), Self::AllowOrigins(requested)) => {
                requested.iter().all(|origin| listed.contains(origin))
            }
            _ => false,
        }
    }
}

fn parse_allowed_origin(text: &str) -> Result<Origin, BuildError> {
    let invalid = |reason| BuildError::InvalidAllowedOrigin { reason };
    let url = Url::parse(text).map_err(|_| invalid("not an absolute URL"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(invalid("scheme is not http or https"));
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(invalid("no host"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid("carries user information"));
    }
    if !matches!(url.path(), "" | "/") {
        return Err(invalid("has a path"));
    }
    if url.query().is_some() {
        return Err(invalid("has a query"));
    }
    if url.fragment().is_some() {
        return Err(invalid("has a fragment"));
    }
    Ok(url.origin())
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "https://api.a.test";
    const B: &str = "https://b.test";
    const C: &str = "https://c.test";
    const D: &str = "https://d.test";

    const REFUSE: Decision = Decision::Refuse;
    const CREDS: Decision = Decision::Follow { credentials: true };
    const ANON: Decision = Decision::Follow { credentials: false };

    fn url(text: &str) -> Url {
        Url::parse(text).expect("test URL parses")
    }

    fn rules(base: &str, policy: &OriginPolicy) -> OriginRules {
        OriginRules::new(&url(base), policy, None).expect("policy is valid")
    }

    fn policies() -> [OriginPolicy; 5] {
        [
            OriginPolicy::same_origin(),
            OriginPolicy::allow_origins([B]),
            OriginPolicy::allow_without_credentials(),
            OriginPolicy::danger_allow_any(),
            OriginPolicy::deny(),
        ]
    }

    /// Per target: expected decisions under, in order, same_origin,
    /// allow_origins([B]), allow_without_credentials, danger_allow_any, deny;
    /// first for `redirect`, then for `link`.
    const R21: &[(&str, [Decision; 5], [Decision; 5])] = &[
        ("https://api.a.test/next", [CREDS, CREDS, CREDS, CREDS, REFUSE], [CREDS; 5]),
        ("https://api.a.test:443/x?y#z", [CREDS, CREDS, CREDS, CREDS, REFUSE], [CREDS; 5]),
        ("http://api.a.test/", [REFUSE, REFUSE, ANON, CREDS, REFUSE], [REFUSE, REFUSE, ANON, CREDS, REFUSE]),
        ("https://b.test/", [REFUSE, CREDS, ANON, CREDS, REFUSE], [REFUSE, CREDS, ANON, CREDS, REFUSE]),
        ("https://other.b.test/", [REFUSE, REFUSE, ANON, CREDS, REFUSE], [REFUSE, REFUSE, ANON, CREDS, REFUSE]),
        ("https://b.test:8443/", [REFUSE, REFUSE, ANON, CREDS, REFUSE], [REFUSE, REFUSE, ANON, CREDS, REFUSE]),
        ("https://a.test/", [REFUSE, REFUSE, ANON, CREDS, REFUSE], [REFUSE, REFUSE, ANON, CREDS, REFUSE]),
        ("https://u:p@api.a.test/", [REFUSE; 5], [REFUSE; 5]),
        ("ftp://api.a.test/", [REFUSE; 5], [REFUSE; 5]),
        ("file:///etc/passwd", [REFUSE; 5], [REFUSE; 5]),
    ];

    #[test]
    fn r21_table_holds_for_every_policy_and_target() {
        for base in [A, "https://api.a.test:443"] {
            for (index, policy) in policies().iter().enumerate() {
                let rules = rules(base, policy);
                for (target, redirects, links) in R21 {
                    let target_url = url(target);
                    assert_eq!(
                        rules.redirect(&target_url),
                        redirects[index],
                        "redirect to {target} under {} from {base}",
                        policy.name()
                    );
                    assert_eq!(
                        rules.link(&target_url),
                        links[index],
                        "link to {target} under {} from {base}",
                        policy.name()
                    );
                }
            }
        }
    }

    #[test]
    fn loopback_ports_are_distinct_origins() {
        let rules = rules("http://127.0.0.1:8080", &OriginPolicy::same_origin());

        assert_eq!(rules.redirect(&url("http://127.0.0.1:8080/x")), CREDS);
        assert_eq!(rules.redirect(&url("http://127.0.0.1:8081/x")), REFUSE);
    }

    #[test]
    fn http_default_port_is_implicit() {
        let rules = rules("http://a.test:80", &OriginPolicy::same_origin());

        assert_eq!(rules.redirect(&url("http://a.test/x")), CREDS);
    }

    #[test]
    fn credentials_follow_the_base_origin_not_the_previous_hop() {
        let rules = rules(A, &OriginPolicy::allow_without_credentials());

        // A -> B -> B -> A
        assert_eq!(rules.redirect(&url("https://b.test/1")), ANON);
        assert_eq!(rules.redirect(&url("https://b.test/2")), ANON);
        assert_eq!(rules.redirect(&url("https://api.a.test/3")), CREDS);
    }

    fn definitions() -> [OriginPolicy; 5] {
        [
            OriginPolicy::same_origin(),
            OriginPolicy::allow_origins([B, C]),
            OriginPolicy::allow_without_credentials(),
            OriginPolicy::danger_allow_any(),
            OriginPolicy::deny(),
        ]
    }

    fn requests() -> [OriginPolicy; 9] {
        [
            OriginPolicy::deny(),
            OriginPolicy::same_origin(),
            OriginPolicy::allow_origins([B]),
            OriginPolicy::allow_origins([B, C]),
            OriginPolicy::allow_origins(Vec::<String>::new()),
            OriginPolicy::allow_origins([B, D]),
            OriginPolicy::allow_origins([D]),
            OriginPolicy::allow_without_credentials(),
            OriginPolicy::danger_allow_any(),
        ]
    }

    /// Per definition (in `definitions()` order): whether each request (in
    /// `requests()` order) is accepted.
    const TIGHTEN: [[bool; 9]; 5] = [
        // deny, same, [B], [B,C], [], [B,D], [D], awc, any
        [true, true, false, false, false, false, false, false, false],
        [true, true, true, true, true, false, false, false, false],
        [true, true, false, false, false, false, false, false, false],
        [true, true, false, false, false, false, false, false, false],
        [true, false, false, false, false, false, false, false, false],
    ];

    #[test]
    fn tighten_only_table_holds_for_every_pair() {
        let base = url(A);
        for (definition, accepted_row) in definitions().iter().zip(TIGHTEN) {
            for (requested, accepted) in requests().iter().zip(accepted_row) {
                let result = OriginRules::new(&base, definition, Some(requested));
                if accepted {
                    assert!(
                        result.is_ok(),
                        "{requested:?} over {definition:?} should be accepted: {result:?}"
                    );
                } else {
                    assert_eq!(
                        result.expect_err("override should be refused"),
                        BuildError::OriginPolicyNotPermitted {
                            definition: definition.name(),
                            requested: requested.name(),
                        },
                        "{requested:?} over {definition:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn subset_compares_parsed_origins() {
        let result = OriginRules::new(
            &url(A),
            &OriginPolicy::allow_origins([B]),
            Some(&OriginPolicy::allow_origins(["https://b.test:443"])),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn same_origin_over_deny_error_names_both_policies() {
        let error = OriginRules::new(&url(A), &OriginPolicy::deny(), Some(&OriginPolicy::same_origin()))
            .expect_err("same_origin over deny is refused");
        let text = error.to_string();

        assert!(text.contains("same_origin"), "{text}");
        assert!(text.contains("deny"), "{text}");
    }

    #[test]
    fn accepted_overrides_take_effect() {
        let base = url(A);
        let build = |definition: &OriginPolicy, requested: &OriginPolicy| {
            OriginRules::new(&base, definition, Some(requested)).expect("override is accepted")
        };

        let denied = build(&OriginPolicy::danger_allow_any(), &OriginPolicy::deny());
        assert_eq!(denied.redirect(&base), REFUSE);
        assert_eq!(denied.link(&base), CREDS);

        let narrowed = build(&OriginPolicy::allow_origins([B, C]), &OriginPolicy::allow_origins([B]));
        assert_eq!(narrowed.redirect(&url("https://b.test/")), CREDS);
        assert_eq!(narrowed.redirect(&url("https://c.test/")), REFUSE);
        assert_eq!(narrowed.link(&url("https://c.test/")), REFUSE);

        let same = build(&OriginPolicy::allow_without_credentials(), &OriginPolicy::same_origin());
        assert_eq!(same.redirect(&url("https://b.test/")), REFUSE);
        assert_eq!(same.redirect(&base), CREDS);

        let emptied = build(&OriginPolicy::allow_origins([B, C]), &OriginPolicy::allow_origins(Vec::<String>::new()));
        assert_eq!(emptied.redirect(&url("https://b.test/")), REFUSE);
    }

    const ALLOWED_ORIGIN_CASES: &[(&str, Option<&str>)] = &[
        ("https://b.test", None),
        ("https://b.test/", None),
        ("https://b.test:443", None),
        ("http://b.test:8080", None),
        ("https://b.test/path", Some("has a path")),
        ("https://b.test?q=1", Some("has a query")),
        ("https://b.test#f", Some("has a fragment")),
        ("https://u@b.test", Some("carries user information")),
        ("https://u:p@b.test", Some("carries user information")),
        ("ftp://b.test", Some("scheme is not http or https")),
        ("b.test", Some("not an absolute URL")),
        ("https://", Some("not an absolute URL")),
    ];

    #[test]
    fn allowed_origin_entries_are_validated_in_definition_and_request() {
        let base = url(A);
        for (entry, expected) in ALLOWED_ORIGIN_CASES {
            let listed = OriginPolicy::allow_origins([*entry]);
            let as_definition = OriginRules::new(&base, &listed, None).err();
            // The definition is valid, so only the requested entry can fail;
            // an invalid entry wins over the permission check.
            let as_request = OriginRules::new(&base, &OriginPolicy::same_origin(), Some(&listed)).err();

            match expected {
                None => {
                    assert!(as_definition.is_none(), "{entry}: {as_definition:?}");
                }
                Some(reason) => {
                    let expected_error = BuildError::InvalidAllowedOrigin { reason };
                    assert_eq!(as_definition.as_ref(), Some(&expected_error), "{entry} as definition");
                    assert_eq!(as_request.as_ref(), Some(&expected_error), "{entry} as request");
                }
            }
        }
    }

    #[test]
    fn invalid_definition_entry_wins_over_permission_check() {
        let error = OriginRules::new(
            &url(A),
            &OriginPolicy::allow_origins(["https://b.test/path"]),
            Some(&OriginPolicy::danger_allow_any()),
        )
        .expect_err("invalid definition entry");

        assert_eq!(error, BuildError::InvalidAllowedOrigin { reason: "has a path" });
    }

    #[test]
    fn default_port_entry_matches_implicit_port_target() {
        let rules = rules(A, &OriginPolicy::allow_origins(["https://b.test:443"]));

        assert_eq!(rules.redirect(&url("https://b.test/x")), CREDS);
    }

    #[test]
    fn name_matches_definition_spelling() {
        let names: Vec<_> = policies().iter().map(OriginPolicy::name).collect();

        assert_eq!(
            names,
            ["same_origin", "allow_origins", "allow_without_credentials", "danger_allow_any", "deny"]
        );
        assert_eq!(OriginPolicy::default(), OriginPolicy::SameOrigin);
    }
}
