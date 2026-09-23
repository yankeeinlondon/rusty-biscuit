//! Runtime provenance uses the binaries actually shipped. External libraries
//! are recorded by their observed content identity (Mach-O UUID for shared-cache
//! images). A consumer whose libraries all resolve but differ in content (a
//! runner image update) is warned, not refused: running the tests is the
//! compatibility proof, and a runner label or a package installation
//! declaration is never that proof. A library that does not resolve is refused.

use super::*;
use std::io::Read;

fn payloads(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            payloads(&path, files)?;
        } else {
            let mut magic = [0_u8; 4];
            let count = fs::File::open(&path)?.read(&mut magic)?;
            if count == 4 && (magic == *b"\x7fELF" || magic[..2] == *b"MZ"
                || matches!(magic, [0xcf, 0xfa, 0xed, 0xfe] | [0xce, 0xfa, 0xed, 0xfe]
                    | [0xca, 0xfe, 0xba, 0xbe])) {
                files.push(path);
            }
        }
    }
    Ok(())
}

fn library_identity(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    Ok(format!("{}@blake3:{}", path.file_name().unwrap().to_string_lossy(),
        blake3_hash_reader(&mut file)?))
}

pub fn observe(roots: &[PathBuf]) -> Result<Runtime> {
    let mut files = Vec::new();
    for root in roots {
        if root.is_dir() { payloads(root, &mut files)?; }
    }
    let mut libraries = BTreeMap::new();
    for file in &files {
        dependencies(file, &files, &mut libraries)?;
    }
    let (arch, abi, libc) = host_runtime();
    Ok(Runtime { arch: arch.to_owned(), abi: abi.to_owned(), libc: libc.to_owned(),
        native_libraries: libraries.into_values().collect::<BTreeSet<_>>().into_iter().collect() })
}

#[cfg(target_os = "linux")]
fn dependencies(file: &Path, files: &[PathBuf], found: &mut BTreeMap<String, String>) -> Result<()> {
    let mut dirs: BTreeSet<PathBuf> = files.iter().filter_map(|f| f.parent().map(Path::to_path_buf)).collect();
    if let Some(paths) = std::env::var_os("LD_LIBRARY_PATH") { dirs.extend(std::env::split_paths(&paths)); }
    let output = Command::new("ldd").arg("-v").arg(file)
        .env("LD_LIBRARY_PATH", std::env::join_paths(dirs)?).output()?;
    let text = format!("{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    if text.contains("not found") { bail!("missing runtime dependency for {}: {text}", file.display()); }
    if !output.status.success() {
        if text.contains("not a dynamic executable") || text.contains("statically linked") { return Ok(()); }
        bail!("ldd could not inspect {}: {text}", file.display());
    }
    for line in text.lines() {
        let line = line.trim();
        let value = line.split_once("=>").map(|(_, value)| value.trim()).unwrap_or(line);
        let Some(name) = value.split_whitespace().next().filter(|name| name.starts_with('/')) else { continue; };
        let path = PathBuf::from(name.trim_end_matches(':'));
        if !path.is_file() || files.contains(&path) { continue; }
        let key = path.to_string_lossy().into_owned();
        if let std::collections::btree_map::Entry::Vacant(entry) = found.entry(key) {
            entry.insert(library_identity(&path)?);
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn dependencies(file: &Path, files: &[PathBuf], found: &mut BTreeMap<String, String>) -> Result<()> {
    let text = capture("dyld_info", &["-linked_dylibs".to_owned(), file.to_string_lossy().into_owned()], Path::new("."))?;
    let mut linked = false;
    for line in text.lines().skip(1) {
        if line.starts_with("    -") {
            linked = line.trim() == "-linked_dylibs:";
            continue;
        }
        if !linked || !line.starts_with("        ") { continue; }
        let line = line.trim();
        // dyld permits absent weak imports; they are not runtime requirements.
        if line.contains("weak-link") { continue; }
        let Some(start) = line.find(['/', '@']) else { continue; };
        let name = &line[start..];
        let name_path = Path::new(name);
        if files.iter().any(|p| p == name_path || (name.starts_with('@') && p.file_name() == name_path.file_name())) { continue; }
        if found.contains_key(name) { continue; }
        let identity = if name.starts_with("/usr/lib/") || name.starts_with("/System/Library/") {
            // These files can exist only in dyld's cache; filesystem existence
            // and otool alone cannot establish whether the host provides them.
            let text = capture("dyld_info", &["-uuid".to_owned(), name.to_owned()], Path::new("."))?;
            let uuids = text.lines().map(str::trim).filter(|line| line.len() == 36 && line.chars().all(|c| c.is_ascii_hexdigit() || c == '-')).collect::<Vec<_>>().join(";");
            if uuids.is_empty() { bail!("dyld supplied no identity for {name}: {text}"); }
            format!("{name}@{uuids}")
        } else {
            if name.starts_with('@') { bail!("unresolved runtime dependency {name} in {}", file.display()); }
            library_identity(name_path).with_context(|| format!("missing runtime dependency {name}"))?
        };
        found.insert(name.to_owned(), identity);
        if name.starts_with("/usr/lib/") || name.starts_with("/System/Library/") {
            // System images belong to the sealed OS distribution. Pin its
            // build too: private shared-cache edges may not be inspectable as
            // standalone Mach-O files even when dyld can resolve them.
            if !found.contains_key("macos-build") {
                let build = capture("sw_vers", &["-buildVersion".into()], Path::new("."))?;
                found.insert("macos-build".into(), format!("macos-build:{build}"));
            }
        } else {
            dependencies(name_path, files, found).with_context(|| format!("dependency of {}", file.display()))?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn dependencies(file: &Path, files: &[PathBuf], found: &mut BTreeMap<String, String>) -> Result<()> {
    use object::Object;
    let bytes = fs::read(file)?;
    let binary = object::File::parse(bytes.as_slice())?;
    let mut dirs = vec![file.parent().unwrap().to_path_buf()];
    if let Some(root) = std::env::var_os("SystemRoot") { dirs.push(PathBuf::from(root).join("System32")); }
    if let Some(paths) = std::env::var_os("PATH") { dirs.extend(std::env::split_paths(&paths)); }
    for import in binary.imports()? {
        let name = String::from_utf8_lossy(import.library()).to_ascii_lowercase();
        if name.starts_with("api-ms-") || name.starts_with("ext-ms-") {
            // API-set names are virtual; the mapping belongs to this exact OS
            // image. Its schema identity is checked alongside concrete DLLs.
            let schema = dirs.iter().map(|d| d.join("apisetschema.dll")).find(|p| p.is_file())
                .context("Windows API-set schema is unavailable")?;
            if !found.contains_key("apisetschema.dll") { found.insert("apisetschema.dll".into(), library_identity(&schema)?); }
            continue;
        }
        if files.iter().any(|p| p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(&name))) { continue; }
        if found.contains_key(&name) { continue; }
        let path = dirs.iter().map(|dir| dir.join(&name)).find(|p| p.is_file())
            .with_context(|| format!("missing runtime dependency {name} for {}", file.display()))?;
        found.insert(name, library_identity(&path)?);
        dependencies(&path, files, found)?;
    }
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn dependencies(_: &Path, _: &[PathBuf], _: &mut BTreeMap<String, String>) -> Result<()> {
    bail!("runtime inspection is unsupported on this platform")
}

pub fn linker(identity: &Identity, workspace: &Path, env: &mut Vec<(String, OsString)>) -> Result<String> {
    #[allow(unused_mut)]
    let mut driver = identity.linker.clone();
    #[cfg(windows)]
    if driver == "link.exe" {
        let tool = find_msvc_tools::find_tool(&identity.target, "link.exe")
            .context("the planned MSVC linker is unavailable")?;
        driver = tool.path().to_string_lossy().into_owned();
        env.extend(tool.env().into_iter().map(|(k, v)| (k.to_string_lossy().into_owned(), v.clone())));
    }
    let program = if driver == "cc" {
        capture("cc", &["-print-prog-name=ld".into()], workspace)?
    } else { driver.clone() };
    let mut flags: Vec<String> = identity.rustflags.split_whitespace().map(str::to_owned).collect();
    flags.push(format!("-Clinker={driver}"));
    if cfg!(target_os = "linux") && driver == "cc" {
        // Recent Rust GNU targets otherwise choose bundled LLD behind cc.
        // Pin the system linker that this producer probes and records.
        flags.push("-Clinker-features=-lld".into());
    }
    env.push(("CARGO_ENCODED_RUSTFLAGS".into(), OsString::from(flags.join("\x1f"))));
    let args = if cfg!(windows) { vec!["/?"] } else if cfg!(target_os = "macos") { vec!["-v"] } else { vec!["--version"] };
    let output = Command::new(&program).args(args).current_dir(workspace).output()
        .with_context(|| format!("probing linker {program}"))?;
    let text = format!("{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    let version = text.lines().find(|line| !line.trim().is_empty()).context("linker supplied no version")?;
    Ok(format!("{program}: {}", version.trim()))
}
