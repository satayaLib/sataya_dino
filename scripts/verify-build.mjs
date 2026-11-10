import assert from 'node:assert/strict';
import { readFile, stat } from 'node:fs/promises';
const root=new URL('../dist/',import.meta.url);
const manifest=JSON.parse(await readFile(new URL('precache.json',root),'utf8'));
assert.match(manifest.version,/^[0-9a-f]{12}$/);
assert.ok(manifest.files.includes('index.html'));
assert.ok(manifest.files.includes('data/catalog.json'));
assert.ok(manifest.files.includes('data/portrait-index.json'));
assert.ok(manifest.files.some(f=>f.endsWith('.wasm')));
for(const file of manifest.files){assert.ok(!file.startsWith('/')&&!file.includes('..'));await stat(new URL(file,root));}
const sw=await readFile(new URL('sw.js',root),'utf8');
assert.ok(sw.includes(manifest.version));assert.ok(!sw.includes('__ARK_BUILD_ID__'));
const ocr=JSON.parse(await readFile(new URL('data/ocr-files.json',root),'utf8'));
for(const file of ocr)await stat(new URL(file,root));
for(const lang of ['eng','rus'])assert.ok(ocr.includes(`ocr/lang/${lang}.traineddata.gz`));
const catalog=JSON.parse(await readFile(new URL('data/catalog.json',root),'utf8'));
assert.equal(catalog.version,1);assert.ok(catalog.creatures.length>=65);
console.log(`Static build verified: ${manifest.files.length} offline files, ${ocr.length} OCR files, ${catalog.creatures.length} creatures`);
