

You're right to ask about customizing Mermaid's appearance. Mermaid's theming system is primarily driven through **CSS variables (custom properties)** and a configuration object called `themeVariables`, rather than traditional CSS classes. This approach allows for powerful, global, and diagram-specific styling without needing to write CSS directly.

Here's a comprehensive breakdown of how Mermaid's theming system works and how to customize it.

### 🎨 1. Understanding Mermaid's Theme System

Mermaid uses a **theme** to determine the base look and feel. Themes are essentially a collection of variable definitions that control colors, fonts, borders, and other visual aspects.

```mermaid
flowchart TD
    A[Mermaid Theme System] --> B[Site-wide Theme<br>via `mermaid.initialize()`]
    A --> C[Diagram-specific Theme<br>via Frontmatter Config]
    B --> D[Built-in Themes]
    C --> D
    D --> E["default"]
    D --> F["neutral"]
    D --> G["dark"]
    D --> H["forest"]
    D --> I["base<br>✨ Only Fully Customizable Theme"]
    I --> J[Customized via<br>themeVariables]
    J --> K[Global Variables]
    J --> L[Diagram-specific Variables]
    K --> M[Apply to All Diagrams]
    L --> N[Apply to Specific Diagram Types]
```

#### 🔧 Setting the Theme

You can set the theme in two ways:

1. **Site-wide (Global)**: This sets the default theme for all diagrams in your application or page.

    ```javascript
    // Initialize Mermaid with a specific theme
    mermaid.initialize({
      securityLevel: 'loose', // Often needed for external interactions
      theme: 'base', // 'base' is the only theme that allows full customization via themeVariables
      // theme: 'dark', // Or use any other built-in theme
    });
    ```

2. **Diagram-specific (Local)**: Override the global theme for an individual diagram using **Frontmatter**.

    ```markdown
    ---
    config:
      theme: 'forest'
    ---
    graph TD
      A[Start] --> B[End]
    ```

> 💡 **Key Point**: Only the **`'base'`** theme can be deeply customized by modifying the `themeVariables`. Other themes (`'default'`, `'dark'`, etc.) are presets that use their own fixed variable values. You can still use them, but you can't override their individual variables as effectively. For maximum control, start with `theme: 'base'` and then define your own `themeVariables`.

### 📊 2. Core Theme Variables (`themeVariables`)

When you set `theme: 'base'`, you can provide a `themeVariables` object to override the default values. These variables control the fundamental look and often have derived values for other properties.

| Variable | Default Value | Description & Notes |
| :--- | :--- | :--- |
| **`darkMode`** | `false` | Affects how derived colors (like text contrast) are calculated. Set to `true` for dark-mode diagrams. |
| **`background`** | `#f4f4f4` | The background color of the entire diagram canvas. Used as a base to calculate other contrasting colors. |
| **`fontFamily`** | `'trebuchet ms, verdana, arial'` | The font family used for all text in the diagram. |
| **`fontSize`** | `16px` | The base font size. |
| **`primaryColor`** | `#fff4dd` | **Core variable**. The main background color for nodes. Many other colors (like `primaryBorderColor`, `primaryTextColor`) are calculated from this. |
| **`primaryTextColor`** | *Calculated* | The text color inside nodes with `primaryColor` backgrounds. Automatically set for contrast. |
| **`primaryBorderColor`** | *Calculated* | The border color for nodes with `primaryColor` backgrounds. Typically a darker or inverted shade of `primaryColor`. |
| **`secondaryColor`** | *Calculated* | Used for secondary elements (e.g., alternate rows, some node backgrounds). Often a complementary or variant of `primaryColor`. |
| **`secondaryBorderColor`** | *Calculated* | Border for elements using `secondaryColor`. |
| **`secondaryTextColor`** | *Calculated* | Text color for elements using `secondaryColor`. |
| **`tertiaryColor`** | *Calculated* | Used for tertiary elements (e.g., cluster backgrounds). Often a lighter or more desaturated variant of `primaryColor`. |
| **`tertiaryBorderColor`** | *Calculated* | Border for elements using `tertiaryColor`. |
| **`tertiaryTextColor`** | *Calculated* | Text color for elements using `tertiaryColor`. |
| **`lineColor`** | *Calculated* | The color of lines and edges (e.g., in flowcharts, sequence diagrams). Often calculated from `background` for contrast. |
| **`textColor`** | *Calculated* | The general text color for labels, titles, and other text not inside a colored node (e.g., signal labels in sequence diagrams). |
| **`mainBkg`** | *Calculated* | The background color for the main elements of various diagrams (e.g., rectangles in flowcharts, classes in class diagrams, actors in sequence diagrams). Often identical to `primaryColor`. |

> ⚠️ **Important**: Mermaid's theming engine **only recognizes hex color codes** (e.g., `'#ff0000'`), not color names (e.g., `'red'`). Always use hex codes for `themeVariables`【turn0fetch0】.

### 🔷 3. Diagram-Specific Variables

Beyond the core variables, you can customize variables that are specific to each diagram type. These are prefixed or grouped logically. Here are the most important ones:

#### 📋 For Flowcharts

| Variable | Default Value | Description |
| :--- | :--- | :--- |
| `nodeBorder` | `primaryBorderColor` | The border color of flowchart nodes. |
| `clusterBkg` | `tertiaryColor` | The background color of subgraphs (clusters). |
| `clusterBorder` | `tertiaryBorderColor` | The border color of subgraphs. |
| `defaultLinkColor` | `lineColor` | The color of the arrows/edges between nodes. |
| `titleColor` | `tertiaryTextColor` | The color of the diagram title. |
| `nodeTextColor` | `primaryTextColor` | The color of text inside flowchart nodes. |

#### 🔁 For Sequence Diagrams

| Variable | Default Value | Description |
| :--- | :--- | :--- |
| `actorBkg` | `mainBkg` | Background color of the actor (participant) boxes. |
| `actorBorder` | `primaryBorderColor` | Border color of the actor boxes. |
| `actorTextColor` | `primaryTextColor` | Text color inside the actor boxes. |
| `actorLineColor` | `actorBorder` | Color of the vertical lifeline below the actor. |
| `signalColor` | `textColor` | Color of the arrow (signal) between lifelines. |
| `signalTextColor` | `textColor` | Color of the text label on the signal arrow. |
| `activationBorderColor` | *Calculated* | Border color of the activation box (the narrow rectangle on a lifeline). |
| `activationBkgColor` | `secondaryColor` | Background color of the activation box. |

#### 🥧 For Pie Charts

| Variable | Default Value | Description |
| :--- | :--- | :--- |
| `pie1` ... `pie12` | `primaryColor`, `secondaryColor`, *Calculated* | Background fill color for each pie slice (up to 12). |
| `pieTitleTextSize` | `25px` | Font size of the chart title. |
| `pieTitleTextColor` | `taskTextDarkColor` | Text color of the chart title. |
| `pieSectionTextSize` | `17px` | Font size of the labels for each pie slice. |
| `pieSectionTextColor` | `textColor` | Text color of the slice labels. |
| `pieStrokeColor` | `black` | Border color of each pie slice. |
| `pieStrokeWidth` | `2px` | Border width of each pie slice. |
| `pieOpacity` | `0.7` | Opacity (0-1) of the pie slices. |

#### 🗺️ For Other Diagrams

Similar specialized variables exist for **State Diagrams**, **Class Diagrams**, **User Journey**, and more. The pattern is consistent: look for variables prefixed with the diagram type (e.g., `class*`, `state*`, `journey*`). For a complete list, refer to the official Mermaid Theming documentation【turn0fetch0】.

### 🧩 4. Putting It All Together: Practical Examples

#### Example 1: Site-wide Dark Theme

```javascript
mermaid.initialize({
  securityLevel: 'loose',
  theme: 'base',
  themeVariables: {
    darkMode: true,
    background: '#1e1e1e',
    primaryColor: '#2b2b2b',
    primaryTextColor: '#d4d4d4',
    primaryBorderColor: '#555555',
    lineColor: '#9cdcfe',
    textColor: '#ce9178', // A nice reddish-brown for contrast
    // Override specific diagram variables
    actorBkg: '#2b2b2b',
    actorBorder: '#555555',
    signalColor: '#4ec9b0', // A nice teal for signal lines
    pieStrokeColor: '#1e1e1e', // Blend stroke with background
    pieOpacity: 0.9,
  },
});
```

#### Example 2: Diagram-specific Forest Theme Override

```markdown
---
config:
  theme: 'forest' # Start with the built-in 'forest' theme
  themeVariables: # Then override specific variables to tweak it
    primaryColor: '#e3f2fd' # Change from the default greenish to a light blue
    primaryTextColor: '#0d47a1'
    lineColor: '#1976d2'
    pie1: '#bbdefb' # Customize first pie slice color
---
pie showData
  title My Custom Blue Pie
  "Slice 1" : 40
  "Slice 2" : 30
  "Slice 3" : 20
  "Slice 4" : 10
```

### 🧩 5. CSS Classes? Not for Theming!

It's a common point of confusion: **Mermaid does not use CSS classes for styling its internal elements**. The entire diagram is rendered within an SVG, and all styling information is determined by the `themeVariables` at the time of rendering.

This means you **cannot** target Mermaid diagram elements (like nodes or lines) with your own CSS in the traditional way. For example, this **won't work**:

```css
/* ❌ This CSS will NOT affect Mermaid diagrams */
.mermaid .node rect {
  fill: my-color !important;
}
```

The only way to influence the style is through the `themeVariables` configuration.

### 🚀 6. Advanced Tips & Best Practices

1. **Start with `'base'` Theme**: For any serious customization, always use `theme: 'base'`. This gives you a clean slate and ensures your `themeVariables` are applied predictably without conflicts from other built-in themes.

2. **Leverage Color Calculation**: Mermaid is smart. If you only set `primaryColor`, it will automatically calculate appropriate values for `primaryBorderColor` and `primaryTextColor` based on your `darkMode` setting. Experiment with setting just the core colors first.

3. **Use a Consistent Color Palette**: Define your core colors (`primaryColor`, `secondaryColor`, `tertiaryColor`, `lineColor`, `textColor`, `background`) to match your brand or design system. The derived variables will fall into place.

4. **Test Across Diagrams**: A color change in a flowchart might look great but cause contrast issues in a sequence diagram. Always test your theme variables across all diagram types you intend to use.

5. **For Dynamic Theming**: If you need to switch themes on the fly (e.g., a light/dark mode toggle in your app), you can re-initialize Mermaid with new configuration when the user changes the theme. Note that this will cause all diagrams to re-render.

```javascript
// Function to update Mermaid theme dynamically
function updateMermaidTheme(isDarkMode) {
  mermaid.initialize({
    securityLevel: 'loose',
    theme: 'base',
    themeVariables: {
      darkMode: isDarkMode,
      // ... set other variables based on isDarkMode
    }
  });
  // Re-render all mermaid diagrams in the page
  mermaid.init(undefined, document.querySelectorAll('.mermaid'));
}
```

### 📚 Summary of Key Variables

For quick reference, here are the most impactful variables you'll likely want to customize.

| Variable | Purpose |
| :--- | :--- |
| **`darkMode`** | **CRITICAL**: Set to `true` or `false`. This is the master switch for contrast calculations. |
| **`background`** | Sets the overall diagram background color. |
| **`fontFamily`** & **`fontSize`** | Control the global typography. |
| **`primaryColor`** | **The single most important variable**. Sets the main node background color. |
| **`lineColor`** | Sets the color of arrows, lines, and edges. |
| **`textColor`** | Sets the color of general text labels. |
| **Diagram-specific vars** (e.g., `actorBkg`, `pie1`) | Override styles for specific diagram types. |

By understanding and manipulating these variables, you have complete control over Mermaid's visual output to match any design requirement. The key is to start with the `theme: 'base'` and build your style from the core variables outward.
