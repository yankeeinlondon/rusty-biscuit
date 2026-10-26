//! Spike S1: subset a face with allsorts, read the subset back, and write a
//! headless-Chrome test page. Throwaway; not part of the workspace.
//!
//! Usage:
//!   s1-subset subset <font> <face-index> <label> <out-dir> <text>
//!   s1-subset page <out-dir> <text> <label>...
//!   s1-subset probe <font> <face-index>
//!   s1-subset advance <font> <text>

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use allsorts::binary::read::ReadScope;
use allsorts::font_data::FontData;
use allsorts::subset::{CmapTarget, SubsetProfile, subset};
use allsorts::tables::FontTableProvider;
use allsorts::tables::cmap::{Cmap, CmapSubtable};
use allsorts::tag;
use base64::Engine;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args[1].as_str() {
        "subset" => cmd_subset(&args[2], args[3].parse().unwrap(), &args[4], &args[5], &args[6]),
        "page" => cmd_page(&args[2], &args[3], &args[4..]),
        "probe" => cmd_probe(&args[2], args[3].parse().unwrap()),
        "advance" => cmd_advance(&args[2], &args[3]),
        other => panic!("unknown command {other}"),
    }
}

fn glyph_ids(provider: &impl FontTableProvider, text: &str) -> (Vec<u16>, Vec<char>) {
    let cmap_data = provider.read_table_data(tag::CMAP).expect("cmap");
    let cmap = ReadScope::new(&cmap_data).read::<Cmap<'_>>().expect("cmap parse");
    let (_, record) = allsorts::font::find_good_cmap_subtable(&cmap).expect("unicode cmap");
    let sub = cmap
        .scope
        .offset(usize::try_from(record.offset).unwrap())
        .read::<CmapSubtable<'_>>()
        .expect("subtable");
    let chars: BTreeSet<char> = text.chars().collect();
    let mut ids = vec![0u16];
    let mut unmapped = Vec::new();
    for ch in chars {
        match sub.map_glyph(ch as u32).expect("map") {
            Some(gid) if gid != 0 => {
                if !ids.contains(&gid) {
                    ids.push(gid);
                }
            }
            _ => unmapped.push(ch),
        }
    }
    (ids, unmapped)
}

fn cmd_subset(font: &str, index: usize, label: &str, out_dir: &str, text: &str) {
    let bytes = fs::read(font).expect("read font");
    let scope = ReadScope::new(&bytes);
    let data = scope.read::<FontData<'_>>().expect("font data");
    let provider = data.table_provider(index).expect("provider");
    let (ids, unmapped) = glyph_ids(&provider, text);
    let out = subset(&provider, &ids, &SubsetProfile::Minimal, CmapTarget::Unicode);
    match out {
        Ok(sub) => {
            fs::create_dir_all(out_dir).unwrap();
            let sub_path = Path::new(out_dir).join(format!("{label}.subset.ttf"));
            fs::write(&sub_path, &sub).unwrap();
            println!(
                "{label}: source {} bytes, subset {} bytes, {} glyphs, unmapped {:?}",
                bytes.len(),
                sub.len(),
                ids.len(),
                unmapped
            );
            report_tables(&sub, 0, text);
        }
        Err(err) => println!("{label}: subset FAILED: {err:?}"),
    }
}

fn report_tables(bytes: &[u8], index: u32, text: &str) {
    let face = ttf_parser::Face::parse(bytes, index).expect("ttf-parser parse");
    let raw = face.raw_face();
    let mut tags = Vec::new();
    // Walk the table directory directly.
    let num = u16::from_be_bytes([raw.data[4], raw.data[5]]) as usize;
    let dir = if &raw.data[0..4] == b"ttcf" { None } else { Some(()) };
    if dir.is_some() {
        for i in 0..num {
            let rec = &raw.data[12 + i * 16..12 + i * 16 + 4];
            tags.push(String::from_utf8_lossy(rec).into_owned());
        }
    }
    println!("  tables: {}", tags.join(","));
    let os2 = face.raw_face().table(ttf_parser::Tag::from_bytes(b"OS/2"));
    if let Some(os2) = os2 {
        println!("  OS/2 len {} fsType 0x{:04X}", os2.len(), u16::from_be_bytes([os2[8], os2[9]]));
    }
    for id in [0u16, 1, 13, 14] {
        let rec = face
            .names()
            .into_iter()
            .find(|n| n.name_id == id && n.is_unicode())
            .and_then(|n| n.to_string());
        println!("  name {id}: {:?}", rec.map(|s| s.chars().take(80).collect::<String>()));
    }
    let missing: Vec<char> = text
        .chars()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|c| face.glyph_index(*c).is_none())
        .collect();
    println!("  cmap misses in subset: {missing:?}");
}

fn cmd_probe(font: &str, index: usize) {
    let bytes = fs::read(font).expect("read font");
    let scope = ReadScope::new(&bytes);
    let data = scope.read::<FontData<'_>>().expect("font data");
    let provider = data.table_provider(index).expect("provider");
    let has = |t: u32| provider.has_table(t);
    let face = ttf_parser::Face::parse(&bytes, index as u32).expect("ttf-parser");
    let fs_type = face
        .raw_face()
        .table(ttf_parser::Tag::from_bytes(b"OS/2"))
        .map(|t| u16::from_be_bytes([t[8], t[9]]));
    println!(
        "{font}#{index}: glyf={} CFF={} CFF2={} fvar={} GPOS={} GSUB={} kern={} fsType={:?} family={:?}",
        has(tag::GLYF),
        has(tag::CFF),
        has(tag::CFF2),
        has(tag::FVAR),
        has(tag::GPOS),
        has(tag::GSUB),
        has(tag::KERN),
        fs_type.map(|v| format!("0x{v:04X}")),
        face.names()
            .into_iter()
            .find(|n| n.name_id == 1 && n.is_unicode())
            .and_then(|n| n.to_string())
    );
    let (ids, _) = glyph_ids(&provider, "Hello World");
    match subset(&provider, &ids, &SubsetProfile::Minimal, CmapTarget::Unicode) {
        Ok(sub) => {
            let parsed = ttf_parser::Face::parse(&sub, 0);
            println!(
                "  subset ok: {} bytes, magic {:02X?}, ttf-parser parse ok={}, glyf={}",
                sub.len(),
                &sub[0..4],
                parsed.is_ok(),
                parsed.map(|f| f.tables().glyf.is_some()).unwrap_or(false)
            );
        }
        Err(err) => println!("  subset FAILED: {err:?}"),
    }
}

/// Advance width of `text` at 1000 units/em with rustybuzz defaults
/// (kerning and ligatures on), to compare against the browser.
fn cmd_advance(font: &str, text: &str) {
    let bytes = fs::read(font).expect("read font");
    let face = rustybuzz::Face::from_slice(&bytes, 0).expect("face");
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);
    let shaped = rustybuzz::shape(&face, &[], buffer);
    let total: i32 = shaped.glyph_positions().iter().map(|p| p.x_advance).sum();
    let no_kern = {
        let mut b = rustybuzz::UnicodeBuffer::new();
        b.push_str(text);
        let feats = [
            rustybuzz::Feature::new(ttf_parser::Tag::from_bytes(b"kern"), 0, ..),
            rustybuzz::Feature::new(ttf_parser::Tag::from_bytes(b"liga"), 0, ..),
            rustybuzz::Feature::new(ttf_parser::Tag::from_bytes(b"calt"), 0, ..),
        ];
        let s = rustybuzz::shape(&face, &feats, b);
        s.glyph_positions().iter().map(|p| p.x_advance).sum::<i32>()
    };
    let upem = f64::from(face.units_per_em());
    println!(
        "{font}: shaped {:.3} px@100, no-kern/liga/calt {:.3} px@100",
        f64::from(total) / upem * 100.0,
        f64::from(no_kern) / upem * 100.0
    );
}

fn cmd_page(out_dir: &str, text: &str, labels: &[String]) {
    let b64 = base64::engine::general_purpose::STANDARD;
    let mut css = String::new();
    for label in labels {
        let weight = if label.contains("bold") { "700" } else { "400" };
        let sub = fs::read(Path::new(out_dir).join(format!("{label}.subset.ttf"))).unwrap();
        css.push_str(&format!(
            "@font-face {{ font-family: \"S\"; font-weight: {weight}; src: url(data:font/ttf;base64,{}) format(\"truetype\"); }}\n",
            b64.encode(&sub)
        ));
    }
    let text_json = format!("{text:?}");
    let html = format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><style>
{css}
@font-face {{ font-family: "F"; font-weight: 400; src: url(full-regular.ttf) format("truetype"); }}
@font-face {{ font-family: "F"; font-weight: 700; src: url(full-bold.ttf) format("truetype"); }}
</style></head><body><pre id="out">pending</pre><script>
const text = {text_json};
async function run() {{
  const lines = [];
  for (const w of ["400", "700"]) {{
    await document.fonts.load(w + " 100px S", text);
    await document.fonts.load(w + " 100px F", text);
  }}
  for (const face of document.fonts) {{ lines.push("face " + face.family + " " + face.weight + " " + face.status); }}
  const c = document.createElement("canvas").getContext("2d");
  for (const w of ["400", "700"]) {{
    lines.push("check S " + w + " = " + document.fonts.check(w + " 100px S", text));
    const width = (fam, kern) => {{ c.font = w + " 100px " + fam; c.fontKerning = kern; return c.measureText(text).width; }};
    lines.push(`w${{w}} subset=${{width('"S"', "normal").toFixed(3)}} full=${{width('"F"', "normal").toFixed(3)}} full-nokern=${{width('"F"', "none").toFixed(3)}}`);
    // Per-glyph: a glyph drawn by a fallback face would match the fallback's width.
    let fallbackHits = [];
    for (const ch of new Set(text)) {{
      const s = width('"S", monospace', "normal");
      c.font = w + ' 100px "S", monospace'; const ws = c.measureText(ch).width;
      c.font = w + ' 100px "F", monospace'; const wf = c.measureText(ch).width;
      c.font = w + ' 100px monospace'; const wm = c.measureText(ch).width;
      if (Math.abs(ws - wf) > 0.01) fallbackHits.push(ch + ":" + ws.toFixed(2) + "!=" + wf.toFixed(2) + "(mono " + wm.toFixed(2) + ")");
    }}
    lines.push("per-glyph mismatches " + w + ": " + (fallbackHits.length ? fallbackHits.join(" ") : "none"));
  }}
  document.getElementById("out").textContent = lines.join("\n");
}}
run().catch(e => document.getElementById("out").textContent = "ERROR " + e);
</script></body></html>"#
    );
    fs::write(Path::new(out_dir).join("page.html"), html).unwrap();
    println!("wrote {out_dir}/page.html");
}
