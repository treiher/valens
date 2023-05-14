use std::collections::BTreeMap;

use chrono::{prelude::*, Duration};
use plotters::prelude::*;
use seed::{prelude::*, *};

pub const ENTER_KEY: u32 = 13;

pub const COLOR_BODY_WEIGHT: usize = 1;
pub const COLOR_AVG_BODY_WEIGHT: usize = 2;
pub const COLOR_BODY_FAT_JP3: usize = 4;
pub const COLOR_BODY_FAT_JP7: usize = 0;
pub const COLOR_PERIOD_INTENSITY: usize = 0;
pub const COLOR_LOAD: usize = 1;
pub const COLOR_INTENSITY: usize = 0;
pub const COLOR_SET_VOLUME: usize = 3;
pub const COLOR_VOLUME_LOAD: usize = 6;
pub const COLOR_TUT: usize = 2;
pub const COLOR_REPS: usize = 3;
pub const COLOR_REPS_RIR: usize = 4;
pub const COLOR_WEIGHT: usize = 8;
pub const COLOR_TIME: usize = 5;

pub struct Interval {
    pub first: NaiveDate,
    pub last: NaiveDate,
}

pub fn init_interval(dates: &[NaiveDate], show_all: bool) -> Interval {
    let today = Local::now().date_naive();
    let mut first = dates.iter().copied().min().unwrap_or(today);
    let mut last = dates.iter().copied().max().unwrap_or(today);

    if not(show_all) && last >= today - Duration::days(30) {
        first = today - Duration::days(30);
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
    let duration = (current.last - current.first) + Duration::days(2);
    let intervals = [
        (
            "ALL",
            all.first,
            all.last,
            all.first == current.first && all.last == current.last,
        ),
        (
            "1Y",
            today - Duration::days(365),
            today,
            current.last == today && duration == Duration::days(367),
        ),
        (
            "6M",
            today - Duration::days(182),
            today,
            current.last == today && duration == Duration::days(184),
        ),
        (
            "3M",
            today - Duration::days(91),
            today,
            current.last == today && duration == Duration::days(93),
        ),
        (
            "1M",
            today - Duration::days(30),
            today,
            current.last == today && duration == Duration::days(32),
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

pub fn automatic_icon<Ms>() -> Node<Ms> {
    span![
        C!["fa-stack"],
        attrs! {
            At::Style => "vertical-align: top;",
        },
        i![C!["fas fa-circle fa-stack-1x"]],
        i![C!["fas fa-a fa-inverse fa-stack-1x"]]
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
    let mut month: NaiveDate = Default::default();
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
                                            St::BackgroundColor => "#FFFFFF"
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
    chart: Result<String, Box<dyn std::error::Error>>,
) -> Node<Ms> {
    div![
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
        raw![&chart.unwrap_or_else(|err| {
            error!("failed to plot chart:", err);
            String::new()
        })],
    ]
}

pub fn plot_line_chart(
    data: &[(Vec<(NaiveDate, f32)>, usize)],
    x_min: NaiveDate,
    x_max: NaiveDate,
    y_min_opt: Option<f32>,
    y_max_opt: Option<f32>,
) -> Result<String, Box<dyn std::error::Error>> {
    let (y_min, y_max, y_margin) = determine_y_bounds(
        data.iter()
            .flat_map(|(s, _)| s.iter().map(|(_, y)| *y))
            .collect::<Vec<_>>(),
        y_min_opt,
        y_max_opt,
    );

    let mut result = String::new();

    {
        let root = SVGBackend::with_string(&mut result, (chart_width(), 200)).into_drawing_area();

        root.fill(&WHITE)?;

        let mut chart_builder = ChartBuilder::on(&root);
        chart_builder
            .margin(10f32)
            .x_label_area_size(30f32)
            .y_label_area_size(40f32);

        let mut chart = chart_builder.build_cartesian_2d(
            x_min..x_max,
            f32::max(0., y_min - y_margin)..y_max + y_margin,
        )?;

        chart
            .configure_mesh()
            .disable_x_mesh()
            .set_all_tick_mark_size(3u32)
            .axis_style(BLACK.mix(0.3))
            .light_line_style(WHITE.mix(0.0))
            .x_labels(2)
            .y_labels(6)
            .draw()?;

        for (series, color_idx) in data {
            let mut series = series.iter().collect::<Vec<_>>();
            series.sort_by_key(|e| e.0);
            let color = Palette99::pick(*color_idx).mix(0.9);

            chart.draw_series(LineSeries::new(
                series.iter().map(|(x, y)| (*x, *y)),
                color.stroke_width(2),
            ))?;

            chart.draw_series(
                series
                    .iter()
                    .map(|(x, y)| Circle::new((*x, *y), 2, color.filled())),
            )?;
        }

        root.present()?;
    }

    Ok(result)
}

pub fn plot_dual_line_chart(
    data: &[(Vec<(NaiveDate, f32)>, usize)],
    secondary_data: &[(Vec<(NaiveDate, f32)>, usize)],
    x_min: NaiveDate,
    x_max: NaiveDate,
) -> Result<String, Box<dyn std::error::Error>> {
    let (y_min, y_max, y_margin) = determine_y_bounds(
        data.iter()
            .flat_map(|(s, _)| s.iter().map(|(_, y)| *y))
            .collect::<Vec<_>>(),
        None,
        None,
    );
    let (y2_min, y2_max, y2_margin) = determine_y_bounds(
        secondary_data
            .iter()
            .flat_map(|(s, _)| s.iter().map(|(_, y)| *y))
            .collect::<Vec<_>>(),
        None,
        None,
    );

    let mut result = String::new();

    {
        let root = SVGBackend::with_string(&mut result, (chart_width(), 200)).into_drawing_area();

        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .margin(10f32)
            .x_label_area_size(30f32)
            .y_label_area_size(40f32)
            .right_y_label_area_size(40f32)
            .build_cartesian_2d(x_min..x_max, y_min - y_margin..y_max + y_margin)?
            .set_secondary_coord(x_min..x_max, y2_min - y2_margin..y2_max + y2_margin);

        chart
            .configure_mesh()
            .disable_x_mesh()
            .set_all_tick_mark_size(3u32)
            .axis_style(BLACK.mix(0.3))
            .light_line_style(WHITE.mix(0.0))
            .x_labels(2)
            .y_labels(6)
            .draw()?;

        chart
            .configure_secondary_axes()
            .set_all_tick_mark_size(3u32)
            .axis_style(BLACK.mix(0.3))
            .draw()?;

        for (series, color_idx) in secondary_data {
            let mut series = series.iter().collect::<Vec<_>>();
            series.sort_by_key(|e| e.0);
            let color = Palette99::pick(*color_idx).mix(0.9);

            chart.draw_secondary_series(LineSeries::new(
                series.iter().map(|(x, y)| (*x, *y)),
                color.stroke_width(2),
            ))?;

            chart.draw_secondary_series(
                series
                    .iter()
                    .map(|(x, y)| Circle::new((*x, *y), 2, color.filled())),
            )?;
        }

        for (series, color_idx) in data {
            let mut series = series.iter().collect::<Vec<_>>();
            series.sort_by_key(|e| e.0);
            let color = Palette99::pick(*color_idx).mix(0.9);

            chart.draw_series(LineSeries::new(
                series.iter().map(|(x, y)| (*x, *y)),
                color.stroke_width(2),
            ))?;

            chart.draw_series(
                series
                    .iter()
                    .map(|(x, y)| Circle::new((*x, *y), 2, color.filled())),
            )?;
        }

        root.present()?;
    }

    Ok(result)
}

pub fn plot_bar_chart(
    data: &[(Vec<(NaiveDate, f32)>, usize)],
    secondary_data: &[(Vec<(NaiveDate, f32)>, usize)],
    x_min: NaiveDate,
    x_max: NaiveDate,
    y_min_opt: Option<f32>,
    y_max_opt: Option<f32>,
) -> Result<String, Box<dyn std::error::Error>> {
    let (y_min, y_max, _) = determine_y_bounds(
        data.iter()
            .flat_map(|(s, _)| s.iter().map(|(_, y)| *y))
            .collect::<Vec<_>>(),
        y_min_opt,
        y_max_opt,
    );
    let y_margin = 0.;
    let (y2_min, y2_max, y2_margin) = determine_y_bounds(
        secondary_data
            .iter()
            .flat_map(|(s, _)| s.iter().map(|(_, y)| *y))
            .collect::<Vec<_>>(),
        None,
        None,
    );

    let mut result = String::new();

    {
        let root = SVGBackend::with_string(&mut result, (chart_width(), 200)).into_drawing_area();

        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .margin(10f32)
            .x_label_area_size(30f32)
            .y_label_area_size(40f32)
            .right_y_label_area_size(30f32)
            .build_cartesian_2d(
                (x_min..x_max).into_segmented(),
                y_min - y_margin..y_max + y_margin,
            )?
            .set_secondary_coord(x_min..x_max, y2_min - y2_margin..y2_max + y2_margin);

        chart
            .configure_mesh()
            .disable_x_mesh()
            .set_all_tick_mark_size(3u32)
            .axis_style(BLACK.mix(0.3))
            .light_line_style(WHITE.mix(0.0))
            .x_labels(2)
            .y_labels(6)
            .draw()?;

        chart
            .configure_secondary_axes()
            .set_all_tick_mark_size(3u32)
            .axis_style(BLACK.mix(0.3))
            .draw()?;

        for (series, color_idx) in data {
            let mut series = series.iter().collect::<Vec<_>>();
            series.sort_by_key(|e| e.0);
            let color = Palette99::pick(*color_idx).mix(0.9).filled();
            let histogram = Histogram::vertical(&chart)
                .style(color)
                .margin(0) // https://github.com/plotters-rs/plotters/issues/300
                .data(series.iter().map(|(x, y)| (*x, *y)));

            chart.draw_series(histogram)?;
        }

        for (series, color_idx) in secondary_data {
            let mut series = series.iter().collect::<Vec<_>>();
            series.sort_by_key(|e| e.0);
            let color = Palette99::pick(*color_idx).mix(0.9);

            chart.draw_secondary_series(LineSeries::new(
                series.iter().map(|(x, y)| (*x, *y)),
                color.stroke_width(2),
            ))?;

            chart.draw_secondary_series(
                series
                    .iter()
                    .map(|(x, y)| Circle::new((*x, *y), 2, color.filled())),
            )?;
        }

        root.present()?;
    }

    Ok(result)
}

fn determine_y_bounds(
    y: Vec<f32>,
    y_min_opt: Option<f32>,
    y_max_opt: Option<f32>,
) -> (f32, f32, f32) {
    let y_min = f32::min(
        y_min_opt.unwrap_or(f32::MAX),
        y.clone().into_iter().reduce(f32::min).unwrap_or(0.),
    );
    let y_max = f32::max(
        y_max_opt.unwrap_or(0.),
        y.into_iter().reduce(f32::max).unwrap_or(0.),
    );
    let y_margin = if y_min != y_max || y_min == 0. {
        (y_max - y_min) * 0.1
    } else {
        0.1
    };

    (y_min, y_max, y_margin)
}

fn chart_width() -> u32 {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
