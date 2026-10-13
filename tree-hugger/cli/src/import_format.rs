//! Import grouping and per-language import-statement formatting.

use std::collections::{BTreeMap, HashSet};

use tree_hugger::{ImportSymbol, ProgrammingLanguage};

pub(crate) fn group_imports(imports: &[ImportSymbol]) -> Vec<Vec<&ImportSymbol>> {
    let mut groups: BTreeMap<(usize, usize), Vec<&ImportSymbol>> = BTreeMap::new();
    for import in imports {
        let key = import
            .statement_range
            .as_ref()
            .map(|range| (range.start_line, range.start_column))
            .unwrap_or((import.range.start_line, import.range.start_column));
        groups.entry(key).or_default().push(import);
    }

    let mut result = Vec::new();
    for (_, mut group) in groups {
        group.sort_by_key(|import| (import.range.start_line, import.range.start_column));
        result.push(group);
    }

    result
}

pub(crate) fn format_import_locations(imports: &[&ImportSymbol]) -> (String, usize) {
    let mut positions: Vec<(usize, usize)> = imports
        .iter()
        .map(|import| (import.range.start_line, import.range.start_column))
        .collect();
    positions.sort();

    let (first_line, _) = positions.first().copied().unwrap_or((1, 1));
    let location = if positions.iter().all(|(line, _)| *line == first_line) {
        let columns = positions
            .iter()
            .map(|(_, column)| column.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{}:{}]", first_line, columns)
    } else {
        let entries = positions
            .iter()
            .map(|(line, column)| format!("{}:{}", line, column))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{}]", entries)
    };

    (location, first_line)
}

pub(crate) fn format_import_group_display(imports: &[&ImportSymbol]) -> String {
    let language = imports
        .first()
        .map(|import| import.language)
        .unwrap_or(ProgrammingLanguage::Rust);
    match language {
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            format_ecma_import_group(imports)
        }
        ProgrammingLanguage::Python => format_python_import_group(imports),
        ProgrammingLanguage::Rust => format_rust_import_group(imports),
        ProgrammingLanguage::Go => format_go_import_group(imports),
        ProgrammingLanguage::Java => format_java_import_group(imports),
        ProgrammingLanguage::CSharp => format_csharp_import_group(imports),
        ProgrammingLanguage::Php => format_php_import_group(imports),
        ProgrammingLanguage::Scala => format_scala_import_group(imports),
        ProgrammingLanguage::Swift => format_swift_import_group(imports),
        _ => format_generic_import_group(imports),
    }
}

pub(crate) fn format_ecma_import_group(imports: &[&ImportSymbol]) -> String {
    let source = imports.first().and_then(|import| import.source.as_deref());
    let is_namespace = imports.len() == 1
        && imports[0]
            .original_name
            .as_deref()
            .is_some_and(|name| name == "*");

    if is_namespace {
        let alias = &imports[0].name;
        if let Some(source) = source {
            return format!("import * as {} from \"{}\"", alias, source);
        }
        return format!("import * as {}", alias);
    }

    let specs = imports
        .iter()
        .map(|import| {
            if let Some(alias) = import.alias.as_deref() {
                let original = import.original_name.as_deref().unwrap_or(&import.name);
                format!("{} as {}", original, alias)
            } else {
                import.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    if let Some(source) = source {
        format!("import {{ {} }} from \"{}\"", specs, source)
    } else {
        format!("import {{ {} }}", specs)
    }
}

pub(crate) fn format_python_import_group(imports: &[&ImportSymbol]) -> String {
    let sources: HashSet<&str> = imports
        .iter()
        .filter_map(|import| import.source.as_deref())
        .collect();

    // An `import` statement entry binds its module (`import a`, `import a.b
    // as c`, whose original is the module) or the module's first segment
    // (`import a.b` binds `a`); a `from` entry binds a name of its module.
    // An unaliased `from a.b import a` is indistinguishable from
    // `import a.b` here and is shown as the latter.
    let is_import_entry = |import: &ImportSymbol| {
        let Some(source) = import.source.as_deref() else { return false };
        match import.alias.as_deref() {
            Some(_) => import.original_name.as_deref() == Some(source),
            None => {
                import.name == source
                    || source.strip_prefix(import.name.as_str()).is_some_and(|rest| rest.starts_with('.'))
            }
        }
    };
    let import_spec = |import: &ImportSymbol| match import.alias.as_deref() {
        Some(alias) => format!("{} as {}", import.original_name.as_deref().unwrap_or(&import.name), alias),
        None => import.source.clone().unwrap_or_else(|| import.name.clone()),
    };
    let from_spec = |import: &ImportSymbol| match (import.original_name.as_deref(), import.alias.as_deref()) {
        (Some("*"), _) => "*".to_string(),
        (original, Some(alias)) => format!("{} as {}", original.unwrap_or(&import.name), alias),
        _ => import.name.clone(),
    };

    if sources.len() == 1 && !imports.iter().all(|import| is_import_entry(import)) {
        let source = *sources.iter().next().unwrap();
        let spec_list = imports.iter().map(|import| from_spec(import)).collect::<Vec<_>>().join(", ");
        format!("from {} import {}", source, spec_list)
    } else {
        let spec_list = imports.iter().map(|import| import_spec(import)).collect::<Vec<_>>().join(", ");
        format!("import {}", spec_list)
    }
}

/// `use` with each entry's whole path (`source::original as alias`,
/// `source::*` for a glob), grouped under the shared source when every
/// entry has the same one.
pub(crate) fn format_rust_import_group(imports: &[&ImportSymbol]) -> String {
    let tail = |import: &ImportSymbol| {
        let original = import.original_name.as_deref();
        match (original, import.alias.as_deref()) {
            (Some("*"), _) => "*".to_string(),
            (original, Some(alias)) => format!("{} as {}", original.unwrap_or(&import.name), alias),
            _ => import.name.clone(),
        }
    };
    // A glob's source is its whole path; the others' is their module.
    let full = |import: &ImportSymbol| match import.source.as_deref() {
        Some(source) => format!("{}::{}", source, tail(import)),
        None => tail(import),
    };

    let first = imports.first().and_then(|import| import.source.as_deref());
    let shared =
        imports.len() > 1 && first.is_some() && imports.iter().all(|import| import.source.as_deref() == first);
    if shared {
        let tails = imports.iter().map(|import| tail(import)).collect::<Vec<_>>().join(", ");
        return format!("use {}::{{{}}}", first.unwrap_or_default(), tails);
    }
    match imports {
        [single] => format!("use {}", full(single)),
        _ => format!("use {{{}}}", imports.iter().map(|import| full(import)).collect::<Vec<_>>().join(", ")),
    }
}

pub(crate) fn format_go_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| {
            let path = import.source.as_deref().unwrap_or(&import.name);
            let quoted = format!("\"{}\"", path);
            if import.original_name.as_deref() == Some("*") {
                format!(". {}", quoted)
            } else if let Some(alias) = import.alias.as_deref() {
                format!("{} {}", alias, quoted)
            } else {
                quoted
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("import {}", specs)
}

pub(crate) fn format_java_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| {
            if import.original_name.as_deref() == Some("*") {
                if let Some(source) = import.source.as_deref() {
                    format!("{}.*", source)
                } else {
                    "*".to_string()
                }
            } else if let Some(source) = import.source.as_deref() {
                format!("{}.{}", source, import.name)
            } else {
                import.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("import {}", specs)
}

pub(crate) fn format_csharp_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| {
            let target = import.source.as_deref().unwrap_or(&import.name);
            match import.alias.as_deref() {
                Some(alias) => format!("{alias} = {target}"),
                None => target.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("using {}", specs)
}

pub(crate) fn format_php_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| {
            let base = import.source.as_deref().unwrap_or(&import.name);
            if let Some(alias) = import.alias.as_deref() {
                format!("{} as {}", base, alias)
            } else {
                base.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("use {}", specs)
}

/// `import` with each entry's path (`a.b.C`), a rename as `a.D as E` and
/// an on-demand import as `a.b.*` or `a.given`.
pub(crate) fn format_scala_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| {
            let path = import.source.as_deref().unwrap_or(&import.name);
            match (import.original_name.as_deref(), import.alias.as_deref()) {
                (Some(opened @ ("*" | "given")), _) => format!("{}.{}", path, opened),
                (_, Some(alias)) => format!("{} as {}", path, alias),
                _ => path.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("import {}", specs)
}

pub(crate) fn format_swift_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| import.source.as_deref().unwrap_or(&import.name).to_string())
        .collect::<Vec<_>>()
        .join(", ");

    format!("import {}", specs)
}

pub(crate) fn format_generic_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| import.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    format!("import {}", specs)
}
