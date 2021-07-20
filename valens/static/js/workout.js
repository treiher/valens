"use strict";

function $(id) {
    return document.getElementById(id);
}

const Stopwatch = function(display, playButton, resetButton) {
    display.onclick = function() {
        if (time == 0 || interval) {
            toggle();
        } else {
            reset();
        }
    };
    playButton.onclick = toggle;
    resetButton.onclick = reset;

    const playIcon = "&#9654;";
    const pauseIcon = "&#9208;";
    const resetIcon = "&#8634;";

    playButton.innerHTML = playIcon;
    resetButton.innerHTML = resetIcon;

    let time;
    let startTime;
    let interval;

    reset();

    function toggle() {
        if (interval) {
            clearInterval(interval);
            interval = null;
            playButton.innerHTML = playIcon;
        } else {
            startTime = Date.now();
            interval = setInterval(update, 10);
            playButton.innerHTML = pauseIcon;
        }
    }

    function reset() {
        time = 0;
        render();
    }

    function update() {
        let now = Date.now();
        time += now - startTime;
        startTime = now;
        render();
    }

    function render() {
        display.innerHTML = (time / 1000).toFixed(1);
    }
};

const Metronome = function(playButton, intervalSelect, stressSelect) {
    playButton.onclick = play;
    intervalSelect.onchange = function() {
        let interval_value = intervalSelect.value;
        if (interval_value && interval_value > 0) {
            interval = parseFloat(interval_value);
        }
    };
    stressSelect.onchange = function() {
        let stress_value = stressSelect.value;
        if (stress_value && stress_value >= 1) {
            stressedBeat = Math.round(parseFloat(stress_value));
        }
    };

    const playIcon = "&#9654;";
    const pauseIcon = "&#9208;";

    playButton.innerHTML = playIcon;

    let isPlaying = false;
    let beatNumber = 0;
    let beatLength = 0.05;
    let nextBeatTime = 0;
    let stressedBeat = 1;
    let interval = 1;
    let audioContext = new AudioContext();
    let clock = new Worker("../static/js/clock.js");

    clock.onmessage = function(e) {
        if (e.data == "tick") {
            scheduleBeat();
        }
    };

    function scheduleBeat() {
        while (nextBeatTime < audioContext.currentTime + 0.1) {
            let oscillator = audioContext.createOscillator();
            oscillator.type = "sine";
            oscillator.connect(audioContext.destination);
            if (beatNumber % stressedBeat === 0) {
                oscillator.frequency.value = 1000;
            } else {
                oscillator.frequency.value = 500;
            }

            oscillator.start(nextBeatTime);
            oscillator.stop(nextBeatTime + beatLength);

            nextBeatTime += interval;
            beatNumber++;
        }
    }

    function play() {
        isPlaying = !isPlaying;

        if (isPlaying) {
            beatNumber = 0;
            nextBeatTime = audioContext.currentTime + beatLength;
            clock.postMessage("start");
            playButton.innerHTML = pauseIcon;
        } else {
            clock.postMessage("stop");
            playButton.innerHTML = playIcon;
        }
    }
};

function openModal() {
    $("modal").classList.add("is-active");
}

function closeModal() {
    $("modal").classList.remove("is-active");
}

function init() {
    $("modal-open").onclick = openModal;
    for (const e of document.getElementsByClassName("modal-close")) {
        e.onclick = closeModal;
    }
    for (const e of document.getElementsByClassName("modal-background")) {
        e.onclick = closeModal;
    }

    new Stopwatch($("stopwatch-time"), $("stopwatch-play"), $("stopwatch-reset"));

    new Metronome($("metronome-play"), $("metronome-interval"), $("metronome-stress"));
}

window.addEventListener("load", init);
