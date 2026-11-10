import { readdir, readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const root = new URL('../dist/', import.meta.url);
const entries = [];
async function walk(dir, prefix = '') {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const name = `${prefix}${entry.name}`;
    if (entry.isDirectory()) {
      if (name !== 'ocr') await walk(new URL(`${entry.name}/`, dir), `${name}/`);
    } else if (!['sw.js', 'precache.json', '.DS_Store', '.trunk-lock', '.nojekyll'].includes(entry.name)) entries.push(name);
  }
}
await walk(root);
const contents = await Promise.all(entries.map(e => readFile(new URL(e, root))));
const version = createHash('sha256').update(Buffer.concat(contents)).digest('hex').slice(0, 12);
await writeFile(new URL('precache.json', root), JSON.stringify({ version, files: entries.sort() }));
const sw = await readFile(new URL('../public/sw.js', import.meta.url), 'utf8');
await writeFile(new URL('sw.js', root), sw.replace('__ARK_BUILD_ID__', version));
await writeFile(new URL('.nojekyll', root), '');
console.log(`Offline manifest ${version}: ${entries.length} files`);
