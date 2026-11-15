// Runs before every service test file. `applyD1Migrations` records what it has
// applied, so a database that already has the schema is left untouched.
import { applyD1Migrations } from "cloudflare:test";
import { env } from "cloudflare:workers";

await applyD1Migrations(env.EVIDENCE_DB, env.TEST_MIGRATIONS);
