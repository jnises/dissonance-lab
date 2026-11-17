const cachePrefix = "dissonance-lab-pwa-";
const cacheName = `${cachePrefix}v24`;
const filesToCache = [
  "./",
  "./index.html",
  "./dissonance-lab.js",
  "./dissonance-lab_bg.wasm",
  "./dissonance_worklet_processor.js",
  "./audio-worklet.js",
  "./audio-worklet_bg.wasm",
];

/* Start the service worker and cache all of the app's content */
self.addEventListener("install", function (e) {
  e.waitUntil(
    caches
      .open(cacheName)
      .then(function (cache) {
        return cache.addAll(filesToCache);
      })
      .then(function () {
        return self.skipWaiting(); // Ensure the new service worker activates immediately
      }),
  );
});

/* Clear old caches when a new service worker is activated */
self.addEventListener("activate", function (e) {
  e.waitUntil(
    caches
      .keys()
      .then(function (cacheNames) {
        return Promise.all(
          cacheNames
            .filter((name) => name.startsWith(cachePrefix) && name !== cacheName)
            .map((name) => caches.delete(name)),
        );
      })
      .then(function () {
        return self.clients.claim(); // Take control of clients immediately
      }),
  );
});

/* Serve cached content when offline */
self.addEventListener("fetch", function (e) {
  if (e.request.method !== "GET") return;

  e.respondWith(
    caches.match(e.request, { cacheName })
      .then((response) => response || fetch(e.request)),
  );
});
