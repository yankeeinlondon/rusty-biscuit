// The deploy command in process with a fake spawn, plus one spawned run for
// the missing-credential wiring.
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { parseVersionId, run } from "../../cli/deploy.ts";
import type { OperatorIo, SpawnResult } from "../../cli/operator.ts";
import { PLACEHOLDER_DATABASE_ID, unprovisionedConfig } from "./unprovisioned-config.ts";

const PACKAGE_ROOT = fileURLToPath(new URL("../..", import.meta.url));
const DATABASE_ID = "0f9e8d7c-6b5a-4321-8fed-cba987654321";
const VERSION_ID = "1a2b3c4d-5e6f-4789-8abc-def012345678";
const CREDENTIALS = { CLOUDFLARE_API_TOKEN: "cf-token-value", CLOUDFLARE_ACCOUNT_ID: "cf-account-value" };
const MIGRATIONS: Record<string, string> = { "0001_first.sql": "CREATE TABLE a(x);\n", "0002_second.sql": "CREATE TABLE b(y);\n" };

let root: string;
beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "evidence-deploy-"));
  writeFileSync(join(root, "package.json"), readFileSync(join(PACKAGE_ROOT, "package.json")));
  mkdirSync(join(root, "migrations"));
  for (const [name, sql] of Object.entries(MIGRATIONS)) writeFileSync(join(root, "migrations", name), sql);
  writeFileSync(join(root, "migrations", "README.txt"), "not a migration");
});
afterEach(() => rmSync(root, { recursive: true, force: true }));

/** The checked-in configuration, with `development` provisioned when `provisioned` is true. */
function writeConfig(provisioned: boolean): void {
  const text = unprovisionedConfig();
  // The first placeholder belongs to `development`, the second to `restore-test`.
  writeFileSync(join(root, "wrangler.jsonc"), provisioned ? text.replace(PLACEHOLDER_DATABASE_ID, DATABASE_ID) : text);
}

function fakeIo(env: NodeJS.ProcessEnv, answers: SpawnResult[] = []) {
  const spawns: { command: string; args: readonly string[] }[] = [];
  const out: string[] = [];
  const err: string[] = [];
  const io: OperatorIo = {
    env,
    fetch: (async () => {
      throw new Error("deploy must not fetch");
    }) as typeof fetch,
    spawn: (command, args) => {
      spawns.push({ command, args });
      return answers[spawns.length - 1] ?? { status: 0, stdout: "", stderr: "" };
    },
    stdout: (text) => out.push(text),
    stderr: (text) => err.push(text),
    platform: process.platform,
    now: () => Date.UTC(2026, 10, 11, 12, 0, 0),
  };
  return { io, spawns, stdout: () => out.join(""), stderr: () => err.join("") };
}

describe("deploy preconditions", () => {
  it("names each missing credential and spawns nothing", async () => {
    writeConfig(true);
    const fake = fakeIo({ CLOUDFLARE_ACCOUNT_ID: "a" });
    expect(await run(["--env", "development"], fake.io, root)).toBe(2);
    expect(fake.stderr()).toContain("set CLOUDFLARE_API_TOKEN (no interactive login");
    expect(fake.spawns).toEqual([]);

    const none = fakeIo({});
    expect(await run(["--env", "development"], none.io, root)).toBe(2);
    expect(none.stderr()).toContain("CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID");
  });

  it("refuses the placeholder database ID, pointing at provision, spawning nothing", async () => {
    writeConfig(false);
    const fake = fakeIo(CREDENTIALS);
    expect(await run(["--env", "development"], fake.io, root)).toBe(2);
    const message = fake.stderr();
    expect(message).toContain("placeholder database_id for development");
    expect(message).toContain("just evidence-store provision --env development");
    expect(fake.spawns).toEqual([]);
  });

  it("rejects an unknown environment and a relative manifest path before spawning", async () => {
    writeConfig(true);
    const unknown = fakeIo(CREDENTIALS);
    expect(await run(["--env", "production"], unknown.io, root)).toBe(2);
    const relative = fakeIo(CREDENTIALS);
    expect(await run(["--env", "development", "--manifest", "out.json"], relative.io, root)).toBe(2);
    expect(relative.stderr()).toContain("--manifest must be an absolute path");
    expect([...unknown.spawns, ...relative.spawns]).toEqual([]);
  });
});

describe("deploy", () => {
  it("applies migrations, then deploys, and writes a redacted manifest", async () => {
    writeConfig(true);
    const output = `Uploaded evidence-store-development\nDeployed evidence-store-development triggers\nCurrent Version ID: ${VERSION_ID}\n`;
    const fake = fakeIo(CREDENTIALS, [
      { status: 0, stdout: "Migrations applied", stderr: "" },
      { status: 0, stdout: output, stderr: "" },
    ]);
    const manifestPath = join(root, "manifest.json");
    expect(await run(["--env", "development", "--manifest", manifestPath], fake.io, root)).toBe(0);

    const wrangler = join(root, "node_modules", "wrangler", "bin", "wrangler.js");
    const config = join(root, "wrangler.jsonc");
    expect(fake.spawns).toEqual([
      {
        command: process.execPath,
        args: [wrangler, "d1", "migrations", "apply", "evidence-store-development", "--remote", "--env", "development", "--config", config],
      },
      { command: process.execPath, args: [wrangler, "deploy", "--env", "development", "--config", config] },
    ]);

    const text = readFileSync(manifestPath, "utf8");
    const sha = (sql: string) => createHash("sha256").update(sql).digest("hex");
    const wranglerVersion = (JSON.parse(readFileSync(join(PACKAGE_ROOT, "package.json"), "utf8")) as { devDependencies: Record<string, string> }).devDependencies["wrangler"];
    expect(JSON.parse(text)).toEqual({
      manifest_version: 1,
      environment: "development",
      worker_name: "evidence-store-development",
      d1: { name: "evidence-store-development", id: DATABASE_ID },
      r2_bucket: "evidence-store-development",
      migrations: [
        { file: "0001_first.sql", sha256: sha(MIGRATIONS["0001_first.sql"]!) },
        { file: "0002_second.sql", sha256: sha(MIGRATIONS["0002_second.sql"]!) },
      ],
      worker_version_id: VERSION_ID,
      endpoint: null,
      wrangler_version: wranglerVersion,
      deployed_at: "2026-11-11T12:00:00.000Z",
    });
    for (const secret of Object.values(CREDENTIALS)) expect(text + fake.stdout() + fake.stderr()).not.toContain(secret);
    expect(fake.stdout()).toBe("");
  });

  it("deploys development when no --env is given", async () => {
    writeConfig(true);
    const fake = fakeIo(CREDENTIALS);
    expect(await run([], fake.io, root)).toBe(0);
    expect(fake.spawns.map((spawn) => spawn.args.slice(1, 4).join(" "))).toEqual(["d1 migrations apply", "deploy --env development"]);
    expect(JSON.parse(fake.stdout()).environment).toBe("development");
  });

  it("prints the manifest to stdout without --manifest and records a missing version as null", async () => {
    writeConfig(true);
    const fake = fakeIo(CREDENTIALS);
    expect(await run(["--env", "development"], fake.io, root)).toBe(0);
    expect(JSON.parse(fake.stdout())).toMatchObject({ environment: "development", worker_version_id: null });
  });

  it("stops before deploying when the migrations step fails, without echoing credentials", async () => {
    writeConfig(true);
    const fake = fakeIo(CREDENTIALS, [{ status: 1, stdout: "", stderr: `auth failed for cf-account-value` }]);
    expect(await run(["--env", "development"], fake.io, root)).toBe(1);
    expect(fake.spawns).toHaveLength(1);
    expect(fake.stderr()).toContain("wrangler d1 migrations apply failed (exit 1); the Worker was not deployed");
    expect(fake.stderr()).not.toContain("cf-account-value");
    expect(fake.stdout()).toBe("");
  });

  it("reports a failed deploy step after applied migrations", async () => {
    writeConfig(true);
    const fake = fakeIo(CREDENTIALS, [{ status: 0, stdout: "", stderr: "" }, { status: 3, stdout: "", stderr: "boom" }]);
    expect(await run(["--env", "development"], fake.io, root)).toBe(1);
    expect(fake.stderr()).toContain("wrangler deploy failed (exit 3); the migrations were applied but the Worker was not deployed");
  });

  it("reads the version ID from Wrangler's deploy output", () => {
    expect(parseVersionId(`Current Version ID: ${VERSION_ID.toUpperCase()}`)).toBe(VERSION_ID);
    expect(parseVersionId("no version here")).toBeNull();
  });
});

describe("deploy CLI wiring", () => {
  it("exits 2 naming the missing credentials when spawned", () => {
    const tsx = fileURLToPath(new URL("../../node_modules/tsx/dist/cli.mjs", import.meta.url));
    const script = fileURLToPath(new URL("../../cli/deploy.ts", import.meta.url));
    const environment = { ...process.env };
    delete environment["CLOUDFLARE_API_TOKEN"];
    delete environment["CLOUDFLARE_ACCOUNT_ID"];
    const result = spawnSync(process.execPath, [tsx, script, "--env", "development"], { env: environment, encoding: "utf8", timeout: 30_000 });
    expect(result.status).toBe(2);
    expect(result.stderr).toContain("CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID");
    expect(result.stdout).toBe("");
  });
});
