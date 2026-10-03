//! Every Claudine option that takes its value as a separate word on a
//! composition command line, with every spelling.
//!
//! Shared through `#[path]` with `src/argv/partition/tests.rs`, whose drift
//! guard fails when these spellings and the clap-derived owned surface
//! (`OwnedFlags::for_composition`) differ, so the completion matrix in
//! `l1/completion_ownership.rs` covers every value slot clap declares.

/// One owned option.
pub(crate) struct OwnedValueOption {
    /// The long name first, then every alias and short form.
    pub(crate) spellings: &'static [&'static str],
    /// Declared by `sequence` alone.
    pub(crate) sequence_only: bool,
    /// A partial value that narrows the option's candidates, when it has any.
    pub(crate) partial: &'static str,
}

const fn option(spellings: &'static [&'static str], partial: &'static str) -> OwnedValueOption {
    OwnedValueOption { spellings, sequence_only: false, partial }
}

const fn sequence_option(spellings: &'static [&'static str], partial: &'static str) -> OwnedValueOption {
    OwnedValueOption { spellings, sequence_only: true, partial }
}

pub(crate) const OWNED_VALUE_OPTIONS: &[OwnedValueOption] = &[
    // Global, accepted after the file.
    option(&["--debug"], "d"),
    option(&["--provider"], "co"),
    option(&["--exclude"], "co"),
    option(&["--include"], "PA"),
    option(&["--model", "-m"], "g"),
    option(&["--output", "-o"], "j"),
    option(&["--append-system-prompt", "--asp"], "pr"),
    option(&["--replace-system-prompt", "--rsp"], "pr"),
    option(&["--timeout", "-t"], "5"),
    option(&["--step-timeout"], "5"),
    option(&["--stall-timeout"], "5"),
    option(&["--operation", "--op"], "re"),
    option(&["--set"], "{"),
    option(&["--use"], "a"),
    option(&["--max-iterations"], "3"),
    option(&["--on-rate-limit"], "a"),
    sequence_option(&["--fail-fast"], "t"),
    sequence_option(&["--budget-ledger"], "pr"),
];
