//! Collect the speech phrases a parsed lifecycle can play.
//!
//! The walk is structural: it visits parsed actions, never evaluates a guard,
//! and never scans arbitrary frontmatter for keys named `say`. Root
//! dictionary bundles, positional and named speech arguments, and the `speak`
//! alias are already plain [`CommunicationAction`]s once parsed, so the walk
//! sees them like any other `say`.

use darkmatter::markdown::compose::expression::Expr;

use super::super::LifecycleConfig;
use super::super::LifecycleSignal;
use super::super::actions::{
    CommunicationAction, CommunicationChannel, LifecycleActionKind, LifecycleNode, LifecyclePath,
    for_each_action,
};
use super::super::bindings::LifecycleScope;

/// One speech phrase found in a parsed stack, not yet classified.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Phrase<'a> {
    /// The scope the phrase is spoken in; it decides which globals the phrase
    /// can read.
    pub scope: LifecycleScope,
    /// The authored path of the speech action, e.g. `success[0].then[1].say`.
    pub path: &'a LifecyclePath,
    /// The phrase as parsed.
    pub text: &'a Expr,
}

/// Every `say`/`speak` phrase in `config`, in event then authored order.
///
/// `initialize` is never walked: it fires moments after preparation, so a warm
/// cache gains it nothing. Both bodies of every conditional are walked,
/// whatever their guards would select.
pub(crate) fn lifecycle_phrases(config: &LifecycleConfig) -> Vec<Phrase<'_>> {
    let mut phrases = Vec::new();
    for signal in LifecycleSignal::ALL {
        if signal == LifecycleSignal::Initialize {
            continue;
        }
        phrases.extend(stack_phrases(
            LifecycleScope::Event(signal),
            config.stack(signal),
        ));
    }
    phrases
}

/// Every `say`/`speak` phrase in one parsed stack, such as a sequence task's
/// `setup:` or `teardown:`, spoken in `scope`.
pub(crate) fn stack_phrases(scope: LifecycleScope, nodes: &[LifecycleNode]) -> Vec<Phrase<'_>> {
    let mut phrases = Vec::new();
    for_each_action(nodes, &mut |action| {
        if let LifecycleActionKind::Communication(CommunicationAction { channel, text }) =
            &action.kind
            && matches!(channel, CommunicationChannel::Say | CommunicationChannel::Speak)
        {
            phrases.push(Phrase {
                scope,
                path: &action.path,
                text,
            });
        }
    });
    phrases
}

#[cfg(test)]
mod tests;
