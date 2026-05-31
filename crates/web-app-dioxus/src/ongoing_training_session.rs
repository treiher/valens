//! State of the training session that is currently in progress.
//!
//! [`OngoingTrainingSession`] is a Dioxus context that holds the single in-progress training
//! session, if any. It is scoped to one user session rather than shared globally, so that an
//! in-progress session cannot leak to the next user.

use dioxus::prelude::*;
use log::warn;

use valens_web_app::{self as web_app, OngoingTrainingSessionService};

use crate::WEB_APP_SERVICE;

#[derive(Clone, Copy)]
pub struct OngoingTrainingSession {
    state: Signal<State>,
}

pub enum State {
    Loading,
    None,
    InProgress(web_app::OngoingTrainingSession),
}

impl OngoingTrainingSession {
    pub fn provide() {
        let state = use_signal(|| State::Loading);
        use_context_provider(move || Self { state });
    }

    /// Loads the persisted state from storage.
    ///
    /// Only the initial [`State::Loading`] is replaced, so a session started by the training
    /// session page while storage is being read is never overwritten.
    pub fn load(mut self) {
        if !matches!(*self.state.peek(), State::Loading) {
            return;
        }
        spawn(async move {
            let loaded = match WEB_APP_SERVICE.read().get_ongoing_training_session().await {
                Ok(Some(ongoing)) => State::InProgress(ongoing),
                Ok(None) => State::None,
                Err(err) => {
                    warn!("failed to load ongoing training session: {err}");
                    State::None
                }
            };
            if matches!(*self.state.peek(), State::Loading) {
                self.state.set(loaded);
            }
        });
    }

    pub async fn set(mut self, ongoing: web_app::OngoingTrainingSession) {
        self.state.set(State::InProgress(ongoing.clone()));
        if let Err(err) = WEB_APP_SERVICE
            .read()
            .set_ongoing_training_session(Some(ongoing))
            .await
        {
            warn!("failed to persist ongoing training session: {err}");
        }
    }

    pub async fn clear(mut self) {
        self.state.set(State::None);
        if let Err(err) = WEB_APP_SERVICE
            .read()
            .set_ongoing_training_session(None)
            .await
        {
            warn!("failed to clear ongoing training session: {err}");
        }
    }

    pub fn is_loaded(self) -> bool {
        !matches!(*self.state.read(), State::Loading)
    }

    pub fn in_progress_other_than(self, training_session_id: u128) -> bool {
        matches!(&*self.state.read(), State::InProgress(o) if o.training_session_id != training_session_id)
    }

    pub fn get(self) -> Option<web_app::OngoingTrainingSession> {
        match &*self.state.read() {
            State::InProgress(ongoing) => Some(ongoing.clone()),
            State::Loading | State::None => None,
        }
    }
}
