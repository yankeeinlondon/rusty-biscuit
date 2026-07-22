use std::fs;
use std::path::Path;

use walkdir::WalkDir;

use crate::error::Result;

/// Hash the contents of a skill directory for comparison.
///
/// Uses `biscuit_hash::xx_hash_bytes`. Walks files recursively,
/// sorts by relative path, concatenates with null separators, then hashes.
/// Resolves symlinks before hashing so that a symlink and a real directory
/// with identical content produce the same hash.
pub fn hash_skill_dir(skill_dir: &Path) -> Result<u64> {
    // Resolve symlinks so we hash the target content
    let resolved = if skill_dir.is_symlink() {
        fs::canonicalize(skill_dir)?
    } else {
        skill_dir.to_path_buf()
    };

    let mut file_paths: Vec<std::path::PathBuf> = Vec::new();

    for entry in WalkDir::new(&resolved).follow_links(true) {
        let entry = entry.map_err(std::io::Error::other)?;
        if entry.file_type().is_file() {
            file_paths.push(entry.into_path());
        }
    }

    // Sort by relative path for deterministic ordering
    file_paths.sort_by(|a, b| {
        let rel_a = a.strip_prefix(&resolved).unwrap_or(a);
        let rel_b = b.strip_prefix(&resolved).unwrap_or(b);
        rel_a.cmp(rel_b)
    });

    let mut combined = Vec::new();
    for path in &file_paths {
        let relative = path
            .strip_prefix(&resolved)
            .map_err(std::io::Error::other)?;
        // Include relative path as separator to distinguish files with
        // identical content but different names
        combined.extend_from_slice(relative.to_string_lossy().as_bytes());
        combined.push(0);
        combined.extend_from_slice(&fs::read(path)?);
        combined.push(0);
    }

    Ok(biscuit_hash::xx_hash_bytes(&combined))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn same_content_produces_same_hash() {
        let tmp1 = TempDir::new().unwrap();
        let tmp2 = TempDir::new().unwrap();

        let skill1 = tmp1.path().join("my-skill");
        let skill2 = tmp2.path().join("my-skill");
        fs::create_dir_all(&skill1).unwrap();
        fs::create_dir_all(&skill2).unwrap();

        let content = "---\nname: test\n---\n# Hello\n";
        fs::write(skill1.join("SKILL.md"), content).unwrap();
        fs::write(skill2.join("SKILL.md"), content).unwrap();

        let hash1 = hash_skill_dir(&skill1).unwrap();
        let hash2 = hash_skill_dir(&skill2).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn different_content_produces_different_hash() {
        let tmp1 = TempDir::new().unwrap();
        let tmp2 = TempDir::new().unwrap();

        let skill1 = tmp1.path().join("my-skill");
        let skill2 = tmp2.path().join("my-skill");
        fs::create_dir_all(&skill1).unwrap();
        fs::create_dir_all(&skill2).unwrap();

        fs::write(skill1.join("SKILL.md"), "version 1").unwrap();
        fs::write(skill2.join("SKILL.md"), "version 2").unwrap();

        let hash1 = hash_skill_dir(&skill1).unwrap();
        let hash2 = hash_skill_dir(&skill2).unwrap();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn different_filenames_produce_different_hash() {
        let tmp1 = TempDir::new().unwrap();
        let tmp2 = TempDir::new().unwrap();

        let skill1 = tmp1.path().join("skill");
        let skill2 = tmp2.path().join("skill");
        fs::create_dir_all(&skill1).unwrap();
        fs::create_dir_all(&skill2).unwrap();

        let content = "same content";
        fs::write(skill1.join("a.md"), content).unwrap();
        fs::write(skill2.join("b.md"), content).unwrap();

        let hash1 = hash_skill_dir(&skill1).unwrap();
        let hash2 = hash_skill_dir(&skill2).unwrap();
        assert_ne!(hash1, hash2);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_and_real_dir_with_same_content_produce_same_hash() {
        let tmp = TempDir::new().unwrap();

        let real_dir = tmp.path().join("real");
        fs::create_dir_all(&real_dir).unwrap();
        fs::write(real_dir.join("SKILL.md"), "---\nname: test\n---\n").unwrap();

        let link_dir = tmp.path().join("linked");
        std::os::unix::fs::symlink(&real_dir, &link_dir).unwrap();

        let hash_real = hash_skill_dir(&real_dir).unwrap();
        let hash_link = hash_skill_dir(&link_dir).unwrap();
        assert_eq!(hash_real, hash_link);
    }

    #[test]
    fn hashes_nested_files() {
        let tmp = TempDir::new().unwrap();
        let skill = tmp.path().join("skill");
        let scripts = skill.join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        fs::write(skill.join("SKILL.md"), "# Skill").unwrap();
        fs::write(scripts.join("run.sh"), "#!/bin/bash\necho hi").unwrap();

        let hash = hash_skill_dir(&skill).unwrap();
        assert_ne!(hash, 0);
    }
}
