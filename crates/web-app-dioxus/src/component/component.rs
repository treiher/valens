use chrono::{DateTime, Duration, Utc};
use dioxus::prelude::*;
use futures_util::StreamExt;
use gloo_timers::future::IntervalStream;
use log::error;
use valens_web_app as web_app;
use web_sys;

#[component]
pub fn Timer(timer_service: Store<TimerService>) -> Element {
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        let mut interval = IntervalStream::new(1000);
        loop {
            interval.next().await;
            timer_service.write().update();
        }
    });

    rsx! {
        div {
            class: if !timer_service.read().is_active() { "is-blinking" },
            onclick: move |_| {
                timer_service.write().start_pause();
            },
            "{timer_service.read().seconds()} s"
        }
    }
}

#[derive(Clone, Default)]
pub struct TimerService {
    reset_seconds: i64,
    remaining_seconds: i64,
    target_time: Option<DateTime<Utc>>,
    audio_context: Option<web_sys::AudioContext>,
    beep_time: f64,
    beep_volume: u8,
}

impl TimerService {
    pub fn new() -> Self {
        Self {
            reset_seconds: i64::MAX,
            remaining_seconds: i64::MAX,
            target_time: None,
            audio_context: match web_sys::AudioContext::new() {
                Ok(audio_context) => Some(audio_context),
                Err(err) => {
                    error!("failed to create audio context: {err:?}");
                    None
                }
            },
            beep_time: 0.,
            beep_volume: 100,
        }
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
                        error!("failed to play beep: {err:?}");
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
                        error!("failed to play beep: {err:?}");
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
                        error!("failed to play beep: {err:?}");
                    }
                }
            }
            self.remaining_seconds = remaining_seconds;
        }
    }

    fn restore(&mut self, timer_state: web_app::TimerState) {
        match timer_state {
            web_app::TimerState::Unset => {
                self.unset();
            }
            web_app::TimerState::Active { target_time } => {
                self.set((target_time - Utc::now()).num_seconds());
                self.start();
            }
            web_app::TimerState::Paused { time } => {
                self.set(time);
                self.pause();
            }
        }
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
