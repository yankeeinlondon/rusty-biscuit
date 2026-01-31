# Terminal Rendering Improvements

**IMPORTANT:** use the `biscuit-terminal` skill for this task

## New Features

1. Cached mermaid renders

    - we will import the `biscuit-hash` library so that we can use it's xxhash functionality
    - when we render a mermaid image we will save this image with a hashed filename where:
        - the mermaid configuration is hashed to create the name
        - we save the image file to a temporary directory which the OS will cleanup for us automatically (at some future point)
    - now when we are asked to render the image we can first quickly check if the cached image is ALREADY available in the temporary directory and use this if it is.

2. TailwindCSS colors

    - We have a robust set of enumerations for color which is encapsulated by the `Color` enum in @biscuit-terminal/lib/src/utils/color.rs`
    - This includes `WebColor` enum and `TailwindColor` which all have color definitions with a "fallback" to a base 16 color palette in case you're operating in a color limited environment.
    - In the any colorization we do within the Biscuit Terminal package we should be leveraging these as a baseline capability
    - Most of the color fundamentals are in place but we need to round out the TailwindColor enum:
        - Tailwind CSS version 4 defines it's colors using OKLCH so that it can address a larger color gamut available to modern browsers on the web
        - For the terminal we're still constrained to an RGB world today so we need to create a `build.rs` file which can iterate over the OKLCH values and map them back to RGB
        - The end result should be a enum which can be mapped back to a `HdrColor` value (RGB, fallback RGB, and OKLCH color)

3. Additional mermaid chart types

    - **bar-chart**
        - the **XY Chart** type from mermaid should be made available as the `bar-chart` command in the CLI
        - the switches available to this command will include similar ones found on other mermaid commands including:
            - `--title` (shortcut `-t`)
            - `--x-axis` (shortcut `-x`)
            - `--y-axis` (shortcut `-y`)
            - `--width` (shortcut `-w`)
        - the orientation of the chart will default to vertical but we provide both of these switches:
            - `--horizontal` (shortcut `-h`)
            - `--vertical` - does nothing as this is the default but just allows a user to be explicit
        - This chart type also supports the following switches
            - `--show-data-label` which turns on the `showDataLabel` property in the mermaid config
            - `--aspect-ratio` / `-r` which describes the aspect ratio (width/height) for the image
                - The **xychart** allows the resolution of the xychart to be set in the configuration with an explicit width and height
                - Rather than exposing the width and height we will
                    - use a width of 1200px when the aspect ratio is greater than 1 and then use the aspect ratio to determine what the height should be
                    - use a height of 800px when the aspect ratio is less than 1 and then use the aspect ratio to determine what the width should be
            - Example xyChart configuration:

              ```yaml
              config:
                  xyChart:
                      width: 1200
                      height: 900
                      showDataLabel: true
              ```

        - Data input
            - the mermaid xychart type supports both _line_ charts and _bar_ charts
            - of course our subcommand `bar-chart` is primarily focused on bar charts not line charts
            - you can have one or more _series_ of data
            - to simplify initially we assume:
                - one series of data
                - data rendered as "bars" (not lines)
            - For cases where this simplified base is ok:
                - `bt bar-chart bar [1,8,7,5]` (this is the format which mermaid uses)
                - `bt bar-chart "1,8,7,5"` (raw CSV as single param)
                - `bt bar-chart 1 8 7 5` (raw values as parameters)
            - If you want more than one data series you can add them with the `--line <csv>` and `--bar <csv>` switches
                - `bt bar-chart 1 8 7 5 --line "2,6, 5,9"`
                    - this uses one of the simplified formats for the primary
                - Note: CSV format _can_ include spaces after the comma but does not need to
            - If you want to add a second series of data using the `line` format you can do that with:
                - `--line <csv>`
            - If you want to add a second series of bar data then include:
                - `--bar <csv>`
            - using the `--line <csv>` and `--bar <csv>` you can add as many data series as you want
        - Color
            - we should expose the `--color <rgb|theme>` switch which can define the color for each data series
            - you can add one or more colors or you can specify a theme:
                - defining RGB values: `bt bar-chart 1 8 7 5 --color #fefefe,#3178C5,#A82146`
                - defining RGB values: `bt bar-chart 1 8 7 5 --color "#fefefe #3178C5 #A82146"`
                - defining using a theme: `bt bar-chart 1 8 7 5 --color "default"`
            - themes will include:
                - `default` will be defined something like:

                  ```rust
                  pub struct XyColorTheme {
                      pub light: Vec<String>,
                      pub dark: Vec<String>,
                  }

                  pub fn xy_theme_default() -> XyColorTheme {
                      XyColorTheme {
                          // For light backgrounds (e.g., #FFFFFF): all of these are ≥3:1 vs white.
                          // Selected from Paul Tol qualitative schemes (muted/vibrant/medium-contrast),
                          // but pruned to avoid low-contrast pastels/yellows on white.  [oai_citation:5‡emitanaka.org](https://emitanaka.org/blog/2022-02-20-color-considerations/color-considerations.html?utm_source=chatgpt.com)
                          light: vec![
                              "#004488", // deep blue
                              "#0077BB", // blue
                              "#117733", // green
                              "#332288", // indigo
                              "#882255", // wine
                              "#AA4499", // purple
                              "#CC3311", // red-orange
                              "#009988", // teal
                          ]
                          .into_iter()
                          .map(str::to_string)
                          .collect(),

                          // For dark backgrounds (e.g., #1E1E1E): Okabe–Ito (with the canonical yellow #F0E442).
                          // Widely used + colorblind-friendly.  [oai_citation:6‡easystats](https://easystats.github.io/see/reference/okabeito_colors.html?utm_source=chatgpt.com)
                          dark: vec![
                              "#E69F00", // orange
                              "#56B4E9", // sky blue
                              "#009E73", // bluish green
                              "#F0E442", // yellow
                              "#0072B2", // blue
                              "#D55E00", // vermillion
                              "#CC79A7", // reddish purple
                              "#999999", // grey
                          ]
                          .into_iter()
                          .map(str::to_string)
                          .collect(),
                      }
                  }
                  ```

                - `blue-purple`

                  ```rust
                  pub struct XyColorTheme {
                      pub light: Vec<String>,
                      pub dark: Vec<String>,
                  }

                  pub fn xy_theme_indigo_ramp() -> XyColorTheme {
                      XyColorTheme {

                          light: vec![
                              Color::Tailwind(Tailwind::Indigo500),
                              Color::Tailwind(Tailwind::Indigo600),
                              Color::Tailwind(Tailwind::Indigo700),
                              Color::Tailwind(Tailwind::Indigo800),
                              Color::Tailwind(Tailwind::Indigo900),
                              Color::Tailwind(Tailwind::Indigo950),
                              Color::Tailwind(Tailwind::Purple500),
                              Color::Tailwind(Tailwind::Purple600),
                              Color::Tailwind(Tailwind::Purple700),
                              Color::Tailwind(Tailwind::Purple800),
                              Color::Tailwind(Tailwind::Purple900),
                              Color::Tailwind(Tailwind::Purple950),
                          ].into_iter().map(str::to_string).collect(),

                          dark: vec![
                              Color::Tailwind(Tailwind::Indigo50),
                              Color::Tailwind(Tailwind::Indigo100),
                              Color::Tailwind(Tailwind::Indigo200),
                              Color::Tailwind(Tailwind::Indigo300),
                              Color::Tailwind(Tailwind::Indigo400),
                              Color::Tailwind(Tailwind::Indigo500),
                              Color::Tailwind(Tailwind::Indigo600),
                              Color::Tailwind(Tailwind::Purple50),
                              Color::Tailwind(Tailwind::Purple100),
                              Color::Tailwind(Tailwind::Purple200),
                              Color::Tailwind(Tailwind::Purple300),
                              Color::Tailwind(Tailwind::Purple400),
                              Color::Tailwind(Tailwind::Purple500),
                              Color::Tailwind(Tailwind::Purple600),
                          ].into_iter().map(str::to_string).collect(),
                      }
                  }
                  ```

                - `rag` (e.g., red-amber-green)

                  ```rust
                  pub struct XyColorTheme {
                      pub light: Vec<String>,
                      pub dark: Vec<String>,
                  }

                  pub fn xy_theme_rag() -> XyColorTheme {
                      XyColorTheme {
                          // For WHITE background (#FFFFFF): pick the darker, higher-contrast end.
                          // Red / Amber / Green are from Tailwind v3 default palette.  [oai_citation:3‡v3.tailwindcss.com](https://v3.tailwindcss.com/docs/customizing-colors)
                          light: vec![
                              Color(Tailwind(Tailwind::Red600)),
                              Color(Tailwind(Tailwind::Amber600)),
                              Color(Tailwind(Tailwind::Green600)),
                              Color(Tailwind(Tailwind::Red700)),
                              Color(Tailwind(Tailwind::Amber700)),
                              Color(Tailwind(Tailwind::Green700)),
                              Color(Tailwind(Tailwind::Red800)),
                              Color(Tailwind(Tailwind::Amber800)),
                              Color(Tailwind(Tailwind::Green800)),
                              Color(Tailwind(Tailwind::Red900)),
                              Color(Tailwind(Tailwind::Amber900)),
                              Color(Tailwind(Tailwind::Green900)),
                          ]
                          .into_iter()
                          .map(str::to_string)
                          .collect(),

                          dark: vec![
                              Color(Tailwind(Tailwind::Red300)),
                              Color(Tailwind(Tailwind::Amber300)),
                              Color(Tailwind(Tailwind::Green300)),
                              Color(Tailwind(Tailwind::Red400)),
                              Color(Tailwind(Tailwind::Amber400)),
                              Color(Tailwind(Tailwind::Green400)),
                              Color(Tailwind(Tailwind::Red500)),
                              Color(Tailwind(Tailwind::Amber500)),
                              Color(Tailwind(Tailwind::Green500)),
                              Color(Tailwind(Tailwind::Red600)),
                              Color(Tailwind(Tailwind::Amber600)),
                              Color(Tailwind(Tailwind::Green600)),
                          ]
                          .into_iter()
                          .map(str::to_string)
                          .collect(),
                      }
                  }
                  ```


    - **line-chart**
        - Just like bar-chart except that we default to viewing data as line-chart series versus bar-chart data
    - **timeline**
        - add the mermaid's timeline chart
        - has the following standard switches:
            - `--title` (shortcut `-t`)
            - `--width` (shortcut `-w`)
        - Data can be entered in the following formats:
            - `bt timeline "2002 : LinkedIn" "2004 : Facebook" "2004 : Google"`
            - alternatively you can use numeric switches with a value or a CSV value:
                - `bt timeline --2002 LinkedIn --2004 Facebook --2004 Google`
                - `bt timeline --2002 LinkedIn --2004 Facebook,Google`
            - we can add "sections" with the `--section <section> <...items>` switch:
                - `bt timeline --section 1970 "70's bands" "Rolling Stones" "Led Zeppelin" --section 1980 "80's bands" "AC/DC" "Bad Company"`
    - **state-diagram**
        - modelled nearly identically to the `flowchart` command but using a stateDiagram-v2 chart instead of a flowchart
    - **erd**
        - uses the mermaid's `erDiagram` chart
        - includes the `--title` and `--width` switches
