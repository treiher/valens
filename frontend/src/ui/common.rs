use std::{
    borrow::BorrowMut,
    collections::{BTreeMap, HashMap},
};

use chrono::{prelude::*, Days, Duration};
use plotters::prelude::*;
use seed::{prelude::*, *};

use crate::{domain, ui::data};

pub const ENTER_KEY: u32 = 13;

pub const COLOR_BODY_WEIGHT: usize = 1;
pub const COLOR_AVG_BODY_WEIGHT: usize = 2;
pub const COLOR_BODY_FAT_JP3: usize = 4;
pub const COLOR_BODY_FAT_JP7: usize = 0;
pub const COLOR_PERIOD_INTENSITY: usize = 0;
pub const COLOR_LOAD: usize = 1;
pub const COLOR_LONG_TERM_LOAD: usize = 2;
pub const COLOR_LONG_TERM_LOAD_BOUNDS: usize = 13;
pub const COLOR_RPE: usize = 0;
pub const COLOR_SET_VOLUME: usize = 3;
pub const COLOR_VOLUME_LOAD: usize = 6;
pub const COLOR_TUT: usize = 2;
pub const COLOR_REPS: usize = 3;
pub const COLOR_REPS_RIR: usize = 4;
pub const COLOR_WEIGHT: usize = 8;
pub const COLOR_TIME: usize = 5;

#[derive(Clone)]
pub enum PlotType {
    Circle(usize, u32),
    Line(usize, u32),
    Histogram(usize),
}

pub fn plot_line_with_dots(color: usize) -> Vec<PlotType> {
    [PlotType::Line(color, 2), PlotType::Circle(color, 2)].to_vec()
}

pub fn plot_line(color: usize) -> Vec<PlotType> {
    [PlotType::Line(color, 2)].to_vec()
}

#[derive(Default)]
pub struct PlotParams {
    pub y_min_opt: Option<f32>,
    pub y_max_opt: Option<f32>,
    pub secondary: bool,
}

impl PlotParams {
    pub fn default() -> Self {
        Self {
            y_min_opt: None,
            y_max_opt: None,
            secondary: false,
        }
    }

    pub fn primary_range(min: f32, max: f32) -> Self {
        Self {
            y_min_opt: Some(min),
            y_max_opt: Some(max),
            secondary: false,
        }
    }

    pub const SECONDARY: Self = Self {
        y_max_opt: None,
        y_min_opt: None,
        secondary: true,
    };
}

pub struct PlotData {
    pub values: Vec<(NaiveDate, f32)>,
    pub plots: Vec<PlotType>,
    pub params: PlotParams,
}

#[derive(Clone, Copy, Default)]
pub struct Bounds {
    min: f32,
    max: f32,
}

impl Bounds {
    fn min_with_margin(self) -> f32 {
        assert!(0. <= self.min);
        assert!(self.min <= self.max);

        if self.min <= f32::EPSILON {
            return self.min;
        }
        self.min - self.margin()
    }

    fn max_with_margin(self) -> f32 {
        assert!(0. <= self.min);
        assert!(self.min <= self.max);

        self.max + self.margin()
    }

    fn margin(self) -> f32 {
        assert!(0. <= self.min);
        assert!(self.min <= self.max);

        if (self.max - self.min).abs() > f32::EPSILON {
            return (self.max - self.min) * 0.1;
        }
        0.1
    }
}

pub struct Interval {
    pub first: NaiveDate,
    pub last: NaiveDate,
}

impl From<std::ops::RangeInclusive<NaiveDate>> for Interval {
    fn from(value: std::ops::RangeInclusive<NaiveDate>) -> Self {
        Interval {
            first: *value.start(),
            last: *value.end(),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum DefaultInterval {
    All,
    _1Y = 365,
    _6M = 182,
    _3M = 91,
    _1M = 30,
}

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

pub fn init_interval(dates: &[NaiveDate], default_interval: DefaultInterval) -> Interval {
    let today = Local::now().date_naive();
    let mut first = dates.iter().copied().min().unwrap_or(today);
    let mut last = dates.iter().copied().max().unwrap_or(today);

    if default_interval != DefaultInterval::All
        && last >= today - Duration::days(default_interval as i64)
    {
        first = today - Duration::days(default_interval as i64);
    };

    last = today;

    Interval { first, last }
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
        &format!("Delete the {element}?"),
        nodes![
            div![
                C!["block"],
                format!(
                    "The {element} and all elements that depend on it will be permanently deleted."
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
                        format!("Yes, delete {element}"),
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
    current: &Interval,
    all: &Interval,
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
            today - Duration::days(DefaultInterval::_1Y as i64),
            today,
            current.last == today && duration == Duration::days(DefaultInterval::_1Y as i64 + 1),
        ),
        (
            "6M",
            today - Duration::days(DefaultInterval::_6M as i64),
            today,
            current.last == today && duration == Duration::days(DefaultInterval::_6M as i64 + 1),
        ),
        (
            "3M",
            today - Duration::days(DefaultInterval::_3M as i64),
            today,
            current.last == today && duration == Duration::days(DefaultInterval::_3M as i64 + 1),
        ),
        (
            "1M",
            today - Duration::days(DefaultInterval::_1M as i64),
            today,
            current.last == today && duration == Duration::days(DefaultInterval::_1M as i64 + 1),
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

pub fn view_calendar<Ms>(entries: Vec<(NaiveDate, usize, f64)>, interval: &Interval) -> Node<Ms> {
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
    labels: &[(&str, usize)],
    chart: Result<Option<String>, Box<dyn std::error::Error>>,
    no_data_label: bool,
) -> Node<Ms> {
    match chart {
        Ok(result) => match result {
            None => if no_data_label {
                div![
                    C!["is-size-7"],
                    C!["block"],
                    C!["has-text-centered"],
                    C!["mb-4"],
                    "No data.".to_string(),
                ] } else { empty![] },
            Some(value) => div![
                C!["container"],
                C!["has-text-centered"],
                h1![
                    C!["is-size-6"],
                    C!["has-text-weight-bold"],
                    labels
                        .iter()
                        .map(|(label, color_idx)| {
                            span![
                                C!["icon-text"],
                                C!["mx-1"],
                                span![
                                    C!["icon"],
                                    style![
                                        St::Color => {
                                            let (r, g, b) = Palette99::pick(*color_idx).mix(0.9).rgb();
                                            format!("#{r:02x}{g:02x}{b:02x}")
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

pub fn plot_chart(
    data: &[PlotData],
    x_min: NaiveDate,
    x_max: NaiveDate,
    theme: &data::Theme,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if all_zeros(data) {
        return Ok(None);
    }

    let (Some(primary_bounds), secondary_bounds) = determine_y_bounds(data) else {
        return Ok(None);
    };

    let mut result = String::new();

    {
        let root = SVGBackend::with_string(&mut result, (chart_width(), 200)).into_drawing_area();
        let (color, background_color) = colors(theme);

        root.fill(&background_color)?;

        let mut chart_builder = ChartBuilder::on(&root);
        chart_builder
            .margin(10f32)
            .x_label_area_size(30f32)
            .y_label_area_size(40f32);

        let mut chart = ChartBuilder::on(&root)
            .margin(10f32)
            .x_label_area_size(30f32)
            .y_label_area_size(40f32)
            .right_y_label_area_size(secondary_bounds.map_or_else(|| 0f32, |_| 40f32))
            .build_cartesian_2d(
                x_min..x_max,
                primary_bounds.min_with_margin()..primary_bounds.max_with_margin(),
            )?
            .set_secondary_coord(
                x_min..x_max,
                secondary_bounds
                    .as_ref()
                    .map_or(0.0..0.0, |b| b.min_with_margin()..b.max_with_margin()),
            );

        chart
            .configure_mesh()
            .disable_x_mesh()
            .set_all_tick_mark_size(3u32)
            .axis_style(color.mix(0.3))
            .bold_line_style(color.mix(0.05))
            .light_line_style(color.mix(0.0))
            .label_style(&color)
            .x_labels(2)
            .y_labels(6)
            .draw()?;

        if secondary_bounds.is_some() {
            chart
                .configure_secondary_axes()
                .set_all_tick_mark_size(3u32)
                .axis_style(color.mix(0.3))
                .label_style(&color)
                .draw()?;
        }

        for plot_data in data {
            let mut series = plot_data.values.iter().collect::<Vec<_>>();
            series.sort_by_key(|e| e.0);

            for plot in &plot_data.plots {
                match *plot {
                    PlotType::Circle(color, size) => {
                        let data = series
                            .iter()
                            .map(|(x, y)| {
                                Circle::new(
                                    (*x, *y),
                                    size,
                                    Palette99::pick(color).mix(0.9).filled(),
                                )
                            })
                            .collect::<Vec<_>>();
                        if plot_data.params.secondary {
                            chart.draw_secondary_series(data)?
                        } else {
                            chart.draw_series(data)?
                        }
                    }
                    PlotType::Line(color, size) => {
                        let data = LineSeries::new(
                            series.iter().map(|(x, y)| (*x, *y)),
                            Palette99::pick(color).mix(0.9).stroke_width(size),
                        );
                        if plot_data.params.secondary {
                            chart.draw_secondary_series(data)?
                        } else {
                            chart.draw_series(data)?
                        }
                    }
                    PlotType::Histogram(color) => {
                        let data = Histogram::vertical(&chart)
                            .style(Palette99::pick(color).mix(0.9).filled())
                            .margin(0) // https://github.com/plotters-rs/plotters/issues/300
                            .data(series.iter().map(|(x, y)| (*x, *y)));

                        if plot_data.params.secondary {
                            chart.draw_secondary_series(data)?
                        } else {
                            chart.draw_series(data)?
                        }
                    }
                };
            }
        }

        root.present()?;
    }

    Ok(Some(result))
}

fn all_zeros(data: &[PlotData]) -> bool {
    data.iter()
        .map(|v| v.values.iter().all(|(_, v)| *v == 0.0))
        .reduce(|l, r| l && r)
        .unwrap_or(true)
}

fn colors(theme: &data::Theme) -> (RGBColor, RGBColor) {
    let dark = RGBColor(20, 22, 26);
    match theme {
        data::Theme::System | data::Theme::Light => (dark, WHITE),
        data::Theme::Dark => (WHITE, dark),
    }
}

fn determine_y_bounds(data: &[PlotData]) -> (Option<Bounds>, Option<Bounds>) {
    let mut primary_bounds: Option<Bounds> = None;
    let mut secondary_bounds: Option<Bounds> = None;

    for plot in data.iter().filter(|plot| !plot.values.is_empty()) {
        let min = plot
            .values
            .iter()
            .map(|(_, v)| *v)
            .fold(plot.params.y_min_opt.unwrap_or(f32::MAX), f32::min);
        let max = plot
            .values
            .iter()
            .map(|(_, v)| *v)
            .fold(plot.params.y_max_opt.unwrap_or(0.), f32::max);

        assert!(min <= max, "min={min}, max={max}");

        let b = if plot.params.secondary {
            secondary_bounds.borrow_mut()
        } else {
            primary_bounds.borrow_mut()
        }
        .get_or_insert(Bounds { min, max });

        b.min = f32::min(b.min, min);
        b.max = f32::max(b.max, max);
    }

    (primary_bounds, secondary_bounds)
}

fn chart_width() -> u32 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    u32::min(
        u32::max(
            window()
                .inner_width()
                .unwrap_or(JsValue::UNDEFINED)
                .as_f64()
                .unwrap_or(420.) as u32
                - 20,
            300,
        ),
        960,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Quartile {
    Q1 = 1,
    Q2 = 2,
    Q3 = 3,
}

pub fn quartile(durations: &[Duration], quartile_num: Quartile) -> Duration {
    if durations.is_empty() {
        return Duration::days(0);
    }
    let idx = durations.len() / 2;
    match quartile_num {
        Quartile::Q1 => quartile(&durations[..idx], Quartile::Q2),
        Quartile::Q2 => {
            if durations.len() % 2 == 0 {
                (durations[idx - 1] + durations[idx]) / 2
            } else {
                durations[idx]
            }
        }
        Quartile::Q3 => {
            if durations.len() % 2 == 0 {
                quartile(&durations[idx..], Quartile::Q2)
            } else {
                quartile(&durations[idx + 1..], Quartile::Q2)
            }
        }
    }
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

#[derive(serde::Serialize)]
#[serde(tag = "task", content = "content")]
pub enum ServiceWorkerMessage {
    UpdateCache,
    ShowNotification {
        title: String,
        options: HashMap<String, String>,
    },
    CloseNotifications,
}

pub fn post_message_to_service_worker(message: &ServiceWorkerMessage) -> Result<(), String> {
    let Some(window) = web_sys::window() else {
        return Err("failed to get window".to_string());
    };
    let Some(service_worker) = window.navigator().service_worker().controller() else {
        return Err("failed to get service worker".to_string());
    };
    match JsValue::from_serde(message) {
        Ok(json_message) => {
            let Err(err) = service_worker.post_message(&json_message) else {
                return Ok(());
            };
            Err(format!("failed to post message to service worker: {err:?}"))
        }
        Err(err) => Err(format!(
            "failed to prepare message for service worker: {err}"
        )),
    }
}

/// Group a series of (date, value) pairs.
///
/// The `radius` parameter determines the number of days before and after the
/// center value to include in the calculation.
///
/// Only values which have a date within `interval` are used as a center value
/// for the calculation. Values outside the interval are included in the
/// calculation if they fall within the radius of a center value.
///
/// Two user-provided functions determine how values are combined:
///
///  - `group_day` is called to combine values of the *same* day.
///  - `group_range` is called to combine values of multiple days after all
///     values for the same day have been combined by `group_day`.
///
/// Return `None` in those functions to indicate the absence of a value.
///
pub fn centered_moving_grouping(
    data: &Vec<(NaiveDate, f32)>,
    interval: &Interval,
    radius: u64,
    group_day: impl Fn(Vec<f32>) -> Option<f32>,
    group_range: impl Fn(Vec<f32>) -> Option<f32>,
) -> Vec<Vec<(NaiveDate, f32)>> {
    let mut date_map: BTreeMap<&NaiveDate, Vec<f32>> = BTreeMap::new();

    for (date, value) in data {
        date_map.entry(date).or_default().push(*value);
    }

    let mut grouped: BTreeMap<&NaiveDate, f32> = BTreeMap::new();

    for (date, values) in date_map {
        if let Some(result) = group_day(values) {
            grouped.insert(date, result);
        }
    }

    interval
        .first
        .iter_days()
        .take_while(|d| *d <= interval.last)
        .fold(
            vec![vec![]],
            |mut result: Vec<Vec<(NaiveDate, f32)>>, center| {
                let value = group_range(
                    center
                        .checked_sub_days(Days::new(radius))
                        .unwrap_or(center)
                        .iter_days()
                        .take_while(|d| {
                            *d <= interval.last
                                && *d
                                    <= center.checked_add_days(Days::new(radius)).unwrap_or(center)
                        })
                        .filter_map(|d| grouped.get(&d))
                        .copied()
                        .collect::<Vec<_>>(),
                );
                if let Some(last) = result.last_mut() {
                    match value {
                        Some(v) => {
                            last.push((center, v));
                        }
                        None => {
                            if !last.is_empty() {
                                result.push(vec![]);
                            }
                        }
                    }
                }
                result
            },
        )
        .into_iter()
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
}

/// Calculate a series of moving totals from a given series of (date, value) pairs.
///
/// The radius argument determines the number of days to include into the calculated
/// total before and after each value within the interval.
///
/// Multiple values for the same date will be summed up.
///
/// An empty result vector may be returned if there is no data within the interval.
pub fn centered_moving_total(
    data: &Vec<(NaiveDate, f32)>,
    interval: &Interval,
    radius: u64,
) -> Vec<(NaiveDate, f32)> {
    centered_moving_grouping(
        data,
        interval,
        radius,
        |d| Some(d.iter().sum()),
        |d| Some(d.iter().sum()),
    )[0]
    .clone()
}

/// Calculate a series of moving averages from a given series of (date, value) pairs.
///
/// The radius argument determines the number of days to include into the calculated
/// average before and after each value within the interval.
///
/// Multiple values for the same date will be averaged.
///
/// An empty result vector may be returned if there is no data within the interval.
/// Multiple result vectors may be returned in cases where there are gaps of more than
/// 2*radius+1 days in the input data within the interval.
pub fn centered_moving_average(
    data: &Vec<(NaiveDate, f32)>,
    interval: &Interval,
    radius: u64,
) -> Vec<Vec<(NaiveDate, f32)>> {
    #[allow(clippy::cast_precision_loss)]
    centered_moving_grouping(
        data,
        interval,
        radius,
        |d| {
            if d.is_empty() {
                None
            } else {
                Some(d.iter().sum::<f32>() / d.len() as f32)
            }
        },
        |d| {
            if d.is_empty() {
                None
            } else {
                Some(d.iter().sum::<f32>() / d.len() as f32)
            }
        },
    )
}

/// Calculate a series of moving averages from a given series of (date, value) pairs.
///
/// The data argument must have only one value per day.
///
/// The radius argument determines the number of values to include into the calculated
/// average before and after each value.
pub fn value_based_centered_moving_average(
    data: &[(NaiveDate, f32)],
    radius: usize,
) -> Vec<(NaiveDate, f32)> {
    let window = 2 * radius + 1;
    let length = data.len();
    data.iter()
        .enumerate()
        .map(|(i, (date, _))| {
            #[allow(clippy::cast_precision_loss)]
            let avg = data[i.saturating_sub(window / 2)..=(i + window / 2).min(length - 1)]
                .iter()
                .map(|(_, value)| value)
                .sum::<f32>()
                / window
                    .min(length - (i.saturating_sub(window / 2)))
                    .min(i + window / 2 + 1) as f32;
            (*date, avg)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn quartile_one() {
        assert_eq!(quartile(&[], Quartile::Q1), Duration::days(0));
        assert_eq!(
            quartile(&[Duration::days(2)], Quartile::Q1),
            Duration::days(0)
        );
        assert_eq!(
            quartile(&[Duration::days(4), Duration::days(12)], Quartile::Q1),
            Duration::days(4)
        );
        assert_eq!(
            quartile(
                &[Duration::days(2), Duration::days(4), Duration::days(6)],
                Quartile::Q1
            ),
            Duration::days(2)
        );
        assert_eq!(
            quartile(
                &[
                    Duration::days(2),
                    Duration::days(4),
                    Duration::days(6),
                    Duration::days(8)
                ],
                Quartile::Q1
            ),
            Duration::days(3)
        );
        assert_eq!(
            quartile(
                &[
                    Duration::days(2),
                    Duration::days(4),
                    Duration::days(5),
                    Duration::days(6),
                    Duration::days(8)
                ],
                Quartile::Q1
            ),
            Duration::days(3)
        );
        assert_eq!(
            quartile(
                &[
                    Duration::days(2),
                    Duration::days(4),
                    Duration::days(5),
                    Duration::days(6),
                    Duration::days(7),
                    Duration::days(8)
                ],
                Quartile::Q1
            ),
            Duration::days(4)
        );
    }

    #[test]
    fn quartile_two() {
        assert_eq!(quartile(&[], Quartile::Q2), Duration::days(0));
        assert_eq!(
            quartile(&[Duration::days(2)], Quartile::Q2),
            Duration::days(2)
        );
        assert_eq!(
            quartile(&[Duration::days(4), Duration::days(12)], Quartile::Q2),
            Duration::days(8)
        );
        assert_eq!(
            quartile(
                &[Duration::days(2), Duration::days(4), Duration::days(6)],
                Quartile::Q2
            ),
            Duration::days(4)
        );
    }

    #[test]
    fn quartile_three() {
        assert_eq!(quartile(&[], Quartile::Q3), Duration::days(0));
        assert_eq!(
            quartile(&[Duration::days(2)], Quartile::Q3),
            Duration::days(0)
        );
        assert_eq!(
            quartile(
                &[Duration::days(2), Duration::days(4), Duration::days(6)],
                Quartile::Q3
            ),
            Duration::days(6)
        );
        assert_eq!(
            quartile(
                &[
                    Duration::days(2),
                    Duration::days(4),
                    Duration::days(6),
                    Duration::days(8)
                ],
                Quartile::Q3
            ),
            Duration::days(7)
        );
        assert_eq!(
            quartile(
                &[
                    Duration::days(2),
                    Duration::days(4),
                    Duration::days(5),
                    Duration::days(6),
                    Duration::days(8)
                ],
                Quartile::Q3
            ),
            Duration::days(7)
        );
        assert_eq!(
            quartile(
                &[
                    Duration::days(2),
                    Duration::days(3),
                    Duration::days(4),
                    Duration::days(5),
                    Duration::days(6),
                    Duration::days(8)
                ],
                Quartile::Q3
            ),
            Duration::days(6)
        );
    }

    #[rstest]
    #[case::empty_series(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[],
        vec![]
    )]
    #[case::value_outside_interval(
        (2020, 3, 3),
        (2020, 3, 5),
        0,
        &[(2020, 2, 3, 1.0)],
        vec![]
    )]
    #[case::zero_radius_single_value(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[(2020, 2, 3, 1.0)],
        vec![vec![(2020, 2, 3, 1.0)]]
    )]
    #[case::zero_radius_multiple_days(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 1.0), (2020, 2, 5, 1.0)],
        vec![vec![(2020, 2, 3, 1.0), (2020, 2, 4, 1.0), (2020, 2, 5, 1.0)]]
    )]
    #[case::zero_radius_multiple_values_per_day(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 1.0), (2020, 2, 5, 1.0), (2020, 2, 3, 3.0)],
        vec![vec![(2020, 2, 3, 2.0), (2020, 2, 4, 1.0), (2020, 2, 5, 1.0)]]
    )]
    #[case::nonzero_radius_multiple_days(
        (2020, 2, 3),
        (2020, 2, 5),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0)],
        vec![vec![(2020, 2, 3, 1.5), (2020, 2, 4, 2.0), (2020, 2, 5, 2.5)]]
    )]
    #[case::nonzero_radius_missing_day(
        (2020, 2, 2),
        (2020, 2, 6),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0)],
        vec![vec![(2020, 2, 2, 1.0), (2020, 2, 3, 1.5), (2020, 2, 4, 2.0), (2020, 2, 5, 2.5), (2020, 2, 6, 3.0)]]
    )]
    #[case::nonzero_radius_with_gap_1(
        (2020, 2, 3),
        (2020, 2, 7),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 7, 1.0)],
        vec![vec![(2020, 2, 3, 1.0), (2020, 2, 4, 1.0)], vec![(2020, 2, 6, 1.0), (2020, 2, 7, 1.0)]]
    )]
    #[case::nonzero_radius_with_gap_2(
        (2020, 2, 3),
        (2020, 2, 9),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 9, 1.0)],
        vec![vec![(2020, 2, 3, 1.0), (2020, 2, 4, 1.0)], vec![(2020, 2, 8, 1.0), (2020, 2, 9, 1.0)]]
    )]
    fn centered_moving_average(
        #[case] start: (i32, u32, u32),
        #[case] end: (i32, u32, u32),
        #[case] radius: u64,
        #[case] input: &[(i32, u32, u32, f32)],
        #[case] expected: Vec<Vec<(i32, u32, u32, f32)>>,
    ) {
        assert_eq!(
            super::centered_moving_average(
                &input
                    .iter()
                    .map(|(y, m, d, v)| (NaiveDate::from_ymd_opt(*y, *m, *d).unwrap(), *v))
                    .collect::<Vec<_>>(),
                &Interval {
                    first: NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap(),
                    last: NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap(),
                },
                radius,
            ),
            expected
                .iter()
                .map(|v| v
                    .iter()
                    .map(|(y, m, d, v)| (NaiveDate::from_ymd_opt(*y, *m, *d).unwrap(), *v))
                    .collect::<Vec<_>>())
                .collect::<Vec<_>>(),
        );
    }

    #[rstest]
    #[case::empty_series(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[],
        &[(2020, 2, 3, 0.0), (2020, 2, 4, 0.0), (2020, 2, 5, 0.0)],
    )]
    #[case::value_outside_interval(
        (2020, 3, 3),
        (2020, 3, 5),
        0,
        &[(2020, 2, 3, 1.0)],
        &[(2020, 3, 3, 0.0), (2020, 3, 4, 0.0), (2020, 3, 5, 0.0)],
    )]
    #[case::zero_radius_single_day(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[(2020, 2, 3, 1.0)],
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 0.0), (2020, 2, 5, 0.0)],
    )]
    #[case::zero_radius_multiple_days(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0)],
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0)],
    )]
    #[case::zero_radius_multiple_values_per_day(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0), (2020, 2, 3, 1.0)],
        &[(2020, 2, 3, 2.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0)],
    )]
    #[case::nonzero_radius_multiple_days(
        (2020, 2, 3),
        (2020, 2, 5),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0)],
        &[(2020, 2, 3, 3.0), (2020, 2, 4, 6.0), (2020, 2, 5, 5.0)],
    )]
    #[case::nonzero_radius_missing_day(
        (2020, 2, 2),
        (2020, 2, 6),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0)],
        &[(2020, 2, 2, 1.0), (2020, 2, 3, 3.0), (2020, 2, 4, 6.0), (2020, 2, 5, 5.0), (2020, 2, 6, 3.0)],
    )]
    #[case::nonzero_radius_multiple_missing_days_1(
        (2020, 2, 3),
        (2020, 2, 7),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 7, 1.0)],
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 1.0), (2020, 2, 5, 0.0), (2020, 2, 6, 1.0), (2020, 2, 7, 1.0)],
    )]
    #[case::nonzero_radius_multiple_missing_days_2(
        (2020, 2, 3),
        (2020, 2, 9),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 9, 1.0)],
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 1.0), (2020, 2, 5, 0.0), (2020, 2, 6, 0.0), (2020, 2, 7, 0.0), (2020, 2, 8, 1.0), (2020, 2, 9, 1.0)]
    )]
    fn centered_moving_total(
        #[case] start: (i32, u32, u32),
        #[case] end: (i32, u32, u32),
        #[case] radius: u64,
        #[case] input: &[(i32, u32, u32, f32)],
        #[case] expected: &[(i32, u32, u32, f32)],
    ) {
        assert_eq!(
            super::centered_moving_total(
                &input
                    .iter()
                    .map(|(y, m, d, v)| (NaiveDate::from_ymd_opt(*y, *m, *d).unwrap(), *v))
                    .collect::<Vec<_>>(),
                &Interval {
                    first: NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap(),
                    last: NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap(),
                },
                radius,
            ),
            expected
                .iter()
                .map(|(y, m, d, v)| (NaiveDate::from_ymd_opt(*y, *m, *d).unwrap(), *v))
                .collect::<Vec<_>>(),
        );
    }

    #[rstest]
    #[case::empty_series(
        0,
        &[],
        vec![]
    )]
    #[case::zero_radius_single_value(
        0,
        &[(2020, 2, 3, 1.0)],
        vec![(2020, 2, 3, 1.0)]
    )]
    #[case::zero_radius_multiple_days(
        0,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 1.0), (2020, 2, 5, 1.0)],
        vec![(2020, 2, 3, 1.0), (2020, 2, 4, 1.0), (2020, 2, 5, 1.0)]
    )]
    #[case::nonzero_radius_multiple_days(
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 5, 2.0), (2020, 2, 7, 3.0)],
        vec![(2020, 2, 3, 1.5), (2020, 2, 5, 2.0), (2020, 2, 7, 2.5)]
    )]
    #[case::nonzero_radius_multiple_days(
        2,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0), (2020, 2, 6, 4.0), (2020, 2, 6, 5.0)],
        vec![(2020, 2, 3, 2.0), (2020, 2, 4, 2.5), (2020, 2, 5, 3.0), (2020, 2, 6, 3.5), (2020, 2, 6, 4.0)]
    )]
    fn test_value_based_centered_moving_average(
        #[case] radius: usize,
        #[case] input: &[(i32, u32, u32, f32)],
        #[case] expected: Vec<(i32, u32, u32, f32)>,
    ) {
        assert_eq!(
            super::value_based_centered_moving_average(
                &input
                    .iter()
                    .map(|(y, m, d, v)| (NaiveDate::from_ymd_opt(*y, *m, *d).unwrap(), *v))
                    .collect::<Vec<_>>(),
                radius,
            ),
            expected
                .iter()
                .map(|(y, m, d, v)| (NaiveDate::from_ymd_opt(*y, *m, *d).unwrap(), *v))
                .collect::<Vec<_>>()
        );
    }
}
