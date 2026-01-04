//! XXH64 hashing utility for Mermaid diagram instructions.

use xxhash_rust::xxh64::xxh64;

/// Computes XXH64 hash of instructions with blank lines removed.
///
/// This function normalizes the input by removing all blank lines before
/// hashing, ensuring that diagrams with different amounts of whitespace
/// produce the same hash.
///
/// ## Examples
///
/// ```rust
/// use shared::mermaid::hash::compute_hash;
///
/// let with_blanks = "flowchart LR\n\n    A --> B\n\n";
/// let without_blanks = "flowchart LR\n    A --> B";
/// assert_eq!(compute_hash(with_blanks), compute_hash(without_blanks));
/// ```
pub fn compute_hash(instructions: &str) -> u64 {
    let normalized: String = instructions
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    xxh64(normalized.as_bytes(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_ignores_blank_lines() {
        let with_blanks = "flowchart LR\n\n    A --> B\n\n";
        let without_blanks = "flowchart LR\n    A --> B";
        assert_eq!(compute_hash(with_blanks), compute_hash(without_blanks));
    }

    #[test]
    fn test_hash_different_content() {
        let a = "flowchart LR\n    A --> B";
        let b = "flowchart LR\n    A --> C";
        assert_ne!(compute_hash(a), compute_hash(b));
    }

    #[test]
    fn test_hash_deterministic() {
        let content = "flowchart LR\n    A --> B";
        assert_eq!(compute_hash(content), compute_hash(content));
    }

    #[test]
    fn test_hash_empty_string() {
        let hash = compute_hash("");
        assert_eq!(hash, compute_hash("   \n\n  ")); // All blank = empty
    }
}
