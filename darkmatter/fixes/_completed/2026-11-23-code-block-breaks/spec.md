# Keep focused diagnostic excerpts in one code block

Claudine's compose diagnostics can show an enclosing YAML key and several
separate ranges around the offending line. Each range currently renders as
its own Darkmatter code block. As a result, even a one-line omission produces
two panels with repeated language labels and padding.

Render the selected source as one YAML panel. Put a single `⋮` row inside the
panel for each omitted range. Align the marker with the shared line-number
gutter. Keep the original numbers and highlights on the displayed source
lines.

For example, a frontmatter parse error on line 41 can show line 19, line 35,
and lines 38–44. In the panel, line 19 is followed by one gap row and line 35.
A second gap row is followed by line 38. The panel has one language label
and line 41 remains highlighted.

Claudine continues to select the focused lines and enforce the stderr TTY
check. Darkmatter renders the panel and its gap rows. With color disabled,
the selected text and gap markers appear within one plain fence.

Verify a multi-gap excerpt in both terminal modes, including gaps across
a change in the number of gutter digits. Check that adjacent source lines
stay adjacent and that omitted content is absent.
