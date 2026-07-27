#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { chmodSync, realpathSync, statSync } from 'node:fs';
import { createRequire } from 'node:module';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);

function packageName(platform = process.platform, arch = process.arch) {
  if (platform === 'linux' && arch === 'x64') {
    return 'codex-reset-status-linux-x64';
  }
  return undefined;
}

function resolveBinary({
  platform = process.platform,
  arch = process.arch,
  resolve = (id) => require.resolve(id),
} = {}) {
  const nativePackage = packageName(platform, arch);
  if (nativePackage == null) {
    return undefined;
  }
  try {
    return resolve(`${nativePackage}/bin/codex-reset-status`);
  } catch {
    return undefined;
  }
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

function ensureExecutable(binaryPath, platform = process.platform) {
  if (platform === 'win32') {
    return;
  }
  const mode = statSync(binaryPath).mode;
  if ((mode & 0o111) === 0) {
    chmodSync(binaryPath, 0o755);
  }
}

async function run(argv, spawnNative = spawn) {
  const binaryPath = resolveBinary();
  if (binaryPath == null) {
    process.stderr.write(
      `codex-reset-status native binary is not available for ${process.platform}-${process.arch}. ` +
        'Reinstall the package so optional native dependencies are installed.\n',
    );
    return 1;
  }

  try {
    ensureExecutable(binaryPath);
  } catch (error) {
    process.stderr.write(`codex-reset-status native binary is not executable: ${error.message}\n`);
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

export { ensureExecutable, isMain, packageName, resolveBinary, run };
