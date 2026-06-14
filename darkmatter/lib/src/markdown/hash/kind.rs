//! The structural kind of a markdown hash.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Structural kind of a markdown hash.
///
/// The five kinds form the vocabulary shared by the library compute path, the
/// stored `hash` property, and the CLI `--kind` flag. Serde, [`FromStr`], and
/// [`Display`] all use the same lowercase tokens: `fm`, `body`, `simple`,
/// `structured`, `detailed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MdHashKind {
    /// Frontmatter only (keys and values), a single hash.
    Fm,
    /// Body content only, a single hash.
    Body,
    /// `{fm}-{body}` — the default, matching historical `md hash` output.
    Simple,
    /// `{fm}-{fm_keys}-{body}-{body_structure}`.
    Structured,
    /// Nested object: frontmatter pair, nullable preamble, ordered sections.
    Detailed,
}

impl MdHashKind {
    /// The lowercase token used by serde, [`FromStr`], and [`Display`].
    pub fn as_token(self) -> &'static str {
        match self {
            MdHashKind::Fm => "fm",
            MdHashKind::Body => "body",
            MdHashKind::Simple => "simple",
            MdHashKind::Structured => "structured",
            MdHashKind::Detailed => "detailed",
        }
    }

    /// Resolution rank within the linear chain `simple < structured < detailed`,
    /// each a strict superset of the one before.
    ///
    /// `fm` and `body` are *partial* kinds — each captures only one side of
    /// `simple` — so they have no place on the linear chain and return `None`.
    /// They are lower resolution than `simple` and incomparable to each other.
    pub fn resolution(self) -> Option<u8> {
        match self {
            MdHashKind::Simple => Some(0),
            MdHashKind::Structured => Some(1),
            MdHashKind::Detailed => Some(2),
            MdHashKind::Fm | MdHashKind::Body => None,
        }
    }

    /// Relates a `stored` kind to a `forced` kind for `--save` branching.
    ///
    /// The partial order: `fm` and `body` are each lower than `simple`, which is
    /// lower than `structured`, lower than `detailed`. `fm` and `body` are
    /// incomparable to one another.
    pub fn relate(stored: MdHashKind, forced: MdHashKind) -> KindRelation {
        use MdHashKind::{Body, Fm};

        if stored == forced {
            return KindRelation::Same;
        }
        match (stored, forced) {
            // The two partial kinds share no overlapping component.
            (Fm, Body) | (Body, Fm) => KindRelation::Incomparable,
            // A partial stored kind is always below any linear forced kind.
            (Fm | Body, _) => KindRelation::Higher,
            // A linear stored kind is always above a partial forced kind.
            (_, Fm | Body) => KindRelation::Lower,
            // Both linear: compare their resolution ranks (equality is `Same`).
            (a, b) => {
                let (ra, rb) = (
                    a.resolution().expect("linear kind has a rank"),
                    b.resolution().expect("linear kind has a rank"),
                );
                if rb > ra {
                    KindRelation::Higher
                } else {
                    KindRelation::Lower
                }
            }
        }
    }
}

/// How a forced `--kind` relates to a document's stored hash kind.
///
/// Drives the `--save` resolution-change branching: an upgrade ([`Higher`]), a
/// downgrade ([`Lower`]), a same-kind comparison ([`Same`]), or an
/// [`Incomparable`] switch where change cannot be evaluated across the boundary.
///
/// [`Higher`]: KindRelation::Higher
/// [`Lower`]: KindRelation::Lower
/// [`Same`]: KindRelation::Same
/// [`Incomparable`]: KindRelation::Incomparable
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindRelation {
    /// Stored and forced kinds are equal.
    Same,
    /// The forced kind is a strict superset (higher resolution) of the stored.
    Higher,
    /// The forced kind is a strict subset (lower resolution) of the stored.
    Lower,
    /// The two kinds share no overlapping component (`fm` ↔ `body`).
    Incomparable,
}

/// Selects the kind to compute: a forced `--kind` wins; otherwise the document's
/// stored kind is matched; otherwise it defaults to [`MdHashKind::Simple`].
pub fn select_kind(stored: Option<MdHashKind>, forced: Option<MdHashKind>) -> MdHashKind {
    forced.or(stored).unwrap_or(MdHashKind::Simple)
}

impl fmt::Display for MdHashKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_token())
    }
}

/// Error returned when a string does not name a [`MdHashKind`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseMdHashKindError {
    /// The unrecognized token.
    pub token: String,
}

impl fmt::Display for ParseMdHashKindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown hash kind '{}' (expected one of: fm, body, simple, structured, detailed)",
            self.token
        )
    }
}

impl std::error::Error for ParseMdHashKindError {}

impl FromStr for MdHashKind {
    type Err = ParseMdHashKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "fm" => Ok(MdHashKind::Fm),
            "body" => Ok(MdHashKind::Body),
            "simple" => Ok(MdHashKind::Simple),
            "structured" => Ok(MdHashKind::Structured),
            "detailed" => Ok(MdHashKind::Detailed),
            other => Err(ParseMdHashKindError {
                token: other.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_round_trips_through_from_str_and_display() {
        for kind in [
            MdHashKind::Fm,
            MdHashKind::Body,
            MdHashKind::Simple,
            MdHashKind::Structured,
            MdHashKind::Detailed,
        ] {
            let token = kind.to_string();
            assert_eq!(token, kind.as_token());
            assert_eq!(MdHashKind::from_str(&token), Ok(kind));
        }
    }

    #[test]
    fn from_str_trims_whitespace() {
        assert_eq!(
            MdHashKind::from_str("  structured "),
            Ok(MdHashKind::Structured)
        );
    }

    #[test]
    fn from_str_rejects_unknown_token() {
        let err = MdHashKind::from_str("bogus").unwrap_err();
        assert_eq!(err.token, "bogus");
        assert!(err.to_string().contains("unknown hash kind 'bogus'"));
    }

    #[test]
    fn serde_uses_lowercase_tokens() {
        let json = serde_json::to_string(&MdHashKind::Detailed).unwrap();
        assert_eq!(json, "\"detailed\"");
        let parsed: MdHashKind = serde_json::from_str("\"fm\"").unwrap();
        assert_eq!(parsed, MdHashKind::Fm);
    }

    #[test]
    fn linear_chain_orders_by_resolution() {
        use MdHashKind::{Detailed, Simple, Structured};
        assert_eq!(MdHashKind::relate(Simple, Structured), KindRelation::Higher);
        assert_eq!(MdHashKind::relate(Simple, Detailed), KindRelation::Higher);
        assert_eq!(
            MdHashKind::relate(Structured, Detailed),
            KindRelation::Higher
        );
        assert_eq!(MdHashKind::relate(Detailed, Simple), KindRelation::Lower);
        assert_eq!(MdHashKind::relate(Structured, Simple), KindRelation::Lower);
        assert_eq!(MdHashKind::relate(Simple, Simple), KindRelation::Same);
    }

    #[test]
    fn partial_kinds_are_below_simple_and_incomparable_to_each_other() {
        use MdHashKind::{Body, Detailed, Fm, Simple};
        assert_eq!(MdHashKind::relate(Simple, Fm), KindRelation::Lower);
        assert_eq!(MdHashKind::relate(Simple, Body), KindRelation::Lower);
        assert_eq!(MdHashKind::relate(Fm, Simple), KindRelation::Higher);
        assert_eq!(MdHashKind::relate(Body, Detailed), KindRelation::Higher);
        assert_eq!(MdHashKind::relate(Fm, Body), KindRelation::Incomparable);
        assert_eq!(MdHashKind::relate(Body, Fm), KindRelation::Incomparable);
        assert_eq!(MdHashKind::relate(Fm, Fm), KindRelation::Same);
    }

    #[test]
    fn select_kind_prefers_forced_then_stored_then_simple() {
        assert_eq!(
            select_kind(Some(MdHashKind::Structured), Some(MdHashKind::Detailed)),
            MdHashKind::Detailed,
        );
        assert_eq!(
            select_kind(Some(MdHashKind::Structured), None),
            MdHashKind::Structured,
        );
        assert_eq!(select_kind(None, None), MdHashKind::Simple);
        assert_eq!(select_kind(None, Some(MdHashKind::Fm)), MdHashKind::Fm);
    }
}
