// The provision command in process with a fake Wrangler that remembers what
// it was asked to create, so reruns see the earlier run's resources.
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { OperatorIo, SpawnResult } from "../../cli/operator.ts";
import { emptyAllowlists, run, withDatabaseId } from "../../cli/provision.ts";
import { PLACEHOLDER_DATABASE_ID as PLACEHOLDER, unprovisionedConfig } from "./unprovisioned-config.ts";

const DATABASE_ID = "0f9e8d7c-6b5a-4321-8fed-cba987654321";
const CREDENTIALS = { CLOUDFLARE_API_TOKEN: "cf-token-value", CLOUDFLARE_ACCOUNT_ID: "cf-account-value" };

let root: string;
beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "evidence-provision-"));
  writeFileSync(join(root, "wrangler.jsonc"), unprovisionedConfig());
});
afterEach(() => rmSync(root, { recursive: true, force: true }));

/** Cloudflare as Wrangler would report it; `fail` makes the named subcommand exit 1. */
interface Account {
  databases: { name: string; uuid: string }[];
  buckets: Set<string>;
  secrets: Set<string>;
  workerExists: boolean;
  fail?: string;
}

function fakeIo(account: Account, env: NodeJS.ProcessEnv = CREDENTIALS) {
  const calls: { args: readonly string[]; input?: string }[] = [];
  const out: string[] = [];
  const err: string[] = [];
  const ok = (stdout = ""): SpawnResult => ({ status: 0, stdout, stderr: "" });
  const no = (stderr: string): SpawnResult => ({ status: 1, stdout: "", stderr });
  const io: OperatorIo = {
    env,
    fetch: (async () => {
      throw new Error("provision must not fetch");
    }) as typeof fetch,
    spawn: (_command, argv, options) => {
      const args = argv.slice(1);
      calls.push(options?.input === undefined ? { args } : { args, input: options.input });
      const verb = args.slice(0, 3).join(" ");
      if (account.fail !== undefined && verb.startsWith(account.fail)) return no(`boom with ${CREDENTIALS.CLOUDFLARE_API_TOKEN}`);
      if (verb.startsWith("d1 list")) return ok(JSON.stringify(account.databases));
      if (verb.startsWith("d1 create")) {
        account.databases.push({ name: args[2]!, uuid: DATABASE_ID });
        return ok("Successfully created DB");
      }
      if (verb.startsWith("r2 bucket info")) return account.buckets.has(args[3]!) ? ok("{}") : no("bucket not found");
      if (verb.startsWith("r2 bucket create")) {
        account.buckets.add(args[3]!);
        return ok();
      }
      if (verb.startsWith("secret list")) {
        return account.workerExists ? ok(JSON.stringify([...account.secrets].map((name) => ({ name, type: "secret_text" })))) : no("Worker not found");
      }
      if (verb.startsWith("secret put")) {
        account.workerExists = true;
        account.secrets.add(args[2]!);
        return ok();
      }
      throw new Error(`unexpected wrangler call: ${args.join(" ")}`);
    },
    stdout: (text) => out.push(text),
    stderr: (text) => err.push(text),
    platform: process.platform,
    now: () => Date.UTC(2026, 10, 11, 12, 0, 0),
  };
  return { io, calls, stdout: () => out.join(""), stderr: () => err.join("") };
}

const empty = (): Account => ({ databases: [], buckets: new Set(), secrets: new Set(), workerExists: false });
const verbs = (calls: { args: readonly string[] }[]) => calls.map((call) => call.args.slice(0, call.args[0] === "r2" ? 3 : 2).join(" "));

describe("provision", () => {
  it("creates everything on a fresh account and writes only development's database_id", async () => {
    const account = empty();
    const fake = fakeIo(account);
    expect(await run(["--env", "development"], fake.io, root)).toBe(0);

    expect(verbs(fake.calls)).toEqual(["d1 list", "d1 create", "d1 list", "r2 bucket info", "r2 bucket create", "secret list", "secret put"]);
    const put = fake.calls.at(-1)!;
    expect(put.input).toMatch(/^[A-Za-z0-9_-]{43}$/);
    expect(put.args).not.toContain(put.input);

    const config = readFileSync(join(root, "wrangler.jsonc"), "utf8");
    expect(config).toContain(`"database_id": "${DATABASE_ID}"`);
    // restore-test keeps its placeholder, and the comments survive.
    expect(config.split(PLACEHOLDER)).toHaveLength(2);
    expect(config).toContain("// Per-principal request rates");

    expect(fake.stdout()).toContain("provisioned development:");
    expect(fake.stdout()).toContain("database_id written; commit it");
    expect(fake.stdout()).toContain("fill SERVICE_CONFIG repositories and oidc.trusted_workflows");
  });

  it("reports an already-provisioned environment and creates nothing on a rerun", async () => {
    const account = empty();
    expect(await run(["--env", "development"], fakeIo(account).io, root)).toBe(0);
    const before = readFileSync(join(root, "wrangler.jsonc"), "utf8");

    const again = fakeIo(account);
    expect(await run(["--env", "development"], again.io, root)).toBe(0);
    expect(verbs(again.calls)).toEqual(["d1 list", "r2 bucket info", "secret list"]);
    expect(readFileSync(join(root, "wrangler.jsonc"), "utf8")).toBe(before);
    expect(again.stdout()).toContain("development is already provisioned; nothing was changed:");
    expect(again.stdout()).not.toContain("created");
  });

  it("finishes a run that failed part-way, reusing what the first run created", async () => {
    const account = empty();
    account.fail = "r2 bucket create";
    const first = fakeIo(account);
    expect(await run(["--env", "development"], first.io, root)).toBe(1);
    expect(first.stderr()).toContain("wrangler r2 bucket create failed (exit 1)");
    expect(first.stderr()).toContain("<CLOUDFLARE_API_TOKEN>");
    expect(first.stderr()).not.toContain(CREDENTIALS.CLOUDFLARE_API_TOKEN);

    delete account.fail;
    const second = fakeIo(account);
    expect(await run(["--env", "development"], second.io, root)).toBe(0);
    expect(verbs(second.calls)).toEqual(["d1 list", "r2 bucket info", "r2 bucket create", "secret list", "secret put"]);
    expect(account.databases).toHaveLength(1);
  });

  it("refuses to overwrite a database_id that disagrees with Cloudflare, creating nothing", async () => {
    const path = join(root, "wrangler.jsonc");
    const other = "11111111-2222-4333-8444-555555555555";
    writeFileSync(path, withDatabaseId(readFileSync(path, "utf8"), "evidence-store-development", other));
    const fake = fakeIo({ ...empty(), databases: [{ name: "evidence-store-development", uuid: DATABASE_ID }], buckets: new Set(["evidence-store-development"]) });
    expect(await run(["--env", "development"], fake.io, root)).toBe(1);
    expect(fake.stderr()).toContain(`names database_id ${other}`);
    expect(readFileSync(path, "utf8")).toContain(other);
    expect(verbs(fake.calls)).not.toContain("secret put");
  });

  it("provisions development when no --env is given", async () => {
    const fake = fakeIo(empty());
    expect(await run([], fake.io, root)).toBe(0);
    expect(fake.calls.find((call) => call.args[0] === "d1" && call.args[1] === "create")?.args[2]).toBe("evidence-store-development");
    expect(fake.calls.at(-1)!.args).toContain("development");
    expect(fake.stdout()).toContain("just evidence-store deploy --manifest /abs/deploy-development.json");
  });

  it("names restore-test explicitly in its next step", async () => {
    const fake = fakeIo(empty());
    expect(await run(["--env", "restore-test"], fake.io, root)).toBe(0);
    expect(fake.stdout()).toContain("just evidence-store deploy --env restore-test --manifest");
  });

  it("names missing credentials and rejects an unknown environment without calling Wrangler", async () => {
    const none = fakeIo(empty(), {});
    expect(await run(["--env", "development"], none.io, root)).toBe(2);
    expect(none.stderr()).toContain("CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID");
    const unknown = fakeIo(empty());
    expect(await run(["--env", "production"], unknown.io, root)).toBe(2);
    const stray = fakeIo(empty());
    expect(await run(["--environment", "development"], stray.io, root)).toBe(2);
    expect(stray.stderr()).toContain("usage:");
    expect([...none.calls, ...unknown.calls, ...stray.calls]).toEqual([]);
  });
});

describe("emptyAllowlists", () => {
  it("names each empty allowlist and none once both are filled", () => {
    const text = unprovisionedConfig();
    expect(emptyAllowlists(text, "development")).toEqual(["repositories", "oidc.trusted_workflows"]);
    const filled = text.replace('\\"repositories\\":[]', '\\"repositories\\":[{}]').replace('\\"trusted_workflows\\":[]', '\\"trusted_workflows\\":[{}]');
    expect(emptyAllowlists(filled, "development")).toEqual([]);
  });
});

describe("progress", () => {
  it("says what each step is for: checks only on a rerun, creations on a fresh account", async () => {
    const account = empty();
    const fresh = fakeIo(account);
    expect(await run([], fresh.io, root)).toBe(0);
    expect(fresh.stderr().trim().split("\n")).toEqual([
      "provision: checking D1 database evidence-store-development…",
      "provision: creating D1 database evidence-store-development…",
      "provision: reading the new database's ID…",
      "provision: checking R2 bucket evidence-store-development…",
      "provision: creating R2 bucket evidence-store-development…",
      `provision: writing database_id ${DATABASE_ID} for development into wrangler.jsonc`,
      "provision: checking secret CURSOR_HMAC_KEY on Worker evidence-store-development…",
      "provision: setting secret CURSOR_HMAC_KEY on Worker evidence-store-development…",
    ]);

    const rerun = fakeIo(account);
    expect(await run([], rerun.io, root)).toBe(0);
    expect(rerun.stderr().trim().split("\n")).toEqual([
      "provision: checking D1 database evidence-store-development…",
      "provision: checking R2 bucket evidence-store-development…",
      "provision: checking secret CURSOR_HMAC_KEY on Worker evidence-store-development…",
    ]);
  });
});
