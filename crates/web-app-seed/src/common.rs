use std::collections::BTreeMap;

use chrono::{prelude::*, Duration};
use plotters::style::{Color, Palette, Palette99, RGBAColor};
use seed::{prelude::*, *};
use valens_domain as domain;

pub const ENTER_KEY: u32 = 13;

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct InputField<T> {
    pub input: String,
    pub parsed: Option<T>,
    pub orig: String,
}

impl<T: Default> Default for InputField<T> {
    fn default() -> Self {
        InputField {
            input: String::new(),
            parsed: Some(T::default()),
            orig: String::new(),
        }
    }
}

impl<T> InputField<T> {
    pub fn valid(&self) -> bool {
        self.parsed.is_some()
    }

    pub fn changed(&self) -> bool {
        self.input != self.orig
    }
}

pub fn view_title<Ms>(title: &Node<Ms>, margin: u8) -> Node<Ms> {
    div![
        C!["container"],
        C!["has-text-centered"],
        C![format!("mb-{margin}")],
        h1![C!["title"], C!["is-5"], title],
    ]
}

pub fn view_box<Ms>(title: &str, content: &str) -> Node<Ms> {
    div![
        C!["box"],
        C!["has-text-centered"],
        C!["mx-2"],
        C!["p-3"],
        p![C!["is-size-6"], title],
        p![C!["is-size-5"], raw![content]]
    ]
}

pub fn view_dialog<Ms>(
    color: &str,
    title: Node<Ms>,
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
                C![format!("is-{color}")],
                C!["mx-2"],
                div![
                    C!["message-body"],
                    C!["has-text-text-bold"],
                    C!["has-background-scheme-main"],
                    div![C!["title"], C![format!("has-text-{color}")], title],
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
        span!["Error"],
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
    element_type: &str,
    element_name: &Node<Ms>,
    delete_event: &EventHandler<Ms>,
    cancel_event: &EventHandler<Ms>,
    loading: bool,
) -> Node<Ms> {
    view_dialog(
        "danger",
        span![format!("Delete the {element_type} "), element_name, "?"],
        nodes![
            div![
                C!["block"],
                format!(
                    "The {element_type} and all elements that depend on it will be permanently deleted."
                ),
            ],
            div![
                C!["field"],
                C!["is-grouped"],
                C!["is-grouped-centered"],
                div![
                    C!["control"],
                    button![
                        C!["button"],
                        C!["is-light"],
                        C!["is-soft"],
                        cancel_event,
                        "No"
                    ]
                ],
                div![
                    C!["control"],
                    button![
                        C!["button"],
                        C!["is-danger"],
                        C![IF![loading => "is-loading"]],
                        delete_event,
                        format!("Yes, delete {element_type}"),
                    ]
                ],
            ],
        ],
        cancel_event,
    )
}

pub fn view_search_box<Ms>(
    search_term: &str,
    search_term_changed: impl FnOnce(String) -> Ms + 'static + Clone,
) -> Node<Ms>
where
    Ms: 'static,
{
    div![
        C!["control"],
        C!["has-icons-left"],
        C!["is-flex-grow-1"],
        input_ev(Ev::Input, search_term_changed),
        span![C!["icon"], C!["is-left"], i![C!["fas fa-search"]]],
        input![
            C!["input"],
            attrs! {
                At::Type => "text",
                At::Value => search_term,
            }
        ],
    ]
}

pub fn view_fab<Ms>(
    icon: &str,
    message: impl FnOnce(web_sys::Event) -> Ms + 'static + Clone,
) -> Node<Ms>
where
    Ms: 'static,
{
    button![
        C!["button"],
        C!["is-fab"],
        C!["is-medium"],
        C!["is-link"],
        ev(Ev::Click, message),
        span![C!["icon"], i![C![format!("fas fa-{icon}")]]]
    ]
}

pub fn view_interval_buttons<Ms>(
    current: &domain::Interval,
    all: &domain::Interval,
    message: fn(NaiveDate, NaiveDate) -> Ms,
) -> Node<Ms>
where
    Ms: 'static,
{
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

    div![
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
        ],
        div![
            C!["mb-4"],
            C!["is-size-6"],
            C!["has-text-centered"],
            format!("{} – {}", current.first, current.last)
        ]
    ]
}

pub fn view_loading<Ms>() -> Node<Ms> {
    div![
        C!["is-size-4"],
        C!["has-text-centered"],
        i![C!["fas fa-spinner fa-pulse"]]
    ]
}

pub fn view_page_loading<Ms>() -> Node<Ms> {
    div![
        C!["is-size-2"],
        C!["has-text-centered"],
        C!["m-6"],
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

pub fn view_versions<Ms>(backend_version: &str) -> Vec<Node<Ms>> {
    nodes![
        p![span![
            C!["icon-text"],
            span![C!["icon"], i![C!["fas fa-mobile-screen"]]],
            span![env!("VALENS_VERSION")],
        ]],
        p![span![
            C!["icon-text"],
            span![C!["icon"], i![C!["fas fa-server"]]],
            span![backend_version],
        ]],
    ]
}

pub fn value_or_dash(option: Option<impl std::fmt::Display>) -> String {
    if let Some(value) = option {
        format!("{value:.1}")
    } else {
        "-".into()
    }
}

pub fn view_rest<Ms>(target_time: u32, automatic: bool) -> Node<Ms> {
    div![
        span![
            C!["icon-text"],
            C!["has-text-weight-bold"],
            C!["mr-5"],
            "Rest"
        ],
        IF![
            target_time > 0 =>
            span![
                C!["icon-text"],
                C!["mr-4"],
                span![C!["mr-2"], i![C!["fas fa-clock-rotate-left"]]],
                span![target_time, " s"]
            ]
        ],
        IF![
            automatic =>
            span![
                C!["icon-text"],
                automatic_icon()
            ]
        ]
    ]
}

pub fn automatic_icon<Ms>() -> Node<Ms> {
    span![
        C!["fa-stack"],
        style! {
            St::Height => "1.5em",
            St::LineHeight => "1.5em",
        },
        i![C!["fas fa-circle fa-stack-1x"]],
        i![
            style! {
                St::Color => "var(--bulma-scheme-main)",
            },
            C!["fas fa-a fa-inverse fa-stack-1x"]
        ]
    ]
}

pub fn view_calendar<Ms>(
    entries: Vec<(NaiveDate, usize, f64)>,
    interval: &domain::Interval,
) -> Node<Ms> {
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

    div![
        C!["table-container"],
        C!["is-calendar"],
        C!["py-2"],
        table![
            C!["table"],
            C!["is-size-7"],
            C!["mx-auto"],
            tbody![
                tr![
                    months.iter().map(|(date, col_span)| {
                        let year = date.year();
                        let month = date.month();
                        td![
                            C!["is-calendar-label"],
                            attrs! {
                                At::ColSpan => col_span,
                            },
                            if *col_span > 1 {
                                format!("{year}-{month:02}")
                            } else {
                                String::new()
                            }
                        ]
                    }),
                    td![C!["is-calendar-label"]]
                ],
                (0..weekdays.len())
                    .map(|weekday| {
                        tr![
                            weekdays[weekday]
                                .iter()
                                .map(|(date, color, opacity)| td![
                                    if *opacity > 0. {
                                        style! {
                                            St::BackgroundColor => {
                                                let (r, g, b) = Palette99::pick(*color).rgb();
                                                format!("rgba({r}, {g}, {b}, {opacity})")
                                            }
                                        }
                                    } else if *date < interval.first || *date > interval.last {
                                        style! {
                                            St::BackgroundColor => "var(--bulma-scheme-main)"
                                        }
                                    } else {
                                        style! {}
                                    },
                                    div![date.day()]
                                ])
                                .collect::<Vec<_>>(),
                            td![
                                C!["is-calendar-label"],
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
                            ]
                        ]
                    })
                    .collect::<Vec<_>>()
            ]
        ]
    ]
}

pub fn view_chart<Ms>(
    labels: &[(&str, usize, f64)],
    chart: Result<Option<String>, Box<dyn std::error::Error>>,
    no_data_label: bool,
) -> Node<Ms> {
    match chart {
        Ok(result) => match result {
            None => if no_data_label { view_no_data() } else { empty![] },
            Some(value) => div![
                C!["container"],
                C!["has-text-centered"],
                h1![
                    C!["is-size-6"],
                    C!["has-text-weight-bold"],
                    labels
                        .iter()
                        .map(|(label, color_idx, opacity)| {
                            span![
                                C!["icon-text"],
                                C!["mx-1"],
                                span![
                                    C!["icon"],
                                    style![
                                        St::Color => {
                                            let RGBAColor(r, g, b, a) = Palette99::pick(*color_idx).mix(*opacity);
                                            #[allow(clippy::cast_possible_truncation)]
                                            #[allow(clippy::cast_sign_loss)]
                                            let a = (a*255.0) as u8;
                                            format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
                                        }
                                    ],
                                    i![C!["fas fa-square"]]
                                ],
                                span![label],
                            ]
                        })
                        .collect::<Vec<_>>(),
                ],
                raw![&value],
            ],
        },
        Err(err) => div![raw![&format!("failed to plot chart: {err}")]],
    }
}

pub fn view_no_data<Ms>() -> Node<Ms> {
    div![
        C!["is-size-7"],
        C!["block"],
        C!["has-text-centered"],
        C!["has-text-grey-light"],
        C!["mb-6"],
        "No data.".to_string(),
    ]
}

pub fn view_sets_per_muscle<Ms>(stimulus_per_muscle: &[(domain::Muscle, u32)]) -> Vec<Node<Ms>>
where
    Ms: 'static,
{
    let mut stimulus_per_muscle = stimulus_per_muscle.to_vec();
    stimulus_per_muscle.sort_by(|a, b| b.1.cmp(&a.1));
    let mut groups = [vec![], vec![], vec![], vec![]];
    for (muscle, stimulus) in stimulus_per_muscle {
        let name = muscle.name();
        let description = muscle.description();
        let sets = f64::from(stimulus) / 100.0;
        let sets_str = format!("{:.1$}", sets, usize::from(sets.fract() != 0.0));
        if sets > 10.0 {
            groups[0].push((name, description, sets_str, vec!["is-dark"]));
        } else if sets >= 3.0 {
            groups[1].push((name, description, sets_str, vec!["is-dark", "is-link"]));
        } else if sets > 0.0 {
            groups[2].push((name, description, sets_str, vec!["is-light", "is-link"]));
        } else {
            groups[3].push((name, description, sets_str, vec![]));
        }
    }
    groups
        .iter()
        .filter(|g| !g.is_empty())
        .map(|g| view_tags_with_addons(g))
        .collect::<Vec<_>>()
}

fn view_tags_with_addons<Ms>(tags: &[(&str, &str, String, Vec<&str>)]) -> Node<Ms>
where
    Ms: 'static,
{
    div![
        C!["field"],
        C!["is-grouped"],
        C!["is-grouped-multiline"],
        C!["is-justify-content-center"],
        C!["mx-2"],
        tags.iter().map(|(name, description, value, attributes)| {
            view_element_with_description(
                div![
                    C!["tags"],
                    C!["has-addons"],
                    span![C!["tag"], attributes.iter().map(|a| C![a]), name],
                    span![C!["tag"], attributes.iter().map(|a| C![a]), value]
                ],
                description,
            )
        })
    ]
}

pub fn view_element_with_description<Ms>(element: Node<Ms>, description: &str) -> Node<Ms> {
    div![
        C!["dropdown"],
        C!["is-hoverable"],
        div![
            C!["dropdown-trigger"],
            div![C!["control"], C!["is-clickable"], element]
        ],
        IF![
            not(description.is_empty()) =>
            div![
                C!["dropdown-menu"],
                C!["has-no-min-width"],
                div![
                    C!["dropdown-content"],
                    div![C!["dropdown-item"], description]
                ]
            ]
        ]
    ]
}

pub fn no_wrap<Ms>(string: &str) -> Node<Ms> {
    span![style! { St::WhiteSpace => "nowrap" }, string]
}

pub fn format_set(
    reps: Option<u32>,
    time: Option<u32>,
    show_tut: bool,
    weight: Option<f32>,
    rpe: Option<f32>,
    show_rpe: bool,
) -> String {
    let mut parts = vec![];

    if let Some(reps) = reps {
        if reps > 0 {
            parts.push(reps.to_string());
        }
    }

    if let Some(time) = time {
        if show_tut && time > 0 {
            parts.push(format!("{time} s"));
        }
    }

    if let Some(weight) = weight {
        if weight > 0.0 {
            parts.push(format!("{weight} kg"));
        }
    }

    let mut result = parts.join(" × ");

    if let Some(rpe) = rpe {
        if show_rpe && rpe > 0.0 {
            result.push_str(&format!(" @ {rpe}"));
        }
    }

    result
}

pub fn valid_reps(reps: u32) -> bool {
    reps > 0 && reps < 1000
}

pub fn valid_time(duration: u32) -> bool {
    duration > 0 && duration < 1000
}

pub fn valid_weight(weight: f32) -> bool {
    weight > 0.0 && weight < 1000.0 && (weight * 10.0 % 1.0).abs() < f32::EPSILON
}

pub fn valid_rpe(rpe: f32) -> bool {
    (0.0..=10.0).contains(&rpe) && (rpe % 0.5).abs() < f32::EPSILON
}
