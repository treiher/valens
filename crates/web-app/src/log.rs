use std::{
    collections::VecDeque,
    ops::DerefMut,
    sync::{Arc, Mutex},
};

use chrono::Local;
use gloo_console;
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use serde::{Deserialize, Serialize};
use thiserror;

pub static LOG: Mutex<Option<Arc<Mutex<dyn Repository>>>> = Mutex::new(None);

#[allow(clippy::missing_errors_doc)]
pub trait Repository: Send + Sync + 'static {
    fn read_entries(&self) -> Result<VecDeque<Entry>, Error>;
    fn write_entry(&self, entry: Entry) -> Result<(), Error>;
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("{0}")]
    Unknown(String),
}

#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub time: String,
    #[serde(with = "LevelDef")]
    pub level: Level,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Level")]
pub enum LevelDef {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

static LOGGER: Logger = Logger;

/// # Errors
///
/// Returns an error if the logger has already been initialized.
pub fn init(storage: Arc<Mutex<dyn Repository>>) -> Result<(), SetLoggerError> {
    if let Ok(mut log) = LOG.lock() {
        *log = Some(storage);
    }
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Trace))
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            if let Some(ref log) = *LOG.lock().unwrap() {
                let message = record.args().to_string();
                match record.level() {
                    Level::Error => gloo_console::error!(message),
                    Level::Warn => gloo_console::warn!(message),
                    Level::Info => gloo_console::info!(message),
                    Level::Debug | Level::Trace => gloo_console::debug!(message),
                }

                let _ = log.lock().unwrap().deref_mut().write_entry(Entry {
                    time: Local::now().format("%b %d %H:%M:%S").to_string(),
                    level: record.level(),
                    message: record.args().to_string(),
                });
            }
        }
    }

    fn flush(&self) {}
}
