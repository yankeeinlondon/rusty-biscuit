//! Throwaway spike: how does `FileReference` resolve `FileChanged` paths
//! against an explicit base directory? Run on the host, prints a table.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileResolutionContext};

fn ctx(base: &Path, home: &Path, repo: Option<&Path>) -> FileResolutionContext {
    let c = FileResolutionContext::from_snapshot(base, Some(home.to_path_buf()), HashMap::new());
    match repo {
        Some(r) => c.with_repository_root(r),
        None => c,
    }
}

fn show(label: &str, raw: &str, c: &FileResolutionContext) {
    match FileReference::new(raw) {
        Err(e) => println!("[{label}] {raw:<22} PARSE-ERR {e}"),
        Ok(r) => {
            let d = r.resolve_detailed(c);
            let cands: Vec<String> = d
                .candidates()
                .iter()
                .map(|p| format!("{}={:?}", p.candidate().path().display(), p.disposition()))
                .collect();
            println!(
                "[{label}] {raw:<22} kind={:?} outcome={:?} err={:?}\n      candidates={cands:?}",
                r.class().kind,
                d.outcome(),
                d.error().map(|e| e.to_string())
            );
        }
    }
}

fn main() {
    // Layout (under a fresh temp root reached through the macOS /var symlink):
    //   root/repo/.git/            (not real git; repo root supplied explicitly)
    //   root/repo/docs/doc.md      <- base = root/repo/docs
    //   root/repo/docs/src/config.rs
    //   root/repo/docs/x
    //   root/repo/x                (only reachable via .. or repo fallback)
    //   root/repo/only-at-repo.rs  (implicit-relative fallback probe)
    //   root/outside               (escape target)
    //   root/home/x                (~ target)
    //   root/repo/docs/unreadable/f (dir mode 000 -> Io vs Missing)
    //   root/repo/docs/link.rs -> src/config.rs (symlink)
    let root = std::env::temp_dir().join(format!("fc-spike-{}", std::process::id()));
    let repo = root.join("repo");
    let base = repo.join("docs");
    let home = root.join("home");
    for d in [&base.join("src"), &home, &base.join("unreadable")] {
        std::fs::create_dir_all(d).unwrap();
    }
    for f in [
        base.join("src/config.rs"),
        base.join("x"),
        repo.join("x"),
        repo.join("only-at-repo.rs"),
        root.join("outside"),
        home.join("x"),
        base.join("unreadable/f"),
        base.join("a\\b"),
    ] {
        std::fs::write(&f, "content\n").unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::os::unix::fs::symlink("src/config.rs", base.join("link.rs")).unwrap();
        std::fs::set_permissions(base.join("unreadable"), std::fs::Permissions::from_mode(0o000))
            .unwrap();
    }
    println!("temp_dir={}  canonical={}", root.display(), std::fs::canonicalize(&root).unwrap().display());
    println!("base={}\n", base.display());

    let no_repo = ctx(&base, &home, None);
    let with_repo = ctx(&base, &home, Some(&repo));
    let cases = [
        "src/config.rs",
        "./x",
        "../x",
        "../../outside",
        "/etc/hosts",
        r"C:\x",
        "C:/x",
        r"a\b",
        "~/x",
        "@x",
        "&x",
        "^x",
        "%x",
        "vault:x",
        "only-at-repo.rs",
        "./only-at-repo.rs",
        "missing.rs",
        "unreadable/f",
        "link.rs",
        "src",
        "{{HOME}}/x",
        "x:y",
        "src//config.rs",
        "src/./config.rs",
    ];
    for c in cases {
        show("no-repo", c, &no_repo);
    }
    println!();
    for c in ["src/config.rs", "only-at-repo.rs", "../x", "../../outside", "&x", "^x", "@x"] {
        show("repo   ", c, &with_repo);
    }
    // Relative base (caller passes "docs" not absolute).
    println!();
    show("relbase", "src/config.rs", &ctx(Path::new("relative/base"), &home, None));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(base.join("unreadable"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
    }
    let _ = std::fs::remove_dir_all(&root);
    let _: PathBuf = root;
}
