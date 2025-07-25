const cacheName = "dissonance-lab-pwa-v14";
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
  const cacheWhitelist = [cacheName]; // Only keep the current cache version

  e.waitUntil(
    caches
      .keys()
      .then(function (cacheNames) {
        return Promise.all(
          cacheNames.map(function (name) {
            if (!cacheWhitelist.includes(name)) {
              return caches.delete(name); // Delete old caches
            }
          }),
        );
      })
      .then(function () {
        return self.clients.claim(); // Take control of clients immediately
      }),
  );
});

/* Serve cached content when offline */
self.addEventListener("fetch", function (e) {
  e.respondWith(
    caches.match(e.request).then(function (response) {
      return response || fetch(e.request);
    }),
  );
});
