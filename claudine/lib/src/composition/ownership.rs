//! Type-aware ownership of the composition arguments after the file.
//!
//! The CLI's pre-clap partition settles what needs no document: Claudine's
//! own options, the first authored `--`, ordering errors, and which token is
//! the composition file. Everything else after the file reaches
//! [`own_arguments`] as an [`ArgumentsAfterFile`], once the file's authored
//! frontmatter has been read. This module decides, token by token, whether
//! each one is a Claudine setter, a Claudine positional (`argv`), or part of
//! the provider tail, using the compiled switch catalog
//! ([`crate::provider::match_switch_token`]) for every candidate provider.
//!
//! The same assignments are checked again, by [`check_launch_tail`], against
//! the provider and native command path a launch actually uses.
//!
//! The rules, with examples, are in `docs/topics/argv-normalization.md`
//! ("Type-aware ownership").

use std::fmt;
use std::sync::Arc;

use super::provider_tail::{ProviderTail, SwitchAssignment};
use crate::provider::{
    Provider, SwitchAttachment, SwitchToken, SwitchValue, VariadicMin, match_switch_token,
};

/// Where ownership reads switch types: the compiled catalog, or a fixed one
/// in tests.
trait SwitchSource {
    fn matched<'t>(&self, provider: Provider, path: &[&str], token: &'t str) -> Option<SwitchToken<'t>>;
}

/// The generated `ProviderInfo::cli_switches` of every provider.
struct CompiledCatalog;

impl SwitchSource for CompiledCatalog {
    fn matched<'t>(&self, provider: Provider, path: &[&str], token: &'t str) -> Option<SwitchToken<'t>> {
        match_switch_token(provider, path, token)
    }
}

/// The frontmatter property that receives positional arguments. It can never
/// be set as a named parameter.
pub const ARGV_KEY: &str = "argv";

/// One caller argument after the composition file, as the pre-clap partition
/// left it.
#[derive(Clone, PartialEq, Eq)]
pub enum CallerArgument {
    /// A token no rule has classified yet.
    Token(String),
    /// A Claudine option (with its value, if any) stood here and was taken
    /// out for clap. It ends any provider switch's value run, so a later
    /// bare word never becomes a value of an earlier switch.
    ClaudineOption,
}

/// The unclassified arguments after the composition file, plus the opaque
/// suffix after an authored `--` (`None` without one).
///
/// Tokens are caller input and may hold secrets, so `Debug` prints counts.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct ArgumentsAfterFile {
    arguments: Vec<CallerArgument>,
    opaque: Option<Vec<String>>,
}

impl ArgumentsAfterFile {
    pub fn new(arguments: Vec<CallerArgument>, opaque: Option<Vec<String>>) -> Self {
        Self { arguments, opaque }
    }

    pub fn arguments(&self) -> &[CallerArgument] {
        &self.arguments
    }

    /// The tokens after the authored `--`, or `None` without one.
    pub fn opaque(&self) -> Option<&[String]> {
        self.opaque.as_deref()
    }
}

impl fmt::Debug for ArgumentsAfterFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArgumentsAfterFile")
            .field("arguments", &self.arguments.len())
            .field("opaque", &self.opaque.as_ref().map(Vec::len))
            .finish()
    }
}

/// The key of a setter-shaped token (`key=value` with a key matching
/// `^[A-Za-z_][A-Za-z0-9_-]*$`), or `None`.
pub fn setter_key(token: &str) -> Option<&str> {
    let (key, _) = token.split_once('=')?;
    let mut chars = key.chars();
    let first = chars.next()?;
    let valid = (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-');
    valid.then_some(key)
}

/// Whether `token` is a number for ownership: a finite decimal with an
/// optional sign, fraction, and exponent (`5`, `+5`, `.5`, `5.`, `1e-3`).
/// Hexadecimal, `NaN`, infinity, and digit separators are not.
pub fn is_ownership_number(token: &str) -> bool {
    let body = token.strip_prefix(['+', '-']).unwrap_or(token);
    let (mantissa, exponent) = match body.find(['e', 'E']) {
        Some(at) => (&body[..at], Some(&body[at + 1..])),
        None => (body, None),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = |text: &str| text.chars().all(|ch| ch.is_ascii_digit());
    let mantissa_ok = digits(whole) && digits(fraction) && !(whole.is_empty() && fraction.is_empty());
    let exponent_ok = exponent.is_none_or(|exp| {
        let exp = exp.strip_prefix(['+', '-']).unwrap_or(exp);
        !exp.is_empty() && digits(exp)
    });
    mantissa_ok && exponent_ok
}

/// What the authored `$schema` establishes about parameter names.
#[derive(Clone)]
pub enum SchemaParameters {
    /// The document declares no `$schema`: no key is a schema parameter.
    NoSchema,
    /// Whether a top-level property name is declared (in any union arm).
    Declared(Arc<dyn Fn(&str) -> bool + Send + Sync>),
    /// A `$schema` is authored but its names cannot be read without
    /// evaluating it (it is templated). A setter a provider switch would
    /// take is then an error, because it may be a parameter.
    Unestablished,
}

impl SchemaParameters {
    /// Parameters declared by name; for tests and fixed schemas.
    pub fn from_names<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let names: Vec<String> = names.into_iter().map(Into::into).collect();
        Self::Declared(Arc::new(move |key| names.iter().any(|name| name == key)))
    }

    /// `None` when the names cannot be established.
    fn declares(&self, key: &str) -> Option<bool> {
        match self {
            Self::NoSchema => Some(false),
            Self::Declared(declares) => Some(declares(key)),
            Self::Unestablished => None,
        }
    }
}

impl fmt::Debug for SchemaParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoSchema => "NoSchema",
            Self::Declared(_) => "Declared",
            Self::Unestablished => "Unestablished",
        })
    }
}

/// A provider whose switch types apply, at the native command path its
/// launch would use (`["exec"]` for a non-interactive Codex run).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipCandidate {
    pub provider: Provider,
    pub command_path: Vec<String>,
}

impl OwnershipCandidate {
    fn path(&self) -> Vec<&str> {
        self.command_path.iter().map(String::as_str).collect()
    }
}

/// The result of ownership: Claudine's setters and positionals in their
/// original order, and the provider tail with each switch's values recorded.
pub struct OwnedArguments {
    /// Setter (`key=value`) and positional tokens, in order.
    pub claudine: Vec<String>,
    pub tail: ProviderTail,
}

impl fmt::Debug for OwnedArguments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnedArguments")
            .field("claudine", &self.claudine.len())
            .field("tail", &self.tail)
            .finish()
    }
}

/// Why the arguments after the file could not be owned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipError {
    /// An `argv=…` setter before `--`.
    ReservedArgv,
    /// Candidate providers disagree about whether a switch takes a word.
    Ambiguous(AmbiguousSwitch),
    /// A setter a provider switch would take may be a schema parameter, and
    /// the authored `$schema` cannot say.
    ContestedSetter { switch: String, key: String },
    /// The tail is wrong for every candidate provider.
    Mismatch(TailMismatch),
}

/// A switch whose following word the candidates read differently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbiguousSwitch {
    /// The switch as it may be shown (never an attached value).
    pub switch: String,
    /// Each candidate, how it types the switch, and whether that reading
    /// takes the word.
    pub readings: Vec<SwitchReading>,
}

/// One candidate's reading of a switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchReading {
    pub provider: Provider,
    pub value: SwitchValue,
    pub takes_word: bool,
}

/// The guidance every ownership error ends with.
const ESCAPE: &str = "Name the provider (for example `--codex`), or pass provider arguments after `--`.";

impl fmt::Display for OwnershipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReservedArgv => write!(
                f,
                "`{ARGV_KEY}` is reserved for positional arguments and cannot be set by name.\n\
                 Pass the values as bare words after the composition file instead \
                 (for example `claudine compose plan.md alpha beta`)."
            ),
            Self::Ambiguous(ambiguous) => {
                writeln!(
                    f,
                    "ambiguous provider argument: the candidate agents read the word after `{}` \
                     differently.",
                    ambiguous.switch
                )?;
                for reading in &ambiguous.readings {
                    writeln!(
                        f,
                        "  {}: {} ({})",
                        reading.provider,
                        describe_value(reading.value),
                        if reading.takes_word {
                            "takes the word as its value"
                        } else {
                            "leaves the word to Claudine"
                        }
                    )?;
                }
                write!(f, "{ESCAPE}")
            }
            Self::ContestedSetter { switch, key } => write!(
                f,
                "cannot tell whether `{key}=…` after `{switch}` is a value for the provider \
                 or a document parameter: the document's `$schema` is templated, so its \
                 parameter names are not known before it runs.\n\
                 Put the provider value after `--`, or pass the setter with `--set`."
            ),
            Self::Mismatch(mismatch) => write!(f, "{mismatch}"),
        }
    }
}

impl std::error::Error for OwnershipError {}

/// A forwarded switch whose values the resolved provider's researched types
/// reject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailMismatch {
    pub provider: Provider,
    pub command_path: Vec<String>,
    /// The switch spelling (never an attached value).
    pub switch: String,
    pub kind: MismatchKind,
    /// The offending value, redacted for display; `None` when the problem is
    /// a missing value.
    pub value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MismatchKind {
    /// The switch requires a value and was given none.
    MissingValue,
    /// The switch takes no (further) value and was given one.
    ExtraValue,
    /// A variadic switch got fewer values than its researched minimum.
    TooFewValues { min: u32, given: usize },
    /// The switch takes its value only attached, and was given a separate one.
    AttachedOnly,
}

impl fmt::Display for TailMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let place = if self.command_path.is_empty() {
            "its root command".to_string()
        } else {
            format!("its `{}` command", self.command_path.join(" "))
        };
        let (provider, switch) = (self.provider, &self.switch);
        let value = self.value.as_deref().unwrap_or("");
        match self.kind {
            MismatchKind::MissingValue => write!(
                f,
                "provider argument `{switch}` takes a value for {provider} (at {place}), but \
                 none was forwarded with it.\n\
                 A `key=value` the document declares as a parameter is Claudine's, never the \
                 switch's value. "
            )?,
            MismatchKind::ExtraValue => write!(
                f,
                "provider argument `{switch}` takes no further value for {provider} (at \
                 {place}), but `{value}` was forwarded as its value because another \
                 candidate agent reads it as one. "
            )?,
            MismatchKind::TooFewValues { min, given } => write!(
                f,
                "provider argument `{switch}` needs at least {min} values for {provider} (at \
                 {place}), but {given} {} forwarded. ",
                if given == 1 { "was" } else { "were" }
            )?,
            MismatchKind::AttachedOnly => write!(
                f,
                "provider argument `{switch}` takes its value only attached for {provider} \
                 (`{switch}=…`, at {place}), but `{value}` was forwarded as a separate value. "
            )?,
        }
        write!(f, "{ESCAPE}")
    }
}

impl std::error::Error for TailMismatch {}

fn describe_value(value: SwitchValue) -> &'static str {
    match value {
        SwitchValue::None => "takes no value",
        SwitchValue::String { optional: false } => "takes one value",
        SwitchValue::String { optional: true } => "takes an optional value",
        SwitchValue::Number { .. } => "takes a number",
        SwitchValue::Variadic { .. } => "takes a list of values",
        SwitchValue::Unknown => "has no established type (takes a following word)",
    }
}

/// How one candidate reads one forwarded switch token.
#[derive(Debug, Clone, Copy)]
struct Arm {
    provider: Provider,
    value: SwitchValue,
    /// The token carries its own value (`--config=x`, `-cx`, or any
    /// `--name=value`).
    attached: bool,
    /// A separate following value is accepted.
    space: bool,
}

impl Arm {
    fn for_token(source: &impl SwitchSource, candidate: &OwnershipCandidate, token: &str) -> Self {
        let matched = source.matched(candidate.provider, &candidate.path(), token);
        match matched {
            Some(matched) if matched.switch.value != SwitchValue::Unknown => Self {
                provider: candidate.provider,
                value: matched.switch.value,
                attached: matched.attached.is_some(),
                space: matched.switch.accepts(SwitchAttachment::Space),
            },
            // Unrecognized (rule 5). A long `--name=value` already holds its
            // value, so it takes nothing more even when it is not researched.
            _ => Self {
                provider: candidate.provider,
                value: SwitchValue::Unknown,
                attached: token.starts_with("--") && token.contains('='),
                space: true,
            },
        }
    }

    /// Whether this reading takes the bare `word` as value number `taken`.
    fn takes_word(&self, word: &str, taken: usize) -> bool {
        if self.attached || !self.space {
            return false;
        }
        match self.value {
            SwitchValue::None => false,
            SwitchValue::String { .. } | SwitchValue::Unknown => taken == 0,
            SwitchValue::Number { .. } => taken == 0 && is_ownership_number(word),
            SwitchValue::Variadic { .. } => true,
        }
    }

    /// Whether this reading would take a `key=value` directly after the
    /// switch (rule 3): a string or variadic switch in space form.
    fn offers_setter(&self) -> bool {
        !self.attached
            && self.space
            && matches!(self.value, SwitchValue::String { .. } | SwitchValue::Variadic { .. })
    }
}

/// The forwarded switch whose value run is still open.
struct OpenSwitch {
    assignment: usize,
    label: String,
    arms: Vec<Arm>,
    taken: usize,
}

/// Classify `arguments` for the given candidate providers.
///
/// `candidates` must not be empty; with one candidate only its types are
/// used. The authored `--` suffix is copied into the tail unclassified.
///
/// ## Errors
///
/// [`OwnershipError`] for an `argv=` setter, a disagreement over a bare word,
/// a setter the authored schema cannot rule on, or a tail that is wrong for
/// every candidate.
pub fn own_arguments(
    arguments: &ArgumentsAfterFile,
    schema: &SchemaParameters,
    candidates: &[OwnershipCandidate],
) -> Result<OwnedArguments, OwnershipError> {
    own_in(&CompiledCatalog, arguments, schema, candidates)
}

fn own_in(
    source: &impl SwitchSource,
    arguments: &ArgumentsAfterFile,
    schema: &SchemaParameters,
    candidates: &[OwnershipCandidate],
) -> Result<OwnedArguments, OwnershipError> {
    let mut claudine = Vec::new();
    let mut implicit: Vec<String> = Vec::new();
    let mut assignments: Vec<SwitchAssignment> = Vec::new();
    let mut open: Option<OpenSwitch> = None;

    for argument in &arguments.arguments {
        let token = match argument {
            CallerArgument::ClaudineOption => {
                open = None;
                continue;
            }
            CallerArgument::Token(token) => token,
        };

        if let Some(key) = setter_key(token) {
            if key == ARGV_KEY {
                return Err(OwnershipError::ReservedArgv);
            }
            let declared = schema.declares(key);
            let offered = open
                .as_ref()
                .is_some_and(|open| open.taken == 0 && open.arms.iter().any(Arm::offers_setter));
            if declared != Some(true) && offered {
                let open = open.as_mut().expect("offered implies an open switch");
                if declared.is_none() {
                    return Err(OwnershipError::ContestedSetter {
                        switch: open.label.clone(),
                        key: key.to_string(),
                    });
                }
                take(open, &mut implicit, &mut assignments, token);
                continue;
            }
            open = None;
            claudine.push(token.clone());
            continue;
        }

        if token.starts_with('-') && token != "-" {
            let at = implicit.len();
            assignments.push(SwitchAssignment {
                switch: at,
                values: at + 1..at + 1,
            });
            implicit.push(token.clone());
            open = Some(OpenSwitch {
                assignment: assignments.len() - 1,
                label: switch_label(source, token, candidates),
                arms: candidates.iter().map(|c| Arm::for_token(source, c, token)).collect(),
                taken: 0,
            });
            continue;
        }

        let Some(current) = open.as_mut() else {
            claudine.push(token.clone());
            continue;
        };
        let readings: Vec<SwitchReading> = current
            .arms
            .iter()
            .map(|arm| SwitchReading {
                provider: arm.provider,
                value: arm.value,
                takes_word: arm.takes_word(token, current.taken),
            })
            .collect();
        if readings.iter().all(|reading| reading.takes_word) {
            take(current, &mut implicit, &mut assignments, token);
        } else if readings.iter().all(|reading| !reading.takes_word) {
            open = None;
            claudine.push(token.clone());
        } else {
            return Err(OwnershipError::Ambiguous(AmbiguousSwitch {
                switch: current.label.clone(),
                readings,
            }));
        }
    }

    let tail = ProviderTail::new(implicit, arguments.opaque.clone()).with_assignments(assignments);
    let mut first_mismatch = None;
    for candidate in candidates {
        match check_in(source, &tail, candidate.provider, &candidate.path()) {
            Ok(()) => {
                first_mismatch = None;
                break;
            }
            Err(mismatch) => {
                first_mismatch.get_or_insert(mismatch);
            }
        }
    }
    if let Some(mismatch) = first_mismatch {
        return Err(OwnershipError::Mismatch(mismatch));
    }
    Ok(OwnedArguments { claudine, tail })
}

fn take(open: &mut OpenSwitch, implicit: &mut Vec<String>, assignments: &mut [SwitchAssignment], token: &str) {
    implicit.push(token.to_string());
    assignments[open.assignment].values.end += 1;
    open.taken += 1;
}

/// A switch token as it may be shown: the researched spelling when a
/// candidate matches it, the name before `=` of a long switch, or a
/// two-character short switch. Other short tokens may carry an attached
/// value and are described without echoing it.
fn switch_label(source: &impl SwitchSource, token: &str, candidates: &[OwnershipCandidate]) -> String {
    for candidate in candidates {
        if let Some(matched) = source.matched(candidate.provider, &candidate.path(), token) {
            return matched.spelling.to_string();
        }
    }
    if token.starts_with("--") {
        return token.split_once('=').map_or(token, |(name, _)| name).to_string();
    }
    if token.chars().count() == 2 {
        return token.to_string();
    }
    "a short switch with attached text".to_string()
}

/// Check the implicit switch assignments of `tail` against `provider` at
/// `command_path`.
///
/// Only researched types fail: a switch the catalog does not establish at
/// that path, or types as unknown, is left to the provider. Explicit tokens
/// after an authored `--` carry no assignment and are never checked. A tail
/// with no recorded assignments (a direct wrapper's) always passes.
///
/// ## Errors
///
/// The first [`TailMismatch`]: a missing required value, a value a switch
/// does not take, fewer values than a variadic minimum, or a separate value
/// for a switch that takes it only attached.
pub fn check_launch_tail(
    tail: &ProviderTail,
    provider: Provider,
    command_path: &[&str],
) -> Result<(), TailMismatch> {
    check_in(&CompiledCatalog, tail, provider, command_path)
}

fn check_in(
    source: &impl SwitchSource,
    tail: &ProviderTail,
    provider: Provider,
    command_path: &[&str],
) -> Result<(), TailMismatch> {
    let args = tail.launch_args();
    for assignment in tail.assignments() {
        let token = &args[assignment.switch];
        let Some(matched) = source.matched(provider, command_path, token) else {
            continue;
        };
        let switch = matched.switch;
        let separate = assignment.values.len();
        let given = separate + usize::from(matched.attached.is_some());
        let mismatch = |kind: MismatchKind, value: Option<usize>| TailMismatch {
            provider,
            command_path: command_path.iter().map(|word| (*word).to_string()).collect(),
            switch: matched.spelling.to_string(),
            kind,
            value: value.map(|index| display_value(matched.spelling, &args[index])),
        };
        let attached_only = separate > 0 && !switch.accepts(SwitchAttachment::Space);
        match switch.value {
            SwitchValue::Unknown => {}
            SwitchValue::None if separate > 0 => {
                return Err(mismatch(MismatchKind::ExtraValue, Some(assignment.values.start)));
            }
            SwitchValue::None => {}
            SwitchValue::String { optional } | SwitchValue::Number { optional } => {
                if given == 0 && !optional {
                    return Err(mismatch(MismatchKind::MissingValue, None));
                }
                if attached_only {
                    return Err(mismatch(MismatchKind::AttachedOnly, Some(assignment.values.start)));
                }
                if given > 1 {
                    let extra = assignment.values.start + (1 - (given - separate));
                    return Err(mismatch(MismatchKind::ExtraValue, Some(extra)));
                }
            }
            SwitchValue::Variadic { min } => {
                if attached_only {
                    return Err(mismatch(MismatchKind::AttachedOnly, Some(assignment.values.start)));
                }
                if let VariadicMin::AtLeast(min) = min
                    && given < min as usize
                {
                    let kind = if given == 0 {
                        MismatchKind::MissingValue
                    } else {
                        MismatchKind::TooFewValues { min, given }
                    };
                    return Err(mismatch(kind, None));
                }
            }
        }
    }
    Ok(())
}

/// A forwarded value as an error may show it: hidden behind a sensitive
/// switch name, recognized secrets masked, and control characters escaped.
fn display_value(switch: &str, value: &str) -> String {
    if crate::secrets::is_sensitive_key_name(switch.trim_start_matches('-')) {
        return crate::secrets::MASK.to_string();
    }
    crate::secrets::mask_secrets(value)
        .chars()
        .flat_map(|ch| {
            let escaped: Vec<char> = if ch.is_control() {
                ch.escape_default().collect()
            } else {
                vec![ch]
            };
            escaped
        })
        .collect()
}

#[cfg(test)]
mod tests;
