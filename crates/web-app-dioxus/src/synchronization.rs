//! Background synchronization from the remote backend into the local database.
//!
//! [`Synchronization`] is a Dioxus context that triggers the domain service to pull remote
//! changes from the backend into the local database, and then triggers [`crate::cache::Cache`]
//! refreshes so the UI reflects the latest state.

use dioxus::prelude::*;

use crate::{
    DOMAIN_SERVICE,
    cache::{Cache, CacheState},
};

macro_rules! sync {
    ($cache: ident, $entry: ident, $sync_method: ident, $refresh_method: ident) => {{
        let mut cache = $cache;
        spawn(async move {
            if matches!(&*cache.$entry.peek(), CacheState::Ready(value) if value.is_empty()) {
                cache.$entry.set(CacheState::Loading);
            }
            let _ = DOMAIN_SERVICE().$sync_method().await;
            cache.$refresh_method();
        });
    }};
}

#[derive(Clone, Copy)]
pub struct Synchronization;

impl Synchronization {
    pub fn init() {
        use_effect(Synchronization::sync);
    }

    pub fn sync() {
        let cache = consume_context::<Cache>();
        sync!(cache, exercises, sync_exercises, refresh_exercises);
        sync!(cache, routines, sync_routines, refresh_routines);
        sync!(
            cache,
            training_sessions,
            sync_training_sessions,
            refresh_training_sessions
        );
        sync!(cache, body_weight, sync_body_weight, refresh_body_weight);
        sync!(cache, body_fat, sync_body_fat, refresh_body_fat);
        sync!(cache, period, sync_period, refresh_period);
    }
}
