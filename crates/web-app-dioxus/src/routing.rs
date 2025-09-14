use dioxus::{prelude::*, router::Navigator};
use log::warn;
use web_sys::wasm_bindgen::{JsCast, prelude::Closure};

use valens_domain as domain;

use crate::{
    navbar::Navbar,
    page::{
        admin::Admin, body_fat::BodyFat, body_weight::BodyWeight, catalog::Catalog,
        exercise::Exercise, exercises::Exercises, home::Home, login::Login,
        menstrual_cycle::MenstrualCycle, muscles::Muscles, not_found::NotFound, root::Root,
        routine::Routine, routines::Routines, training::Training,
        training_session::TrainingSession,
    },
};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Root {},
    #[route("/login")]
    Login {},
    #[route("/home")]
    Home {},
    #[route("/admin")]
    Admin {},
    #[route("/training?:add")]
    Training { add: bool },
    #[route("/training_session/:id")]
    TrainingSession { id: domain::TrainingSessionID },
    #[route("/routines?:add&:search")]
    Routines { add: bool, search: String },
    #[route("/routine/:id")]
    Routine { id: domain::RoutineID },
    #[route("/exercises?:add&:filter")]
    Exercises { add: bool, filter: String },
    #[route("/exercise/:id")]
    Exercise { id: domain::ExerciseID },
    #[route("/catalog/:name")]
    Catalog { name: String },
    #[route("/muscles")]
    Muscles { },
    #[route("/body_weight?:add")]
    BodyWeight { add: bool },
    #[route("/body_fat?:add")]
    BodyFat { add: bool },
    #[route("/menstrual_cycle?:add")]
    MenstrualCycle { add: bool },
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

pub trait NavigatorScrollExt {
    fn replace_preserving_scroll<R: Routable + 'static>(&self, route: R);
}

impl NavigatorScrollExt for Navigator {
    fn replace_preserving_scroll<R: Routable + 'static>(&self, route: R) {
        // Capture scroll position
        let Some(window) = web_sys::window() else {
            warn!("failed to access window");
            self.replace(route);
            return;
        };
        let x = window.scroll_x().unwrap_or(0.0);
        let y = window.scroll_y().unwrap_or(0.0);

        // Navigate
        self.replace(route);

        // Restore scroll on next frame
        let window_clone = window.clone();
        let cb = Closure::once_into_js(move |_ts: f64| {
            window_clone.scroll_to_with_x_and_y(x, y);
        });
        if let Err(e) = window.request_animation_frame(cb.unchecked_ref()) {
            warn!("failed to request animation frame: {e:?}");
        }
    }
}
