import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = fileURLToPath(new URL('../', import.meta.url));
const local = path.join(root, '.tools');
const env = { ...process.env };
if (existsSync(path.join(local, 'cargo/bin/cargo'))) {
  env.CARGO_HOME = path.join(local, 'cargo');
  env.RUSTUP_HOME = path.join(local, 'rustup');
  env.PATH = `${path.join(local, 'cargo/bin')}:${path.join(local, 'bin')}:${env.PATH}`;
  env.XDG_CACHE_HOME = path.join(local, 'cache');
}
function run(program, args) {
  const result = spawnSync(program, args, { cwd: root, env, stdio: 'inherit' });
  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }
  if (result.status !== 0) process.exit(result.status ?? 1);
}
const command = process.argv[2];
if (command === 'check') {
  run('cargo', ['fmt', '--all', '--', '--check']);
  run('cargo', ['test', '-p', 'ark-core', '--locked']);
  run('cargo', ['clippy', '--workspace', '--target', 'wasm32-unknown-unknown', '--locked', '--', '-D', 'warnings']);
  run('node', ['scripts/check-content.mjs']);
  run('node', ['--test', 'scripts/browser.test.mjs']);
} else if (command === 'dev' || command === 'build') {
  run('node', ['scripts/prepare-assets.mjs']);
  const args = command === 'dev' ? ['serve'] : ['build', '--release', '--locked'];
  if (command === 'build' && process.env.ARK_PUBLIC_URL) args.push('--public-url', process.env.ARK_PUBLIC_URL);
  run('trunk', args);
  if (command === 'build') run('node', ['scripts/finalize-build.mjs']);
} else {
  console.error('Usage: node scripts/run.mjs dev|build|check');
  process.exit(1);
}
