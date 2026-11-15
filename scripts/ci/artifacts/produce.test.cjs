const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const {produceOwner, byPackageThenKey, directoryUpload} = require('./produce.cjs');
const {finalizeOwner} = require('./publish.cjs');

const exists = file => fs.access(file).then(() => true, () => false);

// An owner whose records the fake producer below can write. `fails` names the
// packages whose child exits nonzero after writing a failure status, and
// `silent` those whose child exits nonzero without writing anything.
async function owner(packages) {
  const out = await fs.mkdtemp(path.join(os.tmpdir(), 'ci-produce-'));
  const verifier = path.join(out, 'ci-build');
  await fs.writeFile(verifier, 'verifier');
  const builds = packages.map(pkg => ({
    package: pkg, producer: 'linux', key: `k-${pkg}`, artifact: `build-${pkg}-linux-k-${pkg}`, consumers: []
  }));
  // Another owner's record is never produced or published here.
  builds.push({package: 'other', producer: 'macos', key: 'k', artifact: 'build-other-macos-k', consumers: []});
  return {out, verifier, plan: {builds}};
}

function fakeProducer(out, {fails = [], silent = [], events = []} = {}) {
  return async record => {
    events.push(`run ${record.package}`);
    if (silent.includes(record.package)) return 1;
    const result = fails.includes(record.package) ? 'failure' : 'success';
    await fs.writeFile(path.join(out, `${record.artifact}.status.json`),
      JSON.stringify({schema_version: 1, ...record, result, stage: result === 'success' ? 'produce' : 'compile'}));
    if (result === 'failure') return 1;
    await fs.writeFile(path.join(out, `${record.artifact}.manifest.json`),
      JSON.stringify({archive: {file: `${record.artifact}.tar.zst`}, sidecars: [], compiler_work: {compiler_invocations: 1, compiler_ms: 5}}));
    await fs.writeFile(path.join(out, `${record.artifact}.tar.zst`), 'archive');
    return 0;
  };
}

test("records are produced in ci-build's byte order, each published before the next starts", async () => {
  // `Beta` sorts before `alpha` by bytes; `é` (0xC3) sorts after `z`. No two
  // names differ only by case: macOS and Windows file names would collide.
  const {out, verifier, plan} = await owner(['zeta', 'alpha', 'Beta', 'éclair']);
  try {
    const events = [];
    const upload = async name => { events.push(`upload ${name}`); };
    const ok = await produceOwner({plan, producer: 'linux', out, verifier, upload,
      runRecord: fakeProducer(out, {events})});
    assert.equal(ok, true);
    assert.deepEqual(events.filter(e => e.startsWith('run ')).map(e => e.slice(4)), ['Beta', 'alpha', 'zeta', 'éclair']);
    assert.deepEqual(events, ['Beta', 'alpha', 'zeta', 'éclair'].flatMap(pkg => [
      `run ${pkg}`, `upload build-${pkg}-linux-k-${pkg}`, `upload build-status-${pkg}-linux-k-${pkg}`]));
  } finally { await fs.rm(out, {recursive: true, force: true}); }
});

test('the comparator matches Rust byte order on the key once packages tie', () => {
  const records = [{package: 'a', key: 'b'}, {package: 'a', key: 'B'}, {package: 'a', key: 'ä'}];
  assert.deepEqual(records.sort(byPackageThenKey).map(r => r.key), ['B', 'b', 'ä']);
});

test('an archive is deleted only after its upload resolves and the ledger records it', async () => {
  const {out, verifier, plan} = await owner(['alpha', 'beta']);
  const archive = path.join(out, 'build-alpha-linux-k-alpha.tar.zst');
  try {
    let release;
    const pending = new Promise(resolve => { release = resolve; });
    let observed;
    const upload = async name => {
      if (name !== 'build-alpha-linux-k-alpha') return;
      observed = true;
      await pending;
    };
    const runs = [];
    const producer = fakeProducer(out);
    const running = produceOwner({plan, producer: 'linux', out, verifier, upload,
      runRecord: async record => { runs.push(record.package); return producer(record); }});
    while (!observed) await new Promise(resolve => setImmediate(resolve));
    for (let turn = 0; turn < 20; turn++) await new Promise(resolve => setImmediate(resolve));
    assert(await exists(archive), 'the archive is retained while its upload is pending');
    assert.deepEqual(runs, ['alpha'], 'the next record waits for this one to be published');
    assert(!await exists(path.join(out, 'publication-ledger.json')), 'nothing is recorded before the upload resolves');
    release();
    assert.equal(await running, true);
    assert(!await exists(archive), 'the uploaded archive is released');
    const ledger = JSON.parse(await fs.readFile(path.join(out, 'publication-ledger.json'), 'utf8'));
    assert.deepEqual(ledger.records['build-alpha-linux-k-alpha'], {archive: true, status: true});
    assert.deepEqual(runs, ['alpha', 'beta']);
  } finally { await fs.rm(out, {recursive: true, force: true}); }
});

test('a successful record beside a failed one and a silent child is still published', async () => {
  const {out, verifier, plan} = await owner(['alpha', 'beta', 'gamma']);
  try {
    const published = {};
    const upload = async (name, files) => {
      published[name] = name.startsWith('build-status-') ? JSON.parse(await fs.readFile(files[0], 'utf8')) : true;
    };
    const ok = await produceOwner({plan, producer: 'linux', out, verifier, upload, queueSeconds: 4,
      runRecord: fakeProducer(out, {fails: ['alpha'], silent: ['beta']})});
    assert.equal(ok, false, 'a nonzero child fails the owner');
    assert.equal(published['build-status-alpha-linux-k-alpha'].stage, 'compile');
    assert.equal(published['build-gamma-linux-k-gamma'], true);
    assert.equal(published['build-status-gamma-linux-k-gamma'].result, 'success');
    assert.equal(published['build-status-gamma-linux-k-gamma'].stage_seconds.queue_seconds, 4);
    assert.equal(published['build-status-beta-linux-k-beta'], undefined, 'an unwritten status is left to the finalizer');

    const finalized = await finalizeOwner({plan, producer: 'linux', out, verifier, upload, jobStatus: 'failure'});
    assert.equal(finalized, false);
    const beta = published['build-status-beta-linux-k-beta'];
    assert.equal(beta.result, 'failure');
    assert.equal(beta.detail, 'owner concluded failure before writing a status');
    assert.equal(Object.keys(published).filter(n => n.startsWith('build-status-')).length, 3);
    assert(!Object.keys(published).some(n => n.includes('other')), "another owner's record is never published");
  } finally { await fs.rm(out, {recursive: true, force: true}); }
});

test('records the owner never reached get synthesized statuses, and the owner measurement is published once', async () => {
  const {out, verifier, plan} = await owner(['alpha', 'beta']);
  const uploads = path.join(out, 'uploaded');
  try {
    const transport = directoryUpload(uploads);
    const names = [];
    const upload = async (name, ...rest) => { names.push(name); return transport(name, ...rest); };
    const producer = fakeProducer(out);
    // The runner gives out after alpha: beta is never produced.
    const ok = await produceOwner({plan, producer: 'linux', out, verifier, upload, measure: true,
      runRecord: async record => {
        if (record.package === 'beta') throw new Error('runner lost');
        return producer(record);
      }});
    assert.equal(ok, false);
    assert(await exists(path.join(out, 'owner-timing.json')), 'the coordinator owns the owner timing');

    assert.equal(await finalizeOwner({plan, producer: 'linux', out, verifier, upload, jobStatus: 'cancelled'}), false);
    const beta = JSON.parse(await fs.readFile(
      path.join(uploads, 'build-status-beta-linux-k-beta', 'build-status.json'), 'utf8'));
    assert.equal(beta.result, 'cancelled');
    const measured = JSON.parse(await fs.readFile(
      path.join(uploads, 'compiler-work-owner-linux', 'owner-compiler-work.json'), 'utf8'));
    assert.equal(measured.requested_records, 2);
    assert.equal(measured.measured_records, 1);
    assert(Number.isFinite(measured.produce_wall_seconds));
    const published = names.length;
    assert.equal(new Set(names).size, published, `no name was uploaded twice: ${names}`);
    // beta's synthesized status is a failure, so the replay still reports one,
    // but it attempts no upload.
    assert.equal(await finalizeOwner({plan, producer: 'linux', out, verifier, upload, jobStatus: 'cancelled'}), false);
    assert.equal(names.length, published, `a replay publishes nothing: ${names}`);
    assert(await exists(path.join(uploads, 'build-alpha-linux-k-alpha', 'tools', 'ci-build')),
      'the verifier travels with every archive');
    assert(await exists(path.join(out, 'build-alpha-linux-k-alpha.manifest.json')));
  } finally { await fs.rm(out, {recursive: true, force: true}); }
});
