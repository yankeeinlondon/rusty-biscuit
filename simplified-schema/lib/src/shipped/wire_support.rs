//! Serde support for the language types that shipped records hold.
//!
//! Shipped records carry only portable, compiled-in identities. The wire
//! forms here refuse what a shipped artifact must never contain, on both
//! sides: a host path ([`DefNamespace`]'s and [`SchemaSourceId`]'s
//! file-backed variants, a slot's origin file) cannot be serialized or
//! deserialized. An `Option` field marked with [`present`] must be written
//! (`null` for `None`), so a missing member is an error rather than `None`.

use std::collections::BTreeMap;

use indexmap::IndexMap;
use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::simplified::{DefKey, DefNamespace, Definition, DefinitionTable, SchemaSourceId};

/// Deserializes an `Option` that must be present: `null` is `None`, and an
/// absent member is a missing-field error (serde's derive defaults an absent
/// `Option` to `None` unless the field names its own deserializer). Hosts
/// use it on their own records' `Option` fields:
/// `#[serde(deserialize_with = "simplified_schema::shipped::present")]`.
pub fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

/// A slot's origin file: only `None` is representable on the wire, since a
/// shipped schema holds no slot stamped with a host file.
pub(crate) mod no_host_path {
    use super::*;

    pub(crate) fn serialize<S: Serializer>(value: &Option<std::path::PathBuf>, serializer: S) -> Result<S::Ok, S::Error> {
        match value {
            None => serializer.serialize_none(),
            Some(path) => Err(S::Error::custom(format!("a host path (`{}`) cannot be shipped", path.display()))),
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<std::path::PathBuf>, D::Error> {
        match Option::<serde::de::IgnoredAny>::deserialize(deserializer)? {
            None => Ok(None),
            Some(_) => Err(D::Error::custom("a shipped slot cannot name a host file")),
        }
    }
}

/// The portable variants of [`DefNamespace`]. The `Bundle*` variants are
/// the file-backed namespaces of a file inside an embedded bundle's virtual
/// root (`<claudine>/schemas/shared.yaml`, see
/// [`BundleSources::embedded`](crate::resolve::bundle::BundleSources::embedded)),
/// which names no host file.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum WireNamespace {
    InlineRoot,
    RootUnionArm(usize),
    Embedded(String),
    Shipped(String),
    BundleFile(String),
    BundleFileRootUnionArm(String, usize),
    BundleTriggerPayload(String),
    BundleTriggerPayloadUnionArm(String, usize),
}

/// `path` written portably when it lies under an embedded bundle's virtual
/// root `<name>`; `None` for a host path.
fn bundle_path(path: &std::path::Path) -> Option<String> {
    let mut components = path.components();
    let std::path::Component::Normal(root) = components.next()? else { return None };
    let root = root.to_str()?;
    let name = root.strip_prefix('<')?.strip_suffix('>')?;
    if !crate::embedded::is_plain_segment(name) {
        return None;
    }
    let mut portable = root.to_string();
    for component in components {
        let std::path::Component::Normal(segment) = component else { return None };
        portable.push('/');
        portable.push_str(segment.to_str()?);
    }
    Some(portable)
}

impl Serialize for DefNamespace {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let bundled = |path: &std::path::Path| {
            bundle_path(path).ok_or_else(|| S::Error::custom("a file-backed namespace cannot be shipped"))
        };
        let wire = match self {
            Self::InlineRoot => WireNamespace::InlineRoot,
            Self::RootUnionArm(index) => WireNamespace::RootUnionArm(*index),
            Self::Embedded(path) => WireNamespace::Embedded(path.clone()),
            Self::Shipped(path) => WireNamespace::Shipped(path.clone()),
            Self::File(path) => WireNamespace::BundleFile(bundled(path)?),
            Self::FileRootUnionArm(path, index) => WireNamespace::BundleFileRootUnionArm(bundled(path)?, *index),
            Self::TriggerPayload(path) => WireNamespace::BundleTriggerPayload(bundled(path)?),
            Self::TriggerPayloadUnionArm(path, index) => WireNamespace::BundleTriggerPayloadUnionArm(bundled(path)?, *index),
        };
        wire.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DefNamespace {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let local = bundle_local::<D>;
        Ok(match WireNamespace::deserialize(deserializer)? {
            WireNamespace::InlineRoot => Self::InlineRoot,
            WireNamespace::RootUnionArm(index) => Self::RootUnionArm(index),
            WireNamespace::Embedded(path) => Self::Embedded(path),
            WireNamespace::Shipped(path) => Self::Shipped(path),
            WireNamespace::BundleFile(path) => Self::File(local(path)?),
            WireNamespace::BundleFileRootUnionArm(path, index) => Self::FileRootUnionArm(local(path)?, index),
            WireNamespace::BundleTriggerPayload(path) => Self::TriggerPayload(local(path)?),
            WireNamespace::BundleTriggerPayloadUnionArm(path, index) => Self::TriggerPayloadUnionArm(local(path)?, index),
        })
    }
}

/// The portable variants of [`SchemaSourceId`]; `BundleFile` as for
/// [`WireNamespace`].
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum WireSourceId {
    InlineDocument,
    Embedded(String),
    BundleFile(String),
}

impl Serialize for SchemaSourceId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::InlineDocument => WireSourceId::InlineDocument.serialize(serializer),
            Self::Embedded(path) => WireSourceId::Embedded(path.clone()).serialize(serializer),
            Self::File(path) => match bundle_path(path) {
                Some(portable) => WireSourceId::BundleFile(portable).serialize(serializer),
                None => Err(S::Error::custom(format!("a host file (`{}`) cannot be shipped", path.display()))),
            },
        }
    }
}

impl<'de> Deserialize<'de> for SchemaSourceId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match WireSourceId::deserialize(deserializer)? {
            WireSourceId::InlineDocument => Self::InlineDocument,
            WireSourceId::Embedded(path) => Self::Embedded(path),
            WireSourceId::BundleFile(portable) => Self::File(bundle_local::<D>(portable)?),
        })
    }
}

/// The local path of the bundle path `portable`, which must round-trip.
fn bundle_local<'de, D: Deserializer<'de>>(portable: String) -> Result<std::path::PathBuf, D::Error> {
    let path: std::path::PathBuf = portable.split('/').collect();
    bundle_path(&path)
        .filter(|round_trip| *round_trip == portable)
        .map(|_| path)
        .ok_or_else(|| D::Error::custom(format!("`{portable}` is not a path inside an embedded bundle")))
}

/// A [`DefinitionTable`] on the wire: its entries as ordered
/// `[key, definition]` pairs, in allocation order. The `$defs`-key index is
/// rebuilt on decoding.
impl Serialize for DefinitionTable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}

impl<'de> Deserialize<'de> for DefinitionTable {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let pairs = Vec::<(DefKey, Definition)>::deserialize(deserializer)?;
        let mut entries = IndexMap::with_capacity(pairs.len());
        let mut by_json_key = BTreeMap::new();
        for (key, definition) in pairs {
            if by_json_key.insert(definition.json_key.clone(), key.clone()).is_some() {
                return Err(D::Error::custom(format!("`$defs` key `{}` is stored twice", definition.json_key)));
            }
            if entries.insert(key.clone(), definition).is_some() {
                return Err(D::Error::custom(format!("definition `{}` is stored twice", key.name)));
            }
        }
        Ok(DefinitionTable::from_parts(entries, by_json_key))
    }
}
