use std::collections::VecDeque;

use async_trait::async_trait;
use gloo_storage::Storage as GlooStorage;
use valens_web_app::{OngoingTrainingSession, Settings, log};

pub struct UI;

const KEY_SETTINGS: &str = "settings";
const KEY_ONGOING_TRAINING_SESSION: &str = "ongoing training session";

#[async_trait(?Send)]
impl super::UI for UI {
    async fn read_settings(&self) -> Result<Settings, String> {
        match gloo_storage::LocalStorage::get(KEY_SETTINGS) {
            Ok(entries) => Ok(entries),
            Err(err) => match err {
                gloo_storage::errors::StorageError::KeyNotFound(_) => Ok(Settings::default()),
                err => Err(err),
            },
        }
        .map_err(|err| err.to_string())
    }

    async fn write_settings(&self, settings: Settings) -> Result<(), String> {
        gloo_storage::LocalStorage::set(KEY_SETTINGS, settings).map_err(|err| err.to_string())
    }

    async fn read_ongoing_training_session(
        &self,
    ) -> Result<Option<OngoingTrainingSession>, String> {
        match gloo_storage::LocalStorage::get(KEY_ONGOING_TRAINING_SESSION) {
            Ok(entries) => Ok(entries),
            Err(err) => match err {
                gloo_storage::errors::StorageError::KeyNotFound(_) => Ok(None),
                err => Err(err),
            },
        }
        .map_err(|err| err.to_string())
    }

    async fn write_ongoing_training_session(
        &self,
        ongoing_training_session: Option<OngoingTrainingSession>,
    ) -> Result<(), String> {
        gloo_storage::LocalStorage::set(KEY_ONGOING_TRAINING_SESSION, ongoing_training_session)
            .map_err(|err| err.to_string())
    }
}

pub struct Log;

const KEY_LOG: &str = "log";

impl log::Repository for Log {
    fn read_entries(&self) -> Result<VecDeque<log::Entry>, log::Error> {
        match gloo_storage::LocalStorage::get(KEY_LOG) {
            Ok(entries) => Ok(entries),
            Err(err) => match err {
                gloo_storage::errors::StorageError::KeyNotFound(_) => Ok(VecDeque::new()),
                err => Err(err),
            },
        }
        .map_err(|err| log::Error::Unknown(err.to_string()))
    }

    fn write_entry(&self, entry: log::Entry) -> Result<(), log::Error> {
        let mut entries = self.read_entries()?;
        entries.push_front(entry);
        entries.truncate(100);
        gloo_storage::LocalStorage::set(KEY_LOG, entries)
            .map_err(|err| log::Error::Unknown(err.to_string()))
    }
}
