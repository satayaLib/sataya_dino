const scopePath = new URL(self.registration.scope).pathname;
const prefix = `sataya-fieldbook-app-${scopePath.replaceAll('/', '_')}-`;
const absolute = path => new URL(path, self.registration.scope).href;
const buildId = '__ARK_BUILD_ID__';
const activeCache = `${prefix}${buildId}`;

self.addEventListener('install', event => {
  event.waitUntil((async () => {
    const response = await fetch(absolute('precache.json'), { cache: 'no-store' });
    if (!response.ok) throw new Error('Offline manifest unavailable');
    const manifest = await response.json();
    if (manifest.version !== buildId) throw new Error('Deployment still updating');
    const name = activeCache;
    const cache = await caches.open(name);
    try {
      await cache.addAll(manifest.files.map(absolute));
      await cache.put(absolute('precache.json'), new Response(JSON.stringify(manifest), { headers: { 'Content-Type': 'application/json' } }));
    } catch (error) { await caches.delete(name); throw error; }
  })());
});
self.addEventListener('activate', event => {
  event.waitUntil((async () => {
    const keys = (await caches.keys()).filter(k => k.startsWith(prefix));
    await Promise.all(keys.filter(k => k !== activeCache).map(k => caches.delete(k)));
    await self.clients.claim();
  })());
});
self.addEventListener('fetch', event => {
  const url = new URL(event.request.url);
  if (event.request.method !== 'GET' || url.origin !== self.location.origin || !url.pathname.startsWith(scopePath)) return;
  event.respondWith((async () => {
    const requestUrl = event.request.mode === 'navigate' ? absolute('index.html') : event.request.url;
    const cached = await (await caches.open(activeCache)).match(requestUrl);
    if (cached) return cached;
    if (url.pathname.startsWith(new URL(absolute('ocr/')).pathname)) {
      const ocrName = `sataya-fieldbook-ocr-${scopePath.replaceAll('/', '_')}-v1`;
      const cache = await caches.open(ocrName);
      const cached = await cache.match(event.request);
      if (cached) return cached;
      const response = await fetch(event.request);
      if (response.ok) await cache.put(event.request, response.clone());
      return response;
    }
    return fetch(event.request);
  })());
});
