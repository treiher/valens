#![warn(clippy::pedantic)]
#![allow(
    clippy::match_wildcard_for_single_variants,
    clippy::must_use_candidate,
    clippy::too_many_lines,
    clippy::wildcard_imports
)]

use chrono::{prelude::*, Duration};
use seed::{prelude::*, *};

mod common;
mod data;
mod page;

// ------ ------
//     Init
// ------ ------

fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders
        .skip()
        .stream(streams::window_event(Ev::BeforeUnload, Msg::BeforeUnload))
        .subscribe(Msg::UrlRequested)
        .subscribe(Msg::UrlChanged)
        .subscribe(Msg::Data)
        .stream(streams::window_event(Ev::Click, |_| Msg::HideMenu))
        .notify(data::Msg::InitializeSession);

    Model {
        navbar: Navbar {
            title: String::from("Valens"),
            items: Vec::new(),
            menu_visible: false,
        },
        page: None,
        data: data::init(url, &mut orders.proxy(Msg::Data)),
    }
}

// ------ ------
//     Urls
// ------ ------

const LOGIN: &str = "login";
const ADMIN: &str = "admin";
const BODY_WEIGHT: &str = "body_weight";
const BODY_FAT: &str = "body_fat";
const MENSTRUAL_CYCLE: &str = "menstrual_cycle";
const EXERCISES: &str = "exercises";
const EXERCISE: &str = "exercise";
const ROUTINES: &str = "routines";
const ROUTINE: &str = "routine";
const TRAINING: &str = "training";
const TRAINING_SESSION: &str = "training_session";

struct_urls!();
impl<'a> Urls<'a> {
    pub fn home(self) -> Url {
        self.base_url()
    }
    pub fn login(self) -> Url {
        self.base_url().set_hash_path([LOGIN])
    }
    pub fn admin(self) -> Url {
        self.base_url().set_hash_path([ADMIN])
    }
    pub fn body_weight(self) -> Url {
        self.base_url().set_hash_path([BODY_WEIGHT])
    }
    pub fn body_fat(self) -> Url {
        self.base_url().set_hash_path([BODY_FAT])
    }
    pub fn menstrual_cycle(self) -> Url {
        self.base_url().set_hash_path([MENSTRUAL_CYCLE])
    }
    pub fn exercises(self) -> Url {
        self.base_url().set_hash_path([EXERCISES])
    }
    pub fn exercise(self) -> Url {
        self.base_url().set_hash_path([EXERCISE])
    }
    pub fn routines(self) -> Url {
        self.base_url().set_hash_path([ROUTINES])
    }
    pub fn routine(self) -> Url {
        self.base_url().set_hash_path([ROUTINE])
    }
    pub fn training(self) -> Url {
        self.base_url().set_hash_path([TRAINING])
    }
    pub fn training_session(self) -> Url {
        self.base_url().set_hash_path([TRAINING_SESSION])
    }
}

// ------ ------
//     Model
// ------ ------

struct Model {
    navbar: Navbar,
    page: Option<Page>,
    data: data::Model,
}

pub struct Navbar {
    title: String,
    items: Vec<(EventHandler<Msg>, String)>,
    menu_visible: bool,
}

enum Page {
    Home(page::home::Model),
    Login(page::login::Model),
    Admin(page::admin::Model),
    BodyWeight(page::body_weight::Model),
    BodyFat(page::body_fat::Model),
    MenstrualCycle(page::menstrual_cycle::Model),
    Exercises(page::exercises::Model),
    Exercise(page::exercise::Model),
    Routines(page::routines::Model),
    Routine(page::routine::Model),
    Training(page::training::Model),
    TrainingSession(page::training_session::Model),
    NotFound,
}

impl Page {
    fn init(
        mut url: Url,
        orders: &mut impl Orders<Msg>,
        navbar: &mut Navbar,
        data_model: &data::Model,
    ) -> Self {
        navbar.items.clear();

        if data_model.session.is_some() {
            match url.next_hash_path_part() {
                None => Self::Home(page::home::init(
                    url,
                    &mut orders.proxy(Msg::Home),
                    data_model,
                    navbar,
                )),
                Some(LOGIN) => Self::Login(page::login::init(
                    url,
                    &mut orders.proxy(Msg::Login),
                    navbar,
                )),
                Some(ADMIN) => Self::Admin(page::admin::init(
                    url,
                    &mut orders.proxy(Msg::Admin),
                    navbar,
                )),
                Some(BODY_WEIGHT) => Self::BodyWeight(page::body_weight::init(
                    url,
                    &mut orders.proxy(Msg::BodyWeight),
                    data_model,
                    navbar,
                )),
                Some(BODY_FAT) => Self::BodyFat(page::body_fat::init(
                    url,
                    &mut orders.proxy(Msg::BodyFat),
                    data_model,
                    navbar,
                )),
                Some(MENSTRUAL_CYCLE) => Self::MenstrualCycle(page::menstrual_cycle::init(
                    url,
                    &mut orders.proxy(Msg::MenstrualCycle),
                    data_model,
                    navbar,
                )),
                Some(EXERCISES) => Self::Exercises(page::exercises::init(
                    url,
                    &mut orders.proxy(Msg::Exercises),
                    navbar,
                )),
                Some(EXERCISE) => Self::Exercise(page::exercise::init(
                    url,
                    &mut orders.proxy(Msg::Exercise),
                    data_model,
                    navbar,
                )),
                Some(ROUTINES) => Self::Routines(page::routines::init(
                    url,
                    &mut orders.proxy(Msg::Routines),
                    navbar,
                )),
                Some(ROUTINE) => Self::Routine(page::routine::init(
                    url,
                    &mut orders.proxy(Msg::Routine),
                    data_model,
                    navbar,
                )),
                Some(TRAINING) => Self::Training(page::training::init(
                    url,
                    &mut orders.proxy(Msg::Training),
                    data_model,
                    navbar,
                )),
                Some(TRAINING_SESSION) => Self::TrainingSession(page::training_session::init(
                    url,
                    &mut orders.proxy(Msg::TrainingSession),
                    data_model,
                    navbar,
                )),
                Some(_) => Self::NotFound,
            }
        } else {
            match url.next_hash_path_part() {
                Some(ADMIN) => Self::Admin(page::admin::init(
                    url,
                    &mut orders.proxy(Msg::Admin),
                    navbar,
                )),
                None | Some(_) => {
                    Urls::new(&url.to_hash_base_url()).login().go_and_push();
                    Self::Login(page::login::init(
                        url,
                        &mut orders.proxy(Msg::Login),
                        navbar,
                    ))
                }
            }
        }
    }
}

// ------ ------
//    Update
// ------ ------

enum Msg {
    BeforeUnload(web_sys::Event),
    UrlRequested(subs::UrlRequested),
    UrlChanged(subs::UrlChanged),

    ToggleMenu,
    HideMenu,

    GoUp,
    LogOut,

    // ------ Pages ------
    Home(page::home::Msg),
    Login(page::login::Msg),
    Admin(page::admin::Msg),
    BodyWeight(page::body_weight::Msg),
    BodyFat(page::body_fat::Msg),
    MenstrualCycle(page::menstrual_cycle::Msg),
    Exercises(page::exercises::Msg),
    Exercise(page::exercise::Msg),
    Routines(page::routines::Msg),
    Routine(page::routine::Msg),
    Training(page::training::Msg),
    TrainingSession(page::training_session::Msg),

    // ------ Data ------
    Data(data::Msg),
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::BeforeUnload(event) => {
            if warn_about_unsaved_changes(model) {
                let event = event.unchecked_into::<web_sys::BeforeUnloadEvent>();
                event.prevent_default();
                event.set_return_value("");
            }
        }
        Msg::UrlRequested(subs::UrlRequested(_, url_request)) => {
            if warn_about_unsaved_changes(model) {
                if Ok(true)
                    == window().confirm_with_message(
                        "Do you want to leave this page? Changes will not be saved.",
                    )
                {
                    return;
                }
                url_request.handled_and_prevent_refresh();
            }
        }
        Msg::UrlChanged(subs::UrlChanged(url)) => {
            model.page = Some(Page::init(url, orders, &mut model.navbar, &model.data));
            orders.send_msg(Msg::Data(data::Msg::ClearErrors));
            window().scroll_to_with_scroll_to_options(web_sys::ScrollToOptions::new().top(0.));
        }

        Msg::ToggleMenu => model.navbar.menu_visible = not(model.navbar.menu_visible),
        Msg::HideMenu => {
            if model.navbar.menu_visible {
                model.navbar.menu_visible = false;
            } else {
                orders.skip();
            }
        }

        Msg::GoUp => match &model.page {
            Some(Page::Home(_) | Page::Login(_)) => {}
            Some(Page::Admin(_)) => {
                orders.request_url(crate::Urls::new(&model.data.base_url).login());
            }
            Some(
                Page::BodyWeight(_)
                | Page::BodyFat(_)
                | Page::MenstrualCycle(_)
                | Page::Training(_)
                | Page::NotFound,
            )
            | None => {
                orders.request_url(crate::Urls::new(&model.data.base_url).home());
            }
            Some(Page::Exercise(_)) => {
                orders.request_url(crate::Urls::new(&model.data.base_url).exercises());
            }
            Some(Page::Routine(_)) => {
                orders.request_url(crate::Urls::new(&model.data.base_url).routines());
            }
            Some(Page::TrainingSession(_) | Page::Exercises(_) | Page::Routines(_)) => {
                orders.request_url(crate::Urls::new(&model.data.base_url).training());
            }
        },
        Msg::LogOut => {
            orders.skip().notify(data::Msg::DeleteSession);
        }

        // ------ Pages ------
        Msg::Home(msg) => {
            if let Some(Page::Home(page_model)) = &mut model.page {
                page::home::update(msg, page_model, &mut orders.proxy(Msg::Home));
            }
        }
        Msg::Login(msg) => {
            if let Some(Page::Login(page_model)) = &mut model.page {
                page::login::update(msg, page_model, &mut orders.proxy(Msg::Login));
            }
        }
        Msg::Admin(msg) => {
            if let Some(Page::Admin(page_model)) = &mut model.page {
                page::admin::update(msg, page_model, &model.data, &mut orders.proxy(Msg::Admin));
            }
        }
        Msg::BodyWeight(msg) => {
            if let Some(Page::BodyWeight(page_model)) = &mut model.page {
                page::body_weight::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::BodyWeight),
                );
            }
        }
        Msg::BodyFat(msg) => {
            if let Some(Page::BodyFat(page_model)) = &mut model.page {
                page::body_fat::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::BodyFat),
                );
            }
        }
        Msg::MenstrualCycle(msg) => {
            if let Some(Page::MenstrualCycle(page_model)) = &mut model.page {
                page::menstrual_cycle::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::MenstrualCycle),
                );
            }
        }
        Msg::Exercises(msg) => {
            if let Some(Page::Exercises(page_model)) = &mut model.page {
                page::exercises::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::Exercises),
                );
            }
        }
        Msg::Exercise(msg) => {
            if let Some(Page::Exercise(page_model)) = &mut model.page {
                page::exercise::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::Exercise),
                );
            }
        }
        Msg::Routines(msg) => {
            if let Some(Page::Routines(page_model)) = &mut model.page {
                page::routines::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::Routines),
                );
            }
        }
        Msg::Routine(msg) => {
            if let Some(Page::Routine(page_model)) = &mut model.page {
                page::routine::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::Routine),
                );
            }
        }
        Msg::Training(msg) => {
            if let Some(Page::Training(page_model)) = &mut model.page {
                page::training::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::Training),
                );
            }
        }
        Msg::TrainingSession(msg) => {
            if let Some(Page::TrainingSession(page_model)) = &mut model.page {
                page::training_session::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::TrainingSession),
                );
            }
        }

        Msg::Data(msg) => data::update(msg, &mut model.data, &mut orders.proxy(Msg::Data)),
    }
}

fn warn_about_unsaved_changes(model: &Model) -> bool {
    if let Some(page) = &model.page {
        if let Page::Routine(model) = page {
            model.has_unsaved_changes()
        } else if let Page::TrainingSession(model) = page {
            model.has_unsaved_changes()
        } else {
            false
        }
    } else {
        false
    }
}

// ------ ------
//     View
// ------ ------

fn view(model: &Model) -> impl IntoNodes<Msg> {
    nodes![
        view_navbar(&model.navbar, &model.page, &model.data),
        view_page(&model.page, &model.data),
        data::view(&model.data).map_msg(Msg::Data),
    ]
}

fn view_navbar(navbar: &Navbar, page: &Option<Page>, data_model: &data::Model) -> Node<Msg> {
    nav![
        C!["navbar"],
        C!["is-fixed-top"],
        C!["is-primary"],
        C!["has-shadow"],
        C!["has-text-weight-bold"],
        div![
            C!["container"],
            div![
                C!["navbar-brand"],
                C!["is-flex-grow-1"],
                a![
                    C!["navbar-item"],
                    if let Some(Page::Home(_) | Page::Login(_)) = page {
                        C!["has-text-primary"]
                    } else {
                        C![]
                    },
                    C!["is-size-5"],
                    ev(Ev::Click, |_| Msg::GoUp),
                    span![C!["icon"], i![C!["fas fa-chevron-left"]]]
                ],
                div![C!["navbar-item"], C!["is-size-5"], &navbar.title],
                div![C!["mx-auto"]],
                &navbar
                    .items
                    .iter()
                    .map(|item| {
                        a![
                            C!["navbar-item"],
                            C!["is-size-5"],
                            C!["mx-1"],
                            &item.0,
                            span![C!["icon"], i![C![format!("fas fa-{}", item.1)]]],
                        ]
                    })
                    .collect::<Vec<_>>(),
                IF![data_model.session.is_some() =>
                    a![
                        C!["navbar-burger"],
                        C!["ml-0"],
                        C![IF!(navbar.menu_visible => "is-active")],
                        attrs! {
                            At::from("role") => "button",
                            At::AriaLabel => "menu",
                            At::AriaExpanded => navbar.menu_visible,
                        },
                        ev(Ev::Click, |event| {
                            event.stop_propagation();
                            Msg::ToggleMenu
                        }),
                        span![attrs! {At::AriaHidden => "true"}],
                        span![attrs! {At::AriaHidden => "true"}],
                        span![attrs! {At::AriaHidden => "true"}],
                    ]
                ],
            ],
            IF![data_model.session.is_some() =>
                div![
                    C!["navbar-menu"],
                    C!["is-flex-grow-0"],
                    C![IF!(navbar.menu_visible => "is-active")],
                    div![
                        C!["navbar-end"],
                        match &data_model.session {
                            Some(_) => a![
                                C!["navbar-item"],
                                ev(Ev::Click, |_| Msg::Data(data::Msg::Refresh)),
                                span![C!["icon"], C!["px-5"], i![C!["fas fa-rotate"]]],
                                format!(
                                    "Refresh data ({})",
                                    view_duration(Utc::now() - data_model.last_refresh)
                                ),
                            ],
                            None => empty![],
                        },
                        match &data_model.session {
                            Some(s) => a![
                                C!["navbar-item"],
                                ev(Ev::Click, |_| Msg::LogOut),
                                span![C!["icon"], C!["px-5"], i![C!["fas fa-sign-out-alt"]]],
                                format!("Logout ({})", s.name),
                            ],
                            None => empty![],
                        }
                    ],
                ]
            ]
        ]
    ]
}

fn view_duration(duration: Duration) -> String {
    if duration < Duration::minutes(1) {
        String::from("now")
    } else if duration < Duration::hours(1) {
        format!("{} min ago", duration.num_minutes())
    } else if duration < Duration::days(1) {
        format!("{} h ago", duration.num_hours())
    } else {
        format!("{} days ago", duration.num_days())
    }
}

fn view_page(page: &Option<Page>, data_model: &data::Model) -> Node<Msg> {
    div![
        C!["container"],
        C!["is-max-desktop"],
        C!["py-4"],
        match page {
            Some(Page::Home(model)) => page::home::view(model, data_model).map_msg(Msg::Home),
            Some(Page::Login(model)) => page::login::view(model, data_model).map_msg(Msg::Login),
            Some(Page::Admin(model)) => page::admin::view(model, data_model).map_msg(Msg::Admin),
            Some(Page::BodyWeight(model)) =>
                page::body_weight::view(model, data_model).map_msg(Msg::BodyWeight),
            Some(Page::BodyFat(model)) =>
                page::body_fat::view(model, data_model).map_msg(Msg::BodyFat),
            Some(Page::MenstrualCycle(model)) =>
                page::menstrual_cycle::view(model, data_model).map_msg(Msg::MenstrualCycle),
            Some(Page::Exercises(model)) =>
                page::exercises::view(model, data_model).map_msg(Msg::Exercises),
            Some(Page::Exercise(model)) =>
                page::exercise::view(model, data_model).map_msg(Msg::Exercise),
            Some(Page::Routines(model)) =>
                page::routines::view(model, data_model).map_msg(Msg::Routines),
            Some(Page::Routine(model)) =>
                page::routine::view(model, data_model).map_msg(Msg::Routine),
            Some(Page::Training(model)) =>
                page::training::view(model, data_model).map_msg(Msg::Training),
            Some(Page::TrainingSession(model)) =>
                page::training_session::view(model, data_model).map_msg(Msg::TrainingSession),
            Some(Page::NotFound) => page::not_found::view(),
            None => common::view_loading(),
        }
    ]
}

// ------ ------
//     Start
// ------ ------

fn main() {
    App::start("app", init, update, view);
}
