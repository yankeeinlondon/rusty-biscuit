//! The one key-suggestion ranker shared by `md clean`, schema validation's
//! unknown-key hints, and open-object near-miss advice.
//!
//! Two distinct spellings are *near* when their lengths reach three and they
//! differ only by case and `-`/`_` separators, or by at most one edit (two
//! for keys of five characters or more). An edit is an insertion, deletion,
//! substitution, or adjacent transposition (optimal string alignment).

/// Whether two distinct key spellings are suspiciously close.
pub fn near_key_match(first: &str, second: &str) -> bool {
    if first == second {
        return false;
    }
    let length = first.chars().count().max(second.chars().count());
    if length < 3 {
        return false;
    }
    let normalize = |text: &str| text.to_lowercase().replace('_', "-");
    if normalize(first) == normalize(second) {
        return true;
    }
    let distance = edit_distance(first, second);
    (length >= 5 && distance <= 2) || distance <= 1
}

/// Optimal string alignment distance (Damerau with unit costs).
pub fn edit_distance(first: &str, second: &str) -> usize {
    let a: Vec<char> = first.chars().collect();
    let b: Vec<char> = second.chars().collect();
    let (rows, cols) = (a.len() + 1, b.len() + 1);
    let mut distances = vec![vec![0usize; cols]; rows];
    for (row, cell) in distances.iter_mut().enumerate().take(rows) {
        cell[0] = row;
    }
    for (col, cell) in distances[0].iter_mut().enumerate().take(cols) {
        *cell = col;
    }
    for row in 1..rows {
        for col in 1..cols {
            let cost = usize::from(a[row - 1] != b[col - 1]);
            distances[row][col] = (distances[row - 1][col] + 1)
                .min(distances[row][col - 1] + 1)
                .min(distances[row - 1][col - 1] + cost);
            if row > 1 && col > 1 && a[row - 1] == b[col - 2] && a[row - 2] == b[col - 1] {
                distances[row][col] = distances[row][col].min(distances[row - 2][col - 2] + 1);
            }
        }
    }
    distances[rows - 1][cols - 1]
}

/// The near candidate closest to `offending`; on a tie the first in
/// `candidates`' order wins, so a closed object's declaration order decides.
pub fn nearest<'a>(offending: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    if offending.is_empty() {
        return None;
    }
    candidates
        .into_iter()
        .filter(|candidate| near_key_match(candidate, offending))
        .min_by_key(|candidate| edit_distance(candidate, offending))
}

/// The near candidate closest to `offending` only when no other distinct
/// candidate is as close: the evidence open-object advice requires, since an
/// accepted key there is legal data and a coin-flip guess would mislead.
pub fn unique_nearest<'a>(offending: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    if offending.is_empty() {
        return None;
    }
    let mut best: Option<(usize, &'a str)> = None;
    let mut tied = false;
    for candidate in candidates.into_iter().filter(|candidate| near_key_match(candidate, offending)) {
        let distance = edit_distance(candidate, offending);
        match best {
            Some((current, name)) if distance == current && name != candidate => tied = true,
            Some((current, _)) if distance >= current => {}
            _ => {
                best = Some((distance, candidate));
                tied = false;
            }
        }
    }
    best.filter(|_| !tied).map(|(_, name)| name)
}

/// The hint text for a suggested key.
pub fn did_you_mean(suggestion: &str) -> String {
    format!("did you mean `{suggestion}`?")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_match_the_clean_characterization() {
        // Lengths under three never match.
        assert!(!near_key_match("ab", "ac"));
        // Three and four tolerate one edit.
        assert!(near_key_match("tag", "tags"));
        assert!(!near_key_match("tag", "tgsx"));
        // Five and more tolerate two.
        assert!(near_key_match("promt", "prompt"));
        assert!(near_key_match("titel", "title"));
        assert!(!near_key_match("alpha", "omega"));
        // Case and separator variants always match.
        assert!(near_key_match("Max_Parallel", "max-parallel"));
        // Identical spellings are not a suggestion.
        assert!(!near_key_match("name", "name"));
    }

    #[test]
    fn transposition_costs_one_edit() {
        assert_eq!(edit_distance("ab", "ba"), 1);
        assert_eq!(edit_distance("tilte", "title"), 1);
    }

    #[test]
    fn closed_ties_keep_declaration_order_and_open_ties_suggest_nothing() {
        let candidates = ["note", "nope"];
        assert_eq!(nearest("nobe", candidates), Some("note"));
        assert_eq!(unique_nearest("nobe", candidates), None);
        assert_eq!(unique_nearest("promt", ["prompt", "name"]), Some("prompt"));
        assert_eq!(nearest("zzzzzz", ["prompt"]), None);
        assert_eq!(unique_nearest("", ["prompt"]), None);
    }
}
