use std::{
    borrow::BorrowMut,
    collections::{BTreeMap, HashMap},
};

use chrono::{prelude::*, Duration};
use plotters::prelude::*;
use seed::{prelude::*, *};

use crate::{domain, ui};

pub const ENTER_KEY: u32 = 13;

pub const COLOR_BODY_WEIGHT: usize = 1;
pub const COLOR_AVG_BODY_WEIGHT: usize = 1;
pub const COLOR_BODY_FAT_JP3: usize = 4;
pub const COLOR_BODY_FAT_JP7: usize = 0;
pub const COLOR_PERIOD_INTENSITY: usize = 0;
pub const COLOR_LOAD: usize = 1;
pub const COLOR_LONG_TERM_LOAD: usize = 1;
pub const COLOR_RPE: usize = 0;
pub const COLOR_SET_VOLUME: usize = 3;
pub const COLOR_VOLUME_LOAD: usize = 6;
pub const COLOR_TUT: usize = 2;
pub const COLOR_REPS: usize = 4;
pub const COLOR_REPS_RIR: usize = 4;
pub const COLOR_WEIGHT: usize = 8;
pub const COLOR_TIME: usize = 5;
pub const COLOR_1RM: usize = 7;

pub const OPACITY_LINE: f64 = 0.9;
pub const OPACITY_AREA: f64 = 0.3;

pub const WIDTH_LINE: u32 = 2;

pub const FONT: (&str, u32) = ("Roboto", 11);

#[derive(Clone)]
pub enum PlotType {
    Circle(usize, f64, u32),
    Line(usize, f64, u32),
    Histogram(usize, f64),
    Area(usize, f64),
}

pub fn plot_line(color: usize) -> Vec<PlotType> {
    vec![PlotType::Line(color, OPACITY_LINE, WIDTH_LINE)]
}

pub fn plot_area(color: usize) -> Vec<PlotType> {
    vec![PlotType::Area(color, OPACITY_AREA)]
}

pub fn plot_area_with_border(line_color: usize, area_color: usize) -> Vec<PlotType> {
    vec![
        PlotType::Area(area_color, OPACITY_AREA),
        PlotType::Line(line_color, OPACITY_LINE, WIDTH_LINE),
    ]
}

#[derive(Default, Clone, Copy)]
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

#[derive(Clone)]
pub struct PlotData {
    pub values_high: Vec<(NaiveDate, f32)>,
    pub values_low: Option<Vec<(NaiveDate, f32)>>,
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

/// Plot data onto a chart.
///
/// The x domain of the chart is configured by the interval parameter. The
/// theme to be used is determined by the theme parameter.
///
/// Multiple, independent series can be plotted at once. Every `PlotData`
/// element of the data parameter contains one or two such series to be
/// plotted with the same parameters. The `values_high` element contains
/// the first series, the optional `values_low` a possible second series.
///
/// The `plots` element of `PlotData` is a list of plots to perform on
/// each series:
///
///   - Circle: plot a circle with the given color and size for each element
///   - Line: plot the series as a line with the given color and thickness
///   - Histogram: plot the series as a histogram with the given color
///   - Area: plot the series as area or band plot (see details below)
///
/// For the `Area` plot type, values are treated specially. If `values_low`
/// is None, the area below the series in `values_high` is filled with the
/// given color at the given alpha value. If `values_low` contains a series,
/// a band chart between the two series is plotted instead. To ensure proper
/// rendering, the low and high series of a band plot should start and end
/// on the same date.
///
/// The `params` element of `PlotData` configures the y domain and determines
/// whether the series are plotted for the primary or secondary axis of the
/// chart. If `data` contains no series for the secondary axis, the secondary
/// axis is omitted.
///
/// The plotting order (and thus the stacking of plots) is as follows:
///   - Every series in `data` is plotted in order
///   - For every series all plots are plotted in order
///   - For every plot, `values_low` is plotted before `values_high`
///     (except for `AreaPlot`, where both are plotted together)
///
pub fn plot_chart(
    data: &[PlotData],
    interval: &domain::Interval,
    theme: &ui::Theme,
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
                interval.first..interval.last,
                primary_bounds.min_with_margin()..primary_bounds.max_with_margin(),
            )?
            .set_secondary_coord(
                interval.first..interval.last,
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
            .label_style(TextStyle::from(FONT.into_font()).color(&color))
            .x_labels(2)
            .y_labels(6)
            .draw()?;

        if secondary_bounds.is_some() {
            chart
                .configure_secondary_axes()
                .set_all_tick_mark_size(3u32)
                .axis_style(color.mix(0.3))
                .label_style(TextStyle::from(FONT.into_font()).color(&color))
                .draw()?;
        }

        for plot_data in data {
            let mut values_high = Some(plot_data.values_high.clone());
            if let Some(values) = values_high.as_mut() {
                values.sort_by_key(|e| e.0);
            }
            let mut values_low = plot_data.values_low.clone();
            if let Some(values) = values_low.as_mut() {
                values.sort_by_key(|e| e.0);
                values.reverse();
            }

            for plot in &plot_data.plots {
                match *plot {
                    PlotType::Circle(color, opacity, size) => {
                        [values_low.as_ref(), values_high.as_ref()]
                            .into_iter()
                            .flatten()
                            .try_for_each(
                                |values| -> Result<(), DrawingAreaErrorKind<std::io::Error>> {
                                    let data = values.iter().map(|(x, y)| {
                                        Circle::new(
                                            (*x, *y),
                                            size,
                                            Palette99::pick(color).mix(opacity).filled(),
                                        )
                                    });
                                    if plot_data.params.secondary {
                                        chart.draw_secondary_series(data)?;
                                    } else {
                                        chart.draw_series(data)?;
                                    }
                                    Ok(())
                                },
                            )?;
                    }
                    PlotType::Line(color, opacity, size) => {
                        [values_low.as_ref(), values_high.as_ref()]
                            .into_iter()
                            .flatten()
                            .try_for_each(
                                |values| -> Result<(), DrawingAreaErrorKind<std::io::Error>> {
                                    let data = LineSeries::new(
                                        values.iter().map(|(x, y)| (*x, *y)),
                                        Palette99::pick(color).mix(opacity).stroke_width(size),
                                    );
                                    if plot_data.params.secondary {
                                        chart.draw_secondary_series(data)?;
                                    } else {
                                        chart.draw_series(data)?;
                                    }
                                    Ok(())
                                },
                            )?;
                    }
                    PlotType::Histogram(color, opacity) => {
                        [values_low.as_ref(), values_high.as_ref()]
                            .into_iter()
                            .flatten()
                            .try_for_each(
                                |values| -> Result<(), DrawingAreaErrorKind<std::io::Error>> {
                                    let data = Histogram::vertical(&chart)
                                        .style(Palette99::pick(color).mix(opacity).filled())
                                        .margin(0) // https://github.com/plotters-rs/plotters/issues/300
                                        .data(values.iter().map(|(x, y)| (*x, *y)));

                                    if plot_data.params.secondary {
                                        chart.draw_secondary_series(data)?;
                                    } else {
                                        chart.draw_series(data)?;
                                    }
                                    Ok(())
                                },
                            )?;
                    }
                    PlotType::Area(color, opacity) => {
                        if values_low.is_none() {
                            let data = AreaSeries::new(
                                values_high
                                    .as_ref()
                                    .map(|values| {
                                        values.iter().map(|(x, y)| (*x, *y)).collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default(),
                                0.0,
                                Palette99::pick(color).mix(opacity),
                            );
                            if plot_data.params.secondary {
                                chart.draw_secondary_series(data)?;
                            } else {
                                chart.draw_series(data)?;
                            }
                        } else {
                            let data = Polygon::new(
                                values_high
                                    .as_ref()
                                    .map(|values| {
                                        values
                                            .iter()
                                            .chain(values_low.iter().flatten())
                                            .map(|(x, y)| (*x, *y))
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default(),
                                Palette99::pick(color).mix(opacity),
                            );
                            if plot_data.params.secondary {
                                chart.draw_secondary_series(std::iter::once(data))?;
                            } else {
                                chart.draw_series(std::iter::once(data))?;
                            }
                        }
                    }
                };
            }
        }

        root.present()?;
    }

    Ok(Some(result))
}

pub fn plot_min_avg_max(
    data: &Vec<(NaiveDate, f32)>,
    interval: &domain::Interval,
    params: PlotParams,
    color: usize,
    theme: &ui::Theme,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut date_map: BTreeMap<&NaiveDate, Vec<f32>> = BTreeMap::new();

    for (date, value) in data {
        date_map.entry(date).or_default().push(*value);
    }

    let mut values_min: Vec<(NaiveDate, f32)> = vec![];
    let mut values_avg: Vec<(NaiveDate, f32)> = vec![];
    let mut values_max: Vec<(NaiveDate, f32)> = vec![];

    #[allow(clippy::cast_precision_loss)]
    for (date, min, avg, max) in date_map
        .into_iter()
        .skip_while(|(d, _)| **d < interval.first)
        .take_while(|(d, _)| **d <= interval.last)
        .map(|(date, values)| {
            (
                *date,
                values
                    .iter()
                    .fold(f32::MAX, |min, &val| if val < min { val } else { min }),
                values.iter().sum::<f32>() / values.len() as f32,
                values
                    .iter()
                    .fold(f32::MIN, |max, &val| if val > max { val } else { max }),
            )
        })
    {
        values_min.push((date, min));
        values_avg.push((date, avg));
        values_max.push((date, max));
    }

    plot_chart(
        &[
            PlotData {
                values_high: values_min,
                values_low: Some(values_max),
                plots: plot_area(color),
                params,
            },
            PlotData {
                values_high: values_avg,
                values_low: None,
                plots: plot_line(color),
                params,
            },
        ],
        interval,
        theme,
    )
}

fn all_zeros(data: &[PlotData]) -> bool {
    data.iter()
        .map(|v| {
            v.values_high.iter().all(|(_, v)| *v == 0.0)
                && v.values_low
                    .as_ref()
                    .map_or(true, |v| v.iter().all(|(_, v)| *v == 0.0))
        })
        .reduce(|l, r| l && r)
        .unwrap_or(true)
}

fn colors(theme: &ui::Theme) -> (RGBColor, RGBColor) {
    let dark = RGBColor(20, 22, 26);
    match theme {
        ui::Theme::System | ui::Theme::Light => (dark, WHITE),
        ui::Theme::Dark => (WHITE, dark),
    }
}

fn determine_y_bounds(data: &[PlotData]) -> (Option<Bounds>, Option<Bounds>) {
    let mut primary_bounds: Option<Bounds> = None;
    let mut secondary_bounds: Option<Bounds> = None;

    for plot in data.iter().filter(|plot| !plot.values_high.is_empty()) {
        let min = plot
            .values_high
            .iter()
            .chain(plot.values_low.iter().flatten())
            .map(|(_, v)| *v)
            .fold(plot.params.y_min_opt.unwrap_or(f32::MAX), f32::min);
        let max = plot
            .values_high
            .iter()
            .chain(plot.values_low.iter().flatten())
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

pub fn view_element_with_tooltip<Ms>(
    element: Node<Ms>,
    tooltip: Node<Ms>,
    right_aligned: bool,
) -> Node<Ms> {
    div![
        C!["dropdown"],
        IF![right_aligned => C!["is-right"]],
        C!["is-hoverable"],
        div![
            C!["dropdown-trigger"],
            div![C!["control"], C!["is-clickable"], element]
        ],
        if let Node::Element(x) = tooltip {
            div![
                C!["dropdown-menu"],
                C!["has-no-min-width"],
                div![C!["dropdown-content"], div![C!["dropdown-item"], x]]
            ]
        } else {
            div![]
        }
    ]
}

pub fn view_element_with_description<Ms>(element: Node<Ms>, description: &str) -> Node<Ms> {
    view_element_with_tooltip(
        element,
        if description.is_empty() {
            Node::Empty
        } else {
            div![description]
        },
        false,
    )
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
