declare namespace Cloudflare {
  interface Env {
    EVIDENCE_DB: D1Database;
    EVIDENCE_BUCKET: R2Bucket;
    CURSOR_HMAC_KEY: string;
    SERVICE_CONFIG: string;
    READ_LIMITER: RateLimit;
    MUTATION_LIMITER: RateLimit;
    OPERATOR_LIMITER: RateLimit;
    TEST_MIGRATIONS: import("cloudflare:test").D1Migration[];
  }
}

declare module "*?raw" {
  const text: string;
  export default text;
}
