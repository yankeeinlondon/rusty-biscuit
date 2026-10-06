use super::*;

/// Scala visibility. `private` and `protected` map to themselves, and so do
/// `private[this]` and `protected[this]`. A qualifier naming a segment of
/// one of the file's `package` clauses (statement or block form) is Package;
/// any other qualifier (an enclosing class, say) grants access no level
/// describes, so it is Unknown and its text stays in the declaration text.
/// Unmarked declarations are Public. The `package` clause itself has no
/// access level.
pub(super) fn extract_scala_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
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
    count_shared_context_steps(1);
    if scope == "this" {
        Some(unqualified)
    } else if ctx
        .memo
        .scala_package_segments
        .get_or_init(|| package_segments(ctx.root, source))
        .contains(scope)
    {
        Some(Visibility::Package)
    } else {
        None
    }
}

/// Every identifier segment of the file's `package` clauses, block-form
/// clauses nested in a package body included, read once per file into
/// [`VisibilityMemo`].
fn package_segments<'src>(root: Node<'_>, source: &'src str) -> HashSet<&'src str> {
    let mut segments = HashSet::new();
    let mut steps = 0;
    collect_package_clauses(root, source, &mut segments, &mut steps);
    count_shared_context_steps(steps);
    segments
}

/// Reads the name of every `package_clause` child of `parent`, descending
/// into each clause's body (a block package's declarations are not segments,
/// but the packages nested among them are).
fn collect_package_clauses<'src>(
    parent: Node<'_>,
    source: &'src str,
    segments: &mut HashSet<&'src str>,
    steps: &mut usize,
) {
    let mut cursor = parent.walk();
    for clause in parent.children(&mut cursor) {
        *steps += 1;
        if clause.kind() != "package_clause" {
            continue;
        }
        if let Some(name) = clause.child_by_field_name("name") {
            collect_identifiers(name, source, segments);
        }
        if let Some(body) = clause.child_by_field_name("body") {
            collect_package_clauses(body, source, segments, steps);
        }
    }
}

fn collect_identifiers<'src>(
    node: Node<'_>,
    source: &'src str,
    segments: &mut HashSet<&'src str>,
) {
    if node.kind() == "identifier" {
        if let Ok(text) = node.utf8_text(source.as_bytes()) {
            segments.insert(text);
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_identifiers(child, source, segments);
    }
}
