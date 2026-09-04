use std::{borrow::BorrowMut, collections::BTreeMap};

use chrono::prelude::*;
use plotters::{
    chart::ChartBuilder,
    prelude::{Circle, DrawingAreaErrorKind, IntoDrawingArea, Polygon, SVGBackend},
    series::{AreaSeries, DottedLineSeries, Histogram, LineSeries},
    style::{Color, IntoFont, Palette, Palette99, RGBColor, TextStyle, WHITE},
};
use valens_domain as domain;

use crate::Theme;

pub const COLOR_BODY_WEIGHT: usize = 1;
pub const COLOR_AVG_BODY_WEIGHT: usize = 1;
pub const COLOR_BODY_FAT_JP3: usize = 4;
pub const COLOR_BODY_FAT_JP7: usize = 0;
pub const COLOR_FFMI: usize = 7;
pub const COLOR_PERIOD_INTENSITY: usize = 0;
pub const COLOR_LOAD: usize = 1;
pub const COLOR_LONG_TERM_LOAD: usize = 1;
pub const COLOR_RPE: usize = 0;
pub const COLOR_SET_VOLUME: usize = 3;
pub const COLOR_VOLUME_LOAD: usize = 6;
pub const COLOR_TUT: usize = 2;
pub const COLOR_REPS: usize = 4;
pub const COLOR_WEIGHT: usize = 8;
pub const COLOR_TIME: usize = 5;

pub const OPACITY_CIRCLE: f64 = 0.9;
pub const OPACITY_LINE: f64 = 0.9;
pub const OPACITY_DOTTED_LINE: f64 = 0.6;
pub const OPACITY_HISTOGRAM: f64 = 0.9;
pub const OPACITY_AREA: f64 = 0.3;

pub const SIZE_CIRCLE: u32 = 2;
pub const WIDTH_LINE: u32 = 2;
pub const WIDTH_DOTTED_LINE: u32 = 1;

pub const FONT: (&str, u32) = ("Roboto", 11);

#[derive(Clone, PartialEq)]
pub enum PlotType {
    #[allow(dead_code)]
    Circle(usize, f64, u32),
    Line(usize, f64, u32),
    DottedLine(usize, f64, u32),
    Histogram(usize, f64),
    Area(usize, f64),
}

#[must_use]
pub fn plot_line(color: usize) -> Vec<PlotType> {
    vec![PlotType::Line(color, OPACITY_LINE, WIDTH_LINE)]
}

#[must_use]
pub fn plot_dotted_line(color: usize) -> Vec<PlotType> {
    vec![PlotType::DottedLine(
        color,
        OPACITY_DOTTED_LINE,
        WIDTH_DOTTED_LINE,
    )]
}

#[must_use]
pub fn plot_histogram(color: usize) -> Vec<PlotType> {
    vec![PlotType::Histogram(color, OPACITY_HISTOGRAM)]
}

#[must_use]
pub fn plot_area(color: usize) -> Vec<PlotType> {
    vec![PlotType::Area(color, OPACITY_AREA)]
}

#[must_use]
pub fn plot_area_with_border(color: usize) -> Vec<PlotType> {
    vec![
        PlotType::Area(color, OPACITY_AREA),
        PlotType::Line(color, OPACITY_LINE, WIDTH_LINE),
    ]
}

#[derive(Default, Clone, Copy, PartialEq)]
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

#[derive(Clone, PartialEq)]
pub struct PlotData {
    pub values_high: Vec<(NaiveDate, f32)>,
    pub values_low: Option<Vec<(NaiveDate, f32)>>,
    pub plots: Vec<PlotType>,
    pub params: PlotParams,
}

impl PlotData {
    #[must_use]
    pub fn dominant_color_and_opacity(&self) -> (usize, f64) {
        match self.plots.last() {
            Some(
                PlotType::Circle(c, o, _)
                | PlotType::Line(c, o, _)
                | PlotType::DottedLine(c, o, _)
                | PlotType::Histogram(c, o)
                | PlotType::Area(c, o),
            ) => (*c, *o),
            None => (0, 0.0),
        }
    }
}

/// A data point together with its pixel position in the rendered chart SVG.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub date: NaiveDate,
    pub value: f32,
    pub x: i32,
    pub y: i32,
}

/// Pixel-mapped samples of a single series, in the same order as the input.
///
/// `low` is present only for band plots and holds the lower edge.
#[derive(Clone, PartialEq)]
pub struct SeriesSamples {
    pub color: usize,
    pub opacity: f64,
    pub high: Vec<Sample>,
    pub low: Option<Vec<Sample>>,
}

/// Inner plotting rectangle in SVG pixel coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlotArea {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

/// The rendered chart SVG together with the pixel positions of its samples.
#[derive(Clone, PartialEq)]
pub struct PlotResult {
    pub svg: String,
    pub series: Vec<SeriesSamples>,
    pub area: PlotArea,
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
///   - Dotted line: plot the series as a dotted line with the given color and
///     thickness
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
    interval: domain::Interval,
    theme: Theme,
    window_width: u32,
) -> Result<Option<PlotResult>, Box<dyn std::error::Error>> {
    if all_zeros(data) {
        return Ok(None);
    }

    let (Some(primary_bounds), secondary_bounds) = determine_y_bounds(data) else {
        return Ok(None);
    };

    let mut result = String::new();
    let mut series = Vec::with_capacity(data.len());
    let area;

    {
        let root = SVGBackend::with_string(&mut result, (chart_width(window_width), 200))
            .into_drawing_area();
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
                    PlotType::DottedLine(color, opacity, size) => {
                        [values_low.as_ref(), values_high.as_ref()]
                            .into_iter()
                            .flatten()
                            .try_for_each(
                                |values| -> Result<(), DrawingAreaErrorKind<std::io::Error>> {
                                    let data = DottedLineSeries::new(
                                        values.iter().map(|(x, y)| (*x, *y)),
                                        1,
                                        4,
                                        move |c| {
                                            Circle::new(
                                                c,
                                                size,
                                                Palette99::pick(color).mix(opacity).filled(),
                                            )
                                        },
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
                }
            }
        }

        for plot_data in data {
            let (color, opacity) = plot_data.dominant_color_and_opacity();
            let sample = |values: &[(NaiveDate, f32)]| {
                let mut samples = values
                    .iter()
                    .map(|(date, value)| {
                        let (x, y) = if plot_data.params.secondary {
                            chart.borrow_secondary().backend_coord(&(*date, *value))
                        } else {
                            chart.backend_coord(&(*date, *value))
                        };
                        Sample {
                            date: *date,
                            value: *value,
                            x,
                            y,
                        }
                    })
                    .collect::<Vec<_>>();
                samples.sort_by_key(|s| s.date);
                samples
            };
            series.push(SeriesSamples {
                color,
                opacity,
                high: sample(&plot_data.values_high),
                low: plot_data.values_low.as_deref().map(sample),
            });
        }

        let (left, bottom) =
            chart.backend_coord(&(interval.first, primary_bounds.min_with_margin()));
        let (right, top) = chart.backend_coord(&(interval.last, primary_bounds.max_with_margin()));
        area = PlotArea {
            left,
            right,
            top,
            bottom,
        };

        root.present()?;
    }

    Ok(Some(PlotResult {
        svg: result,
        series,
        area,
    }))
}

#[must_use]
pub fn plot_data_min_avg_max<T: Into<f32> + Copy>(
    data: &[(NaiveDate, T)],
    interval: domain::Interval,
    params: PlotParams,
    color: usize,
) -> [PlotData; 2] {
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

    [
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
    ]
}

/// A named series of plot data shown in a chart legend.
///
/// List series in legend order (most prominent first). Chart renderers
/// typically reverse this for drawing, so the first-listed series ends up
/// drawn on top.
///
/// The `data` field is a `Vec<PlotData>` so a single named series can span
/// multiple gap-separated segments (e.g. a 7-day rolling average broken by
/// missing data). All segments share the same plot style and contribute one
/// legend entry.
#[derive(Clone, PartialEq)]
pub struct LabeledSeries {
    pub name: String,
    pub data: Vec<PlotData>,
}

impl LabeledSeries {
    #[must_use]
    pub fn new(name: impl Into<String>, data: impl Into<Vec<PlotData>>) -> Self {
        Self {
            name: name.into(),
            data: data.into(),
        }
    }

    #[must_use]
    pub fn label(&self) -> ChartLabel {
        let (color, opacity) = self
            .data
            .first()
            .map_or((0, 0.0), PlotData::dominant_color_and_opacity);
        ChartLabel {
            name: self.name.clone(),
            color,
            opacity,
        }
    }
}

impl From<PlotData> for Vec<PlotData> {
    fn from(value: PlotData) -> Self {
        vec![value]
    }
}

#[derive(Clone, PartialEq)]
pub struct ChartLabel {
    pub name: String,
    pub color: usize,
    pub opacity: f64,
}

/// Build a legend-ordered pair of [`LabeledSeries`] for a min/avg/max chart.
///
/// Returned in legend order: `["Avg. {metric_name}", "Min./max. {metric_name}"]`.
#[must_use]
pub fn labeled_min_avg_max<T: Into<f32> + Copy>(
    metric_name: &str,
    data: &[(NaiveDate, T)],
    interval: domain::Interval,
    params: PlotParams,
    color: usize,
) -> Vec<LabeledSeries> {
    let [area, line] = plot_data_min_avg_max(data, interval, params, color);
    vec![
        LabeledSeries::new(format!("Avg. {metric_name}"), line),
        LabeledSeries::new(format!("Min./max. {metric_name}"), area),
    ]
}

#[must_use]
pub fn hex_color(color: usize, opacity: f64) -> String {
    let plotters::style::RGBAColor(r, g, b, a) =
        plotters::style::Palette99::pick(color).mix(opacity);
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    let a = (a * 255.0) as u8;
    format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
}

#[must_use]
pub fn rgba_color(color: usize, opacity: f64) -> String {
    let (r, g, b) = Palette99::pick(color).rgb();
    format!("rgba({r}, {g}, {b}, {opacity})")
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

fn colors(theme: Theme) -> (RGBColor, RGBColor) {
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

/// The width of the chart, inset from the window and clamped to the supported range.
fn chart_width(window_width: u32) -> u32 {
    window_width.saturating_sub(20).clamp(300, 960)
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;
    use pretty_assertions::assert_eq;

    use super::*;

    /// The number of colors of `Palette99`, despite the name of the palette.
    const PALETTE_LEN: usize = 21;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 5, day).unwrap()
    }

    fn plot_data(values_high: &[(NaiveDate, f32)], params: PlotParams) -> PlotData {
        PlotData {
            values_high: values_high.to_vec(),
            values_low: None,
            plots: plot_line(0),
            params,
        }
    }

    #[test]
    fn test_determine_y_bounds_without_data() {
        let (primary, secondary) = determine_y_bounds(&[]);

        assert!(primary.is_none());
        assert!(secondary.is_none());
    }

    #[test]
    fn test_determine_y_bounds_ignores_plots_without_values() {
        let (primary, _) = determine_y_bounds(&[plot_data(&[], PlotParams::default())]);

        assert!(primary.is_none());
    }

    #[test]
    fn test_determine_y_bounds_spans_all_values() {
        let (primary, secondary) = determine_y_bounds(&[
            plot_data(&[(date(1), 3.0)], PlotParams::default()),
            plot_data(&[(date(2), 7.0)], PlotParams::default()),
        ]);

        let primary = primary.unwrap();
        assert_approx_eq!(primary.min, 3.0);
        assert_approx_eq!(primary.max, 7.0);
        assert!(secondary.is_none());
    }

    #[test]
    fn test_determine_y_bounds_includes_low_values() {
        let (primary, _) = determine_y_bounds(&[PlotData {
            values_low: Some(vec![(date(1), 9.0)]),
            ..plot_data(&[(date(1), 3.0)], PlotParams::default())
        }]);

        let primary = primary.unwrap();
        assert_approx_eq!(primary.min, 3.0);
        assert_approx_eq!(primary.max, 9.0);
    }

    #[test]
    fn test_determine_y_bounds_of_all_zero_values() {
        let (primary, _) = determine_y_bounds(&[plot_data(
            &[(date(1), 0.0), (date(2), 0.0)],
            PlotParams::default(),
        )]);

        let primary = primary.unwrap();
        assert_approx_eq!(primary.min, 0.0);
        assert_approx_eq!(primary.max, 0.0);
    }

    #[test]
    fn test_determine_y_bounds_extends_the_configured_range() {
        let (primary, _) = determine_y_bounds(&[plot_data(
            &[(date(1), 12.0)],
            PlotParams::primary_range(0.0, 10.0),
        )]);

        let primary = primary.unwrap();
        assert_approx_eq!(primary.min, 0.0);
        assert_approx_eq!(primary.max, 12.0);
    }

    #[test]
    fn test_determine_y_bounds_separates_the_secondary_axis() {
        let (primary, secondary) = determine_y_bounds(&[
            plot_data(&[(date(1), 3.0)], PlotParams::default()),
            plot_data(&[(date(1), 100.0)], PlotParams::SECONDARY),
        ]);

        assert_approx_eq!(primary.unwrap().max, 3.0);
        assert_approx_eq!(secondary.unwrap().max, 100.0);
    }

    #[test]
    fn test_all_zeros_without_data() {
        assert!(all_zeros(&[]));
    }

    #[test]
    fn test_all_zeros_of_zero_values() {
        assert!(all_zeros(&[PlotData {
            values_low: Some(vec![(date(1), 0.0)]),
            ..plot_data(&[(date(1), 0.0)], PlotParams::default())
        }]));
    }

    #[test]
    fn test_all_zeros_of_non_zero_high_value() {
        assert!(!all_zeros(&[plot_data(
            &[(date(1), 0.0), (date(2), 1.0)],
            PlotParams::default()
        )]));
    }

    #[test]
    fn test_all_zeros_of_non_zero_low_value() {
        assert!(!all_zeros(&[PlotData {
            values_low: Some(vec![(date(1), 1.0)]),
            ..plot_data(&[(date(1), 0.0)], PlotParams::default())
        }]));
    }

    #[test]
    fn test_plot_data_min_avg_max() {
        let [area, line] = plot_data_min_avg_max(
            &[(date(1), 1.0), (date(1), 3.0), (date(2), 5.0)],
            domain::Interval {
                first: date(1),
                last: date(1),
            },
            PlotParams::default(),
            0,
        );

        assert_eq!(area.values_high, vec![(date(1), 1.0)]);
        assert_eq!(area.values_low, Some(vec![(date(1), 3.0)]));
        assert_eq!(line.values_high, vec![(date(1), 2.0)]);
        assert_eq!(line.values_low, None);
    }

    #[test]
    fn test_labeled_min_avg_max() {
        let series = labeled_min_avg_max(
            "load",
            &[(date(1), 1.0)],
            domain::Interval {
                first: date(1),
                last: date(1),
            },
            PlotParams::default(),
            0,
        );

        assert_eq!(
            series.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
            vec!["Avg. load", "Min./max. load"]
        );
    }

    #[test]
    fn test_hex_color_opacity() {
        assert!(hex_color(0, 1.0).ends_with("ff"));
        assert!(hex_color(0, 0.0).ends_with("00"));
    }

    #[test]
    fn test_hex_color_wraps_around_the_palette() {
        assert_ne!(hex_color(0, 1.0), hex_color(1, 1.0));
        assert_eq!(hex_color(0, 1.0), hex_color(PALETTE_LEN, 1.0));
    }

    #[test]
    fn test_rgba_color_opacity() {
        assert!(rgba_color(0, 0.5).ends_with(", 0.5)"));
    }

    #[test]
    fn test_rgba_color_wraps_around_the_palette() {
        assert_ne!(rgba_color(0, 1.0), rgba_color(1, 1.0));
        assert_eq!(rgba_color(0, 1.0), rgba_color(PALETTE_LEN, 1.0));
    }

    #[test]
    fn test_colors_are_distinct() {
        let (foreground, background) = colors(Theme::Light);

        assert_ne!(foreground.rgb(), background.rgb());
    }

    #[test]
    fn test_colors_are_swapped_between_themes() {
        let (light_foreground, light_background) = colors(Theme::Light);
        let (dark_foreground, dark_background) = colors(Theme::Dark);

        assert_eq!(light_foreground.rgb(), dark_background.rgb());
        assert_eq!(light_background.rgb(), dark_foreground.rgb());
    }

    #[test]
    fn test_colors_of_the_system_theme_match_the_light_theme() {
        assert_eq!(colors(Theme::System).0.rgb(), colors(Theme::Light).0.rgb());
    }

    /// The window width the plot is asked for, below the lower clamp of the chart width.
    const NARROW_WINDOW: u32 = 320;
    /// The window width the plot is asked for, above the upper clamp of the chart width.
    const WIDE_WINDOW: u32 = 1400;

    fn plot_two_samples(window_width: u32) -> PlotResult {
        plot(
            &[plot_data(
                &[(date(1), 1.0), (date(5), 5.0)],
                PlotParams::default(),
            )],
            domain::Interval {
                first: date(1),
                last: date(5),
            },
            Theme::Light,
            window_width,
        )
        .unwrap()
        .unwrap()
    }

    #[test]
    fn test_plot_area_widens_with_the_window() {
        assert_eq!(
            plot_two_samples(NARROW_WINDOW).area,
            PlotArea {
                left: 50,
                right: 289,
                top: 10,
                bottom: 159,
            }
        );
        assert_eq!(
            plot_two_samples(WIDE_WINDOW).area,
            PlotArea {
                left: 50,
                right: 949,
                top: 10,
                bottom: 159,
            }
        );
    }

    #[test]
    fn test_plot_samples_span_the_plot_area() {
        for window_width in [NARROW_WINDOW, WIDE_WINDOW] {
            let result = plot_two_samples(window_width);
            let samples = &result.series[0].high;

            assert_eq!(
                samples.iter().map(|sample| sample.x).collect::<Vec<_>>(),
                vec![result.area.left, result.area.right]
            );
            for sample in samples {
                assert!(sample.y >= result.area.top && sample.y <= result.area.bottom);
            }
        }
    }
}
