use std::collections::BTreeMap;

use chrono::{Days, Duration, Local, NaiveDate};

#[cfg_attr(test, derive(Debug, PartialEq))]
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

#[must_use]
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
pub fn centered_moving_grouping<T: Into<f32> + Copy>(
    data: &Vec<(NaiveDate, T)>,
    interval: &Interval,
    radius: u64,
    group_day: impl Fn(Vec<f32>) -> Option<f32>,
    group_range: impl Fn(Vec<f32>) -> Option<f32>,
) -> Vec<Vec<(NaiveDate, f32)>> {
    let mut date_map: BTreeMap<&NaiveDate, Vec<f32>> = BTreeMap::new();

    for (date, value) in data {
        date_map
            .entry(date)
            .or_default()
            .push(Into::<f32>::into(*value));
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
#[must_use]
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
#[must_use]
pub fn centered_moving_average<T: Into<f32> + Copy>(
    data: &Vec<(NaiveDate, T)>,
    interval: &Interval,
    radius: u64,
) -> Vec<Vec<(NaiveDate, f32)>> {
    #[allow(clippy::cast_precision_loss)]
    centered_moving_grouping(
        data,
        interval,
        radius,
        |d| Some(d.iter().sum::<f32>() / d.len() as f32),
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
#[must_use]
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
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    static TODAY: std::sync::LazyLock<NaiveDate> =
        std::sync::LazyLock::new(|| Local::now().date_naive());

    #[rstest]
    #[case(*TODAY - Duration::days(21), *TODAY - Duration::days(42))]
    fn test_interval_from_range_inclusive(#[case] first: NaiveDate, #[case] last: NaiveDate) {
        let interval: Interval = (first..=last).into();
        assert_eq!(interval, Interval { first, last });
    }

    #[rstest]
    #[case::no_dates(
        &[],
        DefaultInterval::_1M,
        *TODAY - Duration::days(DefaultInterval::_1M as i64),
        *TODAY
    )]
    #[case::last_date_inside_default_interval(
        &[*TODAY - Duration::days(DefaultInterval::_1M as i64 - 2)],
        DefaultInterval::_1M,
        *TODAY - Duration::days(DefaultInterval::_1M as i64),
        *TODAY
    )]
    #[case::last_date_outside_default_interval(
        &[*TODAY - Duration::days(DefaultInterval::_1M as i64 + 42)],
        DefaultInterval::_1M,
        *TODAY - Duration::days(DefaultInterval::_1M as i64 + 42),
        *TODAY
    )]
    #[case::default_interval_all(
        &[*TODAY - Duration::days(21), *TODAY - Duration::days(42)],
        DefaultInterval::All,
        *TODAY - Duration::days(42),
        *TODAY,
    )]
    fn test_init_interval(
        #[case] dates: &[NaiveDate],
        #[case] default_interval: DefaultInterval,
        #[case] first: NaiveDate,
        #[case] last: NaiveDate,
    ) {
        assert_eq!(
            init_interval(dates, default_interval),
            Interval { first, last }
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
    fn test_centered_moving_average(
        #[case] start: (i32, u32, u32),
        #[case] end: (i32, u32, u32),
        #[case] radius: u64,
        #[case] input: &[(i32, u32, u32, f32)],
        #[case] expected: Vec<Vec<(i32, u32, u32, f32)>>,
    ) {
        assert_eq!(
            centered_moving_average(
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
    fn test_centered_moving_total(
        #[case] start: (i32, u32, u32),
        #[case] end: (i32, u32, u32),
        #[case] radius: u64,
        #[case] input: &[(i32, u32, u32, f32)],
        #[case] expected: &[(i32, u32, u32, f32)],
    ) {
        assert_eq!(
            centered_moving_total(
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
            value_based_centered_moving_average(
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
