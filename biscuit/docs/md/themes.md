# Language Grammars


## Supported Themes

### Via Syntect

| Dark Themes | Light Themes |
| ----------- | ------------ |
| base16-ocean.dark | base16-ocean.light |
| Solarized (dark) | Solarized (light) |
| | InspiredGithub |
| base16-mocha.dark | |
| base16-eighties.dark | |


### Via `two-face`

| Dark Themes | Light Themes |
| ----------- | ------------ |
| Coldark-Cold | Coldark-Cold | 
| Gruvbox-Dark | Gruvbox-Light |
| Monokai Extended | Monokai Extended Light |
| OneHalfDark | OneHalfLight |
| Solarized | Solarized |
| | Github |
| 1337 | |
| DarkNeon | |
| Dracula | |
| Monokai Extended Bright | |
| Monokai Extended Origin | |
| Nord | |
| Sublime Snazzy | |
| Visual Studio Dark+ | | 
| Zenburn | | 

### Theme Enumeration

In our Rust code we will want to add all of these themes into an enumeration; something like:

```rust
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Theme {
    // --- Syntect Defaults ---
    Dark_Base16Ocean,
    Dark_Base16Eighties,
    Dark_Base16Mocha,
    Light_Base16Ocean,
    Light_InspiredGitHub,
    Dark_Solarized,
    Light_Solarized,

    // --- Two-Face / Bat Extras ---
    Dark_1337,
    Light_ColdarkCold,
    Dark_ColdarkDark,
    Dark_DarkNeon,
    Dark_Dracula,
    Light_GitHub,
    Dark_GruvboxDark,
    Light_GruvboxLight,
    Dark_MonokaiExtended,
    Dark_MonokaiExtendedBright,
    Light_MonokaiExtendedLight,
    Dark_MonokaiExtendedOrigin,
    Dark_Nord,
    Dark_OneHalfDark,
    Light_OneHalfLight,
    Dark_SublimeSnazzy,
    Dark_TwoDark,
    Dark_VisualStudioDarkPlus,
    Dark_Zenburn,
}
```

### Theme Descriptions

So that we can at runtime describe any of the themes we will have a helper function similar to:

```rust
use std::collections::HashMap;

pub fn get_theme_descriptions() -> HashMap<Theme, &'static str> {
    let mut descriptions = HashMap::new();

    // Syntect Defaults
    descriptions.insert(Theme::Dark_Base16Ocean, "A balanced, professional dark blue-grey palette with high readability.");
    descriptions.insert(Theme::Dark_Base16Eighties, "A warmer, retro-style dark theme using muted oranges and browns.");
    descriptions.insert(Theme::Dark_Base16Mocha, "A high-contrast dark theme with an earthy, coffee-inspired color palette.");
    descriptions.insert(Theme::Light_Base16Ocean, "A clean, off-white background with professional blue-grey accents.");
    descriptions.insert(Theme::Light_InspiredGitHub, "Mimics the classic, minimalist white-background look of GitHub's mid-2010s syntax.");
    descriptions.insert(Theme::Dark_Solarized, "A precision-tuned, low-contrast teal/cyan base designed to reduce eye strain.");
    descriptions.insert(Theme::Light_Solarized, "A warm parchment-colored light theme using mathematically consistent color relationships.");

    // Two-Face Extras
    descriptions.insert(Theme::Dark_1337, "A vibrant, high-contrast neon-on-black aesthetic often called 'Leet'.");
    descriptions.insert(Theme::Light_ColdarkCold, "A grey-blue light theme focused on WCAG accessibility and reading comfort.");
    descriptions.insert(Theme::Dark_ColdarkDark, "A grey-blue dark variant that avoids harsh pure-black backgrounds for better accessibility.");
    descriptions.insert(Theme::Dark_DarkNeon, "An intense dark theme with highly saturated neon accents that aggressively pop.");
    descriptions.insert(Theme::Dark_Dracula, "A famous high-contrast theme featuring a dark purple background with vampire-inspired highlights.");
    descriptions.insert(Theme::Light_GitHub, "A high-fidelity recreation of the modern light-mode syntax highlighting seen on GitHub.");
    descriptions.insert(Theme::Dark_GruvboxDark, "A retro 'groove' theme with a distinct yellowish/brownish tint and pleasant contrast.");
    descriptions.insert(Theme::Light_GruvboxLight, "A warm light theme that looks like old paper or sand; less clinical than pure white.");
    descriptions.insert(Theme::Dark_MonokaiExtended, "A modern take on the classic charcoal background with bright neon-pastel highlights.");
    descriptions.insert(Theme::Dark_MonokaiExtendedBright, "A version of Monokai with maximum saturation and brightness for the highlights.");
    descriptions.insert(Theme::Light_MonokaiExtendedLight, "A rare translation of the neon Monokai logic onto a light-colored background.");
    descriptions.insert(Theme::Dark_MonokaiExtendedOrigin, "Sticks closer to the original proportions and feel of the very first Monokai release.");
    descriptions.insert(Theme::Dark_Nord, "An elegant, 'arctic' theme using sophisticated frosty blues and slate greys.");
    descriptions.insert(Theme::Dark_OneHalfDark, "A very clean, neutral dark theme based on the Atom editor aesthetic.");
    descriptions.insert(Theme::Light_OneHalfLight, "A minimalist, paper-like light theme widely loved for its simplicity.");
    descriptions.insert(Theme::Dark_SublimeSnazzy, "A vibrant, elegant theme with very bright colors that shine on high-res displays.");
    descriptions.insert(Theme::Dark_TwoDark, "A faithful port of Atom's 'One Dark' theme; highly balanced for professional use.");
    descriptions.insert(Theme::Dark_VisualStudioDarkPlus, "Mimics the default VS Code look; very familiar and close to a GitHub Dark experience.");
    descriptions.insert(Theme::Dark_Zenburn, "A low-contrast 'alien' green/grey theme designed for comfort during long nights.");

    descriptions
}
```



## Theme Pairings

This list represents all themes we support but we will want a useful light/dark abstraction so that a user can give a simpler name and it will adapt to the terminal switching between light and dark.

For instance:

- If a user specifies they want to use the `github` theme then we should be able to map that to a light and dark theme.
    - Some themes have an obvious counter part (e.g., `THEME::LIGHT_Solarize` and `THEME::DARK_Solarize`)
    - Other themes do not including `github` (which only has a light variant)

To address this we'll use another enumeration:

```rust
pub enum ThemePair {
    // natural light/dark pairing
    Base16Ocean(Theme::Light_Base16Ocean, Theme::Dark_Base16Ocean),
    GruvBox(Theme::Light_GruvboxLight, Theme::Dark_GruvboxDark),
    OneHalf(Theme::Light_OneHalfLight, Theme::Dark_OneHalfDark),
    // Base16Mocha is only a dark theme, map -- by default -- all missing 
    // light pairings to Theme::OneHalfLight
    Base16Mocha(Theme::Light_OneHalfLight, Theme::Dark_Base16Mocha),
    // the only two "light only" themes are `Github` and `GithubInspired`
    // in both cases we'll map to Theme::Dark_VisualStudioDarkPlus as 
    // apparently the map to each other well
    Github(Theme::Light_Github, Theme::Dark_VisualStudioDarkPlus),
    GithubInspired(Theme::Light_GithubInspired, Theme::Dark_VisualStudioDarkPlus)
    // ...
}
```

> **Note:** we may be able to get away from having _another_ description for ThemePair's if we just agree to use a formula for combining the light and dark themes in a smart way:
>
> - bullet point description for themes with natural light/dark
>     - **light mode:** _description_
>     - **dark mode:** _description_
> - for dark-only named themes
>     - **dark mode** (THEME NAME): _description_
>     - _light mode_ (half_light): _description_
> - for light-only named themes
>     - **light mode** (THEME NAME): _description_
>     - _dark mode_ (visual_studio_dark_plus): _description_



## Theme Loader

When we want to use a theme, the syntax will vary slightly based on whether we're using a theme from `syntect` or `two_pair`. The key differences are:

- Loading: syntect requires you to initialize a ThemeSet (usually via load_defaults()), while two-face provides the extra() function which returns a ThemeSet populated with the larger library.

- Access: While both allow string lookups (e.g., themes["Name"]), two-face provides the EmbeddedThemeName enum, which prevents runtime crashes caused by typos in theme names.

- Highlighting Logic: Once you have the Theme reference (like nord or github_light above), the actual code to highlight text is identical—both are standard syntect::highlighting::Theme objects.

### Code Example: Loading Themes

Here's just a simple example of using both a `syntect` and `two_face` themes.

```rust
use syntect::highlighting::ThemeSet;
use two_face::theme::EmbeddedThemeName;

fn main() {
    // --- 1. Using a Syntect Built-in Theme ---
    // These are bundled inside the syntect crate itself.
    let syntect_themes = ThemeSet::load_defaults();
    let github_light = &syntect_themes.themes["InspiredGitHub"];
    
    println!("Loaded Syntect theme: {}", github_light.name.as_ref().unwrap());

    // --- 2. Using a Two-Face Theme ---
    // These are loaded from the two-face 'extra' collection.
    let extra_themes = two_face::theme::extra();
    
    // You can use the type-safe Enum provided by two-face
    let nord = extra_themes.get(EmbeddedThemeName::Nord);
    
    // Or access by string name if you prefer
    let dracula = &extra_themes.themes["Dracula"];

    println!("Loaded Two-Face theme: {}", nord.name.as_ref().unwrap());
    println!("Loaded Two-Face theme: {}", dracula.name.as_ref().unwrap());
}
```

### How we will Implement Loaders

First of all we'll establish a static lookup table which maps a `Theme` to the configuration we'll need to load it.


```rust
use std::collections::HashMap;
use lazy_static::lazy_static;
use syntect::highlighting::{Theme as SyntectTheme, ThemeSet};
use two_face::theme_data;

enum ThemeSource {
    Syntect(String),
    TwoFace(EmbeddedThemeName)
}

pub trait ThemeLoaderExt {
    fn get_theme(&self, theme: Theme) -> SyntectTheme;
}

impl ThemeLoaderExt for HashMap<Theme, ThemeSource> {
    fn get_theme(&self, theme: Theme) -> SyntectTheme {
        let source = self.get(&theme)
            .expect("Theme not defined in THEME_LOADER map");

        match source {
            ThemeSource::Syntect(name) => {
                // Load from syntect's default internal collection
                let ts = ThemeSet::load_defaults();
                ts.themes.get(*name)
                    .cloned()
                    .expect("Failed to find internal syntect theme")
            }
            ThemeSource::TwoFace(embedded_name) => {
                // Load from two-face's embedded lazy-loaded binary data
                two_face::get_theme(embedded_name)
            }
        }
    }
}

lazy_static {
    static ref THEME_LOADER: HashMap<Theme, ThemeSource> = {
        let p = HashMap::new();
        p.insert(THEME::Light_Github, TwoFace(EmbeddedTHemeName::Github));
        p.insert(THEME::Light_GithubInspired, Syntect("InspiredGithub"));
        // ...

        p
    };
}
```

