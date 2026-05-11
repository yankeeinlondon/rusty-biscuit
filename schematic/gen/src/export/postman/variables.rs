//! Collection-level variable helpers.

use super::types::PostmanVariable;

/// Appends `additions` to `target`, dropping any whose `key` is already
/// declared. Preserves order: existing entries stay first, new entries
/// follow in argument order.
pub(crate) fn merge_variables(target: &mut Vec<PostmanVariable>, additions: Vec<PostmanVariable>) {
    for var in additions {
        if !target.iter().any(|existing| existing.key == var.key) {
            target.push(var);
        }
    }
}
