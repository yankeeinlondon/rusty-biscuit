//! The Worker's bindings, declared once so route modules and the
//! authenticator share one definition.

export interface Env {
  EVIDENCE_DB: D1Database;
  EVIDENCE_BUCKET: R2Bucket;
  CURSOR_HMAC_KEY: string;
  /**
   * The service configuration document (`api/schemas/service-config.schema.json`)
   * as JSON text. Missing or invalid configuration answers 503 to every
   * request rather than running with a partial allowlist.
   */
  SERVICE_CONFIG: string;
  /** Per-principal request rates; a missing binding fails closed (503). */
  READ_LIMITER: RateLimit;
  MUTATION_LIMITER: RateLimit;
  OPERATOR_LIMITER: RateLimit;
}
