#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

use chrono::{DateTime, Utc};

pub mod chart;
pub mod service_worker;

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
    pub training_session_id: u32,
    pub start_time: DateTime<Utc>,
    pub element_idx: usize,
    pub element_start_time: DateTime<Utc>,
    pub timer_state: TimerState,
}

impl OngoingTrainingSession {
    #[must_use]
    pub fn new(training_session_id: u32) -> Self {
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
