//! Picking a face within one family by stretch, style, and weight.
//!
//! A port of fontdb 0.23's private `find_best_match` (CSS Fonts 3, §5.2,
//! steps 4a–4c), so the chain picks the face `fontdb::Database::query` would
//! pick, but over a candidate list the chain chooses: fontdb's own `query`
//! matches family names case-sensitively and cannot leave out the bundled
//! faces. Keep it in step with fontdb when fontdb is upgraded.

use resvg::usvg::fontdb::{FaceInfo, Stretch, Style};

/// The index into `candidates` of the face CSS matching selects for
/// `weight`/`style`/`stretch`; the first such face in `candidates` order wins
/// a tie. `None` only for an empty list.
pub(crate) fn best_match(
    candidates: &[&FaceInfo],
    weight: u16,
    style: Style,
    stretch: Stretch,
) -> Option<usize> {
    if candidates.is_empty() {
        return None;
    }
    let mut set: Vec<usize> = (0..candidates.len()).collect();

    // Step 4a: font-stretch.
    let stretch_of = |i: &usize| candidates[*i].stretch;
    let matching_stretch = if set.iter().any(|i| stretch_of(i) == stretch) {
        stretch
    } else if stretch <= Stretch::Normal {
        set.iter()
            .filter(|i| stretch_of(i) < stretch)
            .min_by_key(|i| stretch.to_number() - stretch_of(i).to_number())
            .or_else(|| {
                set.iter()
                    .min_by_key(|i| stretch_of(i).to_number() - stretch.to_number())
            })
            .map(stretch_of)?
    } else {
        set.iter()
            .filter(|i| stretch_of(i) > stretch)
            .min_by_key(|i| stretch_of(i).to_number() - stretch.to_number())
            .or_else(|| {
                set.iter()
                    .min_by_key(|i| stretch.to_number() - stretch_of(i).to_number())
            })
            .map(stretch_of)?
    };
    set.retain(|i| stretch_of(i) == matching_stretch);

    // Step 4b: font-style.
    let preference = match style {
        Style::Italic => [Style::Italic, Style::Oblique, Style::Normal],
        Style::Oblique => [Style::Oblique, Style::Italic, Style::Normal],
        Style::Normal => [Style::Normal, Style::Oblique, Style::Italic],
    };
    let matching_style = *preference
        .iter()
        .find(|wanted| set.iter().any(|&i| candidates[i].style == **wanted))?;
    set.retain(|&i| candidates[i].style == matching_style);

    // Step 4c: font-weight (450 is fontdb's cutoff between 400 and 500).
    let weight_of = |i: &usize| candidates[*i].weight.0;
    let matching_weight = if set.iter().any(|i| weight_of(i) == weight) {
        weight
    } else if (400..450).contains(&weight) && set.iter().any(|i| weight_of(i) == 500) {
        500
    } else if (450..=500).contains(&weight) && set.iter().any(|i| weight_of(i) == 400) {
        400
    } else if weight <= 500 {
        set.iter()
            .filter(|i| weight_of(i) <= weight)
            .min_by_key(|i| weight - weight_of(i))
            .or_else(|| set.iter().min_by_key(|i| weight_of(i) - weight))
            .map(weight_of)?
    } else {
        set.iter()
            .filter(|i| weight_of(i) >= weight)
            .min_by_key(|i| weight_of(i) - weight)
            .or_else(|| set.iter().min_by_key(|i| weight - weight_of(i)))
            .map(weight_of)?
    };
    set.retain(|i| weight_of(i) == matching_weight);

    set.into_iter().next()
}
