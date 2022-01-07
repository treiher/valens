use seed::{prelude::*, *};

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
