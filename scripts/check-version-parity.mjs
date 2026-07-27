#!/usr/bin/env node
// Verifies that every version and native-package claim agrees with Cargo.toml.
// Usage: node scripts/check-version-parity.mjs [--binary PATH] [--receipt PATH]
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const PLATFORM_PACKAGE = 'codex-reset-status-linux-x64';

function parseArgs(argv) {
  const options = { binary: undefined, receipt: undefined };
  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    if (flag === '--binary' || flag === '--receipt') {
      const value = argv[index + 1];
      if (value == null) {
        throw new Error(`${flag} requires a value`);
      }
      options[flag.slice(2)] = value;
      index += 1;
      continue;
    }
    throw new Error(`unknown option: ${flag}`);
  }
  return options;
}

function readJson(relativeOrAbsolute) {
  return JSON.parse(readFileSync(resolve(ROOT, relativeOrAbsolute), 'utf8'));
}

function cargoVersion() {
  const manifest = readFileSync(resolve(ROOT, 'Cargo.toml'), 'utf8');
  const packageSection = manifest.split(/^\[/m).find((section) => section.startsWith('package]'));
  if (packageSection == null) {
    throw new Error('Cargo.toml has no [package] section');
  }
  const match = packageSection.match(/^version\s*=\s*"([^"]+)"/m);
  if (match == null) {
    throw new Error('Cargo.toml [package] has no version');
  }
  return match[1];
}

const options = parseArgs(process.argv.slice(2));
const expected = cargoVersion();
const launcher = readJson('npm/codex-reset-status/package.json');
const platform = readJson('npm/codex-reset-status-linux-x64/package.json');
const launcherSource = readFileSync(resolve(ROOT, 'npm/codex-reset-status/bin/cli.js'), 'utf8');
const failures = [];

function expectEqual(label, actual, wanted = expected) {
  if (actual !== wanted) {
    failures.push(`${label}: expected ${JSON.stringify(wanted)}, got ${JSON.stringify(actual)}`);
  }
}

expectEqual('npm/codex-reset-status version', launcher.version);
expectEqual('npm/codex-reset-status-linux-x64 version', platform.version);
expectEqual('launcher optionalDependencies pin', launcher.optionalDependencies?.[PLATFORM_PACKAGE]);
expectEqual(
  'launcher optionalDependencies keys',
  Object.keys(launcher.optionalDependencies ?? {}).join(','),
  PLATFORM_PACKAGE,
);
expectEqual('platform package name', platform.name, PLATFORM_PACKAGE);
expectEqual('platform os gate', (platform.os ?? []).join(','), 'linux');
expectEqual('platform cpu gate', (platform.cpu ?? []).join(','), 'x64');
expectEqual('platform libc gate', (platform.libc ?? []).join(','), 'glibc');

if (!launcherSource.includes(`'${PLATFORM_PACKAGE}'`)) {
  failures.push(`bin/cli.js does not map to ${PLATFORM_PACKAGE}`);
}
if (/\bhttps?:\/\/[^\s'"]*\.(?:tar\.gz|tgz|zip)\b/.test(launcherSource)) {
  failures.push('bin/cli.js must not reference a downloadable archive URL');
}
for (const hook of ['preinstall', 'install', 'postinstall']) {
  if (launcher.scripts?.[hook] != null || platform.scripts?.[hook] != null) {
    failures.push(`install hook "${hook}" is forbidden`);
  }
}

if (options.binary != null) {
  const printed = execFileSync(options.binary, ['--version'], { encoding: 'utf8' }).trim();
  expectEqual('binary --version', printed, `codex-reset-status ${expected}`);
}

if (options.receipt != null) {
  const receipt = readJson(options.receipt);
  expectEqual('receipt version', receipt.version);
  for (const artifact of receipt.artifacts ?? []) {
    if (!artifact.path.includes(expected)) {
      failures.push(`receipt artifact ${artifact.path} does not carry version ${expected}`);
    }
  }
  if (receipt.git?.head == null || receipt.git.head === 'unborn') {
    failures.push(`receipt git.head is not a real commit: ${receipt.git?.head}`);
  }
  if (receipt.git?.dirty !== false) {
    failures.push('receipt git.dirty must be false for a publishable build');
  }
}

if (failures.length > 0) {
  for (const failure of failures) {
    process.stderr.write(`ERROR: ${failure}\n`);
  }
  process.exit(1);
}

process.stdout.write(`Version parity (${expected}): OK\n`);
