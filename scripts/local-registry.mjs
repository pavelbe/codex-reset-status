#!/usr/bin/env node
// Disposable read-only npm registry that serves one packed tarball under its
// production name and version, so `npm install <launcher.tgz>` has to resolve
// the optional platform dependency itself instead of being handed both files.
//
// Usage: node scripts/local-registry.mjs --tarball PATH --name NAME --version V \
//          [--port-file PATH] [--port N]
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import process from 'node:process';

function parseArgs(argv) {
  const options = { tarball: undefined, name: undefined, version: undefined, portFile: undefined, port: 0 };
  const flags = new Map([
    ['--tarball', 'tarball'],
    ['--name', 'name'],
    ['--version', 'version'],
    ['--port-file', 'portFile'],
    ['--port', 'port'],
  ]);
  for (let index = 0; index < argv.length; index += 1) {
    const key = flags.get(argv[index]);
    if (key == null) {
      throw new Error(`unknown option: ${argv[index]}`);
    }
    const value = argv[index + 1];
    if (value == null) {
      throw new Error(`${argv[index]} requires a value`);
    }
    options[key] = key === 'port' ? Number.parseInt(value, 10) : value;
    index += 1;
  }
  for (const required of ['tarball', 'name', 'version']) {
    if (options[required] == null) {
      throw new Error(`--${required} is required`);
    }
  }
  return options;
}

const options = parseArgs(process.argv.slice(2));
const tarball = readFileSync(options.tarball);
const shasum = createHash('sha1').update(tarball).digest('hex');
const integrity = `sha512-${createHash('sha512').update(tarball).digest('base64')}`;
const tarballPath = `/${options.name}/-/${options.name}-${options.version}.tgz`;

const server = createServer((request, response) => {
  const url = new URL(request.url, 'http://127.0.0.1');
  process.stdout.write(`request ${request.method} ${url.pathname}\n`);
  if (url.pathname === tarballPath) {
    response.writeHead(200, { 'content-type': 'application/octet-stream' });
    response.end(tarball);
    return;
  }
  if (url.pathname === `/${options.name}` || url.pathname === `/${encodeURIComponent(options.name)}`) {
    const address = server.address();
    const manifest = {
      name: options.name,
      description: `Local fixture registry entry for ${options.name}`,
      'dist-tags': { latest: options.version },
      versions: {
        [options.version]: {
          name: options.name,
          version: options.version,
          license: 'MIT',
          os: ['linux'],
          cpu: ['x64'],
          bin: undefined,
          dist: {
            tarball: `http://127.0.0.1:${address.port}${tarballPath}`,
            shasum,
            integrity,
          },
        },
      },
    };
    response.writeHead(200, { 'content-type': 'application/json' });
    response.end(JSON.stringify(manifest));
    return;
  }
  response.writeHead(404, { 'content-type': 'application/json' });
  response.end(JSON.stringify({ error: 'not found in fixture registry' }));
});

server.listen(options.port, '127.0.0.1', () => {
  const { port } = server.address();
  if (options.portFile != null) {
    writeFileSync(options.portFile, `${port}\n`);
  }
  process.stdout.write(`fixture registry listening on http://127.0.0.1:${port}\n`);
});

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.once(signal, () => {
    server.close(() => process.exit(0));
  });
}
