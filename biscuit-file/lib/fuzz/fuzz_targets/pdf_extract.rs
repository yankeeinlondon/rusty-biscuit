#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Ensure the data at least starts with the PDF magic bytes
    // so Pdf::from_bytes doesn't immediately reject it.
    if data.len() < 5 {
        return;
    }

    let mut padded = Vec::with_capacity(data.len() + 5);
    padded.extend_from_slice(b"%PDF-");
    padded.extend_from_slice(data);

    if let Ok(pdf) = biscuit_file::Pdf::from_bytes(padded) {
        // Exercise the two main extraction paths
        let _ = pdf.as_text();
        let _ = pdf.toc();
        let _ = pdf.as_markdown(Default::default());
    }
});
