---
sequence: ./style-guide.yaml 
context: |-
    We have started to build a TUI component library for [Ratatui](https://ratatui.rs/) and before we go any further we want to build up a "design language" / "style guide" that we can
    use both as a way to better describe our future needs as well as our current component library while hopefully aligning our design toward existing practices in the TUI space.
style_guide: "@biscuit-tui/style-guide/style-guide.md"
---
::block when="state.name == "Design Language"

## Context

{{context}}

## Task

You are an experienced design lead tasked with crafting a comprehensive design language and style guide for text‑based user interface (TUI) applications. Your guide must provide clear, actionable standards covering typography, color, layout, components, iconography, interactions, accessibility, and examples. Follow these requirements:

1. **Context & Philosophy** – Explain the purpose of the design language and its guiding principles. Describe how TUIs differ from graphical interfaces and why consistent styling improves usability and accessibility.

2. **Typography**  
   - **Primary typefaces** – Recommend sans‑serif, monospaced or easy‑to‑read fonts; avoid italics, script or highly decorative styles. ADA/Section 508 guidelines specify that screen fonts should be sans‑serif and at least 3/16 inch (about 16pt) high for signage, with sufficient contrast [oai_citation:0‡section508.gov](https://www.section508.gov/develop/fonts-typography/#:~:text=The%20ADA%20and%20ABA%20accessibility,fonts%20in%20a%20few%20places). Clarify that typical body text can be 11–12 pt and should contrast strongly with the background [oai_citation:1‡section508.gov](https://www.section508.gov/develop/fonts-typography/#:~:text=Please%20no%21%20That%20is%20a,11pt%2C%20or%2013%20to%2015px).  
   - **Hierarchy & sizing** – Define typographic scales for headings, subheadings, body text, and captions, ensuring readability when resized.  
   - **Spacing** – Specify recommended line heights, letter spacing, and margins to maintain legibility and visual rhythm.

3. **Color palette**  
   - **Capabilities & fallback** – Explain terminal color limitations (monochrome, 8‑color, 16‑color, 256‑color, and 24‑bit true color). Not all terminals support true color, and the first 16 colors are often user‑customizable [oai_citation:2‡p.janouch.name](https://p.janouch.name/article-tui.html#:~:text=Colours). Provide strategies for detecting color capabilities and define fallback palettes, including a monochrome option and instructions to respect the `NO_COLOR` environment variable [oai_citation:3‡p.janouch.name](https://p.janouch.name/article-tui.html#:~:text=Purely%20monochromatic%20terminal%20emulators%20are,variable%2C%20while%20you%27re%20at%20it).  
   - **Contrast & accessibility** – Ensure text and icons meet WCAG contrast ratios: normal text must have a contrast ratio of at least 4.5:1, while large text (≥16 pt bold) can be 3:1 [oai_citation:4‡w3.org](https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum.html#:~:text=The%20visual%20presentation%20of%20text,5%3A1%2C%20except%20for%20the%20following). Emphasize dynamic high‑contrast text where backgrounds may vary [oai_citation:5‡nick-black.com](https://nick-black.com/dankwiki/index.php/Notcurses#:~:text=,color%20of%20higher%20translucent%20ones). State that color must not be the sole means of conveying information; supplement colors with symbols or text [oai_citation:6‡section508.gov](https://www.section508.gov/develop/fonts-typography/#:~:text=1,text%20to%20convey%20the%20information).

4. **Layout & grid**  
   - Propose a simple, consistent grid system suitable for fixed‑width terminal cells (e.g., 80×24 baseline). Address alignment, margins, and spacing guidelines for forms, lists, and panels. Include guidance on using Unicode box‑drawing characters or simple ASCII to construct borders and separators, noting that some terminals may lack full Unicode support [oai_citation:7‡nick-black.com](https://nick-black.com/dankwiki/index.php/Notcurses#:~:text=Notcurses%20understands%20Unicode%20wide%20characters%2C,are%20all%20terminal%20emulator%20limitations).  
   - Provide templates for common layouts (e.g., navigation sidebars, status bars, pop‑up dialogs, tabbed panes) with attention to readability and space constraints.

5. **Interactive components**  
   - **Menus, lists & forms** – Define standard components (menus, lists, checkboxes, radio buttons, input fields, buttons, sliders) with states (default, focused, selected, disabled, error). Describe keyboard interactions (e.g., Tab/Shift+Tab to move focus; Enter/Space to activate) and ensure components work without a mouse.  
   - **Feedback & notifications** – Specify patterns for progress bars, spinners, toast messages, alerts, and error messages. Use clear language and avoid ambiguous tones. Suggest audible or visual cues (e.g., beeps with optional vibration) for critical alerts.  
   - **Focus & navigation** – Emphasize that all content must be accessible with the keyboard alone; provide visible focus indicators and maintain logical navigation order [oai_citation:8‡webaim.org](https://webaim.org/techniques/keyboard/#:~:text=Important). Do not remove focus outlines or hide the indicator; instead, style it with high contrast for clarity [oai_citation:9‡webaim.org](https://webaim.org/techniques/keyboard/#:~:text=Focus%20indicators).

6. **Iconography & symbols**  
   - Recommend a set of ASCII or Unicode symbols for common actions (e.g., arrows, checkmarks, warnings) and advise pairing icons with text labels to avoid reliance on color alone. Mention fallback options for terminals lacking Unicode support.

7. **Accessibility & inclusive design**  
   - Summarize key accessibility requirements: sans‑serif fonts with sufficient size and contrast [oai_citation:10‡section508.gov](https://www.section508.gov/develop/fonts-typography/#:~:text=The%20ADA%20and%20ABA%20accessibility,fonts%20in%20a%20few%20places) [oai_citation:11‡section508.gov](https://www.section508.gov/develop/fonts-typography/#:~:text=Please%20no%21%20That%20is%20a,11pt%2C%20or%2013%20to%2015px); keyboard‑only operation with visible focus indicators [oai_citation:12‡webaim.org](https://webaim.org/techniques/keyboard/#:~:text=Important); logical navigation order and skip‑to‑main‑content options; support for screen readers and assistive technologies; resizable text without loss of function [oai_citation:13‡section508.gov](https://www.section508.gov/develop/fonts-typography/#:~:text=1,text%20to%20convey%20the%20information); and the ability to toggle color schemes (dark/light/monochrome).  
   - Note that color should never be the sole method of conveying status or meaning [oai_citation:14‡section508.gov](https://www.section508.gov/develop/fonts-typography/#:~:text=1,text%20to%20convey%20the%20information). Provide alternative textual cues or symbols.

8. **Component library & examples**  
   - Compile a reference library of components with code snippets (e.g., pseudocode or markup for ncurses, Notcurses, Bubble Tea, etc.). Include example screens demonstrating the design language applied to a small application (e.g., a settings menu, file manager, or dashboard).  
   - Highlight how dynamic high‑contrast text and 24‑bit color can be used where supported [oai_citation:15‡nick-black.com](https://nick-black.com/dankwiki/index.php/Notcurses#:~:text=,color%20of%20higher%20translucent%20ones), and how to gracefully degrade to limited palettes.

9. **Documentation & maintenance**  
   - Describe how to document and version the design language. Provide guidance on updating components and ensuring consistency across projects.  
   - Suggest conducting regular accessibility reviews and user testing, especially with users who rely on screen readers or keyboard navigation.

Deliver the style guide as a well‑structured document with sections, clear headings, and bullet lists. Include diagrams or ASCII examples where helpful, and reference relevant standards (WCAG, Section 508) throughout.

::endblock

::block when="component"
## Context

You are responsible for evaluating the "{{state.name}}" component in **biscuit-tui**. It is described as:

{{state.desc}}

And you can find more complete documentation for it at: {{state.documentation}}

## Task

Your task is to write the review document: 

- @biscuit-tui/style-guide/reviews/{{state.name}}-review.md

This document is meant to be a **review** of the current component. This review fits into two main components:

1. Style Guide

    How would we describe this component using the {{style_guide}} style guide?

    - add this content under the H2 heading `## Style Guide Description and Review`
    - look for a clear and concise summary description of this component using the vernacular found the style guide
    - if there are aspects of this component which you feel don't have a good corollary in the style guide, then suggest how the style-guide might be updated to include language to describe it

    Once done with this section, set the `style_suggestions` frontmatter to the review document to a boolean value indicating whether you have provided any suggestions for changes to the style guide.

2. Component Suggestions

    - add this content under the H2 heading `## Component Review`
    - identify ways in which this implementation could be improved functionally, performance wise, or ergonomically
    - look at the current test coverage and recommend ways to fill any identified gaps in coverage

::end-block
