// Self-consistency of the frozen Python parity fixtures.
//
// `cli/capture-parity.ts` freezes what `scripts/ci/schema.py` answers; these
// tests need no TypeScript port. They prove the fixture set is complete and
// internally coherent, so a later port can trust it as ground truth.

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, readdirSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

type CaseKind = "canonical" | "collection" | "receipt-adapter" | "loader" | "validator";

interface ParityCase {
  readonly name: string;
  readonly kind: CaseKind;
  readonly files: readonly string[];
  readonly input?: string;
  readonly canonical?: string;
  readonly identity?: string;
  readonly observation_ids?: readonly string[];
  readonly problems?: readonly string[] | null;
  readonly codes?: readonly string[] | null;
  readonly valid?: boolean;
  readonly exception?: string;
  readonly accepts?: boolean;
  readonly error?: string;
}

interface ParityManifest {
  readonly cases: readonly ParityCase[];
}

function readManifest(url: URL): ParityManifest {
  return JSON.parse(readFileSync(url, "utf-8")) as ParityManifest;
}

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

function listFiles(root: string): string[] {
  const files: string[] = [];
  const walk = (directory: string): void => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) walk(path);
      else files.push(relative(root, path).split(sep).join("/"));
    }
  };
  walk(root);
  return files.sort();
}

function isAscii(bytes: Uint8Array): boolean {
  return bytes.every((byte) => byte < 0x80);
}

function pythonAvailable(): boolean {
  const python = process.platform === "win32" ? "python" : "python3";
  const probe = spawnSync(python, ["--version"], { encoding: "utf-8", shell: false });
  return probe.error === undefined && probe.status === 0;
}

describe("parity fixtures", () => {
  it("lists exactly the files present, with no orphans and no missing files", () => {
    const directory = new URL("../fixtures/parity/", import.meta.url);
    const manifest = readManifest(new URL("../fixtures/parity/manifest.json", import.meta.url));
    const listed = manifest.cases.flatMap((entry) => entry.files);
    expect(new Set(listed).size).toBe(listed.length);
    expect(new Set(manifest.cases.map((entry) => entry.name)).size).toBe(manifest.cases.length);
    expect(listFiles(fileURLToPath(directory))).toEqual(["manifest.json", ...listed].sort());
  });

  it("covers every case kind", () => {
    const manifest = readManifest(new URL("../fixtures/parity/manifest.json", import.meta.url));
    const kinds = new Set(manifest.cases.map((entry) => entry.kind));
    expect([...kinds].sort()).toEqual(["canonical", "collection", "loader", "receipt-adapter", "validator"]);
  });

  it("records each canonical identity as the SHA-256 of its canonical bytes", () => {
    const directory = new URL("../fixtures/parity/", import.meta.url);
    const manifest = readManifest(new URL("../fixtures/parity/manifest.json", import.meta.url));
    const cases = manifest.cases.filter((entry) => entry.kind === "canonical");
    expect(cases.length).toBeGreaterThan(0);
    for (const entry of cases) {
      const bytes = readFileSync(new URL(entry.canonical ?? "", directory));
      expect(sha256(bytes), entry.name).toBe(entry.identity);
    }
  });

  it("records sorted observation ids and the collection identity of every collection", () => {
    const directory = new URL("../fixtures/parity/", import.meta.url);
    const manifest = readManifest(new URL("../fixtures/parity/manifest.json", import.meta.url));
    const cases = manifest.cases.filter((entry) => entry.canonical !== undefined && entry.kind !== "canonical");
    expect(cases.length).toBeGreaterThan(0);
    for (const entry of cases) {
      const bytes = readFileSync(new URL(entry.canonical ?? "", directory));
      expect(sha256(bytes), entry.name).toBe(entry.identity);
      const ids = entry.observation_ids ?? [];
      expect([...ids].sort(), entry.name).toEqual(ids);
      expect(new Set(ids).size, entry.name).toBe(ids.length);
      const document = JSON.parse(bytes.toString("utf-8")) as { observations: { observation_id: string }[] };
      expect(document.observations.map((observation) => observation.observation_id), entry.name).toEqual(ids);
      expect(entry.problems, entry.name).toEqual([]);
    }
  });

  it("keeps every canonical file pure ASCII", () => {
    const directory = new URL("../fixtures/parity/", import.meta.url);
    const manifest = readManifest(new URL("../fixtures/parity/manifest.json", import.meta.url));
    const files = manifest.cases.flatMap((entry) => entry.files).filter((name) => name.endsWith(".canonical"));
    expect(files.length).toBeGreaterThan(0);
    for (const name of files) {
      expect(isAscii(readFileSync(new URL(name, directory))), name).toBe(true);
    }
  });

  it("records each validator code as the prefix of its problem", () => {
    const manifest = readManifest(new URL("../fixtures/parity/manifest.json", import.meta.url));
    for (const entry of manifest.cases.filter((candidate) => candidate.kind === "validator")) {
      if (entry.exception !== undefined) {
        expect(entry.problems, entry.name).toBeNull();
        continue;
      }
      const problems = entry.problems ?? [];
      expect(entry.codes, entry.name).toEqual(problems.map((problem) => problem.split(":", 1)[0]));
      expect(entry.valid, entry.name).toBe(problems.length === 0);
    }
  });

  const python = pythonAvailable();
  it.skipIf(!python)(
    python
      ? "matches what the Python contract answers today (capture-parity --check)"
      : "matches what the Python contract answers today (SKIPPED: python3 is not on PATH)",
    () => {
      const cli = fileURLToPath(new URL("../../cli/capture-parity.ts", import.meta.url));
      const result = spawnSync(process.execPath, ["--import", "tsx", cli, "--check"], {
        cwd: fileURLToPath(new URL("../../", import.meta.url)),
        encoding: "utf-8",
        shell: false,
        timeout: 20_000,
      });
      expect(result.status, `${result.stdout}${result.stderr}`).toBe(0);
    },
  );
});
