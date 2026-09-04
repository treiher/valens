use std::collections::VecDeque;

use gloo_storage::Storage as GlooStorage;
use valens_web_app::{
    OngoingTrainingSession, OngoingTrainingSessionRepository, Settings, SettingsRepository, log,
};

#[derive(Clone)]
pub struct LocalStorage;

const KEY_SETTINGS: &str = "settings";
const KEY_ONGOING_TRAINING_SESSION: &str = "ongoing training session";
const KEY_LOG: &str = "log";

impl SettingsRepository for LocalStorage {
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
}

impl OngoingTrainingSessionRepository for LocalStorage {
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

impl log::Repository for LocalStorage {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    mod wasm {
        use chrono::{DateTime, Utc};
        use pretty_assertions::assert_eq;
        use valens_web_app::{Theme, TimerState, log::Repository as _};
        use wasm_bindgen_test::wasm_bindgen_test;

        use super::*;

        #[wasm_bindgen_test]
        async fn test_read_settings_without_stored_value() {
            reset();

            assert_eq!(
                LocalStorage.read_settings().await.unwrap(),
                Settings::default()
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_ongoing_training_session_without_stored_value() {
            reset();

            assert_eq!(
                LocalStorage.read_ongoing_training_session().await.unwrap(),
                None
            );
        }

        #[wasm_bindgen_test]
        fn test_read_entries_without_stored_value() {
            reset();

            assert_eq!(LocalStorage.read_entries().unwrap(), VecDeque::new());
        }

        #[wasm_bindgen_test]
        async fn test_read_settings_with_corrupt_stored_value() {
            reset();
            store_corrupt_value(KEY_SETTINGS);

            assert!(LocalStorage.read_settings().await.is_err());
        }

        #[wasm_bindgen_test]
        async fn test_read_ongoing_training_session_with_corrupt_stored_value() {
            reset();
            store_corrupt_value(KEY_ONGOING_TRAINING_SESSION);

            assert!(LocalStorage.read_ongoing_training_session().await.is_err());
        }

        #[wasm_bindgen_test]
        fn test_read_entries_with_corrupt_stored_value() {
            reset();
            store_corrupt_value(KEY_LOG);

            assert!(LocalStorage.read_entries().is_err());
        }

        #[wasm_bindgen_test]
        async fn test_write_and_read_settings() {
            reset();
            let settings = Settings {
                beep_volume: 42,
                theme: Theme::Dark,
                automatic_metronome: true,
                notifications: true,
                show_rpe: false,
                show_tut: false,
                scroll_snapping: true,
            };

            LocalStorage.write_settings(settings).await.unwrap();

            assert_eq!(LocalStorage.read_settings().await.unwrap(), settings);
        }

        #[wasm_bindgen_test]
        async fn test_write_and_read_ongoing_training_session() {
            reset();
            let ongoing_training_session = ongoing_training_session();

            LocalStorage
                .write_ongoing_training_session(Some(ongoing_training_session.clone()))
                .await
                .unwrap();

            assert_eq!(
                LocalStorage.read_ongoing_training_session().await.unwrap(),
                Some(ongoing_training_session)
            );
        }

        #[wasm_bindgen_test]
        fn test_write_and_read_entry() {
            reset();

            LocalStorage
                .write_entry(entry(::log::Level::Warn, "1"))
                .unwrap();

            assert_eq!(
                LocalStorage.read_entries().unwrap(),
                VecDeque::from([entry(::log::Level::Warn, "1")])
            );
        }

        #[wasm_bindgen_test]
        fn test_write_entry_returns_newest_first() {
            reset();

            LocalStorage
                .write_entry(entry(::log::Level::Info, "1"))
                .unwrap();
            LocalStorage
                .write_entry(entry(::log::Level::Info, "2"))
                .unwrap();

            assert_eq!(
                LocalStorage.read_entries().unwrap(),
                VecDeque::from([
                    entry(::log::Level::Info, "2"),
                    entry(::log::Level::Info, "1")
                ])
            );
        }

        #[wasm_bindgen_test]
        fn test_write_entry_drops_oldest_beyond_capacity() {
            reset();

            for i in 0..=100 {
                LocalStorage
                    .write_entry(entry(::log::Level::Info, &i.to_string()))
                    .unwrap();
            }

            let entries = LocalStorage.read_entries().unwrap();

            assert_eq!(entries.len(), 100);
            assert_eq!(entries.front(), Some(&entry(::log::Level::Info, "100")));
            assert_eq!(entries.back(), Some(&entry(::log::Level::Info, "1")));
        }

        #[wasm_bindgen_test]
        async fn test_write_ongoing_training_session_none_keeps_key() {
            reset();
            LocalStorage
                .write_ongoing_training_session(Some(ongoing_training_session()))
                .await
                .unwrap();

            LocalStorage
                .write_ongoing_training_session(None)
                .await
                .unwrap();

            assert_eq!(
                gloo_storage::LocalStorage::get::<Option<OngoingTrainingSession>>(
                    KEY_ONGOING_TRAINING_SESSION
                )
                .unwrap(),
                None
            );
            assert_eq!(
                LocalStorage.read_ongoing_training_session().await.unwrap(),
                None
            );
        }

        fn reset() {
            for key in [KEY_SETTINGS, KEY_ONGOING_TRAINING_SESSION, KEY_LOG] {
                gloo_storage::LocalStorage::delete(key);
            }
        }

        fn store_corrupt_value(key: &str) {
            gloo_storage::LocalStorage::raw()
                .set_item(key, "not json")
                .unwrap();
        }

        fn ongoing_training_session() -> OngoingTrainingSession {
            OngoingTrainingSession {
                training_session_id: 1,
                start_time: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
                element_idx: 2,
                element_start_time: DateTime::<Utc>::from_timestamp(1_700_000_060, 0).unwrap(),
                timer_state: TimerState::Paused { time: 30 },
            }
        }

        fn entry(level: ::log::Level, message: &str) -> log::Entry {
            log::Entry {
                time: "2026-01-01 00:00:00".to_string(),
                level,
                message: message.to_string(),
            }
        }
    }
}
