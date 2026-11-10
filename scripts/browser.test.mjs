import test from 'node:test';
import assert from 'node:assert/strict';
import { File } from 'node:buffer';
import { readText, loadImage } from '../crates/web/browser.js';

test('UTF-8 collection text preserves Cyrillic and multiline notes', async () => {
  const original='имя,заметка\r\nБорис,"первая\nвторая"\r\n';
  assert.equal(await readText(new File(['\uFEFF'+original],'collection.csv')),original);
});
test('invalid UTF-8 is rejected instead of silently corrupting names', async () => {
  await assert.rejects(readText(new File([Uint8Array.from([0xff,0xfe,0xff])],'collection.csv')),/UTF-8/);
});
test('oversized collections are rejected before reading their contents', async () => {
  const file={size:21*1024*1024,arrayBuffer(){throw new Error('Must not read');}};
  await assert.rejects(readText(file),/20 МБ/);
});
test('unsupported screenshot files are rejected before decoding', async () => {
  await assert.rejects(loadImage(new File(['text'],'fake.pdf',{type:'application/pdf'})),/PNG/);
});
test('oversized screenshots are rejected before allocating image memory', async () => {
  await assert.rejects(loadImage({type:'image/png',size:21*1024*1024}),/20 МБ/);
});
