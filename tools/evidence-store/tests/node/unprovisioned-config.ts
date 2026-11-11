// The checked-in wrangler.jsonc as it was before any environment was
// provisioned, so tests do not depend on the real deployment's IDs or
// allowlists.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

export const PLACEHOLDER_DATABASE_ID = "00000000-0000-0000-0000-000000000000";

const SERVICE_CONFIG = /"SERVICE_CONFIG": ("(?:[^"\\]|\\.)*")/g;

export function unprovisionedConfig(): string {
  const text = readFileSync(fileURLToPath(new URL("../../wrangler.jsonc", import.meta.url)), "utf8");
  return text
    .replace(/"database_id": "[^"]*"/g, `"database_id": "${PLACEHOLDER_DATABASE_ID}"`)
    .replace(SERVICE_CONFIG, (_match, literal: string) => {
      const config = JSON.parse(JSON.parse(literal) as string) as { repositories: unknown[]; oidc: { trusted_workflows: unknown[] } };
      config.repositories = [];
      config.oidc.trusted_workflows = [];
      return `"SERVICE_CONFIG": ${JSON.stringify(JSON.stringify(config))}`;
    });
}
