const CACHE_NAME = "valens-{{VERSION}}";

self.addEventListener("install", (event) => {
    event.waitUntil(addResourcesToCache());
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
    const request = event.request;
    if (request.method !== "GET" || new URL(request.url).origin !== self.location.origin) {
        return;
    }
    event.respondWith(
        (async () => {
            try {
                const cachedResponse = await caches.match(request, { cacheName: CACHE_NAME });
                if (cachedResponse) {
                    return cachedResponse;
                }
            } catch (error) {
                console.error(error);
            }

            try {
                return await fetch(request);
            } catch (error) {
                if (request.mode === "navigate") {
                    const appShell = await caches.match("/", { cacheName: CACHE_NAME });
                    if (appShell) {
                        return appShell;
                    }
                }
                throw error;
            }
        })(),
    );
});

self.addEventListener("message", (event) => {
    if (event.data) {
        let task = event.data.task;
        let content = event.data.content;
        if (task === "SkipWaiting") {
            event.waitUntil(self.skipWaiting());
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
            "fonts/fa-solid-900.woff2",
            "images/android-chrome-192x192.png",
            "images/android-chrome-512x512.png",
            "images/apple-touch-icon.png",
            "images/favicon-16x16.png",
            "images/favicon-32x32.png",
            "main.css",
            "manifest.json",
            "valens-web-app-dioxus.js",
            "valens-web-app-dioxus_bg.wasm",
        ]);
    })
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
