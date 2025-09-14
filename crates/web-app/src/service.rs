use std::collections::VecDeque;

use crate::{Settings, SettingsRepository, SettingsService, log};

pub struct Service<R> {
    repository: R,
}

impl<R> Service<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

impl<R: log::Repository> log::Service for Service<R> {
    fn get_log_entries(&self) -> Result<VecDeque<log::Entry>, log::Error> {
        self.repository.read_entries()
    }

    fn add_log_entry(&self, entry: log::Entry) -> Result<(), log::Error> {
        self.repository.write_entry(entry)
    }
}

impl<R: SettingsRepository> SettingsService for Service<R> {
    async fn get_settings(&self) -> Result<Settings, String> {
        self.repository.read_settings().await
    }

    async fn set_settings(&self, settings: Settings) -> Result<(), String> {
        self.repository.write_settings(settings).await
    }
}
