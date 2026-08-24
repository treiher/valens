//! Timer, stopwatch and metronome, and the audio graph their beeps are played through.

use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
};

use chrono::{DateTime, Duration, Utc};
use dioxus::prelude::*;
use log::{error, warn};
use web_sys::{
    self,
    wasm_bindgen::{JsCast, closure::Closure},
};

use valens_web_app as web_app;

use crate::{
    METRONOME,
    ui::{
        element::Icon,
        form::{SelectField, SelectOption},
    },
    wake_lock::{self, WakeLock},
};

#[component]
pub fn Metronome() -> Element {
    rsx! {
        div {
            class: "field is-grouped is-grouped-centered",
            div {
                class: "mx-3",
                SelectField {
                    label: "Interval".to_string(),
                    options: (1..=60).map(|i| {
                        rsx! {
                            SelectOption {
                                text: i.to_string(),
                                value: i.to_string(),
                                selected: i == METRONOME.read().interval,
                            }
                        }
                    }).collect::<Vec<_>>(),
                    has_changed: false,
                    on_change: move |event: FormEvent| {
                        match event.value().parse::<u32>() {
                            Ok(v) => METRONOME.write().interval = v,
                            Err(e) => error!("failed to parse metronome interval: {e}"),
                        }
                    }
                }
            }
            div {
                class: "mx-3",
                SelectField {
                    label: "Stress".to_string(),
                    options: (1..=12).map(|i| {
                        rsx! {
                            SelectOption {
                                text: i.to_string(),
                                value: i.to_string(),
                                selected: i == METRONOME.read().stressed_beat,
                            }
                        }
                    }).collect::<Vec<_>>(),
                    has_changed: false,
                    on_change: move |event: FormEvent| {
                        match event.value().parse::<u32>() {
                            Ok(v) => METRONOME.write().stressed_beat = v,
                            Err(e) => error!("failed to parse metronome stressed beat: {e}"),
                        }
                    }
                }
            }
            div {
                class: "field mx-3",
                label { class: "label", "\u{a0}" }
                div { class: "control",
                    button {
                        class: "button",
                        r#type: "button",
                        onclick: move |_| METRONOME.write().start_pause(),
                        if METRONOME.read().is_active() {
                            Icon { name: "pause" }
                        } else {
                            Icon { name: "play" }
                        }
                    }
                }
            }
        }
    }
}

/// Interval at which the timer, the stopwatch and the metronome are advanced.
pub const TICK_INTERVAL_MS: u32 = 100;

/// How far ahead metronome beats are handed to the audio graph.
///
/// Must exceed `TICK_INTERVAL_MS` by a margin, so that a beat is always scheduled before it is due.
const METRONOME_LOOKAHEAD: f64 = 0.5;

/// Delay between starting the metronome and its first beat.
const METRONOME_START_DELAY: f64 = 0.5;

#[derive(Clone)]
pub struct MetronomeService {
    interval: u32,
    stressed_beat: u32,
    beat_number: u32,
    next_beat_time: f64,
    is_active: bool,
    beep_volume: u8,
}

impl MetronomeService {
    pub fn new() -> Self {
        Self {
            interval: 1,
            stressed_beat: 1,
            beat_number: 0,
            next_beat_time: 0.,
            is_active: false,
            beep_volume: 100,
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn start(&mut self) {
        resume_audio_context();
        self.is_active = true;
        if let Some(now) = audio_context_time() {
            self.beat_number = 0;
            self.next_beat_time = now + METRONOME_START_DELAY;
        }
    }

    pub fn pause(&mut self) {
        self.is_active = false;
    }

    pub fn start_pause(&mut self) {
        if self.is_active() {
            self.pause();
        } else {
            self.start();
        }
    }

    pub fn set_interval(&mut self, interval: u32) {
        self.interval = interval;
    }

    pub fn set_stressed_beat(&mut self, stressed_beat: u32) {
        self.stressed_beat = stressed_beat;
    }

    pub fn set_beep_volume(&mut self, beep_volume: u8) {
        self.beep_volume = beep_volume;
    }

    pub fn update(&mut self) {
        // The loop below would never terminate at an interval of zero.
        if !self.is_active() || self.interval == 0 {
            return;
        }

        let Some(now) = audio_context_time() else {
            return;
        };
        (self.next_beat_time, self.beat_number) =
            resync(self.next_beat_time, now, self.interval, self.beat_number);
        while self.next_beat_time < now + METRONOME_LOOKAHEAD {
            if let Err(err) = play_beep(
                if self.beat_number.is_multiple_of(self.stressed_beat) {
                    1000.
                } else {
                    500.
                },
                self.next_beat_time,
                0.05,
                self.beep_volume,
            ) {
                warn!("failed to play beep: {err:?}");
            }
            self.next_beat_time += f64::from(self.interval);
            self.beat_number += 1;
        }
    }
}

/// Skips the beats missed while the main thread was stalled.
///
/// Returns the next beat at or after `now` and the number of the beat sounding then, so that the
/// stress pattern is preserved.
fn resync(next_beat_time: f64, now: f64, interval: u32, beat_number: u32) -> (f64, u32) {
    if next_beat_time >= now || interval == 0 {
        return (next_beat_time, beat_number);
    }
    let interval = f64::from(interval);
    let missed_beats = ((now - next_beat_time) / interval).ceil();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let missed_beat_count = missed_beats.min(f64::from(u32::MAX)) as u32;
    (
        next_beat_time + missed_beats * interval,
        beat_number.saturating_add(missed_beat_count),
    )
}

#[component]
pub fn Stopwatch(stopwatch: Signal<StopwatchService>) -> Element {
    rsx! {
        p {
            class: "title is-size-1",
            onclick: move |_| stopwatch.write().toggle(),
            "{stopwatch.read().seconds():.1}"
        }
        PlayResetButtons {
            margin_top: 1,
            is_active: stopwatch.read().is_active(),
            on_start_pause: move |_| stopwatch.write().start_pause(),
            on_reset: move |_| stopwatch.write().reset(),
        }
    }
}

#[derive(Clone)]
pub struct StopwatchService {
    milliseconds: i64,
    start_time: Option<DateTime<Utc>>,
}

impl StopwatchService {
    pub fn new() -> Self {
        Self {
            milliseconds: 0,
            start_time: None,
        }
    }

    pub fn seconds(&self) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        (self.milliseconds as f64 / 1000.)
    }

    pub fn is_active(&self) -> bool {
        self.start_time.is_some()
    }

    pub fn toggle(&mut self) {
        if !self.is_active() && self.milliseconds > 0 {
            self.reset();
        } else {
            self.start_pause();
        }
    }

    pub fn start_pause(&mut self) {
        self.start_time = match self.start_time {
            Some(_) => None,
            None => Some(Utc::now() - Duration::milliseconds(self.milliseconds)),
        };
    }

    pub fn reset(&mut self) {
        self.milliseconds = 0;
        if self.start_time.is_some() {
            self.start_time = Some(Utc::now());
        }
    }

    pub fn update(&mut self) {
        if let Some(start_time) = self.start_time {
            self.milliseconds = Utc::now()
                .signed_duration_since(start_time)
                .num_milliseconds();
        }
    }
}

#[component]
pub fn Timer(timer: Store<TimerService>) -> Element {
    rsx! {
        div {
            class: if timer.read().is_active() { "" } else { "is-blinking" },
            onclick: move |_| {
                timer.write().start_pause();
            },
            "{timer.read().seconds()} s"
        }
    }
}

#[component]
pub fn MutableTimer(timer: Signal<TimerService>) -> Element {
    rsx! {
        div {
            class: "field",
            div {
                class: "control",
                input {
                    class: "input title is-size-1 has-text-centered",
                    inputmode: "numeric",
                    size: "4",
                    style: "height:auto; width:auto; padding:0",
                    value: "{timer.read().seconds()}",
                    oninput: move |event| {
                        match event.value().parse::<i64>() {
                            Ok(parsed_time) => {
                                timer.write().set(
                                    if parsed_time <= 9999 {
                                        parsed_time
                                    } else {
                                        9999
                                    });
                            }
                            Err(_) => {
                                timer.write().set(0);
                            }
                        }
                    }
                }
            }
        }
        PlayResetButtons {
            margin_top: 5,
            is_active: timer.read().is_active(),
            on_start_pause: move |_| timer.write().start_pause(),
            on_reset: move |_| timer.write().reset(),
        }
    }
}

/// How far ahead timer beeps are handed to the audio graph.
///
/// A beep inside the window survives a main thread stall of any length, but a suspended audio
/// context pins it, so that it fires on resumption rather than at its wall-clock moment. The
/// window is therefore also the upper bound on such a burst.
const SCHEDULE_LOOKAHEAD: f64 = 15.;

/// How long the screen is kept on after a countdown has reached zero.
const WAKE_LOCK_GRACE_PERIOD: i64 = 60;

/// Deviation between the audio clock and the wall clock above which the schedule is anchored anew.
///
/// Nothing corrects a smaller deviation, so this is also the accuracy of a beep.
const DRIFT_THRESHOLD: f64 = 0.15;

#[derive(Clone)]
pub struct TimerService {
    reset_seconds: i64,
    remaining_seconds: i64,
    target_time: Option<DateTime<Utc>>,
    // Shared between clones so that a stale clone cannot cancel the beeps of the live instance.
    schedule: Rc<RefCell<Schedule>>,
    wake_lock: Option<Rc<WakeLock>>,
    beep_volume: u8,
}

impl TimerService {
    pub fn new(seconds: i64) -> Self {
        let mut timer = Self::default();
        timer.set(seconds);
        timer
    }

    pub fn seconds(&self) -> i64 {
        self.remaining_seconds
    }

    pub fn is_set(&self) -> bool {
        self.reset_seconds != i64::MAX
    }

    pub fn is_active(&self) -> bool {
        self.target_time.is_some()
    }

    pub fn start(&mut self) {
        resume_audio_context();
        self.target_time = Some(Utc::now() + Duration::seconds(self.remaining_seconds));
        self.wake_lock = Some(wake_lock::hold());
        self.reschedule();
    }

    pub fn pause(&mut self) {
        self.target_time = None;
        self.wake_lock = None;
        self.clear_schedule();
    }

    pub fn start_pause(&mut self) {
        if self.is_active() {
            self.pause();
        } else {
            self.start();
        }
    }

    pub fn set(&mut self, seconds: i64) {
        resume_audio_context();
        self.reset_seconds = seconds;
        self.remaining_seconds = seconds;
        if self.target_time.is_some() {
            self.target_time = Some(Utc::now() + Duration::seconds(seconds));
            self.wake_lock = Some(wake_lock::hold());
            self.reschedule();
        }
    }

    pub fn unset(&mut self) {
        self.reset_seconds = i64::MAX;
        self.target_time = None;
        self.wake_lock = None;
        self.clear_schedule();
    }

    pub fn reset(&mut self) {
        self.set(self.reset_seconds);
    }

    pub fn set_beep_volume(&mut self, beep_volume: u8) {
        if beep_volume == self.beep_volume {
            return;
        }
        self.beep_volume = beep_volume;
        // The volume is baked into the pre-rendered beeps, so the scheduled ones would keep the
        // previous volume.
        self.requeue();
    }

    pub fn update(&mut self) {
        if let Some(remaining_seconds) = self.remaining() {
            self.remaining_seconds = remaining_seconds;
            // The countdown keeps running past zero, but the screen is only kept on for as long
            // as the elapsed time is likely to be read off it.
            if remaining_seconds <= -WAKE_LOCK_GRACE_PERIOD {
                self.wake_lock = None;
            }
        }
    }

    /// Keeps the scheduled beeps in step with the countdown.
    ///
    /// Extends the lookahead window by the beeps that entered it since the last call. A deviation
    /// between the audio clock and the wall clock beyond `DRIFT_THRESHOLD` means the audio context
    /// was suspended or the countdown was changed, so the schedule is anchored anew.
    pub fn sync(&self) {
        // Building nodes against the frozen clock of a suspended context would schedule beeps that
        // never fire and are therefore never reclaimed.
        if !audio_context_is_running() {
            return;
        }
        let (Some(now), Some(target_time)) = (audio_context_time(), self.target_time) else {
            return;
        };
        #[allow(clippy::cast_precision_loss)]
        let remaining_seconds = target_time
            .signed_duration_since(Utc::now())
            .num_milliseconds() as f64
            / 1000.;
        let expiry = now + remaining_seconds;
        let mut schedule = self.schedule.borrow_mut();
        if !matches!(schedule.expiry, Some(scheduled) if (scheduled - expiry).abs() <= DRIFT_THRESHOLD)
        {
            schedule.anchor(now, remaining_seconds);
        }
        schedule.extend(now, self.beep_volume);
    }

    /// Whether the remaining time has moved on to a different second.
    pub fn needs_update(&self) -> bool {
        matches!(self.remaining(), Some(seconds) if seconds != self.remaining_seconds)
    }

    /// The remaining time, rounded up so that a second is shown until the moment it is reached.
    ///
    /// Rounding up is what lets the beep of a second sound when its number appears.
    fn remaining(&self) -> Option<i64> {
        self.target_time.map(|target_time| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
            let remaining_seconds = (target_time
                .signed_duration_since(Utc::now())
                .num_milliseconds() as f64
                / 1000.)
                .ceil() as i64;
            remaining_seconds
        })
    }

    fn reschedule(&self) {
        let Some(now) = audio_context_time() else {
            return;
        };
        #[allow(clippy::cast_precision_loss)]
        let remaining_seconds = self.remaining_seconds as f64;
        let mut schedule = self.schedule.borrow_mut();
        schedule.anchor(now, remaining_seconds);
        // Anchoring a countdown restored outside a user gesture must happen even though no node can
        // be built yet, so that the first tick with a running context schedules the whole window.
        if audio_context_is_running() {
            schedule.extend(now, self.beep_volume);
        }
    }

    /// Rebuilds the beeps of the lookahead window without moving the countdown.
    fn requeue(&self) {
        let Some(now) = audio_context_time() else {
            return;
        };
        self.schedule.borrow_mut().requeue(now, self.beep_volume);
    }

    fn clear_schedule(&self) {
        let Some(now) = audio_context_time() else {
            return;
        };
        let mut schedule = self.schedule.borrow_mut();
        schedule.cancel_pending(now);
        schedule.expiry = None;
    }
}

impl Default for TimerService {
    fn default() -> Self {
        Self {
            reset_seconds: i64::MAX,
            remaining_seconds: i64::MAX,
            target_time: None,
            schedule: Rc::default(),
            wake_lock: None,
            beep_volume: 100,
        }
    }
}

impl From<web_app::TimerState> for TimerService {
    fn from(value: web_app::TimerState) -> Self {
        let mut timer = Self::default();
        match value {
            web_app::TimerState::Unset => {
                timer.unset();
            }
            web_app::TimerState::Active { target_time } => {
                timer.set((target_time - Utc::now()).num_seconds());
                timer.start();
            }
            web_app::TimerState::Paused { time } => {
                timer.set(time);
                timer.pause();
            }
        }
        timer
    }
}

impl From<TimerService> for web_app::TimerState {
    fn from(value: TimerService) -> Self {
        if value.is_active() {
            web_app::TimerState::Active {
                target_time: value.target_time.unwrap_or(Utc::now()),
            }
        } else if value.is_set() {
            web_app::TimerState::Paused {
                time: value.remaining_seconds,
            }
        } else {
            web_app::TimerState::Unset
        }
    }
}

/// Beeps handed to the audio graph ahead of time, and the moment the countdown they belong to
/// expires, in audio-clock terms.
#[derive(Default)]
struct Schedule {
    expiry: Option<f64>,
    scheduled_until: f64,
    pending: Vec<(f64, web_sys::AudioBufferSourceNode)>,
}

impl Schedule {
    fn anchor(&mut self, now: f64, remaining_seconds: f64) {
        self.cancel_pending(now);
        self.expiry = if remaining_seconds > 0. {
            Some(now + remaining_seconds)
        } else {
            None
        };
        self.scheduled_until = now;
    }

    /// Hands the audio graph the beeps starting within the lookahead window and not yet scheduled.
    fn extend(&mut self, now: f64, volume: u8) {
        let Some(expiry) = self.expiry else {
            return;
        };
        let from = self.scheduled_until.max(now);
        let to = now + SCHEDULE_LOOKAHEAD;
        if to <= from {
            return;
        }
        for beep in scheduled_beeps(expiry, from, to) {
            match play_beep(beep.frequency, beep.start, beep.length, volume) {
                Ok(Some(source)) => self.pending.push((beep.start, source)),
                Ok(None) => {}
                Err(err) => warn!("failed to play beep: {err:?}"),
            }
        }
        self.scheduled_until = to;
    }

    fn requeue(&mut self, now: f64, volume: u8) {
        self.cancel_pending(now);
        self.scheduled_until = now;
        self.extend(now, volume);
    }

    /// Stops the beeps that have not started yet and forgets all of them.
    ///
    /// A beep already sounding is left to finish, since stopping it would cut it mid-envelope. The
    /// beeps of a countdown that has expired are kept as well, so that the final beep sounds even
    /// when the countdown is replaced at that moment.
    fn cancel_pending(&mut self, now: f64) {
        let expired = matches!(self.expiry, Some(expiry) if now >= expiry - DRIFT_THRESHOLD);
        for (start, source) in self.pending.drain(..) {
            if !expired
                && start > now
                && let Err(err) = scheduled(&source).stop()
            {
                warn!("failed to stop beep: {err:?}");
            }
        }
    }
}

impl Drop for Schedule {
    fn drop(&mut self) {
        if let Some(now) = audio_context_time() {
            self.cancel_pending(now);
        }
    }
}

/// A beep of a countdown, as part of the cue starting at `remaining`.
struct Beep {
    remaining: f64,
    offset: f64,
    frequency: f32,
    length: f64,
}

struct ScheduledBeep {
    start: f64,
    frequency: f32,
    length: f64,
}

/// The beeps of a countdown, by the remaining time at which their cue starts.
///
/// The cue at ten seconds consists of two beeps, so that it is not mistaken for the single beep at
/// two seconds.
fn beeps() -> [Beep; 5] {
    [
        Beep {
            remaining: 10.,
            offset: 0.,
            frequency: 2000.,
            length: 0.1,
        },
        Beep {
            remaining: 10.,
            offset: 0.18,
            frequency: 2000.,
            length: 0.1,
        },
        Beep {
            remaining: 2.,
            offset: 0.,
            frequency: 2000.,
            length: 0.15,
        },
        Beep {
            remaining: 1.,
            offset: 0.,
            frequency: 2000.,
            length: 0.15,
        },
        Beep {
            remaining: 0.,
            offset: 0.,
            frequency: 2000.,
            length: 0.5,
        },
    ]
}

/// Tolerance of the window bounds against floating-point error.
///
/// `expiry` is obtained by adding the remaining time to the current time, so subtracting it again
/// need not reproduce the current time exactly. Without the tolerance, the cue at the very moment
/// a countdown starts could land just inside the window and beep at once.
const SCHEDULE_TOLERANCE: f64 = 1e-6;

/// Beeps of a countdown expiring at `expiry` whose cue starts in `(from, to]`, in audio-clock
/// terms.
///
/// The exclusive lower bound is what lets consecutive windows partition the beeps of a countdown.
/// A cue is scheduled as a whole, so that none of its beeps sounds on its own.
fn scheduled_beeps(expiry: f64, from: f64, to: f64) -> Vec<ScheduledBeep> {
    beeps()
        .into_iter()
        .filter(|beep| {
            let cue = expiry - beep.remaining;
            cue > from + SCHEDULE_TOLERANCE && cue <= to + SCHEDULE_TOLERANCE
        })
        .map(|beep| ScheduledBeep {
            start: expiry - beep.remaining + beep.offset,
            frequency: beep.frequency,
            length: beep.length,
        })
        .collect()
}

/// Plays a beep at `volume`, matching the tone and length of the beeps of an expiring countdown.
pub fn play_volume_preview(volume: u8) {
    resume_audio_context();
    let Some(now) = audio_context_time() else {
        return;
    };
    if let Err(err) = play_beep(2000., now, 0.15, volume) {
        warn!("failed to play beep: {err:?}");
    }
}

/// Duration of the fade at each edge of a beep.
const BEEP_RAMP: f64 = 0.005;

/// Schedules a beep and returns its source, or nothing if the beep would already be over.
///
/// The waveform is computed up front and played from a buffer rather than synthesized while it
/// sounds, so that the thread rendering the audio only has to copy it.
fn play_beep(
    frequency: f32,
    start: f64,
    length: f64,
    volume: u8,
) -> Result<Option<web_sys::AudioBufferSourceNode>, web_sys::wasm_bindgen::JsValue> {
    with_audio_context(|audio_context| {
        let now = audio_context.current_time();
        if start + length <= now {
            return Ok(None);
        }
        let start = start.max(now);
        let sample_rate = audio_context.sample_rate();
        let samples = beep_samples(frequency, length, volume, sample_rate);

        #[allow(clippy::cast_possible_truncation)]
        let buffer = audio_context.create_buffer(1, samples.len() as u32, sample_rate)?;
        buffer.copy_to_channel(&samples, 0)?;
        let source = audio_context.create_buffer_source()?;
        source.set_buffer(Some(&buffer));
        source.connect_with_audio_node(&audio_context.destination())?;
        scheduled(&source).start_with_when(start)?;

        let played_source = source.clone();
        let disconnect = Closure::once_into_js(move |_: web_sys::Event| {
            if let Err(err) = played_source.disconnect() {
                warn!("failed to disconnect beep: {err:?}");
            }
        });
        scheduled(&source).set_onended(Some(disconnect.unchecked_ref()));

        Ok(Some(source))
    })
    .unwrap_or(Ok(None))
}

/// The waveform of a beep, a sine faded in and out to avoid the clicks of a hard edge.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn beep_samples(frequency: f32, length: f64, volume: u8, sample_rate: f32) -> Vec<f32> {
    let count = ((length * f64::from(sample_rate)).round() as usize).max(1);
    let peak = f64::from(volume) / 100.;
    let ramp = BEEP_RAMP.min(length / 2.);
    (0..count)
        .map(|sample| {
            let time = sample as f64 / f64::from(sample_rate);
            let envelope = if ramp <= 0. {
                1.
            } else if time < ramp {
                time / ramp
            } else if time > length - ramp {
                (length - time) / ramp
            } else {
                1.
            };
            ((time * f64::from(frequency) * std::f64::consts::TAU).sin() * peak * envelope) as f32
        })
        .collect()
}

fn scheduled(source: &web_sys::AudioBufferSourceNode) -> &web_sys::AudioScheduledSourceNode {
    source.as_ref()
}

thread_local! {
    static AUDIO_CONTEXT: OnceCell<Option<web_sys::AudioContext>> = const { OnceCell::new() };
    static RESUME_FAILURE_LOGGED: Cell<bool> = const { Cell::new(false) };
}

fn audio_context_time() -> Option<f64> {
    with_audio_context(web_sys::AudioContext::current_time)
}

fn audio_context_is_running() -> bool {
    with_audio_context(|audio_context| audio_context.state() == web_sys::AudioContextState::Running)
        .unwrap_or(false)
}

fn resume_audio_context() {
    with_audio_context(|audio_context| {
        if audio_context.state() == web_sys::AudioContextState::Running {
            RESUME_FAILURE_LOGGED.set(false);
            return;
        }
        if let Err(err) = audio_context.resume()
            && !RESUME_FAILURE_LOGGED.replace(true)
        {
            warn!("failed to resume audio context: {err:?}");
        }
    });
}

/// Runs `f` on the audio context shared by all timers and metronomes, creating it on first use.
fn with_audio_context<R>(f: impl FnOnce(&web_sys::AudioContext) -> R) -> Option<R> {
    AUDIO_CONTEXT.with(|audio_context| {
        audio_context
            .get_or_init(|| match web_sys::AudioContext::new() {
                Ok(audio_context) => {
                    listen_for_user_gestures();
                    Some(audio_context)
                }
                Err(err) => {
                    warn!("failed to create audio context: {err:?}");
                    None
                }
            })
            .as_ref()
            .map(f)
    })
}

/// Resumes the audio context on the next user interaction.
///
/// A context created outside a user gesture starts suspended under the autoplay policy and stays
/// silent until a gesture allows it to run.
fn listen_for_user_gestures() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        warn!("failed to access document");
        return;
    };
    let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
        resume_audio_context();
    }) as Box<dyn FnMut(web_sys::Event)>);
    if let Err(err) =
        document.add_event_listener_with_callback("pointerdown", closure.as_ref().unchecked_ref())
    {
        warn!("failed to listen for user gestures: {err:?}");
        return;
    }
    closure.forget();
}

#[component]
fn PlayResetButtons(
    margin_top: u8,
    is_active: bool,
    on_start_pause: EventHandler<MouseEvent>,
    on_reset: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            class: "button mt-{margin_top} mx-3",
            r#type: "button",
            onclick: on_start_pause,
            if is_active {
                Icon { name: "pause" }
            } else {
                Icon { name: "play" }
            }
        }
        button {
            class: "button mt-{margin_top} mx-3",
            r#type: "button",
            onclick: on_reset,
            Icon { name: "rotate-left" }
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use super::{SCHEDULE_LOOKAHEAD, beep_samples, resync, scheduled_beeps};

    #[test]
    fn beep_samples_are_faded_in_and_out() {
        let samples = beep_samples(1000., 0.1, 100, 48000.);
        assert_eq!(samples.len(), 4800);
        assert_approx_eq!(samples[0], 0., 1e-6);
        assert_approx_eq!(samples[samples.len() - 1], 0., 1e-3);
        assert!(samples.iter().all(|sample| sample.abs() <= 1.));
        assert!(samples.iter().any(|sample| sample.abs() > 0.99));
    }

    #[test]
    fn beep_samples_scale_with_the_volume() {
        let samples = beep_samples(1000., 0.1, 50, 48000.);
        assert!(samples.iter().all(|sample| sample.abs() <= 0.5));
        assert!(samples.iter().any(|sample| sample.abs() > 0.49));
    }

    #[test]
    fn beep_samples_of_a_beep_shorter_than_its_fades_stay_bounded() {
        let samples = beep_samples(1000., 0.002, 100, 48000.);
        assert_eq!(samples.len(), 96);
        assert!(samples.iter().all(|sample| sample.abs() <= 1.));
    }

    fn beep_starts(expiry: f64, from: f64, to: f64) -> Vec<f64> {
        scheduled_beeps(expiry, from, to)
            .into_iter()
            .map(|beep| beep.start)
            .collect()
    }

    #[test]
    fn scheduled_beeps_are_ordered_and_relative_to_expiry() {
        let starts = beep_starts(100., 0., 100.);
        assert_eq!(starts.len(), 5);
        for (start, expected) in starts.iter().zip([90., 90.18, 98., 99., 100.]) {
            assert_approx_eq!(start, expected, 1e-9);
        }
        assert!(starts.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn scheduled_beeps_omits_passed_moments() {
        let starts = beep_starts(5., 0., 5.);
        assert_eq!(starts.len(), 3);
        assert_approx_eq!(starts[0], 3., 1e-9);
        assert!(beep_starts(0., 0., SCHEDULE_LOOKAHEAD).is_empty());
    }

    #[test]
    fn scheduled_beeps_stays_within_window() {
        assert!(beep_starts(90., 0., SCHEDULE_LOOKAHEAD).is_empty());
    }

    #[test]
    fn scheduled_beeps_excludes_lower_and_includes_upper_bound() {
        assert!(beep_starts(100., 90., 90.1).is_empty());
        assert_eq!(beep_starts(100., 89.9, 90.).len(), 2);
    }

    #[test]
    fn scheduled_beeps_omits_a_cue_starting_with_the_countdown() {
        let start_time = 1234.5678;
        let starts = beep_starts(
            start_time + 10.,
            start_time,
            start_time + SCHEDULE_LOOKAHEAD,
        );
        assert_eq!(starts.len(), 3);
        assert_approx_eq!(starts[0], start_time + 8., 1e-9);
    }

    #[test]
    fn consecutive_windows_partition_beeps() {
        let expiry = 100.;
        let mut scheduled_until: f64 = 0.;
        let mut starts = vec![];
        for tick in 0..=1010 {
            let now = f64::from(tick) / 10.;
            let from = scheduled_until.max(now);
            let to = now + SCHEDULE_LOOKAHEAD;
            if to > from {
                starts.extend(beep_starts(expiry, from, to));
                scheduled_until = to;
            }
        }
        assert_eq!(starts.len(), 5);
    }

    #[test]
    fn resync_skips_missed_beats() {
        assert_eq!(resync(10., 25.5, 2, 3), (26., 11));
    }

    #[test]
    fn resync_keeps_upcoming_beat() {
        assert_eq!(resync(10., 9.5, 2, 3), (10., 3));
    }
}
