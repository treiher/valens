const VERSION = "{{VERSION}}";
const CACHE_NAME = `valens-${VERSION}`;

// Resources generated per build, which have to belong to the build of the service worker
const BUILD_RESOURCES = [
    "/",
    `main.css?v=${VERSION}`,
    `valens-web-app-dioxus.js?v=${VERSION}`,
    `valens-web-app-dioxus_bg.wasm?v=${VERSION}`,
];

const CACHED_RESOURCES = [
    ...BUILD_RESOURCES,
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
    "manifest.json",
];

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
                // Every route of the app is answered by the app shell, which names the assets of
                // the build in this cache
                const cachedResponse = await caches.match(
                    request.mode === "navigate" ? "/" : request,
                    { cacheName: CACHE_NAME },
                );
                if (cachedResponse) {
                    return cachedResponse;
                }
            } catch (error) {
                console.error(error);
            }

            return fetch(request);
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

async function addResourcesToCache() {
    // A copy of another build in the HTTP cache must not be stored
    const responses = await Promise.all(CACHED_RESOURCES.map(async (resource) => {
        const response = await fetch(resource, { cache: "reload" });
        if (!response.ok) {
            throw new Error(`Request for ${resource} failed with status ${response.status}`);
        }
        return response;
    }));

    // A deploy during the installation can yield files of different builds. A missing header is
    // accepted, as it can be stripped by a proxy.
    CACHED_RESOURCES.forEach((resource, i) => {
        const version = responses[i].headers.get("Valens-Version");
        if (BUILD_RESOURCES.includes(resource) && version !== null && version !== VERSION) {
            throw new Error(`Response for ${resource} belongs to version ${version}`);
        }
    });

    try {
        const cache = await caches.open(CACHE_NAME);
        await Promise.all(CACHED_RESOURCES.map((resource, i) => cache.put(resource, responses[i])));
    } catch (error) {
        // An incomplete cache would otherwise remain until the next activation
        await caches.delete(CACHE_NAME);
        throw error;
    }
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
