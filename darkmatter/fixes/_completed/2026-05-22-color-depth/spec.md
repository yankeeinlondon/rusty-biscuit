There's a latent library issue behind this: DarkmatterPage::new(&term) captures terminal.color_mode but not
terminal.color_depth, so a page built from a specific Terminal still renders with ambient-env color depth
unless with_color_depth is called. I fixed the tests to be hermetic (the review's explicit prescription)
rather than changing construction behavior, since that would alter the documented byte-for-byte for_terminal
equivalence and all CLI output. If you'd like the page to honor its terminal's color depth by default,
that's a separate, larger change I can scope out.
