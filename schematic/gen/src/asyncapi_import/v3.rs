//! The AsyncAPI 3.0 channel and operation reader.
//!
//! A channel is an endpoint: its key is the endpoint id and its `address`
//! the path. Its `messages` are the endpoint's messages, and the operations
//! that reference them give each its direction from the client application's
//! point of view.

use std::collections::BTreeMap;

use schematic_define::websocket::{ConnectionLifecycle, MessageDirection, WebSocketEndpoint};
use serde_json::{Map, Value};

use super::reader::child;
use super::{Import, MessageParts, X_ROLE, endpoint_role, extensions};
use crate::export::asyncapi::pointer_token;

/// One channel as read, before operations give its messages directions.
struct Channel<'a> {
    key: String,
    at: String,
    object: &'a Map<String, Value>,
    address: String,
    /// `(channel message key, message)`, in key order.
    messages: Vec<(String, MessageParts)>,
}

/// What the operations say about one channel message.
#[derive(Default)]
struct Usage {
    send: bool,
    receive: bool,
    reply: Option<String>,
}

impl<'a> Import<'a> {
    pub(super) fn v3_channels(
        &mut self,
        channels: &'a Map<String, Value>,
        root: &'a Map<String, Value>,
    ) -> Vec<WebSocketEndpoint> {
        let mut read = Vec::new();
        for (key, channel) in channels {
            let at = child("#/channels", key);
            let Some(object) = self.reader.required_object(Some(channel), &at) else { continue };
            let address_at = child(&at, "address");
            let address = match object.get("address") {
                Some(Value::Null) => {
                    self.reader.error(&address_at, "A channel with a dynamic (null) address is not supported");
                    None
                }
                value => self.reader.required_string(value, &address_at),
            };
            let mut messages = Vec::new();
            let messages_at = child(&at, "messages");
            for (message_key, message) in self.reader.object(object.get("messages"), &messages_at).into_iter().flatten() {
                if let Some(parts) = self.message(message, &child(&messages_at, message_key), message_key) {
                    messages.push((message_key.clone(), parts));
                }
            }
            read.push(Channel {
                key: key.clone(),
                at,
                object,
                address: address.unwrap_or_default().to_string(),
                messages,
            });
        }

        let usage = self.operations(root, channels, &read);

        let mut endpoints = Vec::new();
        for channel in read {
            let Channel { key, at, object, address, messages } = channel;
            let mut endpoint_messages = Vec::new();
            for (message_key, parts) in messages {
                let used = usage.get(&(key.clone(), message_key.clone()));
                let direction = match used.map(|u| (u.send, u.receive)) {
                    Some((true, true)) => MessageDirection::Bidirectional,
                    Some((true, false)) => MessageDirection::Client,
                    Some((false, true)) => MessageDirection::Server,
                    _ => {
                        self.reader.warn(
                            &child(&child(&at, "messages"), &message_key),
                            "No operation sends or receives this message; it is not imported",
                        );
                        continue;
                    }
                };
                let mut schema = parts.into_schema(direction);
                schema.responds_with = used.and_then(|u| u.reply.clone());
                endpoint_messages.push(schema);
            }
            let params = self.channel_params(object, &at, &address);
            let mut endpoint = WebSocketEndpoint {
                id: key,
                path: address,
                description: self.reader.string(object.get("description"), &child(&at, "description")).unwrap_or_default().to_string(),
                params,
                role: extensions::role(&mut self.reader, object.get(X_ROLE), &child(&at, X_ROLE), endpoint_role(self.ws_role)),
                lifecycle: ConnectionLifecycle::default(),
                messages: endpoint_messages,
                runtime: None,
            };
            self.finish_endpoint(&mut endpoint, object, &at);
            endpoints.push(endpoint);
        }
        endpoints
    }

    /// How the operations use each `(channel key, message key)`.
    fn operations(
        &mut self,
        root: &'a Map<String, Value>,
        channels: &'a Map<String, Value>,
        read: &[Channel<'a>],
    ) -> BTreeMap<(String, String), Usage> {
        let mut usage: BTreeMap<(String, String), Usage> = BTreeMap::new();
        let Some(operations) = self.reader.object(root.get("operations"), "#/operations") else { return usage };
        for (key, operation) in operations {
            let at = child("#/operations", key);
            let Some(object) = self.reader.required_object(Some(operation), &at) else { continue };
            let action_at = child(&at, "action");
            let action = match self.reader.required_string(object.get("action"), &action_at) {
                Some(action @ ("send" | "receive")) => action,
                Some(other) => {
                    self.reader.error(&action_at, format!("Action '{other}' is not `send` or `receive`"));
                    continue;
                }
                None => continue,
            };
            let channel_at = child(&at, "channel");
            let Some(reference) = self.reader.required_object(object.get("channel"), &channel_at).map(|_| &object["channel"]) else {
                continue;
            };
            let Some((channel_key, _)) = self.reader.local_ref(reference, "#/channels/", Some(channels), &channel_at) else {
                continue;
            };
            let Some(channel) = read.iter().find(|channel| channel.key == channel_key) else { continue };
            if object.contains_key("security") {
                self.reader.warn(&child(&at, "security"), "Operation-level security is not imported; the server's applied security is the API's auth");
            }

            let messages_at = child(&at, "messages");
            let message_keys: Vec<String> = match object.get("messages") {
                // AsyncAPI 3.0: an operation without `messages` covers every
                // message of its channel.
                None => {
                    self.reader.info(&messages_at, "The operation lists no messages, so it covers every message of its channel");
                    channel.messages.iter().map(|(message_key, _)| message_key.clone()).collect()
                }
                Some(value) => {
                    let Some(items) = self.reader.array(Some(value), &messages_at) else { continue };
                    if items.is_empty() {
                        self.reader.warn(&messages_at, "The operation carries no messages");
                    }
                    items
                        .iter()
                        .enumerate()
                        .filter_map(|(index, item)| self.channel_message(item, channel, &child(&messages_at, &index.to_string())))
                        .collect()
                }
            };
            for message_key in &message_keys {
                let entry = usage.entry((channel.key.clone(), message_key.clone())).or_default();
                match action {
                    "send" => entry.send = true,
                    _ => entry.receive = true,
                }
            }

            let reply_at = child(&at, "reply");
            let Some(reply) = self.reader.object(object.get("reply"), &reply_at) else { continue };
            if action == "receive" {
                self.reader.info(&reply_at, "A reply to a received message is not imported");
                continue;
            }
            let replies_at = child(&reply_at, "messages");
            let Some(first) = self.reader.array(reply.get("messages"), &replies_at).and_then(|items| items.first()) else {
                self.reader.warn(&reply_at, "The reply names no message; it is not imported");
                continue;
            };
            let Some(reply_key) = self.channel_message(first, channel, &child(&replies_at, "0")) else { continue };
            let reply_name = channel
                .messages
                .iter()
                .find(|(message_key, _)| *message_key == reply_key)
                .map(|(_, parts)| parts.name.clone());
            for message_key in &message_keys {
                let entry = usage.entry((channel.key.clone(), message_key.clone())).or_default();
                match (&entry.reply, &reply_name) {
                    (Some(existing), Some(name)) if existing != name => self.reader.info(
                        &reply_at,
                        format!("Message '{message_key}' already replies with '{existing}'; '{name}' is not imported"),
                    ),
                    (None, Some(name)) => entry.reply = Some(name.clone()),
                    _ => {}
                }
            }
        }
        usage
    }

    /// The channel message key a `{"$ref": "#/channels/<key>/messages/<m>"}`
    /// names, which must be a message of `channel`.
    fn channel_message(&mut self, reference: &Value, channel: &Channel<'a>, at: &str) -> Option<String> {
        let ref_at = child(at, "$ref");
        let object = self.reader.required_object(Some(reference), at)?;
        let text = self.reader.required_string(object.get("$ref"), &ref_at)?;
        let prefix = format!("#/channels/{}/messages/", pointer_token(&channel.key));
        let key = text.strip_prefix(&prefix).map(super::reader::unescape_token);
        match key.filter(|key| channel.messages.iter().any(|(message_key, _)| message_key == key)) {
            Some(key) => Some(key),
            None => {
                self.reader.error(&ref_at, format!("Reference `{text}` is not a message of channel '{}'", channel.key));
                None
            }
        }
    }
}
