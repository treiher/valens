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
    let current = current_interval.read();
    let today = current_date();
    let duration = current.last - current.first + Duration::days(1);
    let intervals = [
        (
            "1M",
            today - Duration::days(domain::DefaultInterval::_1M as i64),
            today,
            current.last == today
                && duration == Duration::days(domain::DefaultInterval::_1M as i64 + 1),
        ),
        (
            "3M",
            today - Duration::days(domain::DefaultInterval::_3M as i64),
            today,
            current.last == today
                && duration == Duration::days(domain::DefaultInterval::_3M as i64 + 1),
        ),
        (
            "6M",
            today - Duration::days(domain::DefaultInterval::_6M as i64),
            today,
            current.last == today
                && duration == Duration::days(domain::DefaultInterval::_6M as i64 + 1),
        ),
        (
            "1Y",
            today - Duration::days(domain::DefaultInterval::_1Y as i64),
            today,
            current.last == today
                && duration == Duration::days(domain::DefaultInterval::_1Y as i64 + 1),
        ),
        (
            "NOW",
            all.first,
            today,
            current.first == all.first && current.last == today,
        ),
        (
            "ALL",
            all.first,
            all.last,
            current.first == all.first && current.last == all.last,
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
    ];

    let left_first = if current.first - duration / 4 > all.first {
        current.first - duration / 4
    } else {
        all.first
    };
    let left_last = if current.first - duration / 4 > all.first {
        current.last - duration / 4
    } else {
        all.first + duration - Duration::days(1)
    };
    let is_left_disabled = current.first == left_first;

    let right_first = if current.last + duration / 4 < today {
        current.first + duration / 4
    } else {
        today - duration + Duration::days(1)
    };
    let right_last = if current.last + duration / 4 < today {
        current.last + duration / 4
    } else {
        today
    };
    let is_right_disabled = current.last == right_last;

    rsx! {
        div {
            class: "field has-addons has-addons-centered",
            for (name, first, last, is_active) in intervals {
                p {
                    class: "control",
                    a {
                        class: "button is-small",
                        class: if is_active { "is-link" },
                        "data-testid": "interval-{name}",
                        onclick: move |_| { *current_interval.write() = domain::Interval { first, last } },
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
                onclick: move |_| { *current_interval.write() = domain::Interval { first: left_first, last: left_last } },
                Icon { name: "chevron-left" }
            }
            span {
                class: "mx-3",
                "{current.first} – {current.last}"
            }
            button {
                class: "button is-small",
                disabled: is_right_disabled,
                onclick: move |_| { *current_interval.write() = domain::Interval { first: right_first, last: right_last } },
                Icon { name: "chevron-right" }
            }
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
    use_window_width();
    let labels: Vec<web_app::chart::ChartLabel> = series
        .iter()
        .map(web_app::chart::LabeledSeries::label)
        .collect();
    let data: Vec<web_app::chart::PlotData> =
        series.into_iter().rev().flat_map(|s| s.data).collect();
    let chart =
        web_app::chart::plot(&data, interval, settings.current_theme()).map_err(|e| e.to_string());

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

static WINDOW_WIDTH: GlobalSignal<f64> = Signal::global(window_width);
static RESIZE_LISTENER: std::sync::Once = std::sync::Once::new();

/// Subscribes the component to changes of the window width and returns the current width.
fn use_window_width() -> f64 {
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

fn window_width() -> f64 {
    web_sys::window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|width| width.as_f64())
        .unwrap_or_default()
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

    use super::{format_value, nearest_mark};

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, day).unwrap()
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
