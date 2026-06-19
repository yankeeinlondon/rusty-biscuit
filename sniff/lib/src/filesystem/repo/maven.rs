//! Maven multi-module build (`pom.xml` `<modules>`) detection.

use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;

use crate::Result;

use super::detection::{DetectorOutcome, create_package, dedupe_packages, resolve_internal_deps};
use super::standard::{MonorepoStandard, PackageProvenance};

pub(super) fn detect_maven_workspace(root: &Path) -> Result<Option<DetectorOutcome>> {
    let pom = root.join("pom.xml");
    if !pom.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&pom)?;
    let modules = parse_maven_modules(&content);
    if modules.is_empty() {
        return Ok(None);
    }

    let lock_versions = None;
    let mut packages = Vec::new();
    for module in modules {
        let module_path = root.join(&module);
        if !module_path.exists() {
            continue;
        }
        packages.push(create_package(
            &module_path,
            root,
            MonorepoStandard::MavenMultiModule,
            PackageProvenance::Explicit,
            &lock_versions,
        ));
    }

    if packages.is_empty() {
        return Ok(None);
    }

    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::MavenMultiModule,
        root: root.to_path_buf(),
        packages,
    }))
}

/// Parse the `<module>` entries from a Maven `pom.xml`.
///
/// `<module>` only ever appears inside `<modules>` in the Maven schema, so a
/// direct element match is sufficient. Each entry is a relative directory path
/// (the trailing `/pom.xml` Maven also accepts is stripped). Returns an empty
/// vector when no modules are declared, so callers treat that as "not a
/// multi-module build".
pub(super) fn parse_maven_modules(content: &str) -> Vec<String> {
    static MODULE_RE: OnceLock<Regex> = OnceLock::new();
    let re = MODULE_RE.get_or_init(|| Regex::new(r"(?s)<module>\s*(.+?)\s*</module>").unwrap());

    re.captures_iter(content)
        .filter_map(|cap| {
            let raw = cap.get(1)?.as_str().trim();
            let relative = raw.trim_end_matches('/').trim_end_matches("/pom.xml");
            if relative.is_empty() {
                None
            } else {
                Some(relative.to_string())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_modules_reads_module_entries() {
        let content = "<project>\n  <modules>\n    <module>core</module>\n\
             <module>web</module>\n  </modules>\n</project>\n";
        assert_eq!(parse_maven_modules(content), vec!["core", "web"]);
    }

    #[test]
    fn parse_modules_strips_trailing_pom_path() {
        let content = "<modules><module>services/api/pom.xml</module></modules>";
        assert_eq!(parse_maven_modules(content), vec!["services/api"]);
    }

    #[test]
    fn parse_modules_empty_when_absent() {
        assert!(parse_maven_modules("<project><artifactId>solo</artifactId></project>").is_empty());
    }
}
