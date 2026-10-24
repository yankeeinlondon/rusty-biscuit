//! Request value rules compiled into `schematic_runtime::validate` plans.
//!
//! A request's rules come from two places: the [`Constraints`] a definition
//! attaches to parameters, form fields, and a body root, and the stored
//! schema components a JSON body or JSON form field references. Both are
//! compiled into one plan tree per checked value. Each API module gets one
//! private plans module holding every plan as a `static`, so a component
//! reached from several requests is emitted once and a recursive model
//! refers to its own `static` instead of expanding forever.
//!
//! A rule the client cannot enforce is a [`Finding`]: a format (an
//! annotation only), a pattern outside the enforced subset, or a rule
//! beneath a composition or a keyword the plans do not enter (see
//! `unenforceable_rules`). Findings become generation diagnostics and
//! rustdoc labels; a rule beneath a composition is an error unless it
//! sits inside a member of a union that is itself a component, the shape
//! of a Rust enum type. Rules Schemars writes for a Rust enum's own variants
//! (the string `enum`/`const` of a unit enum or of a tag) and for an
//! integer width (`minimum: 0` on a `uint32`) are guaranteed by the Rust
//! type and are neither enforced nor reported.
//!
//! The behavior is described for users in
//! `schematic/docs/generation/request-side.md#value-rules`.

use std::collections::{BTreeSet, HashMap, HashSet};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::constraints::{
    Constraints, PatternAdmission, UnenforceableRule, admit_pattern, admits_only_null, is_width_bound,
    referenced_component, unenforceable_rules,
};
use schematic_define::{ApiRequest, Endpoint, FormFieldKind, QueryParamType, RestApi, SchemaComponent, TypeRef};
use serde_json::{Map, Value};

use crate::codegen::request_structs::path_params::{PathField, PathValue, typed_path_fields};
use crate::codegen::request_structs::query_params::{QueryForm, QueryScalar};
use crate::codegen::request_structs::shared::{
    QueryParamInfo, extract_cookie_params, extract_header_params, extract_query_params, param_field_name,
};
use crate::diagnostics::Severity;

/// Formats that name a numeric width, which the Rust type already states.
const NUMERIC_FORMATS: [&str; 4] = ["int32", "int64", "float", "double"];

/// A rule of one request that the generated client does not enforce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    /// `Error` blocks the Rust client: a supported rule the client would
    /// otherwise silently skip.
    pub severity: Severity,
    /// Where the rule is: `query.limit`, `body`, or a component location.
    pub place: String,
    pub keyword: String,
    pub reason: String,
}

impl Finding {
    pub(crate) fn message(&self) -> String {
        format!("{} `{}` is not enforced: {}", self.place, self.keyword, self.reason)
    }
}

/// Every rule of `api`'s requests that the Rust client does not enforce,
/// with the endpoint whose parameter or body declares it, or `None` for a
/// pattern inside a component, reported once however many requests reach
/// it.
///
/// `api` is the API as the Rust client sees it: its pager view, without
/// the parameters pagination adopted.
pub(crate) fn api_findings(api: &RestApi) -> Vec<(Option<String>, Finding)> {
    let mut rules = ApiValueRules::new(api);
    let mut findings = Vec::new();
    for endpoint in &api.endpoints {
        let request_type = endpoint.id.clone();
        let query = extract_query_params(&endpoint.params, &request_type);
        let headers = extract_header_params(&endpoint.params);
        let cookies = extract_cookie_params(&endpoint.params);
        let typed = typed_path_fields(endpoint, &request_type);
        let request = rules.request(endpoint, &request_type, typed.as_deref(), &query, &headers, &cookies);
        findings.extend(request.own_findings.into_iter().map(|finding| (Some(endpoint.id.clone()), finding)));
    }
    let mut components: Vec<usize> = rules.component_findings.keys().copied().collect();
    components.sort_unstable();
    for index in components {
        findings.extend(rules.component_findings[&index].iter().cloned().map(|finding| (None, finding)));
    }
    findings
}

/// The rustdoc lines that label a parameter's or form field's rules:
/// those every send checks, and those it does not (formats, which are
/// annotations only, and patterns outside the enforced subset). Empty
/// without rules.
pub(crate) fn rule_doc_lines(rules: &Constraints) -> Vec<String> {
    let mut checked = Vec::new();
    let mut unchecked = Vec::new();
    describe(rules, "", &mut checked, &mut unchecked);
    let mut lines = Vec::new();
    if !checked.is_empty() {
        lines.push(format!(" Value rules checked before sending: {}.", checked.join(", ")));
    }
    if !unchecked.is_empty() {
        lines.push(format!(" Not checked: {}.", unchecked.join("; ")));
    }
    lines
}

fn describe(rules: &Constraints, prefix: &str, checked: &mut Vec<String>, unchecked: &mut Vec<String>) {
    for (keyword, count) in [("minLength", rules.min_length), ("maxLength", rules.max_length)] {
        if let Some(count) = count {
            checked.push(format!("{prefix}`{keyword}` {count}"));
        }
    }
    if let Some(source) = &rules.pattern {
        let enforced = !rules.pattern_unenforced && matches!(admit_pattern(source), PatternAdmission::Enforced { .. });
        if enforced {
            checked.push(format!("{prefix}`pattern`"));
        } else {
            unchecked.push(format!("{prefix}`pattern` (outside the enforced subset)"));
        }
    }
    if let Some(format) = &rules.format {
        unchecked.push(format!("{prefix}`format` `{format}` (an annotation only)"));
    }
    for (keyword, number) in [
        ("minimum", &rules.minimum),
        ("exclusiveMinimum", &rules.exclusive_minimum),
        ("maximum", &rules.maximum),
        ("exclusiveMaximum", &rules.exclusive_maximum),
        ("multipleOf", &rules.multiple_of),
    ] {
        if let Some(number) = number {
            checked.push(format!("{prefix}`{keyword}` {}", number.as_str()));
        }
    }
    for (keyword, count) in [("minItems", rules.min_items), ("maxItems", rules.max_items)] {
        if let Some(count) = count {
            checked.push(format!("{prefix}`{keyword}` {count}"));
        }
    }
    if rules.unique_items {
        checked.push(format!("{prefix}`uniqueItems`"));
    }
    if let Some(values) = &rules.allowed_values {
        let noun = if values.len() == 1 { "value" } else { "values" };
        checked.push(format!("{prefix}`enum` ({} {noun})", values.len()));
    }
    if rules.constant.is_some() {
        checked.push(format!("{prefix}`const`"));
    }
    if let Some(items) = &rules.items {
        describe(items, &format!("{prefix}each item's "), checked, unchecked);
    }
}

/// One value's rules, mirroring `schematic_runtime::validate::Plan`.
#[derive(Debug, Clone, Default, PartialEq)]
struct Node {
    nullable: bool,
    min_length: Option<u64>,
    max_length: Option<u64>,
    /// Translated Rust `regex` source.
    pattern: Option<String>,
    minimum: Option<String>,
    exclusive_minimum: Option<String>,
    maximum: Option<String>,
    exclusive_maximum: Option<String>,
    multiple_of: Option<String>,
    min_items: Option<u64>,
    max_items: Option<u64>,
    unique_items: bool,
    allowed: Option<Vec<Value>>,
    constant: Option<Value>,
    items: Option<Box<Node>>,
    /// Sorted by serialized name.
    properties: Vec<(String, Node)>,
    values: Option<Box<Node>>,
    reference: Option<Target>,
}

#[derive(Debug, Clone, PartialEq)]
enum Target {
    Component(usize),
    Inline(Box<Node>),
}

impl Node {
    fn has_own_rules(&self) -> bool {
        self.min_length.is_some()
            || self.max_length.is_some()
            || self.pattern.is_some()
            || self.minimum.is_some()
            || self.exclusive_minimum.is_some()
            || self.maximum.is_some()
            || self.exclusive_maximum.is_some()
            || self.multiple_of.is_some()
            || self.min_items.is_some()
            || self.max_items.is_some()
            || self.unique_items
            || self.allowed.is_some()
            || self.constant.is_some()
    }

    /// The node with only the rules a JSON array check needs beyond item
    /// counts: uniqueness and whole-value equality.
    fn whole_array_rules(&self) -> Node {
        Node {
            unique_items: self.unique_items,
            allowed: self.allowed.clone(),
            constant: self.constant.clone(),
            ..Node::default()
        }
    }
}

struct ComponentPlan<'a> {
    source: &'a SchemaComponent,
    node: Node,
}

/// The plans of one API's requests and the components they reach.
pub(crate) struct ApiValueRules<'a> {
    api: &'a RestApi,
    components: Vec<ComponentPlan<'a>>,
    index: HashMap<String, usize>,
    /// Pattern findings of each compiled component, reported for every
    /// request that reaches the component.
    component_findings: HashMap<usize, Vec<Finding>>,
    findings: Vec<Finding>,
    /// Emitted request-level plans, as `static` items.
    statics: Vec<TokenStream>,
    names: HashSet<String>,
    patterns: Vec<(String, proc_macro2::Ident)>,
    used: BTreeSet<usize>,
    live: Option<Vec<bool>>,
}

/// The checks of one request, ready to splice into its generated code.
#[derive(Default)]
pub(crate) struct RequestRules {
    /// Statements checking parameters and text values of `self` into
    /// `violations`; they never fail.
    pub field_checks: TokenStream,
    pub body: BodyRules,
    /// Rules not enforced, for the request's rustdoc: its own, then those
    /// of the components its body reaches.
    pub findings: Vec<Finding>,
    /// The request's own findings, without its components'.
    own_findings: Vec<Finding>,
}

impl RequestRules {
    /// Whether the request checks anything, and so gets `validate()`.
    pub(crate) fn enforces(&self) -> bool {
        !self.field_checks.is_empty() || !matches!(self.body, BodyRules::Unchecked)
    }
}

/// How a request's JSON values are checked.
#[derive(Default)]
pub(crate) enum BodyRules {
    /// Nothing JSON-valued is checked; the body is sent as before.
    #[default]
    Unchecked,
    /// The JSON body is serialized once, checked against `plan`, and that
    /// value is sent.
    Json { plan: TokenStream },
    /// Multipart JSON fields, each serialized once, checked, and sent as
    /// checked: `(field, wire name, required, plan)`.
    FormJson(Vec<(String, String, bool, TokenStream)>),
}

impl<'a> ApiValueRules<'a> {
    pub(crate) fn new(api: &'a RestApi) -> Self {
        Self {
            api,
            components: Vec::new(),
            index: HashMap::new(),
            component_findings: HashMap::new(),
            findings: Vec::new(),
            statics: Vec::new(),
            names: HashSet::new(),
            patterns: Vec::new(),
            used: BTreeSet::new(),
            live: None,
        }
    }

    /// The private module that holds this API's plans.
    pub(crate) fn module_ident(&self) -> proc_macro2::Ident {
        format_ident!("{}_value_rules", snake_case(&self.api.name))
    }

    /// Compiles the checks of `endpoint`'s request type `struct_name`.
    ///
    /// `typed_paths`, `query`, `headers`, and `cookies` are the request's
    /// fields, in declaration order; their rules are checked in that order,
    /// then the body's.
    pub(crate) fn request(
        &mut self,
        endpoint: &Endpoint,
        struct_name: &str,
        typed_paths: Option<&[PathField]>,
        query: &[QueryParamInfo],
        headers: &[QueryParamInfo],
        cookies: &[QueryParamInfo],
    ) -> RequestRules {
        let before = self.findings.len();
        let mut reached = BTreeSet::new();
        let mut checks = TokenStream::new();
        let prefix = screaming_case(struct_name);

        for field in typed_paths.unwrap_or_default() {
            let declared = endpoint.params.as_ref().and_then(|params| params.path.iter().find(|param| param.name == field.wire));
            let Some(rules) = declared.and_then(|param| param.constraints.as_deref()) else { continue };
            let place = format!("path.{}", field.field);
            let node = self.constraint_node(rules, &place);
            let scalar = match &field.value {
                PathValue::String => Scalar::Str,
                PathValue::Integer | PathValue::Number => Scalar::Number,
                PathValue::Boolean => Scalar::Bool,
                PathValue::Enum { .. } => Scalar::StrEnum,
            };
            checks.extend(self.param_check(&prefix, "path", &field.field, true, Shape { scalar, array: false }, &node));
        }
        for param in query {
            let Some(rules) = param.constraints.as_deref() else { continue };
            let field = param.field_name();
            let node = self.constraint_node(rules, &format!("query.{field}"));
            let Some(shape) = &param.shape else { continue };
            let scalar = match &shape.scalar {
                QueryScalar::String => Scalar::Str,
                QueryScalar::Integer | QueryScalar::Number => Scalar::Number,
                QueryScalar::Boolean => Scalar::Bool,
                QueryScalar::Enum { .. } => Scalar::StrEnum,
            };
            let shape = Shape { scalar, array: shape.form != QueryForm::Single };
            checks.extend(self.param_check(&prefix, "query", &field, param.required, shape, &node));
        }
        for (location, params) in [("header", headers), ("cookie", cookies)] {
            for param in params {
                let Some(rules) = param.constraints.as_deref() else { continue };
                let field = param.field_name();
                let node = self.constraint_node(rules, &format!("{location}.{field}"));
                let shape = named_shape(&param.param_type);
                checks.extend(self.param_check(&prefix, location, &field, false, shape, &node));
            }
        }

        let body = match &endpoint.request {
            Some(ApiRequest::Json(reference)) => {
                let node = self.body_node(reference, endpoint.request_constraints.as_deref(), "body");
                self.reachable(&node, &mut reached);
                match self.emit_static(&format!("{prefix}_BODY"), node) {
                    Some(plan) => BodyRules::Json { plan },
                    None => BodyRules::Unchecked,
                }
            }
            Some(ApiRequest::Text { .. }) => {
                if let Some(rules) = endpoint.request_constraints.as_deref() {
                    let node = self.constraint_node(rules, "body");
                    if let Some(plan) = self.emit_static(&format!("{prefix}_BODY"), node) {
                        checks.extend(quote! {
                            schematic_runtime::validate::check_str(
                                &self.body, &#plan, &[schematic_runtime::validate::PathSegment::Name("body")], &mut violations,
                            );
                        });
                    }
                }
                BodyRules::Unchecked
            }
            Some(ApiRequest::FormData { fields }) | Some(ApiRequest::UrlEncoded { fields }) => {
                let multipart = matches!(endpoint.request, Some(ApiRequest::FormData { .. }));
                let mut json_fields = Vec::new();
                for field in fields {
                    let ident = param_field_name(&field.name);
                    let place = format!("body.{}", field.name);
                    let slot = format!("{prefix}_FORM_{}", screaming_case(&ident));
                    match (&field.kind, multipart) {
                        (FormFieldKind::Json(reference), true) => {
                            let node = self.body_node(reference, field.constraints.as_deref(), &place);
                            self.reachable(&node, &mut reached);
                            if let Some(plan) = self.emit_static(&slot, node) {
                                json_fields.push((ident, field.name.clone(), field.required, plan));
                            }
                        }
                        (FormFieldKind::File { .. } | FormFieldKind::Files { .. }, true) => {}
                        _ => {
                            let Some(rules) = field.constraints.as_deref() else { continue };
                            let node = self.constraint_node(rules, &place);
                            let Some(plan) = self.emit_static(&slot, node) else { continue };
                            let wire = &field.name;
                            let field_ident = format_ident!("{ident}");
                            let check = quote! {
                                schematic_runtime::validate::check_str(
                                    value, &#plan,
                                    &[schematic_runtime::validate::PathSegment::Name("body"), schematic_runtime::validate::PathSegment::Name(#wire)],
                                    &mut violations,
                                );
                            };
                            checks.extend(if field.required {
                                quote! { { let value = &self.body.#field_ident; #check } }
                            } else {
                                quote! { if let Some(value) = &self.body.#field_ident { #check } }
                            });
                        }
                    }
                }
                if json_fields.is_empty() { BodyRules::Unchecked } else { BodyRules::FormJson(json_fields) }
            }
            _ => BodyRules::Unchecked,
        };

        let composition = self.composition_findings(endpoint);
        self.findings.extend(composition);
        let own_findings = self.findings.split_off(before);
        let mut findings = own_findings.clone();
        for index in reached {
            findings.extend(self.component_findings.get(&index).into_iter().flatten().cloned());
        }
        RequestRules { field_checks: checks, body, findings, own_findings }
    }

    /// Adds every component `node` reaches, through references and the
    /// components they reach, to `out`.
    fn reachable(&self, node: &Node, out: &mut BTreeSet<usize>) {
        if let Some(items) = &node.items {
            self.reachable(items, out);
        }
        if let Some(values) = &node.values {
            self.reachable(values, out);
        }
        for (_, child) in &node.properties {
            self.reachable(child, out);
        }
        match &node.reference {
            Some(Target::Component(index)) => {
                if out.insert(*index) {
                    self.reachable(&self.components[*index].node, out);
                }
            }
            Some(Target::Inline(inner)) => self.reachable(inner, out),
            None => {}
        }
    }

    /// The module of every plan the compiled requests use; empty when none
    /// checks anything.
    pub(crate) fn into_tokens(mut self) -> TokenStream {
        if self.statics.is_empty() {
            return TokenStream::new();
        }
        let live = self.live();
        let mut emitted = BTreeSet::new();
        let mut component_statics = Vec::new();
        while let Some(&index) = self.used.iter().find(|index| !emitted.contains(*index)) {
            emitted.insert(index);
            let node = prune(&self.components[index].node, &live).unwrap_or_default();
            let expr = self.expr(&node);
            let name = component_static(self.components[index].source, index);
            component_statics.push(quote! { pub(super) static #name: Plan = #expr; });
        }
        let patterns = self.patterns.iter().map(|(source, name)| quote! { pub(super) static #name: Pattern = Pattern::new(#source); });
        let statics = &self.statics;
        let module = self.module_ident();
        let doc = format!(" Value-rule plans of the `{}` requests.", self.api.name);
        quote! {
            #[doc = #doc]
            mod #module {
                #[allow(unused_imports)]
                use schematic_runtime::validate::{Literal, Pattern, Plan, Property};

                #(#patterns)*
                #(#statics)*
                #(#component_statics)*
            }
        }
    }

    /// Emits `node` as the `static` named after `slot` and returns the path
    /// requests refer to it by; `None` when it checks nothing.
    fn emit_static(&mut self, slot: &str, node: Node) -> Option<TokenStream> {
        let live = self.live();
        let node = prune(&node, &live)?;
        let mut name = slot.to_string();
        let mut suffix = 2;
        while !self.names.insert(name.clone()) {
            name = format!("{slot}_{suffix}");
            suffix += 1;
        }
        let ident = format_ident!("{name}");
        let expr = self.expr(&node);
        self.statics.push(quote! { pub(super) static #ident: Plan = #expr; });
        let module = self.module_ident();
        Some(quote! { #module::#ident })
    }

    /// Which compiled components check anything, directly or through what
    /// they reference. Recomputed whenever new components were compiled.
    fn live(&mut self) -> Vec<bool> {
        if let Some(live) = &self.live
            && live.len() == self.components.len()
        {
            return live.clone();
        }
        let mut live = vec![false; self.components.len()];
        loop {
            let mut changed = false;
            for index in 0..self.components.len() {
                if !live[index] && is_live(&self.components[index].node, &live) {
                    live[index] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        self.live = Some(live.clone());
        live
    }

    /// The plan of a JSON value of type `reference`, with `rules` added at
    /// its root.
    fn body_node(&mut self, reference: &TypeRef, rules: Option<&Constraints>, place: &str) -> Node {
        let schema = self.schema_node(&reference.wire_schema(), None, false, place);
        let Some(rules) = rules else { return schema };
        let mut root = self.constraint_node(rules, place);
        root.reference = Some(Target::Inline(Box::new(schema)));
        root
    }

    /// A definition's [`Constraints`] as a plan, recording the rules it
    /// cannot enforce under `place`.
    fn constraint_node(&mut self, rules: &Constraints, place: &str) -> Node {
        let mut node = Node {
            min_length: rules.min_length,
            max_length: rules.max_length,
            minimum: rules.minimum.as_ref().map(|number| number.as_str().to_owned()),
            exclusive_minimum: rules.exclusive_minimum.as_ref().map(|number| number.as_str().to_owned()),
            maximum: rules.maximum.as_ref().map(|number| number.as_str().to_owned()),
            exclusive_maximum: rules.exclusive_maximum.as_ref().map(|number| number.as_str().to_owned()),
            multiple_of: rules.multiple_of.as_ref().map(|number| number.as_str().to_owned()),
            min_items: rules.min_items,
            max_items: rules.max_items,
            unique_items: rules.unique_items,
            allowed: rules.allowed_values.clone(),
            constant: rules.constant.clone(),
            ..Node::default()
        };
        if let Some(source) = &rules.pattern {
            if rules.pattern_unenforced {
                self.warn(place, "pattern", "the imported pattern is outside the enforced subset".to_owned());
            } else {
                node.pattern = self.admitted(source, place, false);
            }
        }
        if let Some(format) = &rules.format
            && !NUMERIC_FORMATS.contains(&format.as_str())
        {
            self.warn(place, "format", format!("`{format}` is an annotation only"));
        }
        if let Some(items) = &rules.items {
            node.items = Some(Box::new(self.constraint_node(items, &format!("{place}[]"))));
        }
        node
    }

    /// The translated source of an enforceable pattern, or `None`.
    ///
    /// Only a valid pattern outside the enforced subset in a component
    /// schema (`report_unsupported`) is a warning here. Generator validation
    /// already refuses an invalid pattern anywhere, and an endpoint pattern
    /// outside the subset that is not marked unenforced, as definition errors
    /// (`constraint_problems`), so reporting either again would duplicate it.
    fn admitted(&mut self, source: &str, place: &str, report_unsupported: bool) -> Option<String> {
        match admit_pattern(source) {
            PatternAdmission::Enforced { translated } => Some(translated),
            PatternAdmission::Unsupported { reason } if report_unsupported => {
                self.warn(place, "pattern", format!("outside the enforced subset: {reason}"));
                None
            }
            _ => None,
        }
    }

    fn warn(&mut self, place: &str, keyword: &str, reason: String) {
        self.findings.push(Finding { severity: Severity::Warning, place: place.to_owned(), keyword: keyword.to_owned(), reason });
    }

    /// The plan of the schema object `value`, inside the component
    /// `component` (`None` for a wire schema); `root` is the component's
    /// own root.
    ///
    /// Compositions other than an `Option` or a one-member `allOf` are not
    /// entered: their rules are reported by [`Self::composition_findings`].
    fn schema_node(&mut self, value: &Value, component: Option<usize>, root: bool, place: &str) -> Node {
        let Some(object) = value.as_object() else { return Node::default() };
        let mut node = Node {
            nullable: object.get("nullable") == Some(&Value::Bool(true))
                || object.get("type").and_then(Value::as_array).is_some_and(|types| types.iter().any(|kind| kind == "null")),
            ..Node::default()
        };
        // Generator validation refuses a malformed component rule
        // (`constraint_problems`) before any plan is emitted, so a schema
        // whose keywords do not read lowers no rules rather than some.
        let mut rules = Constraints::from_schema(object).unwrap_or_default();
        rules.items = None;
        for keyword in ["minimum", "maximum"] {
            if is_width_bound(object, keyword) {
                match keyword {
                    "minimum" => rules.minimum = None,
                    _ => rules.maximum = None,
                }
            }
        }
        if root && is_enum_value_set(object) {
            rules.allowed_values = None;
            rules.constant = None;
        }
        node.min_length = rules.min_length;
        node.max_length = rules.max_length;
        node.minimum = rules.minimum.map(|number| number.as_str().to_owned());
        node.exclusive_minimum = rules.exclusive_minimum.map(|number| number.as_str().to_owned());
        node.maximum = rules.maximum.map(|number| number.as_str().to_owned());
        node.exclusive_maximum = rules.exclusive_maximum.map(|number| number.as_str().to_owned());
        node.multiple_of = rules.multiple_of.map(|number| number.as_str().to_owned());
        node.min_items = rules.min_items;
        node.max_items = rules.max_items;
        node.unique_items = rules.unique_items;
        node.allowed = rules.allowed_values;
        node.constant = rules.constant;
        if let Some(source) = &rules.pattern {
            node.pattern = match component {
                Some(index) => {
                    let before = self.findings.len();
                    let pattern = self.admitted(source, place, true);
                    let found: Vec<Finding> = self.findings.drain(before..).collect();
                    self.component_findings.entry(index).or_default().extend(found);
                    pattern
                }
                None => self.admitted(source, place, true),
            };
        }

        let current = component.map(|index| self.components[index].source.name.clone());
        if let Some(reference) = object.get("$ref").and_then(Value::as_str)
            && let Some(target) = referenced_component(self.api, reference, current.as_deref())
        {
            node.reference = Some(Target::Component(self.component(target)));
        }
        if let Some(Value::Array(members)) = object.get("anyOf")
            && members.len() == 2
            && members.iter().any(admits_only_null)
            && let Some(member) = members.iter().find(|member| !admits_only_null(member))
        {
            let inner = self.schema_node(member, component, false, place);
            node.nullable = true;
            // Schemars never writes a `$ref` beside an `Option`'s `anyOf`;
            // if a stored schema does, the `$ref` is the one followed.
            if node.reference.is_none() {
                node.reference = Some(Target::Inline(Box::new(inner)));
            }
        }
        // Schemars writes a `$ref` with sibling keywords (a rule beside a
        // model reference) as a one-member `allOf` holding the `$ref`.
        if let Some(Value::Array(members)) = object.get("allOf")
            && let [member] = members.as_slice()
            && node.reference.is_none()
        {
            let inner = self.schema_node(member, component, false, place);
            node.reference = Some(Target::Inline(Box::new(inner)));
        }
        if let Some(Value::Object(properties)) = object.get("properties") {
            let mut names: Vec<&String> = properties.keys().collect();
            names.sort();
            for name in names {
                let child = self.schema_node(&properties[name.as_str()], component, false, &format!("{place}.{name}"));
                node.properties.push((name.clone(), child));
            }
        }
        if let Some(items @ Value::Object(_)) = object.get("items") {
            node.items = Some(Box::new(self.schema_node(items, component, false, &format!("{place}[]"))));
        }
        if let Some(values @ Value::Object(_)) = object.get("additionalProperties") {
            node.values = Some(Box::new(self.schema_node(values, component, false, &format!("{place}[*]"))));
        }
        node
    }

    /// The index of `target`'s plan, compiling it on first use. A component
    /// being compiled is already indexed, so recursion ends at its index.
    fn component(&mut self, target: &'a SchemaComponent) -> usize {
        if let Some(&index) = self.index.get(&target.name) {
            return index;
        }
        let index = self.components.len();
        self.components.push(ComponentPlan { source: target, node: Node::default() });
        self.index.insert(target.name.clone(), index);
        let place = format!("component `{}`", target.name);
        let node = self.schema_node(&target.schema, Some(index), true, &place);
        self.components[index].node = node;
        index
    }

    /// The rules of `endpoint`'s JSON values that sit beneath a composition,
    /// beneath a keyword the plans do not enter (a positional `items` list,
    /// `prefixItems`, `contains`, ...), or beside an unresolved reference.
    fn composition_findings(&self, endpoint: &Endpoint) -> Vec<Finding> {
        let mut references: Vec<(String, &TypeRef)> = Vec::new();
        match &endpoint.request {
            Some(ApiRequest::Json(reference)) => references.push(("body".to_owned(), reference)),
            Some(ApiRequest::FormData { fields }) => {
                for field in fields {
                    if let FormFieldKind::Json(reference) = &field.kind {
                        references.push((format!("body.{}", field.name), reference));
                    }
                }
            }
            _ => {}
        }
        let mut findings = Vec::new();
        for (place, reference) in references {
            for rule in unenforceable_rules(self.api, reference) {
                if self.guaranteed_by_rust_type(&rule) {
                    continue;
                }
                let union_member = rule
                    .composition
                    .as_ref()
                    .is_some_and(|site| site.component.is_some() && site.pointer.is_empty() && matches!(site.keyword, "anyOf" | "oneOf"));
                let location = match &rule.component {
                    Some(component) => format!("{place} (component `{component}` at `#{}`)", rule.pointer),
                    None => format!("{place} (at `#{}`)", rule.pointer),
                };
                findings.push(Finding {
                    severity: if union_member { Severity::Warning } else { Severity::Error },
                    place: location,
                    keyword: rule.keyword.to_owned(),
                    reason: if union_member {
                        format!("{}, a union whose members are not checked", rule.reason)
                    } else if rule.composition.is_some() {
                        format!("{}; the client cannot check it, so remove the rule or the composition", rule.reason)
                    } else {
                        format!("{}; the client cannot check it, so remove the rule or restructure the schema", rule.reason)
                    },
                });
            }
        }
        findings
    }

    /// Whether `rule` is a string `enum`/`const` that Schemars writes for a
    /// Rust enum: a unit enum's value set at a component's root, or a
    /// variant (or its tag property) of a component whose root is the
    /// enum's `oneOf`/`anyOf`. Serializing the Rust value always meets it.
    fn guaranteed_by_rust_type(&self, rule: &UnenforceableRule) -> bool {
        if !matches!(rule.keyword, "enum" | "const") {
            return false;
        }
        let Some(component) = rule.component.as_deref().and_then(|name| self.api.component(name)) else {
            return false;
        };
        let Some(object) = component.schema.pointer(&rule.pointer).and_then(Value::as_object) else {
            return false;
        };
        if !string_values(object) {
            return false;
        }
        if rule.pointer.is_empty() {
            return true;
        }
        let tokens: Vec<&str> = rule.pointer.split('/').skip(1).collect();
        matches!(tokens.as_slice(), ["oneOf" | "anyOf", index] | ["oneOf" | "anyOf", index, "properties", _] if index.parse::<usize>().is_ok())
    }

    /// Statements checking one parameter field of `self` against `node`.
    fn param_check(&mut self, prefix: &str, location: &str, field: &str, required: bool, shape: Shape, node: &Node) -> TokenStream {
        let field_ident = format_ident!("{field}");
        let slot = format!("{prefix}_{}_{}", screaming_case(location), screaming_case(field));
        let segments = quote! {
            schematic_runtime::validate::PathSegment::Name(#location), schematic_runtime::validate::PathSegment::Name(#field)
        };
        let check = if shape.array {
            let counts = Node { min_items: node.min_items, max_items: node.max_items, ..Node::default() };
            let mut statements = TokenStream::new();
            if let Some(plan) = self.emit_static(&slot, counts) {
                statements.extend(quote! {
                    schematic_runtime::validate::check_array_len(value.len(), &#plan, &[#segments], &mut violations);
                });
            }
            if let Some(plan) = self.emit_static(&format!("{slot}_ARRAY"), node.whole_array_rules()) {
                let element = shape.scalar.to_json(quote! { element });
                statements.extend(quote! {
                    let array = serde_json::Value::Array(value.iter().map(|element| #element).collect());
                    schematic_runtime::validate::check_value(&array, &#plan, &[#segments], &mut violations);
                });
            }
            if let Some(items) = &node.items
                && let Some(plan) = self.emit_static(&format!("{slot}_ITEMS"), (**items).clone())
            {
                let check = shape.scalar.check(quote! { element }, &plan, quote! { &[#segments, schematic_runtime::validate::PathSegment::Index(index)] });
                statements.extend(quote! {
                    for (index, element) in value.iter().enumerate() {
                        #check
                    }
                });
            }
            statements
        } else {
            match self.emit_static(&slot, node.clone()) {
                Some(plan) => shape.scalar.check(quote! { value }, &plan, quote! { &[#segments] }),
                None => TokenStream::new(),
            }
        };
        if check.is_empty() {
            return check;
        }
        if required {
            quote! { { let value = &self.#field_ident; #check } }
        } else {
            quote! { if let Some(value) = &self.#field_ident { #check } }
        }
    }

    /// The `Plan` expression of `node`, which [`prune`] has already
    /// reduced to live rules.
    fn expr(&mut self, node: &Node) -> TokenStream {
        let mut tokens = match &node.reference {
            Some(Target::Component(index)) => {
                self.used.insert(*index);
                let name = component_static(self.components[*index].source, *index);
                quote! { Plan::reference(&#name) }
            }
            Some(Target::Inline(inner)) => {
                let inner = self.expr(inner);
                quote! { Plan::reference(&#inner) }
            }
            None => quote! { Plan::new() },
        };
        if node.nullable {
            tokens.extend(quote! { .nullable() });
        }
        for (method, count) in [("min_length", node.min_length), ("max_length", node.max_length), ("min_items", node.min_items), ("max_items", node.max_items)] {
            if let Some(count) = count {
                let method = format_ident!("{method}");
                tokens.extend(quote! { .#method(#count) });
            }
        }
        if let Some(source) = &node.pattern {
            let pattern = self.pattern(source);
            tokens.extend(quote! { .pattern(&#pattern) });
        }
        for (method, number) in [
            ("minimum", &node.minimum),
            ("exclusive_minimum", &node.exclusive_minimum),
            ("maximum", &node.maximum),
            ("exclusive_maximum", &node.exclusive_maximum),
            ("multiple_of", &node.multiple_of),
        ] {
            if let Some(number) = number {
                let method = format_ident!("{method}");
                tokens.extend(quote! { .#method(#number) });
            }
        }
        if node.unique_items {
            tokens.extend(quote! { .unique_items() });
        }
        if let Some(allowed) = &node.allowed {
            let literals = allowed.iter().map(literal);
            tokens.extend(quote! { .allowed_values(&[#(#literals),*]) });
        }
        if let Some(constant) = &node.constant {
            let literal = literal(constant);
            tokens.extend(quote! { .constant(&#literal) });
        }
        if let Some(items) = &node.items {
            let items = self.expr(items);
            tokens.extend(quote! { .items(&#items) });
        }
        if !node.properties.is_empty() {
            let properties: Vec<TokenStream> = node
                .properties
                .iter()
                .map(|(name, child)| {
                    let child = self.expr(child);
                    quote! { Property::new(#name, &#child) }
                })
                .collect();
            tokens.extend(quote! { .properties(&[#(#properties),*]) });
        }
        if let Some(values) = &node.values {
            let values = self.expr(values);
            tokens.extend(quote! { .values(&#values) });
        }
        tokens
    }

    fn pattern(&mut self, source: &str) -> proc_macro2::Ident {
        if let Some((_, name)) = self.patterns.iter().find(|(existing, _)| existing == source) {
            return name.clone();
        }
        let name = format_ident!("PATTERN_{}", self.patterns.len());
        self.patterns.push((source.to_owned(), name.clone()));
        name
    }
}

fn component_static(component: &SchemaComponent, index: usize) -> proc_macro2::Ident {
    format_ident!("COMPONENT_{index}_{}", screaming_case(&component.name))
}

/// The JSON shape of a parameter value.
#[derive(Debug, Clone, Copy)]
struct Shape {
    scalar: Scalar,
    array: bool,
}

#[derive(Debug, Clone, Copy)]
enum Scalar {
    Str,
    /// A generated parameter enum, checked by its wire spelling.
    StrEnum,
    Number,
    Bool,
    Json,
}

impl Scalar {
    /// The statement checking `value`, a reference to one value.
    fn check(self, value: TokenStream, plan: &TokenStream, path: TokenStream) -> TokenStream {
        match self {
            Self::Str => quote! { schematic_runtime::validate::check_str(#value, &#plan, #path, &mut violations); },
            Self::StrEnum => quote! { schematic_runtime::validate::check_str(#value.as_str(), &#plan, #path, &mut violations); },
            Self::Number => quote! { schematic_runtime::validate::check_number(*#value, &#plan, #path, &mut violations); },
            Self::Bool => quote! {
                schematic_runtime::validate::check_value(&serde_json::Value::Bool(*#value), &#plan, #path, &mut violations);
            },
            Self::Json => quote! { schematic_runtime::validate::check_value(#value, &#plan, #path, &mut violations); },
        }
    }

    /// The `serde_json::Value` of `value`, a reference to one element.
    fn to_json(self, value: TokenStream) -> TokenStream {
        match self {
            Self::Str => quote! { serde_json::Value::String(#value.clone()) },
            Self::StrEnum => quote! { serde_json::Value::String(#value.as_str().to_owned()) },
            Self::Number => quote! { serde_json::json!(*#value) },
            Self::Bool => quote! { serde_json::Value::Bool(*#value) },
            Self::Json => quote! { #value.clone() },
        }
    }
}

/// The Rust shape `query_param_type_to_rust_type` gives a header or cookie
/// parameter.
fn named_shape(param_type: &QueryParamType) -> Shape {
    let scalar = |param_type: &QueryParamType| match param_type {
        QueryParamType::Integer | QueryParamType::Number => Scalar::Number,
        QueryParamType::Boolean => Scalar::Bool,
        QueryParamType::Json => Scalar::Json,
        _ => Scalar::Str,
    };
    match param_type {
        QueryParamType::Array(inner) => Shape { scalar: scalar(inner), array: true },
        other => Shape { scalar: scalar(other), array: false },
    }
}

fn is_live(node: &Node, live: &[bool]) -> bool {
    node.has_own_rules()
        || node.items.as_deref().is_some_and(|items| is_live(items, live))
        || node.values.as_deref().is_some_and(|values| is_live(values, live))
        || node.properties.iter().any(|(_, child)| is_live(child, live))
        || match &node.reference {
            Some(Target::Component(index)) => live[*index],
            Some(Target::Inline(inner)) => is_live(inner, live),
            None => false,
        }
}

/// `node` without the children and references that check nothing; `None`
/// when nothing is left.
///
/// While the extra-value plan survives, every declared property is kept,
/// with an empty plan when its own checks nothing: the runtime gives a key
/// absent from the property list the extra-value plan.
fn prune(node: &Node, live: &[bool]) -> Option<Node> {
    if !is_live(node, live) {
        return None;
    }
    let values = node.values.as_deref().and_then(|values| prune(values, live)).map(Box::new);
    let keep_declared = values.is_some();
    Some(Node {
        items: node.items.as_deref().and_then(|items| prune(items, live)).map(Box::new),
        properties: node
            .properties
            .iter()
            .filter_map(|(name, child)| match prune(child, live) {
                Some(child) => Some((name.clone(), child)),
                None => keep_declared.then(|| (name.clone(), Node::default())),
            })
            .collect(),
        values,
        reference: match &node.reference {
            Some(Target::Component(index)) if live[*index] => Some(Target::Component(*index)),
            Some(Target::Inline(inner)) => prune(inner, live).map(|inner| Target::Inline(Box::new(inner))),
            _ => None,
        },
        ..node.clone()
    })
}

/// Whether a component root is a unit enum's value set: a string schema
/// whose `enum` or `const` holds only strings.
fn is_enum_value_set(object: &Map<String, Value>) -> bool {
    object.get("type").and_then(Value::as_str) == Some("string") && string_values(object)
}

/// Whether the object's `enum` (or `const`) holds only strings.
fn string_values(object: &Map<String, Value>) -> bool {
    match (object.get("enum"), object.get("const")) {
        (Some(Value::Array(values)), _) => !values.is_empty() && values.iter().all(Value::is_string),
        (None, Some(Value::String(_))) => true,
        _ => false,
    }
}

/// The `Literal` expression of a JSON value.
fn literal(value: &Value) -> TokenStream {
    match value {
        Value::Null => quote! { Literal::Null },
        Value::Bool(flag) => quote! { Literal::Bool(#flag) },
        Value::Number(number) => {
            let text = number.to_string();
            quote! { Literal::Number(#text) }
        }
        Value::String(text) => quote! { Literal::String(#text) },
        Value::Array(items) => {
            let items = items.iter().map(literal);
            quote! { Literal::Array(&[#(#items),*]) }
        }
        Value::Object(members) => {
            let members = members.iter().map(|(name, member)| {
                let member = literal(member);
                quote! { (#name, #member) }
            });
            quote! { Literal::Object(&[#(#members),*]) }
        }
    }
}

/// `name` in `snake_case`, with acronyms kept together (`OpenAI` →
/// `open_ai`) and anything outside `[A-Za-z0-9]` as `_`.
pub(crate) fn snake_case(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (index, &c) in chars.iter().enumerate() {
        if !c.is_ascii_alphanumeric() {
            if !out.ends_with('_') {
                out.push('_');
            }
            continue;
        }
        if c.is_ascii_uppercase() && index > 0 {
            let previous = chars[index - 1];
            let next_lower = chars.get(index + 1).is_some_and(char::is_ascii_lowercase);
            if (previous.is_ascii_lowercase() || previous.is_ascii_digit() || (previous.is_ascii_uppercase() && next_lower))
                && !out.ends_with('_')
            {
                out.push('_');
            }
        }
        out.push(c.to_ascii_lowercase());
    }
    let out = out.trim_matches('_').to_owned();
    if out.starts_with(|c: char| c.is_ascii_digit()) { format!("_{out}") } else { out }
}

fn screaming_case(name: &str) -> String {
    snake_case(name).trim_start_matches('_').to_ascii_uppercase()
}

#[cfg(test)]
mod tests;
