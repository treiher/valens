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
const PERIOD: &str = "period";
const EXERCISES: &str = "exercises";
const EXERCISE: &str = "exercise";
const ROUTINES: &str = "routines";
const ROUTINE: &str = "routine";
const WORKOUTS: &str = "workouts";
const WORKOUT: &str = "workout";

struct_urls!();
impl<'a> Urls<'a> {
    pub fn home(self) -> Url {
        self.base_url()
    }
    pub fn login(self) -> Url {
        self.base_url().set_hash_path(&[LOGIN])
    }
    pub fn admin(self) -> Url {
        self.base_url().set_hash_path(&[ADMIN])
    }
    pub fn body_weight(self) -> Url {
        self.base_url().set_hash_path(&[BODY_WEIGHT])
    }
    pub fn body_fat(self) -> Url {
        self.base_url().set_hash_path(&[BODY_FAT])
    }
    pub fn period(self) -> Url {
        self.base_url().set_hash_path(&[PERIOD])
    }
    pub fn exercises(self) -> Url {
        self.base_url().set_hash_path(&[EXERCISES])
    }
    pub fn exercise(self) -> Url {
        self.base_url().set_hash_path(&[EXERCISE])
    }
    pub fn routines(self) -> Url {
        self.base_url().set_hash_path(&[ROUTINES])
    }
    pub fn routine(self) -> Url {
        self.base_url().set_hash_path(&[ROUTINE])
    }
    pub fn workouts(self) -> Url {
        self.base_url().set_hash_path(&[WORKOUTS])
    }
    pub fn workout(self) -> Url {
        self.base_url().set_hash_path(&[WORKOUT])
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
    items: Vec<Node<Msg>>,
    menu_visible: bool,
}

enum Page {
    Home(page::home::Model),
    Login(page::login::Model),
    Admin(page::admin::Model),
    BodyWeight(page::body_weight::Model),
    BodyFat(page::body_fat::Model),
    Period(page::period::Model),
    Exercises(page::exercises::Model),
    Exercise(page::exercise::Model),
    Routines(page::routines::Model),
    Routine(page::routine::Model),
    Workouts(page::workouts::Model),
    Workout(page::workout::Model),
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
                Some(PERIOD) => Self::Period(page::period::init(
                    url,
                    &mut orders.proxy(Msg::Period),
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
                Some(WORKOUTS) => Self::Workouts(page::workouts::init(
                    url,
                    &mut orders.proxy(Msg::Workouts),
                    data_model,
                    navbar,
                )),
                Some(WORKOUT) => Self::Workout(page::workout::init(
                    url,
                    &mut orders.proxy(Msg::Workout),
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
    Period(page::period::Msg),
    Exercises(page::exercises::Msg),
    Exercise(page::exercise::Msg),
    Routines(page::routines::Msg),
    Routine(page::routine::Msg),
    Workouts(page::workouts::Msg),
    Workout(page::workout::Msg),

    // ------ Data ------
    Data(data::Msg),
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
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
            Some(Page::Home(_)) | Some(Page::Login(_)) => {}
            Some(Page::Admin(_)) => {
                orders.request_url(crate::Urls::new(&model.data.base_url).login());
            }
            Some(Page::BodyWeight(_))
            | Some(Page::BodyFat(_))
            | Some(Page::Period(_))
            | Some(Page::Exercises(_))
            | Some(Page::Routines(_))
            | Some(Page::Workouts(_))
            | Some(Page::NotFound)
            | None => {
                orders.request_url(crate::Urls::new(&model.data.base_url).home());
            }
            Some(Page::Exercise(_)) => {
                orders.request_url(crate::Urls::new(&model.data.base_url).exercises());
            }
            Some(Page::Routine(_)) => {
                orders.request_url(crate::Urls::new(&model.data.base_url).routines());
            }
            Some(Page::Workout(_)) => {
                orders.request_url(crate::Urls::new(&model.data.base_url).workouts());
            }
        },
        Msg::LogOut => {
            orders.skip().notify(data::Msg::DeleteSession);
        }

        // ------ Pages ------
        Msg::Home(msg) => {
            if let Some(Page::Home(page_model)) = &mut model.page {
                page::home::update(msg, page_model, &mut orders.proxy(Msg::Home))
            }
        }
        Msg::Login(msg) => {
            if let Some(Page::Login(page_model)) = &mut model.page {
                page::login::update(msg, page_model, &mut orders.proxy(Msg::Login))
            }
        }
        Msg::Admin(msg) => {
            if let Some(Page::Admin(page_model)) = &mut model.page {
                page::admin::update(msg, page_model, &model.data, &mut orders.proxy(Msg::Admin))
            }
        }
        Msg::BodyWeight(msg) => {
            if let Some(Page::BodyWeight(page_model)) = &mut model.page {
                page::body_weight::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::BodyWeight),
                )
            }
        }
        Msg::BodyFat(msg) => {
            if let Some(Page::BodyFat(page_model)) = &mut model.page {
                page::body_fat::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::BodyFat),
                )
            }
        }
        Msg::Period(msg) => {
            if let Some(Page::Period(page_model)) = &mut model.page {
                page::period::update(msg, page_model, &model.data, &mut orders.proxy(Msg::Period))
            }
        }
        Msg::Exercises(msg) => {
            if let Some(Page::Exercises(page_model)) = &mut model.page {
                page::exercises::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::Exercises),
                )
            }
        }
        Msg::Exercise(msg) => {
            if let Some(Page::Exercise(page_model)) = &mut model.page {
                page::exercise::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::Exercise),
                )
            }
        }
        Msg::Routines(msg) => {
            if let Some(Page::Routines(page_model)) = &mut model.page {
                page::routines::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::Routines),
                )
            }
        }
        Msg::Routine(msg) => {
            if let Some(Page::Routine(page_model)) = &mut model.page {
                page::routine::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::Routine),
                )
            }
        }
        Msg::Workouts(msg) => {
            if let Some(Page::Workouts(page_model)) = &mut model.page {
                page::workouts::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::Workouts),
                )
            }
        }
        Msg::Workout(msg) => {
            if let Some(Page::Workout(page_model)) = &mut model.page {
                page::workout::update(
                    msg,
                    page_model,
                    &model.data,
                    &mut orders.proxy(Msg::Workout),
                )
            }
        }

        Msg::Data(msg) => data::update(msg, &mut model.data, &mut orders.proxy(Msg::Data)),
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
        div![
            C!["container"],
            div![
                C!["navbar-brand"],
                a![
                    C!["navbar-item"],
                    if let Some(Page::Home(_)) | Some(Page::Login(_)) = page {
                        C!["has-text-primary"]
                    } else {
                        C!["has-text-light"]
                    },
                    C!["has-text-weight-bold"],
                    C!["is-size-5"],
                    ev(Ev::Click, |_| Msg::GoUp),
                    "❮"
                ],
                div![
                    C!["navbar-item"],
                    C!["has-text-light"],
                    C!["has-text-weight-bold"],
                    C!["is-size-5"],
                    &navbar.title,
                ],
                a![
                    C!["navbar-burger"],
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
                ],
            ],
            div![
                C!["navbar-menu"],
                C![IF!(navbar.menu_visible => "is-active")],
                div![
                    C!["navbar-end"],
                    &navbar
                        .items
                        .iter()
                        .map(|item| { a![C!["navbar-item"], C!["has-text-weight-bold"], item] })
                        .collect::<Vec<_>>(),
                    match &data_model.session {
                        Some(_) => a![
                            C!["navbar-item"],
                            C!["has-text-weight-bold"],
                            ev(Ev::Click, |_| Msg::Data(data::Msg::Refresh)),
                            span![C!["icon"], C!["px-5"], i![C!["fas fa-rotate"]]],
                            view_duration(Utc::now() - data_model.last_refresh),
                        ],
                        None => empty![],
                    },
                    match &data_model.session {
                        Some(s) => a![
                            C!["navbar-item"],
                            C!["has-text-weight-bold"],
                            ev(Ev::Click, |_| Msg::LogOut),
                            span![
                                C!["tag"],
                                C!["is-medium"],
                                C!["has-text-light"],
                                C!["has-background-grey"],
                                &s.name
                            ],
                            span![C!["icon"], C!["px-5"], i![C!["fas fa-sign-out-alt"]]]
                        ],
                        None => empty![],
                    }
                ],
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
            Some(Page::Period(model)) => page::period::view(model, data_model).map_msg(Msg::Period),
            Some(Page::Exercises(model)) =>
                page::exercises::view(model, data_model).map_msg(Msg::Exercises),
            Some(Page::Exercise(model)) =>
                page::exercise::view(model, data_model).map_msg(Msg::Exercise),
            Some(Page::Routines(model)) =>
                page::routines::view(model, data_model).map_msg(Msg::Routines),
            Some(Page::Routine(model)) =>
                page::routine::view(model, data_model).map_msg(Msg::Routine),
            Some(Page::Workouts(model)) =>
                page::workouts::view(model, data_model).map_msg(Msg::Workouts),
            Some(Page::Workout(model)) =>
                page::workout::view(model, data_model).map_msg(Msg::Workout),
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
