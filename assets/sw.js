const cachePrefix = "dissonance-lab-pwa-";
const cacheName = `${cachePrefix}v25`;
const scope = new URL(self.registration.scope);
const index = new URL("index.html", scope);
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
        return cache.addAll(
          filesToCache.map(
            (file) => new Request(new URL(file, scope), { cache: "reload" }),
          ),
        );
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
      }),
  );
});

/* Serve cached content when offline */
self.addEventListener("fetch", function (e) {
  if (e.request.method !== "GET") return;

  const url = new URL(e.request.url);
  const isAppNavigation =
    e.request.mode === "navigate" &&
    url.origin === scope.origin &&
    (url.pathname === scope.pathname || url.pathname === index.pathname);

  e.respondWith(
    caches.match(isAppNavigation ? scope.href : e.request, { cacheName })
      .then((response) => response || fetch(e.request)),
  );
});
