//! Diagrams are rasterized once, at their display size, and their PNG bytes
//! reach the Kitty and iTerm2 encoders unchanged. A Sixel route, which has
//! no transparency, gets an opaque raster on the terminal's background.

use std::rc::Rc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::terminal_image::ImageWidth;
use biscuit_terminal::discovery::detection::{ImageSupport, TerminalApp};
use biscuit_terminal::discovery::fonts::CellSize;
use biscuit_terminal::discovery::osc_queries::RgbValue;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Length, TargetValue};
use biscuit_visualized::components::{
    GraphExpression, GraphInputSyntax, MermaidDiagram, TerminalDiagramRenderer,
};
use renderable::tree::{GraphicsMode, RenderNode, RenderStrictness, TerminalMermaidMode};

const FLOW: &str = "flowchart LR\n    A --> B";
/// Wider than 40 columns of 8-pixel cells at its natural size.
const WIDE_FLOW: &str =
    "flowchart LR\n    Alpha --> Bravo --> Charlie --> Delta --> Echo --> Foxtrot --> Golf";

fn terminal(app: TerminalApp, support: ImageSupport, width: u32) -> Terminal {
    Terminal::builder()
        .app(app)
        .is_tty(true)
        .image_support(support)
        .width(width)
        .cell_size(CellSize {
            width: 8,
            height: 16,
        })
        .build()
}

fn kitty_payload(output: &str) -> Vec<u8> {
    let joined: String = output
        .split("\x1b_G")
        .skip(1)
        .map(|chunk| {
            let body = &chunk[chunk.find(';').unwrap() + 1..];
            &body[..body.find("\x1b\\").unwrap()]
        })
        .collect();
    BASE64.decode(joined).unwrap()
}

fn iterm2_payload(output: &str) -> Vec<u8> {
    let start = output.find("size=auto:").unwrap() + "size=auto:".len();
    let end = start + output[start..].find('\x07').unwrap();
    BASE64.decode(&output[start..end]).unwrap()
}

/// The `c=` columns of the first Kitty chunk.
fn kitty_columns(output: &str) -> u32 {
    let start = output.find(",c=").unwrap() + 3;
    let digits: String = output[start..].chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap()
}

fn png_size(bytes: &[u8]) -> (u32, u32) {
    let image = image::load_from_memory(bytes).unwrap();
    (image.width(), image.height())
}

#[test]
fn a_mermaid_png_reaches_kitty_byte_identical_at_its_display_size() {
    let term = terminal(TerminalApp::Kitty, ImageSupport::Kitty, 80);
    let result = MermaidDiagram::new(FLOW)
        .with_width(ImageWidth::Characters(30))
        .try_render(&term)
        .expect("render");

    let artifact = std::fs::read(&result.png_path).unwrap();
    assert_eq!(kitty_payload(&result.output), artifact);

    // 30 columns of 8 px, padded to whole 16 px rows, and that many rows
    // reserved below the image.
    let (width, height) = png_size(&artifact);
    assert_eq!(width, 240);
    assert_eq!(height % 16, 0, "{height}");
    assert!(result.output.contains("c=30,"), "{:?}", result.output);
    assert!(result.output.contains(&format!("\x1b[{}B", height / 16)), "{:?}", result.output);
}

#[test]
fn a_mermaid_png_reaches_iterm2_byte_identical() {
    let term = terminal(TerminalApp::ITerm2, ImageSupport::ITerm, 80);
    let result = MermaidDiagram::new(FLOW)
        .with_width(ImageWidth::Characters(30))
        .try_render(&term)
        .expect("render");

    assert_eq!(iterm2_payload(&result.output), std::fs::read(&result.png_path).unwrap());
    assert!(result.output.contains("width=30;"), "{:?}", result.output);
}

#[test]
fn a_graph_png_reaches_kitty_byte_identical_at_its_display_size() {
    let term = terminal(TerminalApp::Kitty, ImageSupport::Kitty, 80);
    let result = GraphExpression::for_terminal("a -> b -> c", GraphInputSyntax::Auto)
        .unwrap()
        .with_width(ImageWidth::Characters(20))
        .try_render(&term)
        .expect("render");

    let artifact = std::fs::read(&result.png_path).unwrap();
    assert_eq!(kitty_payload(&result.output), artifact);
    let (width, height) = png_size(&artifact);
    assert_eq!(width, 160);
    assert_eq!(height % 16, 0, "{height}");
}

/// The image is displayed at the columns the component resolved, margins
/// included, not re-resolved against the full terminal width (which drew
/// 90% of 80 = 72 columns into a 60-column slot).
#[test]
fn a_graph_with_margins_is_displayed_at_its_resolved_columns() {
    let term = terminal(TerminalApp::Kitty, ImageSupport::Kitty, 80);
    let mut graph = GraphExpression::for_terminal("a -> b", GraphInputSyntax::Auto)
        .unwrap()
        .with_width(ImageWidth::Percent(0.9));
    graph.layout_mut().margin.left = TargetValue::universal(Length::ch(10));
    graph.layout_mut().margin.right = TargetValue::universal(Length::ch(10));

    let result = graph.try_render(&term).expect("render");

    assert_eq!(kitty_columns(&result.output), 60, "{:?}", result.output);
    assert_eq!(png_size(&std::fs::read(&result.png_path).unwrap()).0, 480);
}

fn promoting(term: &Terminal) -> TerminalRenderOptions {
    let mut opts = TerminalRenderOptions::new(term, RenderStrictness::Strict)
        .with_diagram_renderer(Rc::new(TerminalDiagramRenderer));
    opts.context.graphics_mode = GraphicsMode::Rich;
    opts.context.mermaid_mode = TerminalMermaidMode::Image;
    opts
}

/// A fence inside a list item is sized to the item's columns, not the
/// terminal's, and rasterized at exactly that width.
#[test]
fn the_hook_sizes_a_nested_fence_to_its_available_columns() {
    let term = terminal(TerminalApp::Kitty, ImageSupport::Kitty, 40);
    let fence = || RenderNode::code(Some("mermaid".to_string()), None, WIDE_FLOW);

    let top_level = render_terminal_node(&fence(), &promoting(&term)).expect("render").output;
    assert_eq!(kitty_columns(&top_level), 40, "the wide diagram fills the terminal");

    let list = RenderNode::list(
        false,
        None,
        vec![RenderNode::list_item(
            None,
            vec![RenderNode::paragraph(vec![RenderNode::text("item")]), fence()],
        )],
    );
    let nested = render_terminal_node(&list, &promoting(&term)).expect("render").output;
    let columns = kitty_columns(&nested);
    assert!(columns < 40, "a nested fence gets the item's width, got {columns}");
    assert_eq!(png_size(&kitty_payload(&nested)).0, columns * 8);
}

/// The hook sizes to the request's available columns even when the
/// capability snapshot still reports the full terminal width (contexts
/// narrowed through `for_child`/`with_width` leave the terminal untouched).
#[test]
fn the_hook_sizes_to_the_available_columns_not_the_terminal_width() {
    let term = terminal(TerminalApp::Kitty, ImageSupport::Kitty, 40);
    let mut opts = promoting(&term);
    opts.context = opts.context.with_width(24);
    let fence = RenderNode::code(Some("mermaid".to_string()), None, WIDE_FLOW);

    let output = render_terminal_node(&fence, &opts).expect("render").output;

    assert_eq!(kitty_columns(&output), 24, "{output:?}");
    assert_eq!(png_size(&kitty_payload(&output)).0, 24 * 8);
}

/// The top-left pixel of a PNG artifact: the diagram's background.
fn corner(path: &std::path::Path) -> [u8; 4] {
    image::open(path).unwrap().to_rgba8().get_pixel(0, 0).0
}

fn sixel_terminal(background: Option<RgbValue>) -> Terminal {
    let mut term = terminal(TerminalApp::Other("xterm".into()), ImageSupport::Sixel, 80);
    term.background_color = background;
    term
}

#[test]
fn a_mermaid_diagram_on_a_sixel_route_is_opaque_on_the_measured_background() {
    let term = sixel_terminal(Some(RgbValue { r: 10, g: 20, b: 30 }));
    let diagram = MermaidDiagram::new(FLOW).with_width(ImageWidth::Characters(30));

    let sixel = diagram.try_render(&term).expect("render");
    assert!(sixel.output.contains("\x1bP9;1;0q"), "{:?}", sixel.output);
    assert_eq!(corner(&sixel.png_path), [10, 20, 30, 255]);

    // The transparent Kitty raster is a different cache artifact.
    let kitty = diagram
        .try_render(&terminal(TerminalApp::Kitty, ImageSupport::Kitty, 80))
        .expect("render");
    assert_ne!(kitty.png_path, sixel.png_path);
    assert_eq!(corner(&kitty.png_path)[3], 0, "Kitty keeps transparency");
}

#[test]
fn a_mermaid_diagram_on_a_sixel_route_without_a_measurement_uses_the_theme_background() {
    let result = MermaidDiagram::new(FLOW)
        .with_width(ImageWidth::Characters(30))
        .try_render(&sixel_terminal(None))
        .expect("render");
    assert!(result.output.contains("\x1bP9;1;0q"), "{:?}", result.output);
    assert_eq!(corner(&result.png_path)[3], 255, "the theme background is opaque");
}

#[test]
fn a_graph_on_a_sixel_route_is_opaque_on_the_measured_background() {
    let result = GraphExpression::for_terminal("a -> b", GraphInputSyntax::Auto)
        .unwrap()
        .with_width(ImageWidth::Characters(20))
        .try_render(&sixel_terminal(Some(RgbValue { r: 200, g: 210, b: 220 })))
        .expect("render");
    assert!(result.output.contains("\x1bP9;1;0q"), "{:?}", result.output);
    assert_eq!(corner(&result.png_path), [200, 210, 220, 255]);
}
