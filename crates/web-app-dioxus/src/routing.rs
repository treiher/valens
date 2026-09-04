use dioxus::{prelude::*, router::Navigator};
use log::warn;
use web_sys::wasm_bindgen::{JsCast, prelude::Closure};

use valens_domain as domain;

use crate::{
    navbar::Navbar,
    page::{
        body_fat::BodyFat, body_weight::BodyWeight, catalog::Catalog, exercise::Exercise,
        exercises::Exercises, ffmi::Ffmi, home::Home, login::Login,
        menstrual_cycle::MenstrualCycle, muscles::Muscles, not_found::NotFound, routine::Routine,
        routines::Routines, schedule::Schedule, training_session::TrainingSession,
        training_sessions::TrainingSessions,
    },
    session::SessionProvider,
};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/login")]
    Login {},
    #[layout(SessionProvider)]
        #[layout(Navbar)]
            #[redirect("/", || Route::Home {})]
            #[route("/home")]
            Home {},
            #[route("/training_sessions?:add")]
            TrainingSessions { add: bool },
            #[route("/training_session/:id")]
            TrainingSession { id: domain::TrainingSessionID },
            #[route("/routines?:add&:search")]
            Routines { add: bool, search: String },
            #[route("/routine/:id")]
            Routine { id: domain::RoutineID },
            #[route("/schedule")]
            Schedule {},
            #[route("/exercises?:add&:filter")]
            Exercises { add: bool, filter: String },
            #[route("/exercise/:id")]
            Exercise { id: domain::ExerciseID },
            #[route("/catalog/:name")]
            Catalog { name: String },
            #[route("/muscles")]
            Muscles {},
            #[route("/body_weight?:add")]
            BodyWeight { add: bool },
            #[route("/body_fat?:add")]
            BodyFat { add: bool },
            #[route("/ffmi")]
            Ffmi {},
            #[route("/menstrual_cycle?:add")]
            MenstrualCycle { add: bool },
        #[end_layout]
        #[route("/:..route")]
        NotFound { route: Vec<String> },
}

/// The title the navigation bar shows for `route`.
#[must_use]
pub fn page_title(route: &Route, user_name: &domain::Name) -> String {
    match route {
        Route::Login {} => "Valens".to_string(),
        Route::Home {} => user_name.to_string(),
        Route::TrainingSessions { .. } => "Training sessions".to_string(),
        Route::TrainingSession { .. } => "Training session".to_string(),
        Route::Routines { .. } => "Routines".to_string(),
        Route::Routine { .. } => "Routine".to_string(),
        Route::Schedule {} => "Schedule".to_string(),
        Route::Exercises { .. } => "Exercises".to_string(),
        Route::Exercise { .. } => "Exercise".to_string(),
        Route::Catalog { .. } => "Catalog exercise".to_string(),
        Route::Muscles { .. } => "Muscles".to_string(),
        Route::BodyWeight { .. } => "Body weight".to_string(),
        Route::BodyFat { .. } => "Body fat".to_string(),
        Route::Ffmi {} => "FFMI".to_string(),
        Route::MenstrualCycle { .. } => "Menstrual cycle".to_string(),
        Route::NotFound { .. } => String::new(),
    }
}

/// The route the navigation bar leads up to from `route`, if any.
#[must_use]
pub fn go_up_target(route: &Route) -> Option<Route> {
    match route {
        Route::Login {} | Route::Home {} => None,
        Route::TrainingSessions { .. }
        | Route::Routines { .. }
        | Route::Schedule {}
        | Route::Exercises { .. }
        | Route::Muscles { .. }
        | Route::BodyWeight { .. }
        | Route::BodyFat { .. }
        | Route::Ffmi {}
        | Route::MenstrualCycle { .. }
        | Route::NotFound { .. } => Some(Route::Home {}),
        Route::TrainingSession { .. } => Some(Route::TrainingSessions { add: false }),
        Route::Routine { .. } => Some(Route::Routines {
            add: false,
            search: String::new(),
        }),
        Route::Exercise { .. } | Route::Catalog { .. } => Some(Route::Exercises {
            add: false,
            filter: String::new(),
        }),
    }
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn all_routes() -> Vec<Route> {
        vec![
            Route::Login {},
            Route::Home {},
            Route::TrainingSessions { add: false },
            Route::TrainingSession { id: 1.into() },
            Route::Routines {
                add: false,
                search: String::new(),
            },
            Route::Routine { id: 1.into() },
            Route::Schedule {},
            Route::Exercises {
                add: false,
                filter: String::new(),
            },
            Route::Exercise { id: 1.into() },
            Route::Catalog {
                name: String::new(),
            },
            Route::Muscles {},
            Route::BodyWeight { add: false },
            Route::BodyFat { add: false },
            Route::Ffmi {},
            Route::MenstrualCycle { add: false },
            Route::NotFound { route: vec![] },
        ]
    }

    #[test]
    fn test_every_route_leads_up_to_home_in_at_most_two_steps() {
        for route in all_routes() {
            let Some(up) = go_up_target(&route) else {
                continue;
            };
            let up = if up == (Route::Home {}) {
                up
            } else {
                go_up_target(&up).unwrap_or_else(|| panic!("{route:?} leads up to a dead end"))
            };
            assert_eq!(up, Route::Home {}, "reached from {route:?}");
        }
    }

    #[test]
    fn test_only_login_and_home_lead_nowhere() {
        let without_target = all_routes()
            .into_iter()
            .filter(|route| go_up_target(route).is_none())
            .collect::<Vec<_>>();

        assert_eq!(without_target, vec![Route::Login {}, Route::Home {}]);
    }
}
