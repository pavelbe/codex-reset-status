import assert from 'node:assert/strict';
import test from 'node:test';

import {
  checkExecutable,
  detectLibc,
  packageName,
  resolveBinary,
  unavailableMessage,
} from '../bin/cli.js';

const throwingResolve = () => {
  throw new Error('missing');
};
const glibcReport = { getReport: () => ({ header: { glibcVersionRuntime: '2.39' } }) };
const muslReport = { getReport: () => ({ header: {} }) };

test('maps only the verified glibc linux-x64 platform', () => {
  assert.equal(packageName('linux', 'x64', 'glibc'), 'codex-reset-status-linux-x64');
  assert.equal(packageName('linux', 'x64', undefined), 'codex-reset-status-linux-x64');
  assert.equal(packageName('linux', 'x64', 'musl'), undefined);
  assert.equal(packageName('linux', 'arm64', 'glibc'), undefined);
  assert.equal(packageName('darwin', 'x64', undefined), undefined);
  assert.equal(packageName('win32', 'x64', undefined), undefined);
});

test('detects the libc flavour from the process report', () => {
  assert.equal(detectLibc('linux', glibcReport), 'glibc');
  assert.equal(detectLibc('linux', muslReport), 'musl');
  assert.equal(detectLibc('linux', {}), undefined);
  assert.equal(detectLibc('darwin', glibcReport), undefined);
});

test('fails closed when the optional package cannot be resolved', () => {
  const resolution = resolveBinary({
    platform: 'linux',
    arch: 'x64',
    libc: 'glibc',
    resolve: throwingResolve,
  });
  assert.equal(resolution.binaryPath, undefined);
  assert.equal(resolution.reason, 'missing-native-package');
  assert.equal(resolution.nativePackage, 'codex-reset-status-linux-x64');
});

test('separates unsupported platform, musl host and missing package', () => {
  const unsupported = resolveBinary({
    platform: 'darwin',
    arch: 'arm64',
    resolve: throwingResolve,
  });
  assert.equal(unsupported.reason, 'unsupported-platform');
  const unsupportedMessage = unavailableMessage(unsupported, 'darwin', 'arm64');
  assert.match(unsupportedMessage, /no native binary for darwin-arm64/);
  assert.match(unsupportedMessage, /linux-x64 with glibc only/);
  assert.doesNotMatch(unsupportedMessage, /Reinstall/);

  const musl = resolveBinary({
    platform: 'linux',
    arch: 'x64',
    libc: 'musl',
    resolve: throwingResolve,
  });
  assert.equal(musl.reason, 'unsupported-libc');
  const muslMessage = unavailableMessage(musl, 'linux', 'x64');
  assert.match(muslMessage, /ships a glibc binary, but this host uses musl/);
  assert.doesNotMatch(muslMessage, /Reinstall/);

  const missingMessage = unavailableMessage(
    resolveBinary({ platform: 'linux', arch: 'x64', libc: 'glibc', resolve: throwingResolve }),
    'linux',
    'x64',
  );
  assert.match(missingMessage, /could not load its native package codex-reset-status-linux-x64/);
  assert.match(missingMessage, /Reinstall codex-reset-status/);
  assert.doesNotMatch(missingMessage, /build from source/);
});

test('resolves the native binary path when the optional package is present', () => {
  const resolution = resolveBinary({
    platform: 'linux',
    arch: 'x64',
    libc: 'glibc',
    resolve: (id) => `/fake/node_modules/${id}`,
  });
  assert.equal(
    resolution.binaryPath,
    '/fake/node_modules/codex-reset-status-linux-x64/bin/codex-reset-status',
  );
  assert.equal(resolution.reason, undefined);
});

test('reports a lost executable bit instead of repairing it', () => {
  assert.doesNotThrow(() => checkExecutable('/fake/bin', () => ({ mode: 0o100755 })));
  assert.throws(() => checkExecutable('/fake/bin', () => ({ mode: 0o100644 })), {
    message: /not executable \(mode 644\)[\s\S]*do not chmod/,
  });
});
