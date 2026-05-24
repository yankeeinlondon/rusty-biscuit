//! Convenience helpers for constructing components from common inputs.
//!
//! [`choice_builders`] exposes free functions that build a
//! [`ChoiceInput<String>`](crate::components::choose::ChoiceInput) from
//! CSV, markdown list, or YAML/JSON dictionary inputs.

pub mod choice_builders;

pub use choice_builders::{
    ChoiceBuilderError, choice_options_from_csv, choice_options_from_dictionary,
    choice_options_from_markdown_list, choose_many_from_csv, choose_many_from_dictionary,
    choose_many_from_markdown_list, choose_one_from_csv, choose_one_from_dictionary,
    choose_one_from_markdown_list,
};
