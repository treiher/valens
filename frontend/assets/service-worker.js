const CACHE_NAME = "valens";
const OFFLINE_URL = "offline";

self.addEventListener("install", function (event) {
    event.waitUntil(
        caches.open(CACHE_NAME).then(function (cache) {
            return cache.addAll([
                OFFLINE_URL,
                "fonts/Roboto-Bold.woff",
                "fonts/Roboto-BoldItalic.woff",
                "fonts/Roboto-Italic.woff",
                "fonts/Roboto-Regular.woff",
                "fonts/fa-solid-900.ttf",
                "fonts/fa-solid-900.woff2",
                "images/android-chrome-192x192.png",
                "images/android-chrome-512x512.png",
                "images/apple-touch-icon.png",
                "images/favicon-16x16.png",
                "images/favicon-32x32.png",
                "index.css",
                "manifest.json",
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

self.addEventListener("message", (event) => {
    if (event.data && event.data.type === "notification") {
        self.registration.showNotification(event.data.title, event.data.options);
    }
});

const timer_channel = new BroadcastChannel("timer");

self.addEventListener("notificationclick", function(event) {
    event.notification.close();
    if (event.action === "timer-pause" || event.action === "timer-reset") {
        timer_channel.postMessage(event.action);
    }
}, false);
