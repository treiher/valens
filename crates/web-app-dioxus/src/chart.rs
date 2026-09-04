//! Charts, the interval selection accompanying them and the calendar view.

use std::collections::BTreeMap;

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use dioxus::prelude::*;
use log::warn;
use web_sys::wasm_bindgen::{JsCast, closure::Closure};

use valens_domain as domain;
use valens_web_app as web_app;

use crate::{
    current_date::current_date,
    settings::Settings,
    ui::element::{Error, Icon, NoData},
};

#[component]
pub fn IntervalControl(
    current_interval: Signal<domain::Interval>,
    all: domain::Interval,
) -> Element {
    let current = *current_interval.read();
    let today = current_date();
    let intervals = interval_buttons(current, all, today);
    let previous = previous_interval(current, all);
    let is_left_disabled = current.first == previous.first;
    let next = next_interval(current, today);
    let is_right_disabled = current.last == next.last;

    rsx! {
        div {
            class: "field has-addons has-addons-centered",
            for IntervalButton { name, interval, is_active } in intervals {
                p {
                    class: "control",
                    a {
                        class: "button is-small",
                        class: if is_active { "is-link" },
                        "data-testid": "interval-{name}",
                        onclick: move |_| { *current_interval.write() = interval },
                        "{name}"
                    }
                }
            }
        }
        div {
            class: "is-flex is-align-items-center is-justify-content-center mb-4",
            button {
                class: "button is-small",
                disabled: is_left_disabled,
                onclick: move |_| { *current_interval.write() = previous },
                Icon { name: "chevron-left" }
            }
            span {
                class: "mx-3",
                "{current.first} – {current.last}"
            }
            button {
                class: "button is-small",
                disabled: is_right_disabled,
                onclick: move |_| { *current_interval.write() = next },
                Icon { name: "chevron-right" }
            }
        }
    }
}

/// One button of the interval control.
#[derive(Debug, PartialEq)]
struct IntervalButton {
    name: &'static str,
    interval: domain::Interval,
    is_active: bool,
}

/// The intervals the control offers, marking the one the current interval matches.
fn interval_buttons(
    current: domain::Interval,
    all: domain::Interval,
    today: NaiveDate,
) -> Vec<IntervalButton> {
    let duration = current.last - current.first + Duration::days(1);
    let fixed = |name, days: domain::DefaultInterval| IntervalButton {
        name,
        interval: domain::Interval {
            first: today - Duration::days(days as i64),
            last: today,
        },
        is_active: current.last == today && duration == Duration::days(days as i64 + 1),
    };
    let zoom_in = current.first + Duration::days(6) <= current.last - duration / 2;
    vec![
        fixed("1M", domain::DefaultInterval::_1M),
        fixed("3M", domain::DefaultInterval::_3M),
        fixed("6M", domain::DefaultInterval::_6M),
        fixed("1Y", domain::DefaultInterval::_1Y),
        IntervalButton {
            name: "NOW",
            interval: domain::Interval {
                first: all.first,
                last: today,
            },
            is_active: current.first == all.first && current.last == today,
        },
        IntervalButton {
            name: "ALL",
            interval: all,
            is_active: current.first == all.first && current.last == all.last,
        },
        IntervalButton {
            name: "+",
            interval: domain::Interval {
                first: if zoom_in {
                    current.first + duration / 4
                } else {
                    current.first
                },
                last: if zoom_in {
                    current.last - duration / 4
                } else {
                    current.first + Duration::days(6)
                },
            },
            is_active: false,
        },
        IntervalButton {
            name: "\u{2212}",
            interval: domain::Interval {
                first: if current.first - duration / 2 > all.first {
                    current.first - duration / 2
                } else {
                    all.first
                },
                last: if current.last + duration / 2 < today {
                    current.last + duration / 2
                } else {
                    today
                },
            },
            is_active: false,
        },
    ]
}

/// The interval a quarter of its duration earlier, bounded by the first day of `all`.
fn previous_interval(current: domain::Interval, all: domain::Interval) -> domain::Interval {
    let duration = current.last - current.first + Duration::days(1);
    if current.first - duration / 4 > all.first {
        domain::Interval {
            first: current.first - duration / 4,
            last: current.last - duration / 4,
        }
    } else {
        domain::Interval {
            first: all.first,
            last: all.first + duration - Duration::days(1),
        }
    }
}

/// The interval a quarter of its duration later, bounded by `today`.
fn next_interval(current: domain::Interval, today: NaiveDate) -> domain::Interval {
    let duration = current.last - current.first + Duration::days(1);
    if current.last + duration / 4 < today {
        domain::Interval {
            first: current.first + duration / 4,
            last: current.last + duration / 4,
        }
    } else {
        domain::Interval {
            first: today - duration + Duration::days(1),
            last: today,
        }
    }
}

#[component]
pub fn Chart(
    series: Vec<web_app::chart::LabeledSeries>,
    interval: domain::Interval,
    no_data_label: bool,
) -> Element {
    let settings = use_context::<Settings>();
    // The size of the SVG and the pixel positions of the samples depend on the window width at
    // the time of plotting
    let window_width = use_window_width();
    let labels: Vec<web_app::chart::ChartLabel> = series
        .iter()
        .map(web_app::chart::LabeledSeries::label)
        .collect();
    let data: Vec<web_app::chart::PlotData> =
        series.into_iter().rev().flat_map(|s| s.data).collect();
    let chart = web_app::chart::plot(&data, interval, settings.current_theme(), window_width)
        .map_err(|e| e.to_string());

    match chart {
        Ok(None) => {
            if no_data_label {
                rsx! {
                    NoData {}
                }
            } else {
                rsx! {}
            }
        }
        Ok(Some(result)) => {
            let web_app::chart::PlotResult { svg, series, area } = result;

            let mut marks: Vec<(NaiveDate, i32)> = series
                .iter()
                .flat_map(|s| s.high.iter().chain(s.low.iter().flatten()))
                .map(|sample| (sample.date, sample.x))
                .collect();
            marks.sort_by_key(|(_, x)| *x);
            marks.dedup_by_key(|(date, _)| *date);

            rsx! {
                div {
                    class: "container has-text-centered",
                    h1 {
                        class: "is-size-6 has-text-weight-bold",
                        {
                            labels
                                .iter()
                                .map(|label| {
                                    let color = web_app::chart::hex_color(label.color, label.opacity);
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
                        "data-testid": "chart",
                        style: "position: relative; display: inline-block; touch-action: pan-y;
                                -webkit-touch-callout: none; user-select: none; -webkit-user-select: none;",
                        div { dangerous_inner_html: svg }
                        ChartOverlay { marks, series, area }
                    }
                }
            }
        }
        Err(err) => rsx! { Error { message: "{err}" } },
    }
}

static WINDOW_WIDTH: GlobalSignal<u32> = Signal::global(window_width);
static RESIZE_LISTENER: std::sync::Once = std::sync::Once::new();

/// The window width a component renders for, overriding the width of the actual window.
#[derive(Clone, Copy)]
pub struct WindowWidth(pub u32);

/// Subscribes the component to changes of the window width and returns the current width.
fn use_window_width() -> u32 {
    if let Some(width) = try_consume_context::<WindowWidth>() {
        return width.0;
    }

    use_hook(|| {
        RESIZE_LISTENER.call_once(|| {
            let Some(window) = web_sys::window() else {
                warn!("failed to access window");
                return;
            };
            let closure = Closure::<dyn FnMut()>::new(|| {
                *WINDOW_WIDTH.write() = window_width();
            });
            if let Err(e) =
                window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
            {
                warn!("failed to register resize handler: {e:?}");
                return;
            }
            closure.forget();
        });
    });

    WINDOW_WIDTH()
}

fn window_width() -> u32 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    web_sys::window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|width| width.as_f64())
        .map_or(420, |width| width as u32)
}

/// Transparent layer over a chart that tracks the pointer and renders the
/// hover crosshair, markers and value tooltip.
///
/// Owns the hover state, so pointer movement re-renders only this overlay and
/// not the chart SVG.
#[component]
fn ChartOverlay(
    marks: Vec<(NaiveDate, i32)>,
    series: Vec<web_app::chart::SeriesSamples>,
    area: web_app::chart::PlotArea,
) -> Element {
    let mut hovered = use_signal(|| None::<i32>);
    let active = hovered.read().and_then(|x| nearest_mark(&marks, x));

    // A tap emits no pointer movement, so the position must also be tracked on pointer down
    let track_pointer = move |event: PointerEvent| {
        #[allow(clippy::cast_possible_truncation)]
        let x = event.element_coordinates().x as i32;
        if x < area.left || x > area.right {
            hovered.set(None);
        } else {
            hovered.set(Some(x));
        }
    };

    rsx! {
        div {
            "data-testid": "chart-overlay",
            style: "position: absolute; inset: 0;
                    -webkit-touch-callout: none; user-select: none; -webkit-user-select: none;",
            // A long press must not open the browser's context menu
            oncontextmenu: move |event| event.prevent_default(),
            onpointerdown: track_pointer,
            onpointermove: track_pointer,
            onpointerleave: move |_| hovered.set(None),
            onpointercancel: move |_| hovered.set(None),
            if let Some((date, x)) = active {
                ChartHover { series, area, date, x }
            }
        }
    }
}

#[component]
fn ChartHover(
    series: Vec<web_app::chart::SeriesSamples>,
    area: web_app::chart::PlotArea,
    date: NaiveDate,
    x: i32,
) -> Element {
    // One tooltip row per series; a band (both edges present) collapses to a
    // single `low – high` range row.
    let mut rows: Vec<(String, String)> = vec![];
    let mut dots: Vec<(String, i32, i32)> = vec![];
    for s in &series {
        let color = web_app::chart::hex_color(s.color, s.opacity);
        let high = s.high.iter().find(|p| p.date == date);
        let low = s
            .low
            .as_ref()
            .and_then(|l| l.iter().find(|p| p.date == date));
        for sample in high.into_iter().chain(low) {
            dots.push((color.clone(), sample.x, sample.y));
        }
        let label = match (high, low) {
            (Some(h), Some(l)) => {
                let (lo, hi) = (h.value.min(l.value), h.value.max(l.value));
                format!("{} – {}", format_value(lo), format_value(hi))
            }
            (Some(p), None) | (None, Some(p)) => format_value(p.value),
            (None, None) => continue,
        };
        rows.push((color, label));
    }
    // The plotted samples are built from the series in reverse order, so undo
    // that here to match the order of the legend.
    rows.reverse();

    let tooltip_transform = if x.saturating_mul(2) > area.left + area.right {
        "translateX(calc(-100% - 8px))"
    } else {
        "translateX(8px)"
    };

    rsx! {
        div {
            style: "position: absolute; pointer-events: none; width: 1px;
                    left: {x}px; top: {area.top}px; height: {area.bottom - area.top}px;
                    background: rgba(128, 128, 128, 0.8);",
        }
        for (color, dot_x, dot_y) in dots {
            div {
                style: "position: absolute; pointer-events: none; border-radius: 50%;
                        width: 7px; height: 7px; margin: -4px 0 0 -4px;
                        left: {dot_x}px; top: {dot_y}px; background: {color};",
            }
        }
        div {
            class: "box p-2 has-text-left",
            "data-testid": "chart-tooltip",
            style: "position: absolute; pointer-events: none; white-space: nowrap; z-index: 1;
                    left: {x}px; top: {area.top}px; transform: {tooltip_transform};",
            div { class: "is-size-7 has-text-centered has-text-weight-bold", "{date}" }
            for (color, label) in rows {
                div {
                    class: "icon-text is-size-7",
                    style: "flex-wrap: nowrap",
                    span {
                        class: "icon",
                        style: "color: {color}",
                        i { class: "fas fa-square" }
                    }
                    span { "{label}" }
                }
            }
        }
    }
}

fn nearest_mark(marks: &[(NaiveDate, i32)], x: i32) -> Option<(NaiveDate, i32)> {
    marks
        .iter()
        .min_by_key(|(_, mark_x)| (mark_x - x).abs())
        .copied()
}

fn format_value(value: f32) -> String {
    let formatted = format!("{value:.2}");
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    // Negative values that round to zero must not be displayed as `-0`
    if trimmed == "-0" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

#[component]
pub fn Calendar(entries: Vec<(NaiveDate, usize, f64)>, interval: domain::Interval) -> Element {
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
                                        "background-color:{web_app::chart::rgba_color(color, opacity)}"
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

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use pretty_assertions::assert_eq;

    use super::*;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, day).unwrap()
    }

    fn interval(first: u32, last: u32) -> domain::Interval {
        domain::Interval {
            first: date(first),
            last: date(last),
        }
    }

    #[test]
    fn interval_buttons_mark_the_matching_one_as_active() {
        let today = date(31);
        let all = interval(1, 31);

        let active = interval_buttons(
            domain::Interval {
                first: today - Duration::days(domain::DefaultInterval::_3M as i64),
                last: today,
            },
            all,
            today,
        )
        .into_iter()
        .filter(|button| button.is_active)
        .map(|button| button.name)
        .collect::<Vec<_>>();

        assert_eq!(active, vec!["3M"]);
    }

    #[test]
    fn interval_buttons_offer_the_whole_range_and_the_range_until_today() {
        let today = date(31);
        let all = interval(1, 20);

        let buttons = interval_buttons(interval(1, 20), all, today);

        assert_eq!(buttons[4].name, "NOW");
        assert_eq!(buttons[4].interval, interval(1, 31));
        assert_eq!(buttons[5].name, "ALL");
        assert_eq!(buttons[5].interval, all);
        assert!(buttons[5].is_active);
    }

    #[test]
    fn zooming_in_halves_the_interval_around_its_centre() {
        let buttons = interval_buttons(interval(1, 21), interval(1, 31), date(31));

        assert_eq!(buttons[6].name, "+");
        assert_eq!(buttons[6].interval, interval(6, 16));
    }

    #[test]
    fn zooming_in_stops_at_a_week() {
        let buttons = interval_buttons(interval(1, 7), interval(1, 31), date(31));

        assert_eq!(buttons[6].interval, interval(1, 7));
    }

    #[test]
    fn zooming_out_is_bounded_by_the_whole_range_and_today() {
        let buttons = interval_buttons(interval(10, 20), interval(1, 31), date(25));

        assert_eq!(buttons[7].interval, interval(5, 25));
    }

    #[test]
    fn the_previous_interval_shifts_by_a_quarter_of_the_duration() {
        assert_eq!(
            previous_interval(interval(13, 20), interval(1, 31)),
            interval(11, 18)
        );
    }

    #[test]
    fn the_previous_interval_stops_at_the_first_day_of_the_whole_range() {
        assert_eq!(
            previous_interval(interval(2, 9), interval(1, 31)),
            interval(1, 8)
        );
    }

    #[test]
    fn the_next_interval_shifts_by_a_quarter_of_the_duration() {
        assert_eq!(next_interval(interval(1, 8), date(31)), interval(3, 10));
    }

    #[test]
    fn the_next_interval_stops_at_today() {
        assert_eq!(next_interval(interval(20, 30), date(31)), interval(21, 31));
    }

    #[test]
    fn nearest_mark_picks_closest_pixel() {
        let marks = [(date(1), 0), (date(2), 50), (date(3), 100)];
        assert_eq!(nearest_mark(&marks, 60), Some((date(2), 50)));
        assert_eq!(nearest_mark(&marks, 90), Some((date(3), 100)));
        assert_eq!(nearest_mark(&marks, -20), Some((date(1), 0)));
    }

    #[test]
    fn nearest_mark_on_tie_keeps_earlier() {
        let marks = [(date(1), 0), (date(2), 100)];
        assert_eq!(nearest_mark(&marks, 50), Some((date(1), 0)));
    }

    #[test]
    fn nearest_mark_without_marks_is_none() {
        assert_eq!(nearest_mark(&[], 10), None);
    }

    #[test]
    fn format_value_trims_trailing_zeros() {
        assert_eq!(format_value(82.3), "82.3");
        assert_eq!(format_value(5.0), "5");
        assert_eq!(format_value(0.25), "0.25");
        assert_eq!(format_value(100.0), "100");
    }

    #[test]
    fn format_value_avoids_negative_zero() {
        assert_eq!(format_value(-0.001), "0");
        assert_eq!(format_value(-0.25), "-0.25");
        assert_eq!(format_value(-5.0), "-5");
    }
}
