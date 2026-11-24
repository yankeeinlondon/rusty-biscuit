//! The configuration-file spelling of actions and messenger routes.
//!
//! In `~/.claudine/config.yaml` and `{repo-root}/.claudine/config.yaml` an
//! action and a messenger route are one-key mappings: the key names the kind
//! and holds its settings.
//!
//! ```yaml
//! actions:
//!   session_start:
//!     - sound_effect: { effect: chime }
//! messenger:
//!   configurations:
//!     team:
//!       discord: { channel_id: "123" }
//! ```
//!
//! [`HookAction`] and [`MessengerProviderConfig`] keep their internally
//! tagged serde form (`type: sound_effect`, `provider: discord`) everywhere
//! else; the adapters here sit on the configuration fields only and turn one
//! spelling into the other. They add no defaults: a configuration file has
//! the schema's defaults in place before serde reads it
//! ([`super::load`]).
//!
//! The flat spelling that configuration files used before is refused with an
//! example of the keyed one ([`flat_entries`]); the legacy JSON5 converter
//! rewrites it ([`upgrade_flat`]).

use std::collections::HashMap;
use std::fmt::Write as _;

use serde::de::Error as _;
use serde::ser::{Error as _, SerializeMap};
use serde::{Deserialize, Deserializer, Serializer};
use serde_json::{Map, Value};

use crate::actions::HookAction;
use crate::config::messaging_block::MessengerProviderConfig;
use crate::events::AgenticEvent;

/// One keyed family: the tag key of its internal form and its known kinds.
struct Family {
    /// What one entry is called in messages.
    noun: &'static str,
    /// The internal tag key (`type`, `provider`).
    tag: &'static str,
    /// Every kind's key, as written in a configuration file.
    kinds: &'static [&'static str],
}

const ACTIONS: Family = Family {
    noun: "action",
    tag: "type",
    kinds: &["sound_effect", "speak", "bash", "call", "report", "message"],
};

const ROUTES: Family = Family {
    noun: "messenger route",
    tag: "provider",
    kinds: &["discord", "slack", "signal", "whatsapp", "discord_webhook", "slack_webhook"],
};

impl Family {
    /// The configuration key of a flat tag, accepting the hyphenated route
    /// spellings the flat form allowed.
    fn kind_of(&self, tag: &str) -> Option<&'static str> {
        let normalized = tag.replace('-', "_");
        self.kinds.iter().copied().find(|kind| *kind == normalized)
    }

    fn known(&self) -> String {
        self.kinds.iter().map(|kind| format!("`{kind}`")).collect::<Vec<_>>().join(", ")
    }

    /// The tagged value of one keyed entry.
    fn untag(&self, entry: Value) -> Result<Value, String> {
        let Value::Object(entry) = entry else {
            return Err(format!(
                "{} {} must be a mapping with one key naming its kind ({})",
                article(self.noun),
                self.noun,
                self.known()
            ));
        };
        if let Some(example) = self.keyed_example(&entry) {
            return Err(example);
        }
        let mut keys = entry.into_iter();
        let (kind, payload) = match (keys.next(), keys.next()) {
            (Some(only), None) => only,
            (None, _) => return Err(format!("{} {} names no kind; write one of {}", article(self.noun), self.noun, self.known())),
            (Some((first, _)), Some((second, _))) => {
                return Err(format!(
                    "{} {} has one key naming its kind, but `{first}` and `{second}` are both written",
                    article(self.noun),
                    self.noun
                ));
            }
        };
        if !self.kinds.contains(&kind.as_str()) {
            return Err(format!("`{kind}` is not {} {}; write one of {}", article(self.noun), self.noun, self.known()));
        }
        let mut settings = match payload {
            Value::Object(settings) => settings,
            Value::Null => return Err(format!("`{kind}` needs its settings; write `{kind}: {{}}` for none")),
            _ => return Err(format!("the settings of `{kind}` must be a mapping")),
        };
        if settings.contains_key(self.tag) {
            return Err(format!("`{}` is not a setting of `{kind}`", self.tag));
        }
        settings.insert(self.tag.to_string(), Value::String(kind));
        Ok(Value::Object(settings))
    }

    /// The keyed value of one tagged entry.
    fn retag(&self, tagged: Value) -> Result<Value, String> {
        let Value::Object(mut settings) = tagged else {
            return Err(format!("{} {} did not serialize as a mapping", article(self.noun), self.noun));
        };
        let Some(Value::String(tag)) = settings.remove(self.tag) else {
            return Err(format!("{} {} serialized without its `{}`", article(self.noun), self.noun, self.tag));
        };
        let kind = self.kind_of(&tag).ok_or_else(|| format!("`{tag}` is not {} {}", article(self.noun), self.noun))?;
        Ok(Value::Object(Map::from_iter([(kind.to_string(), Value::Object(settings))])))
    }

    /// The flat entry upgraded to its keyed form, when `entry` is a flat
    /// entry of a known kind.
    fn upgraded(&self, entry: &Map<String, Value>) -> Option<Value> {
        let Some(Value::String(tag)) = entry.get(self.tag) else {
            return None;
        };
        let kind = self.kind_of(tag)?;
        let settings: Map<String, Value> =
            entry.iter().filter(|(key, _)| key.as_str() != self.tag).map(|(key, value)| (key.clone(), value.clone())).collect();
        Some(Value::Object(Map::from_iter([(kind.to_string(), Value::Object(settings))])))
    }

    /// The migration message for a flat entry, with its keyed spelling.
    fn keyed_example(&self, entry: &Map<String, Value>) -> Option<String> {
        let keyed = self.upgraded(entry)?;
        let mut message = format!(
            "{} {} is now written as a mapping keyed by its kind, not with `{}:`; write",
            article(self.noun),
            self.noun,
            self.tag
        );
        let _ = write!(message, " `{}`", flow(&keyed));
        Some(message)
    }
}

fn article(noun: &str) -> &'static str {
    if noun.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" }
}

/// A one-line YAML flow spelling of `value`.
fn flow(value: &Value) -> String {
    match value {
        Value::Object(map) if map.is_empty() => "{}".to_string(),
        Value::Object(map) => {
            let entries: Vec<String> = map.iter().map(|(key, value)| format!("{key}: {}", flow(value))).collect();
            format!("{{ {} }}", entries.join(", "))
        }
        Value::Array(items) => format!("[{}]", items.iter().map(flow).collect::<Vec<_>>().join(", ")),
        other => other.to_string(),
    }
}

/// One action or route still written in the flat form.
pub(crate) struct FlatEntry {
    /// The JSON pointer segments of the entry.
    pub(crate) segments: Vec<String>,
    /// Its kind's key, which the keyed view ([`upgrade_flat`]) nests the
    /// entry's settings under.
    pub(crate) kind: &'static str,
    /// The migration message, with the keyed spelling.
    pub(crate) message: String,
}

impl FlatEntry {
    /// The authored path of `keyed`, a path in the document after
    /// [`upgrade_flat`]: inside this entry, the kind segment the keyed view
    /// adds is dropped, so a setting maps back to the key the file wrote.
    pub(crate) fn authored(&self, keyed: &[String]) -> Option<Vec<String>> {
        let rest = keyed.strip_prefix(self.segments.as_slice())?;
        let mut authored = self.segments.clone();
        match rest.split_first() {
            Some((kind, settings)) if kind == self.kind => authored.extend_from_slice(settings),
            _ => authored.extend_from_slice(rest),
        }
        Some(authored)
    }
}

/// Every flat action and route in a configuration document.
///
/// A flat entry is a mapping whose `type` (action) or `provider` (route)
/// names a known kind, the spelling configuration files used before actions
/// and routes were keyed.
pub(crate) fn flat_entries(document: &Value) -> Vec<FlatEntry> {
    let mut found = Vec::new();
    visit_entries(document, &mut |segments, family, entry| {
        if let Value::Object(entry) = entry
            && let Some(Value::String(tag)) = entry.get(family.tag)
            && let Some(kind) = family.kind_of(tag)
            && let Some(message) = family.keyed_example(entry)
        {
            found.push(FlatEntry { segments: segments.to_vec(), kind, message });
        }
    });
    found
}

/// Rewrites every flat action and route of `document` in its keyed form.
/// Anything else, including an entry of an unknown kind, is left as written
/// for the schema to judge.
pub(crate) fn upgrade_flat(document: &mut Value) {
    let mut upgrades = Vec::new();
    visit_entries(document, &mut |segments, family, entry| {
        if let Value::Object(entry) = entry
            && let Some(keyed) = family.upgraded(entry)
        {
            upgrades.push((segments.to_vec(), keyed));
        }
    });
    for (segments, keyed) in upgrades {
        if let Some(slot) = segments.iter().try_fold(&mut *document, |value, segment| match value {
            Value::Object(map) => map.get_mut(segment),
            Value::Array(items) => segment.parse::<usize>().ok().and_then(|index| items.get_mut(index)),
            _ => None,
        }) {
            *slot = keyed;
        }
    }
}

/// Calls `visit` with every action (`actions.<event>[i]`) and route
/// (`messenger.configurations.<name>`) of `document`.
fn visit_entries(document: &Value, visit: &mut dyn FnMut(&[String], &Family, &Value)) {
    if let Some(Value::Object(events)) = document.get("actions") {
        for (event, list) in events {
            if let Value::Array(items) = list {
                for (index, item) in items.iter().enumerate() {
                    visit(&["actions".into(), event.clone(), index.to_string()], &ACTIONS, item);
                }
            }
        }
    }
    if let Some(Value::Object(routes)) = document.get("messenger").and_then(|messenger| messenger.get("configurations")) {
        for (name, route) in routes {
            visit(&["messenger".into(), "configurations".into(), name.clone()], &ROUTES, route);
        }
    }
}

/// Serde adapter for an `actions` field: event lists of keyed actions.
///
/// A `null` field or event list is unset: it reads as no actions for that
/// event (and in a repository file, as no override).
pub(crate) mod actions {
    use super::*;

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<HashMap<AgenticEvent, Vec<HookAction>>, D::Error> {
        let Some(events) = Option::<HashMap<AgenticEvent, Option<Vec<Value>>>>::deserialize(deserializer)? else {
            return Ok(HashMap::new());
        };
        let mut actions = HashMap::with_capacity(events.len());
        for (event, list) in events {
            let Some(list) = list else {
                continue;
            };
            let mut read = Vec::with_capacity(list.len());
            for (index, entry) in list.into_iter().enumerate() {
                let tagged = ACTIONS
                    .untag(entry)
                    .map_err(|message| D::Error::custom(format!("actions.{}[{index}]: {message}", event_key(event))))?;
                read.push(serde_json::from_value(tagged).map_err(|error| {
                    D::Error::custom(format!("actions.{}[{index}]: {error}", event_key(event)))
                })?);
            }
            actions.insert(event, read);
        }
        Ok(actions)
    }

    pub(crate) fn serialize<S: Serializer>(
        actions: &HashMap<AgenticEvent, Vec<HookAction>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(actions.len()))?;
        for (event, list) in actions {
            let keyed = list
                .iter()
                .map(|action| serde_json::to_value(action).map_err(S::Error::custom).and_then(|tagged| ACTIONS.retag(tagged).map_err(S::Error::custom)))
                .collect::<Result<Vec<_>, _>>()?;
            map.serialize_entry(event, &keyed)?;
        }
        map.end()
    }

    fn event_key(event: AgenticEvent) -> String {
        serde_json::to_value(event).ok().and_then(|value| value.as_str().map(str::to_string)).unwrap_or_else(|| format!("{event:?}"))
    }
}

/// Serde adapter for `messenger.configurations`: named keyed routes.
///
/// A route name is the user's own, so a `null` route is an error, never
/// skipped.
pub(crate) mod routes {
    use super::*;

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<HashMap<String, MessengerProviderConfig>, D::Error> {
        let Some(named) = Option::<HashMap<String, Value>>::deserialize(deserializer)? else {
            return Ok(HashMap::new());
        };
        named
            .into_iter()
            .map(|(name, route)| {
                let tagged = ROUTES
                    .untag(route)
                    .map_err(|message| D::Error::custom(format!("messenger.configurations.{name}: {message}")))?;
                let route = serde_json::from_value(tagged)
                    .map_err(|error| D::Error::custom(format!("messenger.configurations.{name}: {error}")))?;
                Ok((name, route))
            })
            .collect()
    }

    pub(crate) fn serialize<S: Serializer>(
        routes: &HashMap<String, MessengerProviderConfig>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(routes.len()))?;
        for (name, route) in routes {
            let tagged = serde_json::to_value(route).map_err(S::Error::custom)?;
            map.serialize_entry(name, &ROUTES.retag(tagged).map_err(S::Error::custom)?)?;
        }
        map.end()
    }
}

/// Reads an optional field whose `null` means unset: the type's default.
pub(crate) fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// Reads a map keyed by a closed set of names (events, providers) whose
/// `null` entries are unset and left out; a `null` map is empty.
pub(crate) fn null_entries_unset<'de, D, K, V>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Eq + std::hash::Hash,
    V: Deserialize<'de>,
{
    let entries = Option::<HashMap<K, Option<V>>>::deserialize(deserializer)?.unwrap_or_default();
    Ok(entries.into_iter().filter_map(|(key, value)| value.map(|value| (key, value))).collect())
}

#[cfg(test)]
#[path = "wire_tests.rs"]
mod tests;
