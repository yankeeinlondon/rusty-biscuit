# Theme & syntax discovery + curation

## List available syntaxes (debug/inspection)

````rust
let syn_set = two_face::syntax::extra_newlines();
for syntax in syn_set.syntaxes() {
    println!("Name: {:<20} Extensions: {:?}", syntax.name, syntax.file_extensions);
}
````

## List available themes (ThemeSet keys)

````rust
let theme_set = two_face::theme::extra();
for name in theme_set.themes.keys() {
    println!("Theme: {name}");
}
````

## Best practice: curate themes

Avoid dumping dozens of theme options into a UX. Prefer:

* 3–8 themes maximum
* names users recognize (Nord, Dracula, Monokai, SolarizedDark)
* separate “dark” and “light” groups if you support both

## Programmatic theme selection

Prefer the enum for embedded themes:

````rust
let theme = &theme_set[two_face::theme::EmbeddedThemeName::Monokai];
````

If you accept user input, map strings to a curated enum set rather than indexing arbitrary theme names.

## Fallback strategy for unknown extensions

A robust pattern:

1. detect by first line (shebang)
1. detect by extension
1. fallback to plain text