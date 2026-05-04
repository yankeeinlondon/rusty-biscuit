#[cfg(test)]
mod tests {
    use darkmatter::markdown::{Markdown, output::{TerminalOptions, for_terminal}};

    #[test]
    fn test_list_after_paragraph_in_blockquote() {
        let content = "> Paragraph one\n> - Item 1\n> - Item 2";
        let md: Markdown = content.into();
        let mut options = TerminalOptions::default();
        options.max_width = Some(80);
        let output = for_terminal(&md, options).unwrap();
        println!("{}", output);
    }
}
