//! Rust `impl` blocks: the implemented type names the container of everything
//! inside the block (by its last path segment, for identity), and
//! [`impl_header`] reads what the per-file relation resolver needs to decide
//! the members' owner and the `relations.implements` of members and self
//! type: the trait as written, negativity, and the self type's text and path.

use super::*;
use crate::file::parents::IndexedParent as _;

/// The `impl_item` whose body directly declares `declaration`, if any.
pub(crate) fn impl_of_member(declaration: Node<'_>) -> Option<Node<'_>> {
    declaration
        .indexed_parent()
        .filter(|list| list.kind() == "declaration_list")?
        .indexed_parent()
        .filter(|parent| parent.kind() == "impl_item")
}

/// The implemented type's name: the last path segment of the self type,
/// without generic arguments (`impl<T> crate::geo::Wrapper<T>` → `Wrapper`).
pub(crate) fn impl_self_type_name(impl_item: Node<'_>, source: &str) -> Option<String> {
    last_path_segment(impl_item.child_by_field_name("type")?, source)
}

/// The self type's path as written, one entry per segment, without generic
/// arguments (`impl<T> right::Wrapper<T>` → `["right", "Wrapper"]`), for the
/// qualified name. Path keywords (`crate`, `self`, `super`) are kept for the
/// caller to resolve. A self type that is not a path (tuple, array,
/// qualified `<T as Trait>` path) is its last segment alone, as in
/// [`impl_self_type_name`], which stays the bare container name.
pub(crate) fn impl_self_type_path(impl_item: Node<'_>, source: &str) -> Option<Vec<String>> {
    let self_type = impl_item.child_by_field_name("type")?;
    path_segments(self_type, source)
        .filter(|segments| !segments.is_empty())
        .or_else(|| last_path_segment(self_type, source).map(|segment| vec![segment]))
}

fn path_segments(node: Node<'_>, source: &str) -> Option<Vec<String>> {
    match node.kind() {
        "generic_type" | "reference_type" | "pointer_type" => {
            path_segments(node.child_by_field_name("type")?, source)
        }
        "scoped_type_identifier" | "scoped_identifier" => {
            let mut segments = match node.child_by_field_name("path") {
                Some(path) => path_segments(path, source)?,
                None => Vec::new(),
            };
            segments.extend(path_segments(node.child_by_field_name("name")?, source)?);
            Some(segments)
        }
        "type_identifier" | "identifier" | "primitive_type" | "crate" | "self" | "super" => {
            let text = node.utf8_text(source.as_bytes()).ok()?.trim();
            (!text.is_empty()).then(|| vec![text.to_string()])
        }
        _ => None,
    }
}

/// The implemented trait's name, by the same rule as the self type
/// (`impl fmt::Display for Point` → `Display`); `None` for an inherent impl.
pub(crate) fn impl_trait_name(impl_item: Node<'_>, source: &str) -> Option<String> {
    last_path_segment(impl_item.child_by_field_name("trait")?, source)
}

fn last_path_segment(node: Node<'_>, source: &str) -> Option<String> {
    match node.kind() {
        "generic_type" | "reference_type" | "pointer_type" => {
            last_path_segment(node.child_by_field_name("type")?, source)
        }
        "scoped_type_identifier" | "scoped_identifier" => {
            last_path_segment(node.child_by_field_name("name")?, source)
        }
        _ => {
            let text = node.utf8_text(source.as_bytes()).ok()?;
            let without_generics = text.split('<').next().unwrap_or(text);
            let segment = without_generics.rsplit("::").next()?.trim();
            (!segment.is_empty()).then(|| segment.to_string())
        }
    }
}

/// What an `impl` header declares, for relations and impl ownership.
pub(crate) struct ImplHeader {
    /// The implemented trait as written (`fmt::Display`, `From<u8>`);
    /// `None` for an inherent impl.
    pub(crate) trait_base: Option<WrittenBase>,
    /// `impl !Trait for T`: asserts no conformance.
    pub(crate) is_negative: bool,
    /// The self type as written (`crate::m::W<T>`, `&W`).
    pub(crate) self_text: String,
    /// The self type's path when it is a (qualified, instantiated) name;
    /// `None` for a reference, tuple, `dyn` or other composite type, which
    /// attaches to no component type.
    pub(crate) self_path: Option<BasePath>,
    /// The impl's own type parameter names (`impl<T> Tr for T`).
    pub(crate) type_params: Vec<String>,
}

/// The header of `impl_item`; `None` when it has no self type.
pub(crate) fn impl_header(impl_item: Node<'_>, source: &str) -> Option<ImplHeader> {
    let language = ProgrammingLanguage::Rust;
    let self_type = impl_item.child_by_field_name("type")?;
    let self_text = self_type.utf8_text(source.as_bytes()).ok()?.trim().to_string();
    let mut cursor = impl_item.walk();
    let is_negative = impl_item.children(&mut cursor).any(|child| child.kind() == "!");
    let type_params = impl_type_parameters(impl_item, source);
    Some(ImplHeader {
        trait_base: impl_item
            .child_by_field_name("trait")
            .and_then(|trait_node| written_base(trait_node, language, source)),
        is_negative,
        self_path: base_path(self_type, language, source),
        self_text,
        type_params,
    })
}

/// The names of `impl_item`'s own type parameters (`impl<T, 'a, const N:
/// usize>` → `T`): a lifetime or `const` parameter names no type.
pub(crate) fn impl_type_parameters(impl_item: Node<'_>, source: &str) -> Vec<String> {
    impl_item
        .child_by_field_name("type_parameters")
        .map(|list| {
            let mut cursor = list.walk();
            list.named_children(&mut cursor)
                .filter(|param| param.kind() == "type_parameter")
                .filter_map(|param| param.child_by_field_name("name"))
                .filter_map(|name| name.utf8_text(source.as_bytes()).ok())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}
