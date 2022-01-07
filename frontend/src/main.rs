use seed::{prelude::*, *};

mod common;
mod page;

// ------ ------
//     Init
// ------ ------

fn init(_: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders
        .subscribe(Msg::UrlChanged)
        .stream(streams::window_event(Ev::Click, |_| Msg::HideMenu))
        .send_msg(Msg::InitializeSession);

    Model {
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

struct Urls;

impl Urls {
    fn home() -> Url {
        Url::new().set_path(&[""])
    }
    fn login() -> Url {
        Url::new().set_path(&[LOGIN])
    }
    fn admin() -> Url {
        Url::new().set_path(&[ADMIN])
    }
}

// ------ ------
//     Model
// ------ ------

struct Model {
    session: Option<Session>,
    navbar: Navbar,
    page: Option<Page>,
    errors: Vec<String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct Session {
    #[allow(dead_code)]
    id: u32,
    name: String,
    #[allow(dead_code)]
    sex: u8,
}

struct Navbar {
    title: String,
    items: Vec<(String, String)>,
    menu_visible: bool,
}

enum Page {
    Home(page::home::Model),
    Login(page::login::Model),
    Admin(page::admin::Model),
    NotFound,
}

impl Page {
    fn init(mut url: Url, orders: &mut impl Orders<Msg>, has_session: bool) -> Self {
        if has_session {
            match url.remaining_path_parts().as_slice() {
                [] => Self::Home(page::home::init(url, &mut orders.proxy(Msg::Home))),
                [LOGIN] => Self::Login(page::login::init(url, &mut orders.proxy(Msg::Login))),
                [ADMIN] => Self::Admin(page::admin::init(url, &mut orders.proxy(Msg::Admin))),
                _ => Self::NotFound,
            }
        } else {
            match url.remaining_path_parts().as_slice() {
                [ADMIN] => Self::Admin(page::admin::init(url, &mut orders.proxy(Msg::Admin))),
                _ => {
                    Urls::login().go_and_push();
                    Self::Login(page::login::init(url, &mut orders.proxy(Msg::Login)))
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
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::CloseErrorDialog => {
            model.errors.remove(0);
        }

        Msg::UrlChanged(subs::UrlChanged(url)) => {
            model.page = Some(Page::init(url, orders, model.session.is_some()));
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
            model.page = Some(Page::init(Url::current(), orders, true));
        }

        Msg::DeleteSession => {
            let request = Request::new("api/session").method(Method::Delete);
            orders.skip().perform_cmd(async {
                common::fetch_no_content(request, Msg::SessionDeleted).await
            });
        }
        Msg::SessionDeleted(Ok(_)) => {
            orders.request_url(Urls::login());
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
        C!["is-success"],
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
                                C!["is-size-5"],
                                C!["has-text-weight-bold"],
                                attrs! {
                                    At::Href => target,
                                },
                                name,
                            ]
                        })
                        .collect::<Vec<_>>(),
                    match &session {
                        Some(s) => div![
                            C!["navbar-item"],
                            span![
                                C!["tag"],
                                C!["is-medium"],
                                C!["has-text-light"],
                                C!["has-background-grey"],
                                C!["has-text-weight-bold"],
                                &s.name
                            ],
                            a![
                                C!["icon"],
                                C!["is-size-5"],
                                C!["has-text-grey"],
                                C!["has-text-weight-bold"],
                                C!["px-5"],
                                ev(Ev::Click, |_| Msg::SwitchUser),
                                i![C!["fas fa-sign-out-alt"]],
                            ]
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
