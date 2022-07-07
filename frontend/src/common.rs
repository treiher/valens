use chrono::{prelude::*, Duration};
use plotters::prelude::*;
use seed::{prelude::*, *};

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
    let today = Local::today().naive_local();
    let duration = (current.last - current.first) + Duration::days(2);
    let intervals = [
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

pub fn view_chart<Ms>(labels: &[(&str, usize)], chart: Vec<Node<Ms>>) -> Node<Ms> {
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
        chart,
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
            .axis_style(&BLACK.mix(0.3))
            .light_line_style(&WHITE.mix(0.0))
            .x_labels(2)
            .y_labels(6)
            .draw()?;

        for (series, color_idx) in data {
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
            .axis_style(&BLACK.mix(0.3))
            .light_line_style(&WHITE.mix(0.0))
            .x_labels(2)
            .y_labels(6)
            .draw()?;

        chart
            .configure_secondary_axes()
            .set_all_tick_mark_size(3u32)
            .axis_style(&BLACK.mix(0.3))
            .draw()?;

        for (series, color_idx) in secondary_data {
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
            .build_cartesian_2d(x_min..x_max, y_min - y_margin..y_max + y_margin)?
            .set_secondary_coord(x_min..x_max, y2_min - y2_margin..y2_max + y2_margin);

        chart
            .configure_mesh()
            .disable_x_mesh()
            .set_all_tick_mark_size(3u32)
            .axis_style(&BLACK.mix(0.3))
            .light_line_style(&WHITE.mix(0.0))
            .x_labels(2)
            .y_labels(6)
            .draw()?;

        chart
            .configure_secondary_axes()
            .set_all_tick_mark_size(3u32)
            .axis_style(&BLACK.mix(0.3))
            .draw()?;

        for (series, color_idx) in data {
            let color = Palette99::pick(*color_idx).mix(0.9).filled();
            let histogram = Histogram::vertical(&chart)
                .style(color)
                .margin(0) // https://github.com/plotters-rs/plotters/issues/300
                .data(series.iter().map(|(x, y)| (*x, *y)));
            chart.draw_series(histogram)?;
        }

        for (series, color_idx) in secondary_data {
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
