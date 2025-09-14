use std::collections::BTreeMap;

use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use dioxus::prelude::*;
use plotters::style::{Color as PlottersColor, Palette, Palette99};
use strum::Display;

use valens_domain as domain;

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
pub fn CenteredBlock(children: Element) -> Element {
    rsx! {
        div { class: "block has-text-centered", {children} }
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
pub fn FloatingActionButton(icon: String, onclick: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: "button is-fab is-medium is-link",
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
pub fn Title(
    title: String,
    class: Option<String>,
    y_margin: Option<u8>,
    x_padding: Option<u8>,
    b_padding: Option<u8>,
) -> Element {
    rsx! {
        CenteredBlock {
            div {
                // TODO: remove y-margin and padding (should be realized by using block)?
                // class: "container my-{y_margin.unwrap_or(3)}",
                class: "container",
                class: if let Some(value) = x_padding { "px-{value}" },
                class: if let Some(value) = b_padding { "pb-{value}" },
                h1 {
                    class: "title is-5",
                    class: if let Some(c) = &class { "{c}" },
                    "{title}"
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
pub fn IntervalControl(
    current_interval: Signal<domain::Interval>,
    all: domain::Interval,
) -> Element {
    let current = current_interval.read();
    let today = Local::now().date_naive();
    let duration = current.last - current.first + Duration::days(1);
    let intervals = [
        (
            "ALL",
            all.first,
            all.last,
            all.first == current.first && all.last == current.last,
        ),
        (
            "1Y",
            today - Duration::days(domain::DefaultInterval::_1Y as i64),
            today,
            current.last == today
                && duration == Duration::days(domain::DefaultInterval::_1Y as i64 + 1),
        ),
        (
            "6M",
            today - Duration::days(domain::DefaultInterval::_6M as i64),
            today,
            current.last == today
                && duration == Duration::days(domain::DefaultInterval::_6M as i64 + 1),
        ),
        (
            "3M",
            today - Duration::days(domain::DefaultInterval::_3M as i64),
            today,
            current.last == today
                && duration == Duration::days(domain::DefaultInterval::_3M as i64 + 1),
        ),
        (
            "1M",
            today - Duration::days(domain::DefaultInterval::_1M as i64),
            today,
            current.last == today
                && duration == Duration::days(domain::DefaultInterval::_1M as i64 + 1),
        ),
        (
            "+",
            if current.first + Duration::days(6) <= current.last - duration / 2 {
                current.first + duration / 4
            } else {
                current.first
            },
            if current.first + Duration::days(6) <= current.last - duration / 2 {
                current.last - duration / 4
            } else {
                current.first + Duration::days(6)
            },
            false,
        ),
        (
            "−",
            if current.first - duration / 2 > all.first {
                current.first - duration / 2
            } else {
                all.first
            },
            if current.last + duration / 2 < today {
                current.last + duration / 2
            } else {
                today
            },
            false,
        ),
        (
            "<",
            if current.first - duration / 4 > all.first {
                current.first - duration / 4
            } else {
                all.first
            },
            if current.first - duration / 4 > all.first {
                current.last - duration / 4
            } else {
                all.first + duration - Duration::days(1)
            },
            false,
        ),
        (
            ">",
            if current.last + duration / 4 < today {
                current.first + duration / 4
            } else {
                today - duration + Duration::days(1)
            },
            if current.last + duration / 4 < today {
                current.last + duration / 4
            } else {
                today
            },
            false,
        ),
    ];

    rsx! {
        div {
            div {
                class: "field has-addons has-addons-centered",
                for (name, first, last, is_active) in intervals {
                    p {
                        class: "control",
                        a {
                            class: "button is-small",
                            class: if is_active { "is-link" },
                            onclick: move |_| { *current_interval.write() = domain::Interval { first, last } },
                            "{name}"
                        }
                    }
                }
            }
            div {
                class: "mb-4 is-size-6 has-text-centered",
                "{current.first} – {current.last}"
            }
        }
    }
}

#[component]
pub fn Chart(
    labels: Vec<ChartLabel>,
    chart: Result<Option<String>, String>,
    no_data_label: bool,
) -> Element {
    match chart {
        Ok(result) => match result {
            None => {
                if no_data_label {
                    rsx! {
                        NoData {}
                    }
                } else {
                    rsx! {}
                }
            }
            Some(value) => rsx! {
                div {
                    class: "container has-text-centered",
                    h1 {
                        class: "is-size-6 has-text-weight-bold",
                        {
                            labels
                                .iter()
                                .map(|label| {
                                    let color = {
                                        // TODO: move plotters-specific code into web-app?
                                        let plotters::style::RGBAColor(r, g, b, a) = plotters::style::Palette99::pick(label.color).mix(label.opacity);
                                        #[allow(clippy::cast_possible_truncation)]
                                        #[allow(clippy::cast_sign_loss)]
                                        let a = (a * 255.0) as u8;
                                        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
                                    };
                                    rsx! {
                                        span {
                                            class: "icon-text mx-1",
                                            span {
                                                class: "icon",
                                                style: "color:{color}",
                                                i { class: "fas fa-square" }
                                            }
                                            span { "{label.name}" }
                                        }
                                    }
                                })
                        }
                    }
                    div {
                        dangerous_inner_html: value,
                    }
                }
            },
        },
        Err(err) => rsx! { Error { message: err } },
    }
}

#[derive(Clone, PartialEq)]
pub struct ChartLabel {
    pub name: String,
    pub color: usize,
    pub opacity: f64,
}

#[component]
pub fn Calendar(entries: Vec<(NaiveDate, usize, f64)>, interval: domain::Interval) -> Element {
    fn style_rgba(color: usize, opacity: f64) -> String {
        // TODO: move plotters-specific code into web-app?
        let (r, g, b) = Palette99::pick(color).rgb();
        format!("rgba({r}, {g}, {b}, {opacity})")
    }

    let mut calendar: BTreeMap<NaiveDate, (usize, f64)> = BTreeMap::new();

    let mut day = interval.first.week(Weekday::Mon).first_day();
    while day <= interval.last.week(Weekday::Mon).last_day() {
        calendar.insert(day, (0, 0.));
        day += Duration::days(1);
    }

    for (date, color, opacity) in entries {
        calendar.entry(date).and_modify(|e| *e = (color, opacity));
    }

    let mut weekdays: [Vec<(NaiveDate, usize, f64)>; 7] = Default::default();
    let mut months: Vec<(NaiveDate, usize)> = vec![];
    let mut month: NaiveDate = NaiveDate::default();
    let mut num_weeks: usize = 0;
    for (i, (date, (color, opacity))) in calendar.iter().enumerate() {
        weekdays[i % 7].push((*date, *color, *opacity));
        if i % 7 == 0 || i == calendar.len() - 1 {
            if i == 0 {
                month = *date;
            } else if month.month() != date.month() || i == calendar.len() - 1 {
                months.push((month, num_weeks));
                num_weeks = 0;
                month = *date;
            }
            num_weeks += 1;
        }
    }

    rsx! {
        div {
            class: "table-container is-calendar py-2",
            table {
                class: "table is-size-7 mx-auto",
                tbody {
                    tr {
                        for (date, colspan) in months {
                            td {
                                class: "is-calendar-label",
                                colspan: colspan,
                                if colspan > 1 {
                                    "{date.year()}-{date.month():02}"
                                }
                            }
                        },
                        td { class: "is-calendar-label" }
                    }
                    for weekday in 0..weekdays.len() {
                        tr {
                            for (date, color, opacity) in weekdays[weekday].clone() {
                                td {
                                    style: if opacity > 0. {
                                        "background-color:{style_rgba(color, opacity)}"
                                    } else if date < interval.first || date > interval.last {
                                        "background-color:var(--bulma-scheme-main)"
                                    },
                                    div { "{date.day()}" }
                                }
                            }
                            td {
                                class: "is-calendar-label",
                                match weekday {
                                    0 => "Mon",
                                    1 => "Tue",
                                    2 => "Wed",
                                    3 => "Thu",
                                    4 => "Fri",
                                    5 => "Sat",
                                    6 => "Sun",
                                    _ => "",
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
