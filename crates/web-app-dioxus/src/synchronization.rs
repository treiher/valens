//! Background synchronization from the remote backend into the local database.
//!
//! [`Synchronization`] is a Dioxus context that triggers the domain service to pull remote
//! changes from the backend into the local database, and then triggers [`crate::cache::Cache`]
//! refreshes so the UI reflects the latest state.

use dioxus::prelude::*;

use valens_domain as domain;

use valens_domain::{SessionService, Unreachable};

use crate::{
    DOMAIN_SERVICE,
    cache::{Cache, CacheState},
    notification::notify,
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
            if let Err(err) = DOMAIN_SERVICE().$sync_method().await {
                handle_sync_error(&mut synchronization, &err);
            }
            cache.$refresh_method();
            synchronization.finish_sync();
        });
    }};
}

#[derive(Clone, Copy)]
pub struct Synchronization {
    unreachable_reported: Signal<bool>,
    error_reported: Signal<bool>,
    in_progress: Signal<bool>,
    pending_sync_count: Signal<u8>,
}

impl Synchronization {
    pub fn provide() {
        let unreachable_reported = use_signal(|| false);
        let error_reported = use_signal(|| false);
        let in_progress = use_signal(|| false);
        let pending_sync_count = use_signal(|| 0);
        use_context_provider(move || Self {
            unreachable_reported,
            error_reported,
            in_progress,
            pending_sync_count,
        });
    }

    pub fn sync(&mut self) {
        if !*self.in_progress.peek() {
            self.unreachable_reported.set(false);
            self.error_reported.set(false);
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
}

/// Notify about a failed synchronization, reporting each kind of failure at most once per
/// synchronization run so that the parallel syncs of the individual collections do not raise the
/// same notification repeatedly.
fn handle_sync_error(synchronization: &mut Synchronization, err: &domain::SyncError) {
    // An unreachable server and other errors have separate flags, so that reporting one does not
    // suppress the other.
    let mut reported = if err.unreachable() {
        synchronization.unreachable_reported
    } else {
        synchronization.error_reported
    };
    if !*reported.peek() {
        reported.set(true);
        notify("synchronize", err);
    }
}
