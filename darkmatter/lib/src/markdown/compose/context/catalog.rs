//! Derived descriptor catalog for runtime context variables.
//!
//! Each [`ContextVariableDescriptor`] describes a single variable that may be
//! captured into `ComposeContext` at runtime. The catalog is **projected from
//! the authored base schema** (`darkmatter/docs/schemas/darkmatter.yaml`) rather
//! than hand-declared: `name`, type, `description`, and the `generated` /
//! `required` / `default` flags all come from parsing the base schema's `ctx`
//! block once via [`crate::markdown::schemas::darkmatter_base_schema`]. This
//! makes the YAML the single source of truth (spec D1/D5) so the schema and the
//! catalog can never drift.
//!
//! Only two pieces of catalog metadata are not schema data and stay in Rust:
//! the presentation grouping ([`CONTEXT_VARIABLE_GROUPING`], `category` /
//! `subsection`) and — deferred — verified examples. Reading the catalog parses
//! the checked-in YAML on first access but performs no host probe, no runtime
//! context capture, and no network or filesystem I/O beyond the compiled-in
//! schema string.
use std::fmt;
use std::sync::LazyLock;

use crate::catalog::{Described, Example};
use crate::markdown::schemas::{
    Constraint, PropertyDef, SimplifiedSchema, SimplifiedType, TypeExpr, darkmatter_base_schema,
};

/// SimplifiedSchema type of a context variable, projected from the base schema.
///
/// Replaces the retired presentation-only enum. After the Group 1–3 collapse
/// (spec D3/D6), every `ctx.*` type is a real SimplifiedSchema type, so this
/// carries only genuine type shape: the primitive keyword, whether it is an
/// array, and whether a `number` is integer-constrained. Optionality is a
/// *flag* ([`ContextVariableDescriptor::required`]), not a type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextValueType {
    /// Projected primitive type keyword (`string`, `datetime`, `object`, …).
    pub base: SimplifiedType,
    /// Whether the schema types the value as an array (`[]`).
    pub is_array: bool,
    /// Whether a `number` value is integer-constrained (`number(integer)`).
    pub integer: bool,
}

impl fmt::Display for ContextValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let keyword = self.base.as_keyword();
        if self.is_array {
            write!(f, "{keyword}[]")
        } else if self.integer && self.base == SimplifiedType::Number {
            write!(f, "number(integer)")
        } else {
            write!(f, "{keyword}")
        }
    }
}

/// Descriptor for a single runtime context variable.
///
/// The `name`, `display_type`, `description`, and the three schema flags are
/// projected from the base schema; `category` / `subsection` come from
/// [`CONTEXT_VARIABLE_GROUPING`]; `order` is the YAML declaration index.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextVariableDescriptor {
    /// Canonical variable name (used in `ctx.NAME`).
    pub name: &'static str,
    /// Projected SimplifiedSchema type.
    pub display_type: ContextValueType,
    /// Short description of the variable's meaning (schema `-> {description}`).
    pub description: &'static str,
    /// Top-level grouping category (presentation-only).
    pub category: &'static str,
    /// Sub-section within the category (presentation-only, may be empty).
    pub subsection: &'static str,
    /// Stable display order: the variable's YAML declaration index (1-based).
    pub order: usize,
    /// Whether the value is host-supplied (`generated`). Uniformly `true` for
    /// `ctx.*`, kept as a projected flag for fidelity and drift-guarding.
    pub generated: bool,
    /// Whether the property is required (non-nullable). `false` means the value
    /// may be captured as `null` when unavailable.
    pub required: bool,
    /// Declared default value, if the schema supplied one.
    pub default: Option<serde_json::Value>,
    /// Optional verified example. Currently `None`: example artifacts move to
    /// schema-plus `example()` files, deferred until the embedded base schema
    /// gains a runtime filesystem anchor (spec E3).
    pub example: Option<Example>,
}

impl Described for ContextVariableDescriptor {
    fn key(&self) -> &'static str {
        self.name
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn category(&self) -> &'static str {
        self.category
    }
    fn order(&self) -> usize {
        self.order
    }
    fn example(&self) -> Option<&Example> {
        self.example.as_ref()
    }
}

/// Presentation grouping (`category`, `subsection`) for each `ctx.*` variable.
///
/// Deliberately kept out of the validation schema (spec D5): grouping is
/// documentation taxonomy, not schema semantics. Ordered to mirror YAML
/// declaration order for readability. Must stay **total** — every projected
/// `ctx.*` key needs an entry and every entry must name a live key. The
/// `grouping_map_is_total` test fails if the YAML adds or removes a key without
/// a matching grouping edit.
const CONTEXT_VARIABLE_GROUPING: &[(&str, &str, &str)] = &[
    // ── Date and Time ───────────────────────────────────────────────
    ("now", "Date and Time", ""),
    ("now_utc", "Date and Time", ""),
    ("today", "Date and Time", ""),
    ("today_utc", "Date and Time", ""),
    ("yesterday", "Date and Time", ""),
    ("yesterday_utc", "Date and Time", ""),
    ("tomorrow", "Date and Time", ""),
    ("tomorrow_utc", "Date and Time", ""),
    ("day", "Date and Time", ""),
    ("day_utc", "Date and Time", ""),
    ("day_abbr", "Date and Time", ""),
    ("day_abbr_utc", "Date and Time", ""),
    ("year", "Date and Time", ""),
    ("year_utc", "Date and Time", ""),
    ("month", "Date and Time", ""),
    ("month_name", "Date and Time", ""),
    ("month_name_abbr", "Date and Time", ""),
    ("day_of_month", "Date and Time", ""),
    ("day_of_month_suffixed", "Date and Time", ""),
    ("time", "Date and Time", ""),
    ("time_military", "Date and Time", ""),
    ("time_utc", "Date and Time", ""),
    ("time_military_utc", "Date and Time", ""),
    ("timezone", "Date and Time", ""),
    ("timezone_offset", "Date and Time", ""),
    ("timezone_iana", "Date and Time", ""),
    ("start_of_week_sun", "Date and Time", ""),
    ("end_of_week_sun", "Date and Time", ""),
    ("start_of_week_mon", "Date and Time", ""),
    ("end_of_week_mon", "Date and Time", ""),
    ("start_of_week_sun_utc", "Date and Time", ""),
    ("end_of_week_sun_utc", "Date and Time", ""),
    ("start_of_week_mon_utc", "Date and Time", ""),
    ("end_of_week_mon_utc", "Date and Time", ""),
    ("season", "Date and Time", ""),
    ("timestamp", "Date and Time", ""),
    ("timestamp_ms", "Date and Time", ""),
    ("utc", "Date and Time", "Aliases"),
    ("dow", "Date and Time", "Aliases"),
    ("dow_abbr", "Date and Time", "Aliases"),
    // ── Repository ──────────────────────────────────────────────────
    ("repo", "Repository", ""),
    ("repo_root", "Repository", ""),
    ("branch", "Repository", "Git"),
    ("worktree", "Repository", "Git"),
    ("is_monorepo", "Repository", ""),
    ("package_root", "Repository", "Packages"),
    ("package_area_root", "Repository", "Packages"),
    ("packages", "Repository", "Packages"),
    ("package_areas", "Repository", "Packages"),
    ("current_package", "Repository", "Packages"),
    ("current_package_area", "Repository", "Packages"),
    ("area", "Repository", "Scope"),
    ("area_description", "Repository", "Scope"),
    ("area_root", "Repository", "Scope"),
    ("current_packages", "Repository", "Scope"),
    ("depends_on", "Repository", "Scope"),
    ("used_by", "Repository", "Scope"),
    // ── File Changes ────────────────────────────────────────────────
    ("dirty_files", "File Changes", ""),
    ("merge_conflicts", "File Changes", "Conflicts"),
    ("dirty_source_code_files", "File Changes", ""),
    ("staged_files", "File Changes", ""),
    ("untracked_files", "File Changes", ""),
    ("dirty_packages", "File Changes", "Packages"),
    ("dirty_package_areas", "File Changes", "Packages"),
    ("staged_packages", "File Changes", "Packages"),
    ("staged_package_areas", "File Changes", "Packages"),
    ("current_package_has_staged_files", "File Changes", "Flags"),
    ("current_package_area_has_staged_files", "File Changes", "Flags"),
    ("current_package_has_dirty_files", "File Changes", "Flags"),
    ("current_package_area_has_dirty_files", "File Changes", "Flags"),
    // ── Languages ───────────────────────────────────────────────────
    ("programming_languages_in_repo", "Languages", ""),
    ("programming_language", "Languages", ""),
    ("package_manager", "Languages", ""),
    // ── Documents ───────────────────────────────────────────────────
    ("docs_readme", "Documents", ""),
    ("docs_blast_radius", "Documents", ""),
    ("docs_drift", "Documents", ""),
    ("docs_skill", "Documents", ""),
    // ── Operating System ────────────────────────────────────────────
    ("os", "Operating System", ""),
    ("os_distro", "Operating System", ""),
    ("os_package_manager", "Operating System", ""),
    ("os_version", "Operating System", ""),
    // ── Hardware ────────────────────────────────────────────────────
    ("memory_total", "Hardware", ""),
    ("memory_used", "Hardware", ""),
    ("memory_avail", "Hardware", ""),
    ("cpu_cores", "Hardware", ""),
    ("cpu_arch", "Hardware", ""),
    ("gpu", "Hardware", ""),
    // ── Agent ───────────────────────────────────────────────────────
    ("agent", "Agent", ""),
    ("model", "Agent", ""),
];

/// All context variable descriptors, projected from the base schema in YAML
/// declaration order. Private: external consumers use
/// [`context_variable_descriptors`]; in-crate consumers may read this static
/// directly.
pub(crate) static CONTEXT_VARIABLE_DESCRIPTORS: LazyLock<Vec<ContextVariableDescriptor>> =
    LazyLock::new(project_descriptors);

/// Returns all context variable descriptors in display order.
pub fn context_variable_descriptors() -> &'static [ContextVariableDescriptor] {
    &CONTEXT_VARIABLE_DESCRIPTORS
}

/// Projects the descriptor catalog from the authored base schema's `ctx` block.
///
/// Runs once through [`CONTEXT_VARIABLE_DESCRIPTORS`]'s `LazyLock`. The `ctx`
/// shape's `IndexMap` preserves YAML declaration order, so descriptor order and
/// `order` indices match the authored document.
fn project_descriptors() -> Vec<ContextVariableDescriptor> {
    let schema = darkmatter_base_schema();
    let ctx_shape = ctx_shape(&schema);
    ctx_shape
        .properties
        .iter()
        .enumerate()
        .map(|(index, (name, def))| project_one(index, name, def))
        .collect()
}

/// Extracts the inline-object shape rooted at the base schema's `ctx` property.
fn ctx_shape(schema: &SimplifiedSchema) -> &crate::markdown::schemas::SchemaShape {
    let SimplifiedSchema::Single(root) = schema else {
        panic!("darkmatter base schema must be a single object shape");
    };
    let ctx = root
        .properties
        .get("ctx")
        .expect("base schema must declare a `ctx` property");
    let atom = match ctx {
        PropertyDef::Single(atom) => atom,
        PropertyDef::Union(atoms) => atoms.first().expect("`ctx` union must be non-empty"),
    };
    match &atom.ty {
        TypeExpr::InlineObject(shape) => shape,
        _ => panic!("base schema `ctx` must be an inline object shape"),
    }
}

/// Projects one `ctx.*` property atom into a descriptor.
fn project_one(index: usize, name: &str, def: &PropertyDef) -> ContextVariableDescriptor {
    let atom = match def {
        PropertyDef::Single(atom) => atom,
        PropertyDef::Union(atoms) => atoms.first().expect("`ctx` property union must be non-empty"),
    };

    let base = match &atom.ty {
        TypeExpr::Primitive(ty) => *ty,
        // `ctx.*` never authors an inline object or import; fall back defensively.
        TypeExpr::InlineObject(_) | TypeExpr::Imported { .. } => SimplifiedType::Object,
    };

    let required = atom
        .constraints
        .iter()
        .chain(&atom.array_constraints)
        .any(|c| matches!(c, Constraint::Required));
    let generated = atom
        .constraints
        .iter()
        .chain(&atom.array_constraints)
        .any(|c| matches!(c, Constraint::Generated));
    let integer = atom
        .constraints
        .iter()
        .any(|c| matches!(c, Constraint::Integer));
    let default = atom.constraints.iter().find_map(|c| match c {
        Constraint::Default(value) => Some(value.clone()),
        _ => None,
    });

    let (category, subsection) = grouping_for(name);

    ContextVariableDescriptor {
        name: leak(name),
        display_type: ContextValueType {
            base,
            is_array: atom.is_array,
            integer,
        },
        description: leak(atom.description.as_deref().unwrap_or("")),
        category,
        subsection,
        order: index + 1,
        generated,
        required,
        default,
        example: None,
    }
}

/// Looks up a variable's presentation grouping. A missing entry projects to the
/// empty group; the `grouping_map_is_total` test — not a runtime panic — is the
/// enforcement surface for grouping totality.
fn grouping_for(name: &str) -> (&'static str, &'static str) {
    CONTEXT_VARIABLE_GROUPING
        .iter()
        .find(|(key, _, _)| *key == name)
        .map(|(_, category, subsection)| (*category, *subsection))
        .unwrap_or(("", ""))
}

/// Leaks a short catalog string to `'static`. The projected catalog is built
/// once behind a `LazyLock` and lives for the program's lifetime, so the ~86
/// leaked names/descriptions are a bounded, one-time cost that keeps the public
/// descriptor fields `&'static str` (matching [`Described`]).
fn leak(text: &str) -> &'static str {
    Box::leak(text.to_string().into_boxed_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::ComposeContext;
    use std::collections::HashSet;

    /// Catalog descriptors and captured runtime keys must be in exact
    /// correspondence: every descriptor has a runtime key and no runtime key
    /// lacks a descriptor. Phase 5 migrated capture to arrays and dropped the
    /// ten `_list` twins, so the Phase 3–5 transitional tolerance is gone.
    #[test]
    fn every_descriptor_has_a_captured_runtime_key() {
        let descriptor_names: HashSet<&str> = context_variable_descriptors()
            .iter()
            .map(|d| d.name)
            .collect();

        let ctx = ComposeContext::capture_for_dir(&std::env::temp_dir());
        let runtime_names: HashSet<String> = ctx.values().keys().cloned().collect();

        let missing: Vec<&&str> = descriptor_names
            .iter()
            .filter(|name| !runtime_names.contains(**name))
            .collect();
        assert!(
            missing.is_empty(),
            "descriptors without runtime keys: {missing:?}"
        );

        let extra: Vec<&String> = runtime_names
            .iter()
            .filter(|name| !descriptor_names.contains(name.as_str()))
            .collect();
        assert!(
            extra.is_empty(),
            "runtime keys without a descriptor: {extra:?}"
        );
    }

    #[test]
    fn descriptor_traversal_order_is_deterministic() {
        let names: Vec<&str> = context_variable_descriptors()
            .iter()
            .map(|d| d.name)
            .collect();
        let names_again: Vec<&str> = context_variable_descriptors()
            .iter()
            .map(|d| d.name)
            .collect();
        assert_eq!(names, names_again);
    }

    #[test]
    fn descriptor_names_are_unique() {
        let mut seen = HashSet::new();
        for d in context_variable_descriptors() {
            assert!(seen.insert(d.name), "Duplicate descriptor name: {}", d.name);
        }
    }

    #[test]
    fn catalog_access_performs_no_capture() {
        // Reading the catalog parses the compiled-in base schema once but does
        // no host probe, no I/O, and no runtime context capture. (The parity
        // test above is the only place that intentionally captures.)
        let _ = context_variable_descriptors();
    }

    /// Drift guard (spec acceptance criterion 2): the projected catalog must
    /// agree with the authored base schema on every `ctx.*` key's name, type,
    /// description, `generated` / `required` / `default` flags, and declaration
    /// order. This walks the schema atoms independently of the projection
    /// helpers, so a projection bug (reading the wrong field) fails here.
    #[test]
    fn projected_descriptors_match_base_schema() {
        use crate::markdown::schemas::darkmatter_base_schema;

        let schema = darkmatter_base_schema();
        let SimplifiedSchema::Single(root) = &schema else {
            panic!("base schema must be a single object shape");
        };
        let PropertyDef::Single(ctx_atom) =
            root.properties.get("ctx").expect("ctx property present")
        else {
            panic!("ctx must be a single atom");
        };
        let TypeExpr::InlineObject(ctx_shape) = &ctx_atom.ty else {
            panic!("ctx must be an inline object");
        };

        let descriptors = context_variable_descriptors();
        assert_eq!(
            descriptors.len(),
            ctx_shape.properties.len(),
            "descriptor count must equal schema ctx key count"
        );

        for (index, (name, def)) in ctx_shape.properties.iter().enumerate() {
            let PropertyDef::Single(atom) = def else {
                panic!("ctx.{name} must be a single atom");
            };
            let d = &descriptors[index];

            assert_eq!(d.name, name.as_str(), "name/order mismatch at index {index}");
            assert_eq!(d.order, index + 1, "order must be the declaration index");

            let expected_base = match &atom.ty {
                TypeExpr::Primitive(ty) => *ty,
                other => panic!("ctx.{name} has unexpected type expr: {other:?}"),
            };
            assert_eq!(d.display_type.base, expected_base, "ctx.{name} base type");
            assert_eq!(d.display_type.is_array, atom.is_array, "ctx.{name} array");
            let expected_integer = atom
                .constraints
                .iter()
                .any(|c| matches!(c, Constraint::Integer));
            assert_eq!(d.display_type.integer, expected_integer, "ctx.{name} integer");

            assert_eq!(
                d.description,
                atom.description.as_deref().unwrap_or(""),
                "ctx.{name} description"
            );

            let expected_required = atom
                .constraints
                .iter()
                .chain(&atom.array_constraints)
                .any(|c| matches!(c, Constraint::Required));
            let expected_generated = atom
                .constraints
                .iter()
                .chain(&atom.array_constraints)
                .any(|c| matches!(c, Constraint::Generated));
            assert_eq!(d.required, expected_required, "ctx.{name} required flag");
            assert_eq!(d.generated, expected_generated, "ctx.{name} generated flag");

            let expected_default = atom.constraints.iter().find_map(|c| match c {
                Constraint::Default(value) => Some(value.clone()),
                _ => None,
            });
            assert_eq!(d.default, expected_default, "ctx.{name} default");
        }
    }

    /// Positive coverage for the `default` projection path. No `ctx.*` property
    /// authors a `default(...)` today, so the drift guard above only ever
    /// compares `None == None`; this drives a synthetic atom carrying a default
    /// through [`project_one`] to prove the constraint reaches the descriptor.
    #[test]
    fn project_one_carries_default_constraint() {
        use crate::markdown::schemas::PropertyAtom;

        let value = serde_json::json!("fallback");
        let def = PropertyDef::Single(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::String),
            is_array: false,
            constraints: vec![Constraint::Default(value.clone())],
            array_constraints: Vec::new(),
            description: None,
        });

        let descriptor = project_one(0, "with_default", &def);
        assert_eq!(descriptor.default, Some(value));
    }

    /// Grouping totality (spec acceptance criterion 3): every projected key has
    /// a grouping entry, and every grouping entry names a live key.
    #[test]
    fn grouping_map_is_total() {
        let catalog_names: HashSet<&str> = context_variable_descriptors()
            .iter()
            .map(|d| d.name)
            .collect();
        let grouping_names: HashSet<&str> =
            CONTEXT_VARIABLE_GROUPING.iter().map(|(name, _, _)| *name).collect();

        let ungrouped: Vec<&&str> = catalog_names.difference(&grouping_names).collect();
        assert!(
            ungrouped.is_empty(),
            "ctx keys with no grouping entry: {ungrouped:?}"
        );

        let orphan_grouping: Vec<&&str> = grouping_names.difference(&catalog_names).collect();
        assert!(
            orphan_grouping.is_empty(),
            "grouping entries with no live ctx key: {orphan_grouping:?}"
        );

        // The projected category/subsection must equal the grouping map entry.
        for d in context_variable_descriptors() {
            let entry = CONTEXT_VARIABLE_GROUPING
                .iter()
                .find(|(name, _, _)| *name == d.name)
                .unwrap();
            assert_eq!(
                (d.category, d.subsection),
                (entry.1, entry.2),
                "projected grouping mismatch for {}",
                d.name
            );
        }
    }

    /// The Group 1 `_list` twins are dropped from the catalog (spec D6 / criterion 10).
    #[test]
    fn removed_list_twins_are_absent() {
        let names: HashSet<&str> = context_variable_descriptors()
            .iter()
            .map(|d| d.name)
            .collect();
        for removed in [
            "packages_list",
            "package_areas_list",
            "dirty_files_list",
            "dirty_source_code_files_list",
            "staged_files_list",
            "untracked_files_list",
            "dirty_packages_list",
            "dirty_package_areas_list",
            "staged_packages_list",
            "staged_package_areas_list",
        ] {
            assert!(!names.contains(removed), "removed `_list` twin still present: {removed}");
        }
    }

    /// Temporal types are corrected (spec D2 / criterion 4): the datetime keys
    /// are `datetime`, and the `today`-family stays `date`.
    #[test]
    fn temporal_types_are_correct() {
        let by_name: std::collections::HashMap<&str, &ContextValueType> =
            context_variable_descriptors()
                .iter()
                .map(|d| (d.name, &d.display_type))
                .collect();

        for datetime_key in ["now", "now_utc", "utc"] {
            assert_eq!(
                by_name[datetime_key].base,
                SimplifiedType::DateTime,
                "{datetime_key} must be datetime"
            );
        }
        for date_key in ["today", "today_utc", "yesterday", "tomorrow"] {
            assert_eq!(
                by_name[date_key].base,
                SimplifiedType::Date,
                "{date_key} must stay date"
            );
        }
    }

    /// The Group 1/2/3 list variables are arrays (spec criterion 9 precondition).
    #[test]
    fn collapsed_list_variables_are_arrays() {
        let by_name: std::collections::HashMap<&str, &ContextValueType> =
            context_variable_descriptors()
                .iter()
                .map(|d| (d.name, &d.display_type))
                .collect();

        for array_key in [
            "packages",
            "package_areas",
            "dirty_files",
            "current_packages",
            "docs_readme",
            "programming_languages_in_repo",
            "depends_on",
            "used_by",
        ] {
            assert!(
                by_name[array_key].is_array,
                "{array_key} must be an array type"
            );
        }
        assert_eq!(by_name["depends_on"].base, SimplifiedType::Object);
        assert_eq!(by_name["packages"].base, SimplifiedType::String);
    }
}

#[cfg(test)]
mod capture_shape_tests {
    use super::*;
    use crate::markdown::compose::ComposeContext;
    use serde_json::Value;

    /// Captured `ctx.*` values must match their projected SimplifiedSchema
    /// type — scalars as their scalar kind, array types as JSON arrays (Phase 5
    /// migrated list capture off pre-rendered strings), and `object[]`
    /// variables as arrays of objects.
    #[test]
    fn capture_shape_matches_projected_type() {
        let repo_root = crate::markdown::compose::find_git_root_from(std::path::Path::new("."))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let ctx = ComposeContext::capture_for_dir(&repo_root);
        let mut failures = Vec::new();
        for d in context_variable_descriptors() {
            let ty = &d.display_type;
            let value = ctx.values().get(d.name).unwrap_or(&Value::Null);
            // Optional variables (generated without `required`) may capture null.
            if value.is_null() && !d.required {
                continue;
            }
            let ok = if ty.is_array {
                // `object[]` items must be objects; other arrays hold scalars.
                value.as_array().is_some_and(|items| {
                    ty.base != SimplifiedType::Object
                        || items.iter().all(Value::is_object)
                })
            } else {
                match ty.base {
                    SimplifiedType::Number => value.is_number(),
                    SimplifiedType::Boolean => value.is_boolean(),
                    SimplifiedType::Object => value.is_object(),
                    // date/datetime/time/string are captured as string scalars.
                    _ => value.is_string(),
                }
            };
            if !ok {
                failures.push((d.name, ty.clone(), value.clone()));
            }
        }
        assert!(
            failures.is_empty(),
            "capture shapes do not match projected type: {failures:?}"
        );
    }
}
