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
  // The owner aggregate is final only after every record, so it travels in its
  // own artifact rather than beside any one record's archive.
  const ownerUpload = () => {
    const upload = uploads.find(u => u.name === 'compiler-work-owner-linux');
    return upload && JSON.parse(upload.bodies[upload.files.indexOf('owner-compiler-work.json')]);
  };
  return {out, ok, records, uploads, measurement, ownerUpload};
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
  const {out, ok, records, uploads, measurement, ownerUpload} = await publishMeasured({
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
    assert.deepEqual(ownerUpload(), owner);
    assertOwnerMeasuredSomething(ownerUpload());
    assert.equal(uploads.filter(u => u.name === 'compiler-work-owner-linux').length, 1,
      'the owner aggregate is published exactly once');
    // A record's own artifact carries only its own counts: published before
    // later records ran, it cannot hold a final owner total.
    assert.deepEqual(measurement(records[0].artifact), {record: documents.alpha},
      "the record block is the manifest's document verbatim");
    assert.deepEqual(measurement(records[1].artifact), {record: documents.beta});
    // An unmeasured record publishes no measurement, which is what holds
    // `measured_records` below `requested_records`.
    assert.equal(measurement(records[2].artifact), undefined);
  } finally { await fs.rm(out, {recursive: true, force: true}); }
});

test('an owner with no measured record publishes no owner measurement', async () => {
  const {out, ok, uploads} = await publishMeasured({works: [null, null], ownerTiming: {produce_wall_seconds: 3}});
  try {
    assert.equal(ok, true);
    assert(!uploads.some(u => u.name.startsWith('compiler-work-owner-')), JSON.stringify(uploads.map(u => u.name)));
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
    const {out, records, measurement, ownerUpload} = await publishMeasured({works: scenario.works, ownerTiming: scenario.timing});
    try {
      assert.deepEqual(measurement(records[0].artifact), {record: scenario.works[0]},
        `${scenario.name}: the record block is published verbatim`);
      const owner = ownerUpload();
      assert.equal(owner.measured_records, scenario.works.length,
        `${scenario.name}: a present but empty document is still a measured record`);
      assert.throws(() => assertOwnerMeasuredSomething(owner), scenario.refusal, scenario.name);
    } finally { await fs.rm(out, {recursive: true, force: true}); }
  }
});

// A finished producer leg: every record has a successful status, a manifest, an
// archive, and a record-owned copied-sidecar directory.
async function producedOwner(packages) {
  const out = await fs.mkdtemp(path.join(os.tmpdir(), 'ci-artifacts-'));
  const verifier = path.join(out, 'ci-build');
  await fs.writeFile(verifier, 'verifier');
  const records = packages.map(pkg => ({
    package: pkg, producer: 'linux', key: pkg, artifact: `build-${pkg}-linux-${pkg}`, consumers: []
  }));
  for (const r of records) {
    const sidecar = {name: 'md', file: `${r.artifact}-sidecars/md`, bytes: 7, blake3: 'digest'};
    await fs.mkdir(path.join(out, `${r.artifact}-sidecars`));
    await fs.writeFile(path.join(out, sidecar.file), 'sidecar');
    await fs.writeFile(path.join(out, `${r.artifact}.status.json`), JSON.stringify({...r, result: 'success'}));
    await fs.writeFile(path.join(out, `${r.artifact}.manifest.json`),
      JSON.stringify({archive: {file: `${r.artifact}.tar.zst`}, sidecars: [sidecar]}));
    await fs.writeFile(path.join(out, `${r.artifact}.tar.zst`), 'archive');
  }
  return {out, verifier, records, plan: {builds: records}};
}

const exists = file => fs.access(file).then(() => true, () => false);

test('a failed status upload is retried alone: the immutable archive is never uploaded twice', async () => {
  const {out, verifier, records, plan} = await producedOwner(['alpha']);
  const [record] = records;
  try {
    const names = [];
    let refuseStatus = true;
    const upload = async name => {
      if (name.startsWith('build-status-') && refuseStatus) throw new Error('status upload interrupted');
      names.push(name);
    };
    assert.equal(await publish({plan, producer: 'linux', out, verifier, upload, jobStatus: 'success'}), false);
    assert.deepEqual(names, [record.artifact]);
    const ledger = JSON.parse(await fs.readFile(path.join(out, 'publication-ledger.json'), 'utf8'));
    assert.deepEqual(ledger.records[record.artifact], {archive: true});

    refuseStatus = false;
    assert.equal(await publish({plan, producer: 'linux', out, verifier, upload, jobStatus: 'success'}), true);
    assert.deepEqual(names, [record.artifact, `build-status-alpha-linux-alpha`],
      'the retry uploads only the outstanding status');
    const status = JSON.parse(await fs.readFile(path.join(out, 'build-status-alpha-linux-alpha', 'build-status.json'), 'utf8'));
    assert.equal(status.result, 'success');
  } finally { await fs.rm(out, {recursive: true, force: true}); }
});

test('replaying the finalizer after a complete publication uploads nothing', async () => {
  const {out, verifier, records, plan} = await producedOwner(['alpha', 'beta']);
  try {
    const names = [];
    const upload = async name => { names.push(name); };
    assert.equal(await publish({plan, producer: 'linux', out, verifier, upload, jobStatus: 'success'}), true);
    assert.equal(names.length, 4);
    assert.equal(await publish({plan, producer: 'linux', out, verifier, upload, jobStatus: 'success'}), true);
    assert.equal(names.length, 4, `nothing is published twice: ${JSON.stringify(names)}`);
    // What a consumer or a later diagnosis reads stays on disk; only the
    // uploaded payload is released.
    for (const r of records) {
      assert(await exists(path.join(out, `${r.artifact}.manifest.json`)));
      assert(await exists(path.join(out, `${r.artifact}.status.json`)));
      assert(!await exists(path.join(out, `${r.artifact}.tar.zst`)));
      assert(!await exists(path.join(out, `${r.artifact}-sidecars`)));
    }
    assert(await exists(path.join(out, 'tools', 'ci-build')), 'the staged verifier is kept');
    assert(await exists(verifier), 'the verifier itself is never touched');
  } finally { await fs.rm(out, {recursive: true, force: true}); }
});

test('a failed archive upload keeps its payload and publishes an upload failure beside a successful sibling', async () => {
  const {out, verifier, records, plan} = await producedOwner(['alpha', 'beta']);
  const [alpha, beta] = records;
  try {
    const statuses = {};
    const upload = async (name, files) => {
      if (name === alpha.artifact) throw new Error('upload interrupted');
      if (name.startsWith('build-status-')) statuses[name] = JSON.parse(await fs.readFile(files[0], 'utf8'));
    };
    assert.equal(await publish({plan, producer: 'linux', out, verifier, upload, jobStatus: 'success'}), false);
    assert.equal(statuses['build-status-alpha-linux-alpha'].stage, 'upload');
    assert.match(statuses['build-status-alpha-linux-alpha'].detail, /upload interrupted/);
    assert.equal(statuses['build-status-beta-linux-beta'].result, 'success');
    assert(await exists(path.join(out, `${alpha.artifact}.tar.zst`)), 'a payload that was not uploaded is kept');
    assert(await exists(path.join(out, `${alpha.artifact}-sidecars`, 'md')));
    assert(!await exists(path.join(out, `${beta.artifact}.tar.zst`)));
  } finally { await fs.rm(out, {recursive: true, force: true}); }
});

test('an uploaded payload outside the output directory or behind a symbolic link is not deleted', async () => {
  const {out, verifier, records, plan} = await producedOwner(['alpha', 'beta', 'gamma']);
  const [alpha, beta, gamma] = records;
  const elsewhere = await fs.mkdtemp(path.join(os.tmpdir(), 'ci-artifacts-elsewhere-'));
  try {
    // alpha's manifest names an archive beside `out`, not in it.
    const escaped = path.join(elsewhere, 'escaped.tar.zst');
    await fs.writeFile(escaped, 'archive');
    await fs.writeFile(path.join(out, `${alpha.artifact}.manifest.json`), JSON.stringify(
      {archive: {file: path.relative(out, escaped)}, sidecars: []}));
    // beta's archive is a link to a file outside `out`.
    const target = path.join(elsewhere, 'linked.tar.zst');
    await fs.writeFile(target, 'archive');
    const link = path.join(out, `${beta.artifact}.tar.zst`);
    await fs.rm(link);
    let linked = true;
    try { await fs.symlink(target, link, 'file'); }
    catch (error) { if (error.code !== 'EPERM') throw error; linked = false; } // unprivileged Windows
    // gamma's archive sits under a directory that links outside `out`.
    const parked = path.join(elsewhere, 'parked');
    await fs.mkdir(parked);
    await fs.writeFile(path.join(parked, 'gamma.tar.zst'), 'archive');
    if (linked) {
      await fs.symlink(parked, path.join(out, 'parked'), 'junction');
      await fs.writeFile(path.join(out, `${gamma.artifact}.manifest.json`), JSON.stringify(
        {archive: {file: path.join('parked', 'gamma.tar.zst')}, sidecars: []}));
    }
    const ok = await publish({plan, producer: 'linux', out, verifier, upload: async () => {}, jobStatus: 'success'});
    assert.equal(ok, true, 'a refused deletion does not change the record outcome');
    assert(await exists(escaped), 'a path outside the output directory is never deleted');
    if (linked) {
      assert(await exists(link), 'a symbolic link is not deleted');
      assert(await exists(target), 'nor is what it points at');
      assert(await exists(path.join(parked, 'gamma.tar.zst')), 'a path that resolves outside through a parent is kept');
    }
  } finally {
    await fs.rm(out, {recursive: true, force: true});
    await fs.rm(elsewhere, {recursive: true, force: true});
  }
});
