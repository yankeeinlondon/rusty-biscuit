# Mermaid Rendering

Darkmatter renders Mermaid diagrams to both terminal (via mmdc CLI) and browser (via mermaid.js).

## Browser Rendering

When targeting the browser, mermaid code blocks are rendered client-side using mermaid.js from CDN.

### Theming

All rendering uses CSS variables for light/dark mode support. Mermaid uses the "base" theme with custom variable mappings:

- `--mermaid-background`
- `--mermaid-primary-color`
- `--mermaid-secondary-color`
- `--mermaid-tertiary-color`
- `--mermaid-primary-border-color`
- `--mermaid-secondary-border-color`
- `--mermaid-tertiary-border-color`
- `--mermaid-primary-text-color`
- `--mermaid-secondary-text-color`
- `--mermaid-tertiary-text-color`
- `--mermaid-line-color`
- `--mermaid-text-color`
- `--mermaid-main-bkg`
- `--mermaid-node-border`

## Terminal Rendering

Terminal rendering requires the `--mermaid` flag. When enabled:

1. **Validation**: Diagram size must be < 10KB
2. **Rendering**: Creates temp file, runs `mmdc` CLI with dark theme and icon packs
3. **Display**: Shows PNG output via viuer
4. **Cleanup**: Removes temporary files

### CLI Detection

The module uses a fallback chain:

1. **Direct `mmdc`**: If globally installed and in PATH
2. **npx fallback**: `npx -p @mermaid-js/mermaid-cli mmdc` with warning:

   ```
   - Mermaid diagrams require mmdc to render to the terminal
   - You do not have the Mermaid CLI installed, using npx to install temporarily
   - To install permanently: npm install -g @mermaid-js/mermaid-cli
   ```

3. **Error**: If neither available, returns error asking for Node.js/npm

### Icon Packs

Terminal rendering enables these icon packs:

- `@iconify-json/fa7-brands` - Font Awesome 7 brand icons
- `@iconify-json/lucide` - Lucide icons
- `@iconify-json/carbon` - Carbon Design icons
- `@iconify-json/system-uicons` - System UI icons

### Fallback Behavior

When terminal rendering fails or is unsupported (no Kitty graphics, mmdc not installed), the diagram is displayed as a syntax-highlighted code block with the `mermaid` language identifier.
