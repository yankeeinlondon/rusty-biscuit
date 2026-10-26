//! Palettes (R-T4, R-T8, D45, R-P3–R-P5, AC-P3): fixed palettes, terminal
//! palettes from injected detection results, and the contrast guarantees
//! every palette carries.

use std::sync::{Arc, OnceLock};

use crate::fonts::FontChain;
use crate::mermaid::contrast::{parse_css_color, readable_text};
use crate::mermaid::{MermaidTheme, Palette, Rgb, TerminalColors};

/// Bundled Inter only, for the contrast pass's text layout.
fn chain() -> &'static Arc<FontChain> {
    static CHAIN: OnceLock<Arc<FontChain>> = OnceLock::new();
    CHAIN.get_or_init(|| Arc::new(FontChain::injected(&[], &[], Some("Inter"))))
}

fn hex(text: &str) -> Rgb {
    Rgb::from_hex(text).expect("test hex")
}

/// S0's Tokyo-Night-like terminal: background, foreground, ANSI 0–15.
fn tokyo_night() -> TerminalColors {
    let ansi = [
        "#15161e", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#a9b1d6",
        "#414868", "#ff899d", "#9fe044", "#faba4a", "#8db0ff", "#c7a9ff", "#a4daff", "#c0caf5",
    ];
    TerminalColors {
        background: Some(hex("#1a1b26")),
        foreground: Some(hex("#c0caf5")),
        ansi: ansi.map(|color| Some(hex(color))),
    }
}

fn background_only(background: &str) -> TerminalColors {
    TerminalColors { background: Some(hex(background)), ..TerminalColors::default() }
}

/// Every palette source the tests exercise, including a hostile terminal
/// whose foreground does not read on its background.
fn every_palette() -> Vec<(String, Palette)> {
    let mut palettes: Vec<(String, Palette)> = [
        MermaidTheme::Dark,
        MermaidTheme::Default,
        MermaidTheme::Forest,
        MermaidTheme::Neutral,
    ]
    .map(|theme| (format!("{theme:?}"), Palette::fixed(theme)))
    .into();
    palettes.push(("tokyo night".to_string(), Palette::for_terminal(&tokyo_night(), true)));
    palettes.push(("background only".to_string(), Palette::for_terminal(&background_only("#1a1b26"), true)));
    palettes.push(("light background only".to_string(), Palette::for_terminal(&background_only("#fdf6e3"), false)));
    palettes.push((
        "mid-gray foreground on mid-gray".to_string(),
        Palette::for_terminal(
            &TerminalColors { foreground: Some(hex("#888888")), ..background_only("#777777") },
            true,
        ),
    ));
    palettes.push((
        "ANSI accents that vanish on the background".to_string(),
        Palette::new(hex("#000000"), hex("#ffffff"), [hex("#010101"); 6]),
    ));
    palettes
}

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

#[test]
fn hex_colors_parse_in_both_lengths_and_reject_everything_else() {
    assert_eq!(Rgb::from_hex("#a1b2c3"), Some(Rgb::new(0xa1, 0xb2, 0xc3)));
    assert_eq!(Rgb::from_hex("a1b2c3"), Some(Rgb::new(0xa1, 0xb2, 0xc3)));
    assert_eq!(Rgb::from_hex("#abc"), Some(Rgb::new(0xaa, 0xbb, 0xcc)));
    for invalid in ["", "#", "#12", "#1234", "#gggggg", "#a1b2c3d4", "#ééé"] {
        assert_eq!(Rgb::from_hex(invalid), None, "{invalid:?}");
    }
    assert_eq!(Rgb::new(0, 0x2b, 0x36).to_string(), "#002b36");
}

#[test]
fn contrast_follows_wcag_2() {
    assert!((Rgb::WHITE.contrast(Rgb::BLACK) - 21.0).abs() < 1e-9);
    assert!((Rgb::WHITE.contrast(Rgb::WHITE) - 1.0).abs() < 1e-9);
    // #767676 on white is the canonical 4.54:1.
    let gray = hex("#767676");
    assert!((gray.contrast(Rgb::WHITE) - 4.54).abs() < 0.01);
    assert_eq!(gray.contrast(Rgb::WHITE), Rgb::WHITE.contrast(gray));
}

/// The CSS colors Merman writes are what the contrast pass reads: hex,
/// fractional `rgb()`, negative-hue `hsl()`, and a few names; translucent and
/// unknown colors are not a known fill.
#[test]
fn css_colors_in_merman_spellings_parse() {
    assert_eq!(parse_css_color("#333"), Some(hex("#333333")));
    assert_eq!(parse_css_color("rgb(8.5000000002, 5.7500000001, 0)"), Some(Rgb::new(9, 6, 0)));
    assert_eq!(parse_css_color("rgb(255 0 0)"), Some(Rgb::new(255, 0, 0)));
    assert_eq!(
        parse_css_color("hsl(-79.4117647059, 100%, 93.3333333333%)"),
        parse_css_color("hsl(280.5882352941, 100%, 93.3333333333%)")
    );
    assert_eq!(parse_css_color("hsl(0, 0%, 100%)"), Some(Rgb::WHITE));
    assert_eq!(parse_css_color("white"), Some(Rgb::WHITE));
    assert_eq!(parse_css_color("rgba(255, 255, 255, 0.25)"), None);
    assert_eq!(parse_css_color("url(#gradient)"), None);
    assert_eq!(parse_css_color("rebeccapurple"), None);
}

// ---------------------------------------------------------------------------
// Fixed palettes (R-T4, D17)
// ---------------------------------------------------------------------------

/// Each fixed palette starts from Mermaid.js 12.2.0's theme of the same name:
/// its background, text, and node colors.
#[test]
fn fixed_palettes_start_from_the_mermaid_themes_of_the_same_name() {
    let cases = [
        (MermaidTheme::Dark, "#333333", "#cccccc", "#1f2020"),
        (MermaidTheme::Default, "#ffffff", "#333333", "#ececff"),
        (MermaidTheme::Forest, "#ffffff", "#000000", "#cde498"),
        (MermaidTheme::Neutral, "#ffffff", "#000000", "#eeeeee"),
    ];
    for (theme, background, text, surface) in cases {
        let palette = Palette::fixed(theme);
        assert_eq!(palette.background(), hex(background), "{theme:?}");
        assert_eq!(palette.text(), hex(text), "{theme:?}");
        assert_eq!(palette.surface(), hex(surface), "{theme:?}");
        assert_eq!(palette.is_dark(), theme == MermaidTheme::Dark, "{theme:?}");
    }
    let accents: Vec<[Rgb; 6]> = [MermaidTheme::Dark, MermaidTheme::Default, MermaidTheme::Forest, MermaidTheme::Neutral]
        .map(|theme| Palette::fixed(theme).accents())
        .into();
    for (index, set) in accents.iter().enumerate() {
        assert!(accents[index + 1..].iter().all(|other| other != set), "two fixed palettes share accents");
    }
}

// ---------------------------------------------------------------------------
// Contrast guarantees (R-T8, D45)
// ---------------------------------------------------------------------------

/// Every palette, whatever its source, keeps its text at 4.5:1 on the
/// background and on every fill a design puts text on, its accents and
/// borders at 3:1 on the background, and a readable label on each accent.
#[test]
fn every_palette_meets_the_contrast_floors() {
    for (name, palette) in every_palette() {
        let text = palette.text();
        let background = palette.background();
        assert!(text.contrast(background) >= 4.5, "{name}: text on background");
        let mut text_fills = vec![("surface", palette.surface()), ("surface_alt", palette.surface_alt())];
        text_fills.extend(palette.tints().map(|tint| ("tint", tint)));
        text_fills.extend(palette.chart_tints().map(|tint| ("chart tint", tint)));
        for (role, fill) in text_fills {
            assert!(text.contrast(fill) >= 4.5, "{name}: text on {role} {fill}");
        }
        for accent in palette.accents() {
            assert!(accent.contrast(background) >= 3.0, "{name}: accent {accent} on background");
            assert!(palette.text_on(accent).contrast(accent) >= 4.5, "{name}: label on accent {accent}");
        }
        assert!(palette.border().contrast(background) >= 3.0, "{name}: border");
        assert!(palette.line().contrast(background) >= 3.0, "{name}: line");
    }
}

// ---------------------------------------------------------------------------
// Terminal palettes (R-P3–R-P5, AC-P3), from injected detection results
// ---------------------------------------------------------------------------

/// Background, foreground, and ANSI known: the accents are ANSI 1–6 and the
/// background and text are the terminal's own.
#[test]
fn a_full_terminal_reply_gives_ansi_accents() {
    let colors = tokyo_night();
    let palette = Palette::for_terminal(&colors, true);

    assert_eq!(palette.background(), hex("#1a1b26"));
    assert_eq!(palette.text(), hex("#c0caf5"));
    let ansi_1_to_6: Vec<Rgb> = colors.ansi[1..=6].iter().map(|slot| slot.unwrap()).collect();
    assert_eq!(palette.accents().to_vec(), ansi_1_to_6);
}

/// No ANSI reply but a known background (R-P4): accents come from the
/// background, so they differ from the fixed palette's and follow the
/// background's lightness.
#[test]
fn a_background_without_ansi_colors_gives_background_derived_accents() {
    let dark = Palette::for_terminal(&background_only("#1a1b26"), true);
    let light = Palette::for_terminal(&background_only("#fdf6e3"), false);

    assert_eq!(dark.background(), hex("#1a1b26"));
    assert_ne!(dark, Palette::fixed(MermaidTheme::Dark));
    assert_ne!(dark.accents(), Palette::fixed(MermaidTheme::Dark).accents());
    // Light accents on a dark background, darker ones on a light background.
    let mean_luminance = |palette: &Palette| palette.accents().iter().map(|accent| accent.luminance()).sum::<f64>() / 6.0;
    assert!(mean_luminance(&dark) > mean_luminance(&light));
    // No foreground: black or white, whichever reads.
    assert_eq!(dark.text(), Rgb::WHITE);
    assert_eq!(light.text(), Rgb::BLACK);
}

/// An ANSI slot the terminal left unanswered gets the background-derived
/// accent for that slot; the answered slots keep the terminal's colors.
#[test]
fn a_partial_ansi_reply_fills_only_the_missing_slots_from_the_background() {
    let mut colors = tokyo_night();
    colors.ansi[3] = None;
    let partial = Palette::for_terminal(&colors, true);
    let derived = Palette::for_terminal(&TerminalColors { ansi: [None; 16], ..tokyo_night() }, true);
    let full = Palette::for_terminal(&tokyo_night(), true);

    assert_eq!(partial.accents()[2], derived.accents()[2]);
    for slot in [0, 1, 3, 4, 5] {
        assert_eq!(partial.accents()[slot], full.accents()[slot], "slot {slot}");
    }
    // ANSI 0 and 7–15 never become accents.
    let mut outside = tokyo_night();
    outside.ansi[0] = None;
    outside.ansi[9] = None;
    assert_eq!(Palette::for_terminal(&outside, true), full);
}

/// Neither background nor ANSI (R-P5): the fixed palette for the color mode,
/// whatever foreground or ANSI colors were reported.
#[test]
fn no_background_gives_the_fixed_palette_for_the_color_mode() {
    assert_eq!(Palette::for_terminal(&TerminalColors::default(), true), Palette::fixed(MermaidTheme::Dark));
    assert_eq!(Palette::for_terminal(&TerminalColors::default(), false), Palette::fixed(MermaidTheme::Default));
    let no_background = TerminalColors { background: None, ..tokyo_night() };
    assert_eq!(Palette::for_terminal(&no_background, true), Palette::fixed(MermaidTheme::Dark));
}

/// A terminal foreground that does not read on its background is moved until
/// it does (or replaced by black or white); one that reads is kept.
#[test]
fn an_unreadable_terminal_foreground_is_corrected() {
    let palette = Palette::for_terminal(
        &TerminalColors { foreground: Some(hex("#888888")), ..background_only("#777777") },
        true,
    );
    assert_ne!(palette.text(), hex("#888888"));
    assert!(palette.text().contrast(hex("#777777")) >= 4.5);
}

/// Output that is not for a terminal sends no query and gets no colors
/// (R-P5, R-P7): nextest captures stdout, and `CI` set short-circuits too.
#[cfg(feature = "terminal")]
#[test]
#[serial_test::serial]
fn detection_reports_nothing_when_stdout_is_not_a_terminal_or_in_ci() {
    assert_eq!(TerminalColors::detect(), TerminalColors::default());
    let previous = std::env::var_os("CI");
    // SAFETY: `#[serial]`; no other test thread reads the environment now.
    unsafe { std::env::set_var("CI", "true") };
    assert_eq!(TerminalColors::detect(), TerminalColors::default());
    match previous {
        // SAFETY: as above.
        Some(value) => unsafe { std::env::set_var("CI", value) },
        // SAFETY: as above.
        None => unsafe { std::env::remove_var("CI") },
    }
}

// ---------------------------------------------------------------------------
// The contrast pass (R-T8), on hand-written SVG in Merman's shapes
// ---------------------------------------------------------------------------

/// A label viewport whose background rect Merman painted with the palette
/// background but whose text kept the `base` theme's `#333` gets readable
/// text, its fill first in the style; the stylesheet and every other
/// declaration are untouched.
#[test]
fn text_on_a_known_label_background_is_made_readable() {
    let palette = Palette::fixed(MermaidTheme::Dark);
    let svg = concat!(
        r##"<svg id="d"><style>#d .x{fill:#333}</style><g class="edgeLabel"><svg x="0">"##,
        r##"<rect width="9" height="9" fill="#333333" style="fill:#333333 !important;opacity:1"/>"##,
        r##"<text x="1" fill="#333" style="text-anchor:middle;fill:#333;font-size:16px">go</text>"##,
        r##"</svg></g></svg>"##
    );
    let fixed = readable_text(svg, &palette, &[], chain());
    let text = palette.text_on(palette.background());
    assert_eq!(
        fixed,
        svg.replace(
            r##"fill="#333" style="text-anchor:middle;fill:#333;"##,
            &format!(r##"fill="{text}" style="fill:{text};text-anchor:middle;"##)
        )
    );
    assert!(fixed.contains("<style>#d .x{fill:#333}</style>"));
}

/// A label drawn in a definitions group and placed with `<use>` is judged
/// against the shape it is placed on: the palette text on an author's pink
/// `style` fill becomes dark, and the author's fill is kept.
#[test]
fn a_used_label_is_judged_against_its_placement() {
    let palette = Palette::fixed(MermaidTheme::Dark);
    let text = palette.text();
    // Merman's requirement element: the shape in its own closed group, then
    // the label group.
    let svg = format!(
        concat!(
            r##"<svg id="d"><g class="node"><g class="outer-path" style="fill:#f9f !important">"##,
            r##"<path d="M0 0" fill="#f9f" style="fill:#f9f !important"/></g>"##,
            r##"<g class="label"><use href="#label-1"/></g></g>"##,
            r##"<defs><g id="label-1"><svg><text fill="{text}" style="fill:{text}">e</text></svg></g></defs></svg>"##
        ),
        text = text
    );
    let fixed = readable_text(&svg, &palette, &[], chain()).into_owned();
    let on_pink = palette.text_on(hex("#ff99ff"));
    assert_ne!(on_pink, text);
    assert!(fixed.contains(&format!(r##"<text fill="{on_pink}" style="fill:{on_pink}">e</text>"##)), "{fixed}");
    assert!(fixed.contains(r##"fill="#f9f" style="fill:#f9f !important""##));
}

/// A text color the author wrote is left as written even when it fails, on
/// any fill; text that already reads is not touched. Nothing to fix borrows
/// the input.
#[test]
fn author_text_and_readable_text_are_left_alone() {
    let palette = Palette::fixed(MermaidTheme::Dark);
    let yellow = hex("#ffff00");
    let cases = [
        // The author's yellow text on the author's white fill.
        r##"<svg><g><rect style="fill:#ffffff"/><text style="fill:#ffff00">a</text></g></svg>"##,
        // The author's yellow text on a white fill the stylesheet paints.
        r##"<svg xmlns="http://www.w3.org/2000/svg"><style>.node{fill:#ffffff}</style><g><rect class="node" width="90" height="40"/><text x="5" y="20" fill="#ffff00" font-family="Inter">a</text></g></svg>"##,
        // Readable palette text on the palette background.
        r##"<svg><g><text fill="#cccccc">b</text></g></svg>"##,
    ];
    for svg in cases {
        assert!(matches!(readable_text(svg, &palette, &[yellow], chain()), std::borrow::Cow::Borrowed(_)), "{svg}");
    }
}

/// A shape the stylesheet paints is resolved: the library's light text on an
/// author's white node fill (what `primaryColor: '#ffffff'` gives, with no
/// compiled design) becomes dark, and the fill stays.
#[test]
fn text_on_a_stylesheet_painted_fill_is_made_readable() {
    let palette = Palette::fixed(MermaidTheme::Dark);
    let svg = concat!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" id="d"><style>#d .node rect{fill:#ffffff}</style>"##,
        r##"<g class="node"><rect width="90" height="40"/>"##,
        r##"<text x="5" y="20" fill="#cccccc" font-family="Inter">c</text></g></svg>"##
    );
    let fixed = readable_text(svg, &palette, &[], chain());
    let dark = palette.text_on(Rgb::WHITE);
    assert!(dark.contrast(Rgb::WHITE) >= 4.5);
    assert_eq!(fixed, svg.replace(
            r##"fill="#cccccc" font-family="Inter">"##,
            &format!(r##"fill="{dark}" font-family="Inter" style="fill:{dark}">"##)
        ));
}
