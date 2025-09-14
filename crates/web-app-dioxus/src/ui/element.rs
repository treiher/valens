//! Generic, domain-agnostic UI elements.

use dioxus::prelude::*;
use strum::Display;

#[allow(dead_code)]
#[derive(Display, Clone, Copy, PartialEq)]
pub enum Color {
    #[strum(to_string = "text")]
    Text,
    #[strum(to_string = "link")]
    Link,
    #[strum(to_string = "primary")]
    Primary,
    #[strum(to_string = "info")]
    Info,
    #[strum(to_string = "success")]
    Success,
    #[strum(to_string = "warning")]
    Warning,
    #[strum(to_string = "danger")]
    Danger,
    #[strum(to_string = "dark")]
    Dark,
}

#[component]
pub fn Block(children: Element, class: Option<String>) -> Element {
    rsx! {
        div {
            class: "block",
            class: if let Some(class) = &class { "{class}" },
            {children}
        }
    }
}

#[component]
pub fn CenteredBlock(children: Element, class: Option<String>) -> Element {
    rsx! {
        div {
            class: "block has-text-centered",
            class: if let Some(class) = &class { "{class}" },
            {children}
        }
    }
}

#[component]
pub fn WhiteBox(children: Element) -> Element {
    rsx! {
        div { class: "box", {children} }
    }
}

#[component]
pub fn DataBox(children: Element, title: String) -> Element {
    rsx! {
        div {
            class: "box has-text-centered mx-2 p-3",
            p {
                class: "is-size-6",
                {title}
            }
            p {
                class: "is-size-5",
                {children}
            }
        }
    }
}

#[component]
pub fn Loading() -> Element {
    rsx! {
        div {
            class: "is-size-4 has-text-centered",
            i { class: "fas fa-spinner fa-pulse" }
        }
    }
}

#[component]
pub fn LoadingDialog() -> Element {
    rsx! {
        div {
            class: "modal is-active is-visible-with-short-delay",
            div {
                class: "modal-background",
            }
            div {
                class: "modal-content is-width-auto",
                div {
                    class: "box",
                    Loading { }
                }
            }
        }
    }
}

#[component]
pub fn LoadingPage() -> Element {
    rsx! {
        div {
            class: "is-size-2 has-text-centered m-6",
            i { class: "fas fa-spinner fa-pulse" }
        }
    }
}

#[component]
pub fn Message(children: Element, color: Color) -> Element {
    rsx! {
        div {
            class: "message my-1 is-{color}",
            div {
                class: "message-body p-2",
                {children}
            }
        }
    }
}

#[component]
pub fn Error(message: String) -> Element {
    rsx! {
        IconText { icon: "triangle-exclamation", text: message, color: Color::Danger }
    }
}

#[component]
pub fn ErrorMessage(message: String) -> Element {
    rsx! {
        div {
            class: "message is-danger mx-2",
            div {
                class: "message-body has-text-dark",
                div {
                    class: "title has-text-danger is-size-4",
                    "{message}"
                }
            }
        }
    }
}

#[component]
pub fn NotFound(element: String) -> Element {
    rsx! {
        ErrorMessage { message: "{element} not found" }
    }
}

#[component]
pub fn NoData() -> Element {
    rsx! {
        div {
            class: "block is-size-7 has-text-centered has-text-grey-light mb-6",
            "No data"
        }
    }
}

#[component]
pub fn NoConnection() -> Element {
    rsx! {
        div {
            class: "block has-text-centered has-text-grey-light mb-6",
            IconText { icon: "plug-circle-xmark", text: "No connection to server" }
        }
    }
}

#[component]
pub fn Icon(
    name: String,
    is_small: Option<bool>,
    px: Option<u8>,
    onclick: Option<EventHandler<MouseEvent>>,
) -> Element {
    rsx! {
        span {
            class: "icon",
            class: if is_small.unwrap_or_default() { "is-small" },
            class: if let Some(px) = px { "px-{px}" },
            onclick: move |evt| {
                if let Some(event_handler) = onclick {
                    event_handler.call(evt);
                }
            },
            i { class: "fas fa-{name}" }
        }
    }
}

#[component]
pub fn IconText(
    icon: String,
    text: String,
    color: Option<Color>,
    onclick: Option<EventHandler<MouseEvent>>,
) -> Element {
    rsx! {
        span {
            class: "icon-text",
            class: if let Some(color) = color { "has-text-{color}" },
            onclick: move |evt| {
                if let Some(event_handler) = onclick {
                    event_handler.call(evt);
                }
            },
            Icon { name: icon }
            span { {text} }
        }
    }
}

#[component]
pub fn ElementWithDescription(
    children: Element,
    description: String,
    right_aligned: Option<bool>,
) -> Element {
    rsx! {
        div {
            class: "dropdown is-hoverable",
            class: if right_aligned.unwrap_or_default() { "is-right" },
            div {
                class: "dropdown-trigger",
                div {
                    class: "control is-clickable",
                    {children}
                }
            }
            if !description.is_empty() {
                div {
                    class: "dropdown-menu has-no-min-width",
                    div {
                        class: "dropdown-content",
                        div {
                            class: "dropdown-item",
                            "{description}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn FloatingActionButton(
    icon: String,
    onclick: EventHandler<MouseEvent>,
    is_loading: Option<bool>,
) -> Element {
    rsx! {
        button {
            class: "button is-fab is-medium is-link",
            class: if is_loading.unwrap_or_default() { "is-loading" },
            onclick,
            Icon { name: icon }
        }
    }
}

#[component]
pub fn Dialog(
    children: Element,
    title: Option<Element>,
    close_event: EventHandler<MouseEvent>,
    color: Option<Color>,
    message_body_class: Option<String>,
) -> Element {
    let color = color.unwrap_or(Color::Primary);
    rsx! {
        div {
            class: "modal is-active",
            div {
                class: "modal-background",
                onclick: close_event
            }
            div {
                class: "modal-content",
                div {
                    class: "message is-{color} mx-2",
                    div {
                        class: "message-body has-text-text-bold has-background-scheme-main",
                        class: if let Some(class) = &message_body_class { "{class}" },
                        if let Some(title) = title {
                            div {
                                class: "title has-text-{color}",
                                {title}
                            }
                        }
                        {children}
                    }
                }
            }
            button {
                aria_label: "close",
                class: "modal-close",
                onclick: close_event,
            }
        }
    }
}

#[component]
pub fn DeleteConfirmationDialog(
    element_type: String,
    element_name: Element,
    delete_event: EventHandler<MouseEvent>,
    cancel_event: EventHandler<MouseEvent>,
    is_loading: bool,
) -> Element {
    rsx! {
        Dialog {
            title: rsx! {
                span {
                    "Delete the {element_type} "
                    {element_name}
                    "?"
                }
            },
            close_event: move |evt| cancel_event.call(evt),
            color: Color::Danger,
            div {
                class: "block",
                "The {element_type} and all elements that depend on it will be permanently deleted."
            }
            div {
                class: "field is-grouped is-grouped-centered",
                div {
                    class: "control",
                    onclick: move |evt| cancel_event.call(evt),
                    button {
                        class: "button is-light is-soft",
                        "No"
                    }
                }
                div {
                    class: "control",
                    onclick: move |evt| delete_event.call(evt),
                    button {
                        class: "button is-danger",
                        class: if is_loading { "is-loading" },
                        "Yes, delete {element_type}"
                    }
                }
            }
        }
    }
}

#[component]
pub fn Container(children: Element, has_text_centered: Option<bool>) -> Element {
    rsx! {
        div {
            class: "container px-3",
            class: if has_text_centered.unwrap_or_default() { "has-text-centered" },
            {children}
        }
    }
}

#[component]
pub fn Title(children: Element, class: Option<String>) -> Element {
    rsx! {
        CenteredBlock {
            div {
                class: "container px-2",
                h1 {
                    class: "title is-5",
                    class: if let Some(c) = &class { "{c}" },
                    {children}
                }
            }
        }
    }
}

#[component]
pub fn Table(head: Option<Vec<Element>>, body: Vec<Vec<Element>>) -> Element {
    rsx! {
        div {
            class: "table-container mt-4",
            table {
                class: "table is-fullwidth is-hoverable",
                if let Some(head) = head {
                    thead {
                        tr {
                            for element in head {
                                th {
                                    {element}
                                }
                            }
                        }
                    }
                }
                tbody {
                    for row in body {
                        tr {
                            for element in row {
                                td {
                                    {element}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn OptionsMenu(options: Vec<Element>, close_event: EventHandler<MouseEvent>) -> Element {
    rsx! {
        div {
            class: "modal is-active",
            div {
                class: "modal-background",
                onclick: move |evt| close_event.call(evt),
            }
            div {
                class: "modal-content",
                div {
                    class: "box mx-2 py-3",
                    for option in options {
                        {option}
                    }
                    button {
                        aria_label: "close",
                        class: "modal-close",
                        onclick: move |evt| close_event.call(evt),
                    }
                }
            }
        }
    }
}

#[component]
pub fn MenuOption(icon: String, text: String, onclick: EventHandler<MouseEvent>) -> Element {
    rsx! {
        p {
            class: "py-2",
            a {
                class: "has-text-weight-bold",
                onclick: move |evt| onclick.call(evt),
                IconText { icon, text }
            }
        }
    }
}
#[component]
pub fn SearchBox(search_term: String, oninput: EventHandler<FormEvent>) -> Element {
    rsx! {
        div {
            class: "control has-icons-left is-flex-grow-1",
            span {
                class: "icon is-left",
                i { class: "fas fa-search" }
            }
            input {
                class: "input",
                r#type: "text",
                value: search_term,
                oninput: move |evt| oninput.call(evt),
            }
        }
    }
}

#[component]
pub fn TagsWithAddon(
    tags: Vec<(&'static str, &'static str, String, Vec<&'static str>)>,
) -> Element {
    rsx! {
        div {
            class: "field is-grouped is-grouped-multiline is-justify-content-center mx-2",
            for (name, description, value, attributes) in tags {
                ElementWithDescription {
                    description,
                    TagWithAddon { name, value, attributes },
                }
            }
        }
    }
}

#[component]
fn TagWithAddon(name: String, value: String, attributes: Vec<&'static str>) -> Element {
    let attr = attributes.join(" ");
    rsx! {
        div {
            class: "tags has-addons",
            span {
                class: "tag",
                class: "{attr}",
                {name}
            }
            span {
                class: "tag",
                class: "{attr}",
                {value}
            }
        }
    }
}

#[component]
pub fn NoWrap(children: Element) -> Element {
    rsx! {
        span { style: "white-space:nowrap", {children} }
    }
}

pub fn value_or_dash(option: Option<impl std::fmt::Display>) -> String {
    if let Some(value) = option {
        format!("{value:.1}")
    } else {
        "-".into()
    }
}
