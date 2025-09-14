const CACHE_NAME = "valens";

self.addEventListener("install", function (event) {
    event.waitUntil(
        addResourcesToCache()
    );
});

self.addEventListener("activate", (event) => {
    event.waitUntil(
        Promise.all([
            deleteDeprecatedCaches(),
            self.clients.claim(),
        ])
    );
});

self.addEventListener("fetch", (event) => {
    event.respondWith(
        (async () => {
            const cachedResponse = await caches.match(event.request);
            if (cachedResponse) {
                return cachedResponse;
            }

            return fetch(event.request);
        })(),
    );
});

self.addEventListener("message", (event) => {
    if (event.data) {
        let task = event.data.task;
        let content = event.data.content;
        if (task === "UpdateCache") {
            event.waitUntil((async () => {
                await deleteCache();
                await addResourcesToCache();
                const clients = await self.clients.matchAll({ includeUncontrolled: true, type: "window" });
                for (const client of clients) {
                    client.postMessage({ task: "Reload" });
                }
            })());
        }
        if (task === "ShowNotification") {
            event.waitUntil(
                self.registration.showNotification(content.title, content.options)
            );
        }
        if (task === "CloseNotifications") {
            event.waitUntil(
                self.registration.getNotifications().then((notifications) => {
                    notifications.forEach(notification => notification.close());
                })
            );
        }
    }
});

function addResourcesToCache() {
    return caches.open(CACHE_NAME).then((cache) => {
        return cache.addAll([
            "/",
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
            "main.css",
            "manifest.json",
            "sw.js",
            "valens-web-app-dioxus.js",
            "valens-web-app-dioxus_bg.wasm",
        ]);
    })
};

function deleteCache() {
    return caches.delete(CACHE_NAME);
};

function deleteDeprecatedCaches() {
    return caches.keys().then((keyList) => {
        return Promise.all(keyList.map((key) => {
            if (key === CACHE_NAME) {
                return;
            }
            return caches.delete(key);
        }));
    })
};
