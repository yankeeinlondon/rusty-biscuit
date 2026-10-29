//! Binding contracts of `function` overloads.
//!
//! An overload set is a union of `function(...)` types. Two overloads that
//! would bind exactly the same calls make every such call ambiguous, so they
//! are a schema-load error. Their *binding contract* is what decides which
//! calls bind: each parameter's type (with its constraints), whether it is
//! optional, and whether it is the rest parameter. Parameter names,
//! descriptions, examples, categories, `fallible`, and return types are not
//! part of it; neither are `any`/`unknown` spelling or union option order.
//! Registered source exclusions, which a Rust registration adds, are compared
//! where overloads are registered, not here.

use indexmap::IndexMap;

use super::serialize::serialize_property_def;
use super::types::{FunctionType, PropertyAtom, PropertyDef, TypeExpr};

/// How many named-type references deep a contract is resolved; deeper (or
/// recursive) references compare by name.
const MAX_REFERENCE_DEPTH: usize = 16;

/// The message for the first pair of `function` options in `arms` that share
/// a binding contract, or `None` when every overload binds differently.
///
/// `named` resolves bare local references (`Name`, `Name@this`) to their
/// definitions; without it references compare by name.
pub(crate) fn duplicate_overload(
    arms: &[PropertyAtom],
    named: Option<&IndexMap<String, PropertyDef>>,
) -> Option<String> {
    duplicate_overload_arm(arms, named).map(|(_, message)| message)
}

/// [`duplicate_overload`] together with the index into `arms` of the later
/// option of the pair, the one that introduced the duplicate.
pub(crate) fn duplicate_overload_arm(
    arms: &[PropertyAtom],
    named: Option<&IndexMap<String, PropertyDef>>,
) -> Option<(usize, String)> {
    let mut seen: Vec<(String, usize)> = Vec::new();
    for (index, arm) in arms.iter().enumerate() {
        let TypeExpr::Function(function) = &arm.ty else {
            continue;
        };
        if arm.is_array {
            continue;
        }
        let contract = binding_contract(function, named);
        if let Some((_, first)) = seen.iter().find(|(seen, _)| *seen == contract) {
            let message = format!(
                "overloads {} and {} have the same binding contract `{contract}`; \
                 every call that binds one binds the other",
                first + 1,
                index + 1
            );
            return Some((index, message));
        }
        seen.push((contract, index));
    }
    None
}

/// The canonical spelling of the calls `function` binds.
pub(crate) fn binding_contract(
    function: &FunctionType,
    named: Option<&IndexMap<String, PropertyDef>>,
) -> String {
    let parameters: Vec<String> = function
        .parameters
        .as_slice()
        .iter()
        .map(|parameter| {
            let ty = serialize_property_def(&canonical_def(&parameter.def, named, 0));
            match (parameter.rest, parameter.optional) {
                (true, _) => format!("...{ty}"),
                (false, true) => format!("({ty})?"),
                (false, false) => ty,
            }
        })
        .collect();
    format!("({})", parameters.join(", "))
}

fn arms_of(def: &PropertyDef) -> &[PropertyAtom] {
    match def {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(arms) => arms,
    }
}

/// `def` with documentation removed, local references inlined, nested unions
/// flattened, and options and constraints in a fixed order.
fn canonical_def(
    def: &PropertyDef,
    named: Option<&IndexMap<String, PropertyDef>>,
    depth: usize,
) -> PropertyDef {
    let mut arms: Vec<PropertyAtom> = arms_of(def)
        .iter()
        .flat_map(|atom| canonical_atoms(atom, named, depth))
        .collect();
    arms.sort_by_cached_key(super::serialize_property_atom);
    arms.dedup();
    if arms.len() == 1 {
        PropertyDef::Single(arms.pop().expect("one option"))
    } else {
        PropertyDef::Union(arms)
    }
}

fn canonical_atoms(
    atom: &PropertyAtom,
    named: Option<&IndexMap<String, PropertyDef>>,
    depth: usize,
) -> Vec<PropertyAtom> {
    let bare = atom.constraints.is_empty();
    if !atom.is_array
        && bare
        && let TypeExpr::Union(arms) = &atom.ty
    {
        return arms.iter().flat_map(|arm| canonical_atoms(arm, named, depth)).collect();
    }
    if bare
        && atom.ty.is_local_reference()
        && depth < MAX_REFERENCE_DEPTH
        && let TypeExpr::Imported { name, .. } = &atom.ty
        && let Some(target) = named.and_then(|named| named.get(name))
    {
        let mut options = match canonical_def(target, named, depth + 1) {
            PropertyDef::Single(atom) => vec![atom],
            PropertyDef::Union(arms) => arms,
        };
        if !atom.is_array {
            return options;
        }
        let array_constraints = canonical_constraints(&atom.array_constraints);
        if options.len() == 1 && !options[0].is_array {
            let mut item = options.pop().expect("one option");
            item.is_array = true;
            item.array_constraints = array_constraints;
            return vec![item];
        }
        return vec![PropertyAtom {
            ty: TypeExpr::Union(options),
            is_array: true,
            constraints: Vec::new(),
            array_constraints,
            description: None,
        }];
    }
    let ty = match &atom.ty {
        TypeExpr::InlineObject(shape) => {
            let mut shape = shape.clone();
            for def in shape.properties.values_mut() {
                *def = canonical_def(def, named, depth);
            }
            shape.properties.sort_keys();
            for pattern in &mut shape.pattern_keys {
                pattern.def = canonical_def(&pattern.def, named, depth);
            }
            TypeExpr::InlineObject(shape)
        }
        TypeExpr::Union(arms) => {
            let mut arms: Vec<PropertyAtom> =
                arms.iter().flat_map(|arm| canonical_atoms(arm, named, depth)).collect();
            arms.sort_by_cached_key(super::serialize_property_atom);
            arms.dedup();
            TypeExpr::Union(arms)
        }
        TypeExpr::Tuple(tuple) => {
            let mut tuple = tuple.clone();
            for element in &mut tuple.elements {
                element.def = canonical_def(&element.def, named, depth);
            }
            if let Some(spread) = &mut tuple.spread
                && let Some(first) = canonical_atoms(spread, named, depth).into_iter().next()
            {
                **spread = first;
            }
            TypeExpr::Tuple(tuple)
        }
        TypeExpr::Function(function) => {
            let mut function = function.as_ref().clone();
            function.examples.clear();
            function.narrows = None;
            function.returns = canonical_def(&function.returns, named, depth);
            if let super::types::FunctionParameters::List(parameters) = &mut function.parameters {
                for parameter in parameters {
                    parameter.name.clear();
                    parameter.def = canonical_def(&parameter.def, named, depth);
                }
            }
            TypeExpr::Function(Box::new(function))
        }
        other => other.clone(),
    };
    vec![PropertyAtom {
        ty,
        is_array: atom.is_array,
        constraints: canonical_constraints(&atom.constraints),
        array_constraints: canonical_constraints(&atom.array_constraints),
        description: None,
    }]
}

fn canonical_constraints(constraints: &[super::types::Constraint]) -> Vec<super::types::Constraint> {
    use super::types::Constraint;
    let mut kept: Vec<Constraint> = constraints
        .iter()
        .filter(|constraint| !matches!(constraint, Constraint::Category(_) | Constraint::Example(_)))
        .cloned()
        .collect();
    kept.sort_by_cached_key(|constraint| format!("{constraint:?}"));
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::simplified::grammar::parse_type_definition;

    fn arms(text: &str) -> Vec<PropertyAtom> {
        match parse_type_definition("f", text) {
            Ok(PropertyDef::Union(arms)) => arms,
            Ok(PropertyDef::Single(atom)) => vec![atom],
            Err(error) => panic!("{text}: {error}"),
        }
    }

    #[test]
    fn contracts_ignore_names_documentation_and_option_order() {
        let first = arms("function(parameters(a: string | number, b?: any); returns(string))");
        let second = arms(
            "function(parameters(x: number | string, y?: unknown); returns(boolean); fallible; category(X))",
        );
        let (TypeExpr::Function(first), TypeExpr::Function(second)) = (&first[0].ty, &second[0].ty) else {
            panic!("functions");
        };
        assert_eq!(binding_contract(first, None), binding_contract(second, None));
    }

    #[test]
    fn local_references_resolve_before_comparison() {
        let mut named = IndexMap::new();
        named.insert(
            "Count".to_string(),
            PropertyDef::Single(crate::markdown::schemas::simplified::grammar::parse_type_expr(
                "Count", "number(integer; min(1)) -> how many",
            ).expect("parses")),
        );
        let union = [
            arms("function(parameters(count: Count); returns(string))").remove(0),
            arms("function(parameters(n: number(min(1); integer)); returns(string[]))").remove(0),
        ];
        assert!(duplicate_overload(&union, None).is_none(), "unresolved names differ");
        assert!(duplicate_overload(&union, Some(&named)).is_some());
    }

    #[test]
    fn duplicate_overload_arm_names_the_later_duplicate() {
        let union = [
            arms("function(parameters(a: string); returns(string))").remove(0),
            arms("function(parameters(a: number); returns(string))").remove(0),
            arms("function(parameters(b: string); returns(number))").remove(0),
        ];
        let (index, message) = duplicate_overload_arm(&union, None).expect("arms 0 and 2 bind alike");
        assert_eq!(index, 2);
        assert_eq!(Some(message), duplicate_overload(&union, None));
    }
}

#[cfg(test)]
mod narrows_contract_tests {
    use super::*;
    use crate::markdown::schemas::simplified::grammar::parse_type_definition;

    fn arm(text: &str) -> PropertyAtom {
        match parse_type_definition("f", text).unwrap_or_else(|error| panic!("{text}: {error}")) {
            PropertyDef::Single(atom) => atom,
            other => panic!("{text}: {other:?}"),
        }
    }

    #[test]
    fn overloads_differing_only_in_narrows_remain_duplicates() {
        let union = [
            arm("function(parameters(x: unknown); returns(boolean); narrows(x: string; true-iff))"),
            arm("function(parameters(x: unknown); returns(boolean); narrows(x: file; true-implies))"),
        ];
        assert!(duplicate_overload(&union, None).is_some());
        let with_and_without = [
            arm("function(parameters(x: unknown); returns(boolean); narrows(x; false-implies))"),
            arm("function(parameters(x: unknown); returns(boolean))"),
        ];
        assert!(duplicate_overload(&with_and_without, None).is_some());
    }

    #[test]
    fn canonical_atoms_clear_the_clause() {
        let atom = arm("function(parameters(x: unknown); returns(boolean); narrows(x; false-implies))");
        let canonical = canonical_atoms(&atom, None, 0).remove(0);
        let TypeExpr::Function(function) = canonical.ty else { panic!("a function") };
        assert_eq!(function.narrows, None);
    }
}
