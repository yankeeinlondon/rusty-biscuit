use super::*;

/// Scala visibility. `private` and `protected` map to themselves, and so do
/// `private[this]` and `protected[this]`. A qualifier naming a segment of the
/// file's `package` clause is Package; any other qualifier (an enclosing
/// class, say) grants access no level describes, so it is Unknown and its text
/// stays in the declaration text. Unmarked declarations are Public. The
/// `package` clause itself has no access level.
pub(super) fn extract_scala_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_>,
) -> Option<Visibility> {
    if declaration.kind() == "package_clause" {
        return None;
    }
    let source = ctx.source;
    let Some(access) = find_child_by_kind(declaration, "modifiers")
        .and_then(|modifiers| find_child_by_kind(modifiers, "access_modifier"))
    else {
        return Some(Visibility::Public);
    };

    let unqualified = match access.child(0)?.kind() {
        "private" => Visibility::Private,
        "protected" => Visibility::Protected,
        _ => return None,
    };
    let Some(qualifier) = find_child_by_kind(access, "access_qualifier") else {
        return Some(unqualified);
    };

    let scope = qualifier
        .utf8_text(source.as_bytes())
        .ok()?
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    if scope == "this" {
        Some(unqualified)
    } else if package_segments(ctx.root, source).any(|segment| segment == scope) {
        Some(Visibility::Package)
    } else {
        None
    }
}

/// Every identifier segment of the file's `package` clauses.
fn package_segments<'src>(root: Node<'_>, source: &'src str) -> impl Iterator<Item = &'src str> {
    let mut segments = Vec::new();
    let mut cursor = root.walk();
    for clause in root
        .children(&mut cursor)
        .filter(|child| child.kind() == "package_clause")
    {
        collect_identifiers(clause, source, &mut segments);
    }
    segments.into_iter()
}

fn collect_identifiers<'src>(node: Node<'_>, source: &'src str, segments: &mut Vec<&'src str>) {
    if node.kind() == "identifier" {
        if let Ok(text) = node.utf8_text(source.as_bytes()) {
            segments.push(text);
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        // A block-form package clause holds declarations, not name segments.
        if child.kind() != "template_body" {
            collect_identifiers(child, source, segments);
        }
    }
}
