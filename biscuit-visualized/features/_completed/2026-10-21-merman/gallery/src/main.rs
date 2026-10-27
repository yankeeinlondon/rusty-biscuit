//! Renders the S1 comparison corpus (one or two inputs per family) under
//! every fixed palette and the two synthetic terminal palettes of D47, as PNG
//! at 2x, into one self-contained HTML page. Also writes each PNG next to the
//! page (`<out>-png/`) for quick inspection.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use biscuit_visualized::artifact::{OutputFormat, RenderRequest};
use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTheme, Palette, Rgb, TerminalColors};

fn hex(text: &str) -> Rgb {
    Rgb::from_hex(text).expect("palette hex")
}

/// S0's Tokyo-Night-like palette: background, foreground, ANSI 0–15.
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

fn palettes() -> Vec<(&'static str, Palette)> {
    vec![
        ("Dark", Palette::fixed(MermaidTheme::Dark)),
        ("Default", Palette::fixed(MermaidTheme::Default)),
        ("Forest", Palette::fixed(MermaidTheme::Forest)),
        ("Neutral", Palette::fixed(MermaidTheme::Neutral)),
        ("Terminal: Tokyo Night (ANSI)", Palette::for_terminal(&tokyo_night(), true)),
        (
            "Terminal: background only",
            Palette::for_terminal(
                &TerminalColors { background: Some(hex("#1a1b26")), ..TerminalColors::default() },
                true,
            ),
        ),
    ]
}

fn corpus(root: &Path) -> Vec<(String, String, String)> {
    let mut cases = Vec::new();
    let mut families: Vec<_> = std::fs::read_dir(root).expect("corpus dir").flatten().collect();
    families.sort_by_key(|entry| entry.file_name());
    for family in families.into_iter().filter(|entry| entry.path().is_dir()) {
        let mut files: Vec<_> = std::fs::read_dir(family.path()).expect("family dir").flatten().collect();
        files.sort_by_key(|entry| entry.file_name());
        for file in files.into_iter().filter(|entry| entry.path().extension().is_some_and(|ext| ext == "mmd")) {
            let source = std::fs::read_to_string(file.path()).expect("case source");
            let name = file.path().file_stem().unwrap_or_default().to_string_lossy().into_owned();
            cases.push((family.file_name().to_string_lossy().into_owned(), name, source));
        }
    }
    cases
}

fn main() {
    let out: PathBuf = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| "themed-gallery.html".into());
    let only: Option<String> = std::env::args().nth(2);
    let corpus_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../spikes/s1-comparison/corpus/merman");
    let png_dir = out.with_extension("").with_file_name(format!(
        "{}-png",
        out.file_stem().unwrap_or_default().to_string_lossy()
    ));
    std::fs::create_dir_all(&png_dir).expect("png dir");
    let palettes = palettes();
    let request = RenderRequest { format: OutputFormat::Png, scale: 2, ..RenderRequest::default() };

    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>Themed gallery</title><style>\
         body{font-family:sans-serif;background:#888;margin:1em}\
         table{border-collapse:collapse}td,th{border:1px solid #555;padding:6px;vertical-align:top}\
         img{max-width:420px}th{background:#ddd}.err{color:#900;font-size:12px;max-width:420px}\
         </style><h1>Themed gallery</h1><table><tr><th>Case</th>",
    );
    for (name, palette) in &palettes {
        let swatches: String = palette
            .accents()
            .iter()
            .map(|accent| format!("<span style=\"background:{accent}\">&nbsp;&nbsp;&nbsp;</span>"))
            .collect();
        let _ = write!(html, "<th>{name}<br>bg {} text {}<br>{swatches}</th>", palette.background(), palette.text());
    }
    html.push_str("</tr>");
    for (family, name, source) in corpus(&corpus_root) {
        if only.as_ref().is_some_and(|only| !family.contains(only.as_str())) {
            continue;
        }
        let _ = write!(html, "<tr><td><b>{family}</b><br><small>{name}</small></td>");
        for (index, (_, palette)) in palettes.iter().enumerate() {
            let diagram = MermaidDiagram::new(source.clone()).with_palette(*palette);
            match diagram.render(&request) {
                Ok(artifact) => {
                    let bytes = std::fs::read(&artifact.path).expect("artifact");
                    let file = png_dir.join(format!("{family}-{name}-{index}.png"));
                    std::fs::write(&file, &bytes).expect("png copy");
                    let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    let _ = write!(html, "<td><img src=\"data:image/png;base64,{data}\"></td>");
                }
                Err(error) => {
                    let _ = write!(html, "<td class=err>{error}</td>");
                }
            }
        }
        html.push_str("</tr>");
        eprintln!("{family}/{name}");
    }
    html.push_str("</table>");
    std::fs::write(&out, html).expect("gallery");
    eprintln!("wrote {}", out.display());
}
