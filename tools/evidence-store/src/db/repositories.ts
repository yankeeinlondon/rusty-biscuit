//! Repository rows: existence checks and operator/test bootstrap.

export interface NewRepository {
  repository_id: string;
  slug: string;
  /** Restore generation; a fresh random value per database lifetime. */
  generation: string;
  /** Stable GitHub owner ID; omitted means hosted identities never match. */
  owner_id?: string;
  registered_at_ms?: number;
}

export async function repositoryExists(db: D1Database, repositoryId: string): Promise<boolean> {
  const row = await db
    .prepare("SELECT 1 AS present FROM repositories WHERE repository_id = ?1")
    .bind(repositoryId)
    .first<{ present: number }>();
  return row !== null;
}

/** Plain insert: registering an existing repository ID fails rather than renaming it. */
export async function insertRepository(db: D1Database, repository: NewRepository): Promise<void> {
  await db
    .prepare(
      `INSERT INTO repositories(repository_id, owner_id, slug, generation, registered_at_ms)
       VALUES (?1, ?2, ?3, ?4, ?5)`,
    )
    .bind(
      repository.repository_id,
      repository.owner_id ?? null,
      repository.slug,
      repository.generation,
      repository.registered_at_ms ?? Date.now(),
    )
    .run();
}
