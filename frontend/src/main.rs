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
        content: Content {
            error_messages: Vec::new(),
            page: None,
        },
    }
}

// ------ ------
//     Urls
// ------ ------

const LOGIN: &str = "login";

struct Urls;

impl Urls {
    fn home() -> Url {
        Url::new().set_path(&[""])
    }
    fn login() -> Url {
        Url::new().set_path(&[LOGIN])
    }
}

// ------ ------
//     Model
// ------ ------

struct Model {
    session: Option<Session>,
    navbar: Navbar,
    content: Content,
}

#[derive(serde::Deserialize, Debug)]
pub struct Session {
    id: u32,
    name: String,
    sex: u8,
}

struct Navbar {
    title: String,
    items: Vec<(String, String)>,
    menu_visible: bool,
}

struct Content {
    error_messages: Vec<String>,
    page: Option<Page>,
}

enum Page {
    Home(page::home::Model),
    Login(page::login::Model),
    NotFound,
}

impl Page {
    fn init(mut url: Url, orders: &mut impl Orders<Msg>, has_session: bool) -> Self {
        if has_session {
            match url.remaining_path_parts().as_slice() {
                [] => Self::Home(page::home::init(url, &mut orders.proxy(Msg::Home))),
                [LOGIN] => Self::Login(page::login::init(url, &mut orders.proxy(Msg::Login))),
                _ => Self::NotFound,
            }
        } else {
            Urls::login().go_and_push();
            Self::Login(page::login::init(url, &mut orders.proxy(Msg::Login)))
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
    InitializeSession,
    SessionFetched(Session),
    SwitchUser,
    RedirectToLogin,
    ShowError(String),

    // ------ Pages ------
    Home(page::home::Msg),
    Login(page::login::Msg),
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::UrlChanged(subs::UrlChanged(url)) => {
            model.content.error_messages.clear();
            model.content.page = Some(Page::init(url, orders, model.session.is_some()));
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
                    Msg::SessionFetched(session)
                } else {
                    Msg::RedirectToLogin
                }
            });
        }
        Msg::SessionFetched(session) => {
            model.session = Some(session);
            model.content.page = Some(Page::init(Url::current(), orders, true));
        }
        Msg::SwitchUser => {
            orders.skip().perform_cmd(async {
                match fetch(Request::new("api/session").method(Method::Delete)).await {
                    Ok(response) => {
                        if response.status().is_ok() {
                            Msg::RedirectToLogin
                        } else {
                            Msg::ShowError("Failed to switch user: unexpected response".into())
                        }
                    }
                    Err(_) => Msg::ShowError("Failed to switch user: no connection".into()),
                }
            });
        }
        Msg::RedirectToLogin => {
            orders.request_url(Urls::login());
            model.session = None;
        }
        Msg::ShowError(message) => {
            model.content.error_messages.push(message);
        }

        // ------ Pages ------
        Msg::Home(msg) => {
            if let Some(Page::Home(model)) = &mut model.content.page {
                page::home::update(msg, model, &mut orders.proxy(Msg::Home))
            }
        }
        Msg::Login(msg) => {
            if let Some(Page::Login(page_model)) = &mut model.content.page {
                page::login::update(
                    msg,
                    page_model,
                    &mut orders.proxy(Msg::Login),
                    &mut model.session,
                )
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
        view_content(&model.content),
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

fn view_content(content: &Content) -> Node<Msg> {
    div![
        C!["container"],
        C!["is-max-desktop"],
        C!["py-4"],
        common::view_errors(&content.error_messages),
        match &content.page {
            Some(Page::Home(model)) => page::home::view(model).map_msg(Msg::Home),
            Some(Page::Login(model)) => page::login::view(model).map_msg(Msg::Login),
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
