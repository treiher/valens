//! Shared, domain-aware UI components used across multiple pages.

use std::{
    cell::{Cell, OnceCell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};
use dioxus::prelude::*;
use log::{error, warn};
use web_sys::{
    self,
    wasm_bindgen::{JsCast, closure::Closure},
};

use valens_domain::{self as domain, Property};
use valens_web_app as web_app;

use crate::{
    DROP_SET_CALCULATOR, METRONOME, ONE_REP_MAX_CALCULATOR,
    current_date::current_date,
    settings::Settings,
    ui::{
        element::{Dialog, Error, Icon, NoData, TagsWithAddon},
        form::{FieldValue, InputField, SelectField, SelectOption},
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
                    max: "9999",
                    min: "0",
                    r#type: "number",
                    size: "4",
                    step: "1",
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

#[component]
pub fn IntervalControl(
    current_interval: Signal<domain::Interval>,
    all: domain::Interval,
) -> Element {
    let current = current_interval.read();
    let today = current_date();
    let duration = current.last - current.first + Duration::days(1);
    let intervals = [
        (
            "1M",
            today - Duration::days(domain::DefaultInterval::_1M as i64),
            today,
            current.last == today
                && duration == Duration::days(domain::DefaultInterval::_1M as i64 + 1),
        ),
        (
            "3M",
            today - Duration::days(domain::DefaultInterval::_3M as i64),
            today,
            current.last == today
                && duration == Duration::days(domain::DefaultInterval::_3M as i64 + 1),
        ),
        (
            "6M",
            today - Duration::days(domain::DefaultInterval::_6M as i64),
            today,
            current.last == today
                && duration == Duration::days(domain::DefaultInterval::_6M as i64 + 1),
        ),
        (
            "1Y",
            today - Duration::days(domain::DefaultInterval::_1Y as i64),
            today,
            current.last == today
                && duration == Duration::days(domain::DefaultInterval::_1Y as i64 + 1),
        ),
        (
            "NOW",
            all.first,
            today,
            current.first == all.first && current.last == today,
        ),
        (
            "ALL",
            all.first,
            all.last,
            current.first == all.first && current.last == all.last,
        ),
        (
            "+",
            if current.first + Duration::days(6) <= current.last - duration / 2 {
                current.first + duration / 4
            } else {
                current.first
            },
            if current.first + Duration::days(6) <= current.last - duration / 2 {
                current.last - duration / 4
            } else {
                current.first + Duration::days(6)
            },
            false,
        ),
        (
            "−",
            if current.first - duration / 2 > all.first {
                current.first - duration / 2
            } else {
                all.first
            },
            if current.last + duration / 2 < today {
                current.last + duration / 2
            } else {
                today
            },
            false,
        ),
    ];

    let left_first = if current.first - duration / 4 > all.first {
        current.first - duration / 4
    } else {
        all.first
    };
    let left_last = if current.first - duration / 4 > all.first {
        current.last - duration / 4
    } else {
        all.first + duration - Duration::days(1)
    };
    let is_left_disabled = current.first == left_first;

    let right_first = if current.last + duration / 4 < today {
        current.first + duration / 4
    } else {
        today - duration + Duration::days(1)
    };
    let right_last = if current.last + duration / 4 < today {
        current.last + duration / 4
    } else {
        today
    };
    let is_right_disabled = current.last == right_last;

    rsx! {
        div {
            class: "field has-addons has-addons-centered",
            for (name, first, last, is_active) in intervals {
                p {
                    class: "control",
                    a {
                        class: "button is-small",
                        class: if is_active { "is-link" },
                        "data-testid": "interval-{name}",
                        onclick: move |_| { *current_interval.write() = domain::Interval { first, last } },
                        "{name}"
                    }
                }
            }
        }
        div {
            class: "is-flex is-align-items-center is-justify-content-center mb-4",
            button {
                class: "button is-small",
                disabled: is_left_disabled,
                onclick: move |_| { *current_interval.write() = domain::Interval { first: left_first, last: left_last } },
                Icon { name: "chevron-left" }
            }
            span {
                class: "mx-3",
                "{current.first} – {current.last}"
            }
            button {
                class: "button is-small",
                disabled: is_right_disabled,
                onclick: move |_| { *current_interval.write() = domain::Interval { first: right_first, last: right_last } },
                Icon { name: "chevron-right" }
            }
        }
    }
}

#[component]
pub fn Chart(
    series: Vec<web_app::chart::LabeledSeries>,
    interval: domain::Interval,
    no_data_label: bool,
) -> Element {
    let settings = use_context::<Settings>();
    // The size of the SVG and the pixel positions of the samples depend on the window width at
    // the time of plotting
    use_window_width();
    let labels: Vec<web_app::chart::ChartLabel> = series
        .iter()
        .map(web_app::chart::LabeledSeries::label)
        .collect();
    let data: Vec<web_app::chart::PlotData> =
        series.into_iter().rev().flat_map(|s| s.data).collect();
    let chart =
        web_app::chart::plot(&data, interval, settings.current_theme()).map_err(|e| e.to_string());

    match chart {
        Ok(None) => {
            if no_data_label {
                rsx! {
                    NoData {}
                }
            } else {
                rsx! {}
            }
        }
        Ok(Some(result)) => {
            let web_app::chart::PlotResult { svg, series, area } = result;

            let mut marks: Vec<(NaiveDate, i32)> = series
                .iter()
                .flat_map(|s| s.high.iter().chain(s.low.iter().flatten()))
                .map(|sample| (sample.date, sample.x))
                .collect();
            marks.sort_by_key(|(_, x)| *x);
            marks.dedup_by_key(|(date, _)| *date);

            rsx! {
                div {
                    class: "container has-text-centered",
                    h1 {
                        class: "is-size-6 has-text-weight-bold",
                        {
                            labels
                                .iter()
                                .map(|label| {
                                    let color = web_app::chart::hex_color(label.color, label.opacity);
                                    rsx! {
                                        span {
                                            class: "icon-text mx-1",
                                            span {
                                                class: "icon",
                                                style: "color:{color}",
                                                i { class: "fas fa-square" }
                                            }
                                            span { "{label.name}" }
                                        }
                                    }
                                })
                        }
                    }
                    div {
                        "data-testid": "chart",
                        style: "position: relative; display: inline-block; touch-action: pan-y;
                                -webkit-touch-callout: none; user-select: none; -webkit-user-select: none;",
                        div { dangerous_inner_html: svg }
                        ChartOverlay { marks, series, area }
                    }
                }
            }
        }
        Err(err) => rsx! { Error { message: "{err}" } },
    }
}

static WINDOW_WIDTH: GlobalSignal<f64> = Signal::global(window_width);
static RESIZE_LISTENER: std::sync::Once = std::sync::Once::new();

/// Subscribes the component to changes of the window width and returns the current width.
fn use_window_width() -> f64 {
    use_hook(|| {
        RESIZE_LISTENER.call_once(|| {
            let Some(window) = web_sys::window() else {
                warn!("failed to access window");
                return;
            };
            let closure = Closure::<dyn FnMut()>::new(|| {
                *WINDOW_WIDTH.write() = window_width();
            });
            if let Err(e) =
                window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
            {
                warn!("failed to register resize handler: {e:?}");
                return;
            }
            closure.forget();
        });
    });

    WINDOW_WIDTH()
}

fn window_width() -> f64 {
    web_sys::window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|width| width.as_f64())
        .unwrap_or_default()
}

/// Transparent layer over a chart that tracks the pointer and renders the
/// hover crosshair, markers and value tooltip.
///
/// Owns the hover state, so pointer movement re-renders only this overlay and
/// not the chart SVG.
#[component]
fn ChartOverlay(
    marks: Vec<(NaiveDate, i32)>,
    series: Vec<web_app::chart::SeriesSamples>,
    area: web_app::chart::PlotArea,
) -> Element {
    let mut hovered = use_signal(|| None::<i32>);
    let active = hovered.read().and_then(|x| nearest_mark(&marks, x));

    // A tap emits no pointer movement, so the position must also be tracked on pointer down
    let track_pointer = move |event: PointerEvent| {
        #[allow(clippy::cast_possible_truncation)]
        let x = event.element_coordinates().x as i32;
        if x < area.left || x > area.right {
            hovered.set(None);
        } else {
            hovered.set(Some(x));
        }
    };

    rsx! {
        div {
            "data-testid": "chart-overlay",
            style: "position: absolute; inset: 0;
                    -webkit-touch-callout: none; user-select: none; -webkit-user-select: none;",
            // A long press must not open the browser's context menu
            oncontextmenu: move |event| event.prevent_default(),
            onpointerdown: track_pointer,
            onpointermove: track_pointer,
            onpointerleave: move |_| hovered.set(None),
            onpointercancel: move |_| hovered.set(None),
            if let Some((date, x)) = active {
                ChartHover { series, area, date, x }
            }
        }
    }
}

#[component]
fn ChartHover(
    series: Vec<web_app::chart::SeriesSamples>,
    area: web_app::chart::PlotArea,
    date: NaiveDate,
    x: i32,
) -> Element {
    // One tooltip row per series; a band (both edges present) collapses to a
    // single `low – high` range row.
    let mut rows: Vec<(String, String)> = vec![];
    let mut dots: Vec<(String, i32, i32)> = vec![];
    for s in &series {
        let color = web_app::chart::hex_color(s.color, s.opacity);
        let high = s.high.iter().find(|p| p.date == date);
        let low = s
            .low
            .as_ref()
            .and_then(|l| l.iter().find(|p| p.date == date));
        for sample in high.into_iter().chain(low) {
            dots.push((color.clone(), sample.x, sample.y));
        }
        let label = match (high, low) {
            (Some(h), Some(l)) => {
                let (lo, hi) = (h.value.min(l.value), h.value.max(l.value));
                format!("{} – {}", format_value(lo), format_value(hi))
            }
            (Some(p), None) | (None, Some(p)) => format_value(p.value),
            (None, None) => continue,
        };
        rows.push((color, label));
    }
    // The plotted samples are built from the series in reverse order, so undo
    // that here to match the order of the legend.
    rows.reverse();

    let tooltip_transform = if x.saturating_mul(2) > area.left + area.right {
        "translateX(calc(-100% - 8px))"
    } else {
        "translateX(8px)"
    };

    rsx! {
        div {
            style: "position: absolute; pointer-events: none; width: 1px;
                    left: {x}px; top: {area.top}px; height: {area.bottom - area.top}px;
                    background: rgba(128, 128, 128, 0.8);",
        }
        for (color, dot_x, dot_y) in dots {
            div {
                style: "position: absolute; pointer-events: none; border-radius: 50%;
                        width: 7px; height: 7px; margin: -4px 0 0 -4px;
                        left: {dot_x}px; top: {dot_y}px; background: {color};",
            }
        }
        div {
            class: "box p-2 has-text-left",
            "data-testid": "chart-tooltip",
            style: "position: absolute; pointer-events: none; white-space: nowrap; z-index: 1;
                    left: {x}px; top: {area.top}px; transform: {tooltip_transform};",
            div { class: "is-size-7 has-text-centered has-text-weight-bold", "{date}" }
            for (color, label) in rows {
                div {
                    class: "icon-text is-size-7",
                    style: "flex-wrap: nowrap",
                    span {
                        class: "icon",
                        style: "color: {color}",
                        i { class: "fas fa-square" }
                    }
                    span { "{label}" }
                }
            }
        }
    }
}

fn nearest_mark(marks: &[(NaiveDate, i32)], x: i32) -> Option<(NaiveDate, i32)> {
    marks
        .iter()
        .min_by_key(|(_, mark_x)| (mark_x - x).abs())
        .copied()
}

fn format_value(value: f32) -> String {
    let formatted = format!("{value:.2}");
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    // Negative values that round to zero must not be displayed as `-0`
    if trimmed == "-0" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

#[component]
pub fn Calendar(entries: Vec<(NaiveDate, usize, f64)>, interval: domain::Interval) -> Element {
    let mut calendar: BTreeMap<NaiveDate, (usize, f64)> = BTreeMap::new();

    let mut day = interval.first.week(Weekday::Mon).first_day();
    while day <= interval.last.week(Weekday::Mon).last_day() {
        calendar.insert(day, (0, 0.));
        day += Duration::days(1);
    }

    for (date, color, opacity) in entries {
        calendar.entry(date).and_modify(|e| *e = (color, opacity));
    }

    let mut weekdays: [Vec<(NaiveDate, usize, f64)>; 7] = Default::default();
    let mut months: Vec<(NaiveDate, usize)> = vec![];
    let mut month: NaiveDate = NaiveDate::default();
    let mut num_weeks: usize = 0;
    for (i, (date, (color, opacity))) in calendar.iter().enumerate() {
        weekdays[i % 7].push((*date, *color, *opacity));
        if i % 7 == 0 || i == calendar.len() - 1 {
            if i == 0 {
                month = *date;
            } else if month.month() != date.month() || i == calendar.len() - 1 {
                months.push((month, num_weeks));
                num_weeks = 0;
                month = *date;
            }
            num_weeks += 1;
        }
    }

    rsx! {
        div {
            class: "table-container is-calendar py-2",
            table {
                class: "table is-size-7 mx-auto",
                tbody {
                    tr {
                        for (date, colspan) in months {
                            td {
                                class: "is-calendar-label",
                                colspan: colspan,
                                if colspan > 1 {
                                    "{date.year()}-{date.month():02}"
                                }
                            }
                        },
                        td { class: "is-calendar-label" }
                    }
                    for weekday in 0..weekdays.len() {
                        tr {
                            for (date, color, opacity) in weekdays[weekday].clone() {
                                td {
                                    style: if opacity > 0. {
                                        "background-color:{web_app::chart::rgba_color(color, opacity)}"
                                    } else if date < interval.first || date > interval.last {
                                        "background-color:var(--bulma-scheme-main)"
                                    },
                                    div { "{date.day()}" }
                                }
                            }
                            td {
                                class: "is-calendar-label",
                                match weekday {
                                    0 => "Mon",
                                    1 => "Tue",
                                    2 => "Wed",
                                    3 => "Thu",
                                    4 => "Fri",
                                    5 => "Sat",
                                    6 => "Sun",
                                    _ => "",
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SetsPerMuscle(stimulus_per_muscle: BTreeMap<domain::MuscleID, domain::Stimulus>) -> Element {
    let mut stimulus_per_muscle = stimulus_per_muscle
        .iter()
        .map(|(muscle_id, stimulus)| (*muscle_id, *stimulus))
        .collect::<Vec<_>>();
    stimulus_per_muscle.sort_by_key(|b| std::cmp::Reverse(b.1));
    let mut groups = [vec![], vec![], vec![], vec![]];
    for (muscle, stimulus) in stimulus_per_muscle {
        let name = muscle.name();
        let description = muscle.description();
        let sets = f64::from(*stimulus) / 100.0;
        let sets_str = format!("{:.1$}", sets, usize::from(sets.fract() != 0.0));
        if sets > 10.0 {
            groups[0].push((name, description, sets_str, vec!["is-dark"]));
        } else if sets >= 3.0 {
            groups[1].push((name, description, sets_str, vec!["is-dark", "is-link"]));
        } else if sets > 0.0 {
            groups[2].push((name, description, sets_str, vec!["is-light", "is-link"]));
        } else {
            groups[3].push((name, description, sets_str, vec![]));
        }
    }
    rsx! {
        for tags in groups {
            if !tags.is_empty() {
                TagsWithAddon { tags }
            }
        }
    }
}

#[component]
pub fn OneRepMaxCalculator() -> Element {
    let initial = ONE_REP_MAX_CALCULATOR.read().clone();
    let mut reps_input = use_signal(|| FieldValue::new(initial.reps));
    let mut weight_input = use_signal(|| FieldValue::new(initial.weight));

    let reps = ONE_REP_MAX_CALCULATOR.read().reps;
    let weight = ONE_REP_MAX_CALCULATOR.read().weight;
    #[allow(clippy::cast_precision_loss)]
    let one_rep_max = domain::one_rep_max(reps as f32, weight);
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let table_rows: Vec<(u32, u32, f32)> = (50u32..=100)
        .rev()
        .step_by(5)
        .map(|p| {
            (
                p,
                domain::reps_for_percentage(p as f32).round() as u32,
                p as f32 / 100.0 * one_rep_max,
            )
        })
        .collect();

    rsx! {
        Dialog {
            on_close: move |_| {
                ONE_REP_MAX_CALCULATOR.write().visible = false;
            },
            div {
                class: "columns is-mobile",
                div {
                    class: "column",
                    InputField {
                        label: "Reps".to_string(),
                        r#type: "number",
                        min: "0",
                        max: "999",
                        step: 1,
                        value: reps_input.read().input.clone(),
                        error: if let Err(err) = &reps_input.read().validated { err.clone() },
                        has_changed: false,
                        "data-testid": "1rm-reps",
                        on_input: move |event: FormEvent| {
                            let input = event.value();
                            let validated = domain::Reps::try_from(input.trim())
                                .map(u32::from)
                                .map_err(|err| err.to_string());
                            if let Ok(value) = &validated {
                                ONE_REP_MAX_CALCULATOR.write().reps = *value;
                            }
                            let mut fv = reps_input.write();
                            fv.input = input;
                            fv.validated = validated;
                        },
                    }
                }
                div {
                    class: "column",
                    InputField {
                        label: "Weight".to_string(),
                        right_icon: rsx! { "kg" },
                        inputmode: "numeric",
                        value: weight_input.read().input.clone(),
                        error: if let Err(err) = &weight_input.read().validated { err.clone() },
                        has_changed: false,
                        "data-testid": "1rm-weight",
                        on_input: move |event: FormEvent| {
                            let input = event.value();
                            let validated = domain::Weight::try_from(input.trim())
                                .map(f32::from)
                                .map_err(|err| err.to_string());
                            if let Ok(value) = &validated {
                                ONE_REP_MAX_CALCULATOR.write().weight = *value;
                            }
                            let mut fv = weight_input.write();
                            fv.input = input;
                            fv.validated = validated;
                        },
                    }
                }
            }
            table {
                class: "table is-striped is-fullwidth",
                style: "white-space: nowrap",
                thead {
                    tr {
                        th { class: "has-text-right", "% 1RM" }
                        th { class: "has-text-right", "Reps" }
                        th { class: "has-text-right", "Weight (kg)" }
                    }
                }
                tbody {
                    for (percentage, row_reps, row_weight) in &table_rows {
                        tr {
                            td { class: "has-text-right", "{percentage}" }
                            td { class: "has-text-right", "{row_reps}" }
                            td { class: "has-text-right", "{row_weight:.2}" }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct OneRepMaxCalculatorState {
    pub visible: bool,
    pub reps: u32,
    pub weight: f32,
}

impl OneRepMaxCalculatorState {
    #[must_use]
    pub fn new(reps: u32, weight: f32) -> Self {
        Self {
            visible: false,
            reps,
            weight,
        }
    }
}

#[component]
pub fn DropSetCalculator() -> Element {
    let state = DROP_SET_CALCULATOR.read().clone();
    let mut start_weight_input = use_signal(|| FieldValue::new(state.start_weight));
    let mut drop_percentage_input = use_signal(|| FieldValue::new(state.drop_percentage));

    let start_weight = state.start_weight;
    let drop_percentage = state.drop_percentage;
    let increment = state.increment;
    let weights = domain::drop_set_weights(start_weight, drop_percentage, increment);
    let dp = decimal_places(increment);

    rsx! {
        Dialog {
            on_close: move |_| {
                DROP_SET_CALCULATOR.write().visible = false;
            },
            div {
                class: "columns is-mobile",
                div {
                    class: "column",
                    InputField {
                        label: "Start".to_string(),
                        right_icon: rsx! { "kg" },
                        inputmode: "numeric",
                        value: start_weight_input.read().input.clone(),
                        error: if let Err(err) = &start_weight_input.read().validated { err.clone() },
                        has_changed: false,
                        "data-testid": "drop-set-start-weight",
                        on_input: move |event: FormEvent| {
                            let input = event.value();
                            let validated = domain::Weight::try_from(input.trim())
                                .map(f32::from)
                                .map_err(|err| err.to_string());
                            if let Ok(value) = &validated {
                                DROP_SET_CALCULATOR.write().start_weight = *value;
                            }
                            let mut fv = start_weight_input.write();
                            fv.input = input;
                            fv.validated = validated;
                        },
                    }
                }
                div {
                    class: "column",
                    InputField {
                        label: "Drop".to_string(),
                        right_icon: rsx! { "%" },
                        inputmode: "numeric",
                        value: drop_percentage_input.read().input.clone(),
                        error: if let Err(err) = &drop_percentage_input.read().validated { err.clone() },
                        has_changed: false,
                        "data-testid": "drop-set-drop-percentage",
                        on_input: move |event: FormEvent| {
                            let input = event.value();
                            let validated = parse_drop_percentage(input.trim());
                            if let Ok(value) = &validated {
                                DROP_SET_CALCULATOR.write().drop_percentage = *value;
                            }
                            let mut fv = drop_percentage_input.write();
                            fv.input = input;
                            fv.validated = validated;
                        },
                    }
                }
                div {
                    class: "column",
                    SelectField {
                        label: "Increment".to_string(),
                        options: DROP_SET_INCREMENT_PRESETS.iter().map(|preset| {
                            rsx! {
                                SelectOption {
                                    text: format!("{preset} kg"),
                                    value: preset.to_string(),
                                    selected: (preset - increment).abs() < 1e-4,
                                }
                            }
                        }).collect::<Vec<_>>(),
                        has_changed: false,
                        is_fullwidth: true,
                        "data-testid": "drop-set-increment",
                        on_change: move |event: FormEvent| {
                            if let Ok(value) = event.value().parse::<f32>() {
                                DROP_SET_CALCULATOR.write().increment = value;
                            }
                        },
                    }
                }
            }
            table {
                class: "table is-striped is-fullwidth",
                style: "white-space: nowrap",
                thead {
                    tr {
                        th { class: "has-text-right", "Nominal %" }
                        th { class: "has-text-right", "Actual %" }
                        th { class: "has-text-right", "Weight (kg)" }
                    }
                }
                tbody {
                    tr {
                        td { class: "has-text-right", "100.0" }
                        td { class: "has-text-right", "100.0" }
                        td { class: "has-text-right", { format!("{start_weight:.dp$}") } }
                    }
                    for (index, w) in weights.iter().enumerate() {
                        {
                            let drop_index = i32::try_from(index + 1).unwrap_or(i32::MAX);
                            let nominal: f32 =
                                100.0 * (1.0f32 - drop_percentage / 100.0).powi(drop_index);
                            let actual: f32 = if start_weight > 0.0 {
                                100.0 * w / start_weight
                            } else {
                                0.0
                            };
                            rsx! {
                                tr {
                                    td { class: "has-text-right", { format!("{nominal:.1}") } }
                                    td { class: "has-text-right", { format!("{actual:.1}") } }
                                    td { class: "has-text-right", { format!("{w:.dp$}") } }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn parse_drop_percentage(input: &str) -> Result<f32, String> {
    let value: f32 = input
        .replace(',', ".")
        .parse()
        .map_err(|_| "drop must be a decimal".to_string())?;
    if !value.is_finite() || value <= 0.0 || value >= 100.0 {
        return Err("drop must be greater than 0 and less than 100 %".to_string());
    }
    Ok(value)
}

const DROP_SET_INCREMENT_PRESETS: &[f32] = &[0.25, 0.5, 1.0, 1.25, 2.0, 2.5, 3.75, 5.0, 10.0];

#[derive(Clone)]
pub struct DropSetCalculatorState {
    pub visible: bool,
    pub start_weight: f32,
    pub drop_percentage: f32,
    pub increment: f32,
}

impl DropSetCalculatorState {
    #[must_use]
    pub fn new(start_weight: f32, drop_percentage: f32, increment: f32) -> Self {
        Self {
            visible: false,
            start_weight,
            drop_percentage,
            increment,
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn decimal_places(increment: f32) -> usize {
    let hundredths = (increment * 100.0).round() as i64;
    if hundredths % 100 == 0 {
        0
    } else if hundredths % 10 == 0 {
        1
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use assert_approx_eq::assert_approx_eq;

    use super::{
        SCHEDULE_LOOKAHEAD, beep_samples, decimal_places, format_value, nearest_mark,
        parse_drop_percentage, resync, scheduled_beeps,
    };

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

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, day).unwrap()
    }

    #[test]
    fn nearest_mark_picks_closest_pixel() {
        let marks = [(date(1), 0), (date(2), 50), (date(3), 100)];
        assert_eq!(nearest_mark(&marks, 60), Some((date(2), 50)));
        assert_eq!(nearest_mark(&marks, 90), Some((date(3), 100)));
        assert_eq!(nearest_mark(&marks, -20), Some((date(1), 0)));
    }

    #[test]
    fn nearest_mark_on_tie_keeps_earlier() {
        let marks = [(date(1), 0), (date(2), 100)];
        assert_eq!(nearest_mark(&marks, 50), Some((date(1), 0)));
    }

    #[test]
    fn nearest_mark_without_marks_is_none() {
        assert_eq!(nearest_mark(&[], 10), None);
    }

    #[test]
    fn format_value_trims_trailing_zeros() {
        assert_eq!(format_value(82.3), "82.3");
        assert_eq!(format_value(5.0), "5");
        assert_eq!(format_value(0.25), "0.25");
        assert_eq!(format_value(100.0), "100");
    }

    #[test]
    fn format_value_avoids_negative_zero() {
        assert_eq!(format_value(-0.001), "0");
        assert_eq!(format_value(-0.25), "-0.25");
        assert_eq!(format_value(-5.0), "-5");
    }

    #[test]
    fn decimal_places_matches_preset_precision() {
        assert_eq!(decimal_places(1.0), 0);
        assert_eq!(decimal_places(2.0), 0);
        assert_eq!(decimal_places(5.0), 0);
        assert_eq!(decimal_places(10.0), 0);
        assert_eq!(decimal_places(0.5), 1);
        assert_eq!(decimal_places(2.5), 1);
        assert_eq!(decimal_places(0.25), 2);
        assert_eq!(decimal_places(1.25), 2);
        assert_eq!(decimal_places(3.75), 2);
    }

    #[test]
    fn parse_drop_percentage_accepts_valid_values() {
        assert_eq!(parse_drop_percentage("20"), Ok(20.0));
        assert_eq!(parse_drop_percentage("12.5"), Ok(12.5));
        assert_eq!(parse_drop_percentage("0.1"), Ok(0.1));
        assert_eq!(parse_drop_percentage("99.9"), Ok(99.9));
    }

    #[test]
    fn parse_drop_percentage_accepts_comma_decimal() {
        assert_eq!(parse_drop_percentage("12,5"), Ok(12.5));
    }

    #[test]
    fn parse_drop_percentage_rejects_non_numeric() {
        assert!(parse_drop_percentage("").is_err());
        assert!(parse_drop_percentage("abc").is_err());
    }

    #[test]
    fn parse_drop_percentage_rejects_out_of_range() {
        assert!(parse_drop_percentage("0").is_err());
        assert!(parse_drop_percentage("-1").is_err());
        assert!(parse_drop_percentage("100").is_err());
        assert!(parse_drop_percentage("150").is_err());
    }

    #[test]
    fn parse_drop_percentage_rejects_non_finite() {
        assert!(parse_drop_percentage("nan").is_err());
        assert!(parse_drop_percentage("inf").is_err());
        assert!(parse_drop_percentage("-inf").is_err());
    }
}
