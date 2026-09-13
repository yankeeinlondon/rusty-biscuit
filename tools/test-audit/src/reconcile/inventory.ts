/**
 * Inventory markdown cross-check.
 *
 * Reads the family index out of `inventory.md` and checks it against the
 * declared families and computed counts.
 */
import { MalformedInput, type Violation } from "../errors.ts";
import type { Family } from "./families.ts";

export interface InventoryRow {
  id: string;
  identities: number;
  disposition: string;
}

const DISPOSITIONS = ["satisfactory", "remediation in this fix", "follow-up"];

export function parseInventoryFamilyIndex(markdown: string): InventoryRow[] {
  const lines = markdown.split("\n");
  const rows: InventoryRow[] = [];
  let cols: { id: number; identities: number; disposition: number } | null = null;
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed.startsWith("|")) {
      cols = null;
      continue;
    }
    const cells = splitTableRow(trimmed);
    if (cols === null) {
      const lower = cells.map((c) => c.toLowerCase());
      const id = lower.indexOf("family");
      const identities = lower.indexOf("identities");
      const disposition = lower.indexOf("disposition");
      if (id >= 0 && identities >= 0 && disposition >= 0) cols = { id, identities, disposition };
      continue;
    }
    if (cells.every((c) => /^:?-{2,}:?$/.test(c))) continue;
    const id = cells[cols.id]?.replace(/`/g, "").trim();
    if (!id) continue;
    const countText = cells[cols.identities]?.trim() ?? "";
    if (!/^\d+$/.test(countText)) {
      throw new MalformedInput(`inventory family index: row ${id} has a non-numeric identity count ${JSON.stringify(countText)}`);
    }
    rows.push({ id, identities: Number(countText), disposition: cells[cols.disposition]?.trim() ?? "" });
  }
  return rows;
}

function splitTableRow(line: string): string[] {
  return line
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((c) => c.trim());
}

export function checkInventory(rows: InventoryRow[], families: Family[], counts: Map<string, number>): Violation[] {
  const violations: Violation[] = [];
  const declared = new Set(families.map((f) => f.id));
  const listed = new Set(rows.map((r) => r.id));
  for (const id of declared) {
    if (!listed.has(id)) {
      violations.push({ kind: "inventory-drift", detail: `family ${id} is declared in families.json but has no inventory row` });
    }
  }
  for (const row of rows) {
    if (!declared.has(row.id)) {
      violations.push({ kind: "inventory-drift", detail: `inventory row ${row.id} names no declared family` });
      continue;
    }
    const expected = counts.get(row.id) ?? 0;
    if (row.identities !== expected) {
      violations.push({
        kind: "inventory-drift",
        detail: `family ${row.id}: inventory says ${row.identities} identities, the captures say ${expected}`,
      });
    }
    if (row.disposition.length === 0) {
      violations.push({ kind: "missing-disposition", detail: `family ${row.id} has an empty disposition` });
    } else if (!DISPOSITIONS.some((d) => row.disposition.toLowerCase().startsWith(d))) {
      violations.push({
        kind: "missing-disposition",
        detail: `family ${row.id}: disposition ${JSON.stringify(row.disposition)} is not one of ${DISPOSITIONS.join(" / ")}`,
      });
    }
  }
  return violations;
}
