const CACHE_NAME = "valens";
const OFFLINE_URL = "offline";

self.addEventListener("install", function (event) {
    event.waitUntil(
        caches.open(CACHE_NAME).then(function (cache) {
            return cache.addAll([
                OFFLINE_URL,
                "static/css/bulma.css",
                "static/fonts/Roboto-Regular.woff",
                "static/manifest.json",
                "static/images/android-chrome-192x192.png",
                "static/images/android-chrome-512x512.png",
            ]);
        })
    );
});

self.addEventListener("activate", (event) => {
    event.waitUntil(caches.keys().then((keyList) => {
        return Promise.all(keyList.map((key) => {
            if (key === CACHE_NAME) {
                return;
            }
            return caches.delete(key);
        }));
    }));
});

self.addEventListener("fetch", function(event) {
    if (event.request.mode === "navigate") {
        event.respondWith(
            (async () => {
                try {
                    const preloadResponse = await event.preloadResponse;
                    if (preloadResponse) {
                        return preloadResponse;
                    }
                    const networkResponse = await fetch(event.request);
                    return networkResponse;
                } catch (error) {
                    const cache = await caches.open(CACHE_NAME);
                    const cachedResponse = await cache.match(OFFLINE_URL);
                    return cachedResponse;
                }
            })()
        );
    } else {
        event.respondWith(
            fetch(event.request).catch(function() {
                return caches.match(event.request);
            })
        );
    }
});
