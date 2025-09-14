const CACHE_NAME = "valens";

self.addEventListener("install", function (event) {
    event.waitUntil(
        addResourcesToCache()
    );
});

self.addEventListener("activate", (event) => {
    event.waitUntil(
        deleteDeprecatedCaches()
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
            deleteCache();
            deleteDeprecatedCaches();
            addResourcesToCache();
        }
        if (task === "ShowNotification") {
            self.registration.showNotification(content.title, content.options);
        }
        if (task === "CloseNotifications") {
            self.registration.getNotifications().then((notifications) => {
                notifications.forEach(notification => notification.close());
            });
        }
    }
});

const timer_channel = new BroadcastChannel("timer");

self.addEventListener("notificationclick", function(event) {
    event.notification.close();
    if (event.action === "timer-pause" || event.action === "timer-reset") {
        timer_channel.postMessage(event.action);
    }
}, false);

function addResourcesToCache() {
    caches.open(CACHE_NAME).then((cache) => {
        return cache.addAll([
            "app",
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
            "service-worker.js",
            "valens-web-app-seed.js",
            "valens-web-app-seed_bg.wasm",
        ]);
    })
};

function deleteCache() {
    caches.delete(CACHE_NAME);
};

function deleteDeprecatedCaches() {
    caches.keys().then((keyList) => {
        return Promise.all(keyList.map((key) => {
            if (key === CACHE_NAME) {
                return;
            }
            return caches.delete(key);
        }));
    })
};
