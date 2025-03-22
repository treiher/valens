#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

use chrono::{DateTime, Utc};

pub mod chart;
pub mod log;
pub mod service_worker;

#[allow(async_fn_in_trait)]
pub trait Repository {
    async fn read_settings(&self) -> Result<Settings, String>;
    async fn write_settings(&self, settings: Settings) -> Result<(), String>;

    async fn read_ongoing_training_session(&self)
    -> Result<Option<OngoingTrainingSession>, String>;
    async fn write_ongoing_training_session(
        &self,
        ongoing_training_session: Option<OngoingTrainingSession>,
    ) -> Result<(), String>;
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub beep_volume: u8,
    pub theme: Theme,
    pub automatic_metronome: bool,
    pub notifications: bool,
    pub show_rpe: bool,
    pub show_tut: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            beep_volume: 80,
            theme: Theme::Light,
            automatic_metronome: false,
            notifications: false,
            show_rpe: true,
            show_tut: true,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct OngoingTrainingSession {
    pub training_session_id: u128,
    pub start_time: DateTime<Utc>,
    pub element_idx: usize,
    pub element_start_time: DateTime<Utc>,
    pub timer_state: TimerState,
}

impl OngoingTrainingSession {
    #[must_use]
    pub fn new(training_session_id: u128) -> Self {
        Self {
            training_session_id,
            start_time: Utc::now(),
            element_idx: 0,
            element_start_time: Utc::now(),
            timer_state: TimerState::Unset,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
pub enum TimerState {
    Unset,
    Active { target_time: DateTime<Utc> },
    Paused { time: i64 },
}
