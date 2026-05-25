#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    // Exercise the main markdown parsing path
    let md: darkmatter::markdown::Markdown = input.into();

    // Exercise various accessors and transformations
    let _ = md.content();
    let _ = md.links();
    let _ = md.image_references();
    let _ = md.has_inline_html();
    let _ = md.inline_html_links();
    let _ = md.inline_html_image_references();

    // Exercise the fallible construction path
    let _ = darkmatter::markdown::Markdown::try_from_content(input);

    // Exercise cleanup (mutates content)
    let mut md2 = md.clone();
    md2.cleanup();

    let mut md3 = md.clone();
    md3.cleanup_compact();

    let mut md4 = md.clone();
    md4.cleanup_loose();
});
