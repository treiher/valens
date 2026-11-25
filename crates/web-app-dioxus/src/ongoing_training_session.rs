//! State of the training session that is currently in progress.
//!
//! [`OngoingTrainingSession`] is a Dioxus context that holds the single in-progress training
//! session, if any. It is persisted across reloads of the app, and discarded when the user
//! session ends or the referenced training session no longer exists.

use dioxus::prelude::*;
use log::warn;

use valens_domain as domain;
use valens_web_app::{self as web_app, OngoingTrainingSessionService};

use crate::{
    WEB_APP_SERVICE,
    cache::{Cache, CacheState},
};

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
    #[cfg(test)]
    pub fn new_for_test(state: State) -> Self {
        Self {
            state: Signal::new(state),
        }
    }

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

    pub fn discard_if_missing(self, cache: Cache) {
        let missing = {
            let (CacheState::Ready(training_sessions), State::InProgress(ongoing)) =
                (&*cache.training_sessions.read(), &*self.state.read())
            else {
                return;
            };
            let id = domain::TrainingSessionID::from(ongoing.training_session_id);
            !training_sessions
                .iter()
                .any(|training_session| training_session.id == id)
        };
        if missing {
            spawn(async move {
                self.clear().await;
            });
        }
    }

    pub async fn set(mut self, ongoing: web_app::OngoingTrainingSession) {
        self.state.set(State::InProgress(ongoing.clone()));
        persist(ongoing).await;
    }

    /// Starts a new ongoing state for `training_session_id` at `element_idx` if the state is
    /// loaded and holds no session.
    ///
    /// The state is set before returning, so a page mounted right afterwards reads it. The
    /// persistence outlives the calling component, which may be left right after the call.
    pub fn start_if_none(mut self, training_session_id: u128, element_idx: usize) {
        if !matches!(*self.state.peek(), State::None) {
            return;
        }
        let ongoing = web_app::OngoingTrainingSession {
            element_idx,
            ..web_app::OngoingTrainingSession::new(training_session_id)
        };
        self.state.set(State::InProgress(ongoing.clone()));
        dioxus::dioxus_core::spawn_forever(persist(ongoing));
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

async fn persist(ongoing: web_app::OngoingTrainingSession) {
    if let Err(err) = WEB_APP_SERVICE
        .read()
        .set_ongoing_training_session(Some(ongoing))
        .await
    {
        warn!("failed to persist ongoing training session: {err}");
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    use crate::test_render::{provide_ongoing_training_session, render, text_of};

    /// Renders the ongoing state resulting from `start_if_none(2, 3)`.
    fn start_if_none_in(state: fn() -> State) -> String {
        render(move || {
            provide_ongoing_training_session(state());
            let ongoing = consume_context::<OngoingTrainingSession>();
            ongoing.start_if_none(2, 3);
            let state = ongoing
                .get()
                .map(|o| format!("{} {}", o.training_session_id, o.element_idx));
            rsx! {
                p { "data-testid": "state", {state} }
            }
        })
    }

    #[test]
    fn test_start_if_none_writes_the_state_at_the_element() {
        let html = start_if_none_in(|| State::None);

        assert_eq!(text_of(&html, "state"), "2 3");
    }

    #[test]
    fn test_start_if_none_leaves_a_loading_state_unchanged() {
        let html = start_if_none_in(|| State::Loading);

        assert_eq!(text_of(&html, "state"), "");
    }

    #[test]
    fn test_start_if_none_leaves_an_ongoing_session_unchanged() {
        let html = start_if_none_in(|| State::InProgress(web_app::OngoingTrainingSession::new(1)));

        assert_eq!(text_of(&html, "state"), "1 0");
    }
}
