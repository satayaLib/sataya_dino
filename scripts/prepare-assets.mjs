import { cp, mkdir, readFile, writeFile, readdir } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const require = createRequire(import.meta.url);
const dest = path.join(root, 'public/ocr');
await mkdir(dest, { recursive: true });
await mkdir(path.join(dest, 'core'), { recursive: true });
await mkdir(path.join(dest, 'lang'), { recursive: true });
const tesseractRoot = path.dirname(require.resolve('tesseract.js/package.json'));
const coreRoot = path.dirname(require.resolve('tesseract.js-core/package.json'));
await cp(path.join(tesseractRoot, 'LICENSE.md'), path.join(dest, 'LICENSE.tesseract.md'));
await cp(path.join(coreRoot, 'LICENSE'), path.join(dest, 'LICENSE.core.txt'));
for (const name of ['tesseract.esm.min.js', 'tesseract.min.js', 'worker.min.js']) {
  await cp(path.join(tesseractRoot, 'dist', name), path.join(dest, name));
}
for (const name of await readdir(coreRoot)) {
  if (/^tesseract-core.*\.(wasm|js)$/.test(name)) await cp(path.join(coreRoot, name), path.join(dest, 'core', name));
}
for (const lang of ['eng', 'rus']) {
  const module = require(`@tesseract.js-data/${lang}`);
  await cp(path.join(module.langPath, `${lang}.traineddata.gz`), path.join(dest, 'lang', `${lang}.traineddata.gz`));
  const metadata = require(`@tesseract.js-data/${lang}/package.json`);
  await writeFile(path.join(dest, 'lang', `${lang}.package.json`), JSON.stringify({ name: metadata.name, version: metadata.version, license: metadata.license, author: metadata.author, repository: metadata.repository }));
}
const source = JSON.parse(await readFile(path.join(root, 'data/creatures.json'), 'utf8'));
const packs = JSON.parse(await readFile(path.join(root, 'data/packs.json'), 'utf8'));
const creatures = source.flatMap(({ games, ...c }) => games.map(game => ({
  ...c, game, id: `${game}:${c.pack_id}:${c.slug}`,
})));
await mkdir(path.join(root, 'public/data'), { recursive: true });
await cp(path.join(root, 'data/portrait-index.json'), path.join(root, 'public/data/portrait-index.json'));
await writeFile(path.join(root, 'public/data/catalog.json'), JSON.stringify({ version: 1, packs, creatures }));
const files = [];
async function walk(folder) {
  for (const entry of await readdir(folder, { withFileTypes: true })) {
    const full = path.join(folder, entry.name);
    if (entry.isDirectory()) await walk(full);
    else files.push(path.relative(path.join(root, 'public'), full).split(path.sep).join('/'));
  }
}
await walk(dest);
await writeFile(path.join(root, 'public/data/ocr-files.json'), JSON.stringify(files));
console.log(`Assets ready: ${creatures.length} creatures, ${files.length} local OCR files`);
