// The deployed acceptance cases run in process against the real
// authenticator: bearer tokens are issued through the operator API and
// checked against local D1 digests. The only shortcut is the operator
// credential, inserted directly as the Wrangler bootstrap would.
import { env } from "cloudflare:workers";
import { beforeAll, describe, it } from "vitest";
import { createAuthenticator } from "../../src/auth.ts";
import { insertRepository } from "../../src/db/repositories.ts";
import { mintToken, secretDigest } from "../../src/tokens.ts";
import { createWorker, type Env } from "../../src/worker.ts";
import { ACCEPTANCE_CASES, type AcceptanceTarget } from "../acceptance/cases.ts";
import { db, hex } from "./helpers.ts";

let clock = Date.now();
const worker = createWorker({ authenticate: createAuthenticator(), now: () => clock });
let target: AcceptanceTarget;

beforeAll(async () => {
  const digits = crypto.getRandomValues(new Uint32Array(2));
  const repositoryId = `${1 + (digits[0]! % 9)}${String(digits[1]).padStart(10, "0")}`;
  const owner = "4417321";
  await insertRepository(db(), { repository_id: repositoryId, owner_id: owner, slug: `owner/repo-${repositoryId}`, generation: hex(32) });
  const config = JSON.parse(env.SERVICE_CONFIG) as Record<string, unknown>;
  const storage = {
    ...(env as unknown as Env),
    SERVICE_CONFIG: JSON.stringify({ ...config, repositories: [{ slug: `owner/repo-${repositoryId}`, repository_id: repositoryId, owner_id: owner }] }),
  } as Env;

  const operator = mintToken();
  await db()
    .prepare(
      `INSERT INTO credentials(credential_id, kind, label, secret_sha256, issued_at_ms, rotate_at_ms, expires_at_ms)
       VALUES (?1, 'operator', 'acceptance-operator', ?2, ?3, ?4, ?4)`,
    )
    .bind(operator.credentialId, await secretDigest(operator.secret), clock, clock + 3650 * 86_400_000)
    .run();

  target = {
    repositoryId,
    operatorToken: operator.token,
    advanceClock: (ms) => {
      clock += ms;
    },
    send: async (path, init) =>
      worker.fetch!(
        new Request(`https://evidence.test${path}`, {
          method: init.method ?? "GET",
          headers: { authorization: `Bearer ${init.token}` },
          ...(init.body === undefined ? {} : { body: init.body }),
        }) as never,
        storage,
        {} as ExecutionContext,
      ),
  };
});

describe("deployed acceptance cases, in process", () => {
  for (const acceptance of ACCEPTANCE_CASES) it(acceptance.name, () => acceptance.run(target));
});
