declare namespace Cloudflare {
  interface Env {
    SPIKE_DB: D1Database;
    SPIKE_TOKEN: string;
    SPIKE_MIGRATIONS: import("cloudflare:test").D1Migration[];
  }
  interface Exports {
    default: Fetcher;
  }
}
