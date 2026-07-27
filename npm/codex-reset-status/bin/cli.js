#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { realpathSync, statSync } from 'node:fs';
import { createRequire } from 'node:module';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const HOMEPAGE = 'https://github.com/pavelbe/codex-reset-status';

/**
 * The shipped binary links against glibc, so a musl host is unsupported even
 * though its platform and architecture match. An unknown libc is not blocked.
 */
function detectLibc(platform = process.platform, report = process.report) {
  if (platform !== 'linux') {
    return undefined;
  }
  const header = typeof report?.getReport === 'function' ? report.getReport().header : undefined;
  if (header == null) {
    return undefined;
  }
  return header.glibcVersionRuntime == null ? 'musl' : 'glibc';
}

function packageName(platform = process.platform, arch = process.arch, libc = detectLibc(platform)) {
  if (platform === 'linux' && arch === 'x64' && libc !== 'musl') {
    return 'codex-reset-status-linux-x64';
  }
  return undefined;
}

/**
 * Distinguishes three failures that need different fixes: a platform this
 * version never ships, a musl host, and a missing optional native package.
 */
function resolveBinary({
  platform = process.platform,
  arch = process.arch,
  libc = detectLibc(platform),
  resolve = (id) => require.resolve(id),
} = {}) {
  const nativePackage = packageName(platform, arch, libc);
  if (nativePackage == null) {
    const reason =
      platform === 'linux' && arch === 'x64' && libc === 'musl'
        ? 'unsupported-libc'
        : 'unsupported-platform';
    return { binaryPath: undefined, reason };
  }
  try {
    return { binaryPath: resolve(`${nativePackage}/bin/codex-reset-status`), reason: undefined };
  } catch {
    return { binaryPath: undefined, reason: 'missing-native-package', nativePackage };
  }
}

function unavailableMessage(resolution, platform = process.platform, arch = process.arch) {
  if (resolution.reason === 'unsupported-libc') {
    return (
      `codex-reset-status ships a glibc binary, but this host uses musl (${platform}-${arch}). ` +
      `Build from source at ${HOMEPAGE}\n`
    );
  }
  if (resolution.reason === 'unsupported-platform') {
    return (
      `codex-reset-status has no native binary for ${platform}-${arch}. ` +
      `This version supports linux-x64 with glibc only; build from source at ${HOMEPAGE}\n`
    );
  }
  return (
    `codex-reset-status could not load its native package ${resolution.nativePackage} for ` +
    `${platform}-${arch}. Reinstall codex-reset-status so the optional dependency is installed, ` +
    'and do not install with --no-optional or --omit=optional.\n'
  );
}

function isMain({ entry = process.argv[1], moduleUrl = import.meta.url } = {}) {
  if (entry == null) {
    return false;
  }
  const modulePath = fileURLToPath(moduleUrl);
  try {
    return realpathSync(entry) === realpathSync(modulePath);
  } catch {
    return entry === modulePath;
  }
}

/**
 * Reports a lost executable bit instead of repairing it: a runtime chmod would
 * mutate a shared package store and hide a broken published tarball.
 */
function checkExecutable(binaryPath, stat = statSync) {
  const mode = stat(binaryPath).mode;
  if ((mode & 0o111) === 0) {
    throw new Error(
      `${binaryPath} is not executable (mode ${(mode & 0o777).toString(8)}). ` +
        'Reinstall codex-reset-status; do not chmod inside node_modules.',
    );
  }
}

async function run(argv, spawnNative = spawn) {
  const resolution = resolveBinary();
  const binaryPath = resolution.binaryPath;
  if (binaryPath == null) {
    process.stderr.write(unavailableMessage(resolution));
    return 1;
  }

  try {
    checkExecutable(binaryPath);
  } catch (error) {
    process.stderr.write(`codex-reset-status: ${error.message}\n`);
    return 1;
  }

  return await new Promise((resolve) => {
    const child = spawnNative(binaryPath, argv, { stdio: 'inherit' });
    const signals = ['SIGINT', 'SIGTERM', 'SIGHUP'];
    const handlers = new Map(
      signals.map((signal) => [
        signal,
        () => {
          if (!child.killed) {
            child.kill(signal);
          }
        },
      ]),
    );
    for (const [signal, handler] of handlers) {
      process.once(signal, handler);
    }
    const cleanup = () => {
      for (const [signal, handler] of handlers) {
        process.removeListener(signal, handler);
      }
    };
    child.on('error', (error) => {
      cleanup();
      process.stderr.write(`${error.message}\n`);
      resolve(1);
    });
    child.on('exit', (status, signal) => {
      cleanup();
      if (signal != null) {
        process.kill(process.pid, signal);
        return;
      }
      resolve(status ?? 1);
    });
  });
}

if (isMain()) {
  process.exitCode = await run(process.argv.slice(2));
}

export { checkExecutable, detectLibc, isMain, packageName, resolveBinary, run, unavailableMessage };
