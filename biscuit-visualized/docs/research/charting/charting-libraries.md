---
prompt: |-
	There are a ton of charting libraries in existence due to the popularity of chart based data visualizations. Here are a few of the notable ones:

    - Javascript/Typescript Ecosystem
        - [Chart.js](https://www.chartjs.org/)
        - [Plotly](https://plotly.com/javascript/) - a scientific charting library
        - [Apache ECharts](https://echarts.apache.org/en/index.html)
        - [Nivo](https://github.com/plouc/nivo) - A popular React library built on top of React
        - [D3.js](https://d3.js) - the OG data visualization package, good for bespoke visualizations; not just charting
        - [ApexCharts](https://apexcharts.com/) - Not open source but high quality; there is a "community edition" which is free for organizations under $2m in revenue.
        - [Highcharts](https://www.highcharts.com/) - Not open source but used extensively for financial reporting
        - [amCharts](https://www.amcharts.com/javascript-charts/) - Not open source but a powerful library that is a popular choice
    - OS Binaries
        - [Graphviz](https://graphviz.org/)
    - Rust Ecosystem
        - [Plotters](https://crates.io/crates/plotters)
        - [FGFPlots](https://crates.io/crates/pgfplots)

    ## Acting as an Orchestrator

    You should act as an orchestrator during this exercise to preserve context window. Remember to communicate progress as much as possible throughout the process so the caller has a good sense of progress. Also update the body of this document as soon as you can.
    
    ## Your Task

    Run the following tasks concurrently with subagents:
    
    - add an H2 heading called "## Available Libraries" and put the libraries above into this section
    - add another H2 heading called "## Library Details"
    - every library listed above should get a subagent assigned to it do a detailed analysis of the library, with at least the following information:
        - name
        - license
        - URLs (website, repo, examples, etc)
        - descriptive overview
        - gotchas that developers report running into with given package and ways to work around them
        - when is this library a good fit?
        - maturity, momentum, last commit date, contributors
        - any big name brands using this library
        - latest major version? when was it released? what were big changes in this release? what warning signs should you look out for in code examples that indicate an older version?
    - In parallel with the above subagents run one additional subagent:
        - Told to find good charting libraries for TS/JS, OS Binaries, and Rust ecosystem OUTSIDE of those listed above
        - this subagent should just return a list of all found packages and their main URL
    - When any of the subagents assigned detailed analysis come back, add their results to a H3 section under "Library Details"
    - When the research on other packages comes back add it to the list ini the "Available Libraries" section
    - When all subagents are complete you done with the first section

    Now you will start section 2:

    - for all the "new" libraries which were uncovered in Section 1's research into other libraries, create a subagent:
        - each subagent should do detailed analysis on the library they are assigned
        - use the same characteristics as in Section 1:
            - name
            - license
            - URLs (website, repo, examples, etc)
            - descriptive overview
            - gotchas that developers report running into with given package and ways to work around them
            - when is this library a good fit?
            - maturity, momentum, last commit date, contributors
            - any big name brands using this library
            - latest major version? when was it released? what were big changes in this release? what warning signs should you look out for in code examples that indicate an older version?
    - When each subagent is done, update this document with their research
    - When all subagents are complete you have completed section 2

    Now you will start section 3 (the final section):

    - Add a final H2 section called "Summary: Comparison, Categories, and Recommendations"
    - create a subagent tasked with:
        - thinking about how to categorize and compare the different packages
        - providing summary tables which allow for a quick visual comparisons of libraries or features of these libraries
        - recommend 1 or more packages (either broadly or on a "per use case" basis)
    - when the subagent returns update the summary section with it's conclusions

last_updated: 2026-03-24
---

## Available Libraries

### JavaScript/TypeScript Ecosystem

- [Chart.js](https://www.chartjs.org/) - Simple, flexible charting
- [Plotly](https://plotly.com/javascript/) - Scientific charting library
- [Apache ECharts](https://echarts.apache.org/en/index.html) - Feature-rich interactive charts
- [Nivo](https://github.com/plouc/nivo) - React library built on D3
- [D3.js](https://d3js.org) - The OG data visualization library
- [ApexCharts](https://apexcharts.com/) - High quality charts (community edition free under $2m revenue)
- [Highcharts](https://www.highcharts.com/) - Enterprise financial reporting
- [amCharts](https://www.amcharts.com/javascript-charts/) - Powerful commercial charting
- [Recharts](https://github.com/recharts/recharts) - Composable React charting built on D3
- [visx](https://github.com/airbnb/visx) - Airbnb's low-level React visualization primitives
- [Observable Plot](https://observablehq.com/plot/) - Concise grammar-of-graphics API by the D3 team
- [uPlot](https://github.com/leeoniya/uPlot) - Tiny (~20KB), extremely fast Canvas 2D time-series charts
- [Frappe Charts](https://github.com/frappe/charts) - Simple, responsive, zero-dependency SVG charts
- [billboard.js](https://naver.github.io/billboard.js/) - Easy-interface charting built on D3 (successor to C3.js)
- [AG Charts](https://www.ag-grid.com/charts/) - Feature-rich charting from the AG Grid team
- [AntV G2](https://g2.antv.vision) - Grammar-of-Graphics from Ant Group
- [SciChart.js](https://www.scichart.com/) - WebGL/WASM high-performance charting (commercial)
- [LightningChart JS](https://lightningchart.com/) - GPU-accelerated commercial charting for scientific/real-time data

### OS Binaries

- [Graphviz](https://graphviz.org/) - Graph visualization software
- [gnuplot](http://www.gnuplot.info/) - Portable command-line graphing utility
- [vl-convert](https://github.com/vega/vl-convert) - Rust CLI for converting Vega-Lite specs to SVG/PNG/PDF
- [YouPlot](https://github.com/red-data-tools/YouPlot) - Ruby CLI for terminal-based charts from TSV/CSV
- [lowcharts](https://crates.io/crates/lowcharts) - Rust CLI for low-resolution terminal charts from piped data

### Rust Ecosystem

- [Plotters](https://crates.io/crates/plotters) - Rust drawing/plotting library
- [PGFPlots](https://crates.io/crates/pgfplots) - Rust PGF/TikZ plots
- [charming](https://github.com/yuankunzhang/charming) - Declarative Rust charts powered by Apache ECharts
- [plotly.rs](https://github.com/plotly/plotly.rs) - Rust bindings for Plotly.js
- [plotlars](https://github.com/alceal/plotlars) - Polars DataFrames to Plotly charts
- [charts-rs](https://crates.io/crates/charts-rs) - Pure Rust chart image generation (PNG/SVG)
- [poloto](https://lib.rs/crates/poloto) - Lightweight 2D SVG plotting
- [textplots](https://crates.io/crates/textplots) - Terminal plotting with Unicode braille characters
- [charton](https://lib.rs/crates/charton) - Plotting with Polars support and Altair-like API

## Library Details

### Graphviz

**Name**: Graphviz (Graph Visualization Software)

**License**: Eclipse Public License 1.0 (EPL-1.0). Historically under Common Public License 1.0 (CPL) when originally released by AT&T Labs Research; transitioned to EPL-1.0 around 2004.

**URLs**:

- Website: <https://graphviz.org/>
- Source repo: <https://gitlab.com/graphviz/graphviz>
- Examples gallery: <https://graphviz.org/gallery/>
- Documentation: <https://graphviz.org/documentation/>
- DOT language reference: <https://graphviz.org/doc/info/lang.html>
- Attribute reference: <https://graphviz.org/doc/info/attrs.html>

**Overview**:
Graphviz is an open-source graph visualization toolkit originally developed at AT&T Labs Research starting in the early 1990s. It takes descriptions of graphs written in the DOT language (a simple, declarative, plain-text format) and automatically computes layouts, producing output in SVG, PNG, PDF, PostScript, and many other formats.

The architecture centers on a graph description language and pluggable layout engines:

| Engine | Purpose |
|--------|---------|
| `dot` | Hierarchical/layered layout for DAGs (default, most used) |
| `neato` | Spring model / force-directed for undirected graphs |
| `fdp` | Force-directed placement using Fruchterman-Reingold |
| `sfdp` | Scalable force-directed for very large graphs (thousands+ nodes) |
| `twopi` | Radial layout — root at center, layers radiating outward |
| `circo` | Circular layout — good for cyclic structures |
| `osage` | Recursive array/treemap packing |
| `patchwork` | Squarified treemap layout |

Key features include automatic layout, a rich attribute system (200+ attributes), HTML-like labels, cluster subgraphs, record-based nodes with port connections, C library API (`libgvc`, `libcgraph`), and language bindings for Python, JavaScript, Ruby, Go, Rust, Java, and more.

**Gotchas**:

- **Label escaping**: Special characters (`<`, `>`, `&`, `"`, `{`, `}`, `|`) must be escaped. HTML-like labels use XML escaping, not DOT escaping — mixing these up is frequent.
- **Large graphs choke `dot`**: The hierarchical engine struggles beyond a few thousand nodes. Use `sfdp` for large graphs.
- **Font handling**: Platform-dependent font resolution. Set `fontname` explicitly and verify on target platform.
- **Rank constraints are fragile**: `rank=same/min/max` behave unexpectedly with `rankdir=LR`. Use invisible edges as workarounds.
- **Cluster naming**: Subgraph names must start with `cluster_` to render as a visual box — not obvious for new users.
- **PNG resolution**: Default 96 DPI looks blurry on HiDPI displays. Use `-Gdpi=300` or output SVG.
- **No interactive editing**: Batch renderer only — generate, inspect, tweak source, regenerate.

**When is it a good fit?**:

- Dependency graphs (packages, builds, modules)
- DAGs: workflow pipelines, CI/CD stages, state machines
- Software architecture diagrams (call graphs, class hierarchies)
- Automated/generated diagrams from code analysis or schemas
- Documentation pipelines (Doxygen, Sphinx, Rustdoc integrate natively)
- Version-controllable, reproducible text-based diagrams

NOT a good fit for: pixel-perfect design, interactive diagrams, very large graphs (50K+ without `sfdp`), or WYSIWYG editing.

**Maturity & Momentum**:

- **Age**: 30+ years (development began early 1990s at AT&T Bell Labs)
- **Contributors**: ~80-100 over its history
- **Maintenance**: After slower development (2015-2019), renewed activity from 2020 under new maintainers. Consistent commits through 2024-2025.
- **Release cadence**: Roughly quarterly since 2020 revival
- **Latest release**: v12.2.1 (December 2024)

**Notable Users**: Doxygen, Sphinx, LLVM/Clang, Linux kernel, Terraform, Puppet, Ansible, PostgreSQL (query plan viz), Bazel, NASA, AT&T, Amazon/AWS

**Latest Major Version**:

- **Version**: 12.2.1 (December 2024) — jumped from 2.x to 7.x+ in 2022 with calendar-adjacent versioning
- **Big changes**: C codebase modernization, CGraph library stability, better SVG compliance, `sfdp` performance improvements, CMake as primary build system, security fixes from fuzzing
- **Old version warning signs**: Version numbers in `2.x` range, missing `sfdp`, autotools-only build, `dot -V` reporting below 2.44

---

### Plotly.js

**Name**: Plotly.js

**License**: MIT

**URLs**:

- Website: <https://plotly.com/javascript/>
- GitHub: <https://github.com/plotly/plotly.js>
- Examples Gallery: <https://plotly.com/javascript/#basic-charts>
- API Reference: <https://plotly.com/javascript/reference/>
- npm: <https://www.npmjs.com/package/plotly.js>

**Overview**: Plotly.js is a high-level, declarative charting library built on D3.js (SVG rendering) and stack.gl/regl (WebGL-accelerated rendering). Charts are described as JSON objects, making them language-agnostic and serializable. Supports 40+ chart types: basic (scatter, bar, line, pie), statistical (box plots, histograms, violin), scientific (contour, heatmap, ternary, parallel coordinates), financial (candlestick, waterfall, OHLC), 3D (surface, mesh, scatter3d), and geographic (choropleth, tile maps). Key features include `Plotly.react()` for efficient differential updates, built-in zoom/pan/hover interactivity, image export (PNG/SVG/PDF), and animation support. Ships as ~3.5 MB UMD bundle with partial bundles available (`plotly.js-basic-dist` ~1 MB). Serves as the rendering engine for Plotly's Python, R, Julia, and MATLAB wrappers, plus the Dash framework.

**Gotchas**:

- **Bundle size**: ~3.5 MB min (~1 MB gzipped). Use partial distribution bundles or dynamic imports.
- **DOM-coupled architecture**: Doesn't play naturally with virtual-DOM frameworks (React, Vue). Must manually call `Plotly.react()` on updates.
- **Memory leaks**: Failing to call `Plotly.purge(divElement)` before removing chart divs leaks event listeners and WebGL contexts.
- **Mapbox deprecation**: Mapbox-based traces deprecated in v3; migrate to MapLibre-based alternatives.
- **CSS conflicts**: Injected CSS for modebar/tooltips can collide with app stylesheets.
- **WebGL limits**: ~1M point limit dictated by GPU texture sizes; exceeding silently drops data.
- **Configuration verbosity**: Extremely deep JSON schema; simple formatting tasks require 5+ nested properties.
- **Open issues backlog**: ~800 open issues on GitHub.

**When is it a good fit?**:

- Scientific, statistical, or financial dashboards needing wide chart type variety
- Projects using Plotly's Python/R ecosystem (Dash apps)
- Data exploration tools needing built-in zoom, pan, hover, lasso/box select
- Situations requiring both SVG (publication-quality) and WebGL (large dataset) rendering
- Rapid prototyping with declarative JSON API
- 3D surface/mesh visualization or geographic choropleths

**Maturity & Momentum**:

- **Age**: 10+ years (first published November 2015)
- **GitHub stars**: ~18,150
- **npm weekly downloads**: ~947,000
- **Contributors**: 248
- **Last commit**: March 2026 (actively maintained)
- **Release cadence**: Roughly monthly minor/patch releases
- **Backing**: Plotly Inc. (commercial company, Montreal), sustained through Dash Enterprise product

**Notable Users**: S&P Global, Intuit, US Foods, Molson Coors, UK Power Networks, MD Anderson Cancer Center, Cox Automotive. Widely used across pharmaceutical, energy, finance, and government sectors via Dash.

**Latest Major Version**: v3.0.0 (August 2025)

- Removed deprecated APIs: string-based `title` (use `title.text`), `bardir` (use `orientation`), Transforms API
- Removed trace types: `pointcloud`, `heatmapgl`, `gl2d` subplots
- Dropped IE support, jQuery integration, AMD format
- Build system migrated from webpack to esbuild
- Improved CSP compatibility
- **Old version warning signs**: `layout.title = "string"` instead of object, `bardir: 'h'`, `pointcloud`/`heatmapgl` traces, `transforms` array, `Plotly.plot()` instead of `Plotly.newPlot()`/`Plotly.react()`, AMD/RequireJS patterns

---

### D3.js

**Name**: D3.js (Data-Driven Documents)

**License**: ISC License

**URLs**:

- Website: <https://d3js.org>
- GitHub: <https://github.com/d3/d3>
- Examples Gallery: <https://observablehq.com/@d3/gallery>
- Documentation: <https://d3js.org/getting-started>
- API Reference: <https://d3js.org/d3-selection> (each module has its own docs page)

**Overview**:
D3.js is a low-level JavaScript library for producing bespoke, interactive data visualizations by binding data to the DOM and applying data-driven transformations. Unlike higher-level charting libraries with pre-built chart types, D3 gives direct control over every visual element through ~30 composable sub-libraries. Core modules include: **d3-selection** (DOM manipulation/data joins), **d3-scale** (domain-to-range mapping), **d3-shape** (SVG path generators), **d3-axis** (axis rendering), **d3-geo** (geographic projections), **d3-hierarchy** (tree/treemap/pack layouts), **d3-force** (force-directed simulation), **d3-transition** (animated interpolation), **d3-zoom/d3-brush/d3-drag** (interactions), and **d3-array** (statistical utilities). Operates directly on SVG, Canvas, or HTML with no virtual DOM or rendering abstraction. Can be imported as monolithic bundle or individual micro-packages.

**Gotchas**:

- **Steep learning curve**: Not a charting library — you build charts from primitives. The data-join pattern (enter/update/exit) is confusing at first.
- **`d3.event` removed in v6**: Events now passed as first argument to callbacks. Massive amounts of legacy tutorials still use `d3.event.pageX`.
- **`this` binding**: D3 binds `this` to DOM elements in listeners, which breaks with arrow functions. Use `function()` syntax.
- **`d3.nest` removed in v6**: Replaced by `d3.group`/`d3.rollup` with native Map/Set.
- **SVG coordinates**: Doesn't abstract away top-left origin or inverted y-axis. Margin conventions confuse newcomers.
- **Bundle size**: Full import ~90KB min+gzip. Import only needed sub-modules for production.
- **Enter-update-exit vs join()**: Since v5, `selection.join()` simplifies the old verbose pattern, but most online examples still use the old way.

**When is it a good fit?**:

- Custom, novel visualizations that don't fit standard chart types (infographics, scrollytelling, data art)
- Complex interactive dashboards with fine-grained transition/tooltip/brushing control
- Geographic/cartographic visualizations with custom projections
- Network and hierarchical data (force graphs, trees, Sankeys)
- Data journalism and storytelling requiring pixel-perfect control
- Combining SVG, Canvas, and HTML in one visualization
- NOT a good fit for standard bar/line/pie charts quickly — use Chart.js, Recharts, or Observable Plot instead

**Maturity & Momentum**:

- **Age**: ~15 years (created September 2010)
- **GitHub stars**: ~112,600
- **npm weekly downloads**: ~9,000,000
- **Contributors**: ~153 on main repo; many more across ~30 sub-module repos
- **Last commit**: December 2025
- **Status**: Mature/stable phase. Creator Mike Bostock now focuses on Observable Plot/Framework; D3 gets maintenance updates rather than major new features.

**Notable Users**: The New York Times (Bostock was a graphics editor there), The Washington Post, The Guardian, Reuters, BBC, Observable, Slack, Airbnb (Visx), Uber (kepler.gl), Spotify, GitHub (contribution graphs), Grafana, Plotly (uses D3 as rendering backbone)

**Latest Major Version**: v7.0.0 (June 2021), latest release v7.9.0 (March 2024)

- Shipped exclusively as ES modules (`"type": "module"`)
- `d3.bin` ignores null values; ordinal scales use `InternMap` with `valueOf()`
- Added `d3.mode`, `d3.flatGroup`, `d3.flatRollup`
- Robust geometric predicates for `d3-delaunay`
- **Old version warning signs**: `d3.event` usage (pre-v6), `d3.nest()` (pre-v6), callback-based data loading (pre-v5), `d3.scale.linear()` dotted namespace (pre-v4), `require("d3")` CommonJS (pre-v7), `d3.queue()` (pre-v5), `d3.voronoi()` (pre-v6)

---

### Apache ECharts

**Name**: Apache ECharts

**License**: Apache License 2.0

**URLs**:

- Website: <https://echarts.apache.org/en/index.html>
- GitHub: <https://github.com/apache/echarts>
- Examples Gallery: <https://echarts.apache.org/examples/en/index.html>
- Documentation: <https://echarts.apache.org/en/option.html>
- API Reference: <https://echarts.apache.org/en/api.html>

**Overview**: Apache ECharts is a declarative, JavaScript-based charting library originally created by Baidu, donated to the Apache Software Foundation in 2018, and graduated as a top-level Apache project in 2021. Provides 20+ built-in chart types (line, bar, pie, scatter, candlestick, map, heatmap, tree, treemap, sunburst, sankey, funnel, gauge, radar, boxplot, parallel, graph/network, and more) with a dozen interactive components (tooltip, legend, dataZoom, visualMap, timeline, toolbox, brush, etc.).

Built on **ZRender**, a lightweight 2D rendering engine abstracting Canvas and SVG behind a unified API. Follows a declarative option-merge pattern: describe charts as JSON-like config objects and call `setOption()`. Supports dataset transforms, progressive/stream loading for big data (up to 10M points), responsive resizing, rich text labels, animation transitions, and built-in accessibility (auto-generated ARIA descriptions and decal patterns for colorblind users). Framework-agnostic with wrappers for React, Vue, and Angular.

**Gotchas**:

- **Memory leaks**: Must call `chart.dispose()` on teardown. Failing to do so in SPAs causes leaks.
- **Resize not automatic**: Must call `chart.resize()` manually via `ResizeObserver` or window resize listener.
- **Option merge vs. replace**: `setOption()` merges by default; old series persist unexpectedly. Use `{ notMerge: true }` or `{ replaceMerge: ['series'] }` for clean replacement.
- **Large bundle**: ~800KB minified (~250KB gzipped). Tree-shaking via `echarts/core` with explicit imports is verbose but cuts 50-70%.
- **Tooltip performance**: Complex tooltips with `trigger: 'axis'` on dense time-series cause jank.
- **SSR complexity**: Requires `echarts-server-renderer` or headless Canvas shim; non-trivial setup.
- **TypeScript types lag**: Definitions occasionally fall behind new features.

**When is it a good fit?**:

- Dashboards/BI apps needing wide chart type variety in a single library
- Data-heavy visualizations (millions of points) with Canvas progressive rendering
- Geographic/map visualizations (built-in geo support, GeoJSON)
- Rich interactivity out of the box (brush selection, data zoom, linked views, drill-down)
- Accessibility compliance (WCAG) via built-in ARIA support
- Cross-framework projects needing a framework-agnostic core
- Animated transitions between chart states

**Maturity & Momentum**:

- **Age**: 13 years (created April 2013)
- **GitHub stars**: ~66,000
- **npm weekly downloads**: ~2.27 million
- **Contributors**: 327
- **Last commit**: March 2026 (actively maintained)
- **Release cadence**: Major versions every few years; minor releases every 4-8 months

**Notable Users**: Baidu, Alibaba, Tencent, Huawei, Xiaomi, JD.com, Amazon (internal dashboards), Intel, Siemens, GitLab. Apache Superset ships ECharts as its primary charting engine, exposing it to Airbnb, Netflix, Dropbox, Lyft.

**Latest Major Version**: v6.0.0 (July 2025)

- New default theme, chord series, matrix coordinate system, axis breaks, scatter jittering
- Reusable custom series, dynamic theme switching, improved axis label layout
- Sankey roaming (pan/zoom), graph thumbnails (minimaps)
- **Old version warning signs**: `itemStyle.normal`/`itemStyle.emphasis` nesting (pre-v5), `map: 'china'` without registering GeoJSON (pre-v5), missing `use([CanvasRenderer])` calls with `echarts/core` (pre-v5), `addData()` API (pre-v4), CommonJS `require('echarts')` (pre-v5)

---

### Chart.js

**Name**: Chart.js

**License**: MIT

**URLs**:

- Website: <https://www.chartjs.org/>
- GitHub: <https://github.com/chartjs/Chart.js>
- Examples/Samples: <https://www.chartjs.org/docs/latest/samples/>
- Documentation: <https://www.chartjs.org/docs/latest/>
- Ecosystem catalog: <https://github.com/chartjs/awesome>

**Overview**: Chart.js is an HTML5 Canvas-based charting library prioritizing simplicity and sensible defaults while remaining extensible. Renders directly to `<canvas>`, giving significant performance advantages over SVG by avoiding DOM overhead. Ships with 8 core chart types: Line, Bar, Radar, Doughnut, Pie, Polar Area, Bubble, and Scatter, plus Area (via line fill) and Mixed charts. Additional types (boxplot, violin, candlestick, treemaps) available through community plugins.

Architecture built around a plugin system and tree-shakable ESM modules. Core components: controllers (one per chart type), elements (visual primitives), scales (linear, logarithmic, category, time, radial), and plugins (legend, tooltip, filler, decimation). Built-in TypeScript typings, responsive resizing, animations, data decimation, and interaction modes. Time axes require a date adapter (date-fns, Luxon, Day.js). Framework wrappers for React, Vue, Angular, Svelte, SolidJS.

**Gotchas**:

- **Canvas, not SVG**: No DOM elements per data point — can't style with CSS, limited accessibility/SEO.
- **Date adapter required**: Time-scale axes fail silently without a separate adapter package.
- **Tree-shaking requires explicit registration**: Must import/register components individually or use `chart.js/auto`. Forgetting produces blank charts with no error.
- **Container sizing**: Chart uses container for sizing; no explicit height = 0 height or erratic resizing. Set `maintainAspectRatio: false`.
- **Destroy before re-create**: Must call `.destroy()` on old instance or get memory leaks and "Canvas is already in use" errors.
- **Limited built-in types**: No native treemaps, heatmaps, Gantt, Sankey, or candlestick charts.

**When is it a good fit?**:

- Dashboards/admin panels needing clean, performant charts with minimal config
- Small bundle size needs (tree-shaking gets well under 100KB gzipped)
- Many charts or large datasets where Canvas outpaces SVG
- Standard chart types (line, bar, pie, scatter, radar, bubble, polar area)
- Rapid prototyping and MVPs
- NOT a good fit for: highly custom visualizations (use D3), DOM interactivity/CSS styling needs, or SSR without headless browser

**Maturity & Momentum**:

- **Age**: 13 years (created March 2013)
- **GitHub stars**: ~67,300
- **npm weekly downloads**: ~7.8 million
- **Contributors**: 526
- **Last commit**: February 2026
- **Release cadence**: Patch releases every 1-3 months; minor releases a few times/year

**Notable Users**: Widely used in open-source admin templates (AdminLTE, CoreUI, Tabler), SaaS dashboards, and internal tools. react-chartjs-2 wrapper alone has ~1.5M weekly npm downloads. Referenced in official tutorials by Laravel, Django, WordPress.

**Latest Major Version**: v4.0.0 (October 2022), latest v4.5.1 (October 2025)

- ESM-only package; tree-shaking enabled (`"sideEffects": false`)
- Grid border config restructured into `border` sub-object
- TypeScript source migration for core helpers
- Scale defaults and plugin hooks renamed
- **Old version warning signs**: `scales: { xAxes: [...], yAxes: [...] }` array syntax (v2), `Chart.defaults.global.*` (v2), `require('chart.js')` CommonJS (v2/v3), `grid.drawBorder` instead of `border.display` (v3), `time.stepSize` instead of `ticks.stepSize` (v3)

---

### Highcharts

**Name**: Highcharts

**License**: Proprietary / dual-license. Free for non-commercial use (CC BY-NC). Commercial use requires a paid license from Highsoft AS (perpetual or annual, per-developer seats). Source is viewable on GitHub but NOT open source in the OSI sense.

**URLs**:

- Website: <https://www.highcharts.com/>
- GitHub: <https://github.com/highcharts/highcharts>
- Demos: <https://www.highcharts.com/demo>
- Documentation: <https://www.highcharts.com/docs>
- API Reference: <https://api.highcharts.com/highcharts/>
- npm: <https://www.npmjs.com/package/highcharts>
- License Shop: <https://shop.highcharts.com/>

**Overview**: Highcharts is a configuration-driven JavaScript charting library built on SVG rendering. Ships as a core module plus optional add-ons: Highcharts Stock (financial time-series), Highcharts Maps (geographic), and Highcharts Gantt (project timelines). Provides 30+ built-in chart types, responsive layouts, drilldown navigation, data export (PNG/JPEG/SVG/PDF/CSV), WCAG accessibility, and Node.js server-side rendering. Architecture follows a modular pattern: core SVG renderer + event management, with series types, axes, and features layered as modules. As of v12, internal data layer uses a `DataTable` abstraction and build system moved to Webpack. Official wrappers for React, Angular, Vue.

**Gotchas**:

- **Licensing confusion**: Source on GitHub leads developers to assume it's free. Commercial projects require paid license.
- **Container sizing**: Container must have explicit/resolved dimensions at render time or chart won't render.
- **Dynamic update performance**: Frequent `chart.update()` with large datasets causes jank. Use `series.setData(data, false)` and batch with `chart.redraw()`.
- **XSS with `useHTML`**: Pre-v9.3.2, `useHTML: true` on labels/tooltips passed to `innerHTML` without sanitization.
- **Data mutation**: Highcharts mutates input arrays by default. Set `allowMutatingData: false` for immutability.
- **Bundle size**: ~300KB gzipped with all modules. Tree-shaking only via ESM imports from `highcharts/es-modules/`.
- **Export server**: Exporting module requires Highcharts-hosted server by default; must self-host for air-gapped environments.

**When is it a good fit?**:

- Enterprise dashboards/BI tools where time-to-production matters
- Financial/stock charting (navigator, range selector, OHLC, candlestick, P&F, Renko)
- Configuration-driven charts without writing SVG/Canvas code
- Built-in export to PNG/PDF/CSV without additional tooling
- Accessibility-critical applications (WCAG, screen reader, keyboard nav)
- Geographic data (Maps with GeoJSON/TopoJSON) and project timelines (Gantt)
- Organizations that budget for commercial license and want vendor support

**Maturity & Momentum**:

- **Age**: 17 years (first released 2009)
- **GitHub stars**: 12,400
- **npm weekly downloads**: ~2.17 million
- **Contributors**: 196
- **Last publish**: January 2026 (v12.5.0)
- **Release cadence**: Major versions roughly annually; minor/patch every 1-3 months
- **Company**: Highsoft AS (Norway), dedicated team with commercial support

**Notable Users**: Claims 80 of world's 100 largest companies (~80,000 customers). Confirmed: GitHub, Visa, Microsoft, Facebook, IBM, Apple, Accenture, Roblox, US EPA.

**Latest Major Version**: v12.0.0 (November 2024), latest v12.5.0 (January 2026)

- DataTable architecture replacing parallel arrays
- Locale-aware formatting via `Intl` API (`lang.locale`)
- Human-friendly date inputs (date strings instead of epoch milliseconds)
- New series: Point & Figure, Renko charts
- Webpack-based UMD builds; ESM packages for tree-shaking
- Module auto-registration (removed factory pattern)
- **Old version warning signs**: `SomeModule(Highcharts)` factory calls (pre-v12), `Highcharts._modules` (pre-v12), `{point.x}` for categories instead of `{category}` (pre-v12), epoch-only dates (pre-v12), `require('highcharts/highstock')` factory pattern (pre-v12)

---

### Plotters

**Name**: Plotters

**License**: MIT

**URLs**:

- crates.io: <https://crates.io/crates/plotters>
- GitHub: <https://github.com/plotters-rs/plotters>
- Documentation: <https://docs.rs/plotters/latest/plotters/>
- Examples: <https://github.com/plotters-rs/plotters/tree/master/plotters/examples>
- Homepage: <https://plotters-rs.github.io/home/>

**Overview**: Plotters is a pure Rust drawing library for data plotting targeting both native applications and WebAssembly. Provides a layered architecture with a low-level drawing backend API and a high-level charting API. Key abstractions:

- **Backends**: Split into independent crates — `plotters-bitmap` (PNG/GIF), `plotters-svg` (vector), `plotters-canvas` (HTML5 Canvas/WASM). Third-party backends for GTK/Cairo.
- **DrawingArea**: Layout primitive that can be subdivided hierarchically with custom coordinate systems for multi-panel figures.
- **ChartBuilder/ChartContext**: High-level chart construction with configurable axes, labels, mesh grids, legends.
- **Series**: Line, point, area, surface (3D), candlestick, histogram, box plot, error bars.
- **Coordinates**: Continuous (f32/f64), discrete, date/time (chrono), logarithmic.

Features include animated GIF output, 3D surface plotting, Jupyter/evcxr integration, color palettes/colormaps, and `ab_glyph` font backend as lighter alternative to `font-kit`.

**Gotchas**:

- **Clipping unreliable**: Data outside plot area may render at edges (#429).
- **Font handling complex**: Default `ttf` feature pulls heavy deps. On Linux without fontconfig, font resolution fails silently. Use `ab_glyph` feature for lighter alternative.
- **Backend inconsistencies**: SVG, bitmap, canvas backends render differently (stroke widths, fills, text positioning).
- **Compile times**: Large dep tree with defaults. Use `default-features = false`.
- **API verbosity**: Multi-step builder chain even for simple charts; steeper than matplotlib-like interfaces.
- **No grouped bar charts** out of the box (#211) — requires manual positioning.

**When is it a good fit?**:

- Static PNG/SVG charts in CLI tools, reports, CI pipelines
- Rust-native WASM web apps (same API, swap backend)
- Benchmarking tools (rendering engine behind `criterion`)
- Scientific/data-heavy apps needing programmatic chart generation
- Jupyter/evcxr notebooks for Rust data exploration
- Animated GIF output (algorithm visualizations)
- Pure-Rust solution with no C/C++ deps (using `ab_glyph`)

**Maturity & Momentum**:

- **Age**: ~7 years (created April 2019)
- **crates.io downloads**: ~140.6 million (largely driven by `criterion` dependency)
- **GitHub stars**: 4,539
- **Contributors**: 111
- **Open issues**: 174 (significant backlog)
- **Last commit**: March 2026
- **Release cadence**: Slow and irregular. No 0.4 or 1.0 on horizon. Maintained but not rapidly evolving.

**Notable Users**: criterion/cargo-criterion (dominant Rust benchmarking), halo2 ZK proof libraries (Zcash), native-windows-gui, fundsp (audio DSP). 423 reverse dependencies.

**Latest Major Version**: 0.3.7 (September 2024)

- 0.3.x line split backends into independent crates, added `ab_glyph` font backend, color maps, dotted lines, evcxr figure save
- **Old version warning signs**: Importing from `plotters::drawing::backend` instead of `prelude`, `Category` coordinate type (pre-0.3), `Path` instead of `PathElement`, version `0.2.x` in Cargo.toml

---

### PGFPlots (Rust)

**Name**: PGFPlots (Rust crate)

**License**: MIT

**URLs**:

- crates.io: <https://crates.io/crates/pgfplots>
- GitHub: <https://github.com/DJDuque/pgfplots>
- Documentation: <https://docs.rs/pgfplots>
- Examples: <https://github.com/DJDuque/pgfplots/tree/main/examples>

**Overview**: `pgfplots` is a Rust library that generates LaTeX PGFPlots code for publication-quality figures. Acts as a **code generator** — constructs valid LaTeX/TikZ environments which are compiled to PDF by `pdflatex` or the bundled `tectonic` engine. Architecture mirrors PGFPlots LaTeX hierarchy: `Picture` > `Axis` > `Plot2D` with `Coordinate2D` data. Customization via typed enum keys with `Custom(String)` escape hatch. ~1,045 lines across 8 source files.

Features: two compilation engines (PdfLatex/Tectonic), 2D plot types (sharp, smooth, step, bar, comb, scatter), error bars, axis customization, LaTeX passthrough in strings, `show_pdf()` for compile-and-open workflow, `to_pdf()` for file output.

**Gotchas**:

- **LaTeX required**: Default engine needs `pdflatex` + `pgfplots` package installed. Fails silently if missing. Use `tectonic` feature to avoid.
- **No 3D support**: Only `Plot2D` available.
- **Limited typed API**: Many PGFPlots options require `Custom(String)` raw LaTeX syntax.
- **Bar chart `compat`**: Must manually set `compat=1.7` via `PictureKey::Custom` for axis-unit bar widths.
- **Feature renamed**: `inclusive` → `tectonic` in v0.5.0.
- **No temp file cleanup**: `show_pdf()` leaves .pdf/.log/.aux in temp directory.

**When is it a good fit?**:

- Publication-quality plots for academic papers/theses matching LaTeX document styling
- Rust data pipelines producing figures for LaTeX documents
- Scientists already using PGFPlots who want to script from Rust
- CI/CD with `tectonic` feature (no system LaTeX needed)
- NOT a good fit for: interactive plots, raster output (PNG/SVG), 3D, comprehensive charting without LaTeX knowledge

**Maturity & Momentum**:

- **Age**: ~3.9 years (created April 2022)
- **crates.io downloads**: ~13,500 total
- **GitHub stars**: 124
- **Contributors**: 2 (essentially solo maintainer)
- **Last commit**: June 2023 (~3 years ago)
- **Last release**: v0.5.1 (January 2023)
- **Status**: **Dormant/abandoned** — 8 releases in 8 months then complete silence. 7 open issues.

**Notable Users**: qtruss (finite-element truss solver), diffurch (ODE/DDE solver). Very limited adoption.

**Latest Major Version**: 0.5.1 (January 2023)

- Renamed `inclusive` feature to `tectonic`
- Unified compilation API: `to_pdf()`/`show_pdf()` with `Engine` enum
- Proper error handling via `thiserror`
- **Old version warning signs**: `features = ["inclusive"]` (pre-0.5), `.show()` instead of `.show_pdf(Engine)` (pre-0.5), manual `pdflatex` invocation (pre-0.5)

---

### Nivo

**Name**: Nivo

**License**: MIT

**URLs**:

- Website: <https://nivo.rocks>
- GitHub: <https://github.com/plouc/nivo>
- Components Explorer: <https://nivo.rocks/components/>
- Documentation: <https://nivo.rocks> (each chart type has interactive docs)

**Overview**: Nivo is a comprehensive React data visualization library built on D3.js. Provides 30+ fully declarative, customizable chart components: Bar, Line, Pie, Heatmap, Sankey, Chord, Treemap, Sunburst, Calendar, Waffle, Radar, Bump, Funnel, Swarmplot, Network, Geo/Choropleth, Boxplot, Marimekko, Stream, Parallel Coordinates, Circle Packing, Icicle, Polar Bar, Radial Bar, Tree, Voronoi. Monorepo with scoped `@nivo/*` packages — install only what you need. Supports three rendering targets: **SVG** (interactivity/styling), **Canvas** (large datasets), **HTML** (DOM-based). Features include react-spring animations, theming with gradient/pattern fills, responsive containers, SSR support, HTTP API for server-side image generation, mesh-based hover detection, and custom layers for injecting arbitrary rendering.

**Gotchas**:

- **Tooltip clipping**: Tooltips cut off in modals or overflow-hidden containers (#2648). Use portal-based custom tooltip.
- **Resize glitches**: Since v0.97.0, charts render incorrectly in dynamically resizing containers (#2784). Pin to v0.96.x if needed.
- **Radial label overlap**: Small pie slices cause unreadable labels (#1378). Use `arcLinkLabelsSkipAngle`.
- **SVG performance**: Struggles beyond ~2,000 data points. Switch to Canvas variants (e.g., `ResponsiveBarCanvas`).
- **Time-scale on Bar**: Limited support (#1913); use Line or custom layer.
- **Bundle size**: Multiple `@nivo/*` packages accumulate D3 sub-module weight.

**When is it a good fit?**:

- React dashboards/admin panels needing variety of chart types with consistent API
- Projects needing both SVG and Canvas rendering of same chart type
- Declarative, prop-driven configuration preferred over imperative D3
- Server-side chart rendering (static images via HTTP API / `@nivo/static`)
- Rapid prototyping via interactive playground at nivo.rocks
- Moderate data volumes (hundreds to low thousands of points per chart)

**Maturity & Momentum**:

- **Age**: 10 years (created April 2016)
- **GitHub stars**: ~14,000
- **npm weekly downloads**: ~1.6M (`@nivo/core`), ~666K (`@nivo/bar`)
- **Contributors**: 206
- **Last commit**: February 2026
- **Release cadence**: Very active — 15 releases in April-May 2025 (v0.89-v0.99), signaling approaching 1.0

**Notable Users**: BBC (mozaik dashboard project). Over 2,000 public GitHub repos depend on it. Common in enterprise React dashboards, analytics platforms, SaaS admin panels.

**Latest Major Version**: v0.99.0 (May 2025) — never released 1.0 yet

- Native ESM support, React 19 compatibility
- New chart types: PolarBar, Icicle, Tree
- Full TypeScript migration (prop-types removed)
- New event handlers: onMouseDown, onMouseUp, onDoubleClick
- **Old version warning signs**: PropTypes imports (pre-v0.87), `recompose` HOCs (pre-v0.87), CommonJS `require()` (pre-v0.89), missing PolarBar/Icicle/Tree components (pre-v0.87-0.89), versions below 0.80

---

### amCharts

**Name**: amCharts

**License**: Proprietary "linkware" dual-license. Free to use with visible branding watermark. Commercial licenses remove branding: Basic ($80/yr/seat), SaaS ($280/yr/seat), OEM ($1,200/yr/seat). Source on GitHub but NOT OSI open-source.

**URLs**:

- Website: <https://www.amcharts.com/>
- GitHub (v5): <https://github.com/amcharts/amcharts5>
- Examples: <https://www.amcharts.com/demos/>
- Documentation: <https://www.amcharts.com/docs/v5/>
- npm: <https://www.npmjs.com/package/@amcharts/amcharts5>

**Overview**: amCharts is a commercial JavaScript/TypeScript charting library with 60+ chart types across 16 categories: XY (line, area, column, bar, candlestick), pie/donut, radar/gauge, maps (choropleth, globe), hierarchy (treemap, sunburst, force-directed), flow (Sankey, chord), stock charts with technical indicators, Gantt, timelines, funnels, word clouds, Venn diagrams. Version 5 built on Canvas with GPU acceleration (~400KB tree-shakeable core). Architecture uses a "root element" pattern: `am5.Root` bound to DOM container, with chart objects, series, axes attached to it. Multiple charts can share a single root/canvas. TypeScript-native with full type definitions. Supports theming, animations, automatic data grouping for large datasets, WCAG AA accessibility, i18n (47 locales, RTL), and export to images/PDF/data. Framework integrations for React, Angular, Vue, Next.js, Nuxt 3, SvelteKit, Remix, Ember.

**Gotchas**:

- **Canvas = no DOM access**: Can't target elements via CSS. Styling only through amCharts API/themes.
- **Flexbox sizing issues**: `getBoundingClientRect()` accounts for transforms, causing resize problems. Use fixed dimensions or unwrapped containers.
- **Stacked charts with missing data**: Gaps cause subsequent stacked series not to render. Fill with `null`/`0`.
- **No SVG export**: Canvas rendering prevents SVG output. PNG and PDF supported.
- **No 3D charts**: Removed in v5 (existed in v4).
- **Watermark on free license**: Visible branding link on every chart unless commercially licensed.
- **Memory management**: Must call `root.dispose()` on teardown to avoid leaks.

**When is it a good fit?**:

- Dashboards needing 60+ chart types from a single library
- Financial apps with stock charts and technical indicators (MACD, RSI, Bollinger)
- Data-heavy apps — Canvas + auto data grouping handles hundreds of thousands of points
- Geographic/map visualizations alongside traditional charts
- Enterprise projects justifying commercial license with dedicated vendor support
- Strong accessibility (WCAG AA) and internationalization requirements
- TypeScript projects valuing strong typing for chart config

**Maturity & Momentum**:

- **Age**: Founded 2004 (originally Flash-based). JS v5 launched August 2021.
- **GitHub stars**: ~424 (v5), ~1,161 (v4) — modest due to commercial/linkware model
- **npm weekly downloads**: ~228K (v5) + ~81K (v4) = ~309K combined
- **Contributors**: Small team (~2 visible on GitHub). Company based in Vilnius, Lithuania.
- **Last commit**: March 2026
- **Release cadence**: Very active — 2-3 releases/month, 37 releases in 2024

**Notable Users**: Microsoft, PayPal, eBay, Samsung, Symantec, Walmart, Morgan Stanley, NEC, US FDA. 22,000+ companies reportedly. Strongest in Financial Services.

**Latest Major Version**: amCharts 5 (v5.16.2, March 2026; initial v5: August 2021)

- Canvas rendering with GPU acceleration (was SVG in v4)
- Root element architecture (charts share one root/canvas)
- TypeScript-native rewrite with tree-shakeable core
- Removed 3D charts, SVG export, IE support
- New incompatible theme system
- **Old version warning signs**: `@amcharts/amcharts4` imports, `am4core`/`am4charts` prefixes (v5 uses `am5`/`am5xy`), `am4core.create()` (v5 uses `am5.Root.new()`), constructor `new LineSeries()` (v5 uses `.new()` factory), CSS styling of elements, mentions of SVG export or 3D charts

---

### ApexCharts

**Name**: ApexCharts

**License**: Dual-license. **Community License** free for organizations under $2M USD annual revenue. **Commercial License** required for $2M+. **OEM License** for redistribution in products. Not MIT — installing via npm constitutes license acceptance.

**URLs**:

- Website: <https://apexcharts.com/>
- GitHub: <https://github.com/apexcharts/apexcharts.js>
- Examples: <https://apexcharts.com/javascript-chart-demos/>
- Documentation: <https://apexcharts.com/docs/installation/>

**Overview**: ApexCharts is an SVG-based interactive charting library using SVG.js as its drawing engine. Supports 19+ chart types: line, area, column, bar, combo, range area, timeline, funnel, candlestick, boxplot, bubble, scatter, heatmap, treemap, slope, pie, donut, radial bar, radar, polar area. Interactive out of the box with zoom, pan, tooltips, selection, and toolbar for export (SVG/PNG/CSV). Configuration-driven: single options object describes everything. As of v5.7+, supports modular tree-shaking via per-chart-type entry points (`apexcharts/line`, `apexcharts/bar`) and per-feature entry points. SSR support via `apexcharts/ssr` for Next.js, Nuxt, SvelteKit, Astro. Official wrappers for React, Vue, Angular. Companion ecosystem: ApexGrid.js, ApexGantt.js, ApexStock.js, ApexTree.js, ApexSankey.js.

**Gotchas**:

- **Bundle size**: ~462KB minified full bundle. Use tree-shakeable modular imports (v5.7+).
- **Null values slow rendering**: Sparse data with many `null` values renders slowly (#3249).
- **Dynamic UI breakage**: Charts in tabs/collapsible sidebars fail to resize (#2137). Call `chart.updateOptions({})` after visibility changes.
- **Export incomplete**: SVG/PNG export omits annotations, images, dark theme styling. Use html2canvas for full fidelity.
- **Vite deduplication**: Must add `apexcharts` to `optimizeDeps.include` with tree-shaken builds.
- **Log scale edge cases**: Known rendering issues with certain data ranges.

**When is it a good fit?**:

- Dashboards/admin panels needing rich interactivity (zoom, pan, selection, toolbar)
- Projects needing 19+ chart types under one API
- Rapid prototyping with declarative config and 100+ copy-paste demos
- Mixed/combo charts (line + bar + area on one chart is first-class)
- Time-series with native datetime axes, auto tick formatting, LTTB downsampling
- SSR/static sites with modern meta-frameworks (v5.5+)
- Small organizations (free under $2M revenue)

**Maturity & Momentum**:

- **Age**: ~8 years (first published July 2018)
- **GitHub stars**: ~15,100
- **npm weekly downloads**: ~1.71 million
- **Contributors**: 218
- **Last commit**: March 2026
- **Release cadence**: Very active — 18 releases in Feb-Mar 2026. Major features every 2-4 weeks.

**Notable Users**: Default charting in Vuetify admin dashboards, CoreUI, Vuexy, Metronic. Broad adoption across enterprise and SaaS.

**Latest Major Version**: v5 (initial v5.3.0, July 2025 — jumped from v4.7.0; latest v5.10.4, March 2026)

- Direct data parsing without manual transformation
- Unified series format (Pie/Donut now support XY format)
- SSR with `renderToHTML()`/`renderToString()` + client-side `hydrate()`
- Accessibility: ARIA labels, keyboard navigation, color-blind modes
- Tree-shaking via modular entry points
- CSS variable colors for runtime theme switching
- Full JSDoc `strict: true` TypeScript checking
- **Old version warning signs**: Pie series as flat arrays (`series: [44, 55]`), no SSR/hydration, `window.ApexCharts` global from `<script>` tag, no accessibility options, manual data transformation, references to `apexcharts/dist/apexcharts.min.js` CDN path

---

### gnuplot

**Name**: gnuplot

**License**: gnuplot license (custom permissive, not GPL despite the name; allows redistribution with attribution; FSF considers it "free but GPL-incompatible")

**URLs**:

- Homepage: <http://www.gnuplot.info/>
- Source: <https://sourceforge.net/projects/gnuplot/>
- Documentation: <http://www.gnuplot.info/documentation.html>

**Overview**: gnuplot is a venerable command-line graphing utility first released in 1986. Produces 2D and 3D plots of functions and data in dozens of output formats (PNG, SVG, PDF, PostScript, LaTeX/TikZ, interactive terminals). Reads data from files or stdin, supports scripting via its own language, driven from shell pipelines. Handles scatter plots, line graphs, histograms, heatmaps, contour plots, surface plots, polar plots, and more. Not part of the GNU project despite the name.

**Gotchas**:

- Idiosyncratic scripting language with decades of accumulated syntax — steep learning curve beyond basics.
- Default styling is utilitarian; publication-quality output requires significant customization.
- Custom license is not OSI-approved and GPL-incompatible.
- No native JSON or CSV-with-headers parsing — preprocess or use column indices.
- Version differences across distros can be significant (Ubuntu LTS ships older versions).
- Error messages can be cryptic.

**When is it a good fit?**:

- Quick exploratory plots from CLI or shell scripts
- Automated report generation pipelines needing static image output without GUI
- Established scientific/engineering workflows
- Environments where installing Python/R/Ruby is undesirable — single binary
- 3D surface/contour plots from CLI — few alternatives match its 3D capabilities

**Maturity & Momentum**:

- **Age**: 38+ years (first released 1986)
- **Status**: Extremely mature. Slow but steady development. Packaged in every major distro, Homebrew, MSYS2.
- **Latest release**: v6.0 (January 2024)

**Notable Users**: Linux kernel `perf` tooling, Octave (default plotting backend), LaTeX users via TikZ, extensive academia/engineering use.

**Latest Major Version**: 6.0 (January 2024)

- Named colormap arrays, revised hidden-surface-removal for 3D, better Unicode/UTF-8 in text terminals

---

### vl-convert

**Name**: vl-convert

**License**: BSD 3-Clause

**URLs**:

- GitHub: <https://github.com/vega/vl-convert>
- crates.io: <https://crates.io/crates/vl-convert>
- PyPI: <https://pypi.org/project/vl-convert-python/>
- npm: <https://www.npmjs.com/package/vl-convert>

**Overview**: Rust-based CLI and library for converting Vega and Vega-Lite visualization specs (JSON) into static images (PNG, SVG) and PDF. Embeds the Deno v8 JavaScript runtime to execute Vega-Lite compiler and scenegraph renderer entirely offline — no browser or Node.js required. Also provides Python and Node.js bindings. Supports multiple Vega-Lite versions simultaneously (v4, v5).

**Gotchas**:

- Binary is large (~100+ MB) due to embedded v8 engine and bundled compilers.
- First invocation has ~1-2 second latency from v8 initialization; subsequent library API calls are fast.
- Only renders Vega/Vega-Lite specs — cannot pass raw CSV directly.
- Font rendering depends on system fonts; missing fonts produce fallback glyphs.
- Cross-compilation non-trivial due to Deno/v8 embedding.

**When is it a good fit?**:

- CI/CD pipelines converting Vega-Lite specs to images without a browser
- Rust projects embedding chart rendering via library crate
- Python data science (via `vl-convert-python`) replacing `altair_saver` + Chrome/Selenium
- Headless environments where installing Chromium is impractical
- Teams already using Vega-Lite specs (Altair, Observable, Deneb in Power BI)

**Maturity & Momentum**:

- **Age**: ~4 years (first release 2022)
- **GitHub stars**: ~700+
- **Maintainer**: Jon Mease (core Vega contributor), under official Vega GitHub org
- **Release cadence**: Multiple releases per month tracking upstream Vega-Lite

**Notable Users**: Altair (recommended static export engine), Quarto (Posit's publishing system).

**Latest Major Version**: v1.8.x (v1.0 landed mid-2024). Recent: Vega-Lite v5.21+ support, improved font handling, locale support, PDF text embedding.

---

### YouPlot

**Name**: YouPlot (command: `uplot`)

**License**: MIT

**URLs**:

- GitHub: <https://github.com/red-data-tools/YouPlot>
- RubyGems: <https://rubygems.org/gems/youplot>

**Overview**: Ruby CLI tool that renders charts directly in the terminal using Unicode braille characters and ANSI colors. Reads delimited data from stdin and produces bar charts, histograms, line plots, scatter plots, density plots, box plots, and count-based charts. Designed for pipeline visualization — pipe output into `uplot` for instant terminal charts. Wraps the `unicode_plot` Ruby gem.

**Gotchas**:

- Requires Ruby runtime (>= 2.5) — non-trivial dependency for minimal environments.
- Resolution limited by terminal dimensions and braille granularity — not for publication.
- Limited chart types — no 3D, pie, annotations, multi-axis.
- Simple delimiter-based parsing; no quoted CSV, JSON, or complex format support.
- No image file export (PNG/SVG) — terminal text output only.
- Low commit activity since 2022 — essentially feature-complete.

**When is it a good fit?**:

- Quick ad-hoc terminal data exploration without leaving CLI
- Shell pipelines for visual inspection of distributions/trends/counts
- SSH sessions without GUI/browser access
- When you need a histogram of piped data in under 5 seconds

**Maturity & Momentum**:

- **Age**: ~6 years (initial release ~2020)
- **GitHub stars**: ~4,000+
- **Status**: Stable, low-activity. Feature-complete for its scope.

**Notable Users**: Popular in DevOps/SRE community. Frequently in "awesome CLI tools" lists. red-data-tools organization (Ruby data science tooling).

**Latest Major Version**: v0.4.6 (2023). Has not reached 1.0 but API is stable. Commands: `uplot bar`, `hist`, `line`, `scatter`, `density`, `box`, `count`.

---

### SciChart.js

**Name**: SciChart.js

**License**: Commercial proprietary. Free "Community Edition" for non-commercial/startup use (revenue under $200K). Standard licenses ~$2,000-$3,000 per developer seat (perpetual + maintenance). Enterprise/OEM priced higher. Trial available.

**URLs**:

- Homepage: <https://www.scichart.com/>
- Documentation: <https://www.scichart.com/documentation/js/current/>
- GitHub (examples): <https://github.com/ABTSoftware/SciChart.JS.Examples>
- npm: `scichart`

**Overview**: High-performance 2D and 3D charting library rendering via WebGL with a WASM engine for data processing. Originated as a WPF/.NET library (~2012), expanded to JS/TS. Targets real-time, data-intensive applications — financial trading, scientific/medical telemetry, IoT sensors. 30+ series types including line, scatter, band, mountain, column, candlestick, heatmap, contour, bubble, pie/donut, 3D surface/scatter/waterfall. Features real-time streaming APIs, auto range adjustment, built-in zoom/pan/crosshairs/rollover, multi-pane synchronized charts, internal data resampling (down-samples millions of points to pixel resolution), Builder API (declarative JSON), and ~1.5-2 MB WASM binary. Official React, Angular, Vue wrappers.

**Gotchas**:

- **WASM cold-start**: ~200-500ms initialization latency. Must call `SciChartSurface.loadWasmFromCDN()` early.
- **Bundle size**: ~2-4 MB including monolithic WASM binary.
- **Canvas-only**: WebGL rendering means no CSS styling of chart elements.
- **License enforcement**: Watermark without valid license key set in code.
- **Learning curve**: Extensive API with concepts like RenderableSeries, DataSeries, ChartModifiers.
- **No native SSR**: Requires browser/WebGL context.

**When is it a good fit?**:

- Real-time financial trading platforms (tick-by-tick streaming, depth charts)
- Scientific/medical visualization (ECG/EEG, spectrograms, multi-channel sensors)
- IoT dashboards with millions of data points
- 1M+ data points at 60fps in a browser
- Enterprise apps needing commercial support/SLA
- Cross-platform teams using SciChart on .NET/WPF/iOS/Android

**Maturity & Momentum**:

- **Age**: Company since 2012; JS edition ~2020
- **Release cadence**: Roughly quarterly with significant feature additions
- **Status**: Actively maintained, responsive support forum, comprehensive docs

**Notable Users**: Bosch, Siemens, ABB, Caterpillar, various fintech/trading firms, medical device companies, defense/aerospace.

**Latest Major Version**: v4.x (2024-2025) — subcharts, WASM performance improvements, enhanced 3D, expanded Builder API.

---

### LightningChart JS

**Name**: LightningChart JS

**License**: Commercial proprietary. All production use requires paid license. Per-developer-seat, tiered by edition: XY only (~$1,495), XY+3D, full suite (~$5,995+). OEM separately negotiated. No free tier for production. Time-limited trial with watermark.

**URLs**:

- Homepage: <https://lightningchart.com/>
- JS product page: <https://lightningchart.com/js-charts/>
- Documentation: <https://lightningchart.com/js-charts/docs/>
- Interactive examples: <https://lightningchart.com/js-charts/interactive-examples/>
- npm: `@arction/lcjs`

**Overview**: GPU-accelerated charting from Arction Ltd. (Finland). Renders via WebGL, purpose-built for extreme data throughput — markets itself as "fastest charting library" with benchmarks showing 1 billion data points. Originated as .NET/WPF component (early 2010s). Features LOD and GPU-side data processing, real-time append at millions of points/second. Chart types: XY (line, scatter, area, spline, OHLC, candlestick, box, heatmap), 3D (surface, point cloud, box, line), polar, pie/donut/funnel, maps (GeoJSON), spider/radar. Dashboard layouts share single WebGL context. Works with React, Angular, Vue, plain JS/TS.

**Gotchas**:

- **Verbose API**: Builder/fluent pattern chains are powerful but unfamiliar. Simple line chart requires multiple chained calls with nested style objects.
- **Bundle size**: ~3-5 MB, no tree-shaking of chart types.
- **License tier gating**: Cheapest license only covers XY charts; 3D/polar/map/pie require higher tiers.
- **No SSR**: Requires browser with WebGL.
- **Limited community**: Commercial niche means fewer SO answers, examples, and third-party tutorials.
- **Canvas-only, no CSS styling**.
- **Prominent trial watermark**, no free tier at all.

**When is it a good fit?**:

- Genuinely need hundreds of millions to billions of data points (seismic, genomics, HFT at extreme scale)
- Real-time monitoring: manufacturing, energy, telecom with extremely high ingestion rates
- Scientific spectrograms/heatmaps, signal processing, physics simulations
- 3D surface plots + point clouds alongside 2D in a unified library
- Enterprise budget available, raw throughput is primary criterion over API ergonomics

**Maturity & Momentum**:

- **Age**: Company since 2009; JS edition ~2019-2020
- **Release cadence**: Monthly patches, annual major versions
- **npm downloads**: Low thousands weekly (commercial niche)

**Notable Users**: BMW Group (marketing reference), energy/SCADA companies, telecom firms, research institutions, defense contractors. References industries more than specific names.

**Latest Major Version**: v6.x (2024-2025) — performance improvements, new chart types, enhanced 3D, API refinements. Migration guides for breaking changes.

---

### visx

**Name**: visx (formerly vx)

**License**: MIT

**URLs**:

- Website: <https://airbnb.io/visx>
- GitHub: <https://github.com/airbnb/visx>
- Documentation: <https://visx.airbnb.tech/docs>
- Gallery: <https://airbnb.io/visx/gallery>

**Overview**: visx is a collection of 33+ low-level, composable React visualization primitives built by Airbnb. Uses D3 only for math (scales, curves, projections, layouts) while React owns all DOM rendering — no `d3.select()` mutations fighting React's virtual DOM. Monorepo of independently installable `@visx/*` packages: shapes, scales, axes, grid, tooltip, brush, drag, zoom, voronoi, geo, hierarchy, network, sankey, heatmap, wordcloud, gradient, pattern, annotation, legend, text, and more. `@visx/xychart` provides a higher-level batteries-included chart component. Pure SVG rendering via React JSX. 97.8% TypeScript. No built-in animations — bring your own (react-spring via `@visx/react-spring`, framer-motion, CSS transitions).

**Gotchas**:

- **Verbosity**: Building a chart requires assembling many packages and writing significantly more code than Recharts/Nivo. `@visx/xychart` mitigates this somewhat.
- **Next.js/SSR**: v3.0 moved D3 deps to ESM-only, breaking CommonJS `require()`. Workaround: `transpilePackages` or dynamic imports with `ssr: false`.
- **No built-in animations**: Must integrate react-spring/framer-motion yourself.
- **SVG-only**: No Canvas fallback — performance degrades with thousands of individual SVG nodes. Implement downsampling for large datasets.
- **Documentation gaps**: Changelog uses PR titles; breaking changes not always explained inline.

**When is it a good fit?**:

- Custom, bespoke visualizations that don't fit standard chart types
- Design-system integration needing full control over every visual element
- React-first teams wanting visualization as normal React components
- Incremental adoption — install only the 2-3 packages you need
- Teams with D3/SVG familiarity wanting React rendering without D3-React impedance mismatch
- NOT good for: quick charts with minimal code, Canvas/WebGL needs, teams lacking D3 familiarity

**Maturity & Momentum**:

- **Age**: ~9 years (vx ~2017, visx September 2020)
- **GitHub stars**: 20,700+
- **npm weekly downloads**: ~483K (`@visx/shape`); ecosystem growing
- **Used by**: 2,900+ dependent repos
- **Last release**: v3.12.0 (November 2024)
- **Status**: Mature, widely-adopted but in maintenance-oriented phase. Commit activity slowed since late 2024.

**Notable Users**: Airbnb (2.5+ years internal use before public 1.0), WHO COVID-19 Dashboard (built on vx predecessor), Figma (analytics).

**Latest Major Version**: v3.0.0 (January 2023), latest v3.12.0 (December 2024)

- All D3 deps upgraded to ESM-only
- Added `@visx/sankey`, XYChart tooltip performance, React 19 support
- TypeScript type improvements throughout
- **Old version warning signs**: `vx` package namespace (deprecated), CommonJS D3 deps (v2.x), missing `@visx/xychart` improvements, no React 18/19 compatibility

---

### Frappe Charts

**Name**: Frappe Charts

**License**: MIT

**URLs**:

- Website: <https://frappe.io/charts>
- GitHub: <https://github.com/frappe/charts>
- Documentation: <https://frappe.io/charts/docs>
- npm: <https://www.npmjs.com/package/frappe-charts>

**Overview**: Lightweight, zero-dependency JavaScript charting library rendering pure SVG. Created by the Frappe team (behind ERPNext). Built with vanilla JS, outputs ESM/UMD/CJS via Rollup. Supported chart types: Bar, Line, Area/Trends, Mixed Axis (bar+line), Pie, Percentage (stacked horizontal bar), Heatmap (GitHub-style contribution calendar), and Scatter. Features responsive sizing, smooth animations, dynamic data updates, tooltips, navigation/region selection, SVG export, and custom color palettes. Framework wrappers for React, Vue, Svelte.

**Gotchas**:

- **Hidden container = zero render**: Chart renders with zero dimensions if container is hidden on construct. Call `chart.draw()` after visibility.
- **SVG rect attribute errors**: Data with null/undefined/NaN values produces console errors (#295). Sanitize data first.
- **No TypeScript types**: Core ships no `.d.ts`. Community types via `@types/frappe-charts`.
- **Limited customization**: Can't hide axis lines/labels or toggle legends easily (#195).
- **CSS import required**: Must explicitly import `frappe-charts/dist/frappe-charts.min.css`.
- **v2 RC never stabilized**: 27 pre-release RCs on npm since ~2020 — avoid in production.

**When is it a good fit?**:

- Simple, clean charts (bar, line, pie, heatmap) without heavy dependencies
- Zero dependencies is a hard requirement
- GitHub-style contribution heatmap with minimal effort
- Frappe/ERPNext ecosystem projects
- NOT good for: advanced chart types, extensive customization, TypeScript-first, active maintenance needs, SSR, accessibility

**Maturity & Momentum**:

- **Age**: ~8.5 years (October 2017)
- **GitHub stars**: ~15,100
- **npm weekly downloads**: ~33,600
- **Contributors**: 46
- **Last stable release**: v1.6.3 (April 2022 — nearly 4 years ago)
- **Status**: Low / quasi-dormant. Downloads healthy (Frappe/ERPNext ecosystem) but no stable releases since 2022.

**Notable Users**: Frappe/ERPNext (primary consumer), Frappe Insights (BI tool). Adoption beyond Frappe ecosystem is small-to-medium projects valuing simplicity.

**Latest Major Version**: v1.6.3 (April 2022)

- Bug fixes for heatmap rendering and tooltip positioning
- v2.0.0-rc27 exists but never stabilized
- **Old version warning signs**: CDN refs below v1.5, v2 RC dependencies in production, old ESM import paths

---

### Recharts

**Name**: Recharts

**License**: MIT

**URLs**:

- Website: <https://recharts.org>
- GitHub: <https://github.com/recharts/recharts>
- Documentation: <https://recharts.org/en-US/api>
- Examples: <https://recharts.org/en-US/examples>

**Overview**: Composable, declarative charting library built on React and D3. Charts assembled by nesting React components (`<LineChart>`, `<XAxis>`, `<Tooltip>`, `<Line>`, etc.) rendering native SVG. v3 rewrote internals to use Redux Toolkit for chart state. Uses D3 indirectly via `victory-vendor` (scales, shapes, interpolation). TypeScript-first with bundled type definitions. Chart types: LineChart, AreaChart, BarChart, ComposedChart (mixed), ScatterChart, PieChart, RadarChart, RadialBarChart, Treemap, Funnel, Sankey. Supporting: CartesianGrid, ReferenceLine/Area/Dot, Brush, Legend, Tooltip, ResponsiveContainer, Labels.

**Gotchas**:

- **Redux state conflicts**: Many charts on one page can produce state collisions with broken legends/tooltips.
- **SSR/Next.js**: `ResponsiveContainer` measures DOM, causing hydration mismatches. Use `next/dynamic` with `ssr: false`.
- **Pie label overlap**: Long-standing #490, no built-in collision avoidance.
- **No zoom/pan**: #710 open since 2016. Brush provides range selection but no true zoom-pan.
- **Bundle size**: Redux + immer + reselect + victory-vendor makes baseline heavier than minimal libs.
- **Animation jank**: Disable `isAnimationActive` for datasets beyond a few hundred points.
- **`connectNulls` v3 change**: Area chart null points now treated as 0 instead of skipped.

**When is it a good fit?**:

- React apps wanting charts that feel like native React components
- Standard dashboard charts (line, bar, area, pie, composed) with tooltips/legends
- JSX-based declarative API preferred over config objects
- Small-to-moderate datasets (hundreds to low thousands of points)
- v3 has `accessibilityLayer` on by default with keyboard navigation
- NOT for: 10K+ data points, geographic/3D/scientific charts, zoom/pan, non-React frameworks

**Maturity & Momentum**:

- **Age**: ~10.5 years (August 2015)
- **GitHub stars**: ~26,900
- **npm weekly downloads**: ~21.6 million
- **Contributors**: ~357
- **Last commit**: March 2026 (active daily)
- **Release cadence**: Roughly monthly (8 releases since v3.0.0)

**Notable Users**: Default charting in shadcn/ui, Tremor, React Admin. 835,000+ dependent repos.

**Latest Major Version**: v3.0.0 (June 2025), latest v3.8.0 (March 2026)

- Complete internal rewrite to Redux Toolkit
- 3,500+ new unit tests
- `accessibilityLayer` enabled by default (keyboard navigation)
- Custom React components in chart tree without `<Customized>` wrapper
- Tooltip/Legend `portal` prop, YAxis `width="auto"`, `symlog` scale
- Dropped ES5, recharts-scale, react-smooth
- **Old version warning signs**: `CategoricalChartState` access, `alwaysShow`/`isFront`/`blendStroke` props, separate `recharts-scale`/`react-smooth` imports, `activeIndex` prop

---

### uPlot

**Name**: uPlot

**License**: MIT

**URLs**:

- GitHub: <https://github.com/leeoniya/uPlot>
- npm: <https://www.npmjs.com/package/uplot>
- Demos: <https://github.com/leeoniya/uPlot/tree/master/demos> (78 runnable HTML demos)

**Overview**: Small, fast Canvas 2D-based charting library focused on time series, lines, areas, OHLC, and bars. Key design decisions: Canvas 2D rendering only (no SVG/WebGL/WASM), columnar data format (array of arrays, not row objects), zero data processing (plots what you give it), and plugin/hooks architecture for extensibility. Performance: 166K points initial render in 34ms, streaming 3,600 points at 60fps uses 10% CPU / 12.3MB RAM. Bundle: ~21.9 KB gzipped. Ships ESM, CJS, IIFE builds with TypeScript definitions.

**Gotchas**:

- **No stacked series**: Author philosophically opposes stacking. Pre-compute cumulative sums yourself.
- **No animations/transitions**: Intentionally omitted ("pure distractions").
- **No axis label collision avoidance**: Handle overlapping ticks manually.
- **No built-in pan**: Zoom is built in; panning requires community plugin.
- **Scatter plot limited**: Not a first-class chart type (#107).
- **Solo maintainer**: Leon Sorokin has 1,412 of ~1,435 commits. Bus factor of 1.
- **Data must be sorted**: X-axis must be monotonically increasing; unsorted data garbles silently.

**When is it a good fit?**:

- High-frequency time series dashboards (monitoring, observability, IoT, trading)
- Tens of thousands to millions of data points where other libs choke
- Streaming/live data at 60fps with low CPU
- Bundle size matters (many chart instances)
- Projects with their own data processing pipeline
- NOT for: rich interactive charting with animations, stacked/polar/radar/treemap charts, declarative APIs, scatter-heavy use cases

**Maturity & Momentum**:

- **Age**: ~6.5 years (September 2019)
- **GitHub stars**: ~10,000
- **npm weekly downloads**: ~765K (2x year-over-year growth)
- **Contributors**: ~30 (overwhelmingly solo author)
- **Last commit**: February 2026
- **Release cadence**: Irregular, 1-4 patches/year. API stable for 4+ years within 1.x.

**Notable Users**: **Grafana** (replaced Flot as core time series renderer — primary download driver), Prometheus community, Immich, SigNoz, Matter Labs (zkSync).

**Latest Major Version**: 1.6.32 (March 2025) — never had a 2.0

- 1.6.x line since mid-2021 with 32 patches, no breaking changes
- **Old version warning signs**: `scales: { x: { time: true } }` without `auto` property, missing `paths` plugin references (pre-1.6)

---

### Observable Plot

**Name**: Observable Plot

**License**: ISC (permissive, functionally equivalent to MIT)

**URLs**:

- Website: <https://observablehq.com/plot/>
- GitHub: <https://github.com/observablehq/plot>
- Documentation: <https://observablehq.com/plot/getting-started>
- Gallery: <https://observablehq.com/@observablehq/plot-gallery>
- npm: <https://www.npmjs.com/package/@observablehq/plot>

**Overview**: Free, open-source library for exploratory data visualization by Mike Bostock (D3 creator) and Philippe Riviere. Implements a layered grammar of graphics with four core abstractions: **Marks** (35+ types: Area, Arrow, Bar, Box, Contour, Density, Dot, Geo, Line, Raster, Text, Tip, Waffle, etc.), **Scales** (linear, log, ordinal, temporal, diverging, etc.), **Transforms** (Bin, Group, Stack, Normalize, Hexbin, Window, etc.), and **Facets** (small multiples via `fx`/`fy`). Call `Plot.plot({ marks: [...] })` to get a detached SVG element — functional, side-effect-free design works in any framework. Features: concise declarative API (~5 lines for a bar chart), SVG output, SSR via JSDOM/linkedom, TypeScript via JSDoc, responsive width, built-in legends, Apache Arrow support, GeoJSON across all marks.

**Gotchas**:

- **Manual DOM insertion**: Returns detached element; React needs `useRef`/`useEffect` pattern.
- **No dual-axis (y2)**: Deliberate omission (considered misleading). Use normalization or facets.
- **Margins don't auto-adjust**: Long tick labels get clipped; manually increase margins.
- **SVG-only**: Tens of thousands of elements become slow. Use raster mark or pre-aggregate.
- **No animations/transitions**: Re-render entirely for each update.
- **Re-rendering replaces SVG**: Stateless; handle cleanup in frameworks to avoid DOM leaks.

**When is it a good fit?**:

- Exploratory data analysis and rapid prototyping
- Static/lightly interactive dashboards (tooltips, highlights)
- Teams familiar with D3 wanting higher-level abstractions
- Data journalism and report generation
- Server-side chart rendering (SVG in Node.js for emails/PDFs)
- Observable Framework/notebook users
- NOT for: highly interactive/animated viz, massive Canvas/WebGL datasets, dual y-axes

**Maturity & Momentum**:

- **Age**: ~5 years (May 2021)
- **GitHub stars**: ~5,200
- **npm weekly downloads**: ~150K
- **Contributors**: 28
- **Last commit**: March 2026 (active)
- **Key maintainers**: Mike Bostock, Philippe Riviere (Observable)
- **Status**: Actively maintained, pre-1.0 but cautious API approach (0.6.x since Sep 2022)

**Notable Users**: The Washington Post, The Marshall Project, Stitch Fix, Sumitovant Biopharma, 500K+ Observable platform users. Microsoft Teams integration partnership.

**Latest Major Version**: v0.6.17 (February 2025) — never reached 1.0

- Key 0.6.x additions: Tip mark (tooltips), Geo mark + projections, Waffle mark, Difference mark, Bollinger mark, Apache Arrow support, GeoJSON shorthand, SSR document option
- **Old version warning signs**: No tip marks (pre-0.6.7), no geo mark (pre-0.6.1), versions below 0.5 missing density/Delaunay/Voronoi marks

---

### AntV G2

**Name**: AntV G2

**License**: MIT

**URLs**:

- Website: <https://g2.antv.antgroup.com>
- GitHub: <https://github.com/antvis/G2>
- Documentation: <https://g2.antv.antgroup.com/en/manual/introduction/what-is-g2>
- Examples: <https://g2.antv.antgroup.com/en/examples>
- npm: <https://www.npmjs.com/package/@antv/g2>

**Overview**: Declarative visualization grammar library by AntV (Ant Group's data visualization team), implementing Wilkinson's *Grammar of Graphics*. Built on `@antv/g`, an abstract rendering engine supporting Canvas (default), SVG, and WebGL backends. Data flows through a pipeline: raw data → transforms (bin, group, stack, sort, filter) → scales (linear, log, ordinal, time) → coordinate systems (Cartesian, polar, theta, parallel) → marks → layout → render. Marks include interval, line, point, area, cell, rect, polygon, boxplot, density, heatmap, and more. Features: dual API (functional chainable + declarative spec), SSR via node-canvas/JSDOM, theme system, React wrapper (`@ant-design/charts`), 3D extensions, declarative animation syntax, multi-view compositions (facets, repeat). Published as peer-reviewed framework in *Visual Informatics* (2026).

**Gotchas**:

- **Documentation primarily Chinese**: English docs exist but incomplete and laggy.
- **V4→V5 is a full rewrite**: API changed dramatically; V4 code won't work in V5.
- **Dynamic data instability**: Stacked bars miscalculate heights, slider interactions cause incorrect rendering.
- **Interaction conflicts**: `elementSelect` and `elementHighlight` backgrounds conflict (#6052).
- **Bundle size**: Large dependency tree (`@antv/g`, `@antv/g-canvas`, `@antv/coord`, `@antv/scale`, etc.).
- **No RTL support** (#3938).

**When is it a good fit?**:

- Enterprise dashboards/BI in Chinese tech ecosystems using Ant Design
- Grammar-of-graphics composition (custom chart types beyond fixed catalog)
- Multiple rendering targets (Canvas/SVG/WebGL) from same spec
- React + Ant Design teams via `@ant-design/charts`
- Server-side chart generation (emails, PDFs, automated dashboards)
- Statistical/analytical visualizations (distributions, facets, parallel coordinates)
- NOT for: English-first teams, simple charts, real-time streaming, RTL

**Maturity & Momentum**:

- **Age**: ~10 years (May 2016)
- **GitHub stars**: ~12,500
- **npm weekly downloads**: ~251K
- **Contributors**: 224
- **Last commit**: March 2026 (actively maintained)
- **Backing**: Ant Group (corporate sustainability)
- **Release cadence**: Roughly monthly patches

**Notable Users**: Alibaba Cloud, Alipay, Taobao, Tmall, JD.com, Cainiao. `@ant-design/charts` is default charting in Ant Design Pro admin template.

**Latest Major Version**: v5.0.0 (March 2023), latest v5.4.8 (January 2026)

- Marks as first-class citizens (complete architectural rewrite)
- Transform API inspired by Observable Plot/Vega-Lite
- View tree for composable multi-view dashboards
- Declarative animation syntax, unified annotations pipeline
- First-class SSR, spec API alongside functional API
- New types: parallel coordinates, mosaic, gauges, Venn diagrams, geographic/network
- **Old version warning signs**: V4 geometry methods like `chart.interval().position('x*y')`, `Chart` constructor with `{ container, width, height }`, imports without mark-centric API

---

### billboard.js

**Name**: billboard.js

**License**: MIT

**URLs**:

- Website: <https://naver.github.io/billboard.js/>
- GitHub: <https://github.com/naver/billboard.js>
- API Docs: <https://naver.github.io/billboard.js/release/latest/doc/>
- Demos: <https://naver.github.io/billboard.js/demo/>
- npm: <https://www.npmjs.com/package/billboard.js>

**Overview**: Reusable, high-level charting library built on D3.js, fork and successor to C3.js. Created by NAVER Corp. (2017). Ships individual D3 sub-modules as direct deps (no separate D3 install needed). Modular since v2: chart types and interactions importable individually for tree-shaking. Renders SVG via D3. 18 chart types: Line, Spline, Step, Area, Area Range, Bar, Stacked Bar, Scatter, Bubble, Candlestick, Donut, Pie, Gauge, Polar, Radar, Treemap, Funnel, Combination. Features: C3.js API-compatible migration, 6 built-in CSS themes, plugin system (Stanford, TextOverlap, BubbleCompare, TableView, Sparkline), dynamic data loading, flow animation, zoom/pan, subchart, data export (PNG/SVG), React wrapper (`@billboard.js/react`).

**Gotchas**:

- **Tooltip positioning during zoom**: Tooltips jump/misalign after zoom, especially grouped bar tooltips. Use `tooltip.position` callback.
- **Font measurement timing**: Custom CSS fonts cause miscalculated tick dimensions. Load fonts before init or call `chart.flush()`.
- **Special characters in data names**: Break LinearGradient and other features. Use simple alphanumeric names.
- **D3 version coupling**: v3 requires D3 v6+; mixing versions causes silent failures.
- **Security**: Pre-3.17.3 has script injection vulnerability. Ensure 3.17.3+.
- **SVG-only**: Degrades with thousands of data points. No Canvas/WebGL.

**When is it a good fit?**:

- High-level declarative API without writing raw D3
- Migrating from C3.js (direct successor, compatible API)
- Broad chart type coverage (18 types) in single library
- CSS-based theming (swap one CSS file)
- Lightweight, no-framework-required (vanilla JS + React wrapper)
- Small-to-moderate datasets
- NOT for: Canvas/WebGL performance, bespoke visualizations, 3D/maps

**Maturity & Momentum**:

- **Age**: ~9 years (June 2017)
- **GitHub stars**: ~5,975
- **npm weekly downloads**: ~40K
- **Contributors**: 2 primary maintainers (NAVER) + ~30 community
- **Last commit**: March 2026 (actively maintained)
- **Release cadence**: Patches every 1-4 weeks; minors every 2-4 months

**Notable Users**: Canonical (snapcraft.io), Telstra, Prudential, Accenture, JFrog, NIH, City of Boston, Drupal Charts module, Liferay, NAVER DataLab, GitLens.

**Latest Major Version**: v3.0.0 (March 2021), latest v3.18.0 (January 2026)

- Upgraded to D3 v6+ (breaking event handling change)
- Candlestick, Polar, Funnel, Treemap chart types added across 3.x
- Enhanced export API, subchart APIs, arc annotations
- **Old version warning signs**: `d3.event` usage (v1/v2), monolithic `d3` import (v1/v2), all chart types available without explicit imports (v1), versions below 3.17.3 (security vuln)

---

### AG Charts

**Name**: AG Charts

**License**: Dual-license. Community edition (`ag-charts-community`) is **MIT** and free. Enterprise edition (`ag-charts-enterprise`) requires commercial license starting at **$499/dev/year**. Enterprise Bundle (AG Grid + AG Charts) from $1,498/dev/year.

**URLs**:

- Website: <https://www.ag-grid.com/charts/>
- GitHub: <https://github.com/ag-grid/ag-charts>
- Documentation: <https://www.ag-grid.com/charts/javascript/quick-start/>
- Examples: <https://www.ag-grid.com/charts/javascript-charts/>
- npm: <https://www.npmjs.com/package/ag-charts-community>

**Overview**: Built on a custom tree-based scene graph abstracting over HTML5 Canvas. Dirty-flag optimization prevents unnecessary redraws. M4 algorithm for time-series dimensionality reduction enables 1M+ data points with zoom/pan at 60fps. Zero runtime dependencies. **Community (free/MIT)**: Bar, Line, Area, Scatter, Bubble, Pie, Donut, Box Plot, Combination + accessibility, localization, tooltips, themes, stylers. **Enterprise ($499+)**: Adds Candlestick, OHLC, Heatmap, Histogram, Radar, Radial, Range, Sunburst, Treemap, Waterfall, Sankey, Chord, Gauges, Maps, Financial Charts, animations, annotations, zoom, crosshairs, navigator, high-frequency updates. v13+ supports selective module imports for up to 45% bundle reduction. Official React, Angular, Vue wrappers.

**AG Grid Integration**: Seamless — charts generated directly from grid data with shared theming. `chartThemeOverrides` API delegates to AG Charts theming.

**Gotchas**:

- **Frequent breaking changes**: Every major version (v10-v13, ~every 6 months) introduces substantial API renames/removals. No automated codemods.
- **Enterprise feature wall**: Animations, zoom, crosshairs, financial charts, maps all require paid license.
- **Main-thread bound**: All Canvas rendering on main thread; no Web Worker/OffscreenCanvas offloading.
- **Canvas-only**: No SVG output for print/export fidelity.
- **Tooltip issues**: Multiple charts on same page can produce rendering problems.

**When is it a good fit?**:

- Already using AG Grid (seamless integration, shared theming)
- Enterprise finance/trading/monitoring dashboards (high-frequency updates, Financial Charts, M4 algorithm)
- Canvas-first 60fps performance with large datasets
- Zero-dependency policy
- Framework-agnostic teams (same core across React/Angular/Vue/vanilla)
- NOT for: free-only basic charts (use Chart.js/ECharts), SVG output needs, teams intolerant of frequent breaking changes

**Maturity & Momentum**:

- **Age**: Standalone repo since July 2023; originally part of AG Grid (~2020)
- **GitHub stars**: ~447 (standalone; AG Grid main repo has 13K+)
- **npm weekly downloads**: ~731K (community), 1M+ across all packages
- **Release cadence**: Major every ~6 months, minor every ~2 months
- **LTS**: v10-lts branch maintained
- **Company**: AG Grid Ltd, commercially funded, 1,000+ enterprise customers

**Notable Users**: JPMorgan Chase (Salt design system), Adobe, Microsoft, Amazon, PayPal, IBM, BNP Paribas, NASA (AMMOS), MongoDB (Compass). 90%+ Fortune 500 use the AG Grid platform.

**Latest Major Version**: v13.0.0 (December 2025), latest v13.1.0 (February 2026)

- Module-based architecture (45% bundle reduction)
- High-frequency data updates at display-refresh rates
- `applyTransaction` API for efficient batch operations
- Zoom on data change strategies, simplified axis config
- Dynamic context menus, scrollbars, bar width customization
- **Old version warning signs**: `AgChart` (singular, pre-v10), `autosize` property (pre-v10), CSS `ag-chart-*` prefix (pre-v11), omitted `series[].type` (pre-v12), `nodeClick` event name (pre-v12)

---

### charming

**Name**: charming

**License**: MIT OR Apache-2.0

**URLs**:

- crates.io: <https://crates.io/crates/charming>
- GitHub: <https://github.com/yuankunzhang/charming>
- docs.rs: <https://docs.rs/charming>

**Overview**: Rust visualization library leveraging Apache ECharts for rendering. Declarative builder-pattern API mirroring ECharts component model. Output options: interactive HTML fragments, server-side images (PNG, JPEG, GIF, WEBP, SVG via embedded Deno engine), and client-side WASM. 15+ chart types: bar, line, area, pie, scatter, bubble, heatmap, radar, sankey, parallel coordinates, candlestick, boxplot, gauge, funnel, graph, calendar. 13+ built-in themes. Convenience macros (`df!`, `ds!`, `dim!`, `val!`).

**Gotchas**:

- `ssr` feature embeds `deno_core`/`serde_v8` — substantial compile time and binary size.
- Only 2.5% docs coverage on docs.rs — read examples and ECharts docs instead.
- `wasm` and `ssr` features are mutually exclusive.
- No stated MSRV; tracks latest stable Rust.

**When is it a good fit?**: Full ECharts power (rich interactivity, 15+ types, theming) in Rust. Ideal for dashboards, HTML reports, WASM web apps.

**Maturity & Momentum**: ~2.8 years, ~901K downloads, 2,500 stars, single maintainer. Last publish June 2025 (v0.6.0).

**Notable Users**: 9 reverse dependencies. **Latest**: v0.6.0 (June 2025).

---

### plotly.rs

**Name**: plotly (plotly.rs)

**License**: MIT

**URLs**:

- crates.io: <https://crates.io/crates/plotly>
- GitHub: <https://github.com/plotly/plotly.rs>
- docs.rs: <https://docs.rs/plotly>

**Overview**: Official-adjacent Rust bindings for Plotly.js under the `plotly` GitHub org. Generates interactive HTML or static images (PNG, JPEG, WEBP, SVG, PDF). Rust-native structs serializing to Plotly.js JSON. Feature flags: ndarray integration, image processing, embedded JS (~3.5MB), WASM (Yew). LTTB downsampling for large timeseries. Static export via WebDriver (Chrome/Firefox) replacing legacy Kaleido.

**Gotchas**:

- ~2.4MB crate size from bundled Plotly.js assets.
- Static image export requires running Chrome/Firefox with WebDriver.
- `plotly_embed_js` inflates HTML by ~3.5MB per file.
- Still 0.x after 6+ years — API can break between minors.

**When is it a good fit?**: Full Plotly.js ecosystem (50+ trace types) with HTML/browser output. Data science workflows, Jupyter-like notebooks, web dashboards. ndarray integration for numerical computing.

**Maturity & Momentum**: ~6 years, ~2.78M downloads, 1,400 stars, 83 reverse deps (most depended-upon Rust charting crate). Last publish February 2026 (v0.14.1). Actively maintained.

---

### plotlars

**Name**: plotlars

**License**: MIT

**URLs**:

- crates.io: <https://crates.io/crates/plotlars>
- GitHub: <https://github.com/alceal/plotlars>
- docs.rs: <https://docs.rs/plotlars>

**Overview**: Bridge library connecting Polars DataFrames directly to Plotly.js visualization. Column-name-based builder API maps DataFrame columns to axes, colors, facets. 30+ plot types: scatter, line, bar, histogram, box, contour, heatmap, pie, candlestick, OHLC, Sankey, 3D scatter, surface, scatter geo, density mapbox, time series, tables, subplots. Image export via WebDriver.

**Gotchas**:

- Tightly coupled to Polars — unnecessary overhead if data isn't in DataFrames.
- 42 versions in ~1.7 years (0.x) — rapidly evolving, potentially unstable API.
- Solo maintainer. Inherits plotly.rs weight plus Polars dependency — very heavy.

**When is it a good fit?**: Polars-centric data pipelines wanting DataFrame-to-chart with minimal boilerplate. Exploratory data analysis.

**Maturity & Momentum**: ~1.7 years, ~53K downloads, 633 stars. Last publish March 2026 (v0.11.8). Very active single maintainer.

---

### charts-rs

**Name**: charts-rs

**License**: Apache-2.0

**URLs**:

- crates.io: <https://crates.io/crates/charts-rs>
- GitHub: <https://github.com/vicanso/charts-rs>
- docs.rs: <https://docs.rs/charts-rs>

**Overview**: Pure-Rust charting rendering directly to SVG, PNG, JPEG, WEBP, AVIF — no JavaScript or browser required. 10 chart types: bar, horizontal bar, line, pie, radar, scatter, candlestick, table, heatmap, multi-chart compositions. 9 built-in themes. Custom TTF/OTF font loading, dual Y-axes, smooth curves, area fills, mark points/lines, JSON-based configuration.

**Gotchas**:

- Apache-2.0 only (no MIT dual option).
- Fewer chart types (10) than ECharts/Plotly-based alternatives.
- 75+ versions at 0.3.x — significant API churn.
- Font rendering edge cases with non-Latin scripts.

**When is it a good fit?**: Pure-Rust chart image generation without JavaScript/browser/WebDriver. Server-side in constrained environments (Docker, CI, embedded). JSON config for dynamic generation.

**Maturity & Momentum**: ~3 years, 304 stars, solo maintainer. Last publish March 2026 (v0.3.28). Actively maintained.

---

### poloto

**Name**: poloto

**License**: MIT

**URLs**:

- crates.io: <https://crates.io/crates/poloto>
- GitHub: <https://github.com/tiby312/poloto-project>
- docs.rs: <https://docs.rs/poloto>

**Overview**: Lightweight 2D plotting outputting pure SVG styled via CSS. Minimal design: generate clean SVG themeable with CSS (light/dark mode, hover, animations). Supports line, scatter, bar charts. Trait-based API with `plots!` macro for chaining series. Companion crate `poloto-chrono` for timestamp axes.

**Gotchas**:

- **Appears abandoned** — last publish July 2023, last commit June 2023.
- Unconventional versioning (jumped 18.x to 19.x) signals major API churn.
- Only 27% docs coverage. Limited to 3 basic chart types.
- Warns against many-plot scenarios due to SVG performance.

**When is it a good fit?**: Dead-simple CSS-themeable SVG for static sites or documentation. Tiny dependency footprint.

**Maturity & Momentum**: ~4 years, 164 stars, solo maintainer. **Effectively unmaintained** since mid-2023. Latest: v19.1.2 (July 2023).

---

### textplots

**Name**: textplots

**License**: MIT

**URLs**:

- crates.io: <https://crates.io/crates/textplots>
- GitHub: <https://github.com/loony-bean/textplots-rs>
- docs.rs: <https://docs.rs/textplots>

**Overview**: Terminal plotting using Unicode Braille characters in any monospaced terminal. Builder-pattern API: `Chart` with viewport dimensions, add `lineplot()` with `Shape` (continuous function or discrete points), call `display()`. Colored output via `ColorPlot`, configurable axis labels and tick density. Optional CLI binary for plotting math expressions.

**Gotchas**:

- Only line plots — no bar, histogram, scatter-only, or heatmap.
- Requires terminal with Unicode Braille block support (U+2800-U+28FF).
- `Shape::Continuous` forces heap allocation via `Box<dyn Fn>`.
- Optional `tool` feature uses deprecated `structopt`.

**When is it a good fit?**: Quick inline terminal visualization of numerical data or function curves. CLI sparklines/trends, debugging output, REPL data exploration. Braille gives surprisingly readable results.

**Maturity & Momentum**: ~7.8 years (one of oldest Rust plotting crates), ~862K downloads, 281 stars, 45 reverse deps. Maintained but infrequent updates. Latest: v0.8.7 (February 2025).

---

### lowcharts

**Name**: lowcharts

**License**: MIT

**URLs**:

- crates.io: <https://crates.io/crates/lowcharts>
- GitHub: <https://github.com/juan-leon/lowcharts>
- docs.rs: <https://docs.rs/lowcharts>

**Overview**: CLI tool and library for low-resolution terminal charts, designed for operational troubleshooting. 6 chart types: bar charts (pattern counting), histograms (numerical distribution, optional log scale), time histograms (log frequency over time with auto-detected timestamps), split time histograms, common-term histograms (top-N ranking), and X-Y plots (metric evolution with chunk averaging). `stats` module for supporting statistical functions.

**Gotchas**:

- Only 3 crate versions ever published; crates.io (v0.5.8, Jan 2023) lags GitHub (v0.5.9, Feb 2025) by 2+ years.
- Primarily a CLI tool; library API is secondary and may lack ergonomic patterns.
- Plain text with basic block characters — no Braille, no color.

**When is it a good fit?**: Pipe-friendly CLI for visualizing log files and stdin. Operational debugging: response time distributions, request frequency, common error messages. Library for embedding simple histogram/bar output in Rust CLIs.

**Maturity & Momentum**: ~3.8 years, ~466K downloads, 246 stars, solo maintainer. CLI-focused with high download count. Latest on crates.io: v0.5.8 (January 2023); GitHub: v0.5.9 (February 2025).

## Summary: Comparison, Categories, and Recommendations

### Categorization

Libraries fall into six natural groupings based on abstraction level and deployment target:

| Category | Libraries | Characteristics |
|---|---|---|
| **High-Level Declarative (JS)** | Chart.js, ECharts, ApexCharts, Highcharts, AG Charts, billboard.js, Frappe Charts | Config-driven, batteries-included, minimal code to first chart |
| **React-Specific** | Recharts, Nivo, visx | Component-based APIs, deep React integration |
| **Low-Level / Grammar-of-Graphics** | D3.js, Observable Plot, AntV G2 | Maximum flexibility, steeper learning curve, composable primitives |
| **Performance-Specialized (JS)** | uPlot, Plotly.js, SciChart.js, LightningChart JS | Optimized for large datasets or 3D; Canvas/WebGL rendering |
| **Rust Native** | Plotters, charming, plotly.rs, plotlars, charts-rs, poloto, textplots, lowcharts | Compile-time safety, server-side generation, no browser required |
| **OS Binaries / CLI** | Graphviz, gnuplot, vl-convert, YouPlot | Standalone executables, scriptable, piped workflows |

### JS/TS Quick Comparison

| Library | License | Renderer | Bundle (min+gz) | Framework | Chart Types | npm/wk | Best For |
|---|---|---|---|---|---|---|---|
| Chart.js | MIT | Canvas | ~70 KB | Any | ~20 | 7.8M | Quick dashboards |
| Plotly.js | MIT | SVG+WebGL | ~300 KB (partial) | Any | 50+ | 947K | Scientific / 3D |
| ECharts | Apache 2.0 | Canvas+SVG | ~180 KB (shaken) | Any | 30+ | 2.27M | Feature-rich dashboards |
| Nivo | MIT | SVG+Canvas+HTML | ~50 KB/component | React | 25+ | 1.6M | Beautiful React dashboards |
| D3.js | ISC | SVG/Canvas/HTML | ~90 KB (full) | Any | Unlimited | 9M | Custom bespoke viz |
| ApexCharts | Dual (<$2M free) | SVG | ~130 KB | Any | 18+ | 1.71M | Interactive dashboards |
| Highcharts | Proprietary | SVG | ~80 KB | Any | 30+ | 2.17M | Enterprise (with budget) |
| amCharts | Linkware/Paid | Canvas | ~200 KB+ | Any | 25+ | 309K | Geo/maps, complex |
| Recharts | MIT | SVG | ~50 KB (shaken) | React | 15+ | 21.6M | React projects (most popular) |
| visx | MIT | SVG | ~5-15 KB/pkg | React | Composable | 483K | Custom React viz |
| Observable Plot | ISC | SVG | ~50 KB | Any | Grammar-based | 150K | Exploratory analysis |
| uPlot | MIT | Canvas | ~35 KB | Any | Time-series | 765K | Performance-critical |
| Frappe Charts | MIT | SVG | ~18 KB | Any | 6 | 34K | Minimal needs |
| billboard.js | MIT | SVG | ~120 KB | Any | 15+ | 40K | D3-based declarative |
| AG Charts | MIT/$499+ | Canvas | ~150 KB | Any | 20+ | 731K | AG Grid integration |
| AntV G2 | MIT | Canvas+SVG+WebGL | ~200 KB | Any | Grammar-based | 251K | Chinese ecosystem |

### Rust Quick Comparison

| Library | License | Output | Key Deps | Downloads | Status | Best For |
|---|---|---|---|---|---|---|
| Plotters | MIT | PNG, SVG, Canvas | image, ttf-parser | 140M | Active | General-purpose server-side |
| charming | MIT/Apache-2.0 | HTML (ECharts) | serde, ECharts runtime | 901K | Active | ECharts from Rust |
| plotly.rs | MIT | HTML (Plotly.js) | serde, Plotly.js runtime | 2.78M | Active | Interactive HTML from Rust |
| plotlars | MIT | HTML (Plotly.js) | polars, plotly.rs | 53K | Active | Polars DataFrames |
| charts-rs | Apache-2.0 | PNG, SVG | image, resvg | — | Active | Pure-Rust image gen |
| textplots | MIT | Terminal (braille) | None significant | 862K | Maintained | Terminal sparklines |
| lowcharts | MIT | Terminal (block) | crossterm | 466K | Maintained | CLI histograms |
| poloto | MIT | SVG only | None significant | — | Unmaintained | Lightweight SVG |
| PGFPlots | MIT | PDF (LaTeX) | tectonic/pdflatex | — | Dormant | Academic papers |

### License and Cost Comparison

| Library | Free Tier | Paid Tier | Notes |
|---|---|---|---|
| Chart.js | Full (MIT) | — | Completely free |
| D3.js | Full (ISC) | — | Completely free |
| ECharts | Full (Apache 2.0) | — | Completely free |
| Recharts | Full (MIT) | — | Completely free |
| Nivo | Full (MIT) | — | Completely free |
| visx | Full (MIT) | — | Completely free |
| uPlot | Full (MIT) | — | Completely free |
| Observable Plot | Full (ISC) | — | Completely free |
| Plotly.js | Full (MIT) | Dash Enterprise (hosting) | Core library free |
| ApexCharts | Free if revenue <$2M | Commercial required >$2M | Revenue threshold |
| AG Charts | Community (MIT, limited) | $499+/dev/yr | Financial charts enterprise-only |
| Highcharts | Free non-commercial | $590-$2,340/dev (perpetual) | Per-product licensing |
| amCharts | Free with linkware | $490-$990 (remove branding) | Must display amCharts link if free |
| SciChart.js | Trial only | $2,000+/dev (perpetual) | No free production use |
| LightningChart JS | Trial only | $1,500-$6,000/dev/yr | Subscription only |
| Plotters | Full (MIT) | — | Completely free |
| plotly.rs | Full (MIT) | — | Completely free |
| Graphviz | Full (EPL-1.0) | — | Completely free |
| gnuplot | Full (permissive) | — | Completely free |

### Recommendations by Use Case

#### General-Purpose Dashboard Charting

| Priority | Library | Rationale |
|---|---|---|
| 1st | **ECharts** | Richest built-in chart types, excellent theming, good tree-shaking, framework-agnostic |
| 2nd | **Chart.js** | Smaller bundle, simpler API, massive community, sufficient for most dashboards |
| 3rd | **Highcharts** | Best documentation and polish — if budget allows |

#### React-Specific Charting

| Priority | Library | Rationale |
|---|---|---|
| 1st | **Recharts** | 21.6M weekly downloads, idiomatic React components, declarative API |
| 2nd | **Nivo** | More chart types, better default aesthetics, server-side rendering support |
| 3rd | **visx** | Maximum control with React primitives — pick when Recharts/Nivo are too opinionated |

#### Financial / Trading Applications

| Priority | Library | Rationale |
|---|---|---|
| 1st | **SciChart.js** | Purpose-built for trading: real-time streaming, OHLC, depth charts, WASM performance |
| 2nd | **LightningChart JS** | Billion-point scale, WebGL, real-time data ingestion |
| 3rd | **AG Charts Enterprise** | Candlestick, waterfall, range area built-in; pairs with AG Grid |
| Budget | **uPlot** + custom overlays | MIT-licensed alternative with excellent time-series performance |

#### Scientific / Data Analysis

| Priority | Library | Rationale |
|---|---|---|
| 1st | **Plotly.js** | 50+ chart types, 3D surfaces, statistical charts, LaTeX labels, Jupyter integration |
| 2nd | **Observable Plot** | Grammar-of-graphics, exploratory workflow, concise API |
| 3rd | **ECharts** | Heatmaps, parallel coordinates, graph analysis built-in |
| Rust | **plotly.rs** | Generate Plotly HTML from Rust data pipelines |

#### Large Dataset Performance (100K+ Points)

| Scale | Library | Approach |
|---|---|---|
| 100K-1M | **uPlot** | Canvas, ~35 KB, fastest open-source option |
| 100K-10M | **ECharts** (with `largeMode`) | Canvas downsampling, progressive rendering |
| 1M-100M | **SciChart.js** | WebGL + WASM, built for this scale |
| 100M-1B+ | **LightningChart JS** | WebGL, purpose-built for extreme datasets |
| Any scale | Avoid SVG-based renderers | SVG DOM nodes choke beyond ~10K elements |

#### Minimal Bundle Size

| Priority | Library | Size (min+gz) |
|---|---|---|
| 1st | **Frappe Charts** | ~18 KB |
| 2nd | **uPlot** | ~35 KB |
| 3rd | **visx** (individual packages) | ~5-15 KB per package |
| 4th | **Chart.js** (tree-shaken) | ~40-70 KB |

#### Server-Side Image Generation

**JavaScript (Node.js):**

| Priority | Library | Approach |
|---|---|---|
| 1st | **vl-convert** | Vega-Lite spec to PNG/SVG/PDF — no browser needed |
| 2nd | **ECharts** + `node-canvas` | SSR support built-in |
| 3rd | **Chart.js** + `chartjs-node-canvas` | Mature Node.js canvas integration |

**Rust:**

| Priority | Library | Approach |
|---|---|---|
| 1st | **Plotters** | Direct PNG/SVG output, no runtime dependencies |
| 2nd | **charts-rs** | Pure-Rust image generation via resvg |
| 3rd | **charming** (SSR mode) | ECharts rendering without browser via built-in renderer |

#### Terminal / CLI Visualization

| Priority | Library | Approach |
|---|---|---|
| 1st | **textplots** (Rust) | Braille characters, zero dependencies, embeddable |
| 2nd | **lowcharts** (Rust) | Histograms, bar charts, time-series in terminal |
| 3rd | **YouPlot** (Ruby) | Pipe-friendly CLI: `cat data.csv \| uplot bar` |
| 4th | **gnuplot** (dumb terminal) | `set terminal dumb` for ASCII output |

#### Bespoke / Custom Visualizations

| Priority | Library | Rationale |
|---|---|---|
| 1st | **D3.js** | The gold standard — bind data to any DOM element, unlimited flexibility |
| 2nd | **visx** | D3 power with React component ergonomics |
| 3rd | **Observable Plot** | D3-based but higher-level; good middle ground |

#### Graph / Network Visualization

| Priority | Library | Rationale |
|---|---|---|
| 1st | **Graphviz** | 30+ years, battle-tested, DOT language, automatic layouts |
| 2nd | **D3.js** (force layout) | Interactive, customizable, browser-native |
| 3rd | **ECharts** (graph type) | Built-in force/circular/tree layouts, less effort than D3 |

#### Geographic / Map Visualization

| Priority | Library | Rationale |
|---|---|---|
| 1st | **ECharts** (geo/map) | Built-in geo projection, choropleth, flight routes |
| 2nd | **D3.js** + d3-geo | Maximum cartographic control, any projection |
| 3rd | **amCharts** | Best out-of-box map experience (drill-down, heat maps) |
| 4th | **Plotly.js** | Mapbox integration, choropleths, scattergeo |

### Key Takeaways

- **The JS charting landscape is mature and saturated.** For most dashboard use cases, Chart.js or ECharts cover 90% of needs at zero cost. Only reach for paid libraries when you have genuine performance or feature requirements that free options cannot meet.

- **React dominance skews downloads.** Recharts at 21.6M weekly downloads dwarfs everything else, but this reflects React's market share, not inherent superiority. Framework-agnostic libraries (ECharts, Chart.js) are safer long-term bets for projects that may outlive their current framework.

- **Rendering technology dictates performance ceilings.** SVG tops out around 5-10K elements; Canvas handles 100K-1M points; WebGL/WASM pushes into billions. Choose the renderer for your data scale, not your current dataset.

- **The Rust charting ecosystem is narrower but viable.** Plotters is the clear leader for server-side image generation. For interactive output, plotly.rs and charming bridge to mature JS ecosystems. Pure-Rust options lack the breadth of JS libraries but excel at headless, high-throughput generation.

- **Commercial libraries justify their cost only at extremes.** SciChart and LightningChart deliver genuine value for real-time financial data or billion-point scientific datasets. For everything else, MIT/Apache alternatives match or exceed their capabilities.

- **D3.js remains unmatched for bespoke work** but is overkill for standard charts. Its 9M weekly downloads reflect its role as a dependency (Recharts, Nivo, billboard.js, Observable Plot all build on it) more than direct usage.

- **Terminal charting is an underserved niche with adequate tools.** textplots and lowcharts in Rust, YouPlot in Ruby, and gnuplot's dumb terminal mode cover basic needs, but none approach the polish of browser-based options. This is an opportunity space for Rust CLI tooling.
