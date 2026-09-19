const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const {publish} = require('./publish.cjs');

// `ci-build`'s own serialization of the document it stores as a manifest's
// `compiler_work`, written by
// `ci-build-tests.rs::the_javascript_publisher_reads_a_fixture_generated_from_this_report`.
// Reading it rather than hand-writing a shape is what keeps this file's
// expectations tied to the Rust struct across the language boundary.
const documents = require('../../fixtures/compiler-work/publisher-documents.json');

// The owner aggregate is the evidence the compile-once claim is read from, so a
// zero counter, an unmeasured owner, or a lost produce timing is a measurement
// failure rather than a datum. Kept separate from the value assertions so the
// degenerate fixtures below can prove it actually rejects them.
function assertOwnerMeasuredSomething(owner) {
  const seen = JSON.stringify(owner);
  assert(owner.measured_records > 0, `no record carried compiler work: ${seen}`);
  assert(owner.compiler_invocations > 0, `owner recorded no rustc invocation: ${seen}`);
  assert(owner.compiler_ms > 0, `owner recorded no compiler time: ${seen}`);
  assert(Number.isFinite(owner.produce_wall_seconds), `owner timing was not merged: ${seen}`);
}

// One successful owner leg. `works[i]` is the record's `compiler_work`, or
// `null` for a producer run that was never measured.
async function publishMeasured({works, ownerTiming}) {
  const out = await fs.mkdtemp(path.join(os.tmpdir(), 'ci-artifacts-'));
  const verifier = path.join(out, 'ci-build');
  await fs.writeFile(verifier, 'verifier');
  const records = works.map((_, index) => ({
    package: `pkg${index}`, producer: 'linux', key: `k${index}`,
    artifact: `build-pkg${index}-linux-k${index}`, consumers: []
  }));
  for (const [index, record] of records.entries()) {
    const manifest = {archive: {file: `${record.artifact}.tar.zst`}, sidecars: []};
    if (works[index] !== null) manifest.compiler_work = works[index];
    await fs.writeFile(path.join(out, `${record.artifact}.status.json`), JSON.stringify({...record, result: 'success'}));
    await fs.writeFile(path.join(out, `${record.artifact}.manifest.json`), JSON.stringify(manifest));
    await fs.writeFile(path.join(out, `${record.artifact}.tar.zst`), 'archive');
  }
  if (ownerTiming) await fs.writeFile(path.join(out, 'owner-timing.json'), JSON.stringify(ownerTiming));
  const uploads = [];
  const ok = await publish({plan: {builds: records}, producer: 'linux', out, verifier, jobStatus: 'success',
    upload: async (name, files, root, options) => {
      const bodies = await Promise.all(files.map(f => fs.readFile(f, 'utf8')));
      uploads.push({name, files: files.map(f => path.relative(root, f)), bodies, options});
    }});
  const measurement = artifact => {
    const upload = uploads.find(u => u.name === artifact);
    const index = upload.files.indexOf(`${artifact}.compiler-work.json`);
    return index === -1 ? undefined : JSON.parse(upload.bodies[index]);
  };
  return {out, ok, records, uploads, measurement};
}

test('partial owners retain package identities, isolate uploads, and account for every key', async () => {
  const out = await fs.mkdtemp(path.join(os.tmpdir(), 'ci-artifacts-'));
  try {
    const verifier = path.join(out, 'ci-build');
    await fs.writeFile(verifier, 'verifier');
    const records = ['alpha', 'beta', 'gamma'].map(pkg => ({
      package: pkg, producer: 'linux', key: pkg, artifact: `build-${pkg}-linux-${pkg}`, consumers: []
    }));
    for (const r of records.slice(0, 2)) {
      await fs.writeFile(path.join(out, `${r.artifact}.status.json`), JSON.stringify({...r, result: 'success'}));
      await fs.writeFile(path.join(out, `${r.artifact}.manifest.json`), JSON.stringify({archive: {file: `${r.artifact}.tar.zst`}, sidecars: [], compiler_work: documents.alpha}));
      await fs.writeFile(path.join(out, `${r.artifact}.tar.zst`), 'archive');
    }
    const uploads = [];
    const result = await publish({plan: {builds: records}, producer: 'linux', out, verifier,
      jobStatus: 'failure', queueSeconds: 2, upload: async (name, files, root, options) => {
        if (name === records[0].artifact) throw new Error('upload interrupted');
        const bodies = await Promise.all(files.map(f => fs.readFile(f, 'utf8')));
        uploads.push({name, files: files.map(f => path.relative(root, f)), bodies, options});
      }});
    assert.equal(result, false);
    const archive = uploads.find(u => u.name === records[1].artifact);
    assert.equal(archive.options.retentionDays, 1);
    assert(archive.files.includes('tools/ci-build'));
    assert(archive.files.includes(`${records[1].artifact}.compiler-work.json`));
    assert(!archive.files.some(f => f.includes('alpha') || f.includes('gamma')));
    const statuses = uploads.filter(u => u.name.startsWith('build-status-'));
    assert.equal(statuses.length, 3);
    const [alpha, beta, gamma] = statuses.map(u => JSON.parse(u.bodies[0]));
    assert.equal(alpha.stage, 'upload'); assert.equal(alpha.result, 'failure');
    assert.equal(beta.result, 'success');
    assert.equal(gamma.stage, 'produce'); assert.equal(gamma.result, 'failure');
    // The rollup reads both windows as integers (`ProducerStageSeconds`); a
    // fractional upload window fails every area's audit.
    for (const status of [alpha, beta, gamma]) {
      assert(Number.isInteger(status.stage_seconds.upload_seconds), `upload_seconds must be whole seconds: ${JSON.stringify(status.stage_seconds)}`);
      assert.equal(status.stage_seconds.queue_seconds, 2);
    }
  } finally { await fs.rm(out, {recursive: true, force: true}); }
});

test('a declared sidecar is uploaded beside the archive, read from the manifest shape ci-build writes', async () => {
  const out = await fs.mkdtemp(path.join(os.tmpdir(), 'ci-artifacts-'));
  try {
    const verifier = path.join(out, 'ci-build');
    await fs.writeFile(verifier, 'verifier');
    const record = {package: 'darkmatter', producer: 'linux', key: 'k', artifact: 'build-darkmatter-linux-k', consumers: []};
    // `NamedFile` flattens its `FileRecord`, so `file`, `bytes`, and `blake3`
    // sit beside `name`. Run 35326800778 published nine sidecar-declaring
    // packages as `stage=upload` failures by reading a nested `record.file`.
    const sidecar = {name: 'darkmatter-md', file: 'sidecars/md', bytes: 7, blake3: 'digest'};
    await fs.writeFile(path.join(out, `${record.artifact}.status.json`), JSON.stringify({...record, result: 'success'}));
    await fs.writeFile(path.join(out, `${record.artifact}.manifest.json`),
      JSON.stringify({archive: {file: `${record.artifact}.tar.zst`}, sidecars: [sidecar]}));
    await fs.writeFile(path.join(out, `${record.artifact}.tar.zst`), 'archive');
    await fs.mkdir(path.join(out, 'sidecars'), {recursive: true});
    await fs.writeFile(path.join(out, sidecar.file), 'sidecar');
    const uploads = [];
    const ok = await publish({plan: {builds: [record]}, producer: 'linux', out, verifier, jobStatus: 'success',
      upload: async (name, files, root, options) => {
        const bodies = await Promise.all(files.map(f => fs.readFile(f, 'utf8')));
        uploads.push({name, files: files.map(f => path.relative(root, f)), bodies, options});
      }});
    assert.equal(ok, true);
    const archive = uploads.find(u => u.name === record.artifact);
    assert(archive.files.includes(sidecar.file), `sidecar missing from ${JSON.stringify(archive.files)}`);
    assert.equal(archive.bodies[archive.files.indexOf(sidecar.file)], 'sidecar');
    const status = JSON.parse(uploads.find(u => u.name.startsWith('build-status-')).bodies[0]);
    assert.equal(status.result, 'success');
  } finally { await fs.rm(out, {recursive: true, force: true}); }
});

test("the owner aggregate is the sum of its records' producer-written compiler work", async () => {
  const {out, ok, records, measurement} = await publishMeasured({
    works: [documents.alpha, documents.beta, null],
    ownerTiming: {produce_wall_seconds: 41}
  });
  try {
    assert.equal(ok, true);
    const owner = {
      producer: 'linux',
      requested_records: 3,
      measured_records: 2,
      compiler_invocations: documents.alpha.compiler_invocations + documents.beta.compiler_invocations,
      compiler_ms: documents.alpha.compiler_ms + documents.beta.compiler_ms,
      produce_wall_seconds: 41
    };
    const first = measurement(records[0].artifact);
    assert.deepEqual(first.record, documents.alpha, "the record block is the manifest's document verbatim");
    assert.deepEqual(first.owner, owner);
    assertOwnerMeasuredSomething(first.owner);
    // Compilation is per owner, so every measured record republishes the same
    // aggregate beside its own counts.
    const second = measurement(records[1].artifact);
    assert.deepEqual(second.record, documents.beta);
    assert.deepEqual(second.owner, owner);
    // An unmeasured record publishes no measurement, which is what holds
    // `measured_records` below `requested_records`.
    assert.equal(measurement(records[2].artifact), undefined);
  } finally { await fs.rm(out, {recursive: true, force: true}); }
});

test('a measurement that counts nothing cannot pass for compile-once evidence', async () => {
  const noDuration = {...documents.alpha};
  delete noDuration.compiler_ms;
  // Every one of these satisfied the earlier contract, which asserted only that
  // a measurement filename was uploaded.
  const scenarios = [
    {name: 'a pre-schema field name', works: [{invocations: 1}, {invocations: 1}],
      timing: {produce_wall_seconds: 9}, refusal: /no rustc invocation/},
    {name: 'probe-only compilation', works: [documents.probes_only, documents.probes_only],
      timing: {produce_wall_seconds: 9}, refusal: /no rustc invocation/},
    {name: 'missing duration data', works: [noDuration, noDuration],
      timing: {produce_wall_seconds: 9}, refusal: /no compiler time/},
    {name: 'a lost owner timing', works: [documents.alpha], timing: null,
      refusal: /owner timing was not merged/}
  ];
  for (const scenario of scenarios) {
    const {out, records, measurement} = await publishMeasured({works: scenario.works, ownerTiming: scenario.timing});
    try {
      const published = measurement(records[0].artifact);
      assert.deepEqual(published.record, scenario.works[0], `${scenario.name}: the record block is published verbatim`);
      assert.equal(published.owner.measured_records, scenario.works.length,
        `${scenario.name}: a present but empty document is still a measured record`);
      assert.throws(() => assertOwnerMeasuredSomething(published.owner), scenario.refusal, scenario.name);
    } finally { await fs.rm(out, {recursive: true, force: true}); }
  }
});
