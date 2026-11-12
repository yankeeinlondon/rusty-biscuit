// Operator routes through the Worker's public `fetch` with the real
// authenticator, against local D1 built from `migrations/`. The operator
// credential is seeded the way `operator bootstrap` seeds it (digest only).
import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import { createAuthenticator } from "../../src/auth.ts";
import { insertRepository } from "../../src/db/repositories.ts";
import { mintToken, secretDigest } from "../../src/tokens.ts";
import { createWorker, type Env } from "../../src/worker.ts";
import { db, hex } from "./helpers.ts";

const DAY_MS = 86_400_000;
let clock = 1_800_000_000_000;

const BASE_CONFIG = JSON.parse(env.SERVICE_CONFIG) as Record<string, any>;

interface Allowance {
  repository_id: string;
  owner_id: string;
  slug?: string;
}

function envWith(repositories: Allowance[]): Env {
  const config = {
    ...BASE_CONFIG,
    repositories: repositories.map((r) => ({ slug: `owner/repo-${r.repository_id}`, ...r })),
  };
  return { ...(env as unknown as Env), SERVICE_CONFIG: JSON.stringify(config) };
}

const worker = createWorker({ authenticate: createAuthenticator(), now: () => clock });

async function call(storage: Env, token: string, path: string, body?: unknown, method = "POST"): Promise<Response> {
  const init: RequestInit = { method, headers: { authorization: `Bearer ${token}` } };
  if (body !== undefined) init.body = typeof body === "string" ? body : JSON.stringify(body);
  return worker.fetch!(new Request(`https://evidence.test${path}`, init) as never, storage, {} as ExecutionContext);
}

interface ErrorEnvelope {
  error: { code: string; location: string | null };
}

async function expectError(response: Response, status: number, code: string, location?: string | null) {
  expect(response.status).toBe(status);
  const body = (await response.json()) as ErrorEnvelope;
  expect(body.error.code).toBe(code);
  if (location !== undefined) expect(body.error.location).toBe(location);
}

function repositoryId(): string {
  const digits = crypto.getRandomValues(new Uint32Array(2));
  return `${1 + (digits[0]! % 9)}${String(digits[1]).padStart(10, "0")}`;
}

/** The operator credential, inserted as bootstrap does. */
async function seedOperator(): Promise<{ token: string; id: string }> {
  const minted = mintToken();
  await db()
    .prepare(
      `INSERT INTO credentials(credential_id, kind, label, secret_sha256, issued_at_ms, rotate_at_ms, expires_at_ms)
       VALUES (?1, 'operator', 'ops', ?2, 0, ?3, ?3)`,
    )
    .bind(minted.credentialId, await secretDigest(minted.secret), 4_000_000_000_000)
    .run();
  return { token: minted.token, id: minted.credentialId };
}

async function registered(owner = "4417321"): Promise<Allowance> {
  const id = repositoryId();
  await insertRepository(db(), { repository_id: id, owner_id: owner, slug: `owner/repo-${id}`, generation: hex(32), registered_at_ms: 1 });
  return { repository_id: id, owner_id: owner };
}

interface Issued {
  credential: Record<string, any>;
  secret: string;
}

async function issue(storage: Env, operator: string, request: Record<string, unknown>): Promise<Issued> {
  const response = await call(storage, operator, "/v1/operator/credentials", request);
  expect(response.status).toBe(201);
  return (await response.json()) as Issued;
}

async function auditRows(where: string, ...bindings: string[]) {
  const rows = await db()
    .prepare(`SELECT * FROM audit_log WHERE ${where} ORDER BY seq`)
    .bind(...bindings)
    .all<Record<string, string | number | null>>();
  return rows.results;
}

describe("POST /v1/operator/repositories", () => {
  it("registers an allowlisted repository, then answers 200 for the same registration and records a rename", async () => {
    const operator = await seedOperator();
    const id = repositoryId();
    const storage = envWith([{ repository_id: id, owner_id: "4417321" }]);
    const registration = { repository_id: id, owner_id: "4417321", slug: `owner/first-${id}` };

    const created = await call(storage, operator.token, "/v1/operator/repositories", registration);
    expect(created.status).toBe(201);
    const record = (await created.json()) as Record<string, any>;
    expect(record).toMatchObject({ ...registration, registered_at_ms: clock });
    expect(record["snapshot"]).toMatchObject({ repository_id: id, retention_epoch: 0, evidence_revision: 0, observation_event_watermark: 0, location_event_watermark: 0 });
    expect(record["snapshot"].generation).toMatch(/^[0-9a-f]{32}$/);

    const again = await call(storage, operator.token, "/v1/operator/repositories", registration);
    expect(again.status).toBe(200);
    expect(await again.json()).toEqual(record);

    const renamed = await call(storage, operator.token, "/v1/operator/repositories", { ...registration, slug: `owner/second-${id}` });
    expect(renamed.status).toBe(200);
    expect(await renamed.json()).toEqual({ ...record, slug: `owner/second-${id}` });

    const audit = await auditRows("repository_id = ?1", id);
    expect(audit.map((row) => row["action"])).toEqual(["repository.register", "repository.rename"]);
    expect(JSON.parse(audit[0]!["actor"] as string)).toEqual({ kind: "credential", credential_id: operator.id });
    expect(audit[0]!["actor_credential_id"]).toBe(operator.id);
  });

  it("answers 409 for another owner, an unset stored owner, or a slug another repository holds", async () => {
    const operator = await seedOperator();
    const owned = await registered("111");
    const ownerless = repositoryId();
    await insertRepository(db(), { repository_id: ownerless, slug: `owner/repo-${ownerless}`, generation: hex(32) });
    const fresh = repositoryId();
    const storage = envWith([
      { repository_id: owned.repository_id, owner_id: "222" },
      { repository_id: ownerless, owner_id: "222" },
      { repository_id: fresh, owner_id: "222" },
    ]);
    const register = (body: unknown) => call(storage, operator.token, "/v1/operator/repositories", body);

    await expectError(await register({ repository_id: owned.repository_id, owner_id: "222", slug: "owner/x" }), 409, "identity_conflict", "/owner_id");
    await expectError(await register({ repository_id: ownerless, owner_id: "222", slug: "owner/y" }), 409, "identity_conflict", "/owner_id");
    await expectError(
      await register({ repository_id: fresh, owner_id: "222", slug: `owner/repo-${owned.repository_id}` }),
      409,
      "identity_conflict",
      "/slug",
    );
    const row = await db().prepare("SELECT 1 FROM repositories WHERE repository_id = ?1").bind(fresh).first();
    expect(row).toBeNull();
  });

  it("answers 403 for a repository or owner outside the configuration and writes nothing", async () => {
    const operator = await seedOperator();
    const id = repositoryId();
    const storage = envWith([{ repository_id: id, owner_id: "4417321" }]);
    await expectError(await call(storage, operator.token, "/v1/operator/repositories", { repository_id: id, owner_id: "9", slug: "o/r" }), 403, "denied");
    const other = repositoryId();
    await expectError(await call(storage, operator.token, "/v1/operator/repositories", { repository_id: other, owner_id: "4417321", slug: "o/r" }), 403, "denied");
    expect(await db().prepare("SELECT 1 FROM repositories WHERE repository_id IN (?1, ?2)").bind(id, other).first()).toBeNull();
    expect(await auditRows("repository_id IN (?1, ?2)", id, other)).toEqual([]);
  });

  it("refuses an invalid registration body, located", async () => {
    const operator = await seedOperator();
    const storage = envWith([]);
    await expectError(await call(storage, operator.token, "/v1/operator/repositories", { repository_id: "1", owner_id: "2", slug: "no-slash" }), 400, "invalid_input", "/slug");
  });
});

describe("POST /v1/operator/credentials", () => {
  it("issues a token that works on the next request, storing only its digest", async () => {
    const operator = await seedOperator();
    const repo = await registered();
    const storage = envWith([repo]);
    const issued = await issue(storage, operator.token, { label: "rig-linux-01", repository_ids: [repo.repository_id], roles: ["reader"] });
    expect(issued.secret).toMatch(/^ebt_[0-9a-f]{32}\.[A-Za-z0-9_-]{43}$/);
    expect(issued.credential).toEqual({
      credential_id: issued.secret.slice(4, 36),
      label: "rig-linux-01",
      repository_ids: [repo.repository_id],
      roles: ["reader"],
      issued_at_ms: clock,
      expires_at_ms: clock + 90 * DAY_MS,
      rotate_at_ms: clock + 60 * DAY_MS,
    });

    const read = await call(storage, issued.secret, `/v1/repos/${repo.repository_id}/snapshot`, undefined, "GET");
    expect([401, 403]).not.toContain(read.status);

    const stored = await db().prepare("SELECT * FROM credentials WHERE credential_id = ?1").bind(issued.credential["credential_id"]).first<Record<string, unknown>>();
    expect(stored!["secret_sha256"]).toBe(await secretDigest(issued.secret.split(".")[1]!));
    expect(JSON.stringify(stored)).not.toContain(issued.secret.split(".")[1]);

    const audit = await auditRows("target_credential_id = ?1", issued.credential["credential_id"]);
    expect(audit).toHaveLength(1);
    expect(audit[0]).toMatchObject({ action: "credential.issue", repository_id: repo.repository_id, actor_credential_id: operator.id });
  });

  it("gives a credential with empty repositories or roles nothing", async () => {
    const operator = await seedOperator();
    const repo = await registered();
    const storage = envWith([repo]);
    for (const request of [
      { label: "no-roles", repository_ids: [repo.repository_id], roles: [] },
      { label: "no-repos", repository_ids: [], roles: ["reader"] },
    ]) {
      const issued = await issue(storage, operator.token, request);
      expect(issued.credential).toMatchObject({ repository_ids: request.repository_ids, roles: request.roles });
      await expectError(await call(storage, issued.secret, `/v1/repos/${repo.repository_id}/snapshot`, undefined, "GET"), 403, "denied");
    }
  });

  it("answers 404 located at the first unregistered repository and writes nothing", async () => {
    const operator = await seedOperator();
    const repo = await registered();
    const missing = repositoryId();
    const before = await db().prepare("SELECT COUNT(*) AS n FROM credentials").first<{ n: number }>();
    await expectError(
      await call(envWith([repo]), operator.token, "/v1/operator/credentials", { label: "x", repository_ids: [repo.repository_id, missing], roles: ["reader"] }),
      404,
      "not_found",
      "/repository_ids/1",
    );
    expect(await db().prepare("SELECT COUNT(*) AS n FROM credentials").first<{ n: number }>()).toEqual(before);
  });

  it("refuses the operator role and an explicit null through the shared validator, located", async () => {
    const operator = await seedOperator();
    const storage = envWith([]);
    await expectError(await call(storage, operator.token, "/v1/operator/credentials", { label: "x", repository_ids: [], roles: ["operator"] }), 400, "invalid_input", "/roles/0");
    await expectError(
      await call(storage, operator.token, "/v1/operator/credentials", { label: "x", repository_ids: [], roles: [], expires_in_days: null }),
      400,
      "invalid_input",
      "/expires_in_days",
    );
  });

  it("shortens a replaced credential to the seven-day overlap and refuses unknown, revoked, or operator replacements", async () => {
    const operator = await seedOperator();
    const repo = await registered();
    const storage = envWith([repo]);
    const old = await issue(storage, operator.token, { label: "rig", repository_ids: [repo.repository_id], roles: ["reader"] });
    clock += 10 * DAY_MS;
    const replacement = await issue(storage, operator.token, {
      label: "rig",
      repository_ids: [repo.repository_id],
      roles: ["reader"],
      replaces: old.credential["credential_id"],
    });
    expect(replacement.credential["replaces"]).toBe(old.credential["credential_id"]);
    const shortened = await db()
      .prepare("SELECT rotate_at_ms, expires_at_ms FROM credentials WHERE credential_id = ?1")
      .bind(old.credential["credential_id"])
      .first<{ rotate_at_ms: number; expires_at_ms: number }>();
    expect(shortened).toEqual({ rotate_at_ms: clock + 7 * DAY_MS, expires_at_ms: clock + 7 * DAY_MS });
    const audit = await auditRows("target_credential_id = ?1", replacement.credential["credential_id"]);
    expect(JSON.parse(audit[0]!["detail_json"] as string)).toMatchObject({ replaces: old.credential["credential_id"] });

    const replace = (replaces: string) =>
      call(storage, operator.token, "/v1/operator/credentials", { label: "rig", repository_ids: [], roles: [], replaces });
    await expectError(await replace(hex(32)), 404, "not_found", "/replaces");
    await call(storage, operator.token, `/v1/operator/credentials/${old.credential["credential_id"]}/revoke`, { reason: "rotated" });
    await expectError(await replace(old.credential["credential_id"]), 400, "invalid_input", "/replaces");
    await expectError(await replace(operator.id), 400, "invalid_input", "/replaces");
  });
});

describe("POST /v1/operator/credentials/{id}/revoke", () => {
  it("revokes once, answers the original revocation on repeat, and audits once", async () => {
    const operator = await seedOperator();
    const storage = envWith([]);
    const issued = await issue(storage, operator.token, { label: "rig", repository_ids: [], roles: [] });
    const id = issued.credential["credential_id"] as string;
    const revokedAt = clock;
    const first = await call(storage, operator.token, `/v1/operator/credentials/${id}/revoke`, { reason: "decommissioned" });
    expect(first.status).toBe(200);
    const record = await first.json();
    expect(record).toEqual({ ...issued.credential, revoked_at_ms: revokedAt, revocation_reason: "decommissioned" });

    clock += 1_000;
    const repeat = await call(storage, operator.token, `/v1/operator/credentials/${id}/revoke`, { reason: "another reason" });
    expect(repeat.status).toBe(200);
    expect(await repeat.json()).toEqual(record);
    expect((await auditRows("target_credential_id = ?1 AND action = 'credential.revoke'", id)).length).toBe(1);
  });

  it("answers 404 for an unknown credential and 400 for a missing reason", async () => {
    const operator = await seedOperator();
    const storage = envWith([]);
    await expectError(await call(storage, operator.token, `/v1/operator/credentials/${hex(32)}/revoke`, { reason: "x" }), 404, "not_found");
    await expectError(await call(storage, operator.token, `/v1/operator/credentials/${operator.id}/revoke`, {}), 400, "invalid_input", "/reason");
  });
});

describe("POST /v1/operator/audit/query", () => {
  async function scenario() {
    const operator = await seedOperator();
    const repo = await registered();
    const storage = envWith([repo]);
    const start = clock;
    const issued: string[] = [];
    for (let index = 0; index < 3; index += 1) {
      clock += 1_000;
      const credential = await issue(storage, operator.token, { label: `rig-${index}`, repository_ids: [repo.repository_id], roles: ["reader"] });
      issued.push(credential.secret);
    }
    clock += 1_000;
    const firstId = issued[0]!.slice(4, 36);
    await call(storage, operator.token, `/v1/operator/credentials/${firstId}/revoke`, { reason: "lost" });
    const query = (body: unknown) => call(storage, operator.token, "/v1/operator/audit/query", body);
    return { operator, repo, storage, start, issued, firstId, query };
  }

  it("filters by repository, credential, action, and time", async () => {
    const { operator, repo, start, firstId, query } = await scenario();
    const page = async (body: unknown) => {
      const response = await query(body);
      expect(response.status).toBe(200);
      return (await response.json()) as { items: Record<string, any>[]; has_more: boolean; next_cursor: string | null };
    };

    const byRepository = await page({ repository_id: repo.repository_id });
    expect(byRepository.items.map((item) => item["action"])).toEqual(["credential.issue", "credential.issue", "credential.issue"]);
    expect(byRepository).toMatchObject({ has_more: false, next_cursor: null });
    expect(byRepository.items[0]).toEqual({
      sequence: expect.any(Number),
      at_ms: start + 1_000,
      action: "credential.issue",
      actor: { kind: "credential", credential_id: operator.id },
      repository_id: repo.repository_id,
      target_credential_id: firstId,
      request_id: expect.any(String),
    });

    const byTarget = await page({ credential_id: firstId });
    expect(byTarget.items.map((item) => item["action"])).toEqual(["credential.issue", "credential.revoke"]);
    expect(byTarget.items[1]).not.toHaveProperty("repository_id");

    const byActor = await page({ credential_id: operator.id, action: "credential.revoke" });
    expect(byActor.items.map((item) => item["target_credential_id"])).toEqual([firstId]);

    const window = await page({ repository_id: repo.repository_id, since_ms: start + 2_000, until_ms: start + 2_000 });
    expect(window.items.map((item) => item["at_ms"])).toEqual([start + 2_000]);
  });

  it("pages with a signed cursor and refuses a tampered or foreign one", async () => {
    const { repo, query } = await scenario();
    const seen: number[] = [];
    let cursor: string | null = null;
    do {
      const body: Record<string, unknown> = { repository_id: repo.repository_id, page_size: 1 };
      if (cursor !== null) body["cursor"] = cursor;
      const page = (await (await query(body)).json()) as { items: { sequence: number }[]; has_more: boolean; next_cursor: string | null };
      expect(page.has_more).toBe(page.next_cursor !== null);
      seen.push(...page.items.map((item) => item.sequence));
      cursor = page.next_cursor;
    } while (cursor !== null);
    expect(seen).toHaveLength(3);
    expect([...seen].sort((a, b) => a - b)).toEqual(seen);

    const first = (await (await query({ repository_id: repo.repository_id, page_size: 1 })).json()) as { next_cursor: string };
    const [payload, mac] = first.next_cursor.split(".");
    const tampered = `${payload}.${mac!.startsWith("A") ? "B" : "A"}${mac!.slice(1)}`;
    await expectError(await query({ repository_id: repo.repository_id, page_size: 1, cursor: tampered }), 400, "invalid_cursor", "/cursor");
    await expectError(await query({ repository_id: repo.repository_id, page_size: 2, cursor: first.next_cursor }), 400, "invalid_cursor", "/cursor");
    await expectError(await query({ page_size: 0 }), 400, "invalid_input", "/page_size");
    await expectError(await query({ action: null }), 400, "invalid_input", "/action");
  });

  it("never records or returns a secret or its digest", async () => {
    const { operator, issued, query } = await scenario();
    const page = await (await query({ page_size: 256 })).text();
    const rows = JSON.stringify(await auditRows("1 = 1"));
    for (const token of [...issued, operator.token]) {
      const secret = token.split(".")[1]!;
      const digest = await secretDigest(secret);
      for (const text of [page, rows]) {
        expect(text).not.toContain(secret);
        expect(text).not.toContain(digest);
      }
    }
  });
});
