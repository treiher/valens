use std::{borrow::BorrowMut, collections::BTreeMap};

use chrono::prelude::*;
use gloo_utils::window;
use plotters::{
    chart::ChartBuilder,
    prelude::{Circle, DrawingAreaErrorKind, IntoDrawingArea, Polygon, SVGBackend},
    series::{AreaSeries, Histogram, LineSeries},
    style::{Color, IntoFont, Palette, Palette99, RGBColor, TextStyle, WHITE},
};
use valens_domain as domain;
use wasm_bindgen::JsValue;

use crate::Theme;

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

pub const OPACITY_LINE: f64 = 0.9;
pub const OPACITY_AREA: f64 = 0.3;

pub const WIDTH_LINE: u32 = 2;

pub const FONT: (&str, u32) = ("Roboto", 11);

#[derive(Clone)]
pub enum PlotType {
    #[allow(dead_code)]
    Circle(usize, f64, u32),
    Line(usize, f64, u32),
    Histogram(usize, f64),
    Area(usize, f64),
}

#[must_use]
pub fn plot_line(color: usize) -> Vec<PlotType> {
    vec![PlotType::Line(color, OPACITY_LINE, WIDTH_LINE)]
}

#[must_use]
pub fn plot_area(color: usize) -> Vec<PlotType> {
    vec![PlotType::Area(color, OPACITY_AREA)]
}

#[must_use]
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
    #[must_use]
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
struct Bounds {
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
#[allow(clippy::missing_errors_doc)]
pub fn plot(
    data: &[PlotData],
    interval: &domain::Interval,
    theme: &Theme,
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

#[allow(clippy::missing_errors_doc)]
pub fn plot_min_avg_max<T: Into<f32> + Copy>(
    data: &Vec<(NaiveDate, T)>,
    interval: &domain::Interval,
    params: PlotParams,
    color: usize,
    theme: &Theme,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut date_map: BTreeMap<&NaiveDate, Vec<f32>> = BTreeMap::new();

    for (date, value) in data {
        date_map
            .entry(date)
            .or_default()
            .push(Into::<f32>::into(*value));
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

    plot(
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
                    .is_none_or(|v| v.iter().all(|(_, v)| *v == 0.0))
        })
        .reduce(|l, r| l && r)
        .unwrap_or(true)
}

fn colors(theme: &Theme) -> (RGBColor, RGBColor) {
    let dark = RGBColor(20, 22, 26);
    match theme {
        Theme::System | Theme::Light => (dark, WHITE),
        Theme::Dark => (WHITE, dark),
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
