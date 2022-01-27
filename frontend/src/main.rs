use seed::{prelude::*, *};

mod common;
mod page;

// ------ ------
//     Init
// ------ ------

fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders
        .subscribe(Msg::UrlChanged)
        .stream(streams::window_event(Ev::Click, |_| Msg::HideMenu))
        .send_msg(Msg::InitializeSession);

    Model {
        base_url: url.to_hash_base_url(),
        session: None,
        navbar: Navbar {
            title: String::from("Valens"),
            items: Vec::new(),
            menu_visible: false,
        },
        page: None,
        errors: Vec::new(),
    }
}

// ------ ------
//     Urls
// ------ ------

const LOGIN: &str = "login";
const ADMIN: &str = "admin";
const WORKOUTS: &str = "workouts";
const ROUTINES: &str = "routines";
const EXERCISES: &str = "exercises";
const BODY_WEIGHT: &str = "body_weight";
const BODY_FAT: &str = "body_fat";
const PERIOD: &str = "period";

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
    pub fn workouts(self) -> Url {
        self.base_url().set_hash_path(&[WORKOUTS])
    }
    pub fn routines(self) -> Url {
        self.base_url().set_hash_path(&[ROUTINES])
    }
    pub fn exercises(self) -> Url {
        self.base_url().set_hash_path(&[EXERCISES])
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
}

// ------ ------
//     Model
// ------ ------

struct Model {
    base_url: Url,
    session: Option<Session>,
    navbar: Navbar,
    page: Option<Page>,
    errors: Vec<String>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Session {
    #[allow(dead_code)]
    id: u32,
    name: String,
    #[allow(dead_code)]
    sex: u8,
}

pub struct Navbar {
    title: String,
    items: Vec<(String, Url)>,
    menu_visible: bool,
}

enum Page {
    Home(page::home::Model),
    Login(page::login::Model),
    Admin(page::admin::Model),
    BodyWeight(page::body_weight::Model),
    BodyFat(page::body_fat::Model),
    Period(page::period::Model),
    NotFound,
}

impl Page {
    fn init(
        mut url: Url,
        orders: &mut impl Orders<Msg>,
        navbar: &mut Navbar,
        session: &Option<Session>,
    ) -> Self {
        navbar.items.clear();

        if let Some(session) = session {
            match url.next_hash_path_part() {
                None => Self::Home(page::home::init(
                    url,
                    &mut orders.proxy(Msg::Home),
                    session.clone(),
                )),
                Some(LOGIN) => Self::Login(page::login::init(
                    url,
                    &mut orders.proxy(Msg::Login),
                    navbar,
                )),
                Some(ADMIN) => Self::Admin(page::admin::init(url, &mut orders.proxy(Msg::Admin))),
                Some(BODY_WEIGHT) => Self::BodyWeight(page::body_weight::init(
                    url,
                    &mut orders.proxy(Msg::BodyWeight),
                )),
                Some(BODY_FAT) => Self::BodyFat(page::body_fat::init(
                    url,
                    &mut orders.proxy(Msg::BodyFat),
                    session.sex,
                )),
                Some(PERIOD) => {
                    Self::Period(page::period::init(url, &mut orders.proxy(Msg::Period)))
                }
                Some(_) => Self::NotFound,
            }
        } else {
            match url.next_hash_path_part() {
                Some(ADMIN) => Self::Admin(page::admin::init(url, &mut orders.proxy(Msg::Admin))),
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
    CloseErrorDialog,

    UrlChanged(subs::UrlChanged),

    ToggleMenu,
    HideMenu,

    InitializeSession,
    SessionInitialized(Session),

    DeleteSession,
    SessionDeleted(Result<(), String>),

    // ------ Pages ------
    Home(page::home::Msg),
    Login(page::login::Msg),
    Admin(page::admin::Msg),
    BodyWeight(page::body_weight::Msg),
    BodyFat(page::body_fat::Msg),
    Period(page::period::Msg),
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::CloseErrorDialog => {
            model.errors.remove(0);
        }

        Msg::UrlChanged(subs::UrlChanged(url)) => {
            model.page = Some(Page::init(url, orders, &mut model.navbar, &model.session));
            model.errors.clear();
        }

        Msg::ToggleMenu => model.navbar.menu_visible = not(model.navbar.menu_visible),
        Msg::HideMenu => {
            if model.navbar.menu_visible {
                model.navbar.menu_visible = false;
            } else {
                orders.skip();
            }
        }

        Msg::InitializeSession => {
            orders.skip().perform_cmd(async {
                let response = fetch("api/session").await.expect("HTTP request failed");
                if response.status().is_ok() {
                    let session = response
                        .json::<Session>()
                        .await
                        .expect("deserialization failed");
                    Msg::SessionInitialized(session)
                } else {
                    Msg::UrlChanged(subs::UrlChanged(Url::current()))
                }
            });
        }
        Msg::SessionInitialized(session) => {
            model.session = Some(session);
            model.page = Some(Page::init(
                Url::current(),
                orders,
                &mut model.navbar,
                &model.session,
            ));
        }

        Msg::DeleteSession => {
            let request = Request::new("api/session").method(Method::Delete);
            orders.skip().perform_cmd(async {
                common::fetch_no_content(request, Msg::SessionDeleted).await
            });
        }
        Msg::SessionDeleted(Ok(_)) => {
            orders.request_url(Urls::new(&model.base_url).login());
            model.session = None;
        }
        Msg::SessionDeleted(Err(message)) => {
            model
                .errors
                .push("Failed to switch users: ".to_owned() + &message);
        }

        // ------ Pages ------
        Msg::Home(msg) => {
            if let Some(Page::Home(model)) = &mut model.page {
                page::home::update(msg, model, &mut orders.proxy(Msg::Home))
            }
        }
        Msg::Login(msg) => {
            if let Some(Page::Login(page_model)) = &mut model.page {
                page::login::update(
                    msg,
                    page_model,
                    &mut orders.proxy(Msg::Login),
                    &mut model.session,
                )
            }
        }
        Msg::Admin(msg) => {
            if let Some(Page::Admin(model)) = &mut model.page {
                page::admin::update(msg, model, &mut orders.proxy(Msg::Admin))
            }
        }
        Msg::BodyWeight(msg) => {
            if let Some(Page::BodyWeight(model)) = &mut model.page {
                page::body_weight::update(msg, model, &mut orders.proxy(Msg::BodyWeight))
            }
        }
        Msg::BodyFat(msg) => {
            if let Some(Page::BodyFat(model)) = &mut model.page {
                page::body_fat::update(msg, model, &mut orders.proxy(Msg::BodyFat))
            }
        }
        Msg::Period(msg) => {
            if let Some(Page::Period(model)) = &mut model.page {
                page::period::update(msg, model, &mut orders.proxy(Msg::Period))
            }
        }
    }
}

// ------ ------
//     View
// ------ ------

fn view(model: &Model) -> impl IntoNodes<Msg> {
    nodes![
        view_navbar(&model.navbar, &model.session),
        view_page(&model.page),
        common::view_error_dialog(&model.errors, &ev(Ev::Click, |_| Msg::CloseErrorDialog)),
    ]
}

fn view_navbar(navbar: &Navbar, session: &Option<Session>) -> Node<Msg> {
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
                    C!["has-text-light"],
                    C!["has-text-weight-bold"],
                    C!["is-size-5"],
                    ev(Ev::Click, |_| Url::go_back(1)),
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
                        .map(|(name, target)| {
                            a![
                                C!["navbar-item"],
                                C!["has-text-weight-bold"],
                                attrs! {
                                    At::Href => target,
                                },
                                name,
                            ]
                        })
                        .collect::<Vec<_>>(),
                    match &session {
                        Some(s) => a![
                            C!["navbar-item"],
                            C!["has-text-weight-bold"],
                            ev(Ev::Click, |_| Msg::DeleteSession),
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

fn view_page(page: &Option<Page>) -> Node<Msg> {
    div![
        C!["container"],
        C!["is-max-desktop"],
        C!["py-4"],
        match page {
            Some(Page::Home(model)) => page::home::view(model).map_msg(Msg::Home),
            Some(Page::Login(model)) => page::login::view(model).map_msg(Msg::Login),
            Some(Page::Admin(model)) => page::admin::view(model).map_msg(Msg::Admin),
            Some(Page::BodyWeight(model)) =>
                page::body_weight::view(model).map_msg(Msg::BodyWeight),
            Some(Page::BodyFat(model)) => page::body_fat::view(model).map_msg(Msg::BodyFat),
            Some(Page::Period(model)) => page::period::view(model).map_msg(Msg::Period),
            Some(Page::NotFound) => page::not_found::view(),
            None => div![
                C!["is-size-5"],
                C!["has-text-centered"],
                i![C!["fas fa-spinner fa-pulse"]]
            ],
        }
    ]
}

// ------ ------
//     Start
// ------ ------

fn main() {
    App::start("app", init, update, view);
}
