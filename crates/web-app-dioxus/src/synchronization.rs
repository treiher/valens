//! Background synchronization from the remote backend into the local database.
//!
//! [`Synchronization`] is a Dioxus context that triggers the domain service to pull remote
//! changes from the backend into the local database, and then triggers [`crate::cache::Cache`]
//! refreshes so the UI reflects the latest state.

use dioxus::prelude::*;

use valens_domain as domain;

use valens_domain::SessionService;

use crate::{
    DOMAIN_SERVICE, NO_CONNECTION,
    cache::{Cache, CacheState},
    notification::{notify, notify_warning},
    session::SessionRefresh,
};

macro_rules! sync {
    ($entry: ident, $sync_method: ident, $refresh_method: ident) => {{
        let mut cache = consume_context::<Cache>();
        let mut synchronization = consume_context::<Synchronization>();
        synchronization
            .pending_sync_count
            .with_mut(|count| *count += 1);
        spawn(async move {
            if matches!(&*cache.$entry.peek(), CacheState::Ready(value) if value.is_empty()) {
                cache.$entry.set(CacheState::Loading);
            }
            match DOMAIN_SERVICE().$sync_method().await {
                Err(err) => handle_sync_error(&mut synchronization, &err),
                Ok(_) => {
                    *NO_CONNECTION.write() = false;
                }
            }
            cache.$refresh_method();
            synchronization.finish_sync();
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
            self.sync_session();
            sync!(exercises, sync_exercises, refresh_exercises);
            sync!(routines, sync_routines, refresh_routines);
            sync!(schedule, sync_schedule, refresh_schedule);
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

    /// Update the session user from the server and re-read the session if the user has been
    /// changed or signed out.
    fn sync_session(&self) {
        let session_refresh = consume_context::<SessionRefresh>();
        let mut synchronization = *self;
        synchronization
            .pending_sync_count
            .with_mut(|count| *count += 1);
        spawn(async move {
            let previous_user = DOMAIN_SERVICE().get_session().await.ok();
            match DOMAIN_SERVICE().sync_session().await {
                Err(err) => handle_sync_error(&mut synchronization, &err),
                Ok(user) => {
                    *NO_CONNECTION.write() = false;
                    if user != previous_user {
                        session_refresh.refresh();
                    }
                }
            }
            synchronization.finish_sync();
        });
    }

    fn finish_sync(&mut self) {
        self.pending_sync_count.with_mut(|count| {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.in_progress.set(false);
            }
        });
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

fn handle_sync_error(synchronization: &mut Synchronization, err: &domain::SyncError) {
    if matches!(
        err,
        domain::SyncError::Storage(domain::StorageError::NoConnection)
    ) {
        if !NO_CONNECTION() {
            *NO_CONNECTION.write() = true;
            notify_warning("No connection to server");
        }
    } else if !synchronization.has_error() {
        synchronization
            .error
            .set(format!("Synchronization failed: {err}"));
        notify("Synchronization failed", err);
        *NO_CONNECTION.write() = false;
    }
}
