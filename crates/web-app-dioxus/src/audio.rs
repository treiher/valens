//! Timer, stopwatch and metronome, and the audio graph their beeps are played through.

use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
};

use chrono::{DateTime, Duration, Utc};
use dioxus::prelude::*;
use log::warn;
use web_sys::{
    self,
    wasm_bindgen::{JsCast, closure::Closure},
};

use valens_web_app as web_app;

use valens_domain as domain;

use crate::{
    tempo::{phase_fields, playable_tempo, update_phase_field, validate_tempo},
    ui::{
        element::{Icon, PhaseBar},
        form::PhaseFields,
    },
    wake_lock::{self, WakeLock},
};

/// The phases of the metronome, the segmented bar showing the running one and the play button.
#[component]
pub fn Metronome(metronome: Store<MetronomeService>) -> Element {
    let mut phases = use_signal(|| phase_fields(&metronome.peek().tempo()));
    let is_playable = playable_tempo(&phases.read()).is_some();

    rsx! {
        PhaseFields {
            label: "Tempo",
            unit: "s",
            phases: phases.read().iter().map(|field| field.input.clone()).collect::<Vec<_>>(),
            field_errors: phases.read().iter().map(|field| field.validated.clone().err()).collect::<Vec<_>>(),
            error: validate_tempo(&phases.read()).err(),
            has_changed: false,
            field_testid: "metronome-tempo",
            on_input: move |(index, value): (usize, String)| {
                update_phase_field(&mut phases.write()[index], &value);
                // Playing the previous tempo would contradict the fields it is taken from.
                match playable_tempo(&phases.read()) {
                    Some(tempo) => metronome.write().set_tempo(tempo),
                    None => metronome.write().pause(),
                }
            },
        }
        MetronomeControls { metronome, is_playable }
    }
}

/// The part of the metronome that moves while it plays.
#[component]
fn MetronomeControls(metronome: Store<MetronomeService>, is_playable: bool) -> Element {
    let is_active = metronome.read().is_active();
    let _wake_lock = use_hook(|| Rc::new(RefCell::new(None::<Rc<WakeLock>>)));
    *_wake_lock.borrow_mut() = is_active.then(wake_lock::hold);

    rsx! {
        PhaseBar {
            class: "mb-2",
            phases: metronome.read().tempo().phases().iter().copied().map(u32::from).collect::<Vec<_>>(),
            position: metronome.read().position().map(|(_, index, remaining)| (index, remaining)),
        }
        div {
            class: "field is-grouped is-grouped-centered",
            div { class: "control",
                button {
                    class: "button",
                    r#type: "button",
                    disabled: !is_playable,
                    "data-testid": "metronome-play",
                    onclick: move |_| metronome.write().start_pause(),
                    if is_active {
                        Icon { name: "pause" }
                    } else {
                        Icon { name: "play" }
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
pub const METRONOME_START_DELAY: f64 = 0.5;

/// Frequency of the beat opening each phase of a repetition, descending in roughly equal steps.
///
/// The pitch says which phase is running, and the highest one marks the opening of a repetition.
const PHASE_BEEP_FREQUENCIES: [f32; domain::Tempo::MAX_PHASES] = [1000., 800., 630., 500.];

#[derive(Store, Clone)]
pub struct MetronomeService {
    tempo: domain::Tempo,
    /// Audio clock time the tempo started at, and the wall clock moment it was derived from.
    anchor: Option<f64>,
    start_time: Option<DateTime<Utc>>,
    /// The next beat that has not been handed to the audio graph yet.
    next_beat: u32,
    /// The seconds after which the tempo ends, if it is not open-ended.
    duration: Option<f64>,
    is_active: bool,
    beep_volume: u8,
    // Shared between clones so that a stale clone cannot cancel the beeps of the live instance.
    pending: Rc<RefCell<PendingBeeps>>,
}

impl MetronomeService {
    pub fn new() -> Self {
        Self {
            tempo: domain::Tempo::new(&[1]).unwrap(),
            anchor: None,
            start_time: None,
            next_beat: 0,
            duration: None,
            is_active: false,
            beep_volume: 100,
            pending: Rc::default(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn tempo(&self) -> domain::Tempo {
        self.tempo
    }

    /// Starts the tempo `elapsed` seconds into its first repetition, ending after `duration`
    /// seconds.
    ///
    /// A negative `elapsed` places the start that far in the future. Without a duration the tempo
    /// runs until it is paused.
    pub fn start(&mut self, elapsed: f64, duration: Option<f64>) {
        resume_audio_context();
        // Starting a tempo that already runs where it is asked to would hand its beats to the
        // audio graph a second time, which doubles their amplitude.
        if self.is_active
            && self.duration == duration
            && audio_context_time().is_some_and(|now| !needs_reanchor(self.anchor, now - elapsed))
        {
            return;
        }
        self.is_active = true;
        self.duration = duration;
        self.anchor_at(elapsed);
        // Waiting for the next tick would drop a beat that is due at once.
        self.update();
    }

    pub fn pause(&mut self) {
        self.is_active = false;
        self.cancel_pending();
    }

    pub fn start_pause(&mut self) {
        if self.is_active() {
            self.pause();
        } else {
            self.start(-METRONOME_START_DELAY, None);
        }
    }

    pub fn set_tempo(&mut self, tempo: domain::Tempo) {
        if self.tempo == tempo {
            return;
        }
        self.tempo = tempo;
        if self.is_active {
            self.anchor_at(0.);
            self.update();
        }
    }

    pub fn set_beep_volume(&mut self, beep_volume: u8) {
        self.beep_volume = beep_volume;
    }

    /// The running repetition, the index of the phase within it and the seconds remaining in that
    /// phase, taken from the audio clock.
    pub fn position(&self) -> Option<(u32, usize, f64)> {
        if !self.is_active {
            return None;
        }
        let elapsed = audio_context_time()? - self.anchor?;
        self.tempo.phase_at(elapsed.max(0.))
    }

    pub fn update(&mut self) {
        if !self.is_active || self.tempo.phases().is_empty() {
            return;
        }

        let Some(now) = audio_context_time() else {
            return;
        };

        // The audio clock stops while the context is suspended, so the anchor is taken anew once
        // it and the wall clock disagree.
        let Some(elapsed_since_start) = self.start_time.map(elapsed_seconds) else {
            return;
        };
        if needs_reanchor(self.anchor, now - elapsed_since_start) {
            self.cancel_pending();
            self.anchor = Some(now - elapsed_since_start);
            self.next_beat = self.beat_at_or_after(elapsed_since_start);
        }

        let Some(anchor) = self.anchor else {
            return;
        };
        let elapsed = now - anchor;
        if self
            .tempo
            .beat_at(self.next_beat)
            .is_some_and(|(time, _)| time < elapsed)
        {
            self.next_beat = self.beat_at_or_after(elapsed);
        }

        let until = (elapsed + METRONOME_LOOKAHEAD).min(self.duration.unwrap_or(f64::INFINITY));
        let (beats, next_beat) = scheduled_beats(&self.tempo, self.next_beat, until);
        self.next_beat = next_beat;

        let mut pending = self.pending.borrow_mut();
        pending.forget_played(now);
        for (time, frequency) in beats {
            match play_beep(frequency, anchor + time, 0.05, self.beep_volume) {
                Ok(Some(source)) => pending.push(anchor + time, source),
                Ok(None) => {}
                Err(err) => warn!("failed to play beep: {err:?}"),
            }
        }
    }

    fn cancel_pending(&self) {
        if let Some(now) = audio_context_time() {
            self.pending.borrow_mut().cancel(now);
        }
    }

    fn anchor_at(&mut self, elapsed: f64) {
        self.cancel_pending();
        self.start_time = Some(Utc::now() - Duration::milliseconds(milliseconds(elapsed)));
        self.anchor = audio_context_time().map(|now| now - elapsed);
        // A tempo started by a countdown is anchored only once the countdown has been observed as
        // running, which on a slow device happens later than `BEAT_TOLERANCE`.
        self.next_beat = if elapsed < START_TOLERANCE {
            0
        } else {
            self.beat_at_or_after(elapsed)
        };
    }

    /// The number of the first beat falling at or after `elapsed`.
    fn beat_at_or_after(&self, elapsed: f64) -> u32 {
        let Some((repetition, index, _)) = self.tempo.phase_at(elapsed.max(0.)) else {
            return 0;
        };
        #[allow(clippy::cast_possible_truncation)]
        let mut beat = repetition * self.tempo.phases().len() as u32 + index as u32;
        let due = elapsed - BEAT_TOLERANCE;
        // Beats of a phase of zero seconds share the moment of the phase that runs then.
        while beat > 0
            && self
                .tempo
                .beat_at(beat - 1)
                .is_some_and(|(time, _)| time >= due)
        {
            beat -= 1;
        }
        if self
            .tempo
            .beat_at(beat)
            .is_some_and(|(time, _)| time >= due)
        {
            beat
        } else {
            beat.saturating_add(1)
        }
    }
}

impl Default for MetronomeService {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether the audio clock has drifted away from the wall clock far enough to be anchored anew.
fn needs_reanchor(anchor: Option<f64>, expected_anchor: f64) -> bool {
    !matches!(anchor, Some(anchor) if (anchor - expected_anchor).abs() <= DRIFT_THRESHOLD)
}

/// The beats that sound between `first_beat` and `until` seconds after the start of the tempo,
/// with their frequencies, and the beat that follows them.
///
/// A beat falling at `until` belongs to what comes after the window.
///
/// The frequency follows the phase the beat opens, whichever of the phases before it are skipped.
fn scheduled_beats(tempo: &domain::Tempo, first_beat: u32, until: f64) -> (Vec<(f64, f32)>, u32) {
    #[allow(clippy::cast_possible_truncation)]
    let phases_per_rep = tempo.phases().len() as u32;
    let mut beats = vec![];
    let mut beat = first_beat;
    while let Some((time, sounds)) = tempo.beat_at(beat)
        && time < until
    {
        if sounds {
            beats.push((
                time,
                PHASE_BEEP_FREQUENCIES[(beat % phases_per_rep) as usize],
            ));
        }
        beat = beat.saturating_add(1);
    }
    (beats, beat)
}

fn elapsed_seconds(start_time: DateTime<Utc>) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let seconds = Utc::now()
        .signed_duration_since(start_time)
        .num_milliseconds() as f64
        / 1000.;
    seconds
}

#[allow(clippy::cast_possible_truncation)]
fn milliseconds(seconds: f64) -> i64 {
    (seconds * 1000.) as i64
}

#[component]
pub fn Stopwatch(stopwatch: Signal<StopwatchService>) -> Element {
    rsx! {
        p {
            class: "title is-size-1",
            "data-testid": "stopwatch-time",
            onclick: move |_| stopwatch.write().toggle(),
            "{stopwatch.read().seconds():.1}"
        }
        PlayResetButtons {
            margin_top: 1,
            play_testid: "stopwatch-play",
            reset_testid: "stopwatch-reset",
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
            "data-testid": "countdown",
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
                    "data-testid": "timer-time",
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
            play_testid: "timer-play",
            reset_testid: "timer-reset",
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

/// Tolerance within which a beat that has just passed still counts as due.
///
/// The elapsed time an anchor is placed at is taken a moment after the tempo started, which would
/// otherwise drop the beat opening the repetition.
const BEAT_TOLERANCE: f64 = 0.05;

/// Elapsed time within which an anchored tempo still counts as starting at its first beat.
#[allow(clippy::cast_precision_loss)]
const START_TOLERANCE: f64 = TICK_INTERVAL_MS as f64 / 1000.;

/// Deviation between the audio clock and the wall clock above which the schedule is anchored anew.
///
/// Nothing corrects a smaller deviation, so this is also the accuracy of a beep.
const DRIFT_THRESHOLD: f64 = 0.15;

#[derive(Clone)]
pub struct TimerService {
    reset_seconds: i64,
    remaining_seconds: i64,
    target_time: Option<DateTime<Utc>>,
    /// The exact time left while the countdown is paused, so that holding it neither rounds the
    /// elapsed time nor extends the countdown.
    paused_seconds: Option<f64>,
    /// The tempo beeped alongside the countdown, starting with it.
    tempo: Option<domain::Tempo>,
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
        #[allow(clippy::cast_precision_loss)]
        let remaining = self
            .paused_seconds
            .take()
            .unwrap_or(self.remaining_seconds as f64);
        self.target_time = Some(Utc::now() + Duration::milliseconds(milliseconds(remaining)));
        self.wake_lock = Some(wake_lock::hold());
        self.reschedule(remaining);
    }

    pub fn pause(&mut self) {
        self.paused_seconds = Some(self.remaining_exact());
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

    /// Restores a countdown that was set to `total` seconds and has `remaining` seconds left.
    fn restore(&mut self, remaining: i64, total: i64) {
        self.set(total);
        self.remaining_seconds = remaining;
    }

    pub fn set(&mut self, seconds: i64) {
        resume_audio_context();
        self.reset_seconds = seconds;
        self.remaining_seconds = seconds;
        self.paused_seconds = None;
        if self.target_time.is_some() {
            self.target_time = Some(Utc::now() + Duration::seconds(seconds));
            self.wake_lock = Some(wake_lock::hold());
            #[allow(clippy::cast_precision_loss)]
            self.reschedule(seconds as f64);
        }
    }

    pub fn unset(&mut self) {
        self.reset_seconds = i64::MAX;
        self.target_time = None;
        self.paused_seconds = None;
        self.wake_lock = None;
        self.clear_schedule();
    }

    pub fn reset(&mut self) {
        self.set(self.reset_seconds);
    }

    pub fn tempo(&self) -> Option<domain::Tempo> {
        self.tempo
    }

    pub fn set_tempo(&mut self, tempo: Option<domain::Tempo>) {
        if tempo == self.tempo {
            return;
        }
        self.tempo = tempo;
        // Which cues are silenced follows from the tempo, so the scheduled ones would follow the
        // previous one.
        self.requeue();
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
        schedule.extend(now, self.beep_volume, self.beeped_tempo());
    }

    /// The remaining time in seconds, without the rounding of `seconds`.
    pub fn remaining_exact(&self) -> f64 {
        self.target_time.map_or_else(
            || {
                self.paused_seconds.unwrap_or_else(|| {
                    #[allow(clippy::cast_precision_loss)]
                    let seconds = self.remaining_seconds as f64;
                    seconds
                })
            },
            |target_time| {
                #[allow(clippy::cast_precision_loss)]
                let seconds = target_time
                    .signed_duration_since(Utc::now())
                    .num_milliseconds() as f64
                    / 1000.;
                seconds
            },
        )
    }

    /// The seconds elapsed since the countdown was set, held while it is paused.
    pub fn elapsed_exact(&self) -> f64 {
        if !self.is_set() {
            return 0.;
        }
        #[allow(clippy::cast_precision_loss)]
        let total = self.reset_seconds as f64;
        (total - self.remaining_exact()).max(0.)
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

    fn reschedule(&self, remaining_seconds: f64) {
        let Some(now) = audio_context_time() else {
            return;
        };
        let mut schedule = self.schedule.borrow_mut();
        schedule.anchor(now, remaining_seconds);
        // Anchoring a countdown restored outside a user gesture must happen even though no node can
        // be built yet, so that the first tick with a running context schedules the whole window.
        if audio_context_is_running() {
            schedule.extend(now, self.beep_volume, self.beeped_tempo());
        }
    }

    /// Rebuilds the beeps of the lookahead window without moving the countdown.
    fn requeue(&self) {
        let Some(now) = audio_context_time() else {
            return;
        };
        self.schedule
            .borrow_mut()
            .requeue(now, self.beep_volume, self.beeped_tempo());
    }

    /// The tempo beeped alongside the countdown and the seconds it runs for.
    fn beeped_tempo(&self) -> Option<(domain::Tempo, f64)> {
        #[allow(clippy::cast_precision_loss)]
        self.tempo.map(|tempo| (tempo, self.reset_seconds as f64))
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
            paused_seconds: None,
            tempo: None,
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
            web_app::TimerState::Active { target_time, total } => {
                let remaining = (target_time - Utc::now()).num_seconds();
                timer.restore(remaining, total.unwrap_or(remaining));
                timer.start();
            }
            web_app::TimerState::Paused { time, total } => {
                timer.restore(time, total.unwrap_or(time));
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
                total: Some(value.reset_seconds),
            }
        } else if value.is_set() {
            web_app::TimerState::Paused {
                time: value.remaining_seconds,
                total: Some(value.reset_seconds),
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
    pending: PendingBeeps,
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
    fn extend(&mut self, now: f64, volume: u8, tempo: Option<(domain::Tempo, f64)>) {
        let Some(expiry) = self.expiry else {
            return;
        };
        let from = self.scheduled_until.max(now);
        let to = now + SCHEDULE_LOOKAHEAD;
        if to <= from {
            return;
        }
        for beep in scheduled_beeps(expiry, from, to, tempo) {
            match play_beep(beep.frequency, beep.start, beep.length, volume) {
                Ok(Some(source)) => self.pending.push(beep.start, source),
                Ok(None) => {}
                Err(err) => warn!("failed to play beep: {err:?}"),
            }
        }
        self.scheduled_until = to;
    }

    fn requeue(&mut self, now: f64, volume: u8, tempo: Option<(domain::Tempo, f64)>) {
        self.cancel_pending(now);
        self.scheduled_until = now;
        self.extend(now, volume, tempo);
    }

    fn cancel_pending(&mut self, now: f64) {
        // The beeps of a countdown that has expired are kept, so that the final beep sounds even
        // when the countdown is replaced at that moment.
        if matches!(self.expiry, Some(expiry) if now >= expiry - DRIFT_THRESHOLD) {
            self.pending.forget();
        } else {
            self.pending.cancel(now);
        }
    }
}

/// Beeps handed to the audio graph that are not over yet.
#[derive(Default)]
struct PendingBeeps(Vec<(f64, web_sys::AudioBufferSourceNode)>);

impl PendingBeeps {
    fn push(&mut self, start: f64, source: web_sys::AudioBufferSourceNode) {
        self.0.push((start, source));
    }

    /// Stops the beeps that have not started yet and forgets all of them.
    ///
    /// A beep already sounding is left to finish, since stopping it would cut it mid-envelope.
    fn cancel(&mut self, now: f64) {
        for (start, source) in self.0.drain(..) {
            if start > now
                && let Err(err) = scheduled(&source).stop()
            {
                warn!("failed to stop beep: {err:?}");
            }
        }
    }

    fn forget(&mut self) {
        self.0.clear();
    }

    /// Forgets the beeps that have started, which can no longer be stopped.
    fn forget_played(&mut self, now: f64) {
        self.0.retain(|(start, _)| *start > now);
    }
}

impl Drop for PendingBeeps {
    fn drop(&mut self) {
        if let Some(now) = audio_context_time() {
            self.cancel(now);
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
/// terms, without the cues colliding with `tempo`, which runs for the given seconds and starts
/// with the countdown.
///
/// The exclusive lower bound is what lets consecutive windows partition the beeps of a countdown.
/// A cue is scheduled as a whole, so that none of its beeps sounds on its own, and is dropped as a
/// whole for the same reason.
fn scheduled_beeps(
    expiry: f64,
    from: f64,
    to: f64,
    tempo: Option<(domain::Tempo, f64)>,
) -> Vec<ScheduledBeep> {
    beeps()
        .into_iter()
        .filter(|beep| {
            let cue = expiry - beep.remaining;
            cue > from + SCHEDULE_TOLERANCE && cue <= to + SCHEDULE_TOLERANCE
        })
        .filter(|beep| !collides_with_tempo(beep.remaining, tempo))
        .map(|beep| ScheduledBeep {
            start: expiry - beep.remaining + beep.offset,
            frequency: beep.frequency,
            length: beep.length,
        })
        .collect()
}

/// Distance within which a cue of a countdown and a beat of the tempo beeped alongside it collide.
///
/// A cue sounding on a beat masks which phase that beat opens, and one sounding next to a beat is
/// taken for a beat itself, so the distance covers the tones and a gap wide enough to tell them
/// apart.
const CUE_COLLISION_TOLERANCE: f64 = 0.25;

/// Whether a cue starting `remaining` seconds before the end of the countdown falls on a beat of
/// the tempo beeped alongside it.
///
/// The beat opening the repetition that would follow the tempo does not sound, so the cue at the
/// end of the countdown is left alone.
fn collides_with_tempo(remaining: f64, tempo: Option<(domain::Tempo, f64)>) -> bool {
    let Some((tempo, seconds)) = tempo else {
        return false;
    };
    tempo
        .beat_near(seconds - remaining, CUE_COLLISION_TOLERANCE)
        .is_some_and(|beat| beat < seconds)
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
            .get_or_init(|| {
                // Off wasm the constructor panics rather than returning an error
                if !cfg!(target_arch = "wasm32") {
                    return None;
                }
                match web_sys::AudioContext::new() {
                    Ok(audio_context) => {
                        listen_for_user_gestures();
                        Some(audio_context)
                    }
                    Err(err) => {
                        warn!("failed to create audio context: {err:?}");
                        None
                    }
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
    play_testid: String,
    reset_testid: String,
    on_start_pause: EventHandler<MouseEvent>,
    on_reset: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            class: "button mt-{margin_top} mx-3",
            r#type: "button",
            "data-testid": "{play_testid}",
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
            "data-testid": "{reset_testid}",
            onclick: on_reset,
            Icon { name: "rotate-left" }
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use valens_domain as domain;

    use super::{
        DRIFT_THRESHOLD, MetronomeService, PHASE_BEEP_FREQUENCIES, SCHEDULE_LOOKAHEAD,
        TimerService, beep_samples, needs_reanchor, scheduled_beats, scheduled_beeps, web_app,
    };

    #[rstest]
    fn test_timer_holds_its_exact_remaining_time_while_paused() {
        let mut timer = TimerService::new(10);

        timer.start();
        std::thread::sleep(std::time::Duration::from_millis(200));
        timer.pause();
        let paused = timer.remaining_exact();
        timer.start();

        assert!(paused < 10., "{paused}");
        assert!((timer.remaining_exact() - paused).abs() < 0.05, "{paused}");
    }

    #[rstest]
    #[case::active(true)]
    #[case::paused(false)]
    fn test_a_restored_timer_keeps_the_time_it_was_set_to(#[case] active: bool) {
        let mut timer = TimerService::default();
        timer.restore(20, 60);
        if active {
            timer.start();
        }

        let timer = TimerService::from(web_app::TimerState::from(timer));

        // A running countdown is restored from its target time, truncated to whole seconds.
        assert_approx_eq!(timer.elapsed_exact(), 40., 1.05);
    }

    fn metronome(phases: &[u32]) -> MetronomeService {
        let mut metronome = MetronomeService::new();
        metronome.set_tempo(domain::Tempo::new(phases).unwrap());
        metronome
    }

    #[rstest]
    #[case::start(&[3, 1, 1, 0], 0., 0)]
    #[case::within_the_first_phase(&[3, 1, 1, 0], 0.5, 1)]
    #[case::at_a_beat(&[3, 1, 1, 0], 3., 1)]
    #[case::after_a_stall(&[3, 1, 1, 0], 12.5, 9)]
    #[case::single_phase(&[1], 4.5, 5)]
    #[case::first_phase_of_zero(&[0, 2], 0., 0)]
    #[case::just_after_a_beat(&[3, 1, 1, 0], 0.01, 0)]
    fn test_beat_at_or_after(#[case] phases: &[u32], #[case] elapsed: f64, #[case] expected: u32) {
        assert_eq!(metronome(phases).beat_at_or_after(elapsed), expected);
    }

    #[rstest]
    #[case::delayed_start(0.08, 0)]
    #[case::within_the_first_phase(0.5, 1)]
    fn test_anchor_at_keeps_the_opening_beat_of_a_delayed_start(
        #[case] elapsed: f64,
        #[case] expected: u32,
    ) {
        let mut metronome = metronome(&[3, 1, 1, 0]);

        metronome.anchor_at(elapsed);

        assert_eq!(metronome.next_beat, expected);
    }

    #[rstest]
    #[case::unanchored(None, 10., true)]
    #[case::in_step(Some(10.), 10. + DRIFT_THRESHOLD / 2., false)]
    #[case::drifted(Some(10.), 10. + DRIFT_THRESHOLD * 2., true)]
    fn test_needs_reanchor(
        #[case] anchor: Option<f64>,
        #[case] expected_anchor: f64,
        #[case] expected: bool,
    ) {
        assert_eq!(needs_reanchor(anchor, expected_anchor), expected);
    }

    #[rstest]
    fn test_scheduled_beats_take_the_pitch_of_the_phase_they_open() {
        let tempo = domain::Tempo::new(&[3, 1]).unwrap();

        assert_eq!(
            scheduled_beats(&tempo, 0, 8.),
            (
                vec![
                    (0., PHASE_BEEP_FREQUENCIES[0]),
                    (3., PHASE_BEEP_FREQUENCIES[1]),
                    (4., PHASE_BEEP_FREQUENCIES[0]),
                    (7., PHASE_BEEP_FREQUENCIES[1]),
                ],
                4
            )
        );
    }

    #[rstest]
    fn test_scheduled_beats_of_a_single_phase_share_one_pitch() {
        let tempo = domain::Tempo::new(&[1]).unwrap();

        assert_eq!(
            scheduled_beats(&tempo, 0, 3.),
            (
                vec![
                    (0., PHASE_BEEP_FREQUENCIES[0]),
                    (1., PHASE_BEEP_FREQUENCIES[0]),
                    (2., PHASE_BEEP_FREQUENCIES[0]),
                ],
                3
            )
        );
    }

    #[rstest]
    fn test_scheduled_beats_of_four_phases_descend_within_a_repetition() {
        let tempo = domain::Tempo::new(&[3, 1, 1, 1]).unwrap();

        assert_eq!(
            scheduled_beats(&tempo, 0, 6.),
            (
                vec![
                    (0., PHASE_BEEP_FREQUENCIES[0]),
                    (3., PHASE_BEEP_FREQUENCIES[1]),
                    (4., PHASE_BEEP_FREQUENCIES[2]),
                    (5., PHASE_BEEP_FREQUENCIES[3]),
                ],
                4
            )
        );
    }

    #[rstest]
    fn test_scheduled_beats_of_a_repetition_opening_on_a_zero_phase() {
        let tempo = domain::Tempo::new(&[0, 2]).unwrap();

        assert_eq!(
            scheduled_beats(&tempo, 0, 4.),
            (
                vec![
                    (0., PHASE_BEEP_FREQUENCIES[0]),
                    (2., PHASE_BEEP_FREQUENCIES[0])
                ],
                4
            )
        );
    }

    #[rstest]
    fn test_scheduled_beats_end_before_the_beat_bounding_them() {
        let tempo = domain::Tempo::new(&[2, 2]).unwrap();

        assert_eq!(
            scheduled_beats(&tempo, 0, 4.),
            (
                vec![
                    (0., PHASE_BEEP_FREQUENCIES[0]),
                    (2., PHASE_BEEP_FREQUENCIES[1]),
                ],
                2
            )
        );
    }

    #[rstest]
    fn test_scheduled_beats_after_a_stall_skips_the_missed_beats() {
        let tempo = domain::Tempo::new(&[1]).unwrap();
        let metronome = metronome(&[1]);

        assert_eq!(
            scheduled_beats(&tempo, metronome.beat_at_or_after(10.5), 12.),
            (vec![(11., PHASE_BEEP_FREQUENCIES[0])], 12)
        );
    }

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
        scheduled_beeps(expiry, from, to, None)
            .into_iter()
            .map(|beep| beep.start)
            .collect()
    }

    fn beep_starts_with_tempo(phases: &[u32], seconds: f64) -> Vec<f64> {
        let tempo = domain::Tempo::new(phases).unwrap();
        scheduled_beeps(100., 0., 100., Some((tempo, seconds)))
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
    fn scheduled_beeps_drops_a_cue_falling_on_a_beat() {
        let starts = beep_starts_with_tempo(&[1], 12.);
        assert_eq!(starts.len(), 1);
        assert_approx_eq!(starts[0], 100., 1e-9);
    }

    #[test]
    fn scheduled_beeps_keeps_a_cue_between_two_beats() {
        let starts = beep_starts_with_tempo(&[3], 12.);
        assert_eq!(starts.len(), 5);
        for (start, expected) in starts.iter().zip([90., 90.18, 98., 99., 100.]) {
            assert_approx_eq!(start, expected, 1e-9);
        }
    }

    #[test]
    fn scheduled_beeps_keeps_the_cue_at_the_end_of_the_tempo() {
        let starts = beep_starts_with_tempo(&[2], 10.);
        assert_approx_eq!(starts[starts.len() - 1], 100., 1e-9);
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
}
