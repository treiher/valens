use chrono::{DateTime, Utc};

#[allow(async_fn_in_trait)]
pub trait OngoingTrainingSessionService {
    async fn get_ongoing_training_session(&self) -> Result<Option<OngoingTrainingSession>, String>;
    async fn set_ongoing_training_session(
        &self,
        ongoing_training_session: Option<OngoingTrainingSession>,
    ) -> Result<(), String>;
}

#[allow(async_fn_in_trait)]
pub trait OngoingTrainingSessionRepository {
    async fn read_ongoing_training_session(&self)
    -> Result<Option<OngoingTrainingSession>, String>;
    async fn write_ongoing_training_session(
        &self,
        ongoing_training_session: Option<OngoingTrainingSession>,
    ) -> Result<(), String>;
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
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

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum TimerState {
    Unset,
    Active {
        target_time: DateTime<Utc>,
        /// The time the countdown was set to, absent in sessions stored before it was recorded.
        #[serde(default)]
        total: Option<i64>,
    },
    Paused {
        time: i64,
        #[serde(default)]
        total: Option<i64>,
    },
}
