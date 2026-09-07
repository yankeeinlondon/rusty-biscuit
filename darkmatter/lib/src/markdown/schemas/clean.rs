//! Schema-aware repair analysis for `md clean` (the Phase 5 layer).
//!
//! This module layers Darkmatter's schema-aware diagnostics on top of
//! biscuit-file's schema-agnostic engine, sharing the single diagnostic
//! vocabulary ([`YamlDiagnostic`], [`YamlRepair`], [`YamlCertainty`]) so the
//! CLI renders one uniform shape:
//!
//! - **Schema-proven scalar quoting (S2, auto-applied)** — a raw,
//!   non-coercing validation query identifies exactly one plain-scalar node
//!   failing solely because the schema requires a string; the candidate
//!   quotes the exact authored lexeme and must pass the complete effective
//!   schema. Coercion is bypassed precisely because it would hide the data
//!   loss (the number `1.2` coerces to `"1.2"`, silently dropping the
//!   authored `1.20`).
//! - **Schema-guided key correction and shape/type repair (report-only)** —
//!   undeclared keys get a did-you-mean against declared properties; missing
//!   required keys, genuine type mismatches, and constraint violations are
//!   reported with authored spans. No `default` annotations are inserted and
//!   no enum/type/parent change is guessed.
//! - **Schema-side safety for schema-independent edits (S1)** —
//!   [`schema_result_set_identical`] verifies the ratified schema half of the
//!   parse-equivalence proof when an effective schema exists.
//!
//! The [`CleanSchemaConfig`] / [`CleanSchemaContext`] pair is the
//! library-owned, clap-free configuration and resolution surface (decision
//! D7): baseline choice, explicit schema override, trigger discovery, and
//! document-path context, resolved at most once per clean run and only after
//! a non-empty frontmatter block exists.

use std::path::{Path, PathBuf};

use biscuit_file::{
    SourceSpan, YamlCertainty, YamlDiagnostic, YamlDiagnosticCode, YamlPathSegment, YamlRepair,
    locate_yaml_key, locate_yaml_value,
};
use serde_json::Value;

use super::{
    DarkmatterSchemas, EffectiveSchema, SchemaError, SimplifiedSchema, ValidationProblem,
    ValidationProblemCode, ValidationReport, validate,
};
use crate::markdown::Markdown;
use crate::markdown::compose::capture_file_resolution_context;

/// The baseline-schema choice for a clean run (decision D7).
///
/// The default is Darkmatter's built-in baseline (covering the
/// Darkmatter-owned `ctx`, `hash`, `style`, and `replace` keys); the CLI's
/// `--baseline-schema PATH` maps to [`CleanBaselineSchema::File`] and
/// `--no-baseline-schema` to [`CleanBaselineSchema::Disabled`].
#[derive(Debug, Clone, Default)]
pub enum CleanBaselineSchema {
    /// Darkmatter's built-in baseline schema.
    #[default]
    Default,
    /// No baseline at all.
    Disabled,
    /// A caller-parsed SimplifiedSchema baseline.
    Schema(SimplifiedSchema),
    /// A baseline loaded from a SimplifiedSchema (or raw JSON Schema) file.
    File(PathBuf),
}

/// Library-owned, clap-free schema configuration for a clean run.
///
/// Mirrors the ratified `md clean` schema flags without depending on clap
/// types: baseline selection, an optional document-`$schema` override, and
/// trigger-discovery opt-out. Resolution is lazy — nothing here touches the
/// filesystem until [`Self::resolve`] is called, and callers resolve only
/// after a non-empty frontmatter block exists (the performance contract).
#[derive(Debug, Clone, Default)]
pub struct CleanSchemaConfig {
    baseline: CleanBaselineSchema,
    schema_override: Option<Value>,
    trigger_schemas: bool,
}

impl CleanSchemaConfig {
    /// The default configuration: Darkmatter baseline on, trigger discovery
    /// on (for file-backed documents).
    #[must_use]
    pub fn new() -> Self {
        Self {
            baseline: CleanBaselineSchema::Default,
            schema_override: None,
            trigger_schemas: true,
        }
    }

    /// Replaces the default baseline with a caller-parsed SimplifiedSchema
    /// (the library form of `--baseline-schema PATH`).
    #[must_use]
    pub fn with_baseline_schema(mut self, schema: SimplifiedSchema) -> Self {
        self.baseline = CleanBaselineSchema::Schema(schema);
        self
    }

    /// Replaces the default baseline with a schema loaded from `path`.
    #[must_use]
    pub fn with_baseline_schema_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.baseline = CleanBaselineSchema::File(path.into());
        self
    }

    /// Disables the baseline entirely (`--no-baseline-schema`).
    #[must_use]
    pub fn without_baseline_schema(mut self) -> Self {
        self.baseline = CleanBaselineSchema::Disabled;
        self
    }

    /// Replaces the document's own `$schema` with the given schema value —
    /// an operator override matching `md schema validate --schema`'s
    /// explicit-schema posture. The value takes the same shapes a `$schema`
    /// frontmatter value takes: a string file reference or bare name
    /// (`Value::String`), an inline mapping, or a root-union sequence, and
    /// resolves against the document's own base directory and trigger schema
    /// roots.
    #[must_use]
    pub fn with_schema_override(mut self, reference: Value) -> Self {
        self.schema_override = Some(reference);
        self
    }

    /// Enables or disables repository trigger-schema discovery and bare-name
    /// schema-root lookup (`--no-trigger-schemas`).
    #[must_use]
    pub fn with_trigger_schemas(mut self, enabled: bool) -> Self {
        self.trigger_schemas = enabled;
        self
    }

    /// Resolves the configuration into a per-run [`CleanSchemaContext`].
    ///
    /// `document_path` is the resolved Markdown file path when the input is
    /// file-backed; trigger discovery is silently inert without it (stdin
    /// parity with compose's non-file sources). Discovery walks from the
    /// document to the repository root captured at this command boundary.
    /// At most one context is built per clean invocation.
    ///
    /// ## Errors
    ///
    /// Propagates [`SchemaError`] from baseline loading/validation and
    /// trigger-envelope scanning.
    pub fn resolve(&self, document_path: Option<&Path>) -> Result<CleanSchemaContext, SchemaError> {
        let mut builder = DarkmatterSchemas::new();
        builder = match &self.baseline {
            CleanBaselineSchema::Default => builder.with_darkmatter_baseline_json_schema()?,
            CleanBaselineSchema::Disabled => builder,
            CleanBaselineSchema::Schema(schema) => builder.with_baseline(schema.clone())?,
            CleanBaselineSchema::File(path) => builder.with_baseline_from_file(path)?,
        };
        if self.trigger_schemas
            && let Some(path) = document_path
            && let Some(boundary) = capture_file_resolution_context(
                path.parent().unwrap_or(path),
            )
                .repository_root()
                .map(Path::to_path_buf)
        {
            builder = builder.with_trigger_discovery(path, boundary)?;
        }
        Ok(CleanSchemaContext {
            schemas: builder,
            schema_override: self.schema_override.clone(),
        })
    }
}

/// The resolved per-run schema context for `md clean`.
///
/// Holds the configured [`DarkmatterSchemas`] (baseline + trigger registry +
/// shared validator cache) and the optional document-`$schema` override.
/// Build once per clean invocation and reuse for every schema-aware query.
#[derive(Clone)]
pub struct CleanSchemaContext {
    schemas: DarkmatterSchemas,
    schema_override: Option<Value>,
}

impl CleanSchemaContext {
    /// The underlying schemas handle (baseline, triggers, validator cache).
    pub fn schemas(&self) -> &DarkmatterSchemas {
        &self.schemas
    }

    /// Resolves the effective schema for `source`, applying the configured
    /// document-`$schema` override when present. Returns `Ok(None)` when no
    /// baseline, no matching trigger, and no document schema constrain the
    /// document — the schema-aware tier is then silent for every key
    /// (decision D8).
    ///
    /// ## Errors
    ///
    /// Propagates [`SchemaError`] from schema resolution and validator
    /// construction.
    pub fn effective_schema(&self, source: &Markdown) -> Result<Option<EffectiveSchema>, SchemaError> {
        self.schemas
            .effective_for_with_override(source, self.schema_override.as_ref())
    }

    /// Runs schema-aware analysis over one parseable frontmatter block.
    ///
    /// `yaml_source` is the raw YAML text between the frontmatter delimiters
    /// (post schema-agnostic repair); `source` is the parsed document,
    /// retaining the full document path context relative `$schema` files and
    /// trigger matching need. Unparseable `yaml_source` yields an empty
    /// analysis — the schema-agnostic layer owns that diagnostic class.
    ///
    /// ## Errors
    ///
    /// Propagates [`SchemaError`] from effective-schema resolution.
    pub fn analyze(
        &self,
        source: &Markdown,
        yaml_source: &str,
    ) -> Result<SchemaCleanAnalysis, SchemaError> {
        let Some(effective) = self.effective_schema(source)? else {
            return Ok(SchemaCleanAnalysis::default());
        };
        Ok(analyze_frontmatter(&effective, yaml_source).with_effective(effective))
    }
}

/// The outcome of schema-aware analysis of one frontmatter block: shared
/// diagnostics plus the resolved effective schema for downstream safety
/// checks ([`schema_result_set_identical`]) and final validation.
#[derive(Clone, Default)]
pub struct SchemaCleanAnalysis {
    effective: Option<EffectiveSchema>,
    diagnostics: Vec<YamlDiagnostic>,
}

impl SchemaCleanAnalysis {
    fn with_effective(mut self, effective: EffectiveSchema) -> Self {
        self.effective = Some(effective);
        self
    }

    /// The effective schema the analysis ran against, when one constrains
    /// the document.
    pub fn effective(&self) -> Option<&EffectiveSchema> {
        self.effective.as_ref()
    }

    /// The schema-aware diagnostics in stable source order.
    pub fn diagnostics(&self) -> &[YamlDiagnostic] {
        &self.diagnostics
    }

    /// `true` when no schema-aware findings were produced.
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Consumes the analysis into its diagnostics.
    pub fn into_diagnostics(self) -> Vec<YamlDiagnostic> {
        self.diagnostics
    }
}

/// Runs schema-aware analysis over one frontmatter block against an already
/// resolved effective schema.
///
/// When the raw query finds exactly one problem and it is a plain-scalar
/// node failing solely because a string is required, the result is a single
/// deterministic `schema.type-mismatch` diagnostic carrying the proven
/// quoting repair. Otherwise every finding is report-only.
#[must_use]
pub fn analyze_frontmatter(
    effective: &EffectiveSchema,
    yaml_source: &str,
) -> SchemaCleanAnalysis {
    let Some(instance) = parse_instance(yaml_source) else {
        return SchemaCleanAnalysis::default();
    };
    let raw = effective.validate_raw(&instance);

    if raw.problems.len() == 1
        && let Some(diagnostic) =
            schema_proven_quoting(effective, yaml_source, &instance, &raw.problems[0])
    {
        return SchemaCleanAnalysis {
            effective: None,
            diagnostics: vec![diagnostic],
        };
    }

    let mut diagnostics: Vec<YamlDiagnostic> = Vec::new();
    // Raw string-required mismatches the S2 proof rejected: the problem is
    // certain, the repair is not — report-only with no candidate repairs.
    let mut covered_paths: Vec<String> = Vec::new();
    for problem in &raw.problems {
        if is_string_required_mismatch(effective, &instance, problem) {
            covered_paths.push(problem.path.clone());
            diagnostics.push(raw_type_mismatch_diagnostic(yaml_source, problem));
        }
    }
    // Compose-parity coerced report: problems that survive coercion are real
    // under shared schema semantics and map onto report-only suggestions.
    let coerced = effective.validate(&instance);
    for problem in &coerced.problems {
        if covered_paths.contains(&problem.path) {
            continue;
        }
        diagnostics.push(report_only_diagnostic(effective, yaml_source, problem));
    }
    sort_diagnostics(&mut diagnostics);
    SchemaCleanAnalysis {
        effective: None,
        diagnostics,
    }
}

/// Verifies the ratified schema half of the S1 parse-equivalence proof: when
/// an effective schema exists, a schema-independent candidate edit is
/// auto-apply eligible only when the candidate's schema result set is
/// identical to the original's. Both sources must parse; any parse failure
/// fails the proof.
///
/// Identity is required on **both** validation halves, each keyed on
/// `(path, code, arm_index)`: the coercing set (the same result set compose
/// observes) and the raw, non-coercing set. The raw half is load-bearing —
/// coercion maps an authored `42` and an authored `"42"` onto the same
/// problem set, so the coerced comparison alone cannot see a type-changing
/// edit and would wrongly report it as unchanged. The coerced half is
/// retained because a raw-identical pair can still diverge under coercion
/// (`"42"` and `"abc"` against `number` are both raw mismatches, but only
/// the first coerces).
///
/// This check is for schema-**independent** (parse-equivalent) candidates
/// only; it must never gate the explicitly invalid-to-valid schema-quoting
/// transition (S2), which intentionally shrinks the raw problem set.
#[must_use]
pub fn schema_result_set_identical(
    effective: &EffectiveSchema,
    original_source: &str,
    candidate_source: &str,
) -> bool {
    let (Some(original), Some(candidate)) = (
        parse_instance(original_source),
        parse_instance(candidate_source),
    ) else {
        return false;
    };
    problem_set(&effective.validate_raw(&original))
        == problem_set(&effective.validate_raw(&candidate))
        && problem_set(&effective.validate(&original)) == problem_set(&effective.validate(&candidate))
}

/// The `(path, code, arm_index)` multiset of a validation report, sorted for
/// order-independent comparison.
fn problem_set(report: &ValidationReport) -> Vec<(String, ValidationProblemCode, Option<usize>)> {
    let mut set: Vec<_> = report
        .problems
        .iter()
        .map(|problem| (problem.path.clone(), problem.code, problem.arm_index))
        .collect();
    set.sort();
    set
}

/// Parses raw frontmatter YAML into the JSON instance validators consume,
/// stripping the `$schema` control key exactly as
/// `frontmatter_as_json` does. `None` when the source does not parse (the
/// schema-agnostic layer owns that class) or is empty.
fn parse_instance(yaml_source: &str) -> Option<Value> {
    if yaml_source.trim().is_empty() {
        return None;
    }
    let mut value: Value = serde_yaml_ng::from_str(yaml_source).ok()?;
    if let Value::Object(map) = &mut value {
        map.remove("$schema");
    }
    Some(value)
}

/// The S2 proof: returns a deterministic diagnostic with the quoting repair
/// when (and only when) `problem` is the document's sole raw problem, a
/// bool/number plain-scalar node failing solely because the schema requires
/// a string, and quoting the exact authored lexeme passes the complete
/// effective schema.
fn schema_proven_quoting(
    effective: &EffectiveSchema,
    yaml_source: &str,
    instance: &Value,
    problem: &ValidationProblem,
) -> Option<YamlDiagnostic> {
    if !is_string_required_mismatch(effective, instance, problem) {
        return None;
    }
    let path = instance_path_segments(problem);
    let located = locate_yaml_value(yaml_source, &path)?;
    if !located.plain {
        return None;
    }
    let lexeme = &yaml_source[located.span.clone()];
    // A single-token plain scalar, double-quote-safe without escapes — the
    // same bounded shape biscuit-file's S3 grammar ratifies. Anchored,
    // tagged, or otherwise decorated values carry whitespace and stop here.
    if lexeme.is_empty()
        || lexeme.bytes().any(|byte| matches!(byte, b' ' | b'\t'))
        || lexeme.contains('"')
        || lexeme.contains('\\')
    {
        return None;
    }
    // The lexeme must reparse to exactly the instance node, proving the
    // located bytes are the node the schema rejected (not a folded
    // continuation or a sibling token).
    let reparsed: Value = serde_yaml_ng::from_str(lexeme).ok()?;
    let node = instance_node_at(instance, problem.instance_path.segments())?;
    if reparsed != *node {
        return None;
    }
    // Candidate: quote exactly the lexeme and nothing else.
    let mut candidate = String::with_capacity(yaml_source.len() + 2);
    candidate.push_str(&yaml_source[..located.span.start]);
    candidate.push('"');
    candidate.push_str(lexeme);
    candidate.push('"');
    candidate.push_str(&yaml_source[located.span.end..]);
    let candidate_instance = parse_instance(&candidate)?;
    // The candidate must parse and pass the **complete** effective schema;
    // with a single original problem, zero remaining problems is the strict
    // subset (no unrelated regression) the safety matrix requires. Value
    // equality is deliberately not required — the type change is the point.
    if !effective.validate_raw(&candidate_instance).valid {
        return None;
    }
    let kind = match node {
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        _ => "a non-string scalar",
    };
    Some(YamlDiagnostic {
        code: YamlDiagnosticCode::SchemaTypeMismatch,
        span: located.span.clone(),
        classification: YamlCertainty::Deterministic,
        message: format!(
            "value at `{}` is {kind} but the schema requires a string",
            problem.path
        ),
        repairs: vec![YamlRepair {
            span: located.span,
            replacement: format!("\"{lexeme}\""),
            explanation:
                "quote the exact authored scalar so the schema-required string preserves the source text"
                    .to_string(),
        }],
    })
}

/// `true` when `problem` is a type mismatch on a bool/number scalar node
/// whose schema requires a string — the S2 failure family, whether or not
/// the full auto-apply proof succeeds.
fn is_string_required_mismatch(
    effective: &EffectiveSchema,
    instance: &Value,
    problem: &ValidationProblem,
) -> bool {
    if problem.code != ValidationProblemCode::TypeMismatch {
        return false;
    }
    let Some(node) = instance_node_at(instance, problem.instance_path.segments()) else {
        return false;
    };
    if !matches!(node, Value::Bool(_) | Value::Number(_)) {
        return false;
    }
    schema_requires_string(effective, problem)
}

/// Walks the compiled schema to the property node `problem` failed against
/// and reports whether that node requires the JSON `string` type — solely.
/// Nullable wrappers (`anyOf: [null, T]`) are unwrapped; a union with two or
/// more non-null arms is never "solely string".
fn schema_requires_string(effective: &EffectiveSchema, problem: &ValidationProblem) -> bool {
    let mut node = match problem.arm_index {
        Some(index) => effective
            .json_schema
            .get("anyOf")
            .and_then(Value::as_array)
            .and_then(|arms| arms.get(index))
            .unwrap_or(&effective.json_schema),
        None => &effective.json_schema,
    };
    for segment in problem.instance_path.segments() {
        node = validate::unwrap_nullable_arm(node);
        let next = if segment.bytes().all(|byte| byte.is_ascii_digit()) {
            node.get("items")
        } else {
            node.get("properties").and_then(|props| props.get(segment))
        };
        let Some(next) = next else { return false };
        node = next;
    }
    let node = validate::unwrap_nullable_arm(node);
    if node.get("anyOf").is_some() {
        return false;
    }
    node.get("type").and_then(Value::as_str) == Some("string")
}

/// Navigates the JSON instance along decoded pointer segments.
fn instance_node_at<'a>(instance: &'a Value, segments: &[String]) -> Option<&'a Value> {
    let mut node = instance;
    for segment in segments {
        node = match node {
            Value::Object(map) => map.get(segment)?,
            Value::Array(items) => items.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(node)
}

/// Converts a problem's decoded instance path into scanner path segments:
/// all-numeric segments are sequence indices, everything else is a mapping
/// key.
fn instance_path_segments(problem: &ValidationProblem) -> Vec<YamlPathSegment> {
    problem
        .instance_path
        .segments()
        .iter()
        .map(|segment| match segment.parse::<usize>() {
            Ok(index) => YamlPathSegment::Index(index),
            Err(_) => YamlPathSegment::Key(segment.clone()),
        })
        .collect()
}

/// A report-only `schema.type-mismatch` for a raw string-required mismatch
/// whose repair could not be proven (unlocatable lexeme, flow/quoted value,
/// or a candidate that fails the complete schema).
fn raw_type_mismatch_diagnostic(yaml_source: &str, problem: &ValidationProblem) -> YamlDiagnostic {
    YamlDiagnostic {
        code: YamlDiagnosticCode::SchemaTypeMismatch,
        span: value_span_for(yaml_source, problem),
        classification: YamlCertainty::DeterministicFindNonDeterministicSolution,
        message: format!(
            "{}; the authored scalar could not be proven safe to quote",
            problem.message
        ),
        repairs: Vec::new(),
    }
}

/// Maps a problem from the compose-parity (coerced) report onto its
/// report-only suggestion diagnostic.
fn report_only_diagnostic(
    effective: &EffectiveSchema,
    yaml_source: &str,
    problem: &ValidationProblem,
) -> YamlDiagnostic {
    match problem.code {
        ValidationProblemCode::UnknownKey => unknown_key_diagnostic(effective, yaml_source, problem),
        ValidationProblemCode::MissingRequired => YamlDiagnostic {
            code: YamlDiagnosticCode::SchemaShapeRepair,
            span: missing_key_span_for(yaml_source, problem),
            classification: YamlCertainty::DeterministicFindNonDeterministicSolution,
            message: problem.message.clone(),
            repairs: Vec::new(),
        },
        ValidationProblemCode::TypeMismatch => YamlDiagnostic {
            code: YamlDiagnosticCode::SchemaTypeMismatch,
            span: value_span_for(yaml_source, problem),
            classification: YamlCertainty::DeterministicFindNonDeterministicSolution,
            message: problem.message.clone(),
            repairs: Vec::new(),
        },
        _ => YamlDiagnostic {
            code: YamlDiagnosticCode::SchemaShapeRepair,
            span: value_span_for(yaml_source, problem),
            classification: YamlCertainty::DeterministicFindNonDeterministicSolution,
            message: problem.message.clone(),
            repairs: Vec::new(),
        },
    }
}

/// The report-only key-correction diagnostic for an undeclared key, with a
/// did-you-mean against the keys the schema declares at the same object when
/// a near match exists.
fn unknown_key_diagnostic(
    effective: &EffectiveSchema,
    yaml_source: &str,
    problem: &ValidationProblem,
) -> YamlDiagnostic {
    let offending = problem.offending_property.clone().unwrap_or_default();
    let mut key_path = instance_path_segments(problem);
    if !offending.is_empty() {
        key_path.push(YamlPathSegment::Key(offending.clone()));
    }
    let span = locate_yaml_key(yaml_source, &key_path)
        .or_else(|| locate_yaml_key(yaml_source, &instance_path_segments(problem)))
        .unwrap_or(0..0);
    let message = match nearest_declared_key(effective, problem, &offending) {
        Some(suggestion) => format!(
            "key `{offending}` is not declared by the schema; did you mean `{suggestion}`?"
        ),
        None => format!("key `{offending}` is not declared by the schema"),
    };
    YamlDiagnostic {
        code: YamlDiagnosticCode::SchemaKeyCorrection,
        span,
        classification: YamlCertainty::DeterministicFindNonDeterministicSolution,
        message,
        repairs: Vec::new(),
    }
}

/// The nearest declared key to `offending` at the object `problem` points
/// at, using the same Damerau distance and thresholds as biscuit-file's
/// similar-key lint.
fn nearest_declared_key(
    effective: &EffectiveSchema,
    problem: &ValidationProblem,
    offending: &str,
) -> Option<String> {
    if offending.is_empty() {
        return None;
    }
    let mut node = match problem.arm_index {
        Some(index) => effective
            .json_schema
            .get("anyOf")
            .and_then(Value::as_array)
            .and_then(|arms| arms.get(index))
            .unwrap_or(&effective.json_schema),
        None => &effective.json_schema,
    };
    for segment in problem.instance_path.segments() {
        node = validate::unwrap_nullable_arm(node);
        node = if segment.bytes().all(|byte| byte.is_ascii_digit()) {
            node.get("items")
        } else {
            node.get("properties").and_then(|props| props.get(segment))
        }?;
    }
    let node = validate::unwrap_nullable_arm(node);
    let declared: Vec<&str> = node
        .get("properties")
        .and_then(Value::as_object)
        .map(|props| props.keys().map(String::as_str).collect())
        .unwrap_or_default();
    declared
        .into_iter()
        .filter(|candidate| near_key_match(candidate, offending))
        .min_by_key(|candidate| edit_distance(candidate, offending))
        .map(str::to_string)
}

/// Two distinct key spellings are suspiciously close: lengths under three
/// never match, lengths three and four tolerate one edit, longer keys
/// tolerate two; case and `-`/`_` separator variants always match. Mirrors
/// biscuit-file's similar-key threshold so suggestion behavior agrees across
/// the shared diagnostics.
fn near_key_match(first: &str, second: &str) -> bool {
    if first == second {
        return false;
    }
    let length = first.chars().count().max(second.chars().count());
    if length < 3 {
        return false;
    }
    let normalize = |text: &str| text.to_lowercase().replace('_', "-");
    if normalize(first) == normalize(second) {
        return true;
    }
    let distance = edit_distance(first, second);
    (length >= 5 && distance <= 2) || distance <= 1
}

/// Optimal string alignment (Damerau with unit costs): insertions,
/// deletions, substitutions, and adjacent transpositions.
fn edit_distance(first: &str, second: &str) -> usize {
    let a: Vec<char> = first.chars().collect();
    let b: Vec<char> = second.chars().collect();
    let (rows, cols) = (a.len() + 1, b.len() + 1);
    let mut distances = vec![vec![0usize; cols]; rows];
    for (row, cell) in distances.iter_mut().enumerate().take(rows) {
        cell[0] = row;
    }
    for (col, cell) in distances[0].iter_mut().enumerate().take(cols) {
        *cell = col;
    }
    for row in 1..rows {
        for col in 1..cols {
            let cost = usize::from(a[row - 1] != b[col - 1]);
            distances[row][col] = (distances[row - 1][col] + 1)
                .min(distances[row][col - 1] + 1)
                .min(distances[row - 1][col - 1] + cost);
            if row > 1 && col > 1 && a[row - 1] == b[col - 2] && a[row - 2] == b[col - 1] {
                distances[row][col] = distances[row][col].min(distances[row - 2][col - 2] + 1);
            }
        }
    }
    distances[rows - 1][cols - 1]
}

/// The authored span of the value `problem` points at, or the zero-width
/// document-start span when the node cannot be located (flow context,
/// unrepresentable path).
fn value_span_for(yaml_source: &str, problem: &ValidationProblem) -> SourceSpan {
    locate_yaml_value(yaml_source, &instance_path_segments(problem))
        .map(|located| located.span)
        .unwrap_or(0..0)
}

/// The authored span anchoring a missing-required problem: the parent
/// mapping's key when it can be located, else the zero-width document-start
/// span (the message names the missing property).
fn missing_key_span_for(yaml_source: &str, problem: &ValidationProblem) -> SourceSpan {
    locate_yaml_key(yaml_source, &instance_path_segments(problem)).unwrap_or(0..0)
}

/// Stable source order for mixed diagnostic sets: span start, span end, then
/// diagnostic code for total determinism.
fn sort_diagnostics(diagnostics: &mut [YamlDiagnostic]) {
    diagnostics.sort_by(|first, second| {
        (first.span.start, first.span.end, first.code.as_str()).cmp(&(
            second.span.start,
            second.span.end,
            second.code.as_str(),
        ))
    });
}
