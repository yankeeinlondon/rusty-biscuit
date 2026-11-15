// The operator CLI's exported commands in process (fake fetch and spawn),
// plus spawned runs only for the shared-loader wiring of the grant file: one
// control and two malformed documents.
import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, statSync, writeFileSync, existsSync } from "node:fs";
import { createServer, type Server } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { bootstrapRecord, bootstrapSql, databaseName, run, stripJsonComments, type OperatorIo, type SpawnResult } from "../../cli/operator.ts";
import { mintToken, secretDigest } from "../../src/tokens.ts";

const OPERATOR_TOKEN = `ebt_${"1".repeat(32)}.${"o".repeat(43)}`;
const ISSUED_SECRET = `ebt_${"2".repeat(32)}.${"s".repeat(43)}`;
const CREDENTIAL = {
  credential_id: "2".repeat(32),
  label: "rig-linux-01",
  repository_ids: ["873451209"],
  roles: ["producer", "reader"],
  issued_at_ms: 1,
  expires_at_ms: 3,
  rotate_at_ms: 2,
};

let dir: string;
beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "evidence-operator-"));
  writeFileSync(join(dir, "operator.token"), `${OPERATOR_TOKEN}\n`);
});
afterEach(() => rmSync(dir, { recursive: true, force: true }));

interface Call {
  url: string;
  init: RequestInit | undefined;
}

function fakeIo(answer: (call: Call) => Response, env: NodeJS.ProcessEnv = {}, spawned: SpawnResult = { status: 0, stdout: "", stderr: "" }) {
  const calls: Call[] = [];
  const spawns: { command: string; args: readonly string[] }[] = [];
  const out: string[] = [];
  const err: string[] = [];
  const io: OperatorIo = {
    env,
    fetch: (async (url: string, init?: RequestInit) => {
      const call = { url: String(url), init };
      calls.push(call);
      return answer(call);
    }) as typeof fetch,
    spawn: (command, args) => {
      spawns.push({ command, args });
      return spawned;
    },
    stdout: (text) => out.push(text),
    stderr: (text) => err.push(text),
    platform: process.platform,
    now: () => 1_800_000_000_000,
  };
  return { io, calls, spawns, stdout: () => out.join(""), stderr: () => err.join("") };
}

const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

function grantFile(overrides: Record<string, unknown> = {}): { grant: string; secret: string } {
  const secret = join(dir, "rig.token");
  const grant = join(dir, "grant.json");
  writeFileSync(grant, JSON.stringify({ label: "rig-linux-01", repository_ids: ["873451209"], roles: ["producer", "reader"], secret_file: secret, ...overrides }));
  return { grant, secret };
}

const common = () => ["--endpoint", "https://evidence.test/", "--token-file", join(dir, "operator.token")];

describe("operator issue", () => {
  it("writes the secret to an owner-only file and prints only the public record", async () => {
    const { grant, secret } = grantFile();
    const fake = fakeIo(() => json({ credential: CREDENTIAL, secret: ISSUED_SECRET }, 201));
    expect(await run(["issue", ...common(), "--grant", grant], fake.io)).toBe(0);

    expect(fake.calls).toHaveLength(1);
    expect(fake.calls[0]!.url).toBe("https://evidence.test/v1/operator/credentials");
    expect((fake.calls[0]!.init!.headers as Record<string, string>)["authorization"]).toBe(`Bearer ${OPERATOR_TOKEN}`);
    expect(JSON.parse(fake.calls[0]!.init!.body as string)).toEqual({
      label: "rig-linux-01",
      repository_ids: ["873451209"],
      roles: ["producer", "reader"],
      expires_in_days: 90,
    });
    expect(readFileSync(secret, "utf8")).toBe(`${ISSUED_SECRET}\n`);
    if (process.platform !== "win32") expect(statSync(secret).mode & 0o777).toBe(0o600);
    expect(JSON.parse(fake.stdout())).toEqual(CREDENTIAL);
    for (const text of [fake.stdout(), fake.stderr()]) {
      expect(text).not.toContain(ISSUED_SECRET);
      expect(text).not.toContain(OPERATOR_TOKEN);
    }
  });

  it("refuses an existing secret_file before any request", async () => {
    const { grant, secret } = grantFile();
    writeFileSync(secret, "keep me");
    const fake = fakeIo(() => json({}, 500));
    expect(await run(["issue", ...common(), "--grant", grant], fake.io)).toBe(1);
    expect(fake.calls).toEqual([]);
    expect(fake.stderr()).toContain("already exists");
    expect(readFileSync(secret, "utf8")).toBe("keep me");
  });

  it("refuses an invalid grant field before any request, located", async () => {
    const { grant, secret } = grantFile({ roles: ["operator"] });
    const fake = fakeIo(() => json({}, 500));
    expect(await run(["issue", ...common(), "--grant", grant], fake.io)).toBe(1);
    expect(fake.calls).toEqual([]);
    expect(fake.stderr()).toContain('"/roles/0"');
    expect(existsSync(secret)).toBe(false);
  });

  it("prints the API error's code, message, location, and request ID and writes no secret", async () => {
    const { grant, secret } = grantFile();
    const fake = fakeIo(() =>
      json({ error: { code: "not_found", message: "repository 873451209 is not registered", location: "/repository_ids/0", retryable: false, retry_after_ms: null }, request_id: "req-1" }, 404),
    );
    expect(await run(["issue", ...common(), "--grant", grant], fake.io)).toBe(1);
    const stderr = fake.stderr();
    for (const part of ["not_found", "repository 873451209 is not registered", "/repository_ids/0", "req-1"]) expect(stderr).toContain(part);
    expect(stderr).not.toContain(OPERATOR_TOKEN);
    expect(existsSync(secret)).toBe(false);
  });
});

describe("operator register", () => {
  it("registers by GitHub's stable IDs, not the slug", async () => {
    const fake = fakeIo(
      (call) =>
        call.url.startsWith("https://api.github.com/")
          ? json({ id: 873451209, full_name: "Example-Owner/rusty-biscuit", owner: { id: 4417321 } })
          : json({ repository_id: "873451209" }, 201),
      { GH_TOKEN: "gh-token" },
    );
    expect(await run(["register", ...common(), "--slug", "example-owner/rusty-biscuit"], fake.io)).toBe(0);
    expect(fake.calls.map((call) => call.url)).toEqual([
      "https://api.github.com/repos/example-owner/rusty-biscuit",
      "https://evidence.test/v1/operator/repositories",
    ]);
    expect((fake.calls[0]!.init!.headers as Record<string, string>)["authorization"]).toBe("Bearer gh-token");
    expect(JSON.parse(fake.calls[1]!.init!.body as string)).toEqual({
      repository_id: "873451209",
      owner_id: "4417321",
      slug: "Example-Owner/rusty-biscuit",
    });
  });

  it("exits 2 naming GITHUB_TOKEN and GH_TOKEN when neither is set", async () => {
    const fake = fakeIo(() => json({}));
    expect(await run(["register", ...common(), "--slug", "o/r"], fake.io)).toBe(2);
    expect(fake.stderr()).toContain("GITHUB_TOKEN or GH_TOKEN");
    expect(fake.calls).toEqual([]);
  });
});

describe("operator revoke", () => {
  it("posts the reason to the credential's revoke route", async () => {
    const fake = fakeIo(() => json({ ...CREDENTIAL, revoked_at_ms: 5, revocation_reason: "lost" }));
    expect(await run(["revoke", ...common(), "--credential", CREDENTIAL.credential_id, "--reason", "lost"], fake.io)).toBe(0);
    expect(fake.calls[0]!.url).toBe(`https://evidence.test/v1/operator/credentials/${CREDENTIAL.credential_id}/revoke`);
    expect(JSON.parse(fake.calls[0]!.init!.body as string)).toEqual({ reason: "lost" });
  });
});

describe("operator bootstrap", () => {
  const cloudflare = { CLOUDFLARE_API_TOKEN: "t", CLOUDFLARE_ACCOUNT_ID: "a" };

  it("exits 2 naming both Cloudflare variables and writes nothing", async () => {
    const tokenFile = join(dir, "new-operator.token");
    const fake = fakeIo(() => json({}));
    expect(await run(["bootstrap", "--env", "development", "--label", "ken", "--token-file", tokenFile], fake.io)).toBe(2);
    expect(fake.stderr()).toContain("CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID");
    expect(fake.spawns).toEqual([]);
    expect(existsSync(tokenFile)).toBe(false);
  });

  it("writes the token file, then inserts only the digest through Wrangler", async () => {
    const tokenFile = join(dir, "new-operator.token");
    const fake = fakeIo(() => json({}), cloudflare);
    expect(await run(["bootstrap", "--env", "restore-test", "--label", "ken", "--token-file", tokenFile], fake.io)).toBe(0);
    const token = readFileSync(tokenFile, "utf8").trim();
    const secret = token.split(".")[1]!;
    if (process.platform !== "win32") expect(statSync(tokenFile).mode & 0o777).toBe(0o600);
    const [call] = fake.spawns;
    expect(call!.command).toBe(process.execPath);
    expect(call!.args.slice(1, 9)).toEqual(["d1", "execute", "evidence-store-restore-test", "--env", "restore-test", "--remote", "--yes", "--config"]);
    const sql = call!.args[call!.args.indexOf("--command") + 1]!;
    expect(sql).toContain(await secretDigest(secret));
    expect(call!.args.join(" ")).not.toContain(secret);
    expect(fake.stdout() + fake.stderr()).not.toContain(secret);
  });

  it("bootstraps development when no --env is given", async () => {
    const tokenFile = join(dir, "new-operator.token");
    const fake = fakeIo(() => json({}), cloudflare);
    expect(await run(["bootstrap", "--label", "ken", "--token-file", tokenFile], fake.io)).toBe(0);
    expect(fake.spawns[0]!.args.slice(1, 5)).toEqual(["d1", "execute", "evidence-store-development", "--env"]);
    expect(fake.spawns[0]!.args[5]).toBe("development");
  });

  it("removes the token file when Wrangler fails", async () => {
    const tokenFile = join(dir, "new-operator.token");
    const fake = fakeIo(() => json({}), cloudflare, { status: 1, stdout: "", stderr: "Authentication error" });
    expect(await run(["bootstrap", "--env", "development", "--label", "ken", "--token-file", tokenFile], fake.io)).toBe(1);
    expect(existsSync(tokenFile)).toBe(false);
    expect(fake.stderr()).toContain("Authentication error");
  });

  it("builds SQL with the digest and an operator.bootstrap audit row, never the secret", async () => {
    const minted = mintToken();
    const digest = await secretDigest(minted.secret);
    const sql = bootstrapSql(
      bootstrapRecord({ credentialId: minted.credentialId, digest, label: "ken's laptop", nowMs: 1_000, expiresInDays: 90, requestId: "bootstrap-1" }),
    );
    expect(sql).toContain(digest);
    expect(sql).toContain("'ken''s laptop'");
    expect(sql).toContain("'operator.bootstrap'");
    expect(sql).toContain(`'{"kind":"credential","credential_id":"${minted.credentialId}"}'`);
    expect(sql).not.toContain(minted.secret);
  });

  it("reads each environment's database name from wrangler.jsonc, comments and URLs intact", () => {
    const text = readFileSync(new URL("../../wrangler.jsonc", import.meta.url), "utf8");
    expect(databaseName(text, "development")).toBe("evidence-store-development");
    expect(databaseName(text, "restore-test")).toBe("evidence-store-restore-test");
    expect(JSON.parse(stripJsonComments(`{"url":"https://x/y" /* c */, // tail\n"n":1}`))).toEqual({ url: "https://x/y", n: 1 });
  });
});

describe("operator issue (spawned, grant file loader wiring)", () => {
  const script = fileURLToPath(new URL("../../cli/operator.ts", import.meta.url));
  const tsx = fileURLToPath(new URL("../../node_modules/tsx/dist/cli.mjs", import.meta.url));
  let server: Server;
  let endpoint: string;

  beforeEach(async () => {
    server = createServer((_request, response) => {
      response.writeHead(201, { "content-type": "application/json" });
      response.end(JSON.stringify({ credential: CREDENTIAL, secret: ISSUED_SECRET }));
    });
    await new Promise<void>((done) => server.listen(0, "127.0.0.1", done));
    const address = server.address() as { port: number };
    endpoint = `http://127.0.0.1:${address.port}`;
  });
  afterEach(() => new Promise<void>((done) => server.close(() => done())));

  function cli(grant: string): Promise<{ status: number | null; stdout: string; stderr: string }> {
    return new Promise((done) => {
      const child = spawn(process.execPath, [tsx, script, "issue", "--endpoint", endpoint, "--token-file", join(dir, "operator.token"), "--grant", grant]);
      let stdout = "";
      let stderr = "";
      child.stdout.on("data", (chunk) => (stdout += chunk));
      child.stderr.on("data", (chunk) => (stderr += chunk));
      child.on("close", (status) => done({ status, stdout, stderr }));
    });
  }

  it("issues from a valid grant file", async () => {
    const { grant, secret } = grantFile();
    const result = await cli(grant);
    expect(result.status).toBe(0);
    expect(JSON.parse(result.stdout)).toEqual(CREDENTIAL);
    expect(readFileSync(secret, "utf8")).toBe(`${ISSUED_SECRET}\n`);
  });

  for (const [name, edit, location] of [
    ["a duplicate key", (text: string) => text.replace('{"label":"rig-linux-01"', '{"label":"rig-linux-01","label":"rig-linux-02"'), '"/label"'],
    ["trailing garbage", (text: string) => `${text} garbage`, '""'],
  ] as const) {
    it(`refuses a grant file with ${name}, located, without creating the secret file`, async () => {
      const { grant, secret } = grantFile();
      writeFileSync(grant, edit(readFileSync(grant, "utf8")));
      const result = await cli(grant);
      expect(result.status).not.toBe(0);
      expect(result.stderr).toContain(`invalid at ${location}`);
      expect(existsSync(secret)).toBe(false);
    });
  }
});
