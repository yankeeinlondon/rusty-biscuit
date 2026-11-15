const {spawn} = require('node:child_process');
const fs = require('node:fs');
const fsp = require('node:fs/promises');
const path = require('node:path');
const {finalizeOwner, loadLedger, publishRecord, stageVerifier} = require('./publish.cjs');

// The owner's records in `ci-build produce`'s own order: Rust compares
// `(package, key)` as UTF-8 bytes, which `localeCompare` and UTF-16 `<` do not.
function byPackageThenKey(left, right) {
  return Buffer.compare(Buffer.from(left.package), Buffer.from(right.package))
    || Buffer.compare(Buffer.from(left.key), Buffer.from(right.key));
}

// Produce one record at a time into the shared target tree and publish it
// before the next starts, so a completed archive leaves the runner's disk as
// soon as its upload is recorded. A failed child or upload is remembered and
// the remaining records are still attempted. Statuses the producer never wrote
// are left to `finalizeOwner`.
async function produceOwner({plan, producer, out, verifier, upload, runRecord, queueSeconds, measure}) {
  const records = plan.builds.filter(r => r.producer === producer).sort(byPackageThenKey);
  const started = Date.now();
  await fsp.mkdir(out, {recursive: true});
  const ledger = await loadLedger(out);
  const verifierCopy = await stageVerifier(out, verifier);
  let ok = true;
  for (const record of records) {
    let code;
    try { code = await runRecord(record); }
    catch (error) { code = 1; process.stderr.write(`${record.artifact}: ${error}\n`); }
    if (code !== 0) ok = false;
    if (!await publishRecord({record, producer, out, upload, ledger, verifierCopy, queueSeconds})) ok = false;
  }
  if (measure) {
    await fsp.writeFile(path.join(out, 'owner-timing.json'),
      JSON.stringify({produce_wall_seconds: Math.round((Date.now() - started) / 1000)}));
  }
  return ok;
}

// `bash` from PATH, resolved here rather than by the OS: on a Windows runner
// the system directory holds the WSL launcher under the same name.
function findBash() {
  const file = process.platform === 'win32' ? 'bash.exe' : 'bash';
  for (const dir of (process.env.PATH || '').split(path.delimiter)) {
    if (dir && fs.existsSync(path.join(dir, file))) return path.join(dir, file);
  }
  return 'bash';
}

// One `produce-owner.sh --key` child per record, in record mode: the
// coordinator, not the wrapper, owns the owner's timing.
function recordRunner({script, planPath, producer, out, measure, extraArgs = [], cwd}) {
  const bash = findBash();
  return record => new Promise((resolve, reject) => {
    const child = spawn(bash, [script, '--key', record.key, ...extraArgs], {
      cwd, stdio: 'inherit',
      env: {...process.env, PRODUCE_MODE: 'record', PRODUCER: producer, CI_BUILD_PLAN: planPath,
        CI_BUILD_OUT: out, MEASURE: measure ? 'true' : 'false'}
    });
    child.on('error', reject);
    child.on('exit', (code, signal) => resolve(signal ? 1 : code));
  });
}

// The build job's arguments, shared by its produce and finalize steps.
// `@actions/artifact` reads the runtime token GitHub gives only to JavaScript
// actions, so it is loaded here, never by the local CLI.
function hostedOptions(env = process.env) {
  const root = env.GITHUB_WORKSPACE;
  const {DefaultArtifactClient} = require('@actions/artifact');
  const client = new DefaultArtifactClient();
  const planPath = path.join(root, 'ci-artifacts/ci-resolved-plan/resolved-plan.json');
  const out = path.join(env.RUNNER_TEMP, 'build');
  const measure = env.MEASURE === 'true';
  return {
    plan: JSON.parse(fs.readFileSync(planPath, 'utf8')),
    producer: env.PRODUCER,
    out,
    verifier: path.join(root, 'target/release/ci-build' + (process.platform === 'win32' ? '.exe' : '')),
    jobStatus: env.JOB_STATUS,
    queueSeconds: env.PLAN_EPOCH && env.JOB_EPOCH ? Number(env.JOB_EPOCH) - Number(env.PLAN_EPOCH) : undefined,
    measure,
    upload: (...args) => client.uploadArtifact(...args),
    runRecord: recordRunner({script: path.join(root, 'scripts/ci/produce-owner.sh'), planPath,
      producer: env.PRODUCER, out, measure, cwd: root}),
  };
}

// A local transport standing in for the artifact service: each artifact's
// files land under `<dir>/<name>/`, and a name already used is refused, as the
// service refuses a second upload of an immutable name.
function directoryUpload(dir) {
  return async (name, files, root) => {
    const destination = path.join(dir, name);
    await fsp.mkdir(dir, {recursive: true});
    await fsp.mkdir(destination);
    for (const file of files) {
      const target = path.join(destination, path.relative(root, file));
      await fsp.mkdir(path.dirname(target), {recursive: true});
      await fsp.copyFile(file, target);
    }
  };
}

async function main(argv) {
  const separator = argv.indexOf('--');
  const own = separator === -1 ? argv : argv.slice(0, separator);
  const extraArgs = separator === -1 ? [] : argv.slice(separator + 1);
  const options = {measure: false};
  for (let index = 0; index < own.length; index++) {
    const flag = own[index];
    if (flag === '--measure') { options.measure = true; continue; }
    const name = {'--plan': 'plan', '--producer': 'producer', '--out': 'out',
      '--verifier': 'verifier', '--upload-dir': 'uploadDir'}[flag];
    if (!name || index + 1 >= own.length) throw new Error(`unexpected argument ${flag}`);
    options[name] = own[++index];
  }
  for (const name of ['plan', 'producer', 'out', 'verifier', 'uploadDir']) {
    if (!options[name]) throw new Error(`missing --${name.replace('uploadDir', 'upload-dir')}`);
  }
  const planPath = path.resolve(options.plan);
  const out = path.resolve(options.out);
  const common = {
    plan: JSON.parse(await fsp.readFile(planPath, 'utf8')),
    producer: options.producer, out, verifier: path.resolve(options.verifier),
    upload: directoryUpload(path.resolve(options.uploadDir)),
  };
  const produced = await produceOwner({...common, measure: options.measure,
    runRecord: recordRunner({script: path.join(__dirname, '..', 'produce-owner.sh'), planPath,
      producer: options.producer, out, measure: options.measure, extraArgs})});
  const finalized = await finalizeOwner({...common, jobStatus: produced ? 'success' : 'failure'});
  return produced && finalized;
}

if (require.main === module) {
  main(process.argv.slice(2)).then(
    ok => { process.exitCode = ok ? 0 : 1; },
    error => { process.stderr.write(`${error.stack || error}\n`); process.exitCode = 2; });
}

module.exports = {produceOwner, hostedOptions, recordRunner, directoryUpload, byPackageThenKey};
