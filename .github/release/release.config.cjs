const path = require('node:path');
const python = process.platform === 'win32' ? 'python' : 'python3';

module.exports = {
  branches: ['main'],
  plugins: [
    [require.resolve('@semantic-release/commit-analyzer'), {
      preset: 'conventionalcommits',
    }],
    [require.resolve('@semantic-release/release-notes-generator'), {
      preset: 'conventionalcommits',
      presetConfig: {
        types: [
          { section: 'Added', type: 'feat' },
          { section: 'Fixed', type: 'fix' },
          { section: 'Changed', type: 'perf' },
          { section: 'Reverted', type: 'revert' },
        ],
      },
    }],
    [require.resolve('@semantic-release/changelog'), {
      changelogFile: 'CHANGELOG.md',
    }],
    [require.resolve('@semantic-release/exec'), {
      prepareCmd: `${python} ${path.join('scripts', 'set_version.py')} \${nextRelease.version}`,
    }],
    [require.resolve('@semantic-release/git'), {
      assets: ['CHANGELOG.md', 'Cargo.lock', 'Cargo.toml'],
      message: 'chore(version): ${nextRelease.version} [skip ci]',
    }],
  ],
  tagFormat: 'v${version}',
};
