// Authentication and authorization through the Worker's public `fetch` with
// the real authenticator: host tokens checked against local D1, and GitHub
// OIDC tokens signed at test time and verified against a local JWKS (GitHub
// is never contacted). Every refusal is checked for its status, code, and
// zero evidence writes.
import { env } from "cloudflare:workers";
import { createLocalJWKSet, exportJWK, generateKeyPair, SignJWT, type JWTVerifyGetKey } from "jose";
import { beforeAll, describe, expect, it } from "vitest";
import { createAuthenticator, publisherKey, type Publisher } from "../../src/auth.ts";
import { OPERATIONS, type OperationId } from "../../src/contracts/api.ts";
import { canonical, identity, loadStrict, type JsonObject } from "../../src/contracts/index.ts";
import { insertRepository } from "../../src/db/repositories.ts";
import { mintToken, secretDigest } from "../../src/tokens.ts";
import { createWorker, type Env } from "../../src/worker.ts";
import rigFixture from "../fixtures/parity/collection-rig.json?raw";
import { db, hex } from "./helpers.ts";

const DAY_MS = 86_400_000;
const OWNER = "4417321";
const SLUG = "example-owner/rusty-biscuit";
const WORKFLOW = ".github/workflows/evidence-store-demo.yml";
const ISSUER = "https://token.actions.githubusercontent.com";
const AUDIENCE = "rusty-biscuit-evidence-store";
let clock = 1_800_000_000_000;

const BASE_CONFIG = JSON.parse(env.SERVICE_CONFIG) as Record<string, any>;
/** Every token minted in this file, for the redaction checks. */
const minted: string[] = [];

let signingKey: CryptoKey;
let foreignKey: CryptoKey;
let localKeys: JWTVerifyGetKey;
const jwks: JWTVerifyGetKey = (header, token) => localKeys(header, token);

beforeAll(async () => {
  const pair = await generateKeyPair("RS256", { extractable: true });
  signingKey = pair.privateKey;
  foreignKey = (await generateKeyPair("RS256")).privateKey;
  localKeys = createLocalJWKSet({ keys: [{ ...(await exportJWK(pair.publicKey)), kid: "test-key", alg: "RS256", use: "sig" }] });
});

interface Allowance {
  repository_id: string;
  owner_id: string;
}

interface Trusted {
  repository_id: string;
  workflow?: string;
  ref?: string;
  event_name?: string;
  roles?: string[];
}

function envWith(repositories: Allowance[], trusted: Trusted[] = [], overrides: Partial<Record<keyof Env, unknown>> = {}): Env {
  const config = {
    ...BASE_CONFIG,
    repositories: repositories.map((r) => ({ slug: SLUG, ...r })),
    oidc: {
      ...BASE_CONFIG["oidc"],
      trusted_workflows: trusted.map((t) => ({ workflow: WORKFLOW, ref: "refs/heads/main", event_name: "workflow_dispatch", roles: ["producer", "reader"], ...t })),
    },
  };
  return { ...(env as unknown as Env), SERVICE_CONFIG: JSON.stringify(config), ...overrides } as Env;
}

const worker = createWorker({ authenticate: createAuthenticator({ jwks }), now: () => clock });

async function call(storage: Env, authorization: string | null, path: string, init: { method?: string; body?: string } = {}): Promise<Response> {
  const headers: Record<string, string> = authorization === null ? {} : { authorization };
  const request = new Request(`https://evidence.test${path}`, { method: init.method ?? "GET", headers, ...(init.body === undefined ? {} : { body: init.body }) });
  return worker.fetch!(request as never, storage, {} as ExecutionContext);
}

const bearer = (token: string) => `Bearer ${token}`;

interface ErrorEnvelope {
  error: { code: string; message: string; location: string | null; retryable: boolean; retry_after_ms: number | null };
}

async function expectError(response: Response, status: number, code: string): Promise<ErrorEnvelope> {
  const text = await response.text();
  expect({ status: response.status, code: (JSON.parse(text) as ErrorEnvelope).error.code }).toEqual({ status, code });
  return JSON.parse(text) as ErrorEnvelope;
}

function repositoryId(): string {
  const digits = crypto.getRandomValues(new Uint32Array(2));
  return `${1 + (digits[0]! % 9)}${String(digits[1]).padStart(10, "0")}`;
}

async function registered(owner = OWNER): Promise<Allowance> {
  const id = repositoryId();
  await insertRepository(db(), { repository_id: id, owner_id: owner, slug: `owner/repo-${id}`, generation: hex(32), registered_at_ms: 1 });
  return { repository_id: id, owner_id: owner };
}

async function seedOperator(): Promise<{ token: string; id: string }> {
  const token = mintToken();
  minted.push(token.token);
  await db()
    .prepare(
      `INSERT INTO credentials(credential_id, kind, label, secret_sha256, issued_at_ms, rotate_at_ms, expires_at_ms)
       VALUES (?1, 'operator', 'ops', ?2, 0, ?3, ?3)`,
    )
    .bind(token.credentialId, await secretDigest(token.secret), 4_000_000_000_000)
    .run();
  return { token: token.token, id: token.credentialId };
}

let operatorToken: string | undefined;

/** A host credential issued through the operator API. */
async function hostToken(repositories: string[], roles: string[], extra: Record<string, unknown> = {}): Promise<{ token: string; id: string }> {
  operatorToken ??= (await seedOperator()).token;
  const response = await call(envWith([]), bearer(operatorToken), "/v1/operator/credentials", {
    method: "POST",
    body: JSON.stringify({ label: "rig", repository_ids: repositories, roles, ...extra }),
  });
  expect(response.status).toBe(201);
  const issued = (await response.json()) as { secret: string; credential: { credential_id: string } };
  minted.push(issued.secret);
  return { token: issued.secret, id: issued.credential.credential_id };
}

const snapshot = (storage: Env, authorization: string | null, repo: string) => call(storage, authorization, `/v1/repos/${repo}/snapshot`);

function expectAdmitted(response: Response) {
  expect([401, 403, 429, 503]).not.toContain(response.status);
}

function load(text: string): JsonObject {
  const loaded = loadStrict(new TextEncoder().encode(text));
  if (!loaded.ok) throw new Error(JSON.stringify(loaded.problems));
  return loaded.value as JsonObject;
}

const rig = () => load(rigFixture);
const submission = (collection: JsonObject) => `{"source_version":3,"collection":${canonical(collection)}}`;

async function evidenceWrites(repo: string): Promise<number> {
  const row = await db()
    .prepare(
      `SELECT (SELECT COUNT(*) FROM observations WHERE repository_id = ?1)
            + (SELECT COUNT(*) FROM collections WHERE repository_id = ?1)
            + (SELECT COUNT(*) FROM location_events WHERE repository_id = ?1)
            + (SELECT COUNT(*) FROM report_locations WHERE repository_id = ?1) AS n`,
    )
    .bind(repo)
    .first<{ n: number }>();
  return row!.n;
}

describe("host tokens", () => {
  it("admits a valid token on every request and refuses it once expired", async () => {
    const repo = await registered();
    const storage = envWith([repo]);
    const { token } = await hostToken([repo.repository_id], ["reader"], { expires_in_days: 1 });
    expectAdmitted(await snapshot(storage, bearer(token), repo.repository_id));
    clock += DAY_MS - 1_000;
    expectAdmitted(await snapshot(storage, bearer(token), repo.repository_id));
    clock += 1_000;
    await expectError(await snapshot(storage, bearer(token), repo.repository_id), 401, "unauthenticated");
  });

  it("refuses a revoked token on the very next request", async () => {
    const repo = await registered();
    const storage = envWith([repo]);
    const { token, id } = await hostToken([repo.repository_id], ["reader"]);
    expectAdmitted(await snapshot(storage, bearer(token), repo.repository_id));
    const revoked = await call(storage, bearer(operatorToken!), `/v1/operator/credentials/${id}/revoke`, { method: "POST", body: '{"reason":"lost"}' });
    expect(revoked.status).toBe(200);
    await expectError(await snapshot(storage, bearer(token), repo.repository_id), 401, "unauthenticated");
  });

  it("refuses a wrong secret and an unknown credential alike, never echoing the token", async () => {
    const repo = await registered();
    const storage = envWith([repo]);
    const { id } = await hostToken([repo.repository_id], ["reader"]);
    const wrong = `ebt_${id}.${"A".repeat(43)}`;
    const unknown = mintToken().token;
    const answers = [];
    for (const token of [wrong, unknown]) {
      const response = await snapshot(storage, bearer(token), repo.repository_id);
      const text = await response.clone().text();
      expect(text).not.toContain(token.split(".")[1]);
      answers.push((await expectError(response, 401, "unauthenticated")).error.message);
    }
    expect(answers[0]).toBe(answers[1]);
  });

  it("refuses a missing or malformed Authorization header", async () => {
    const repo = await registered();
    const storage = envWith([repo]);
    for (const authorization of [null, "Basic dXNlcjpwYXNz", "Bearer", "Bearer ebt_short.secret", `Bearer ebt_${hex(32)}.${"a".repeat(42)}`, "Bearer not-a-jwt"]) {
      await expectError(await snapshot(storage, authorization, repo.repository_id), 401, "unauthenticated");
    }
  });

  it("keeps a replaced token working only until the shortened overlap ends", async () => {
    const repo = await registered();
    const storage = envWith([repo]);
    const old = await hostToken([repo.repository_id], ["reader"]);
    clock += 10 * DAY_MS;
    const replacement = await hostToken([repo.repository_id], ["reader"], { replaces: old.id });
    expectAdmitted(await snapshot(storage, bearer(old.token), repo.repository_id));
    expectAdmitted(await snapshot(storage, bearer(replacement.token), repo.repository_id));
    clock += 7 * DAY_MS - 1_000;
    expectAdmitted(await snapshot(storage, bearer(old.token), repo.repository_id));
    clock += 1_000;
    await expectError(await snapshot(storage, bearer(old.token), repo.repository_id), 401, "unauthenticated");
    expectAdmitted(await snapshot(storage, bearer(replacement.token), repo.repository_id));
  });

  it("denies another repository, a repository missing from the configuration, and empty grants", async () => {
    const granted = await registered();
    const other = await registered();
    const storage = envWith([granted, other]);
    const { token } = await hostToken([granted.repository_id], ["reader", "producer"]);
    await expectError(await snapshot(storage, bearer(token), other.repository_id), 403, "denied");

    const unlisted = envWith([other]);
    await expectError(await snapshot(unlisted, bearer(token), granted.repository_id), 403, "denied");

    const empty = await hostToken([granted.repository_id], []);
    await expectError(await snapshot(storage, bearer(empty.token), granted.repository_id), 403, "denied");
  });

  it("refuses host credentials on operator routes and audits the refusal", async () => {
    const repo = await registered();
    const storage = envWith([repo]);
    const { token, id } = await hostToken([repo.repository_id], ["reader", "producer", "verifier", "retention-verifier"]);
    await expectError(
      await call(storage, bearer(token), "/v1/operator/credentials", { method: "POST", body: JSON.stringify({ label: "x", repository_ids: [], roles: [] }) }),
      403,
      "denied",
    );
    const audit = await db()
      .prepare("SELECT action, repository_id, detail_json FROM audit_log WHERE actor_credential_id = ?1")
      .bind(id)
      .all<{ action: string; repository_id: string | null; detail_json: string }>();
    expect(audit.results).toEqual([{ action: "credential.denied", repository_id: null, detail_json: '{"operation":"operator"}' }]);
  });

  it("records the authenticated credential as the publisher, not the fixture's producer fields", async () => {
    const repo = await registered();
    const storage = envWith([repo]);
    const { token, id } = await hostToken([repo.repository_id], ["producer", "reader"]);
    const collection = rig();
    const response = await call(storage, bearer(token), `/v1/repos/${repo.repository_id}/collections`, { method: "POST", body: submission(collection) });
    expect(response.status).toBe(200);

    const publisher: Publisher = { kind: "credential", credential_id: id };
    const rows = await db()
      .prepare("SELECT DISTINCT accepted_principal FROM observations WHERE repository_id = ?1")
      .bind(repo.repository_id)
      .all<{ accepted_principal: string }>();
    expect(rows.results).toEqual([{ accepted_principal: publisherKey(publisher) }]);
    const claimed = JSON.stringify((collection["observations"] as JsonObject[]).map((observation) => observation["producer"]));
    expect(claimed).not.toContain(id);

    const exported = await call(storage, bearer(token), `/v1/repos/${repo.repository_id}/collections/${identity(canonical(collection))}`);
    expect(exported.status).toBe(200);
    expect((JSON.parse(await exported.text()) as { acceptance: { accepted_by: unknown } }).acceptance.accepted_by).toEqual(publisher);
  });
});

/** Repository operations the Worker routes today, with one request each. */
const ROUTED: Record<string, { method: string; path: (repo: string) => string; body?: () => string }> = {
  ingestCollection: { method: "POST", path: (repo) => `/v1/repos/${repo}/collections`, body: () => submission(rig()) },
  exportCollection: { method: "GET", path: (repo) => `/v1/repos/${repo}/collections/${"a".repeat(64)}` },
  getObservation: { method: "GET", path: (repo) => `/v1/repos/${repo}/observations/${"a".repeat(64)}` },
  queryHistory: { method: "POST", path: (repo) => `/v1/repos/${repo}/history/query`, body: () => "{}" },
  getSnapshot: { method: "GET", path: (repo) => `/v1/repos/${repo}/snapshot` },
  queryChanges: { method: "POST", path: (repo) => `/v1/repos/${repo}/changes/query`, body: () => "{}" },
  appendLocation: { method: "POST", path: (repo) => `/v1/repos/${repo}/locations`, body: () => "{}" },
  publishEvaluation: { method: "POST", path: (repo) => `/v1/repos/${repo}/evaluations`, body: () => "{}" },
};

describe("every role against every routed repository operation", () => {
  const roles = ["reader", "producer", "verifier", "retention-verifier", "operator"] as const;
  const tokens = new Map<string, { token: string; id: string }>();
  let repo: Allowance;
  let storage: Env;

  beforeAll(async () => {
    repo = await registered();
    storage = envWith([repo]);
    for (const role of roles) tokens.set(role, role === "operator" ? await seedOperator() : await hostToken([repo.repository_id], [role]));
  });

  it("covers only operations the contract declares", () => {
    for (const operation of Object.keys(ROUTED)) expect(Object.keys(OPERATIONS)).toContain(operation);
  });

  for (const [operation, spec] of Object.entries(ROUTED)) {
    for (const role of roles) {
      const allowed = (OPERATIONS[operation as OperationId].roles as readonly string[]).includes(role);
      const expectation =
        operation === "publishEvaluation" ? "evaluation_publication_disabled" : allowed ? "admitted" : "denied";
      it(`${operation} as ${role}: ${expectation}`, async () => {
        const { token, id } = tokens.get(role)!;
        const before = await evidenceWrites(repo.repository_id);
        const response = await call(storage, bearer(token), spec.path(repo.repository_id), {
          method: spec.method,
          ...(spec.body === undefined ? {} : { body: spec.body() }),
        });
        if (expectation === "admitted") {
          expectAdmitted(response);
          return;
        }
        await expectError(response, 403, expectation);
        expect(await evidenceWrites(repo.repository_id)).toBe(before);
        if (expectation === "denied") {
          const audit = await db()
            .prepare("SELECT detail_json FROM audit_log WHERE actor_credential_id = ?1 AND action = 'credential.denied' AND repository_id = ?2")
            .bind(id, repo.repository_id)
            .all<{ detail_json: string }>();
          expect(audit.results.map((row) => JSON.parse(row.detail_json).operation)).toContain(operation);
        }
      });
    }
  }
});

describe("GitHub OIDC", () => {
  type Claims = Record<string, unknown>;

  function claims(repo: string, overrides: Claims = {}): Claims {
    const base: Claims = {
      repository: SLUG,
      repository_id: repo,
      repository_owner_id: OWNER,
      workflow_ref: `${SLUG}/${WORKFLOW}@refs/heads/main`,
      ref: "refs/heads/main",
      event_name: "workflow_dispatch",
      run_id: "9876543210",
      run_attempt: "2",
      head_ref: "",
      base_ref: "",
    };
    const merged: Claims = { ...base, ...overrides };
    for (const key of Object.keys(merged)) if (merged[key] === undefined) delete merged[key];
    return merged;
  }

  interface Signing {
    key?: CryptoKey;
    issuer?: string;
    audience?: string;
    expiresIn?: number;
    notBefore?: number;
  }

  function jwt(payload: Claims, signing: Signing = {}): Promise<string> {
    const now = Math.floor(clock / 1000);
    return new SignJWT(payload)
      .setProtectedHeader({ alg: "RS256", kid: "test-key" })
      .setIssuer(signing.issuer ?? ISSUER)
      .setAudience(signing.audience ?? AUDIENCE)
      .setIssuedAt(now)
      .setNotBefore(now + (signing.notBefore ?? 0))
      .setExpirationTime(now + (signing.expiresIn ?? 300))
      .sign(signing.key ?? signingKey);
  }

  async function trustedRepository() {
    const repo = await registered();
    const storage = envWith([repo], [{ repository_id: repo.repository_id }]);
    return { repo, storage };
  }

  it("grants producer and reader to the trusted workflow_dispatch on refs/heads/main, and records the workflow as publisher", async () => {
    const { repo, storage } = await trustedRepository();
    const token = await jwt(claims(repo.repository_id));
    expectAdmitted(await snapshot(storage, bearer(token), repo.repository_id));
    const ingested = await call(storage, bearer(token), `/v1/repos/${repo.repository_id}/collections`, { method: "POST", body: submission(rig()) });
    expect(ingested.status).toBe(200);
    const row = await db().prepare("SELECT accepted_principal FROM observations WHERE repository_id = ?1 LIMIT 1").bind(repo.repository_id).first<{ accepted_principal: string }>();
    expect(JSON.parse(row!.accepted_principal)).toEqual({
      kind: "github-oidc",
      repository_id: repo.repository_id,
      workflow: WORKFLOW,
      ref: "refs/heads/main",
      event_name: "workflow_dispatch",
      run_id: "9876543210",
      run_attempt: 2,
    });
  });

  it("never grants verifier: appendLocation is denied", async () => {
    const { repo, storage } = await trustedRepository();
    const token = await jwt(claims(repo.repository_id));
    await expectError(await call(storage, bearer(token), `/v1/repos/${repo.repository_id}/locations`, { method: "POST", body: "{}" }), 403, "denied");
  });

  it("accepts expiry within the clock tolerance", async () => {
    const { repo, storage } = await trustedRepository();
    expectAdmitted(await snapshot(storage, bearer(await jwt(claims(repo.repository_id), { expiresIn: -30 })), repo.repository_id));
  });

  const unverified: [string, (repo: string) => Promise<string>][] = [
    ["a wrong issuer", (repo) => jwt(claims(repo), { issuer: "https://example.com" })],
    ["a wrong audience", (repo) => jwt(claims(repo), { audience: "another-service" })],
    ["expiry beyond the clock tolerance", (repo) => jwt(claims(repo), { expiresIn: -61 })],
    ["a not-before in the future", (repo) => jwt(claims(repo), { notBefore: 120 })],
    ["a signature by another key", (repo) => jwt(claims(repo), { key: foreignKey })],
    [
      "alg none",
      async (repo) => {
        const part = (value: unknown) => btoa(JSON.stringify(value)).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
        const now = Math.floor(clock / 1000);
        return `${part({ alg: "none", typ: "JWT" })}.${part({ ...claims(repo), iss: ISSUER, aud: AUDIENCE, exp: now + 300 })}.`;
      },
    ],
    [
      "an HS256 signature",
      (repo) =>
        new SignJWT(claims(repo))
          .setProtectedHeader({ alg: "HS256", kid: "test-key" })
          .setIssuer(ISSUER)
          .setAudience(AUDIENCE)
          .setExpirationTime(Math.floor(clock / 1000) + 300)
          .sign(new TextEncoder().encode("a-shared-secret-of-thirty-two-by")),
    ],
  ];
  for (const [name, mint] of unverified) {
    it(`answers 401 for ${name}`, async () => {
      const { repo, storage } = await trustedRepository();
      await expectError(await snapshot(storage, bearer(await mint(repo.repository_id)), repo.repository_id), 401, "unauthenticated");
    });
  }

  const untrusted: [string, Claims][] = [
    ["a wrong ref", { ref: "refs/heads/dev", workflow_ref: `${SLUG}/${WORKFLOW}@refs/heads/dev` }],
    ["a workflow_ref whose ref disagrees with the ref claim", { workflow_ref: `${SLUG}/${WORKFLOW}@refs/heads/dev` }],
    ["a workflow_ref for another repository", { workflow_ref: `other/repo/${WORKFLOW}@refs/heads/main` }],
    ["a wrong workflow", { workflow_ref: `${SLUG}/.github/workflows/other.yml@refs/heads/main` }],
    ["a wrong event", { event_name: "push" }],
    ["a pull_request event", { event_name: "pull_request" }],
    ["a pull_request_target event", { event_name: "pull_request_target" }],
    ["a nonempty head_ref", { head_ref: "feature" }],
    ["a nonempty base_ref", { base_ref: "main" }],
    ["an owner mismatch", { repository_owner_id: "999" }],
    ["a non-numeric repository_id", { repository_id: "rusty-biscuit" }],
    ["a missing repository_owner_id", { repository_owner_id: undefined }],
  ];
  for (const [name, overrides] of untrusted) {
    it(`answers 403 for a verified token with ${name}`, async () => {
      const { repo, storage } = await trustedRepository();
      const before = await evidenceWrites(repo.repository_id);
      const token = await jwt(claims(repo.repository_id, overrides));
      await expectError(await snapshot(storage, bearer(token), repo.repository_id), 403, "denied");
      await expectError(
        await call(storage, bearer(token), `/v1/repos/${repo.repository_id}/collections`, { method: "POST", body: submission(rig()) }),
        403,
        "denied",
      );
      expect(await evidenceWrites(repo.repository_id)).toBe(before);
    });
  }

  it("answers 403 when the token's repository_id is another repository's", async () => {
    const { repo, storage } = await trustedRepository();
    const other = await registered();
    const token = await jwt(claims(other.repository_id));
    await expectError(await snapshot(storage, bearer(token), other.repository_id), 403, "denied");
    const ownToken = await jwt(claims(repo.repository_id));
    await expectError(await snapshot(envWith([repo, other], [{ repository_id: repo.repository_id }]), bearer(ownToken), other.repository_id), 403, "denied");
  });

  it("answers 403 when the registered owner differs from the configured owner", async () => {
    const repo = await registered("111");
    const storage = envWith([{ repository_id: repo.repository_id, owner_id: OWNER }], [{ repository_id: repo.repository_id }]);
    await expectError(await snapshot(storage, bearer(await jwt(claims(repo.repository_id))), repo.repository_id), 403, "denied");
  });

  it("denies every token when trusted_workflows is empty", async () => {
    const repo = await registered();
    await expectError(await snapshot(envWith([repo]), bearer(await jwt(claims(repo.repository_id))), repo.repository_id), 403, "denied");
  });

  it("answers 503 when the signing keys cannot be fetched", async () => {
    const { repo, storage } = await trustedRepository();
    const offline = createWorker({
      authenticate: createAuthenticator({
        jwks: async () => {
          throw new TypeError("fetch failed");
        },
      }),
      now: () => clock,
    });
    const token = await jwt(claims(repo.repository_id));
    const response = await offline.fetch!(
      new Request(`https://evidence.test/v1/repos/${repo.repository_id}/snapshot`, { headers: { authorization: bearer(token) } }) as never,
      storage,
      {} as ExecutionContext,
    );
    await expectError(response, 503, "unavailable");
  });
});

describe("service limits and configuration", () => {
  it("answers 429 rate_limited with a retry hint when the limiter refuses", async () => {
    const repo = await registered();
    const { token } = await hostToken([repo.repository_id], ["reader"]);
    const storage = envWith([repo], [], { READ_LIMITER: { limit: async () => ({ success: false }) } });
    const body = await expectError(await snapshot(storage, bearer(token), repo.repository_id), 429, "rate_limited");
    expect(body.error.retryable).toBe(true);
    expect(body.error.retry_after_ms).toBeGreaterThan(0);
  });

  it("answers 503 when a limiter binding is missing", async () => {
    const repo = await registered();
    const { token } = await hostToken([repo.repository_id], ["reader"]);
    await expectError(await snapshot(envWith([repo], [], { READ_LIMITER: undefined }), bearer(token), repo.repository_id), 503, "unavailable");
  });

  it("answers 503 to every request while the configuration is missing or invalid", async () => {
    const repo = await registered();
    const { token } = await hostToken([repo.repository_id], ["reader"]);
    for (const config of ["{}", "not json", JSON.stringify({ ...BASE_CONFIG, repositories: null }), undefined]) {
      const storage = { ...(env as unknown as Env), SERVICE_CONFIG: config } as Env;
      await expectError(await snapshot(storage, bearer(token), repo.repository_id), 503, "unavailable");
      await expectError(await snapshot(storage, null, repo.repository_id), 503, "unavailable");
      await expectError(await call(storage, bearer(operatorToken!), "/v1/operator/audit/query", { method: "POST", body: "{}" }), 503, "unavailable");
    }
  });

  it("never stores or returns a token secret or its digest in audit rows", async () => {
    const rows = JSON.stringify((await db().prepare("SELECT * FROM audit_log").all()).results);
    expect(minted.length).toBeGreaterThan(3);
    for (const token of minted) {
      const secret = token.split(".")[1]!;
      expect(rows).not.toContain(secret);
      expect(rows).not.toContain(await secretDigest(secret));
    }
  });
});
