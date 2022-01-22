use chrono::{prelude::*, Duration};
use seed::{prelude::*, *};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub const ENTER_KEY: u32 = 13;

pub struct Interval {
    pub first: NaiveDate,
    pub last: NaiveDate,
}

pub async fn fetch<'a, Ms, T>(
    request: impl Into<Request<'a>>,
    message: fn(Result<T, String>) -> Ms,
) -> Ms
where
    T: 'static + for<'de> serde::Deserialize<'de>,
{
    match seed::browser::fetch::fetch(request).await {
        Ok(response) => {
            if response.status().is_ok() {
                match response.json::<T>().await {
                    Ok(data) => message(Ok(data)),
                    Err(_) => message(Err("deserialization failed".into())),
                }
            } else {
                message(Err("unexpected response".into()))
            }
        }
        Err(_) => message(Err("no connection".into())),
    }
}

pub async fn fetch_no_content<'a, Ms>(
    request: impl Into<Request<'a>>,
    message: fn(Result<(), String>) -> Ms,
) -> Ms {
    match seed::browser::fetch::fetch(request).await {
        Ok(response) => {
            if response.status().is_ok() {
                message(Ok(()))
            } else {
                message(Err("unexpected response".into()))
            }
        }
        Err(_) => message(Err("no connection".into())),
    }
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
            div![C!["block"], &error_messages[0]],
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
