//! Shared, domain-aware UI components used across multiple pages.

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Utc, Weekday};
use dioxus::prelude::*;
use futures_util::StreamExt;
use gloo_timers::future::IntervalStream;
use log::{error, warn};
use web_sys;

use valens_domain::{self as domain, Property};
use valens_web_app as web_app;

use crate::{
    METRONOME,
    ui::{
        element::{Error, Icon, NoData, TagsWithAddon},
        form::{SelectField, SelectOption},
    },
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
                    onchange: move |event: FormEvent| {
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
                    onchange: move |event: FormEvent| {
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

#[derive(Clone)]
pub struct MetronomeService {
    interval: u32,
    stressed_beat: u32,
    beat_number: u32,
    next_beat_time: f64,
    is_active: bool,
    audio_context: Option<web_sys::AudioContext>,
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
            audio_context: match web_sys::AudioContext::new() {
                Ok(audio_context) => Some(audio_context),
                Err(err) => {
                    warn!("failed to create audio context: {err:?}");
                    None
                }
            },
            beep_volume: 100,
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn start(&mut self) {
        resume_audio_context(self.audio_context.as_ref());
        self.is_active = true;
        if let Some(audio_context) = &self.audio_context {
            self.beat_number = 0;
            self.next_beat_time = audio_context.current_time() + 0.5;
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
        if !self.is_active() {
            return;
        }

        if let Some(audio_context) = &self.audio_context {
            while self.next_beat_time < audio_context.current_time() + 0.5 {
                if let Err(err) = play_beep(
                    audio_context,
                    if self.beat_number % self.stressed_beat == 0 {
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
}

#[component]
pub fn Stopwatch(stopwatch: Signal<StopwatchService>) -> Element {
    rsx! {
        p {
            class: "title is-size-1",
            onclick: move |_| stopwatch.write().toggle(),
            "{stopwatch.read().seconds():.1}"
        }
        button {
            class: "button mt-1 mx-3",
            r#type: "button",
            onclick: move |_| stopwatch.write().start_pause(),
            if stopwatch.read().is_active() {
                Icon { name: "pause" }
            } else {
                Icon { name: "play" }
            }
        }
        button {
            class: "button mt-1 mx-3",
            r#type: "button",
            onclick: move |_| stopwatch.write().reset(),
            Icon { name: "rotate-left" }
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
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        let mut interval = IntervalStream::new(1000);
        loop {
            interval.next().await;
            timer.write().update();
        }
    });

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
        button {
            class: "button mt-5 mx-3",
            r#type: "button",
            onclick: move |_| timer.write().start_pause(),
            if timer.read().is_active() {
                Icon { name: "pause" }
            } else {
                Icon { name: "play" }
            }
        }
        button {
            class: "button mt-5 mx-3",
            r#type: "button",
            onclick: move |_| timer.write().reset(),
            Icon { name: "rotate-left" }
        }
    }
}

#[derive(Clone)]
pub struct TimerService {
    reset_seconds: i64,
    remaining_seconds: i64,
    target_time: Option<DateTime<Utc>>,
    audio_context: Option<web_sys::AudioContext>,
    beep_time: f64,
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
        resume_audio_context(self.audio_context.as_ref());
        self.target_time = Some(Utc::now() + Duration::seconds(self.remaining_seconds));
    }

    pub fn pause(&mut self) {
        self.target_time = None;
    }

    pub fn start_pause(&mut self) {
        if self.is_active() {
            self.pause();
        } else {
            self.start();
        }
    }

    pub fn set(&mut self, seconds: i64) {
        resume_audio_context(self.audio_context.as_ref());
        self.reset_seconds = seconds;
        self.remaining_seconds = seconds;
        if self.target_time.is_some() {
            self.target_time = Some(Utc::now() + Duration::seconds(seconds));
        }
    }

    pub fn unset(&mut self) {
        self.reset_seconds = i64::MAX;
        self.target_time = None;
        self.beep_time = 0.;
    }

    pub fn reset(&mut self) {
        self.set(self.reset_seconds);
    }

    pub fn set_beep_volume(&mut self, beep_volume: u8) {
        self.beep_volume = beep_volume;
    }

    pub fn update(&mut self) {
        if let Some(target_time) = self.target_time {
            #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
            let remaining_seconds = (target_time
                .signed_duration_since(Utc::now())
                .num_milliseconds() as f64
                / 1000.)
                .round() as i64;
            if let Some(audio_context) = &self.audio_context {
                // Only schedule beeps once per second
                if self.beep_time < audio_context.current_time() - 0.98 {
                    if remaining_seconds == 10 {
                        if let Err(err) = play_beep(
                            audio_context,
                            2000.,
                            {
                                self.beep_time = audio_context.current_time() + 0.01;
                                self.beep_time
                            },
                            0.1,
                            self.beep_volume,
                        ) {
                            warn!("failed to play beep: {err:?}");
                        }
                        if let Err(err) = play_beep(
                            audio_context,
                            2000.,
                            {
                                self.beep_time = audio_context.current_time() + 0.18;
                                self.beep_time
                            },
                            0.1,
                            self.beep_volume,
                        ) {
                            warn!("failed to play beep: {err:?}");
                        }
                    }
                    if (0..=2).contains(&remaining_seconds) {
                        if let Err(err) = play_beep(
                            audio_context,
                            2000.,
                            if remaining_seconds == 2 {
                                self.beep_time = audio_context.current_time() + 0.01;
                                self.beep_time
                            } else {
                                self.beep_time += 1.;
                                self.beep_time
                            },
                            if remaining_seconds == 0 { 0.5 } else { 0.15 },
                            self.beep_volume,
                        ) {
                            warn!("failed to play beep: {err:?}");
                        }
                    }
                }
            }
            self.remaining_seconds = remaining_seconds;
        }
    }
}

impl Default for TimerService {
    fn default() -> Self {
        Self {
            reset_seconds: i64::MAX,
            remaining_seconds: i64::MAX,
            target_time: None,
            audio_context: match web_sys::AudioContext::new() {
                Ok(audio_context) => Some(audio_context),
                Err(err) => {
                    warn!("failed to create audio context: {err:?}");
                    None
                }
            },
            beep_time: 0.,
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
        };
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

fn play_beep(
    audio_context: &web_sys::AudioContext,
    frequency: f32,
    start: f64,
    length: f64,
    volume: u8,
) -> Result<(), web_sys::wasm_bindgen::JsValue> {
    let oscillator = audio_context.create_oscillator()?;
    let gain = audio_context.create_gain()?;
    gain.gain().set_value(f32::from(volume) / 100.);
    gain.connect_with_audio_node(&audio_context.destination())?;
    oscillator.connect_with_audio_node(&gain)?;
    oscillator.frequency().set_value(frequency);
    oscillator.start_with_when(start)?;
    oscillator.stop_with_when(start + length)?;
    Ok(())
}

fn resume_audio_context(audio_context: Option<&web_sys::AudioContext>) {
    if let Some(audio_context) = &audio_context {
        if let Err(err) = audio_context.resume() {
            warn!("failed to resume audio context: {err:?}");
        }
    }
}

#[component]
pub fn IntervalControl(
    current_interval: Signal<domain::Interval>,
    all: domain::Interval,
) -> Element {
    let current = current_interval.read();
    let today = Local::now().date_naive();
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
    labels: Vec<ChartLabel>,
    chart: Result<Option<String>, String>,
    no_data_label: bool,
) -> Element {
    match chart {
        Ok(result) => match result {
            None => {
                if no_data_label {
                    rsx! {
                        NoData {}
                    }
                } else {
                    rsx! {}
                }
            }
            Some(value) => rsx! {
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
                        dangerous_inner_html: value,
                    }
                }
            },
        },
        Err(err) => rsx! { Error { message: err } },
    }
}

#[derive(Clone, PartialEq)]
pub struct ChartLabel {
    pub name: String,
    pub color: usize,
    pub opacity: f64,
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
                                    },
                                    style: if date < interval.first || date > interval.last {
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
    stimulus_per_muscle.sort_by(|a, b| b.1.cmp(&a.1));
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
