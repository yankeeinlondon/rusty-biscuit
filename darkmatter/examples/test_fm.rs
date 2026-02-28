use darkmatter::markdown::Markdown;

fn main() {
    let content = std::fs::read_to_string("../../sniff/docs/audio-programming/macOS.md").unwrap();
    let md: Markdown = content.as_str().into();
    
    println!("=== Content (first 800 chars) ===");
    let c = md.content();
    println!("{}", &c[..800.min(c.len())]);
    println!("\n=== Headings (first 5) ===");
    let toc = md.toc();
    for h in toc.all_headings().iter().take(5) {
        println!("level={}, title=\"{}\"", h.level, h.title);
    }
}
