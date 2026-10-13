'use strict';

const baseVersion = '0.1.0';
const expectedUpdatePaths = [
  '.release-please-manifest.json',
  'CHANGELOG.md',
  'Cargo.lock',
  'Cargo.toml',
  'version.txt',
];
const forbiddenIo = [];
const initialSha = '1111111111111111111111111111111111111111';
const mergedSha = 'ffffffffffffffffffffffffffffffffffffffff';
const pendingLabel = 'autorelease: pending';
const taggedLabel = 'autorelease: tagged';
const { after, afterEach, before, mock, test } = require('node:test');
const assert = require('node:assert/strict');
const childProcess = require('node:child_process');
const crypto = require('node:crypto');
const fs = require('node:fs');
const http = require('node:http');
const https = require('node:https');
const net = require('node:net');
const path = require('node:path');
const releaseLogger = require('release-please/build/src/util/logger');
const tls = require('node:tls');
const toml = require('@iarna/toml');
const originalLogger = releaseLogger.logger;
const quietLogger = Object.fromEntries(
  ['debug', 'error', 'info', 'trace', 'warn'].map(level => [level, () => {}]),
);
const root = path.resolve(__dirname, '..', '..');
const workspace = loadWorkspace();

before(() => {
  for (const [target, method] of [
    [globalThis, 'fetch'],
    [http, 'get'],
    [http, 'request'],
    [https, 'get'],
    [https, 'request'],
    [net, 'connect'],
    [net, 'createConnection'],
    [net.Socket.prototype, 'connect'],
    [tls, 'connect'],
  ]) {
    mock.method(target, method, () => unexpectedIo(`network: ${method}`));
  }
  releaseLogger.setLogger(quietLogger);
});

afterEach(() => {
  assert.deepEqual(forbiddenIo, [], 'No unexpected GitHub access or network is allowed');
});

after(() => {
  releaseLogger.setLogger(originalLogger);
  mock.restoreAll();
});

function applyCandidate(candidate, files) {
  const logger = {
    ...quietLogger,
    error: (...messages) => assert.fail(messages.join(' ')),
    warn: (...messages) => assert.fail(messages.join(' ')),
  };
  const updated = new Map(files);

  assert.deepEqual(candidate.updates.map(update => update.path).sort(), expectedUpdatePaths);
  for (const update of candidate.updates) {
    assert.equal(update.createIfMissing, update.path === 'CHANGELOG.md');
    assert.ok(files.has(update.path), `Missing fixture file: ${update.path}`);
    updated.set(update.path, update.updater.updateContent(files.get(update.path), logger));
  }
  return updated;
}

function assertAligned(beforeFiles, afterFiles, version) {
  const afterRoot = toml.parse(afterFiles.get('Cargo.toml'));
  const beforeRoot = toml.parse(beforeFiles.get('Cargo.toml'));
  const expectedRoot = structuredClone(beforeRoot);

  expectedRoot.workspace.package.version = version;
  assert.deepEqual(afterRoot, expectedRoot);
  assert.equal(afterRoot.workspace.package.publish, false);
  assert.equal(afterFiles.get('Cargo.toml'), expectedRootUpdate(beforeFiles.get('Cargo.toml'), version));
  assert.equal(afterFiles.get('version.txt'), `${version}\n`);
  assert.deepEqual(JSON.parse(afterFiles.get('.release-please-manifest.json')), { '.': version });
  assertLockfile(beforeFiles.get('Cargo.lock'), afterFiles.get('Cargo.lock'), version);
  assert.deepEqual([...afterFiles.keys()], [...beforeFiles.keys()]);
  for (const [filename, content] of beforeFiles) {
    if (!expectedUpdatePaths.includes(filename)) {
      assert.equal(afterFiles.get(filename), content, `${filename} must remain byte-identical`);
    }
  }
  for (const member of workspace.members) {
    const manifest = toml.parse(afterFiles.get(member.manifestPath));

    assert.deepEqual(manifest.package.publish, { workspace: true }, member.manifestPath);
    assert.deepEqual(manifest.package.version, { workspace: true }, member.manifestPath);
  }
}

function assertCandidate(candidate, version) {
  assert.equal(candidate.version.toString(), version);
  assert.equal(candidate.title.toString(), `chore(release): release ${version}`);
  assert.equal(candidate.headRefName, 'release-please--branches--main');
  assert.equal(candidate.draft, false);
  assert.deepEqual(candidate.labels, [pendingLabel]);
  assert.equal(candidate.body.releaseData.length, 1);
  assert.equal(candidate.body.releaseData[0].component, '');
  assert.equal(candidate.body.releaseData[0].version.toString(), version);
  assert.deepEqual(candidate.updates.map(update => update.path).sort(), expectedUpdatePaths);
}

function assertLockfile(before, after, version) {
  const afterData = toml.parse(after);
  const beforeData = toml.parse(before);
  const expected = structuredClone(beforeData);
  const local = beforeData.package.filter(entry => !Object.hasOwn(entry, 'source'));
  const names = workspace.members.map(member => member.name).sort();

  assert.equal(local.length, workspace.members.length);
  assert.deepEqual(local.map(entry => entry.name).sort(), names);
  for (const entry of expected.package) {
    if (!Object.hasOwn(entry, 'source')) {
      entry.version = version;
    }
  }
  assert.deepEqual(afterData, expected, 'Only source-less package versions may change');
  assert.equal(afterData.version, beforeData.version, 'Cargo lock format is not a package version');
  assert.equal(after, expectedLockUpdate(before, version), 'Preserve every other byte, including checksums');
}

async function buildCandidate(scenario, version) {
  const { manifest } = await manifestFor(scenario);
  const candidates = await manifest.buildPullRequests();

  assert.equal(candidates.length, 1);
  assertCandidate(candidates[0], version);
  return candidates[0];
}

function commit(message, files) {
  return {
    files,
    message,
    sha: crypto.createHash('sha1').update(`${message}\0${files.join('\0')}`).digest('hex'),
  };
}

function expectedLockUpdate(content, version) {
  return content.split(/(?=^\[\[package\]\]\r?$)/m).map(section => {
    if (!section.startsWith('[[package]]')) {
      return section;
    }
    const [entry] = toml.parse(section).package;

    if (Object.hasOwn(entry, 'source')) {
      return section;
    }
    assert.match(section, /^version[ \t]*=[ \t]*"[^"\r\n]+"/m);
    return section.replace(
      /^(version[ \t]*=[ \t]*")[^"\r\n]+(")/m,
      (_match, prefix, suffix) => `${prefix}${version}${suffix}`,
    );
  }).join('');
}

function expectedRootUpdate(content, version) {
  const pattern = /^(\[workspace\.package\]\r?\n[\s\S]*?^version[ \t]*=[ \t]*")[^"\r\n]+(")/m;

  assert.match(content, pattern);
  return content.replace(pattern, (_match, prefix, suffix) => `${prefix}${version}${suffix}`);
}

function fixture({ bootstrap = false, commits = [], version = baseVersion } = {}) {
  // Only names/paths are discovered; fixture versions and history never follow the live release.
  const lock = `# Preserve this comment, its 0.1.0, and the lock format.
version = 4

${workspace.members.map(member => `[[package]]
name = "${member.name}"
version = "${version}" # shared version
dependencies = [
 "mdview-core 0.1.0 (registry+https://github.com/rust-lang/crates.io-index)",
]
`).join('\n')}
[[package]]
name = "mdview-core"
version = "0.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"

[[package]]
name = "mdview-core"
version = "0.1.0"
source = "git+https://example.invalid/fixture#2222222222222222222222222222222222222222"
`;
  const rootManifest = `# Preserve this comment and its 0.1.0.
[workspace]
members = ["apps/*", "crates/*"]
resolver = "2"

[workspace.package]
edition = "2021"
publish = false
version = "${version}" # shared version

[workspace.dependencies]
mdview-core = { version = "0.1.0", features = ["fixture"] }

[workspace.metadata]
unchanged = "0.1.0"
`;
  const files = new Map([
    ['.release-please-manifest.json', bootstrap ? '{}\n' : JSON.stringify({ '.': version }, null, 2) + '\n'],
    ['CHANGELOG.md', bootstrap ? '' : `# Changelog\n\n## ${version} (2024-01-01)\n\n### Added\n\n* Fixture baseline entry.\n`],
    ['Cargo.lock', lock],
    ['Cargo.toml', rootManifest],
    ['release-please-config.json', workspace.files.get('release-please-config.json')],
    ['version.txt', `${version}\n`],
    ...workspace.members.map(member => [
      member.manifestPath,
      `[package]\nname = "${member.name}"\npublish.workspace = true\nversion.workspace = true\nedition = "2021"\n`,
    ]),
    ['apps/mdview/src/main.rs', 'fn main() {}\n'],
    ['lua/mdview/init.lua', 'return {}\n'],
    ['sidecar/src/index.ts', 'export {};\n'],
  ]);

  return {
    commits: bootstrap ? commits : [
      ...commits,
      { files: expectedUpdatePaths, message: `chore(release): release ${version}`, sha: initialSha },
    ],
    files,
    releases: bootstrap ? [] : [publishedRelease(version, initialSha)],
    tags: bootstrap ? [] : [{ name: `v${version}`, sha: initialSha }],
  };
}

function githubBlobFiles(files) {
  const blobs = new Map(files);

  // GitHub reads Git blobs, not autocrlf checkouts. Prove Git's clean result before normalizing.
  for (const filename of ['Cargo.lock', 'Cargo.toml']) {
    const content = files.get(filename);
    const normalized = content.replace(/\r\n/g, '\n');

    if (content !== normalized) {
      const cleanHash = childProcess.execFileSync('git', ['hash-object', '--path', filename, '--stdin'], {
        cwd: root, encoding: 'utf8', input: content,
      });
      const normalizedHash = childProcess.execFileSync('git', ['hash-object', '--no-filters', '--stdin'], {
        cwd: root, encoding: 'utf8', input: normalized,
      });

      assert.equal(cleanHash, normalizedHash, `${filename} must be stored as LF in Git`);
    }
    blobs.set(filename, normalized);
  }
  return blobs;
}

function githubReader({ commits = [], files, pullRequests = [], releases = [], tags = [] }) {
  const calls = [];
  const reads = {
    repository: { defaultBranch: 'main', owner: 'release-fixture', repo: 'mdview' },
    async getFileJson(filename, ref) {
      calls.push(['getFileJson', filename, ref]);
      assert.equal(ref, 'main');
      assert.ok(['.release-please-manifest.json', 'release-please-config.json'].includes(filename));
      assert.ok(files.has(filename), `Unexpected file read: ${filename}`);
      return JSON.parse(files.get(filename));
    },
    async *mergeCommitIterator(ref, options) {
      calls.push(['mergeCommitIterator', ref, options]);
      assert.equal(ref, 'main');
      assert.deepEqual(options, { maxResults: 500, backfillFiles: true, batchSize: 10 });
      yield* structuredClone(commits);
    },
    async *pullRequestIterator(ref, status, limit, includeFiles) {
      calls.push(['pullRequestIterator', ref, status, limit, includeFiles]);
      assert.deepEqual([ref, status, limit, includeFiles], ['main', 'MERGED', 200, false]);
      for (const record of pullRequests) {
        if (record.state === status) {
          yield structuredClone(record.pullRequest);
        }
      }
    },
    async *releaseIterator(options) {
      calls.push(['releaseIterator', options]);
      assert.deepEqual(options, { maxResults: 400 });
      yield* structuredClone(releases);
    },
    async *tagIterator(...args) {
      calls.push(['tagIterator']);
      assert.deepEqual(args, []);
      yield* structuredClone(tags);
    },
  };
  // This is the entire mocked boundary. No GitHub client, token, write method, or fallback exists.
  const github = new Proxy(Object.freeze(reads), {
    get(target, key) {
      return Object.hasOwn(target, key) ? target[key] : unexpectedIo(`GitHub read/write: ${String(key)}`);
    },
    set(_target, key) {
      return unexpectedIo(`GitHub mutation: ${String(key)}`);
    },
  });

  return { calls, github };
}

function loadWorkspace() {
  const cargo = toml.parse(readFile('Cargo.toml'));
  const manifestPaths = fs.globSync(
    cargo.workspace.members.map(member => path.join(member, 'Cargo.toml')),
    { cwd: root },
  ).map(filename => filename.split(path.sep).join('/')).sort();
  const files = new Map(
    [...expectedUpdatePaths, 'release-please-config.json', ...manifestPaths].map(filename => [filename, readFile(filename)]),
  );
  const members = manifestPaths.map(manifestPath => ({
    manifestPath,
    name: toml.parse(files.get(manifestPath)).package.name,
  }));

  return { files, members };
}

async function manifestFor(scenario) {
  const { Manifest } = require('release-please');
  const reader = githubReader(scenario);
  const manifest = await Manifest.fromManifest(
    reader.github,
    'main',
    'release-please-config.json',
    '.release-please-manifest.json',
    { logger: quietLogger },
  );

  return { ...reader, manifest };
}

function mergedPullRequest(candidate) {
  return {
    baseBranchName: 'main',
    body: candidate.body.toString(),
    files: candidate.updates.map(update => update.path),
    headBranchName: candidate.headRefName,
    labels: [...candidate.labels],
    mergeCommitOid: mergedSha,
    number: 46,
    sha: mergedSha,
    title: candidate.title.toString(),
  };
}

function publishedRelease(version, sha) {
  return {
    draft: false,
    id: 1,
    name: `v${version}`,
    notes: 'Previously published fixture release.',
    sha,
    tagName: `v${version}`,
    url: `https://github.com/release-fixture/mdview/releases/tag/v${version}`,
  };
}

function readFile(filename) {
  return fs.readFileSync(path.join(root, ...filename.split('/')), 'utf8');
}

function unexpectedIo(description) {
  forbiddenIo.push(description);
  assert.fail(description);
}

test('configuration parses as one simple product with the accepted release policy', async () => {
  const { calls, manifest } = await manifestFor(fixture());
  const config = JSON.parse(workspace.files.get('release-please-config.json'));
  const parsed = manifest.repositoryConfig['.'];

  assert.deepEqual(Object.keys(config.packages), ['.']);
  assert.deepEqual(Object.keys(manifest.repositoryConfig), ['.']);
  assert.deepEqual(config.plugins ?? [], []);
  assert.deepEqual(manifest.plugins, []);
  assert.equal(parsed.releaseType, 'simple');
  assert.equal(parsed.initialVersion, baseVersion);
  assert.equal(parsed.bumpMinorPreMajor, true);
  assert.equal(parsed.bumpPatchForMinorPreMajor, false);
  assert.equal(parsed.draft, false);
  assert.equal(parsed.prerelease, false);
  assert.equal(parsed.includeComponentInTag, false);
  assert.equal(parsed.includeVInReleaseName, true);
  assert.equal(parsed.includeVInTag, true);
  assert.equal(parsed.pullRequestTitlePattern, 'chore(release): release ${version}');
  assert.equal(parsed.changelogPath, 'CHANGELOG.md');
  assert.equal(parsed.releaseAs, undefined);
  assert.equal(parsed.skipGithubRelease, undefined);
  assert.equal(parsed.skipChangelog, undefined);
  assert.equal(parsed.versionFile, 'version.txt');
  assert.deepEqual(parsed.changelogSections, [
    { section: 'Added', type: 'feat' },
    { section: 'Fixed', type: 'fix' },
    { section: 'Changed', type: 'perf' },
  ]);
  assert.deepEqual(parsed.extraFiles, [
    { jsonpath: '$.package[?(!@.source)].version', path: 'Cargo.lock', type: 'toml' },
    { jsonpath: '$.workspace.package.version', path: 'Cargo.toml', type: 'toml' },
  ]);
  assert.deepEqual(calls.map(call => call.slice(0, 2)).sort(), [
    ['getFileJson', '.release-please-manifest.json'],
    ['getFileJson', 'release-please-config.json'],
  ]);
});

test('test dependencies are private, exactly pinned, and locked to the action library', () => {
  const installed = require('release-please/package.json');
  const lock = JSON.parse(fs.readFileSync(path.join(__dirname, 'package-lock.json'), 'utf8'));
  const packageJson = JSON.parse(fs.readFileSync(path.join(__dirname, 'package.json'), 'utf8'));

  assert.equal(packageJson.private, true);
  assert.deepEqual(packageJson.devDependencies, { '@iarna/toml': '2.2.5', 'release-please': '17.3.0' });
  assert.equal(installed.version, '17.3.0');
  assert.deepEqual(lock.packages[''].devDependencies, packageJson.devDependencies);
  assert.equal(lock.packages['node_modules/@iarna/toml'].version, '2.2.5');
  assert.equal(lock.packages['node_modules/release-please'].version, '17.3.0');
});

test('all discovered workspace members inherit non-publishable, synchronized versions', () => {
  const config = JSON.parse(workspace.files.get('release-please-config.json'));
  const manifest = JSON.parse(workspace.files.get('.release-please-manifest.json'));
  const rootManifest = toml.parse(workspace.files.get('Cargo.toml'));
  const version = rootManifest.workspace.package.version;

  assert.ok(workspace.members.length > 0);
  assert.equal(new Set(workspace.members.map(member => member.name)).size, workspace.members.length);
  assert.equal(rootManifest.workspace.package.publish, false);
  assert.match(version, /^\d+\.\d+\.\d+$/);
  assert.equal(workspace.files.get('version.txt').trim(), version);
  if (Object.keys(manifest).length === 0) {
    assert.equal(version, config.packages['.']['initial-version']);
    assert.equal(workspace.files.get('CHANGELOG.md').trim(), '');
  } else {
    assert.deepEqual(manifest, { '.': version });
  }
  for (const member of workspace.members) {
    const memberManifest = toml.parse(workspace.files.get(member.manifestPath));

    assert.deepEqual(memberManifest.package.publish, { workspace: true }, member.manifestPath);
    assert.deepEqual(memberManifest.package.version, { workspace: true }, member.manifestPath);
  }
  assertLockfile(workspace.files.get('Cargo.lock'), workspace.files.get('Cargo.lock'), version);
});

for (const { message, section, version } of [
  { message: 'feat: add fixture capability', section: 'Added', version: '0.2.0' },
  { message: 'fix: correct fixture rendering', section: 'Fixed', version: '0.1.1' },
  { message: 'perf: reduce fixture rendering cost', section: 'Changed', version: '0.1.1' },
]) {
  test(`${message.split(':')[0]} generates ${version} and the ${section} changelog section`, async () => {
    const change = commit(message, ['apps/mdview/src/main.rs']);
    const scenario = fixture({ commits: [change] });
    const candidate = await buildCandidate(scenario, version);
    const updated = applyCandidate(candidate, scenario.files);
    const changelog = updated.get('CHANGELOG.md');

    assertAligned(scenario.files, updated, version);
    assert.ok(changelog.includes(`### ${section}\n`));
    assert.ok(changelog.includes(message.split(': ')[1]));
    assert.ok(changelog.includes(change.sha));
    assert.equal(changelog.match(/^# Changelog$/gm).length, 1);
    assert.ok(changelog.endsWith(scenario.files.get('CHANGELOG.md').slice('# Changelog\n'.length)));
  });
}

for (const { message, previous, version } of [
  { message: 'feat!: change fixture API', previous: '0.1.0', version: '0.2.0' },
  { message: 'fix: change fixture API\n\nBREAKING CHANGE: old clients need migration.', previous: '0.1.0', version: '0.2.0' },
  { message: 'feat!: change stable fixture API', previous: '1.2.3', version: '2.0.0' },
]) {
  test(`breaking change from ${previous} becomes ${version}: ${message.split('\n')[0]}`, async () => {
    const scenario = fixture({ commits: [commit(message, ['crates/mdview-core/src/lib.rs'])], version: previous });
    const candidate = await buildCandidate(scenario, version);
    const updated = applyCandidate(candidate, scenario.files);

    assertAligned(scenario.files, updated, version);
  });
}

for (const [area, filename] of [
  ['app', 'apps/mdview/src/main.rs'],
  ['internal library', 'crates/mdview-core/src/lib.rs'],
  ['standalone server', 'crates/mdview-server/src/lib.rs'],
  ['Lua plugin', 'lua/mdview/init.lua'],
  ['sidecar', 'sidecar/src/index.ts'],
]) {
  test(`a single ${area} commit releases the same root product`, async () => {
    const message = `fix: correct ${area} fixture`;
    const scenario = fixture({ commits: [commit(message, [filename])] });
    const candidate = await buildCandidate(scenario, '0.1.1');

    assert.ok(candidate.body.toString().includes(message.split(': ')[1]));
    assertAligned(scenario.files, applyCandidate(candidate, scenario.files), '0.1.1');
  });
}

test('mixed Rust, Lua, and sidecar history produces one PR with Added, Fixed, and Changed', async () => {
  const changes = [
    commit('feat: extend fixture sidecar', ['sidecar/src/index.ts']),
    commit('fix: correct fixture Lua bridge', ['lua/mdview/init.lua']),
    commit('perf: accelerate fixture library', ['crates/mdview-core/src/lib.rs']),
    commit('docs: explain fixture setup', ['README.md']),
    commit('chore: tidy fixture tooling', ['tests/release/package.json']),
  ];
  const scenario = fixture({ commits: changes });
  const candidate = await buildCandidate(scenario, '0.2.0');
  const updated = applyCandidate(candidate, scenario.files);
  const changelog = updated.get('CHANGELOG.md');

  assertAligned(scenario.files, updated, '0.2.0');
  for (const section of ['Added', 'Changed', 'Fixed']) {
    assert.ok(candidate.body.toString().includes(`### ${section}\n`));
  }
  for (const change of changes.slice(0, 3)) {
    assert.equal(changelog.split(change.message.split(': ')[1]).length - 1, 1);
  }
  assert.ok(!changelog.includes('explain fixture setup'));
  assert.ok(!changelog.includes('tidy fixture tooling'));
});

for (const [name, changes] of [
  ['no new commits', []],
  ['docs only', [commit('docs: explain fixture usage', ['README.md'])]],
  ['chore only', [commit('chore: clean fixture tooling', ['tests/release/package.json'])]],
  ['docs and chores', [
    commit('docs: explain fixture usage', ['README.md']),
    commit('chore: clean fixture tooling', ['tests/release/package.json']),
  ]],
]) {
  test(`${name} creates no release PR`, async () => {
    const { manifest } = await manifestFor(fixture({ commits: changes }));

    assert.deepEqual(await manifest.buildPullRequests(), []);
  });
}

test('bootstrap creates one 0.1.0 entry from an empty manifest and blank changelog', async () => {
  const changes = [
    commit('feat: add historical fixture renderer', ['apps/mdview/src/main.rs']),
    commit('fix: correct historical fixture bridge', ['lua/mdview/init.lua']),
    commit('perf: accelerate historical fixture sidecar', ['sidecar/src/index.ts']),
  ];
  const scenario = fixture({ bootstrap: true, commits: changes });
  const candidate = await buildCandidate(scenario, baseVersion);
  const updated = applyCandidate(candidate, scenario.files);
  const changelog = updated.get('CHANGELOG.md');

  assertAligned(scenario.files, updated, baseVersion);
  assert.equal(changelog.match(/^# Changelog$/gm).length, 1);
  assert.equal(changelog.match(/^## Changelog$/gm)?.length ?? 0, 0);
  assert.equal(changelog.match(/^#{2,3} \[?0\.1\.0\b.*$/gm).length, 1);
  for (const change of changes) {
    assert.equal(changelog.split(change.message.split(': ')[1]).length - 1, 1);
  }
});

test('actual candidate updaters preserve real Cargo dependencies, comments, and member files', async () => {
  const files = githubBlobFiles(workspace.files);
  const scenario = fixture({ commits: [commit('fix: exercise real manifest updates', ['apps/mdview/src/main.rs'])] });
  const candidate = await buildCandidate(scenario, '0.1.1');
  const updated = applyCandidate(candidate, files);

  assertAligned(files, updated, '0.1.1');
  for (const filename of ['Cargo.lock', 'Cargo.toml']) {
    assert.equal(candidate.updates.find(update => update.path === filename).updater.constructor.name, 'GenericToml');
  }
});

test('real generated PR round-trips into exactly one published v0.1.1 release candidate', async () => {
  const scenario = fixture({ commits: [commit('fix: round trip fixture release', ['apps/mdview/src/main.rs'])] });
  const candidate = await buildCandidate(scenario, '0.1.1');
  const pullRequest = mergedPullRequest(candidate);
  const updated = applyCandidate(candidate, scenario.files);
  const { calls, manifest } = await manifestFor({
    ...scenario,
    files: updated,
    pullRequests: [{ pullRequest, state: 'MERGED' }],
  });
  const releases = await manifest.buildReleases();

  assert.equal(releases.length, 1);
  assert.equal(releases[0].path, '.');
  assert.equal(releases[0].tag.toString(), 'v0.1.1');
  assert.equal(releases[0].name, 'v0.1.1');
  assert.equal(releases[0].sha, mergedSha);
  assert.equal(releases[0].draft, false);
  assert.equal(releases[0].prerelease, false);
  assert.equal(releases[0].notes, candidate.body.releaseData[0].notes);
  assert.equal(releases[0].pullRequest.title, candidate.title.toString());
  assert.equal(releases[0].pullRequest.body, candidate.body.toString());
  assert.equal(releases[0].pullRequest.headBranchName, candidate.headRefName);
  assert.deepEqual(releases[0].pullRequest.labels, [pendingLabel]);
  assert.ok(calls.some(call => call[0] === 'pullRequestIterator' && call[2] === 'MERGED'));
});

test('open and closed-unmerged generated PRs cannot release', async () => {
  const scenario = fixture({ commits: [commit('fix: pending fixture release', ['apps/mdview/src/main.rs'])] });
  const candidate = await buildCandidate(scenario, '0.1.1');
  const pullRequest = { ...mergedPullRequest(candidate), mergeCommitOid: undefined, sha: undefined };
  const { calls, manifest } = await manifestFor({
    ...scenario,
    pullRequests: [
      { pullRequest, state: 'OPEN' },
      { pullRequest: { ...pullRequest, number: 47 }, state: 'CLOSED' },
    ],
  });

  assert.deepEqual(await manifest.buildReleases(), []);
  assert.ok(calls.some(call => call[0] === 'pullRequestIterator' && call[2] === 'MERGED'));
});

test('the actual candidate builder rejects a PR without a merge SHA', async () => {
  const scenario = fixture({ commits: [commit('fix: missing merge SHA fixture', ['apps/mdview/src/main.rs'])] });
  const candidate = await buildCandidate(scenario, '0.1.1');
  const pullRequest = { ...mergedPullRequest(candidate), mergeCommitOid: undefined, sha: undefined };
  const { manifest } = await manifestFor({ ...scenario, pullRequests: [{ pullRequest, state: 'MERGED' }] });

  assert.deepEqual(await manifest.buildReleases(), []);
});

for (const [name, overrides] of [
  ['ordinary PR title', { title: 'fix: an ordinary merged PR' }],
  ['ordinary branch', { headBranchName: 'fix/ordinary-change' }],
  ['ordinary PR body', { body: 'This fixes a bug, but is not a generated release PR.' }],
  ['missing pending label', { labels: [] }],
]) {
  test(`${name} does not pass the real release gate`, async () => {
    const scenario = fixture({ commits: [commit('fix: check fixture release gate', ['apps/mdview/src/main.rs'])] });
    const candidate = await buildCandidate(scenario, '0.1.1');
    const pullRequest = { ...mergedPullRequest(candidate), ...overrides };
    const { manifest } = await manifestFor({ ...scenario, pullRequests: [{ pullRequest, state: 'MERGED' }] });

    assert.deepEqual(await manifest.buildReleases(), []);
  });
}

test('an already published and tagged generated PR creates no duplicate PR or release', async () => {
  const scenario = fixture({ commits: [commit('fix: already released fixture', ['apps/mdview/src/main.rs'])] });
  const candidate = await buildCandidate(scenario, '0.1.1');
  const pullRequest = { ...mergedPullRequest(candidate), labels: [taggedLabel] };
  const updated = applyCandidate(candidate, scenario.files);
  const { manifest } = await manifestFor({
    commits: [{ files: pullRequest.files, message: pullRequest.title, pullRequest, sha: mergedSha }, ...scenario.commits],
    files: updated,
    pullRequests: [{ pullRequest, state: 'MERGED' }],
    releases: [publishedRelease('0.1.1', mergedSha), ...scenario.releases],
    tags: [{ name: 'v0.1.1', sha: mergedSha }, ...scenario.tags],
  });

  assert.deepEqual(await manifest.buildPullRequests(), []);
  assert.deepEqual(await manifest.buildReleases(), []);
});
