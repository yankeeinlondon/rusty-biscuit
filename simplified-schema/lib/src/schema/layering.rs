//! Layering: merging other schemas into a [`SimplifiedSchema`] with the
//! resolver's whole-property replacement.

use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use super::{Definition, SimplifiedSchema, SimplifiedSchemaError};
use crate::resolve::merge_baseline;

/// Which side of a layering call wins a property both declare.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Winner {
    Receiver,
    Incoming,
}

impl SimplifiedSchema {
    /// Layers `incoming` beneath this schema: the receiver keeps every
    /// property it declares, and gains the ones only `incoming` declares.
    ///
    /// `incoming` runs from low to high priority, so a later schema wins a
    /// property two of them declare. A declaration is replaced whole, never
    /// intersected. The sources are only borrowed and never change; the
    /// returned reference is the receiver itself.
    ///
    /// ## Errors
    ///
    /// The merge's typed error ([`SimplifiedSchemaError::Schema`]) when a
    /// lower layer is not a simple object schema (a root union, for one) or
    /// its references would not keep their meaning, and
    /// [`SimplifiedSchemaError::Unprepared`] when a participant cannot be
    /// lowered. The call is atomic: on `Err` the receiver's definition,
    /// revision, and prepared validation state are exactly as before.
    pub fn layer_under(&mut self, incoming: &[&SimplifiedSchema]) -> Result<&mut Self, SimplifiedSchemaError> {
        self.layer(incoming, Winner::Receiver)
    }

    /// Layers `incoming` over this schema: every property `incoming`
    /// declares replaces the receiver's, and the receiver keeps the rest.
    ///
    /// Otherwise as [`Self::layer_under`]: low-to-high order, whole-property
    /// replacement, borrowed sources, atomic.
    ///
    /// ## Errors
    ///
    /// As [`Self::layer_under`].
    pub fn layer_over(&mut self, incoming: &[&SimplifiedSchema]) -> Result<&mut Self, SimplifiedSchemaError> {
        self.layer(incoming, Winner::Incoming)
    }

    /// `base` as this schema's base: [`Self::layer_under`] with one layer.
    ///
    /// ## Errors
    ///
    /// As [`Self::layer_under`].
    pub fn as_base(&mut self, base: &SimplifiedSchema) -> Result<&mut Self, SimplifiedSchemaError> {
        self.layer_under(&[base])
    }

    /// `layer` overriding this schema: [`Self::layer_over`] with one layer.
    ///
    /// ## Errors
    ///
    /// As [`Self::layer_under`].
    pub fn override_with(&mut self, layer: &SimplifiedSchema) -> Result<&mut Self, SimplifiedSchemaError> {
        self.layer_over(&[layer])
    }

    /// Stages the whole call in owned merge data and commits with one
    /// assignment, so a failure anywhere leaves the receiver untouched.
    fn layer(&mut self, incoming: &[&SimplifiedSchema], winner: Winner) -> Result<&mut Self, SimplifiedSchemaError> {
        let Some((lowest, higher)) = incoming.split_first() else {
            return Ok(self);
        };
        let lowered = |schema: &SimplifiedSchema| schema.lowered().map_err(SimplifiedSchemaError::Unprepared);

        // The incoming list merged low to high. The lowest schema is the
        // borrowed first base; a merge consumes its higher side, so each
        // higher one is cloned once.
        let lowest = lowered(lowest)?;
        let mut combined: Cow<'_, Value> = Cow::Borrowed(lowest.as_ref());
        for schema in higher {
            let next = lowered(schema)?;
            combined = Cow::Owned(merge_baseline(&combined, next.as_ref().clone())?);
        }

        let receiver = lowered(self)?;
        let merged = match winner {
            Winner::Receiver => merge_baseline(&combined, receiver.as_ref().clone())?,
            Winner::Incoming => merge_baseline(&receiver, combined.into_owned())?,
        };
        if merged == *receiver {
            // Nothing incoming survived the merge: the definition is
            // unchanged, so its revision and prepared state stay valid.
            return Ok(self);
        }

        let incoming_layers: Vec<_> =
            incoming.iter().rev().flat_map(|schema| schema.definition.layers.iter().cloned()).collect();
        let own = self.definition.layers.iter().cloned();
        let (layers, receiver) = match winner {
            Winner::Receiver => (own.chain(incoming_layers).collect(), self.definition.receiver),
            Winner::Incoming => {
                let receiver = incoming_layers.len() + self.definition.receiver;
                (incoming_layers.into_iter().chain(own).collect(), receiver)
            }
        };
        let definition = Arc::new(Definition {
            layers,
            receiver,
            merged: Some(Arc::new(merged)),
        });

        // Commit.
        self.definition = definition;
        self.revision += 1;
        self.prepared = Mutex::new(None);
        Ok(self)
    }
}
