//! The font-resolution chain over injected font sets.
//!
//! Every chain here is built with [`FontChain::injected`], never from the
//! host's fonts, so results are the same on every OS. The only font files are
//! the bundled Inter and Noto Emoji. Families the tests need beyond those are
//! made by patching a copy of those bytes in memory (see [`renamed`] and
//! [`classed_sans_serif`]): fontdb cannot register one file under two family
//! names, and the patched copies keep every glyph, so they behave as real
//! installed fonts with a different name or OS/2 class.

use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use merman::svg::TextStyle;
use resvg::usvg::fontdb::{ID, Source, Style};
use tracing::span;

use crate::fonts::bundled::{INTER_BOLD, INTER_REGULAR};
use crate::fonts::LabelMeasurer;
use crate::fonts::chain::{FaceIdentity, FontChain, ResolutionStep};
use crate::fonts::metadata::{FaceMetadata, LATIN_BLOCKS, choose_sans_serif_family};
use crate::fonts::query::{FamilyName, FontQuery, GenericFamily, parse_font_family_list};
use crate::raster::png::rasterize_svg_to_png_bytes_with;
use crate::raster::usvg_options;

/// Noto Emoji's bytes on every OS. The library compiles the file in on Linux
/// builds only; tests use it as an installed font that lacks Latin letters.
pub(super) static NOTO_EMOJI_FILE: &[u8] =
    include_bytes!("../../assets/fonts/noto-emoji/NotoEmoji-Variable.ttf");

pub(super) const MERMAID_STACK: &str = r#""trebuchet ms", verdana, arial, sans-serif"#;

// ------------------------------------------------------------------ helpers

pub(super) fn table(font: &[u8], tag: &[u8; 4]) -> Range<usize> {
    let count = u16::from_be_bytes([font[4], font[5]]) as usize;
    (0..count)
        .map(|i| &font[12 + 16 * i..28 + 16 * i])
        .find(|record| &record[..4] == tag)
        .map(|record| {
            let offset = u32::from_be_bytes(record[8..12].try_into().unwrap()) as usize;
            let length = u32::from_be_bytes(record[12..16].try_into().unwrap()) as usize;
            offset..offset + length
        })
        .unwrap_or_else(|| panic!("no {} table", String::from_utf8_lossy(tag)))
}

fn replace_all(bytes: &mut [u8], from: &[u8], to: &[u8]) {
    let mut at = 0;
    while let Some(pos) = bytes[at..].windows(from.len()).position(|w| w == from) {
        let start = at + pos;
        bytes[start..start + to.len()].copy_from_slice(to);
        at = start + to.len();
    }
}

/// A copy of `font` whose `name` table says `to` wherever it said `from`
/// (both ASCII, equal length), in Mac Roman and UTF-16BE records alike.
pub(super) fn renamed(font: &[u8], from: &str, to: &str) -> Vec<u8> {
    assert_eq!(from.len(), to.len(), "an in-place rename keeps the length");
    let mut bytes = font.to_vec();
    let names = table(&bytes, b"name");
    let utf16 = |s: &str| s.encode_utf16().flat_map(u16::to_be_bytes).collect::<Vec<u8>>();
    replace_all(&mut bytes[names.clone()], &utf16(from), &utf16(to));
    replace_all(&mut bytes[names], from.as_bytes(), to.as_bytes());
    bytes
}

/// A copy of `font` whose OS/2 `sFamilyClass` says sans-serif (class 8).
fn classed_sans_serif(mut font: Vec<u8>) -> Vec<u8> {
    let os2 = table(&font, b"OS/2");
    font[os2.start + 30] = 8;
    font[os2.start + 31] = 0;
    font
}

fn family_of(chain: &FontChain, id: ID) -> String {
    chain.database().face(id).expect("face").families[0].0.clone()
}

fn postscript_of(chain: &FontChain, id: ID) -> String {
    chain.database().face(id).expect("face").post_script_name.clone()
}

fn resolved(chain: &FontChain, query: &FontQuery) -> (String, ResolutionStep) {
    let (id, step) = chain.resolve_with_step(query).expect("a face");
    (family_of(chain, id), step)
}

fn stack(css: &str) -> FontQuery {
    FontQuery::from_css(css, 400, false)
}

fn painted_pixels(png: &[u8]) -> usize {
    image::load_from_memory(png)
        .expect("PNG decodes")
        .to_rgba8()
        .pixels()
        .filter(|pixel| pixel.0[3] > 0)
        .count()
}

fn text_svg(font_family: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="40">
             <text x="5" y="30" font-size="24" font-family='{font_family}'>Hello</text>
           </svg>"#
    )
}

/// Counts WARN events from this crate. Installed with
/// `tracing::subscriber::with_default`, so it sees only this thread.
struct WarnCounter(Arc<AtomicUsize>);

impl tracing::Subscriber for WarnCounter {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }
    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let metadata = event.metadata();
        if *metadata.level() == tracing::Level::WARN
            && metadata.target().starts_with("biscuit_visualized")
        {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn enter(&self, _: &span::Id) {}
    fn exit(&self, _: &span::Id) {}
}

fn warnings_while(f: impl FnOnce()) -> usize {
    let count = Arc::new(AtomicUsize::new(0));
    tracing::subscriber::with_default(WarnCounter(Arc::clone(&count)), f);
    count.load(Ordering::SeqCst)
}

// ------------------------------------------------------------ CSS parsing

#[test]
fn css_family_lists_parse_quoted_names_and_generics_ignoring_case() {
    let families = parse_font_family_list(r#" "trebuchet ms" , Verdana,  Times   New Roman, 'a, b', SANS-SERIF,monospace,,"#);

    assert_eq!(
        families,
        vec![
            FamilyName::Named("trebuchet ms".into()),
            FamilyName::Named("Verdana".into()),
            FamilyName::Named("Times New Roman".into()),
            FamilyName::Named("a, b".into()),
            FamilyName::Generic(GenericFamily::SansSerif),
            FamilyName::Generic(GenericFamily::Monospace),
        ]
    );
}

#[test]
fn css_queries_carry_weight_and_italic() {
    let query = FontQuery::from_css("serif", 700, true);

    assert_eq!(query.families, vec![FamilyName::Generic(GenericFamily::Serif)]);
    assert_eq!(query.weight, 700);
    assert_eq!(query.style, Style::Italic);
}

// ------------------------------------------- AC-M3: each step is reachable

/// AC-M3, step 1: `MERMAID_FONT` naming the bundled Inter, in any casing,
/// beats an installed family the stack names.
#[test]
fn mermaid_font_names_bundled_inter_in_any_casing() {
    let anter = renamed(INTER_REGULAR, "Inter", "Anter");
    for casing in ["Inter", "inter", "INTER", "  iNtEr "] {
        let chain = FontChain::injected(&[&anter], &[], Some(casing));

        let (id, step) = chain.resolve_with_step(&stack("Anter")).expect("a face");

        assert_eq!(step, ResolutionStep::MermaidFont, "{casing:?}");
        assert!(chain.bundled_ids().contains(&id), "{casing:?}");
        assert_eq!(postscript_of(&chain, id), "Inter-Regular");
    }
}

/// AC-M3, step 1: `MERMAID_FONT` naming an installed family.
#[test]
fn mermaid_font_names_an_installed_family_ignoring_case() {
    let anter = renamed(INTER_REGULAR, "Inter", "Anter");
    let bnter = renamed(INTER_REGULAR, "Inter", "Bnter");
    let chain = FontChain::injected(&[&anter, &bnter], &[], Some("bNTER"));

    assert_eq!(resolved(&chain, &stack("Anter")), ("Bnter".into(), ResolutionStep::MermaidFont));
}

/// AC-M3: unset, empty, and not-installed `MERMAID_FONT` all fall through to
/// step 2.
#[test]
fn unset_empty_or_unknown_mermaid_font_falls_through_to_the_stack() {
    let anter = renamed(INTER_REGULAR, "Inter", "Anter");
    for mermaid_font in [None, Some(""), Some("   "), Some("No Such Family")] {
        let chain = FontChain::injected(&[&anter], &[], mermaid_font);

        assert_eq!(
            resolved(&chain, &stack(MERMAID_STACK.replace("verdana", "anter").as_str())),
            ("Anter".into(), ResolutionStep::StackName),
            "{mermaid_font:?}"
        );
    }
}

/// AC-M3, R-M6: named families of the stack match ignoring case, in stack
/// order.
#[test]
fn stack_names_match_ignoring_case_in_stack_order() {
    let anter = renamed(INTER_REGULAR, "Inter", "Anter");
    let bnter = renamed(INTER_REGULAR, "Inter", "Bnter");
    let chain = FontChain::injected(&[&anter, &bnter], &[], None);

    assert_eq!(
        resolved(&chain, &stack(r#""no such", "BNTER", anter"#)),
        ("Bnter".into(), ResolutionStep::StackName)
    );
}

/// AC-M3, step 2: a generic in the stack is answered by the OS's answer for
/// that generic.
#[test]
fn a_stack_generic_takes_the_os_answer_for_that_generic() {
    let anter = renamed(INTER_REGULAR, "Inter", "Anter");
    let bnter = renamed(INTER_REGULAR, "Inter", "Bnter");
    let chain = FontChain::injected(
        &[&anter, &bnter],
        &[(GenericFamily::SansSerif, "anter"), (GenericFamily::Monospace, "Bnter")],
        None,
    );

    assert_eq!(
        resolved(&chain, &stack("courier, monospace")),
        ("Bnter".into(), ResolutionStep::StackGeneric)
    );
}

/// AC-M3, step 3, R-M7: with no stack name installed, the OS's sans-serif
/// answer draws the text.
#[test]
fn no_installed_stack_name_takes_the_os_sans_serif_answer() {
    let anter = renamed(INTER_REGULAR, "Inter", "Anter");
    let chain = FontChain::injected(&[&anter], &[(GenericFamily::SansSerif, "ANTER")], None);

    assert_eq!(
        resolved(&chain, &stack(r#""trebuchet ms", verdana"#)),
        ("Anter".into(), ResolutionStep::OsSansSerif)
    );
}

/// AC-M3, step 4: with no generic answer, the installed sans-serif rule.
#[test]
fn no_generic_answer_takes_the_installed_sans_serif_rule() {
    let sans = classed_sans_serif(renamed(INTER_REGULAR, "Inter", "Anter"));
    let chain = FontChain::injected(&[&sans], &[], None);

    assert_eq!(
        resolved(&chain, &stack(MERMAID_STACK)),
        ("Anter".into(), ResolutionStep::InstalledSansSerif)
    );
}

/// AC-M3, AC-M8: the bundled Inter is matched by name only, never by the
/// installed sans-serif rule; with nothing installed and nothing naming it,
/// no face resolves.
#[test]
fn bundled_inter_is_matched_by_name_only() {
    let chain = FontChain::injected(&[], &[], None);

    assert_eq!(chain.resolve(&stack(MERMAID_STACK)), None);
    assert_eq!(
        resolved(&chain, &stack("verdana, inter")),
        ("Inter".into(), ResolutionStep::StackName)
    );
}

// --------------------------------------- AC-M3, R-M5, D51: one warning only

/// AC-M3, D51: an uninstalled `MERMAID_FONT` warns once per process, however
/// many chains resolve with it.
///
/// The guard is process-global, so this must be the only test in its
/// process that sets a missing `MERMAID_FONT`. nextest runs every test in its
/// own process, which makes that so; `cargo test` would share the guard
/// between tests.
#[test]
fn an_uninstalled_mermaid_font_warns_exactly_once_per_process() {
    let anter = renamed(INTER_REGULAR, "Inter", "Anter");

    let warnings = warnings_while(|| {
        for _ in 0..2 {
            let chain = FontChain::injected(&[&anter], &[], Some("No Such Family"));
            assert!(chain.resolve(&stack("anter")).is_some());
            assert!(chain.resolve(&stack("anter")).is_some());
        }
    });

    assert_eq!(warnings, 1);
}

/// AC-M3, R-M5: unset or empty `MERMAID_FONT` never logs a warning.
#[test]
fn unset_or_empty_mermaid_font_never_warns() {
    let anter = renamed(INTER_REGULAR, "Inter", "Anter");

    let warnings = warnings_while(|| {
        for mermaid_font in [None, Some(""), Some(" ")] {
            let chain = FontChain::injected(&[&anter], &[], mermaid_font);
            assert!(chain.resolve(&stack("anter")).is_some());
        }
    });

    assert_eq!(warnings, 0);
}

// ------------------------------------------- AC-M4: a host of non-stack fonts

/// AC-M4, R-M7: with only a face no stack names and no OS answer, every
/// query still resolves to a face.
#[test]
fn a_host_of_only_non_stack_fonts_resolves_every_query() {
    let sans = classed_sans_serif(renamed(INTER_REGULAR, "Inter", "Dnter"));
    let chain = FontChain::injected(&[&sans], &[], None);

    let queries = [
        stack(MERMAID_STACK),
        FontQuery::from_css(MERMAID_STACK, 700, false),
        FontQuery::from_css(MERMAID_STACK, 400, true),
        stack("courier"),
        stack("monospace"),
        stack("serif"),
        stack(""),
    ];

    for query in &queries {
        let (family, _) = resolved(&chain, query);
        assert_eq!(family, "Dnter", "{query:?}");
    }
}

/// AC-M4: the same host draws an SVG `<text>` in Mermaid's default stack.
#[test]
fn a_host_of_only_non_stack_fonts_paints_text() {
    let sans = classed_sans_serif(renamed(INTER_REGULAR, "Inter", "Dnter"));
    let chain = Arc::new(FontChain::injected(&[&sans], &[], None));

    let png = rasterize_svg_to_png_bytes_with(&text_svg(MERMAID_STACK), 1, &chain)
        .expect("rasterizes");

    assert!(painted_pixels(&png) > 50);
}

/// With nothing installed and no `MERMAID_FONT`, text in Mermaid's stack
/// resolves to no face and paints nothing: the control for the test above.
#[test]
fn an_empty_host_without_mermaid_font_paints_no_text() {
    let chain = Arc::new(FontChain::injected(&[], &[], None));

    let png = rasterize_svg_to_png_bytes_with(&text_svg(MERMAID_STACK), 1, &chain)
        .expect("rasterizes");

    assert_eq!(painted_pixels(&png), 0);
}

/// The rasterizer resolves a lowercase Mermaid stack to `MERMAID_FONT=Inter`.
#[test]
fn rasterizer_paints_a_lowercase_mermaid_stack_in_bundled_inter() {
    let chain = Arc::new(FontChain::injected(&[], &[], Some("Inter")));

    let png = rasterize_svg_to_png_bytes_with(&text_svg(MERMAID_STACK), 1, &chain)
        .expect("rasterizes");

    assert!(painted_pixels(&png) > 50);
}

// -------------------------------------------------- AC-M9: bundled Inter

/// AC-M9: with no host font, `MERMAID_FONT=Inter` resolves to the bundled
/// Inter Regular, and a bold query to Inter Bold.
#[test]
fn mermaid_font_inter_on_an_empty_host_resolves_regular_and_bold() {
    let chain = FontChain::injected(&[], &[], Some("Inter"));

    let regular = chain.resolve(&stack(MERMAID_STACK)).expect("regular");
    let bold = chain
        .resolve(&FontQuery::from_css(MERMAID_STACK, 700, false))
        .expect("bold");

    assert_eq!(postscript_of(&chain, regular), "Inter-Regular");
    assert_eq!(postscript_of(&chain, bold), "Inter-Bold");
}

// ------------------------------------------- AC-M8: the installed sans rule

fn face(family: &str, weight: u16, latin_coverage: u32, fixed_pitch: bool) -> FaceMetadata {
    FaceMetadata {
        family: family.into(),
        family_class: Some(8),
        weight,
        italic: false,
        latin_coverage,
        fixed_pitch,
    }
}

/// AC-M8, preference 1: a family with regular and bold beats one without,
/// even with less Latin coverage and a later name.
#[test]
fn sans_rule_prefers_a_family_with_regular_and_bold() {
    let faces = [
        face("Alpha", 400, 900, false),
        face("Zulu", 400, 100, false),
        face("Zulu", 700, 100, false),
    ];

    assert_eq!(choose_sans_serif_family(&faces).as_deref(), Some("zulu"));
}

/// AC-M8, preference 2: tied on regular and bold, the wider Latin coverage
/// wins, even monospace against proportional and with a later name.
#[test]
fn sans_rule_then_prefers_wider_latin_coverage() {
    let faces = [
        face("Alpha", 400, 200, false),
        face("Alpha", 700, 200, false),
        face("Zulu", 400, 300, true),
        face("Zulu", 700, 300, true),
    ];

    assert_eq!(choose_sans_serif_family(&faces).as_deref(), Some("zulu"));
}

/// AC-M8, preference 3: tied above, proportional beats monospace.
#[test]
fn sans_rule_then_prefers_proportional_over_monospace() {
    let faces = [
        face("Alpha", 400, 300, true),
        face("Alpha", 700, 300, true),
        face("Zulu", 400, 300, false),
        face("Zulu", 700, 300, false),
    ];

    assert_eq!(choose_sans_serif_family(&faces).as_deref(), Some("zulu"));
}

/// AC-M8, preference 4: tied on everything, the alphabetically first name
/// ignoring case (`alpha` before `Beta`, although `B` sorts before `a`).
#[test]
fn sans_rule_finally_prefers_the_first_name_ignoring_case() {
    let faces = [
        face("Beta", 400, 300, false),
        face("Beta", 700, 300, false),
        face("alpha", 400, 300, false),
        face("alpha", 700, 300, false),
    ];

    assert_eq!(choose_sans_serif_family(&faces).as_deref(), Some("alpha"));
}

/// AC-M8: only faces whose OS/2 class is sans-serif are candidates.
#[test]
fn sans_rule_considers_only_sans_serif_classed_faces() {
    let mut serif = face("Alpha", 400, 900, false);
    serif.family_class = Some(1);
    let unclassified = FaceMetadata {
        family_class: Some(0),
        ..face("Beta", 400, 900, false)
    };

    assert_eq!(choose_sans_serif_family(&[serif.clone(), unclassified.clone()]), None);
    assert_eq!(
        choose_sans_serif_family(&[serif, unclassified, face("Zulu", 400, 10, true)]).as_deref(),
        Some("zulu")
    );
}

/// The metadata read from Inter Regular is what Inter's tables say; the
/// Latin count is checked against the `cmap` subtables directly.
#[test]
fn metadata_of_inter_regular_matches_its_tables() {
    let parsed = ttf_parser::Face::parse(INTER_REGULAR, 0).expect("parses");
    let mut latin = std::collections::BTreeSet::new();
    for subtable in parsed.tables().cmap.expect("cmap").subtables {
        if subtable.is_unicode() {
            subtable.codepoints(|cp| {
                if LATIN_BLOCKS.iter().any(|block| block.contains(&cp))
                    && subtable.glyph_index(cp).is_some()
                {
                    latin.insert(cp);
                }
            });
        }
    }

    let metadata = FaceMetadata::read(INTER_REGULAR, 0, "Inter").expect("reads");

    assert_eq!(
        metadata,
        FaceMetadata {
            family: "Inter".into(),
            // Inter leaves sFamilyClass unset ("no classification").
            family_class: Some(0),
            weight: 400,
            italic: false,
            latin_coverage: latin.len() as u32,
            fixed_pitch: false,
        }
    );
    assert!(metadata.latin_coverage > 500, "{}", metadata.latin_coverage);
}

// ---------------------------------------------- AC-M10, D59: fallback order

/// A file-backed face the chain resolves or falls back to is kept mapped
/// for drawing (same ID, held in memory), while the metadata database still
/// names the file; text measures and draws exactly as the same fonts held in
/// memory from the start.
#[test]
fn file_faces_the_chain_answers_with_stay_mapped_and_draw_as_in_memory() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let anter = renamed(INTER_REGULAR, "Inter", "Anter");
    let zebra = renamed(NOTO_EMOJI_FILE, "Noto Emoji", "Zebra Mono");
    let anter_path = dir.path().join("anter.ttf");
    let zebra_path = dir.path().join("zebra.ttf");
    std::fs::write(&anter_path, &anter).expect("writes Anter");
    std::fs::write(&zebra_path, &zebra).expect("writes Zebra");
    let generics = [(GenericFamily::SansSerif, "Anter")];
    let from_files = Arc::new(FontChain::injected_files(&[&anter_path, &zebra_path], &generics, None));
    let in_memory = Arc::new(FontChain::injected(&[&anter, &zebra], &generics, None));
    let file_backed = |chain: &FontChain, id: ID| matches!(chain.drawing_database().face(id).expect("face").source, Source::File(_));
    let mapped = |chain: &FontChain, id: ID| {
        matches!(chain.drawing_database().face(id).expect("face").source, Source::SharedFile(..))
    };
    let named_file = |chain: &FontChain, id: ID| matches!(chain.database().face(id).expect("face").source, Source::File(_));
    let unresolved: Vec<ID> = from_files.database().faces().map(|face| face.id).collect();
    assert!(unresolved.iter().filter(|id| named_file(&from_files, **id)).all(|id| file_backed(&from_files, *id)));

    let anter_id = from_files.resolve(&stack(MERMAID_STACK)).expect("Anter");
    assert!(mapped(&from_files, anter_id));
    let zebra_id = from_files.fallback('\u{1F600}', &[anter_id]).expect("Zebra");
    assert!(mapped(&from_files, zebra_id));
    assert!(named_file(&from_files, anter_id) && named_file(&from_files, zebra_id));

    let style = TextStyle {
        font_family: Some(MERMAID_STACK.to_string()),
        font_size: 16.0,
        font_weight: None,
        font_style: None,
    };
    for text in ["Hello, world", "Hello \u{1F600}"] {
        assert_eq!(
            LabelMeasurer::new(&from_files).width(text, &style),
            LabelMeasurer::new(&in_memory).width(text, &style),
            "{text}"
        );
    }
    assert_eq!(
        rasterize_svg_to_png_bytes_with(&text_svg("sans-serif"), 1, &from_files).expect("renders"),
        rasterize_svg_to_png_bytes_with(&text_svg("sans-serif"), 1, &in_memory).expect("renders")
    );
}

/// AC-M10, D59: a character the resolved face lacks falls back to the first
/// installed face, by case-folded family name, that has it, and the
/// rasterizer's `select_fallback` returns the same face as `fallback`.
///
/// `boto Emoji` sorts before `Coto Emoji` only when case is ignored.
#[test]
fn a_missing_character_falls_back_by_case_folded_family_name() {
    let coto = renamed(NOTO_EMOJI_FILE, "Noto", "Coto");
    let boto = renamed(NOTO_EMOJI_FILE, "Noto", "boto");
    let chain = Arc::new(FontChain::injected(&[&coto, &boto], &[], Some("Inter")));
    let inter = chain.resolve(&stack(MERMAID_STACK)).expect("Inter");

    let chosen = chain.fallback('\u{1F600}', &[inter]).expect("a fallback face");

    assert_eq!(family_of(&chain, chosen), "boto Emoji");
    let options = usvg_options(&chain);
    let mut db = Arc::clone(chain.database());
    assert_eq!(
        (options.font_resolver.select_fallback)('\u{1F600}', &[inter], &mut db),
        Some(chosen)
    );
    let next = chain.fallback('\u{1F600}', &[inter, chosen]).expect("the next face");
    assert_eq!(family_of(&chain, next), "Coto Emoji");
}

/// D59, D65: the fallback order is the installed faces by case-folded name
/// (`anter` before `Zebra Mono`, although `Z` sorts before `a`), without the
/// bundled Inter, then on Linux builds the bundled Noto Emoji.
#[test]
fn fallback_order_lists_installed_faces_only_then_last_resort() {
    let anter = renamed(INTER_REGULAR, "Inter", "anter");
    let zebra = renamed(NOTO_EMOJI_FILE, "Noto Emoji", "Zebra Mono");
    let chain = FontChain::injected(&[&zebra, &anter], &[], None);

    let order: Vec<String> = chain
        .fallback_order(None)
        .iter()
        .map(|id| family_of(&chain, *id))
        .collect();

    let mut expected = vec!["anter".to_string(), "Zebra Mono".to_string()];
    if cfg!(target_os = "linux") {
        expected.push("Noto Emoji".into());
    }
    assert_eq!(order, expected);
}

/// D59: within one family the face matching the resolved face's weight
/// comes first.
#[test]
fn fallback_within_a_family_prefers_the_matching_weight() {
    let regular = renamed(INTER_REGULAR, "Inter", "Anter");
    let bold = renamed(INTER_BOLD, "Inter", "Anter");
    let chain = FontChain::injected(&[&regular, &bold], &[], Some("Inter"));
    let inter_bold = chain
        .resolve(&FontQuery::from_css("x", 700, false))
        .expect("Inter Bold");

    let first = chain.fallback_order(Some(inter_bold))[0];

    assert_eq!(postscript_of(&chain, first), "Anter-Bold");
}

/// D65: on Linux builds, the bundled Noto Emoji is the last fallback face
/// and is never resolved by name.
#[cfg(target_os = "linux")]
#[test]
fn bundled_noto_emoji_is_last_and_never_resolved() {
    let anter = renamed(INTER_REGULAR, "Inter", "Anter");
    let chain = FontChain::injected(&[&anter], &[], Some("Noto Emoji"));

    let order = chain.fallback_order(None);
    let last = *order.last().expect("non-empty");

    assert_eq!(family_of(&chain, last), "Noto Emoji");
    assert!(chain.bundled_ids().contains(&last));
    assert_eq!(chain.resolve(&stack("noto emoji")), None);
    assert_eq!(chain.fallback('\u{1F600}', &[order[0]]), Some(last));
}

/// D65: macOS and Windows builds register no Noto Emoji face.
#[cfg(not(target_os = "linux"))]
#[test]
fn no_noto_emoji_face_is_registered_off_linux() {
    let chain = FontChain::injected(&[], &[], None);

    let families: Vec<String> = chain
        .database()
        .faces()
        .map(|face| face.families[0].0.clone())
        .collect();

    assert_eq!(families, vec!["Inter".to_string(), "Inter".to_string()]);
    assert_eq!(chain.bundled_ids().len(), 2);
}

// -------------------------------------------------- D63: face identities

/// D63: a face's identity is its content hash and face index, the same for
/// the same bytes in another chain, and a different `MERMAID_FONT` changes
/// the resolved identity.
#[test]
fn face_identities_follow_content_and_resolution() {
    let anter = renamed(INTER_REGULAR, "Inter", "Anter");
    let plain = FontChain::injected(&[&anter], &[], None);
    let pinned = FontChain::injected(&[&anter], &[], Some("Inter"));
    let queries = [stack("anter")];

    let plain_ids = plain.query_identities(&queries);
    let pinned_ids = pinned.query_identities(&queries);

    let anter_identity = plain_ids[0].resolved.clone().expect("resolved");
    assert_eq!(
        anter_identity,
        FaceIdentity::Contents {
            hash: biscuit_hash::xx_hash_bytes(&anter),
            index: 0
        }
    );
    assert_eq!(
        pinned_ids[0].resolved,
        Some(FaceIdentity::Contents {
            hash: biscuit_hash::xx_hash_bytes(INTER_REGULAR),
            index: 0
        })
    );
    let mut expected_fallbacks = vec![anter_identity];
    if cfg!(target_os = "linux") {
        expected_fallbacks.push(FaceIdentity::Contents {
            hash: biscuit_hash::xx_hash_bytes(NOTO_EMOJI_FILE),
            index: 0,
        });
    }
    assert_eq!(pinned_ids[0].fallbacks, expected_fallbacks);
    assert_eq!(plain.font_set_hash(&queries), pinned.font_set_hash(&queries));
}

/// D41: the font-set hash depends on the faces, not on their loading order.
#[test]
fn font_set_hash_ignores_loading_order_and_follows_content() {
    let anter = renamed(INTER_REGULAR, "Inter", "Anter");
    let bnter = renamed(INTER_REGULAR, "Inter", "Bnter");
    let queries = [stack(MERMAID_STACK)];

    let forward = FontChain::injected(&[&anter, &bnter], &[], None).font_set_hash(&queries);
    let backward = FontChain::injected(&[&bnter, &anter], &[], None).font_set_hash(&queries);
    let fewer = FontChain::injected(&[&anter], &[], None).font_set_hash(&queries);

    assert_eq!(forward, backward);
    assert_ne!(forward, fewer);
}

// ------------------------------------------------------ the default chain

/// The families matchable by name are the installed ones and the bundled
/// `Inter`; the bundled Noto Emoji is not among them.
#[test]
fn family_names_list_installed_families_and_bundled_inter() {
    let anter = renamed(INTER_REGULAR, "Inter", "Anter");
    let chain = FontChain::injected(&[&anter], &[], None);

    assert_eq!(chain.family_names(), vec!["Anter".to_string(), "Inter".to_string()]);
}

/// The process chain over the host's fonts (loaded once here) lists the
/// bundled `Inter` among the families the rasterizer can draw by name.
#[test]
fn available_font_families_include_bundled_inter() {
    let families = crate::raster::available_font_families();

    assert!(families.iter().any(|family| family == "Inter"));
}
