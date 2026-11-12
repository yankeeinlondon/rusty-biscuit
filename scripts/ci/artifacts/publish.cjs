const fs = require('node:fs/promises');
const path = require('node:path');

// Transport is per record even though compilation is per owner. Continue after
// an individual upload fails so unrelated packages keep their artifacts.
//
// Artifact names are immutable within a run, so every upload is recorded in a
// local ledger (`publication-ledger.json` in `out`) before anything it covers
// is deleted, and nothing the ledger already holds is uploaded again. A process
// lost between a remote upload and its ledger write is an infrastructure
// failure: the retry fails on the existing name rather than overwriting it.
const LEDGER = 'publication-ledger.json';

async function readJson(file) {
  return JSON.parse(await fs.readFile(file, 'utf8'));
}

async function loadLedger(out) {
  try {
    const ledger = await readJson(path.join(out, LEDGER));
    return {records: ledger.records || {}, owner_measurement: ledger.owner_measurement === true};
  } catch (error) {
    if (error.code === 'ENOENT') return {records: {}, owner_measurement: false};
    throw error;
  }
}

// Temporary file plus rename in the same directory, so a reader sees either the
// previous ledger or the new one, never a torn write.
async function saveLedger(out, ledger) {
  const file = path.join(out, LEDGER);
  const temporary = `${file}.${process.pid}.tmp`;
  await fs.writeFile(temporary, JSON.stringify(ledger));
  await fs.rename(temporary, file);
}

async function mark(out, ledger, artifact, field) {
  ledger.records[artifact] = {...ledger.records[artifact], [field]: true};
  await saveLedger(out, ledger);
}

// Copied into `out/tools` once per entry point: every archive carries the
// verifier its consumer checks it with. A copy failure fails each successful
// record at the upload stage rather than the owner as a whole.
async function stageVerifier(out, verifier) {
  const tools = path.join(out, 'tools');
  await fs.mkdir(tools, {recursive: true});
  const tool = path.join(tools, path.basename(verifier));
  try { await fs.copyFile(verifier, tool); return {tool}; }
  catch (error) { return {tool, error}; }
}

function inside(root, target) {
  const relative = path.relative(root, target);
  return relative !== '' && !path.isAbsolute(relative) && relative.split(path.sep)[0] !== '..';
}

// Removes an uploaded payload only when it is what the producer wrote inside
// `out`: never a symlink, and never a path that resolves outside `out` through
// one of its parents. A missing path is already released.
async function removeOwned(out, target, kind) {
  if (!inside(out, target)) throw new Error(`refusing to delete ${target}: outside ${out}`);
  let stat;
  try { stat = await fs.lstat(target); }
  catch (error) { if (error.code === 'ENOENT') return; throw error; }
  if (stat.isSymbolicLink()) throw new Error(`refusing to delete ${target}: a symbolic link`);
  if (!inside(await fs.realpath(out), await fs.realpath(target))) {
    throw new Error(`refusing to delete ${target}: resolves outside ${out}`);
  }
  if (kind === 'directory' && stat.isDirectory()) await fs.rm(target, {recursive: true});
  else if (kind === 'file' && stat.isFile()) await fs.unlink(target);
  else throw new Error(`refusing to delete ${target}: not a ${kind}`);
}

function statusDir(out, {package: pkg, key}, producer) {
  return path.join(out, `build-status-${pkg}-${producer}-${key}`);
}

// Publish one record: its archive (when the producer succeeded) and then its
// status. `jobStatus` is required only to synthesize the status of a record the
// producer never reached; without it such a record is left outstanding.
//
// Returns false when the record failed or anything it needed could not be
// published. The published status, once written, is what a later call retries:
// a status upload never causes an archive to be uploaded again.
async function publishRecord({record, producer, out, upload, ledger, verifierCopy, jobStatus, queueSeconds}) {
  const {artifact, key, package: pkg, consumers} = record;
  const entry = ledger.records[artifact] || {};
  const dir = statusDir(out, record, producer);
  const file = path.join(dir, 'build-status.json');
  if (entry.status) {
    try { return (await readJson(file)).result === 'success'; } catch { return false; }
  }
  let status;
  try { status = await readJson(file); }
  catch {
    try { status = await readJson(path.join(out, `${artifact}.status.json`)); }
    catch {
      if (jobStatus === undefined) return false;
      status = {schema_version: 1, key, package: pkg, producer, artifact, consumers,
        result: jobStatus === 'cancelled' ? 'cancelled' : 'failure', stage: 'produce',
        detail: `owner concluded ${jobStatus} before writing a status`};
    }
    const start = Date.now();
    if (status.result === 'success' && !entry.archive) {
      let manifest;
      try {
        if (verifierCopy.error) throw verifierCopy.error;
        const manifestPath = path.join(out, `${artifact}.manifest.json`);
        manifest = await readJson(manifestPath);
        // A sidecar entry is `NamedFile` in `ci-build-archive.rs`: its `FileRecord`
        // is `#[serde(flatten)]`ed, so `file` sits beside `name`, not under `record`.
        const files = [manifestPath, path.join(out, manifest.archive.file), verifierCopy.tool,
          ...manifest.sidecars.map(s => path.join(out, s.file))];
        // The record's own counts only; the owner aggregate is final only once
        // every record has run, so `finalizeOwner` publishes it separately.
        if (manifest.compiler_work) {
          const counters = path.join(out, `${artifact}.compiler-work.json`);
          await fs.writeFile(counters, JSON.stringify({record: manifest.compiler_work}));
          files.push(counters);
        }
        await upload(artifact, files, out, {retentionDays: 1, compressionLevel: 0});
        await mark(out, ledger, artifact, 'archive');
      } catch (error) {
        manifest = undefined;
        status.result = 'failure'; status.stage = 'upload'; status.detail = String(error);
      }
      if (manifest) {
        // Storage is the reason for publishing per record, but a payload that
        // cannot be released leaves the published record intact.
        for (const [target, kind] of [[path.join(out, manifest.archive.file), 'file'],
          [path.join(out, `${artifact}-sidecars`), 'directory']]) {
          try { await removeOwned(out, target, kind); }
          catch (error) { process.stderr.write(`${error}\n`); }
        }
      }
    }
    // Whole seconds, like every other `*_seconds` field: the rollup reads
    // `ProducerStageSeconds` as integers and refused `1.472` for every area on
    // run 35412170320.
    status.stage_seconds = {upload_seconds: Math.round((Date.now() - start) / 1000)};
    if (Number.isFinite(queueSeconds)) status.stage_seconds.queue_seconds = queueSeconds;
    await fs.mkdir(dir, {recursive: true});
    await fs.writeFile(file, JSON.stringify(status));
  }
  let ok = status.result === 'success';
  try {
    await upload(path.basename(dir), [file], dir, {retentionDays: 3});
    await mark(out, ledger, artifact, 'status');
  } catch (error) { ok = false; process.stderr.write(`${error}\n`); }
  return ok;
}

// The owner's compiler-work aggregate, from every retained manifest and the
// coordinator's `owner-timing.json`. Undefined when no record was measured.
async function ownerMeasurement(records, producer, out) {
  const owner = {producer, requested_records: records.length, measured_records: 0,
    compiler_invocations: 0, compiler_ms: 0};
  for (const record of records) {
    try {
      const manifest = await readJson(path.join(out, `${record.artifact}.manifest.json`));
      if (manifest.compiler_work) {
        owner.measured_records++;
        owner.compiler_invocations += manifest.compiler_work.compiler_invocations || 0;
        owner.compiler_ms += manifest.compiler_work.compiler_ms || 0;
      }
    } catch { /* Failed records have statuses but may have no manifest. */ }
  }
  if (owner.measured_records === 0) return undefined;
  try { Object.assign(owner, await readJson(path.join(out, 'owner-timing.json'))); }
  catch { /* A lost owner leaves no timing observation. */ }
  return owner;
}

// Publish whatever the owner has not: records never reached get a synthesized
// status, outstanding archives and statuses are uploaded, and the owner
// measurement artifact is published once. Safe to run after a complete
// coordinator, and more than once.
async function finalizeOwner({plan, producer, out, verifier, upload, jobStatus, queueSeconds}) {
  const records = plan.builds.filter(r => r.producer === producer);
  await fs.mkdir(out, {recursive: true});
  const ledger = await loadLedger(out);
  const verifierCopy = await stageVerifier(out, verifier);
  let ok = true;
  for (const record of records) {
    if (!await publishRecord({record, producer, out, upload, ledger, verifierCopy, jobStatus, queueSeconds})) ok = false;
  }
  const owner = await ownerMeasurement(records, producer, out);
  if (owner && !ledger.owner_measurement) {
    try {
      const file = path.join(out, 'owner-compiler-work.json');
      await fs.writeFile(file, JSON.stringify(owner));
      await upload(`compiler-work-owner-${producer}`, [file], out, {retentionDays: 1, compressionLevel: 0});
      ledger.owner_measurement = true;
      await saveLedger(out, ledger);
    } catch (error) { ok = false; process.stderr.write(`${error}\n`); }
  }
  return ok;
}

module.exports = {publish: finalizeOwner, finalizeOwner, publishRecord, loadLedger, stageVerifier, LEDGER};
