let worker;
let workerLanguage;
let requestId = 0;
let activeProgress;
let channel;
let portraitIndex;
let updateReady = false;
let reloadAfterUpdate = false;
const watchedRegistrations = new WeakSet();
const appPrefix = 'sataya-fieldbook';

export function assetUrl(path) { return new URL(path, document.baseURI).href; }
export function databaseName() { return `${appPrefix}:${new URL('./', document.baseURI).pathname}`; }
export function newId() { return crypto.randomUUID(); }
export function now() { return new Date().toISOString(); }
export function applyTheme(theme) { document.documentElement.dataset.theme = theme; }
export function download(content, name, mime) {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = name;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 30000);
}
export async function readText(file) {
  if (file.size > 20 * 1024 * 1024) throw new Error('Файл превышает 20 МБ');
  const bytes = new Uint8Array(await file.arrayBuffer());
  try { return new TextDecoder('utf-8', { fatal: true }).decode(bytes); }
  catch { throw new Error('Сохраните файл в UTF-8. Эта кодировка нужна для корректного переноса кириллицы.'); }
}
export async function loadImage(file) {
  if (!['image/png', 'image/jpeg', 'image/webp'].includes(file.type)) throw new Error('Выберите PNG, JPEG или WebP');
  if (file.size > 20 * 1024 * 1024) throw new Error('Изображение превышает 20 МБ');
  const url = URL.createObjectURL(file);
  try {
    const img = await imageElement(url);
    if (img.naturalWidth * img.naturalHeight > 24000000) throw new Error('Изображение слишком большое: до 24 мегапикселей');
    return JSON.stringify({ url, width: img.naturalWidth, height: img.naturalHeight });
  } catch (error) { URL.revokeObjectURL(url); throw error; }
}
export function releaseImage(url) { if (url.startsWith('blob:')) URL.revokeObjectURL(url); }
async function imageElement(url) {
  const image = new Image();
  image.src = url;
  await image.decode();
  return image;
}

export function findIconStatPanel(pixels, width, height) {
  const bands = [];
  const minimum = Math.max(120, width * 0.035);
  const sameColor = (a, b) => a.every((value, index) => Math.abs(value - b[index]) < 0.25);
  for (let y = 0; y < height; y++) {
    let start = -1;
    let color;
    for (let x = 0; x <= width; x++) {
      const at = (y * width + x) * 4;
      const r = pixels[at], g = pixels[at + 1], b = pixels[at + 2];
      const maximum = Math.max(r, g, b);
      const accent = x < width && maximum > 120 && maximum - Math.min(r, g, b) > 60;
      const current = accent ? [r / maximum, g / maximum, b / maximum] : null;
      if (start >= 0 && (!accent || !sameColor(color, current))) {
        if (x - start >= minimum) {
          const tolerance = Math.max(3, (x - start) * 0.008);
          const previous = bands.findLast(line => y - line.bottom <= 2 && Math.abs(line.left - start) < tolerance && Math.abs(line.right - x) < tolerance && sameColor(line.color, color));
          if (previous) previous.bottom = y;
          else bands.push({ left: start, right: x, top: y, bottom: y, color });
        }
        start = -1;
      }
      if (accent && start < 0) { start = x; color = current; }
    }
  }
  const thin = bands.filter(line => line.bottom - line.top <= Math.max(3, (line.right - line.left) * 0.015));
  for (const first of thin) {
    const span = first.right - first.left;
    const matching = thin.filter(line => line.top >= first.top && Math.abs(line.left - first.left) < span * 0.012 && Math.abs(line.right - first.right) < span * 0.012 && sameColor(line.color, first.color)).sort((a, b) => a.top - b.top);
    if (matching.length < 6) continue;
    const edges = matching.slice(0, 6);
    const heights = [0, 2, 4].map(i => edges[i + 1].bottom - edges[i].top + 1);
    const average = heights.reduce((a, b) => a + b) / 3;
    const gaps = [edges[2].top - edges[1].bottom, edges[4].top - edges[3].bottom];
    if (average / span < 0.04 || average / span > 0.16 || heights.some(h => Math.abs(h - average) > average * 0.2) || gaps.some(g => g < average * 0.15 || g > average * 1.1)) continue;
    return { left: first.left, width: span, color: first.color, barHeight: average, bars: [0, 2, 4].map((i, index) => ({ left: first.left, top: edges[i].top, width: span, height: heights[index] })) };
  }
  return null;
}

export function findPortraitRegion(pixels, width, height, panel) {
  const edges = [];
  const limit = Math.min(height, Math.round(panel.bars[0].top - panel.barHeight * 5));
  const startX = Math.max(0, Math.floor(panel.left - panel.width * 0.03));
  const endX = Math.min(width, Math.ceil(panel.left + panel.width * 0.45));
  for (let y = 0; y < limit; y++) {
    let start = -1;
    for (let x = startX; x <= endX; x++) {
      const at = (y * width + x) * 4;
      const values = [pixels[at], pixels[at + 1], pixels[at + 2]];
      const maximum = Math.max(...values);
      const frame = x < endX && maximum > 120 && values.every((value, i) => Math.abs(value / maximum - panel.color[i]) < 0.25);
      if (frame && start < 0) start = x;
      if (!frame && start >= 0) {
        const span = x - start;
        if (span > panel.width * 0.2 && span < panel.width * 0.4 && Math.abs(start - panel.left) < panel.width * 0.03) {
          const previous = edges.findLast(edge => y - edge.bottom < 3 && Math.abs(edge.left - start) < 3 && Math.abs(edge.width - span) < 3);
          if (previous) previous.bottom = y;
          else edges.push({ left: start, top: y, bottom: y, width: span });
        }
        start = -1;
      }
    }
  }
  for (const top of edges) {
    const bottom = edges.find(edge => edge.top > top.bottom && Math.abs(edge.left - top.left) < 4 && Math.abs(edge.width - top.width) < 4 && Math.abs(edge.bottom - top.top - top.width) < top.width * 0.12);
    if (bottom) {
      const inset = Math.ceil(top.width * 0.075);
      return { left: top.left + inset, top: top.top + inset, width: top.width - inset * 2, height: bottom.bottom - top.top + 1 - inset * 2 };
    }
  }
  return null;
}

function resizeGray(input, width, height, outputWidth, outputHeight) {
  const horizontal = new Float64Array(outputWidth * height);
  const scaleX = Math.max(1, width / outputWidth), scaleY = Math.max(1, height / outputHeight);
  for (let y = 0; y < height; y++) for (let x = 0; x < outputWidth; x++) {
    const center = (x + 0.5) * width / outputWidth - 0.5;
    let sum = 0, weight = 0;
    for (let ix = Math.max(0, Math.ceil(center - scaleX)); ix <= Math.min(width - 1, Math.floor(center + scaleX)); ix++) {
      const factor = Math.max(0, 1 - Math.abs(ix - center) / scaleX);
      sum += input[y * width + ix] * factor; weight += factor;
    }
    horizontal[y * outputWidth + x] = sum / weight;
  }
  const output = new Uint8Array(outputWidth * outputHeight);
  for (let y = 0; y < outputHeight; y++) for (let x = 0; x < outputWidth; x++) {
    const center = (y + 0.5) * height / outputHeight - 0.5;
    let sum = 0, weight = 0;
    for (let iy = Math.max(0, Math.ceil(center - scaleY)); iy <= Math.min(height - 1, Math.floor(center + scaleY)); iy++) {
      const factor = Math.max(0, 1 - Math.abs(iy - center) / scaleY);
      sum += horizontal[iy * outputWidth + x] * factor; weight += factor;
    }
    output[y * outputWidth + x] = Math.round(sum / weight);
  }
  return output;
}

function bitsHex(bits) {
  let result = '';
  for (let i = 0; i < bits.length; i += 4) result += ((bits[i] ? 8 : 0) | (bits[i + 1] ? 4 : 0) | (bits[i + 2] ? 2 : 0) | (bits[i + 3] ? 1 : 0)).toString(16);
  return result;
}

export function portraitFingerprint(pixels, width, height, rectangle) {
  const gray = new Uint8Array(rectangle.width * rectangle.height);
  let minX = rectangle.width, minY = rectangle.height, maxX = -1, maxY = -1;
  for (let y = 0; y < rectangle.height; y++) for (let x = 0; x < rectangle.width; x++) {
    const at = ((y + rectangle.top) * width + x + rectangle.left) * 4;
    const value = Math.max(0, Math.min(255, Math.round((Math.min(pixels[at + 1], pixels[at + 2]) - pixels[at] - 35) * 255 / 180)));
    gray[y * rectangle.width + x] = value;
    if (value > 45) { minX = Math.min(minX, x); minY = Math.min(minY, y); maxX = Math.max(maxX, x); maxY = Math.max(maxY, y); }
  }
  if (maxX - minX < 10 || maxY - minY < 10) return null;
  const cropWidth = maxX - minX + 1, cropHeight = maxY - minY + 1;
  const cropped = new Uint8Array(cropWidth * cropHeight);
  for (let y = 0; y < cropHeight; y++) cropped.set(gray.subarray((y + minY) * rectangle.width + minX, (y + minY) * rectangle.width + maxX + 1), y * cropWidth);
  const average = resizeGray(cropped, cropWidth, cropHeight, 32, 32);
  const mean = average.reduce((sum, value) => sum + value, 0) / average.length;
  const difference = resizeGray(cropped, cropWidth, cropHeight, 17, 16);
  return { ahash32: bitsHex([...average].map(value => value >= mean)), dhash16: bitsHex(Array.from({ length: 256 }, (_, i) => difference[Math.floor(i / 16) * 17 + i % 16] > difference[Math.floor(i / 16) * 17 + i % 16 + 1])) };
}

export function matchPortrait(fingerprint, index) {
  if (!fingerprint) return null;
  const population = [0,1,1,2,1,2,2,3,1,2,2,3,2,3,3,4];
  const distance = (left, right) => [...left].reduce((sum, digit, i) => sum + population[parseInt(digit, 16) ^ parseInt(right[i], 16)], 0);
  const families = new Map();
  for (const entry of index.entries) {
    const score = 0.7 * (1 - distance(fingerprint.ahash32, entry.ahash32) / 1024) + 0.3 * (1 - distance(fingerprint.dhash16, entry.dhash16) / 256);
    if (!families.has(entry.family) || families.get(entry.family).score < score) families.set(entry.family, { family: entry.family, token: entry.token, score });
  }
  const sorted = [...families.values()].sort((a, b) => b.score - a.score);
  const first = sorted[0];
  if (!first || first.score < 0.85 || first.score - (sorted[1]?.score ?? 0) < 0.12) return null;
  const aliases = { icejumper: 'Managarmr', shapeshifter: 'Ferox', spacewhale: 'Astrocetus', spacefish: 'Astrodelphis', xenomorph: 'Reaper', lanternbird: 'Featherlight', lanterngoat: 'Shinehorn', lanternpug: 'Bulbdog', lanternlizard: 'Glowtail', spindles: 'Bloodstalker', jugbug: 'Jug Bug' };
  return { ...first, token: aliases[first.family] ?? first.token };
}

export function maskStatText(pixels) {
  const result = new Uint8ClampedArray(pixels.length);
  for (let at = 0; at < pixels.length; at += 4) {
    const r = pixels[at], g = pixels[at + 1], b = pixels[at + 2];
    const ink = Math.min(r, g, b) > 145 && Math.max(r, g, b) - Math.min(r, g, b) < 95;
    result[at] = result[at + 1] = result[at + 2] = ink ? 0 : 255;
    result[at + 3] = 255;
  }
  return result;
}

export function iconStatRegions(panel) {
  const { left, width, barHeight, bars } = panel;
  const regions = {};
  const rectangle = (x, y, w, h) => ({ left: Math.round(x), top: Math.round(y), width: Math.round(w), height: Math.round(h) });
  ['health', 'food', 'torpor'].forEach((key, i) => {
    const bar = bars[i];
    regions[key] = rectangle(left + width * 0.2, bar.top + barHeight * 0.08, width * 0.78, bar.height - barHeight * 0.16);
  });
  [['stamina', 'oxygen'], ['food_summary', 'weight'], ['melee_percent', 'speed_percent']].forEach((keys, row) => {
    const center = bars[0].top - barHeight * (1.95 + (2 - row) * 1.1);
    keys.forEach((key, column) => {
      regions[key] = rectangle(left + width * (column ? 0.6 : 0.075), center - barHeight * 0.48, width * (column ? 0.2 : 0.215), barHeight * 0.96);
      regions[`points_${key}`] = rectangle(left + width * (column ? 0.81 : 0.29), center - barHeight * 0.48, width * 0.2, barHeight * 0.96);
    });
  });
  return Object.fromEntries(Object.entries(regions).filter(([, rectangle]) => rectangle.left >= 0 && rectangle.top >= 0 && rectangle.width > 0 && rectangle.height > 0));
}

export async function recognizeIconStats(activeWorker, input, panel, progress = () => {}, isCurrent = () => true) {
  await activeWorker.setParameters({ tessedit_pageseg_mode: '7', tessedit_char_whitelist: '0123456789.,%/()- KkКкMmМм' });
  const regions = {};
  const entries = Object.entries(iconStatRegions(panel));
  for (let i = 0; i < entries.length; i++) {
    if (!isCurrent()) throw new Error('Распознавание отменено');
    const [key, rectangle] = entries[i];
    progress(`Читаем статы по иконкам · ${i + 1} из ${entries.length}`);
    const { data } = await activeWorker.recognize(input, { rectangle }, { text: true });
    regions[key] = { text: data.text.trim(), confidence: data.confidence, bbox: [rectangle.left, rectangle.top, rectangle.left + rectangle.width, rectangle.top + rectangle.height] };
  }
  return regions;
}

export async function recognizeImage(url, cropJson, lang, progress) {
  const current = ++requestId;
  activeProgress = progress;
  try {
  const image = await imageElement(url);
  const crop = JSON.parse(cropJson);
  const sx = Math.max(0, crop.x) * image.naturalWidth / 100;
  const sy = Math.max(0, crop.y) * image.naturalHeight / 100;
  const sw = Math.min(crop.w, 100 - crop.x) * image.naturalWidth / 100;
  const sh = Math.min(crop.h, 100 - crop.y) * image.naturalHeight / 100;
  if (sw < 10 || sh < 10) throw new Error('Выделите область со статами крупнее');
  const scale = Math.min(2, 2600 / sw);
  const canvas = document.createElement('canvas');
  canvas.width = Math.round(sw * scale);
  canvas.height = Math.round(sh * scale);
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  ctx.drawImage(image, sx, sy, sw, sh, 0, 0, canvas.width, canvas.height);
  progress('Загружаем локальное распознавание…');
  const { default: tesseract } = await import(assetUrl('ocr/tesseract.esm.min.js'));
  const { createWorker, PSM } = tesseract;
  if (current !== requestId) throw new Error('Распознавание отменено');
  if (!worker || workerLanguage !== lang) {
    if (worker) await worker.terminate();
    worker = undefined;
    const created = await createWorker(lang.split('+'), 1, {
      workerPath: assetUrl('ocr/worker.min.js'),
      corePath: assetUrl('ocr/core/'),
      langPath: assetUrl('ocr/lang').replace(/\/$/, ''),
      workerBlobURL: false,
      cachePath: `${appPrefix}-ocr-v1`,
      logger: m => activeProgress?.(`${m.status === 'recognizing text' ? 'Читаем статы' : 'Подготавливаем OCR'} · ${Math.round((m.progress ?? 0) * 100)}%`),
    });
    if (current !== requestId) { await created.terminate(); throw new Error('Распознавание отменено'); }
    worker = created;
    workerLanguage = lang;
  }
  const activeWorker = worker;
  await activeWorker.setParameters({ tessedit_pageseg_mode: PSM.SPARSE_TEXT, preserve_interword_spaces: '1', user_defined_dpi: '300', tessedit_char_whitelist: '' });
  const result = await activeWorker.recognize(canvas, {}, { blocks: true, text: true });
  if (current !== requestId) throw new Error('Распознавание отменено');
  const lines = [];
  for (const block of result.data.blocks ?? []) for (const paragraph of block.paragraphs ?? []) for (const line of paragraph.lines ?? []) {
    const b = line.bbox;
    lines.push({ text: line.text, confidence: line.confidence, bbox: b ? [b.x0, b.y0, b.x1, b.y1] : null });
  }
  const pixels = ctx.getImageData(0, 0, canvas.width, canvas.height);
  const panel = findIconStatPanel(pixels.data, canvas.width, canvas.height);
  let regions = {};
  let portraitFamily;
  if (panel) {
    const portrait = findPortraitRegion(pixels.data, canvas.width, canvas.height, panel);
    if (portrait) {
      const fingerprint = portraitFingerprint(pixels.data, canvas.width, canvas.height, portrait);
      try {
        portraitIndex ??= fetch(assetUrl('data/portrait-index.json')).then(response => { if (!response.ok) throw new Error('Portrait index unavailable'); return response.json(); });
        portraitFamily = matchPortrait(fingerprint, await portraitIndex)?.token;
      } catch { portraitIndex = undefined; }
    }
    pixels.data.set(maskStatText(pixels.data));
    ctx.putImageData(pixels, 0, 0);
    regions = await recognizeIconStats(activeWorker, canvas, panel, progress, () => current === requestId);
  }
  canvas.width = canvas.height = 0;
  return JSON.stringify({ text: result.data.text, lines, layout: panel ? 'icon_panel' : 'labels', regions, portrait_family: portraitFamily });
  } finally {
    if (current === requestId) activeProgress = undefined;
  }
}
export function cancelOcr() {
  requestId += 1;
  activeProgress = undefined;
  const current = worker;
  worker = undefined;
  workerLanguage = undefined;
  if (current) current.terminate().catch(() => {});
}
export function updateAvailable() { return updateReady; }
function watchRegistration(registration) {
  if (watchedRegistrations.has(registration)) return;
  watchedRegistrations.add(registration);
  const announce = () => {
    if (navigator.serviceWorker.controller) {
      updateReady = true;
      window.dispatchEvent(new Event('ark-update'));
    }
  };
  if (registration.waiting) announce();
  const observe = worker => worker?.addEventListener('statechange', () => { if (worker.state === 'installed') announce(); });
  observe(registration.installing);
  registration.addEventListener('updatefound', () => observe(registration.installing));
  window.addEventListener('focus', () => registration.update().catch(() => {}));
}
export async function activateUpdate() {
  if (document.querySelector('dialog[open]')) throw new Error('Сохраните или закройте форму перед обновлением');
  const registration = await navigator.serviceWorker.getRegistration(assetUrl('index.html'));
  if (!registration?.waiting) throw new Error('Обновление ещё подготавливается. Повторите через несколько секунд.');
  reloadAfterUpdate = true;
  registration.waiting.postMessage({ type: 'ARK_ACTIVATE_UPDATE' });
}
export function initializeBrowser() {
  if ('BroadcastChannel' in window && !channel) {
    channel = new BroadcastChannel(`${appPrefix}-sync-${new URL('./', document.baseURI).pathname}`);
    channel.onmessage = () => window.dispatchEvent(new CustomEvent('ark-sync'));
  }
  if ('serviceWorker' in navigator && !['localhost', '127.0.0.1'].includes(location.hostname)) {
    navigator.serviceWorker.register(assetUrl('sw.js'), { scope: new URL('./', document.baseURI).pathname }).then(watchRegistration).catch(error => console.error('Offline:', error));
  }
  navigator.serviceWorker?.addEventListener('controllerchange', () => { if (reloadAfterUpdate) location.reload(); });
  const notifyNetwork = () => window.dispatchEvent(new CustomEvent('ark-network'));
  window.addEventListener('online', notifyNetwork);
  window.addEventListener('offline', notifyNetwork);
  document.addEventListener('keydown', event => {
    if (event.key !== '/' || event.ctrlKey || event.metaKey || event.altKey || document.querySelector('dialog[open]')) return;
    if (event.target.closest('input,textarea,select,[contenteditable]')) return;
    const search = document.querySelector('input[aria-label="Поиск существ"],input[aria-label="Поиск в коллекции"]');
    if (search) { event.preventDefault(); search.focus(); }
  });
}
export function notifyChange() { channel?.postMessage({ changed: true }); }
export async function storageInfo() {
  const estimate = await navigator.storage?.estimate?.() ?? {};
  const persistent = await navigator.storage?.persisted?.() ?? false;
  return JSON.stringify({ usage: estimate.usage ?? 0, quota: estimate.quota ?? 0, persistent, online: navigator.onLine });
}
export async function requestPersistence() { return await navigator.storage?.persist?.() ?? false; }
export async function prepareOffline(progress) {
  if (!('serviceWorker' in navigator)) throw new Error('Этот браузер не поддерживает работу без сети');
  const registration = await navigator.serviceWorker.register(assetUrl('sw.js'), { scope: new URL('./', document.baseURI).pathname });
  watchRegistration(registration);
  await navigator.serviceWorker.ready;
  const response = await fetch(assetUrl('data/ocr-files.json'));
  if (!response.ok) throw new Error('Не удалось получить список OCR-файлов');
  const files = await response.json();
  const scope = new URL('./', document.baseURI).pathname.replaceAll('/', '_');
  const cache = await caches.open(`${appPrefix}-ocr-${scope}-v1`);
  for (let i = 0; i < files.length; i++) {
    const url = assetUrl(files[i]);
    if (!await cache.match(url)) {
      const asset = await fetch(url);
      if (!asset.ok) throw new Error(`Не удалось загрузить ${files[i]}`);
      await cache.put(url, asset);
    }
    progress(`Подготовка офлайн · ${i + 1} из ${files.length}`);
  }
  return 'Справочник и распознавание готовы к работе без сети';
}
