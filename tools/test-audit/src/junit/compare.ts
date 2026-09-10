/**
 * Baseline ↔ candidate comparison, one environment against itself.
 *
 * Legs are never compared across environments: a `windows-latest` identity set
 * is not an `ubuntu-latest` one.
 */
import type { CollectionResult } from "./collect.ts";

export interface EnvironmentComparison {
  environment: string;
  /** Identities present in both sets; the summed columns cover these only. */
  matched: number;
  baselineMatchedSeconds: number;
  candidateMatchedSeconds: number;
  /** Candidate-only identities and the time they carry. */
  added: string[];
  addedSeconds: number;
  /** Baseline-only identities and the time they carried. */
  removed: string[];
  removedSeconds: number;
}

/**
 * Compare every environment present in both sets against its own baseline.
 *
 * Legs present on one side only are left out rather than matched against a
 * neighbour: reporting a missing leg as hundreds of removals would be exactly
 * the cross-platform count comparison the plan forbids.
 */
export function compare(baseline: CollectionResult, candidate: CollectionResult): EnvironmentComparison[] {
  const comparisons: EnvironmentComparison[] = [];
  for (const [environment, candidateCases] of Array.from(candidate.casesByEnvironment)) {
    const baselineCases = baseline.casesByEnvironment.get(environment);
    if (!baselineCases) continue;
    const before = new Map(baselineCases.map((c) => [c.identity, c.seconds]));
    const after = new Map(candidateCases.map((c) => [c.identity, c.seconds]));
    const comparison: EnvironmentComparison = {
      environment,
      matched: 0,
      baselineMatchedSeconds: 0,
      candidateMatchedSeconds: 0,
      added: [],
      addedSeconds: 0,
      removed: [],
      removedSeconds: 0,
    };
    for (const [identity, seconds] of Array.from(after)) {
      const previous = before.get(identity);
      if (previous === undefined) {
        comparison.added.push(identity);
        comparison.addedSeconds += seconds;
        continue;
      }
      comparison.matched += 1;
      comparison.baselineMatchedSeconds += previous;
      comparison.candidateMatchedSeconds += seconds;
    }
    for (const [identity, seconds] of Array.from(before)) {
      if (after.has(identity)) continue;
      comparison.removed.push(identity);
      comparison.removedSeconds += seconds;
    }
    comparison.added.sort();
    comparison.removed.sort();
    comparisons.push(comparison);
  }
  return comparisons;
}
