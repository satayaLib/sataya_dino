import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { File } from 'node:buffer';
import { readText, loadImage, findIconStatPanel, iconStatRegions, maskStatText, matchPortrait } from '../crates/web/browser.js';

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

function panelImage(barCount) {
  const width = 600, height = 800;
  const pixels = new Uint8ClampedArray(width * height * 4);
  for (let bar = 0; bar < barCount; bar++) {
    const top = 400 + bar * 60;
    for (let y = top; y <= top + 40; y++) for (let x = 30; x <= 530; x++) {
      if (y !== top && y !== top + 40 && x !== 30 && x !== 530) continue;
      pixels.set([0, 220, 240, 255], (y * width + x) * 4);
    }
  }
  return { pixels, width, height };
}

test('icon panel finds all three bars and bounded stat regions', () => {
  const { pixels, width, height } = panelImage(3);
  const panel = findIconStatPanel(pixels, width, height);
  assert.ok(panel);
  assert.equal(panel.bars.length, 3);
  assert.ok(panel.bars[2].top > panel.bars[1].top);
  const regions = iconStatRegions(panel);
  assert.equal(Object.keys(regions).length, 15);
  for (const rectangle of Object.values(regions)) {
    for (const value of Object.values(rectangle)) assert.ok(Number.isFinite(value));
    assert.ok(rectangle.width > 0 && rectangle.height > 0);
    assert.ok(rectangle.left >= 0 && rectangle.left + rectangle.width <= width);
    assert.ok(rectangle.top >= 0 && rectangle.top + rectangle.height <= height);
  }
});

test('incomplete bars are not treated as a complete icon panel', () => {
  const { pixels, width, height } = panelImage(2);
  assert.equal(findIconStatPanel(pixels, width, height), null);
});

test('stat preprocessing separates white text from colored bars and borders', () => {
  const result = maskStatText(Uint8Array.from([255,255,255,255,255,0,0,255,0,255,40,255,0,220,240,255,15,50,75,255]));
  assert.deepEqual([...result.filter((_, index) => index % 4 === 0)], [0,255,255,255,255]);
});

test('cropped stat bars do not turn missing grid rows into health readings', () => {
  const { pixels, width, height } = panelImage(3);
  const panel = findIconStatPanel(pixels, width, height);
  panel.bars = panel.bars.map((bar, i) => ({ ...bar, top: i * 60 }));
  const regions = iconStatRegions(panel);
  assert.ok(regions.health);
  assert.equal(regions.stamina, undefined);
  assert.equal(regions.oxygen, undefined);
});

test('portrait matching declines identical scores from different families', () => {
  const fingerprint = { ahash32: '0'.repeat(256), dhash16: '0'.repeat(64) };
  const entries = ['rex','wyvern'].map(family => ({ ...fingerprint, family, token: family }));
  assert.equal(matchPortrait(fingerprint, { entries }), null);
});

test('reference HUD portrait matches Wyvern family among the full icon index', async () => {
  const index = JSON.parse(await readFile(new URL('../data/portrait-index.json', import.meta.url), 'utf8'));
  const entry = index.entries.find(item => item.family === 'wyvern');
  const match = matchPortrait({ ahash32: entry.ahash32, dhash16: entry.dhash16 }, index);
  assert.equal(match.family, 'wyvern');
});
