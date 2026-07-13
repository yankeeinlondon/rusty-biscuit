#!/usr/bin/env -S npx tsx
/**
 * File-level import-cycle and coupling-hub finder for Rust crates.
 *
 * Builds a module-file graph from `use crate::/super::/self::` statements plus
 * inline `crate::` qualified paths, then reports:
 *   - strongly connected components of size > 1 (import cycles), with the
 *     intra-cycle edges so the back-edge to break is visible, and
 *   - the highest fan-in / fan-out module files (coupling hubs).
 *
 * Usage: tsx module-cycles.ts [dir ...] [--no-inline] [--md]
 *        bun module-cycles.ts [dir ...] [--no-inline] [--md]
 *
 * `--md` fences the report in a ```text code block so it survives
 * Markdown-pipeline whitespace cleanup verbatim (for `::shell` embedding).
 *
 * Each dir may be a crate `src/` directory (analyzed directly) or a package
 * area (every `<child>/Cargo.toml` + `<child>/src` crate inside it is
 * discovered, plus the dir's own crate if it has one). Defaults to the
 * current directory. Cross-crate imports cannot cycle (Cargo forbids them),
 * so each crate is analyzed independently. `--no-inline` restricts the graph
 * to `use` statements, ignoring inline `crate::path::item` expressions.
 *
 * No dependencies — node:fs/node:path only.
 */

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { basename, join, relative, sep } from "node:path";

/** A module path is stored as its segments joined with "::" ("" = crate root). */
type ModPath = string;

function stripCommentsAndStrings(text: string): string {
  // crude but adequate: remove block comments, line comments, string literals
  return text
    .replace(/\/\*[\s\S]*?\*\//g, " ")
    .replace(/\/\/[^\n]*/g, " ")
    .replace(/"(?:\\[\s\S]|[^"\\])*"/g, '""');
}

function rustFiles(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      out.push(...rustFiles(full));
    } else if (entry.endsWith(".rs")) {
      out.push(full);
    }
  }
  return out;
}

/** module path -> file */
function moduleMap(src: string): Map<ModPath, string> {
  const mods = new Map<ModPath, string>();
  for (const file of rustFiles(src)) {
    const parts = relative(src, file).split(sep);
    const last = parts[parts.length - 1];
    if (parts.length === 1 && (last === "lib.rs" || last === "main.rs")) {
      mods.set("", file);
      continue;
    }
    const stem = last.slice(0, -3);
    const dirs = parts.slice(0, -1);
    const segments = stem === "mod" ? dirs : [...dirs, stem];
    mods.set(segments.join("::"), file);
  }
  return mods;
}

/** Expand a use-statement body (after `use `, before `;`) into path lists. */
function expandUse(body: string): string[][] {
  const results: string[][] = [];

  function walk(prefix: string[], s: string): void {
    s = s.trim();
    // split top-level commas
    let depth = 0;
    const items: string[] = [];
    let cur = "";
    for (const ch of s) {
      if (ch === "{") {
        depth += 1;
        cur += ch;
      } else if (ch === "}") {
        depth -= 1;
        cur += ch;
      } else if (ch === "," && depth === 0) {
        items.push(cur);
        cur = "";
      } else {
        cur += ch;
      }
    }
    if (cur.trim()) items.push(cur);

    for (let item of items) {
      item = item.trim();
      if (!item) continue;
      const group = item.match(/^([\s\S]*?)::\{([\s\S]*)\}$/);
      if (group) {
        walk([...prefix, ...group[1].split("::").filter(Boolean)], group[2]);
      } else if (item.startsWith("{") && item.endsWith("}")) {
        walk(prefix, item.slice(1, -1));
      } else {
        item = item.replace(/\s+as\s+\w+$/, "");
        results.push([...prefix, ...item.split("::").map((p) => p.trim()).filter(Boolean)]);
      }
    }
  }

  walk([], body);
  return results;
}

/** Resolve a use path to a module, or null (external crate / std). */
function resolve(pathParts: string[], currentMod: ModPath, mods: Map<ModPath, string>): ModPath | null {
  const parts = pathParts.map((p) => p.trim()).filter(Boolean);
  if (parts.length === 0) return null;
  const head = parts[0];
  let base: string[];
  let rest: string[];
  if (head === "crate") {
    base = [];
    rest = parts.slice(1);
  } else if (head === "super") {
    base = currentMod === "" ? [] : currentMod.split("::");
    rest = parts;
    while (rest.length > 0 && rest[0] === "super") {
      base = base.slice(0, -1);
      rest = rest.slice(1);
    }
  } else if (head === "self") {
    base = currentMod === "" ? [] : currentMod.split("::");
    rest = parts.slice(1);
  } else {
    return null;
  }
  const full = [...base, ...rest];
  // longest prefix that is a module file
  for (let i = full.length; i > 0; i--) {
    const cand = full.slice(0, i).join("::");
    if (mods.has(cand)) return cand;
  }
  return mods.has("") ? "" : null;
}

function buildGraph(src: string, inline: boolean): { mods: Map<ModPath, string>; edges: Map<ModPath, Set<ModPath>> } {
  const mods = moduleMap(src);
  const edges = new Map<ModPath, Set<ModPath>>();
  const addEdge = (from: ModPath, to: ModPath) => {
    if (to === from) return;
    if (!edges.has(from)) edges.set(from, new Set());
    edges.get(from)!.add(to);
  };

  for (const [cur, file] of mods) {
    const text = stripCommentsAndStrings(readFileSync(file, "utf-8"));
    for (const m of text.matchAll(/\buse\s+([\w:{},\s*]+?);/g)) {
      for (const path of expandUse(m[1])) {
        const tgt = resolve(path.filter((p) => p !== "*"), cur, mods);
        if (tgt !== null) addEdge(cur, tgt);
      }
    }
    if (inline) {
      for (const m of text.matchAll(/\bcrate::([\w:]+)/g)) {
        const tgt = resolve(["crate", ...m[1].split("::")], cur, mods);
        if (tgt !== null) addEdge(cur, tgt);
      }
    }
  }
  return { mods, edges };
}

/** Iterative Tarjan; returns SCCs of size > 1. */
function sccs(nodes: ModPath[], edges: Map<ModPath, Set<ModPath>>): ModPath[][] {
  let counter = 0;
  const index = new Map<ModPath, number>();
  const lowlink = new Map<ModPath, number>();
  const stack: ModPath[] = [];
  const onStack = new Set<ModPath>();
  const result: ModPath[][] = [];

  function strongconnect(v: ModPath): void {
    const work: [ModPath, number][] = [[v, 0]];
    while (work.length > 0) {
      const frame = work[work.length - 1];
      const [node, pi] = frame;
      if (pi === 0) {
        index.set(node, counter);
        lowlink.set(node, counter);
        counter += 1;
        stack.push(node);
        onStack.add(node);
      }
      let recursed = false;
      const succs = [...(edges.get(node) ?? [])].sort();
      for (let i = pi; i < succs.length; i++) {
        const w = succs[i];
        if (!index.has(w)) {
          frame[1] = i + 1;
          work.push([w, 0]);
          recursed = true;
          break;
        } else if (onStack.has(w)) {
          lowlink.set(node, Math.min(lowlink.get(node)!, index.get(w)!));
        }
      }
      if (recursed) continue;
      if (lowlink.get(node) === index.get(node)) {
        const comp: ModPath[] = [];
        for (;;) {
          const w = stack.pop()!;
          onStack.delete(w);
          comp.push(w);
          if (w === node) break;
        }
        result.push(comp);
      }
      work.pop();
      if (work.length > 0) {
        const parent = work[work.length - 1][0];
        lowlink.set(parent, Math.min(lowlink.get(parent)!, lowlink.get(node)!));
      }
    }
  }

  for (const v of [...nodes].sort()) {
    if (!index.has(v)) strongconnect(v);
  }
  return result.filter((c) => c.length > 1);
}

function fmt(mod: ModPath): string {
  return mod === "" ? "crate (root)" : `crate::${mod}`;
}

/** Expand an argument into crate `src/` directories (see usage doc). */
function crateSrcDirs(dir: string): string[] {
  const isSrcDir = basename(dir) === "src" || existsSync(join(dir, "lib.rs")) || existsSync(join(dir, "main.rs"));
  if (isSrcDir) return [dir];
  const out: string[] = [];
  if (existsSync(join(dir, "Cargo.toml")) && existsSync(join(dir, "src"))) {
    out.push(join(dir, "src"));
  }
  for (const entry of readdirSync(dir)) {
    const child = join(dir, entry);
    if (
      statSync(child).isDirectory() &&
      existsSync(join(child, "Cargo.toml")) &&
      existsSync(join(child, "src"))
    ) {
      out.push(join(child, "src"));
    }
  }
  return out;
}

function analyze(src: string, inline: boolean): void {
  const { mods, edges } = buildGraph(src, inline);
  const comps = sccs([...mods.keys()], edges);
  console.log(`${src}  (${mods.size} module files, inline=${inline})`);

  console.log(`\n== cycles: ${comps.length} SCC(s) ==`);
  if (comps.length === 0) console.log("  none");
  for (const comp of [...comps].sort((a, b) => a.length - b.length)) {
    console.log(`  CYCLE (${comp.length} modules):`);
    const compSet = new Set(comp);
    for (const m of [...comp].sort()) {
      const intra = [...(edges.get(m) ?? [])].filter((t) => compSet.has(t)).sort();
      console.log(`    ${fmt(m)} -> ${intra.map(fmt).join(", ")}`);
    }
  }

  const fanIn = new Map<ModPath, number>();
  for (const tgts of edges.values()) {
    for (const t of tgts) fanIn.set(t, (fanIn.get(t) ?? 0) + 1);
  }
  console.log("\n== top fan-out (imports the most sibling modules) ==");
  for (const [m, tgts] of [...edges.entries()].sort((a, b) => b[1].size - a[1].size).slice(0, 5)) {
    console.log(`  ${String(tgts.size).padStart(3)}  ${fmt(m)}`);
  }
  console.log("== top fan-in (imported by the most sibling modules) ==");
  for (const [m, n] of [...fanIn.entries()].sort((a, b) => b[1] - a[1]).slice(0, 5)) {
    console.log(`  ${String(n).padStart(3)}  ${fmt(m)}`);
  }
}

function main(): void {
  // exit quietly when a downstream pipe (e.g. `| head`) closes early
  process.stdout.on("error", (e: NodeJS.ErrnoException) => {
    if (e.code === "EPIPE") process.exit(0);
    throw e;
  });
  const args = process.argv.slice(2);
  const inline = !args.includes("--no-inline");
  const dirs = args.filter((a) => !a.startsWith("--"));
  const srcDirs = (dirs.length > 0 ? dirs : ["."]).flatMap(crateSrcDirs);
  if (srcDirs.length === 0) {
    console.error("module-cycles.ts: no crate src/ directories found");
    process.exit(2);
  }
  const fence = args.includes("--md");
  if (fence) console.log("```text");
  srcDirs.forEach((src, i) => {
    if (i > 0) console.log();
    analyze(src, inline);
  });
  if (fence) console.log("```");
}

main();
