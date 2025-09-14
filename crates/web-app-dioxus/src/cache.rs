//! Reactive in-memory cache that is the single source of truth for all data displayed in the app.
//!
//! [`Cache`] is a Dioxus context that holds all domain entities the UI needs. Components never
//! fetch data themselves; they read from the cache and call the appropriate `refresh_*` method
//! after a mutation, which re-fetches the affected data from the local database via the domain
//! service.
//!
//! The cache is initialized once at the app root via [`Cache::init`]. After the synchronization
//! layer has pulled remote changes into the local database, it triggers cache refreshes to keep
//! the view up-to-date (see [`crate::synchronization`]). On logout, all entries are reset by
//! [`Cache::clear`].

use dioxus::prelude::*;
use valens_domain::{
    self as domain, BodyFatService, BodyWeightService, ExerciseService, PeriodService,
    RoutineService, TrainingSessionService,
};

use crate::DOMAIN_SERVICE;

macro_rules! refresh {
    ($self:ident, $field:ident, $method:ident) => {{
        let mut signal = $self.$field;
        spawn({
            async move {
                match DOMAIN_SERVICE().$method().await {
                    Ok(values) => {
                        signal.set(CacheState::Ready(values));
                    }
                    Err(err) => {
                        signal.set(CacheState::Error(err));
                    }
                }
            }
        });
    }};
}

macro_rules! call {
    ($self: ident, $($method:ident),*) => {{
        $($self.$method());*
    }};
}

macro_rules! clear {
    ($self: ident, $($field:ident),*) => {{
        $($self.$field.set(CacheState::Loading));*
    }};
}

#[derive(Clone, Copy)]
pub struct Cache {
    pub body_weight: Signal<CacheState<domain::BodyWeight>>,
    pub body_fat: Signal<CacheState<domain::BodyFat>>,
    pub period: Signal<CacheState<domain::Period>>,
    pub exercises: Signal<CacheState<domain::Exercise>>,
    pub routines: Signal<CacheState<domain::Routine>>,
    pub training_sessions: Signal<CacheState<domain::TrainingSession>>,
}

impl Cache {
    pub fn init() {
        let body_weight = use_signal(|| CacheState::Loading);
        let body_fat = use_signal(|| CacheState::Loading);
        let period = use_signal(|| CacheState::Loading);
        let exercises = use_signal(|| CacheState::Loading);
        let routines = use_signal(|| CacheState::Loading);
        let training_sessions = use_signal(|| CacheState::Loading);
        use_context_provider(move || Self {
            body_weight,
            body_fat,
            period,
            exercises,
            routines,
            training_sessions,
        });
        use_effect(|| consume_context::<Cache>().refresh());
    }

    pub fn refresh(&self) {
        call!(
            self,
            refresh_body_weight,
            refresh_body_fat,
            refresh_period,
            refresh_exercises,
            refresh_routines,
            refresh_training_sessions
        );
    }

    pub fn refresh_body_weight(&self) {
        refresh!(self, body_weight, get_body_weight);
    }

    pub fn refresh_body_fat(&self) {
        refresh!(self, body_fat, get_body_fat);
    }

    pub fn refresh_period(&self) {
        refresh!(self, period, get_period);
    }

    pub fn refresh_exercises(&self) {
        refresh!(self, exercises, get_exercises);
    }

    pub fn refresh_routines(&self) {
        refresh!(self, routines, get_routines);
    }

    pub fn refresh_training_sessions(&self) {
        refresh!(self, training_sessions, get_training_sessions);
    }

    pub fn add_training_session(&mut self, training_session: domain::TrainingSession) {
        self.training_sessions.with_mut(|training_sessions| {
            if let CacheState::Ready(training_sessions) = training_sessions {
                training_sessions.push(training_session);
            }
        });
    }

    pub fn clear(&mut self) {
        clear!(
            self,
            body_weight,
            body_fat,
            period,
            exercises,
            routines,
            training_sessions
        );
    }
}

#[derive(Debug)]
pub enum CacheState<T> {
    Loading,
    Error(domain::ReadError),
    Ready(Vec<T>),
}
