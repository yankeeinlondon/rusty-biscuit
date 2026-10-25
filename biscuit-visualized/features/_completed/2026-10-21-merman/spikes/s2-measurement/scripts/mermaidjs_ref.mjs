// Mermaid.js references for spike S2: renders each `.mmd` in headless Chrome
// with the local mermaid-cli's Mermaid and puppeteer, then reads back, for every
// label, the lines the browser actually laid out (character rects grouped by
// row). Also writes the reference SVG and a PNG screenshot.
//
// Usage: node scripts/mermaidjs_ref.mjs <out-dir> <fontFamily> [<regular-file>[,<bold-file>]] -- <a.mmd> <b.mmd> ...
// With <font-file>, that face is loaded through @font-face under <fontFamily>
// (used to give the Linux host's DejaVu Sans to Chrome on macOS).
import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";

const cli = "/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@mermaid-js/mermaid-cli/";
const require = createRequire(cli + "package.json");
const puppeteer = require("puppeteer");
// MERMAID_PACKAGE=<dir> swaps in another Mermaid build (an unpacked npm package)
// in the same Chrome; default is the one mermaid-cli bundles.
const mermaidDir = process.env.MERMAID_PACKAGE ?? path.dirname(require.resolve("mermaid/package.json"));
const mermaidJs = fs.readFileSync(path.join(mermaidDir, "dist/mermaid.min.js"), "utf8");
const mermaidVersion = JSON.parse(fs.readFileSync(path.join(mermaidDir, "package.json"))).version;

const args = process.argv.slice(2);
const sep = args.indexOf("--");
const [outDir, fontFamily, fontFile] = args.slice(0, sep);
const inputs = args.slice(sep + 1);
fs.mkdirSync(outDir, { recursive: true });

// `<regular>[,<bold>]`
const fontFace = fontFile
  ? fontFile.split(",").map((file, i) =>
      `@font-face{font-family:"${fontFamily}";font-weight:${i ? 700 : 400};src:url(data:font/ttf;base64,${fs.readFileSync(file).toString("base64")});}`).join("")
  : "";

const browser = await puppeteer.launch({ headless: true, args: ["--no-sandbox"] });
const page = await browser.newPage();
await page.setViewport({ width: 1600, height: 1200, deviceScaleFactor: 2 });
await page.setContent(`<!doctype html><html><head><style>${fontFace} body{margin:0;background:#1f2020}</style></head><body><div id="c"></div></body></html>`);
await page.addScriptTag({ content: mermaidJs });
if (fontFile) {
  await page.evaluate(async (family) => { await document.fonts.load(`16px "${family}"`); }, fontFamily);
}

const results = { mermaid_version: mermaidVersion, font_family: fontFamily, font_file: fontFile ?? null, diagrams: {} };
for (const input of inputs) {
  const name = path.basename(input, ".mmd");
  const source = fs.readFileSync(input, "utf8");
  const result = await page.evaluate(async (source, fontFamily) => {
    // Fresh initialize per diagram: Mermaid keeps some config between renders.
    mermaid.initialize({ startOnLoad: false, theme: "dark", fontFamily, themeVariables: { fontFamily } });
    const container = document.getElementById("c");
    container.innerHTML = "";
    let svg;
    try {
      ({ svg } = await mermaid.render("ref", source));
    } catch (error) {
      return { error: String(error && error.message || error) };
    }
    container.innerHTML = svg;
    const root = container.querySelector("svg");
    // A label is a foreignObject, or a <text> that is not inside one.
    const units = [...root.querySelectorAll("foreignObject")];
    for (const t of root.querySelectorAll("text")) {
      if (!t.closest("foreignObject")) units.push(t);
    }
    const labels = [];
    for (const unit of units) {
      const walker = document.createTreeWalker(unit, NodeFilter.SHOW_TEXT);
      const rows = [];
      let node;
      while ((node = walker.nextNode())) {
        const text = node.textContent;
        for (let i = 0; i < text.length; i++) {
          const range = document.createRange();
          range.setStart(node, i);
          range.setEnd(node, i + 1);
          const rect = [...range.getClientRects()][0];
          if (!rect || rect.width === 0 && /\s/.test(text[i])) {
            if (rows.length && /\s/.test(text[i])) rows[rows.length - 1].text += " ";
            continue;
          }
          const mid = rect.top + rect.height / 2;
          let row = rows.find((r) => Math.abs(r.mid - mid) < rect.height / 2);
          if (!row) {
            row = { mid, text: "", left: rect.left, right: rect.right };
            rows.push(row);
          }
          row.text += text[i];
          row.left = Math.min(row.left, rect.left);
          row.right = Math.max(row.right, rect.right);
        }
      }
      rows.sort((a, b) => a.mid - b.mid);
      const lines = rows.map((r) => r.text.replace(/\s+/g, " ").trim()).filter((l) => l.length);
      if (lines.length) {
        const box = unit.getBoundingClientRect();
        labels.push({
          kind: unit.tagName.toLowerCase(),
          lines,
          widths: rows.filter((r) => r.text.trim()).map((r) => r.right - r.left),
          box_width: box.width,
        });
      }
    }
    const bbox = root.getBoundingClientRect();
    return { svg: root.outerHTML, labels, width: bbox.width, height: bbox.height };
  }, source, fontFamily);
  if (result.error) {
    results.diagrams[name] = { error: result.error };
    console.error(`FAIL ${name}: ${result.error.slice(0, 200)}`);
    continue;
  }
  fs.writeFileSync(path.join(outDir, `${name}.mermaid-js.svg`), result.svg);
  const element = await page.$("#c svg");
  await element.screenshot({ path: path.join(outDir, `${name}.mermaid-js.png`), omitBackground: false });
  results.diagrams[name] = { labels: result.labels, width: result.width, height: result.height };
}
fs.writeFileSync(path.join(outDir, "mermaid-js.json"), JSON.stringify(results, null, 2) + "\n");
await browser.close();
console.log(`Mermaid ${mermaidVersion}: ${inputs.length} inputs -> ${outDir}`);
