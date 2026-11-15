// The operator CLI spawned for real, only for the backup manifest's shared
// loader wiring: one control (`verify-backup` on the frozen backup fixture)
// and two malformed manifests. Everything else runs in process
// (`recovery.test.ts`).
import { spawn } from "node:child_process";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

const script = fileURLToPath(new URL("../../cli/operator.ts", import.meta.url));
const tsx = fileURLToPath(new URL("../../node_modules/tsx/dist/cli.mjs", import.meta.url));
const BACKUP_ID = "0123456789abcdef0123456789abcdef";
const fixture = fileURLToPath(new URL(`../fixtures/recovery/backup/${BACKUP_ID}`, import.meta.url));

let dir: string;
beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "evidence recovery cli "));
});
afterEach(() => rmSync(dir, { recursive: true, force: true }));

function cli(args: string[]): Promise<{ status: number | null; stdout: string; stderr: string }> {
  return new Promise((done) => {
    const child = spawn(process.execPath, [tsx, script, ...args], { stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => (stdout += chunk));
    child.stderr.on("data", (chunk) => (stderr += chunk));
    child.on("close", (status) => done({ status, stdout, stderr }));
  });
}

/** A copy of the fixture backup whose manifest text is `edit`ed. */
function backupWith(edit: (text: string) => string): string {
  const copy = join(dir, BACKUP_ID);
  cpSync(fixture, copy, { recursive: true });
  const manifest = join(copy, "manifest.json");
  writeFileSync(manifest, edit(readFileSync(manifest, "utf8")));
  return copy;
}

describe("operator verify-backup (spawned, manifest loader wiring)", () => {
  it("verifies the frozen fixture backup (control)", async () => {
    const result = await cli(["verify-backup", "--backup", fixture]);
    expect(result.status, result.stderr).toBe(0);
    expect(JSON.parse(result.stdout)).toMatchObject({ backup_id: BACKUP_ID, ok: true, checked_files: 2, problems: [] });
  });

  it("refuses a manifest with a duplicate key, located", async () => {
    const result = await cli(["verify-backup", "--backup", backupWith((text) => text.replace('"migration": 7,', '"migration": 7, "migration": 7,'))]);
    expect(result.status).toBe(2);
    expect(result.stderr).toMatch(/invalid at "\/migration": duplicate-key/);
  });

  it("refuses a manifest with a wrongly typed field, located", async () => {
    const result = await cli(["verify-backup", "--backup", backupWith((text) => text.replace('"created_at_ms": 1790000100000', '"created_at_ms": "1790000100000"'))]);
    expect(result.status).toBe(2);
    expect(result.stderr).toMatch(/invalid at "\/created_at_ms"/);
  });
});
