use super::*;
use sniff::filesystem::repo::Package;

use super::datetime::{string_array, strip_trailing_sep};
use super::snapshot::ContextCapture;

pub(super) const KEYS: &[&str] = &[
    "repo", "repo_root", "is_monorepo", "package_root", "package_area_root", "packages",
    "package_areas", "current_package", "current_package_area", "area", "area_description",
    "area_root", "current_packages", "depends_on", "used_by",
];

/// Render a directory path for `ctx.*` as portable text.
///
/// Native Windows spellings (`\\` separators, `\\?\\` verbatim prefixes) are
/// mangled once interpolated into a Markdown body, because CommonMark treats
/// `\\` before punctuation as an escape; forward slashes are valid on every
/// platform and survive composition intact.
fn portable_dir(path: &std::path::Path) -> String {
    strip_trailing_sep(&biscuit_file::to_portable_string(path))
}

pub(super) fn populate_repo(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let repo = cap.repo_info.as_ref();

    // repo and repo_root
    values.insert(
        "repo".into(),
        cap.repo_name
            .as_ref()
            .map_or(Value::Null, |r| Value::String(r.clone())),
    );
    values.insert(
        "repo_root".into(),
        cap.repo_root.as_ref().map_or(Value::Null, |r| {
            Value::String(portable_dir(r))
        }),
    );

    // is_monorepo
    values.insert(
        "is_monorepo".into(),
        Value::Bool(repo.is_some_and(|r| r.is_monorepo)),
    );

    // Package fields (null if not monorepo)
    let is_mono = repo.is_some_and(|r| r.is_monorepo);
    if is_mono {
        values.insert(
            "package_root".into(),
            cap.current_package.as_ref().map_or(Value::Null, |p| {
                Value::String(portable_dir(&p.path))
            }),
        );

        values.insert(
            "package_area_root".into(),
            cap.current_package_area
                .as_ref()
                .map_or(Value::Null, |area| {
                    let root = repo.unwrap().root.join(area);
                    Value::String(portable_dir(&root))
                }),
        );

        let packages: Vec<String> = repo
            .and_then(|r| r.packages.as_ref())
            .map(|pkgs| pkgs.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default();
        values.insert("packages".into(), string_array(packages));

        let areas: Vec<String> = repo
            .and_then(|r| r.packages.as_ref())
            .map(|pkgs| {
                let mut areas: Vec<String> = pkgs.iter().map(|p| p.package_area.clone()).collect();
                areas.sort();
                areas.dedup();
                areas
            })
            .unwrap_or_default();
        values.insert("package_areas".into(), string_array(areas));

        values.insert(
            "current_package".into(),
            cap.current_package
                .as_ref()
                .map_or(Value::Null, |p| Value::String(p.name.clone())),
        );
        values.insert(
            "current_package_area".into(),
            cap.current_package_area
                .as_ref()
                .map_or(Value::Null, |a| Value::String(a.clone())),
        );
    } else {
        values.insert("package_root".into(), Value::Null);
        values.insert("package_area_root".into(), Value::Null);
        values.insert("packages".into(), Value::Null);
        values.insert("package_areas".into(), Value::Null);
        values.insert("current_package".into(), Value::Null);
        values.insert("current_package_area".into(), Value::Null);
    }

    populate_monorepo_area(cap, values);
}

/// Populate the monorepo-scoped `area*`, `current_packages`, `depends_on`, and
/// `used_by` variables.
///
/// These are documented as monorepo-only. Outside a monorepo the `area*`
/// variables fall back per spec (empty strings; `area_root` is the repo root)
/// and the list-shaped variables render as empty strings.
pub(super) fn populate_monorepo_area(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let repo = cap.repo_info.as_ref();
    let is_monorepo = repo.is_some_and(|r| r.is_monorepo);

    let repo_root_str = || {
        cap.repo_root
            .as_ref()
            .map(|r| portable_dir(r))
            .unwrap_or_default()
    };

    let (area, area_description, area_root) = if !is_monorepo {
        (String::new(), String::new(), repo_root_str())
    } else if let Some(pkg) = cap.current_package.as_ref() {
        // Inside a package folder: the area is the package itself.
        (
            pkg.name.clone(),
            format!("{} package", pkg.name),
            portable_dir(&pkg.path),
        )
    } else if let Some(area_name) = cap.current_package_area.as_deref().filter(|a| *a != "root") {
        // Inside a package area but not a package folder.
        let area_path = cap
            .repo_root
            .as_ref()
            .map(|r| portable_dir(&r.join(area_name)))
            .unwrap_or_default();
        (
            area_name.to_string(),
            format!("{area_name} package area"),
            area_path,
        )
    } else {
        // Monorepo root.
        (String::new(), String::new(), repo_root_str())
    };

    values.insert("area".into(), Value::String(area.clone()));
    values.insert("area_description".into(), Value::String(area_description));
    values.insert("area_root".into(), Value::String(area_root));

    // current_packages: array of packages under the cwd, each `name (relative)`.
    // `{{ as_unordered_list(ctx.current_packages) }}` reproduces the old bullets.
    let current_packages: Vec<String> = repo
        .and_then(|r| r.packages.as_ref())
        .map(|pkgs| {
            pkgs.iter()
                .filter(|p| p.path.starts_with(&cap.base_dir))
                .map(|p| format!("{} ({})", p.name, p.relative))
                .collect()
        })
        .unwrap_or_default();
    values.insert("current_packages".into(), string_array(current_packages));

    // depends_on / used_by are scoped to the current `area`: the current
    // package when in one, all packages in the area when in an area, else all.
    let scope: Vec<&Package> = if let Some(pkg) = cap.current_package.as_ref() {
        vec![pkg]
    } else if !area.is_empty() {
        repo.and_then(|r| r.packages.as_ref())
            .map(|pkgs| pkgs.iter().filter(|p| p.package_area == area).collect())
            .unwrap_or_default()
    } else {
        repo.and_then(|r| r.packages.as_ref())
            .map(|pkgs| pkgs.iter().collect())
            .unwrap_or_default()
    };

    let depends_on = render_dependency_objects(&scope, |p| p.depends_on.as_slice(), "dependencies");
    values.insert("depends_on".into(), depends_on);

    let used_by = render_dependency_objects(&scope, |p| p.used_by.as_slice(), "users");
    values.insert("used_by".into(), used_by);
}

/// Build the object-array shape for `depends_on` / `used_by` (spec Group 3).
///
/// Each scoped package becomes `{ package: <name>, <list_field>: [<edges>] }`.
/// `list_field` is `dependencies` for `depends_on` and `users` for `used_by`,
/// matching the base schema's documented object shape. Rendering (nested bullets
/// via `as_unordered_list`, line-separated by default) is the caller's choice;
/// the composed verb wording of the old pre-rendered form is dropped (spec).
fn render_dependency_objects(
    scope: &[&Package],
    edges: impl Fn(&Package) -> &[String],
    list_field: &str,
) -> Value {
    let items: Vec<Value> = scope
        .iter()
        .map(|&pkg| {
            let mut obj = Map::new();
            obj.insert("package".into(), Value::String(pkg.name.clone()));
            obj.insert(
                list_field.into(),
                string_array(edges(pkg).iter().map(|d| d.to_string()).collect()),
            );
            Value::Object(obj)
        })
        .collect();
    Value::Array(items)
}

// ── File changes context ──────────────────────────────────────────


#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::context::capture::{capture_runtime_context_for_groups, ContextGroup};

    #[test]
    fn strip_trailing_sep_removes_single_separator() {
        assert_eq!(strip_trailing_sep("/repo/"), "/repo");
        assert_eq!(strip_trailing_sep("/repo"), "/repo");
        assert_eq!(strip_trailing_sep("C:\\repo\\"), "C:\\repo");
        assert_eq!(strip_trailing_sep(""), "");
    }

    #[test]
    fn repo_root_has_no_trailing_slash() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let (values, _, _, _) = capture_runtime_context_for_groups(root, &[ContextGroup::Repo]);
        if let Some(Value::String(rr)) = values.get("repo_root") {
            assert!(
                !rr.ends_with('/'),
                "repo_root must not end with '/': got {rr:?}"
            );
            assert!(!rr.is_empty(), "repo_root should be non-empty in a repo");
        }
    }

    #[test]
    fn area_vars_empty_when_not_monorepo() {
        let cap = ContextCapture::for_test_non_monorepo();
        let mut values = Map::new();
        populate_repo(&cap, &mut values);
        assert_eq!(values.get("area"), Some(&Value::String(String::new())));
        assert_eq!(
            values.get("area_description"),
            Some(&Value::String(String::new()))
        );
        // area_root falls back to repo root (no trailing slash) when not a monorepo.
        if let Some(Value::String(ar)) = values.get("area_root") {
            assert!(
                !ar.ends_with('/'),
                "area_root must not end with '/': {ar:?}"
            );
        }
    }

    #[test]
    fn current_packages_captured_as_string_array() {
        let cap = ContextCapture::for_test_monorepo_with_packages(&[
            ("alpha", "alpha/lib"),
            ("alpha-cli", "alpha/cli"),
        ]);
        let mut values = Map::new();
        populate_repo(&cap, &mut values);
        let listing = values
            .get("current_packages")
            .and_then(Value::as_array)
            .expect("current_packages must be an array");
        assert!(
            listing.contains(&Value::String("alpha (alpha/lib)".into())),
            "got: {listing:?}"
        );
        assert!(
            listing.contains(&Value::String("alpha-cli (alpha/cli)".into())),
            "got: {listing:?}"
        );
    }

    #[test]
    fn depends_on_captured_as_object_array_scoped_to_area() {
        let cap = ContextCapture::for_test_monorepo_in_package("alpha", &["beta", "gamma"], &[]);
        let mut values = Map::new();
        populate_repo(&cap, &mut values);
        let items = values
            .get("depends_on")
            .and_then(Value::as_array)
            .expect("depends_on must be an array");
        assert_eq!(items.len(), 1, "got: {items:?}");
        let obj = items[0].as_object().expect("item must be an object");
        assert_eq!(obj.get("package"), Some(&Value::String("alpha".into())));
        assert_eq!(
            obj.get("dependencies").and_then(Value::as_array),
            Some(&vec![
                Value::String("beta".into()),
                Value::String("gamma".into()),
            ]),
            "got: {obj:?}"
        );
    }

    #[test]
    fn used_by_captured_as_object_array_with_empty_users() {
        let cap = ContextCapture::for_test_monorepo_in_package("alpha", &[], &[]);
        let mut values = Map::new();
        populate_repo(&cap, &mut values);
        let items = values
            .get("used_by")
            .and_then(Value::as_array)
            .expect("used_by must be an array");
        assert_eq!(items.len(), 1, "got: {items:?}");
        let obj = items[0].as_object().expect("item must be an object");
        assert_eq!(obj.get("package"), Some(&Value::String("alpha".into())));
        assert_eq!(
            obj.get("users").and_then(Value::as_array),
            Some(&Vec::new()),
            "got: {obj:?}"
        );
    }
}
