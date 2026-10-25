//! AC-E1 evidence: renders one multi-diagram page with the embedded Mermaid
//! font and writes it twice, `page.html` (as rendered) and `nofont.html`
//! (the page's `@font-face` style element removed, the control). Each file
//! carries a probe that writes every label's advance width and SVG box into
//! `<pre id="ac-e1-probe">` once fonts are ready, for `chrome --dump-dom`.

use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::MermaidMode;

const PAGE: &str = r#"# AC-E1

```mermaid
flowchart LR
    A[Start here] --> B{Is it ready?}
    B -->|Yes| C[Café naïve façade]
    B -->|No| D["Wait (5 min) & retry"]
    D --> A
```

```mermaid
sequenceDiagram
    participant Alice
    participant Bob
    Alice->>Bob: Hello Bob, how are you?
    Bob-->>Alice: Fine — thanks!
```

```mermaid
classDiagram
    class Animal {
        +String name
        +int age
        +eat(food) void
    }
    class WaveformAnalyzer
    Animal <|-- WaveformAnalyzer
```

```mermaid
erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE_ITEM : contains
```

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Running : start
    Running --> Idle : stop
```
"#;

// `advance` is a label's advance width in its own computed font; `bbox` is
// Chrome's SVG bounding box of the element. The interval keeps
// `--virtual-time-budget` running until the probe finishes: without a pending
// timer, Linux headless Chromium dumps the DOM before the font promises settle.
const PROBE: &str = r#"<pre id="ac-e1-probe"></pre>
<script>
const keepAlive = setInterval(() => {}, 20);
(async () => {
  await Promise.all([...document.fonts].map((face) => face.load().catch(() => null)));
  await document.fonts.ready;
  const faces = [...document.fonts].map((f) => ({ family: f.family, weight: f.weight, status: f.status }));
  const context = document.createElement('canvas').getContext('2d');
  const labels = [];
  for (const el of document.querySelectorAll('svg text, svg tspan')) {
    const text = el.textContent.trim();
    if (el.children.length || !text) continue;
    const style = getComputedStyle(el);
    context.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
    labels.push({ text, family: style.fontFamily, weight: style.fontWeight, size: style.fontSize,
      advance: context.measureText(text).width, bbox: el.getBBox().width });
  }
  document.getElementById('ac-e1-probe').textContent = JSON.stringify({ ua: navigator.userAgent, faces, labels });
})().catch((error) => { document.getElementById('ac-e1-probe').textContent = 'ERROR ' + error; })
  .finally(() => clearInterval(keepAlive));
</script>"#;

fn document(fragment: &str) -> String {
    format!("<!doctype html>\n<html><head><meta charset=\"utf-8\"></head><body style=\"background:#fff\">\n{fragment}\n{PROBE}\n</body></html>\n")
}

fn without_font(fragment: &str) -> String {
    let at = fragment.find("@font-face").expect("the page embeds a font");
    let start = fragment[..at].rfind("<style>").expect("style start");
    let end = at + fragment[at..].find("</style>").expect("style end") + "</style>".len();
    format!("{}{}", &fragment[..start], &fragment[end..])
}

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&out).expect("create output directory");
    let md: Markdown = PAGE.into();
    let term = Terminal::new_optimistic(100);
    let fragment = DarkmatterPage::new(&term)
        .with_mermaid_mode(MermaidMode::Image)
        .with_embed_mermaid_font(true)
        .render_to_browser(&md)
        .expect("render_to_browser");
    std::fs::write(out.join("page.html"), document(&fragment)).expect("write page");
    std::fs::write(out.join("nofont.html"), document(&without_font(&fragment))).expect("write control");
}
