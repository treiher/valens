use async_trait::async_trait;
use gloo_storage::Storage as GlooStorage;

use crate::ui::{OngoingTrainingSession, Settings};

pub struct UI;

const KEY_SETTINGS: &str = "settings";
const KEY_ONGOING_TRAINING_SESSION: &str = "ongoing training session";

#[async_trait(?Send)]
impl super::UI for UI {
    async fn read_settings(&self) -> Result<Settings, String> {
        gloo_storage::LocalStorage::get(KEY_SETTINGS).map_err(|err| err.to_string())
    }

    async fn write_settings(&self, settings: Settings) -> Result<(), String> {
        gloo_storage::LocalStorage::set(KEY_SETTINGS, settings.clone())
            .map_err(|err| err.to_string())
    }

    async fn read_ongoing_training_session(
        &self,
    ) -> Result<Option<OngoingTrainingSession>, String> {
        gloo_storage::LocalStorage::get(KEY_ONGOING_TRAINING_SESSION).map_err(|err| err.to_string())
    }

    async fn write_ongoing_training_session(
        &self,
        ongoing_training_session: Option<OngoingTrainingSession>,
    ) -> Result<(), String> {
        gloo_storage::LocalStorage::set(
            KEY_ONGOING_TRAINING_SESSION,
            ongoing_training_session.clone(),
        )
        .map_err(|err| err.to_string())
    }
}
