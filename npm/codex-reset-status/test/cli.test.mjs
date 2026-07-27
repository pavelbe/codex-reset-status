import assert from 'node:assert/strict';
import test from 'node:test';

import { packageName, resolveBinary } from '../bin/cli.js';

test('maps only the verified v0.1 platform', () => {
  assert.equal(packageName('linux', 'x64'), 'codex-reset-status-linux-x64');
  assert.equal(packageName('linux', 'arm64'), undefined);
  assert.equal(packageName('darwin', 'x64'), undefined);
  assert.equal(packageName('win32', 'x64'), undefined);
});

test('fails closed when the optional package cannot be resolved', () => {
  const resolved = resolveBinary({
    platform: 'linux',
    arch: 'x64',
    resolve: () => {
      throw new Error('missing');
    },
  });
  assert.equal(resolved, undefined);
});
