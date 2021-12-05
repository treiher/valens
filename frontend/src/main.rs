use seed::{prelude::*, *};

// ------ ------
//     Init
// ------ ------

fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders
        .subscribe(Msg::UrlChanged)
        .notify(subs::UrlChanged(url));

    Model {
        navbar: Navbar {
            title: String::from("Valens"),
            back_target: None,
            items: Vec::new(),
            username: None,
        },
        content: Content {
            error_messages: Vec::new(),
        },
    }
}

// ------ ------
//     Model
// ------ ------

struct Model {
    navbar: Navbar,
    content: Content,
}

struct Navbar {
    title: String,
    back_target: Option<String>,
    items: Vec<(String, String)>,
    username: Option<String>,
}

struct Content {
    error_messages: Vec<String>,
}

// ------ ------
//    Update
// ------ ------

enum Msg {
    UrlChanged(subs::UrlChanged),
}

fn update(msg: Msg, _: &mut Model, _: &mut impl Orders<Msg>) {
    match msg {
        Msg::UrlChanged(subs::UrlChanged(url)) => {
            log!("UrlChanged", url);
        }
    }
}

// ------ ------
//     View
// ------ ------

fn view(model: &Model) -> impl IntoNodes<Msg> {
    nodes![view_navbar(&model.navbar), view_content(&model.content),]
}

fn view_navbar(navbar: &Navbar) -> Node<Msg> {
    nav![
        C!["navbar"],
        C!["is-fixed-top"],
        C!["is-success"],
        div![
            C!["container"],
            div![
                C!["navbar-brand"],
                match &navbar.back_target {
                    Some(back_target) => a![
                        C!["navbar-item"],
                        C!["has-text-light"],
                        C!["has-text-weight-bold"],
                        C!["is-size-5"],
                        attrs! {
                            At::Href => back_target,
                        },
                        "❮"
                    ],
                    None => a![
                        C!["navbar-item"],
                        C!["has-text-success"],
                        C!["has-text-weight-bold"],
                        C!["is-size-5"],
                        "❮"
                    ],
                },
                div![
                    C!["navbar-item"],
                    C!["has-text-light"],
                    C!["has-text-weight-bold"],
                    C!["is-size-5"],
                    &navbar.title,
                ],
            ],
            input![
                C!["navbar-burger-toggle is-hidden"],
                id!("navbar-burger-toggle"),
                attrs! {
                    At::Type => "checkbox",
                },
            ],
            label![
                C!["navbar-burger"],
                attrs! {
                    At::from("for") => "navbar-burger-toggle",
                },
                span![],
                span![],
                span![],
            ],
            div![
                C!["navbar-menu"],
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
                    match &navbar.username {
                        Some(username) => div![
                            C!["navbar-item"],
                            span![
                                C!["tag"],
                                C!["is-medium"],
                                C!["has-text-light"],
                                C!["has-background-grey"],
                                C!["has-text-weight-bold"],
                                username
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
        &content
            .error_messages
            .iter()
            .map(|message| {
                div![
                    C!["message"],
                    C!["is-danger"],
                    C!["mx-2"],
                    div![C!["message-body"], message],
                ]
            })
            .collect::<Vec<_>>(),
    ]
}

// ------ ------
//     Start
// ------ ------

fn main() {
    App::start("app", init, update, view);
}
