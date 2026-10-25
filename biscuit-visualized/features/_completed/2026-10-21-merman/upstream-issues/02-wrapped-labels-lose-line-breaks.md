# Upstream issue draft 2: wrapped labels lose their line breaks

> Agents do not file this issue; Ken does. Everything below the rule is ready
> to paste into a new issue on `Latias94/merman`.

- **Repository:** `Latias94/merman`
- **Observed at:** `482fcde7faffb05a466f198085d615e604075d97` (0.9.1)
- **Open it at:** https://github.com/Latias94/merman/issues/new

---

## Title

Wrapped labels lose their line breaks in resvg-safe output (swimlane edge label)

## Body

### Summary

In the resvg-safe pipeline, a swimlane edge label that the layout wraps onto two
lines is drawn as one line. The layout box is sized for two lines, so the single
drawn line overruns it and is cut off. Because the box and the text disagree,
this happens whichever measurer is used, including exact real-font measurement.

### Reproduction

Merman's own fixture `swimlane/upstream_docs_basic`:

```mermaid
swimlane-beta LR
  subgraph Customer
    request[Request service]
    receive[Receive update]
  end

  subgraph Support
    triage[Triage request]
    answer[Send answer]
  end

  subgraph Engineering
    investigate[Investigate issue]
    fix[Prepare fix]
  end

  request --> triage
  triage -->|Known issue| answer
  triage -->|Needs code change| investigate
  investigate --> fix --> answer
  answer --> receive
```

Render with `SvgPipeline::resvg_safe()`.

### Expected

As in Mermaid.js 12.2.0, the edge label "Needs code change" is drawn on two
lines, `Needs code` / `change`, inside its 120 px wide box.

### Actual

The layout box is sized for two lines (120×48), but the text is drawn as one
line, `Needs code change`, which runs past the box and is clipped.

### Evidence

Measured from the rendered SVG with the faces resvg draws with. Every measurer
we tried gives the same result: Merman's native measurement, a host
`TextMeasurer`, and `DeterministicTextMeasurer::with_width_callback`.

| Host and face | Mermaid.js 12.2.0 lines (box) | Merman drawn line | Clipped by |
|---|---|---|---|
| macOS, Trebuchet MS | `Needs code` / `change` (120 px) | 136 px on a 120 px shape | 15.7 px (12%) |
| macOS, Helvetica | — | 140 px on a 120 px shape | 20.4 px (15%) |
| Linux, DejaVu Sans | `Needs code` / `change` (120 px) | 155 px on a 120 px shape | 35.4 px (23%) |
| Linux, DejaVu Serif | — | 157 px on a 120 px shape | 37.1 px (24%) |

### Environment

- Merman `482fcde7faffb05a466f198085d615e604075d97` (0.9.1),
  `default-features = false`, features `svg`, `all-diagrams`,
  `layout-cytoscape` (plus `native-fonts` for the native comparison),
  `SvgPipeline::resvg_safe()`, site config `{"theme": "dark"}`, ELK off.
- Rasterizer: resvg/usvg 0.47.0, fontdb 0.23.0, rustybuzz 0.20.1.
- Hosts: macOS 27.2 (Apple M4 Max); Debian 13, whose only fonts are DejaVu.
- Reference: Mermaid.js 12.2.0 in headless Chrome 131, with `fontFamily` set
  to the face each host draws with.
