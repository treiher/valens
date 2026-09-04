use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use crate::{
    OngoingTrainingSession, OngoingTrainingSessionRepository, Settings, SettingsRepository, log,
};

/// Repository holding the settings, the ongoing training session and the log in memory.
///
/// Every operation returns an error instead of its result while the repository is marked as
/// failing.
#[derive(Clone, Default)]
pub struct FakeRepository {
    settings: Arc<Mutex<Settings>>,
    ongoing_training_session: Arc<Mutex<Option<OngoingTrainingSession>>>,
    entries: Arc<Mutex<VecDeque<log::Entry>>>,
    failing: Arc<Mutex<bool>>,
}

impl FakeRepository {
    #[must_use]
    pub fn with_settings(self, settings: Settings) -> Self {
        *locked(&self.settings) = settings;
        self
    }

    #[must_use]
    pub fn with_ongoing_training_session(
        self,
        ongoing_training_session: Option<OngoingTrainingSession>,
    ) -> Self {
        *locked(&self.ongoing_training_session) = ongoing_training_session;
        self
    }

    #[must_use]
    pub fn with_entries(self, entries: VecDeque<log::Entry>) -> Self {
        *locked(&self.entries) = entries;
        self
    }

    #[must_use]
    pub fn failing(self) -> Self {
        *locked(&self.failing) = true;
        self
    }

    fn failed<T>(&self) -> Option<Result<T, String>> {
        (*locked(&self.failing)).then(|| Err("storage is unavailable".to_string()))
    }
}

/// The guard of `value`, ignoring a poisoned lock so that a test fake never panics.
fn locked<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(PoisonError::into_inner)
}

impl SettingsRepository for FakeRepository {
    async fn read_settings(&self) -> Result<Settings, String> {
        self.failed().unwrap_or_else(|| Ok(*locked(&self.settings)))
    }

    async fn write_settings(&self, settings: Settings) -> Result<(), String> {
        self.failed().unwrap_or_else(|| {
            *locked(&self.settings) = settings;
            Ok(())
        })
    }
}

impl OngoingTrainingSessionRepository for FakeRepository {
    async fn read_ongoing_training_session(
        &self,
    ) -> Result<Option<OngoingTrainingSession>, String> {
        self.failed()
            .unwrap_or_else(|| Ok(locked(&self.ongoing_training_session).clone()))
    }

    async fn write_ongoing_training_session(
        &self,
        ongoing_training_session: Option<OngoingTrainingSession>,
    ) -> Result<(), String> {
        self.failed().unwrap_or_else(|| {
            *locked(&self.ongoing_training_session) = ongoing_training_session;
            Ok(())
        })
    }
}

impl log::Repository for FakeRepository {
    fn read_entries(&self) -> Result<VecDeque<log::Entry>, log::Error> {
        if *locked(&self.failing) {
            return Err(log::Error::Unknown("storage is unavailable".to_string()));
        }
        Ok(locked(&self.entries).clone())
    }

    fn write_entry(&self, entry: log::Entry) -> Result<(), log::Error> {
        if *locked(&self.failing) {
            return Err(log::Error::Unknown("storage is unavailable".to_string()));
        }
        locked(&self.entries).push_front(entry);
        Ok(())
    }
}
