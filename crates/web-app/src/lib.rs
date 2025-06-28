#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

pub mod chart;
pub mod log;
pub mod service_worker;

pub use notification::{close_notifications, request_notification_permission, show_notification};
pub use ongoing_training_session::{
    OngoingTrainingSession, OngoingTrainingSessionRepository, OngoingTrainingSessionService,
    TimerState,
};
pub use service::Service;
pub use settings::{Settings, SettingsRepository, SettingsService, Theme};

mod notification;
mod ongoing_training_session;
mod service;
mod settings;
