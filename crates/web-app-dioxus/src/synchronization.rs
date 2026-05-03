//! Background synchronization from the remote backend into the local database.
//!
//! [`Synchronization`] is a Dioxus context that triggers the domain service to pull remote
//! changes from the backend into the local database, and then triggers [`crate::cache::Cache`]
//! refreshes so the UI reflects the latest state.

use dioxus::prelude::*;

use log::warn;

use valens_domain as domain;

use crate::{
    DOMAIN_SERVICE, ERRORS, NO_CONNECTION,
    cache::{Cache, CacheState},
};

macro_rules! sync {
    ($entry: ident, $sync_method: ident, $refresh_method: ident) => {{
        let mut cache = consume_context::<Cache>();
        let mut synchronization = consume_context::<Synchronization>();
        spawn(async move {
            if matches!(&*cache.$entry.peek(), CacheState::Ready(value) if value.is_empty()) {
                cache.$entry.set(CacheState::Loading);
            }
            match DOMAIN_SERVICE().$sync_method().await {
                Err(domain::SyncError::Storage(domain::StorageError::NoConnection)) => {
                    if !NO_CONNECTION() {
                        *NO_CONNECTION.write() = true;
                        ERRORS.write().push("No connection to server".to_string());
                    }
                }
                Err(err) => {
                    if !synchronization.has_error() {
                        warn!("synchronization failed: {err}");
                        let error_message = format!("Synchronization failed: {err}");
                        synchronization.error.set(error_message.clone());
                        ERRORS.write().push(error_message);
                        *NO_CONNECTION.write() = false;
                    }
                }
                Ok(_) => {
                    *NO_CONNECTION.write() = false;
                }
            }
            cache.$refresh_method();
            synchronization.pending_sync_count.with_mut(|count| {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    synchronization.in_progress.set(false);
                }
            });
        });
    }};
}

#[derive(Clone, Copy)]
pub struct Synchronization {
    error: Signal<String>,
    in_progress: Signal<bool>,
    pending_sync_count: Signal<u8>,
}

impl Synchronization {
    pub fn provide() {
        let error = use_signal(String::new);
        let in_progress = use_signal(|| false);
        let pending_sync_count = use_signal(|| 0);
        use_context_provider(move || Self {
            error,
            in_progress,
            pending_sync_count,
        });
    }

    pub fn sync(&mut self) {
        if !*self.in_progress.peek() {
            self.error.set(String::new());
            self.in_progress.set(true);
            self.pending_sync_count.set(6);
            sync!(exercises, sync_exercises, refresh_exercises);
            sync!(routines, sync_routines, refresh_routines);
            sync!(
                training_sessions,
                sync_training_sessions,
                refresh_training_sessions
            );
            sync!(body_weight, sync_body_weight, refresh_body_weight);
            sync!(body_fat, sync_body_fat, refresh_body_fat);
            sync!(period, sync_period, refresh_period);
        }
    }

    pub fn in_progress(&self) -> bool {
        self.in_progress.cloned()
    }

    pub fn has_error(&self) -> bool {
        !self.error.read().is_empty()
    }

    pub fn error(&self) -> String {
        self.error.cloned()
    }
}
