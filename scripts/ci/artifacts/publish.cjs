const fs = require('node:fs/promises');
const path = require('node:path');

// Transport is per record even though compilation is per owner. Continue after
// an individual upload fails so unrelated packages keep their artifacts.
async function publish({plan, producer, out, verifier, upload, jobStatus, queueSeconds}) {
  const records = plan.builds.filter(r => r.producer === producer);
  let failed = false;
  const owner = {producer, requested_records: records.length, measured_records: 0,
    compiler_invocations: 0, compiler_ms: 0};
  for (const record of records) {
    try {
      const manifest = JSON.parse(await fs.readFile(path.join(out, `${record.artifact}.manifest.json`), 'utf8'));
      if (manifest.compiler_work) {
        owner.measured_records++;
        owner.compiler_invocations += manifest.compiler_work.compiler_invocations || 0;
        owner.compiler_ms += manifest.compiler_work.compiler_ms || 0;
      }
    } catch { /* Failed records have statuses but may have no manifest. */ }
  }
  try { Object.assign(owner, JSON.parse(await fs.readFile(path.join(out, 'owner-timing.json'), 'utf8'))); }
  catch { /* A lost owner leaves no timing observation. */ }
  const tools = path.join(out, 'tools');
  await fs.mkdir(tools, {recursive: true});
  const tool = path.join(tools, path.basename(verifier));
  let toolError;
  try { await fs.copyFile(verifier, tool); } catch (error) { toolError = error; }
  for (const record of records) {
    const {artifact, key, package: pkg, consumers} = record;
    let status;
    try { status = JSON.parse(await fs.readFile(path.join(out, `${artifact}.status.json`), 'utf8')); }
    catch {
      status = {schema_version: 1, key, package: pkg, producer, artifact, consumers,
        result: jobStatus === 'cancelled' ? 'cancelled' : 'failure', stage: 'produce',
        detail: `owner concluded ${jobStatus} before writing a status`};
    }
    const start = Date.now();
    if (status.result === 'success') {
      try {
        if (toolError) throw toolError;
        const manifestPath = path.join(out, `${artifact}.manifest.json`);
        const manifest = JSON.parse(await fs.readFile(manifestPath, 'utf8'));
        const files = [manifestPath, path.join(out, manifest.archive.file), tool,
          ...manifest.sidecars.map(s => path.join(out, s.record.file))];
        if (manifest.compiler_work) {
          const counters = path.join(out, `${artifact}.compiler-work.json`);
          await fs.writeFile(counters, JSON.stringify({record: manifest.compiler_work, owner}));
          files.push(counters);
        }
        await upload(artifact, files, out, {retentionDays: 1, compressionLevel: 0});
      } catch (error) {
        status.result = 'failure'; status.stage = 'upload'; status.detail = String(error);
      }
    }
    if (status.result !== 'success') failed = true;
    status.stage_seconds = {upload_seconds: (Date.now() - start) / 1000};
    if (Number.isFinite(queueSeconds)) status.stage_seconds.queue_seconds = queueSeconds;
    const dir = path.join(out, `build-status-${pkg}-${producer}-${key}`);
    await fs.mkdir(dir, {recursive: true});
    const file = path.join(dir, 'build-status.json');
    await fs.writeFile(file, JSON.stringify(status));
    try { await upload(path.basename(dir), [file], dir, {retentionDays: 3}); }
    catch (error) { failed = true; process.stderr.write(`${error}\n`); }
  }
  return !failed;
}
module.exports = {publish};
