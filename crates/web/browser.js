let worker;
let workerLanguage;
let requestId = 0;
let activeProgress;
let channel;
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
    await worker.setParameters({ tessedit_pageseg_mode: PSM.SPARSE_TEXT, preserve_interword_spaces: '1', user_defined_dpi: '300' });
  }
  const result = await worker.recognize(canvas, {}, { blocks: true, text: true });
  if (current !== requestId) throw new Error('Распознавание отменено');
  const lines = [];
  for (const block of result.data.blocks ?? []) for (const paragraph of block.paragraphs ?? []) for (const line of paragraph.lines ?? []) {
    const b = line.bbox;
    lines.push({ text: line.text, confidence: line.confidence, bbox: b ? [b.x0, b.y0, b.x1, b.y1] : null });
  }
  canvas.width = canvas.height = 0;
  return JSON.stringify({ text: result.data.text, lines });
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
export function initializeBrowser() {
  if ('BroadcastChannel' in window && !channel) {
    channel = new BroadcastChannel(`${appPrefix}-sync-${new URL('./', document.baseURI).pathname}`);
    channel.onmessage = () => window.dispatchEvent(new CustomEvent('ark-sync'));
  }
  if ('serviceWorker' in navigator && !['localhost', '127.0.0.1'].includes(location.hostname)) {
    navigator.serviceWorker.register(assetUrl('sw.js'), { scope: new URL('./', document.baseURI).pathname }).catch(error => console.error('Offline:', error));
  }
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
  await navigator.serviceWorker.register(assetUrl('sw.js'), { scope: new URL('./', document.baseURI).pathname });
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
