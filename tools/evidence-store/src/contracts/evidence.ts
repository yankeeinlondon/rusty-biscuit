//! Shapes of the existing evidence documents the store accepts.
//!
//! These mirror the executable contract in `scripts/ci/schema.py`
//! (`RECEIPT_*_FIELDS`, `OBSERVATION_*_FIELDS`, `COLLECTION_FIELDS`), which stays
//! authoritative: where this file and the Python disagree, the Python wins and
//! this file is the defect. Numeric fields are carried as `JsonNumber` so an
//! integer token and a float token stay distinguishable until canonicalization.

import type { JsonNumber, JsonValue } from "./json.ts";

export const COLLECTION_SCHEMA_VERSION = 3;
export const RECEIPT_SCHEMA_VERSION = 2;
/** A version-1 receipt is legacy evidence; this store has no adapter for it. */
export const LEGACY_RECEIPT_SCHEMA_VERSION = 1;

export const ENVIRONMENTS = [
  "ubuntu-latest",
  "windows-latest",
  "macos-latest",
  "wsl2-ubuntu",
] as const;
export type Environment = (typeof ENVIRONMENTS)[number];

export const GATES = ["lint", "check", "L1", "L2", "browser"] as const;
export type Gate = (typeof GATES)[number];

export const COMPLETIONS = ["complete", "interrupted", "partial"] as const;
export type Completion = (typeof COMPLETIONS)[number];

export const OUTCOMES = ["pass", "fail"] as const;
export type Outcome = (typeof OUTCOMES)[number];

export const PRODUCER_KINDS = ["hook", "rig", "hosted", "seed"] as const;
export type ProducerKind = (typeof PRODUCER_KINDS)[number];

/** Where an observation's report bytes originally lived. R2 is not one of these. */
export const REPORT_STORES = ["host-dir", "artifact"] as const;
export type ReportStore = (typeof REPORT_STORES)[number];

export const FAILURE_DETAIL_LIMIT = 20;

/** File names, inside `producer.host.report_dir`, of the documents a proof cites. */
export const PROOF_EXPECTED_FILE = "{package}-{gate}.expected.json";
export const PROOF_COMPLETION_FILE = "{package}-{gate}.completion.json";

export interface Counts {
  total: JsonNumber;
  passed: JsonNumber;
  failed: JsonNumber;
  skipped: JsonNumber;
  errored: JsonNumber;
}

export interface ReceiptBuild {
  key: string;
  digest: string;
}

/** A version-2 receipt cell; an observation's `cell` is this same contract. */
export interface ReceiptCell {
  package: string;
  gate: Gate;
  outcome: Outcome;
  exit_code: JsonNumber;
  completion: Completion;
  counts: Counts;
  duration_s: JsonNumber;
  gate_input_identity: string;
  backends: string[];
  report: string;
  failed_tests?: string[];
  failure_detail_truncated?: JsonValue;
  build?: ReceiptBuild;
  test_filter?: string;
}

export interface ReceiptHost {
  os: string;
  kernel: string;
  /** Interpreted on the submitting host only (with `~` expansion); never by the Worker. */
  report_dir: string;
}

export interface Receipt {
  schema_version: 2;
  environment: Environment;
  base: string;
  head: string;
  tree: string;
  scope_identity: string;
  completion: Completion;
  host: ReceiptHost;
  cells: ReceiptCell[];
}

export interface ObservationHost {
  os: string;
  kernel: string;
  label?: string;
  report_dir?: string;
}

export interface ObservationProducer {
  kind: ProducerKind;
  host: ObservationHost;
  workflow?: string;
  run_id?: string;
  attempt?: JsonNumber;
  seed_id?: string;
}

/** Digests are nonempty strings, deliberately not narrowed to hex. */
export interface ObservationProof {
  nextest_version: string;
  target: string;
  expected_listing_digest: string;
  completion_record_digest?: string;
}

export interface ObservationReport {
  /** Provenance only: never resolved by the Worker. */
  path: string;
  sha256: string;
  bytes: JsonNumber;
  store: ReportStore;
}

export interface Observation {
  observation_id: string;
  retry_of?: string;
  producer: ObservationProducer;
  base: string;
  scope_identity: string;
  completion: Completion;
  cell: ReceiptCell;
  proof?: ObservationProof;
  reports: ObservationReport[];
  /** Present but empty is distinct from absent and stays a string. */
  features?: string;
}

export interface Collection {
  schema_version: 3;
  environment: Environment;
  head: string;
  tree: string;
  observations: Observation[];
}

/** Which evidence generation a submitted collection was adapted from. */
export type SourceVersion = 2 | 3;

/**
 * A collection as the API accepts it: always version 3, plus provenance of the
 * adapter that produced it. `source_version` is never part of identity.
 */
export interface Submission {
  source_version: SourceVersion;
  collection: Collection;
}
