use std::{convert::Infallible, fmt};

use dioxus::{
    prelude::*,
    router::{FromRouteSegments, Navigator, ToRouteSegments},
};
use log::warn;
use web_sys::wasm_bindgen::{JsCast, prelude::Closure};

use valens_domain as domain;

use crate::{
    cache::{Cache, CacheState},
    navbar::Navbar,
    page::{
        body_fat::BodyFat, body_weight::BodyWeight, catalog::Catalog, exercise::Exercise,
        exercises::Exercises, ffmi::Ffmi, home::Home, login::Login,
        menstrual_cycle::MenstrualCycle, not_found::NotFound, routine::Routine, routines::Routines,
        schedule::Schedule, training_session::TrainingSession, training_sessions::TrainingSessions,
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
            // `add` comes last, as an absent last parameter would leave a trailing `&`
            #[route("/training_sessions/:..tab?:muscle&:add")]
            TrainingSessions { tab: TrainingSessionsTab, muscle: Option<u8>, add: bool },
            #[route("/training_session/:id")]
            TrainingSession { id: domain::TrainingSessionID },
            #[route("/routines?:add&:search")]
            Routines { add: bool, search: String },
            #[route("/routine/:id/:..tab")]
            Routine { id: domain::RoutineID, tab: RoutineTab },
            #[route("/schedule")]
            Schedule {},
            #[route("/exercises?:add&:filter")]
            Exercises { add: bool, filter: String },
            #[route("/exercise/:id/:..tab")]
            Exercise { id: domain::ExerciseID, tab: ExerciseTab },
            #[route("/catalog/:name")]
            Catalog { name: String },
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

/// The title the navigation bar shows for a route.
#[derive(Clone, Debug, PartialEq)]
pub struct PageTitle {
    pub text: String,
    /// A trailing part of the title that links to a related page.
    pub link: Option<(String, Route)>,
}

impl From<String> for PageTitle {
    fn from(text: String) -> Self {
        Self { text, link: None }
    }
}

impl From<&str> for PageTitle {
    fn from(text: &str) -> Self {
        text.to_string().into()
    }
}

/// The title the navigation bar shows for `route`. A page of a single entry is titled by the entry,
/// or by the kind of page while the entry is not loaded or does not exist.
#[must_use]
pub fn page_title(route: &Route, user_name: &domain::Name, cache: &Cache) -> PageTitle {
    match route {
        Route::Login {} => "Valens".into(),
        Route::Home {} => user_name.to_string().into(),
        Route::TrainingSessions { .. } => "Training sessions".into(),
        Route::TrainingSession { id } => {
            training_session_title(*id, cache).unwrap_or_else(|| "Training session".into())
        }
        Route::Routines { .. } => "Routines".into(),
        Route::Routine { id, .. } => ready(&cache.routines.read())
            .iter()
            .find(|r| r.id == *id)
            .map_or_else(|| "Routine".into(), |r| r.name.to_string().into()),
        Route::Schedule {} => "Schedule".into(),
        Route::Exercises { .. } => "Exercises".into(),
        Route::Exercise { id, .. } => ready(&cache.exercises.read())
            .iter()
            .find(|e| e.id == *id)
            .map_or_else(|| "Exercise".into(), |e| e.name.to_string().into()),
        Route::Catalog { name } => domain::Name::new(name)
            .ok()
            .and_then(|name| domain::catalog::exercise(&name))
            .map_or_else(|| "Catalog exercise".into(), |e| e.name.to_string().into()),
        Route::BodyWeight { .. } => "Body weight".into(),
        Route::BodyFat { .. } => "Body fat".into(),
        Route::Ffmi {} => "FFMI".into(),
        Route::MenstrualCycle { .. } => "Menstrual cycle".into(),
        Route::NotFound { .. } => "".into(),
    }
}

fn training_session_title(id: domain::TrainingSessionID, cache: &Cache) -> Option<PageTitle> {
    let training_sessions = cache.training_sessions.read();
    let training_session = ready(&training_sessions).iter().find(|t| t.id == id)?;
    let routines = cache.routines.read();
    let link = ready(&routines)
        .iter()
        .find(|r| r.id == training_session.routine_id)
        .map(|r| {
            (
                r.name.to_string(),
                Route::Routine {
                    id: r.id,
                    tab: RoutineTab::Routine,
                },
            )
        });
    Some(PageTitle {
        text: training_session.date.to_string(),
        link,
    })
}

fn ready<T>(state: &CacheState<Vec<T>>) -> &[T] {
    match state {
        CacheState::Ready(entries) => entries,
        CacheState::Loading | CacheState::Error(_) => &[],
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
        | Route::BodyWeight { .. }
        | Route::BodyFat { .. }
        | Route::Ffmi {}
        | Route::MenstrualCycle { .. }
        | Route::NotFound { .. } => Some(Route::Home {}),
        Route::TrainingSession { .. } => Some(Route::TrainingSessions {
            tab: TrainingSessionsTab::Sessions,
            muscle: None,
            add: false,
        }),
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

/// Defines the tabs of a page as an enum for the last segment of a route path. The first tab is the
/// default, which adds no segment, and every other tab is written as a segment of its given name.
/// An unknown segment is read as the default.
macro_rules! page_tabs {
    ($(#[$meta:meta])* $name:ident { $default:ident $(, $tab:ident => $query:literal)* $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        pub enum $name {
            #[default]
            $default,
            $($tab,)*
        }

        impl ToRouteSegments for $name {
            fn display_route_segments(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $name::$default => Ok(()),
                    $($name::$tab => write!(f, "/{}", $query),)*
                }
            }
        }

        impl FromRouteSegments for $name {
            type Err = Infallible;

            fn from_route_segments(segments: &[&str]) -> Result<Self, Self::Err> {
                Ok(match segments {
                    $([$query] => $name::$tab,)*
                    _ => $name::$default,
                })
            }
        }
    };
}

page_tabs!(
    /// A tab of the training sessions page.
    TrainingSessionsTab { Sessions, Charts => "charts" }
);

page_tabs!(
    /// A tab of the routine page.
    RoutineTab { Routine, Sessions => "sessions", Charts => "charts" }
);

page_tabs!(
    /// A tab of the exercise page.
    ExerciseTab { Exercise, Sessions => "sessions", Charts => "charts" }
);

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
            Route::TrainingSessions {
                tab: TrainingSessionsTab::Sessions,
                muscle: None,
                add: false,
            },
            Route::TrainingSession { id: 1.into() },
            Route::Routines {
                add: false,
                search: String::new(),
            },
            Route::Routine {
                id: 1.into(),
                tab: RoutineTab::Routine,
            },
            Route::Schedule {},
            Route::Exercises {
                add: false,
                filter: String::new(),
            },
            Route::Exercise {
                id: 1.into(),
                tab: ExerciseTab::Exercise,
            },
            Route::Catalog {
                name: String::new(),
            },
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
    fn test_only_tabs_other_than_the_first_add_a_segment() {
        let route = |tab| {
            Route::TrainingSessions {
                tab,
                muscle: Some(7),
                add: false,
            }
            .to_string()
        };

        assert_eq!(
            route(TrainingSessionsTab::Sessions),
            "/training_sessions?muscle=7&add=false"
        );
        assert_eq!(
            route(TrainingSessionsTab::Charts),
            "/training_sessions/charts?muscle=7&add=false"
        );
    }

    #[rstest::rstest]
    #[case::default("/training_sessions", TrainingSessionsTab::Sessions)]
    #[case::other("/training_sessions/charts", TrainingSessionsTab::Charts)]
    #[case::unknown("/training_sessions/x", TrainingSessionsTab::Sessions)]
    #[case::empty("/training_sessions/", TrainingSessionsTab::Sessions)]
    fn test_tabs_are_read_from_the_path_and_unknown_segments_as_the_first_tab(
        #[case] path: &str,
        #[case] tab: TrainingSessionsTab,
    ) {
        assert_eq!(
            path.parse::<Route>().unwrap(),
            Route::TrainingSessions {
                tab,
                muscle: None,
                add: false,
            }
        );
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
