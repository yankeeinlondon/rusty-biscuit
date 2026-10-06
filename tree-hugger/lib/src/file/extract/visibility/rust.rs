use super::*;

/// Rust visibility.
///
/// `pub` is Public; `pub(crate)`, `pub(super)` and `pub(in path)` are Crate
/// (an upper bound: the original text stays in the declaration text);
/// `pub(self)` is Private. An unmarked item or field is Private, except:
///
/// - enum variants and their fields take the enum's level;
/// - associated items of a trait declaration take the trait's level;
/// - items of `impl Trait for Type` take the level of a same-file `Trait`,
///   and are Unknown when the trait is declared elsewhere.
pub(super) fn extract_rust_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
) -> Option<Visibility> {
    let source = ctx.source;
    if let Some(level) = modifier_level(declaration, source) {
        return Some(level);
    }

    match declaration.kind() {
        // `macro_rules!` macros are textually scoped, not items with access.
        "macro_definition" => return None,
        "enum_variant" => return enclosing_enum_level(declaration, source),
        "field_declaration" if variant_of_field(declaration).is_some() => {
            return enclosing_enum_level(declaration, source);
        }
        _ => {}
    }

    if let Some(impl_item) = impl_of_member(declaration) {
        return match impl_trait_name(impl_item, source) {
            Some(trait_name) => same_file_trait_level(ctx, &trait_name),
            None => Some(Visibility::Private),
        };
    }

    if let Some(trait_item) = trait_of_member(declaration) {
        return Some(item_level(trait_item, source));
    }

    Some(Visibility::Private)
}

/// The level an item's own modifier gives it, Private when unmarked.
fn item_level(item: Node<'_>, source: &str) -> Visibility {
    modifier_level(item, source).unwrap_or(Visibility::Private)
}

fn modifier_level(declaration: Node<'_>, source: &str) -> Option<Visibility> {
    let modifier = find_child_by_kind(declaration, "visibility_modifier")?;
    let text = modifier.utf8_text(source.as_bytes()).ok()?;
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let Some(restriction) = compact
        .strip_prefix("pub(")
        .and_then(|rest| rest.strip_suffix(')'))
    else {
        return (compact == "pub").then_some(Visibility::Public);
    };
    let path = restriction.strip_prefix("in").unwrap_or(restriction);
    Some(if path == "self" {
        Visibility::Private
    } else {
        Visibility::Crate
    })
}

fn variant_of_field(field: Node<'_>) -> Option<Node<'_>> {
    field
        .parent()
        .filter(|list| list.kind() == "field_declaration_list")?
        .parent()
        .filter(|parent| parent.kind() == "enum_variant")
}

fn enclosing_enum_level(declaration: Node<'_>, source: &str) -> Option<Visibility> {
    let mut current = declaration.parent();
    while let Some(node) = current {
        if node.kind() == "enum_item" {
            return Some(item_level(node, source));
        }
        current = node.parent();
    }
    None
}

/// The `trait_item` whose body directly declares `declaration`, if any.
fn trait_of_member(declaration: Node<'_>) -> Option<Node<'_>> {
    declaration
        .parent()
        .filter(|list| list.kind() == "declaration_list")?
        .parent()
        .filter(|parent| parent.kind() == "trait_item")
}

/// The level of the trait named `name` declared anywhere in this file; Unknown
/// when there is none, or when same-named traits disagree. The file's traits
/// are collected once per file into [`VisibilityMemo`].
fn same_file_trait_level(ctx: &DeclarationContext<'_, '_>, name: &str) -> Option<Visibility> {
    let traits = ctx.memo.rust_trait_levels.get_or_init(|| {
        let mut traits = HashMap::new();
        collect_trait_levels(ctx.root, ctx.source, &mut traits);
        traits
    });
    let levels = traits.get(name)?;
    count_shared_context_steps(levels.len());
    let first = *levels.first()?;
    levels.iter().all(|level| *level == first).then_some(first)
}

fn collect_trait_levels<'src>(
    node: Node<'_>,
    source: &'src str,
    traits: &mut HashMap<&'src str, Vec<Visibility>>,
) {
    count_shared_context_steps(1);
    if node.kind() == "trait_item"
        && let Some(name) = node
            .child_by_field_name("name")
            .and_then(|name_node| name_node.utf8_text(source.as_bytes()).ok())
    {
        traits.entry(name).or_default().push(item_level(node, source));
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        // A trait is an item, so only item containers can hold one.
        if matches!(
            child.kind(),
            "trait_item" | "mod_item" | "declaration_list" | "function_item" | "block"
        ) {
            collect_trait_levels(child, source, traits);
        }
    }
}
