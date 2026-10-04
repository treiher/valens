//! Navigation bar.

use dioxus::{prelude::*, web::WebEventExt};

use futures_util::StreamExt;
use gloo_timers::future::IntervalStream;
use log::warn;

use valens_domain as domain;

use crate::{
    DROP_SET_CALCULATOR, ONE_REP_MAX_CALCULATOR, Route,
    audio::{
        Metronome, MetronomeService, MutableTimer, Stopwatch, StopwatchService, TICK_INTERVAL_MS,
        TimerService,
    },
    cache::Cache,
    dialog::{
        about::AboutDialog, admin::AdminDialog, drop_set::DropSetCalculator,
        one_rep_max::OneRepMaxCalculator, profile::ProfileDialog, settings::SettingsDialog,
    },
    ongoing_training_session::OngoingTrainingSession,
    routing::{go_up_target, page_title},
    session::{Session, sign_out},
    settings::Settings,
    synchronization::Synchronization,
    ui::element::{ActivityBar, Dialog, ElementWithDescription, Icon, SectionCaption},
};

#[component]
pub fn Navbar() -> Element {
    let ongoing = consume_context::<OngoingTrainingSession>();
    use_effect(|| {
        set_body_class("has-navbar-fixed-top", true);
        set_body_class("has-navbar-fixed-bottom", true);
    });
    use_effect(move || {
        set_body_class("has-activity-bar", ongoing.get().is_some());
    });
    use_drop(|| {
        set_body_class("has-navbar-fixed-top", false);
        set_body_class("has-navbar-fixed-bottom", false);
        set_body_class("has-activity-bar", false);
    });

    let mut menu_visible = use_signal(|| false);
    let mut metronome_time_stopwatch_visible = use_signal(|| false);
    let mut settings_visible = use_signal(|| false);
    let mut profile_visible = use_signal(|| false);
    let mut admin_visible = use_signal(|| false);
    let mut about_visible = use_signal(|| false);
    let session = consume_context::<Session>();
    let settings = use_context::<Settings>();

    let mut stopwatch = use_signal(StopwatchService::new);
    let mut timer = use_signal(|| TimerService::new(60));
    let mut metronome = use_store(MetronomeService::new);
    use_effect(move || {
        metronome.write().set_beep_volume(settings.beep_volume());
        timer.write().set_beep_volume(settings.beep_volume());
    });
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        let mut interval = IntervalStream::new(TICK_INTERVAL_MS);
        while interval.next().await.is_some() {
            // Writing on every tick would re-render the navigation bar ten times a second.
            if metronome.peek().is_active() {
                metronome.write().update();
            }
            if stopwatch.peek().is_active() {
                stopwatch.write().update();
            }
            timer.peek().sync();
            if timer.peek().needs_update() {
                timer.write().update();
            }
        }
    });

    let user = session.user();
    let route = use_route::<Route>();
    let go_up_target = go_up_target(&route);

    let mut synchronization = consume_context::<Synchronization>();

    rsx! {
        nav {
            class: "navbar is-fixed-top is-primary has-shadow has-text-weight-bold",
            div {
                class: "container",
                div {
                    class: "navbar-brand is-flex-grow-1",
                    div {
                        class: "navbar-item is-clickable is-size-5",
                        class: if go_up_target.is_none() { "has-text-primary" },
                        "data-testid": "navbar-back",
                        onclick: {
                            let go_up_target = go_up_target.clone();
                            move |_| {
                                if let Some(go_up_target) = &go_up_target {
                                    navigator().push(go_up_target.clone());
                                }
                            }
                        },
                        Icon {
                            name: "chevron-left",
                        }
                    }
                    NavbarTitle { route: route.clone(), user_name: user.name.clone() }
                    div { class: "mx-auto" }
                    if synchronization.in_progress() {
                        a {
                            class: "navbar-item is-size-5 mx-1",
                            "data-testid": "navbar-sync-indicator",
                            ElementWithDescription {
                                description: rsx! { "Synchronization in progress" },
                                right_aligned: true,
                                Icon { name: "rotate fa-pulse" }
                            }
                        }
                    }
                    a {
                        aria_expanded: menu_visible(),
                        aria_label: "menu",
                        class: "navbar-burger ml-0",
                        class: if menu_visible() { "is-active" },
                        "data-testid": "navbar-menu",
                        role: "button",
                        onclick: move |_| { menu_visible.toggle() },
                        span { aria_hidden: "true" }
                        span { aria_hidden: "true" }
                        span { aria_hidden: "true" }
                        span { aria_hidden: "true" }
                    }
                }
                div {
                    class: "navbar-menu is-flex-grow-0",
                    class: if menu_visible() { "is-active" },
                    div {
                        class: "navbar-end",
                        a {
                            class: "navbar-item",
                            "data-testid": "navbar-metronome-timer-stopwatch",
                            onclick: move |_| {
                                metronome_time_stopwatch_visible.set(true);
                                menu_visible.set(false);
                            },
                            Icon { name: "stopwatch", px: 5 }
                            "Metronome · Stopwatch · Timer"
                        }
                        a {
                            class: "navbar-item",
                            "data-testid": "navbar-1rm-calculator",
                            onclick: move |_| {
                                ONE_REP_MAX_CALCULATOR.write().visible = true;
                                menu_visible.set(false);
                            },
                            Icon { name: "dumbbell", px: 5 }
                            "1RM calculator"
                        }
                        a {
                            class: "navbar-item",
                            "data-testid": "navbar-drop-set-calculator",
                            onclick: move |_| {
                                DROP_SET_CALCULATOR.write().visible = true;
                                menu_visible.set(false);
                            },
                            Icon { name: "arrow-down-wide-short", px: 5 }
                            "Drop set calculator"
                        }
                        a {
                            class: "navbar-item",
                            "data-testid": "navbar-settings",
                            onclick: move |_| {
                                menu_visible.set(false);
                                settings_visible.set(true);
                            },
                            Icon { name: "gear", px: 5 }
                            "Settings"
                        }
                        a {
                            class: "navbar-item",
                            "data-testid": "navbar-profile",
                            onclick: move |_| {
                                menu_visible.set(false);
                                profile_visible.set(true);
                            },
                            Icon { name: "user", px: 5 }
                            "Profile"
                        }
                        if user.role == domain::Role::ADMIN {
                            a {
                                class: "navbar-item",
                                "data-testid": "navbar-administration",
                                onclick: move |_| {
                                    menu_visible.set(false);
                                    admin_visible.set(true);
                                },
                                Icon { name: "gears", px: 5 }
                                "Administration"
                            }
                        }
                        a {
                            class: "navbar-item",
                            "data-testid": "navbar-about",
                            onclick: move |_| {
                                menu_visible.set(false);
                                about_visible.set(true);
                            },
                            Icon { name: "circle-info", px: 5 }
                            "About"
                        }
                        a {
                            class: "navbar-item",
                            "data-testid": "navbar-refresh",
                            onclick: move |_| {
                                synchronization.sync();
                                menu_visible.set(false);
                            },
                            Icon { name: "rotate", px: 5 }
                            "Refresh data"
                        }
                        a {
                            class: "navbar-item",
                            "data-testid": "navbar-logout",
                            onclick: move |_| async move {
                                if sign_out().await {
                                    ongoing.clear().await;
                                    navigator().push(Route::Login {});
                                }
                                menu_visible.set(false);
                            },
                            Icon { name: "sign-out-alt", px: 5 }
                            "Sign out ({user.name})"
                        }
                    }
                }
            }
        }

        if metronome_time_stopwatch_visible() {
            MetronomeTimerStopwatch {
                metronome,
                stopwatch,
                timer,
                on_close: move |_| { metronome_time_stopwatch_visible.set(false); }
            }
        }

        if settings_visible() {
            SettingsDialog {
                on_close: move |_| { settings_visible.set(false); }
            }
        }

        if profile_visible() {
            ProfileDialog {
                on_close: move |_| { profile_visible.set(false); }
            }
        }

        if admin_visible() {
            AdminDialog {
                on_close: move |_| { admin_visible.set(false); }
            }
        }

        if about_visible() {
            AboutDialog {
                on_close: move |_| { about_visible.set(false); }
            }
        }

        if ONE_REP_MAX_CALCULATOR.read().visible {
            OneRepMaxCalculator {}
        }

        if DROP_SET_CALCULATOR.read().visible {
            DropSetCalculator {
                on_close: move |_| { DROP_SET_CALCULATOR.write().visible = false; }
            }
        }

        div {
            class: "container py-4",
            // The columns of the schedule benefit from the full screen width
            class: if !matches!(route, Route::Schedule {}) { "is-max-desktop" },
            Outlet::<Route> {}
        }

        ActivityBarNavigate { route: route.clone() }
    }
}

fn set_body_class(name: &str, present: bool) {
    let Some(body) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.body())
    else {
        warn!("failed to access document body");
        return;
    };
    let result = if present {
        body.class_list().add_1(name)
    } else {
        body.class_list().remove_1(name)
    };
    if let Err(e) = result {
        warn!("failed to update body class `{name}`: {e:?}");
    }
}

/// The title of the page, scrolled horizontally if it exceeds the available width, with a fade at
/// each edge beyond which a part is hidden.
#[component]
fn NavbarTitle(route: Route, user_name: domain::Name) -> Element {
    let cache = consume_context::<Cache>();
    let title = page_title(&route, &user_name, &cache);
    let element = use_signal(|| None::<web_sys::Element>);
    let mut clipped = use_signal(|| (false, false));
    let mut update_clipped = move || {
        if let Some(element) = &*element.peek() {
            clipped.set(clipped_edges(element));
        }
    };
    use_effect(use_reactive((&title,), move |_| {
        if let Some(element) = &*element.peek() {
            element.set_scroll_left(0);
        }
        update_clipped();
    }));
    let (clipped_start, clipped_end) = clipped();

    rsx! {
        div {
            class: "navbar-item is-size-5 navbar-title",
            "data-testid": "page-title",
            "data-clipped-start": "{clipped_start}",
            "data-clipped-end": "{clipped_end}",
            onmounted: move |event| {
                let mut element = element;
                element.set(event.data().try_as_web_event());
                update_clipped();
            },
            onscroll: move |_| update_clipped(),
            onresize: move |_| update_clipped(),
            span {
                "{title.text}"
                if let Some((name, to)) = &title.link {
                    " · "
                    Link { "data-testid": "page-title-link", to: to.clone(), "{name}" }
                }
            }
        }
    }
}

/// Whether a part of the content of `element` is scrolled out of view at its start and at its end.
fn clipped_edges(element: &web_sys::Element) -> (bool, bool) {
    let scroll_left = element.scroll_left();
    // The scroll position is truncated to whole pixels, so the end may be reached one pixel short
    (
        scroll_left > 0,
        scroll_left + element.client_width() < element.scroll_width() - 1,
    )
}

#[component]
fn ActivityBarNavigate(route: Route) -> Element {
    let ongoing = consume_context::<OngoingTrainingSession>();
    let show_on_route = match &route {
        Route::Login {} | Route::NotFound { .. } => false,
        Route::TrainingSession { id } => ongoing.in_progress_other_than(id.as_u128()),
        _ => true,
    };
    if !show_on_route {
        return rsx! {};
    }
    let Some(ongoing) = ongoing.get() else {
        return rsx! {};
    };
    let target_id = domain::TrainingSessionID::from(ongoing.training_session_id);
    rsx! {
        ActivityBar {
            Link {
                "data-testid": "activity-bar",
                class: "is-flex is-align-items-center is-undecorated",
                to: Route::TrainingSession { id: target_id },
                span {
                    class: "icon has-text-info mr-3",
                    i { class: "fas fa-dumbbell" }
                }
                div {
                    class: "is-flex-grow-1 has-text-centered has-text-weight-bold",
                    "Training session in progress"
                }
                span {
                    class: "icon has-text-info ml-3",
                    i { class: "fas fa-chevron-right" }
                }
            }
        }
    }
}

#[component]
fn MetronomeTimerStopwatch(
    metronome: Store<MetronomeService>,
    stopwatch: Signal<StopwatchService>,
    timer: Signal<TimerService>,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        Dialog {
            on_close,
            div { class: "block",
                SectionCaption { "Metronome" }
                div {
                    class: "container has-text-centered p-4",
                    Metronome { metronome }
                }
            }
            div { class: "block",
                SectionCaption { "Stopwatch" }
                div {
                    class: "container has-text-centered p-4",
                    Stopwatch { stopwatch }
                }
            }
            div { class: "block",
                SectionCaption { "Timer" }
                div {
                    class: "container has-text-centered p-4",
                    MutableTimer { timer }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use crate::{
        routing::{ExerciseTab, RoutineTab},
        test_render::{TestCache, attribute_of, contains, render, text_of},
    };

    use super::*;

    #[rstest]
    #[case::exercise(Route::Exercise { id: 1.into(), tab: ExerciseTab::Charts }, "Squat")]
    #[case::routine(Route::Routine { id: 1.into(), tab: RoutineTab::Sessions }, "A")]
    #[case::catalog(Route::Catalog { name: "Back Extension".to_string() }, "Back Extension")]
    fn test_a_page_of_a_single_entry_is_titled_by_the_entry(
        #[case] route: Route,
        #[case] title: &str,
    ) {
        let html = render_title(route, with_entries);

        assert_eq!(text_of(&html, "page-title"), title);
    }

    #[rstest]
    #[case::exercise_loading(Route::Exercise { id: 1.into(), tab: ExerciseTab::Exercise }, TestCache::loading, "Exercise")]
    #[case::exercise_missing(Route::Exercise { id: 2.into(), tab: ExerciseTab::Exercise }, with_entries, "Exercise")]
    #[case::routine_loading(Route::Routine { id: 1.into(), tab: RoutineTab::Routine }, TestCache::loading, "Routine")]
    #[case::routine_missing(Route::Routine { id: 2.into(), tab: RoutineTab::Routine }, with_entries, "Routine")]
    #[case::training_session_loading(Route::TrainingSession { id: 1.into() }, TestCache::loading, "Training session")]
    #[case::training_session_missing(Route::TrainingSession { id: 2.into() }, with_entries, "Training session")]
    #[case::catalog_missing(Route::Catalog { name: "Unknown".to_string() }, with_entries, "Catalog exercise")]
    fn test_a_page_of_an_unavailable_entry_is_titled_by_the_kind_of_page(
        #[case] route: Route,
        #[case] cache: fn() -> TestCache,
        #[case] title: &str,
    ) {
        let html = render_title(route, cache);

        assert_eq!(text_of(&html, "page-title"), title);
    }

    #[test]
    fn test_a_training_session_is_titled_by_its_date_and_routine_linking_to_the_routine() {
        let html = render_title(Route::TrainingSession { id: 1.into() }, with_entries);

        assert_eq!(text_of(&html, "page-title"), "2026-10-04 · A");
        assert_eq!(
            attribute_of(&html, "page-title-link", "href"),
            Route::Routine {
                id: 1.into(),
                tab: RoutineTab::Routine
            }
            .to_string()
        );
    }

    #[test]
    fn test_a_training_session_without_routine_is_titled_by_its_date() {
        let html = render_title(Route::TrainingSession { id: 1.into() }, || {
            with_entries().with_routines(vec![])
        });

        assert_eq!(text_of(&html, "page-title"), "2026-10-04");
        assert!(!contains(&html, "page-title-link"));
    }

    fn render_title(route: Route, cache: impl Fn() -> TestCache + 'static) -> String {
        render(move || {
            cache().provide();
            rsx! {
                NavbarTitle { route: route.clone(), user_name: domain::Name::new("Alice").unwrap() }
            }
        })
    }

    fn with_entries() -> TestCache {
        TestCache::default()
            .with_exercises(vec![domain::Exercise {
                id: 1.into(),
                name: domain::Name::new("Squat").unwrap(),
                notes: String::new(),
                muscles: vec![],
                force: None,
                mechanic: None,
                laterality: None,
                assistance: None,
                equipment: vec![],
                category: None,
            }])
            .with_routines(vec![domain::Routine {
                id: 1.into(),
                name: domain::Name::new("A").unwrap(),
                notes: String::new(),
                archived: false,
                sections: vec![],
            }])
            .with_training_sessions(vec![domain::TrainingSession {
                id: 1.into(),
                routine_id: 1.into(),
                date: NaiveDate::from_ymd_opt(2026, 10, 4).unwrap(),
                notes: String::new(),
                elements: vec![],
                exercise_notes: std::collections::BTreeMap::new(),
                exercise_measurements: std::collections::BTreeMap::new(),
            }])
    }
}
