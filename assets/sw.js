const cachePrefix = "dissonance-lab-pwa-";
const cacheName = `${cachePrefix}v25`;
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
        // Fetch one fresh set of assets. The UI and worklet must share the same protocol.
        return cache.addAll(filesToCache.map(
          (file) => new Request(new URL(file, self.registration.scope), { cache: "reload" }),
        ));
      }),
  );
});

/* Activation waits for tabs using the previous version to close. */
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
