var interval = null;

self.onmessage = function(e) {
    if (e.data == "start") {
        interval = setInterval(function() {postMessage("tick");}, 50);
    } else if (e.data == "stop") {
        clearInterval(interval);
        interval = null;
    }
};
