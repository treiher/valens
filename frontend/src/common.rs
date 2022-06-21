use chrono::{prelude::*, Duration};
use seed::{prelude::*, *};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub const ENTER_KEY: u32 = 13;

pub struct Interval {
    pub first: NaiveDate,
    pub last: NaiveDate,
}

pub fn init_interval(dates: &[NaiveDate], show_all: bool) -> Interval {
    let today = Local::today().naive_local();
    let mut first = dates.iter().copied().min().unwrap_or(today);
    let mut last = dates.iter().copied().max().unwrap_or(today);

    if not(show_all) && last >= today - Duration::days(30) {
        first = today - Duration::days(30);
    };

    last = today;

    Interval { first, last }
}

pub fn view_dialog<Ms>(
    color: &str,
    title: &str,
    content: Vec<Node<Ms>>,
    close_event: &EventHandler<Ms>,
) -> Node<Ms> {
    div![
        C!["modal"],
        C!["is-active"],
        div![C!["modal-background"], close_event],
        div![
            C!["modal-content"],
            div![
                C!["message"],
                C!["has-background-white"],
                C![format!("is-{}", color)],
                C!["mx-2"],
                div![
                    C!["message-body"],
                    C!["has-text-dark"],
                    div![C!["title"], C![format!("has-text-{}", color)], title],
                    content
                ]
            ]
        ],
        button![
            C!["modal-close"],
            attrs! {
                At::AriaLabel => "close",
            },
            close_event,
        ]
    ]
}

pub fn view_error_dialog<Ms>(
    error_messages: &[String],
    close_event: &EventHandler<Ms>,
) -> Node<Ms> {
    if error_messages.is_empty() {
        return Node::Empty;
    }

    view_dialog(
        "danger",
        "Error",
        nodes![
            div![C!["block"], &error_messages.last()],
            div![
                C!["field"],
                C!["is-grouped"],
                C!["is-grouped-centered"],
                div![
                    C!["control"],
                    button![C!["button"], C!["is-danger"], close_event, "Close"]
                ],
            ],
        ],
        close_event,
    )
}

pub fn view_delete_confirmation_dialog<Ms>(
    element: &str,
    delete_event: &EventHandler<Ms>,
    cancel_event: &EventHandler<Ms>,
    loading: bool,
) -> Node<Ms> {
    view_dialog(
        "danger",
        &format!("Delete the {}?", element),
        nodes![
            div![
                C!["block"],
                format!(
                    "The {} and all elements that depend on it will be permanently deleted.",
                    element
                ),
            ],
            div![
                C!["field"],
                C!["is-grouped"],
                C!["is-grouped-centered"],
                div![
                    C!["control"],
                    button![C!["button"], C!["is-light"], cancel_event, "No"]
                ],
                div![
                    C!["control"],
                    button![
                        C!["button"],
                        C!["is-danger"],
                        C![IF![loading => "is-loading"]],
                        delete_event,
                        format!("Yes, delete {}", element),
                    ]
                ],
            ],
        ],
        cancel_event,
    )
}

pub fn view_fab<Ms>(message: impl FnOnce(web_sys::Event) -> Ms + 'static + Clone) -> Node<Ms>
where
    Ms: 'static,
{
    button![
        C!["button"],
        C!["is-fab"],
        C!["is-medium"],
        C!["is-link"],
        ev(Ev::Click, message),
        span![C!["icon"], i![C!["fas fa-plus"]]]
    ]
}

pub fn view_interval_buttons<Ms>(
    current: &Interval,
    message: fn(NaiveDate, NaiveDate) -> Ms,
) -> Node<Ms>
where
    Ms: 'static,
{
    let duration = (current.last - current.first) + Duration::days(2);
    let intervals = [
        (
            "1Y",
            current.last - Duration::days(365),
            current.last,
            duration == Duration::days(367),
        ),
        (
            "6M",
            current.last - Duration::days(182),
            current.last,
            duration == Duration::days(184),
        ),
        (
            "3M",
            current.last - Duration::days(91),
            current.last,
            duration == Duration::days(93),
        ),
        (
            "1M",
            current.last - Duration::days(30),
            current.last,
            duration == Duration::days(32),
        ),
        (
            "+",
            current.first + duration / 4,
            current.last - duration / 4,
            false,
        ),
        (
            "−",
            current.first - duration / 2,
            current.last + duration / 2,
            false,
        ),
        (
            "<",
            current.first - duration / 4,
            current.last - duration / 4,
            false,
        ),
        (
            ">",
            current.first + duration / 4,
            current.last + duration / 4,
            false,
        ),
    ];

    div![
        C!["field"],
        C!["has-addons"],
        C!["has-addons-centered"],
        intervals
            .iter()
            .map(|(name, first, last, is_active)| {
                #[allow(clippy::clone_on_copy)]
                let f = first.clone();
                #[allow(clippy::clone_on_copy)]
                let l = last.clone();
                p![
                    C!["control"],
                    a![
                        C!["button"],
                        C!["is-small"],
                        C![IF![*is_active => "is-link"]],
                        ev(Ev::Click, move |_| message(f, l)),
                        name,
                    ]
                ]
            })
            .collect::<Vec<_>>()
    ]
}

pub fn view_diagram<Ms>(
    base_url: &Url,
    kind: &str,
    interval: &Interval,
    represented_data: &impl Hash,
) -> Node<Ms> {
    // The hash must uniquely represent the state of the represented data to ensure that the
    // right image is loaded when the data has been changed.
    let mut hasher = DefaultHasher::new();
    represented_data.hash(&mut hasher);
    div![
        C!["container"],
        C!["has-text-centered"],
        style! {
            St::MaxWidth => "360pt"
        },
        img![attrs! {
            At::Src => {
                base_url
                    .clone()
                    .add_path_part("api")
                    .add_path_part("images")
                    .add_path_part(kind)
                    .set_hash("")
                    .set_search(UrlSearch::new(vec![
                            ("first", vec![interval.first.to_string()]),
                            ("last", vec![interval.last.to_string()]),
                            ("x", vec![format!("{:x}", hasher.finish())]),
                    ]))
            }
        }]
    ]
}

pub fn view_loading<Ms>() -> Node<Ms> {
    div![
        C!["is-size-4"],
        C!["has-text-centered"],
        i![C!["fas fa-spinner fa-pulse"]]
    ]
}

pub fn view_error_not_found<Ms>(element: &str) -> Node<Ms> {
    div![
        C!["message"],
        C!["has-background-white"],
        C!["is-danger"],
        C!["mx-2"],
        div![
            C!["message-body"],
            C!["has-text-dark"],
            div![
                C!["title"],
                C!["has-text-danger"],
                C!["is-size-4"],
                format!("{element} not found")
            ],
        ]
    ]
}

pub fn value_or_dash(option: Option<impl std::fmt::Display>) -> String {
    if let Some(value) = option {
        format!("{:.1}", value)
    } else {
        "-".into()
    }
}
