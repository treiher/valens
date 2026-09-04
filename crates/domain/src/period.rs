use std::slice::Iter;

use chrono::{Duration, Local, NaiveDate};
use derive_more::Display;

use crate::{CreateError, DeleteError, ReadError, SyncError, UpdateError, ValidationError};

#[allow(async_fn_in_trait)]
pub trait PeriodService {
    async fn get_cycles(&self) -> Result<Vec<Cycle>, ReadError>;
    async fn get_current_cycle(&self) -> Result<CurrentCycle, ReadError>;
    async fn get_period(&self) -> Result<Vec<Period>, ReadError>;
    async fn create_period(&self, period: Period) -> Result<Period, CreateError>;
    async fn replace_period(&self, period: Period) -> Result<Period, UpdateError>;
    async fn delete_period(&self, date: NaiveDate) -> Result<(), DeleteError>;

    async fn validate_period_date(&self, date: &str) -> Result<NaiveDate, ValidationError> {
        match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            Ok(parsed_date) => {
                if parsed_date <= Local::now().date_naive() {
                    match self.get_period().await {
                        Ok(periods) => {
                            if periods.iter().all(|u| u.date != parsed_date) {
                                Ok(parsed_date)
                            } else {
                                Err(ValidationError::Conflict("date".to_string()))
                            }
                        }
                        Err(err) => Err(ValidationError::Other(err.into())),
                    }
                } else {
                    Err(ValidationError::Other(
                        "date must not be in the future".into(),
                    ))
                }
            }
            Err(_) => Err(ValidationError::Other("invalid date".into())),
        }
    }

    fn validate_period_intensity(&self, value: &str) -> Result<Intensity, ValidationError> {
        match value.trim().parse::<u8>() {
            Ok(parsed_value) => {
                Intensity::try_from(parsed_value).map_err(|err| ValidationError::Other(err.into()))
            }
            Err(_) => Err(ValidationError::Other(
                "intensity must be a positive whole number".into(),
            )),
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait PeriodRepository {
    async fn sync_period(&self) -> Result<Vec<Period>, SyncError>;
    async fn read_period(&self) -> Result<Vec<Period>, ReadError>;
    async fn create_period(&self, period: Period) -> Result<Period, CreateError>;
    async fn replace_period(&self, period: Period) -> Result<Period, UpdateError>;
    async fn delete_period(&self, date: NaiveDate) -> Result<(), DeleteError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Period {
    pub date: NaiveDate,
    pub intensity: Intensity,
}

#[derive(Debug, Clone, Copy, Display, PartialEq, Eq)]
pub enum Intensity {
    #[display("1")]
    Spotting = 1,
    #[display("2")]
    Light = 2,
    #[display("3")]
    Medium = 3,
    #[display("4")]
    Heavy = 4,
}

impl Intensity {
    pub fn iter() -> Iter<'static, Intensity> {
        static INTENSITY: [Intensity; 4] = [
            Intensity::Spotting,
            Intensity::Light,
            Intensity::Medium,
            Intensity::Heavy,
        ];
        INTENSITY.iter()
    }
}

impl TryFrom<u8> for Intensity {
    type Error = IntensityError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            x if x == Intensity::Spotting as u8 => Ok(Intensity::Spotting),
            x if x == Intensity::Light as u8 => Ok(Intensity::Light),
            x if x == Intensity::Medium as u8 => Ok(Intensity::Medium),
            x if x == Intensity::Heavy as u8 => Ok(Intensity::Heavy),
            _ => Err(IntensityError::OutOfRange),
        }
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum IntensityError {
    #[error("intensity must be in the range 1 to 4")]
    OutOfRange,
}

/// Returns the cycles of `period`, which must be sorted by date in ascending order.
#[must_use]
pub fn cycles(period: &[Period]) -> Vec<Cycle> {
    if period.is_empty() {
        return vec![];
    }

    let mut result = vec![];
    let mut begin = period.iter().map(|p| p.date).min().unwrap_or_default();
    let mut last = begin;

    for p in &period[1..] {
        if p.date - last > Duration::days(3) {
            result.push(Cycle {
                begin,
                length: p.date - begin,
            });
            begin = p.date;
        }
        last = p.date;
    }

    result
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct Cycle {
    pub begin: NaiveDate,
    pub length: Duration,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct CurrentCycle {
    pub begin: NaiveDate,
    pub time_left: Duration,
    pub time_left_variation: Duration,
}

#[must_use]
pub fn current_cycle(cycles: &[Cycle]) -> Option<CurrentCycle> {
    if cycles.is_empty() {
        return None;
    }

    let today = Local::now().date_naive();
    let cycles = cycles
        .iter()
        .filter(|c| c.begin >= today - Duration::days(182) && c.begin <= today)
        .collect::<Vec<_>>();
    let stats = cycle_stats(&cycles);

    if let Some(last_cycle) = cycles.last() {
        let begin = last_cycle.begin + last_cycle.length;
        Some(CurrentCycle {
            begin,
            time_left: stats.length_median - (today - begin + Duration::days(1)),
            time_left_variation: stats.length_variation,
        })
    } else {
        None
    }
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct CycleStats {
    pub length_median: Duration,
    pub length_variation: Duration,
}

#[must_use]
pub fn cycle_stats(cycles: &[&Cycle]) -> CycleStats {
    let mut cycle_lengths = cycles.iter().map(|c| c.length).collect::<Vec<_>>();
    cycle_lengths.sort();
    CycleStats {
        length_median: quartile(&cycle_lengths, Quartile::Q2),
        length_variation: (quartile(&cycle_lengths, Quartile::Q3)
            - quartile(&cycle_lengths, Quartile::Q1))
            / 2,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Quartile {
    Q1 = 1,
    Q2 = 2,
    Q3 = 3,
}

/// Returns the requested quartile of `durations`, which must be sorted in ascending order.
#[must_use]
pub fn quartile(durations: &[Duration], quartile_num: Quartile) -> Duration {
    if durations.is_empty() {
        return Duration::days(0);
    }
    let idx = durations.len() / 2;
    match quartile_num {
        Quartile::Q1 => quartile(&durations[..idx], Quartile::Q2),
        Quartile::Q2 => {
            if durations.len().is_multiple_of(2) {
                (durations[idx - 1] + durations[idx]) / 2
            } else {
                durations[idx]
            }
        }
        Quartile::Q3 => {
            if durations.len().is_multiple_of(2) {
                quartile(&durations[idx..], Quartile::Q2)
            } else {
                quartile(&durations[idx + 1..], Quartile::Q2)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;
    use rstest::rstest;

    use crate::{
        Service,
        tests::{Call, FakeRepository},
    };

    use super::*;

    static TODAY: std::sync::LazyLock<NaiveDate> =
        std::sync::LazyLock::new(|| Local::now().date_naive());

    #[test]
    fn test_intensity_iter() {
        assert_eq!(
            Intensity::iter().copied().collect::<Vec<_>>(),
            [
                Intensity::Spotting,
                Intensity::Light,
                Intensity::Medium,
                Intensity::Heavy
            ]
        );
    }

    #[test]
    fn test_intensity_try_from_u8() {
        for intensity in Intensity::iter() {
            assert_eq!(Intensity::try_from(*intensity as u8), Ok(*intensity));
        }

        assert_eq!(Intensity::try_from(0), Err(IntensityError::OutOfRange));
    }

    #[test]
    fn test_cycles() {
        assert_eq!(cycles(&[]), vec![]);
        assert_eq!(
            cycles(&[
                Period {
                    date: from_num_days(1),
                    intensity: Intensity::Medium,
                },
                Period {
                    date: from_num_days(5),
                    intensity: Intensity::Heavy,
                },
                Period {
                    date: from_num_days(8),
                    intensity: Intensity::Light,
                },
                Period {
                    date: from_num_days(33),
                    intensity: Intensity::Spotting,
                }
            ]),
            vec![
                Cycle {
                    begin: from_num_days(1),
                    length: Duration::days(4),
                },
                Cycle {
                    begin: from_num_days(5),
                    length: Duration::days(28),
                }
            ]
        );
    }

    #[rstest]
    #[case::no_cycle(&[], None)]
    #[case::no_recent_cycles(
        &[
            Cycle {
                begin: *TODAY - Duration::days(228),
                length: Duration::days(26),
            },
            Cycle {
                begin: *TODAY - Duration::days(202),
                length: Duration::days(28),
            }
        ],
        None
    )]
    #[case::one_cycle(
        &[
            Cycle {
                begin: *TODAY - Duration::days(42),
                length: Duration::days(28),
            }
        ],
        Some(
            CurrentCycle {
                begin: *TODAY - Duration::days(14),
                time_left: Duration::days(13),
                time_left_variation: Duration::days(0)
            }
        )
    )]
    #[case::multiple_cycles(
        &[
            Cycle {
                begin: *TODAY - Duration::days(68),
                length: Duration::days(26),
            },
            Cycle {
                begin: *TODAY - Duration::days(42),
                length: Duration::days(28),
            }
        ],
        Some(
            CurrentCycle {
                begin: *TODAY - Duration::days(14),
                time_left: Duration::days(12),
                time_left_variation: Duration::days(1)
            }
        )
    )]
    fn test_current_cycle(#[case] cycles: &[Cycle], #[case] expected: Option<CurrentCycle>) {
        assert_eq!(current_cycle(cycles), expected);
    }

    #[test]
    fn test_quartile_one() {
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
    fn test_quartile_two() {
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
    fn test_quartile_three() {
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

    fn from_num_days(days: i32) -> NaiveDate {
        NaiveDate::from_num_days_from_ce_opt(days).unwrap()
    }

    #[rstest]
    #[case::unused("2020-02-03", Ok("2020-02-03"))]
    #[case::already_used("2020-02-02", Err("entry with this date already exists"))]
    #[case::in_the_future("2999-01-01", Err("date must not be in the future"))]
    #[case::unparsable("02/02/2020", Err("invalid date"))]
    fn test_validate_period_date(#[case] input: &str, #[case] expected: Result<&str, &str>) {
        let service = Service::new(FakeRepository::default().with_period(vec![Period {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            intensity: Intensity::Medium,
        }]));

        assert_eq!(
            pollster::block_on(service.validate_period_date(input))
                .map(|date| date.to_string())
                .map_err(|err| err.to_string()),
            expected.map(str::to_string).map_err(str::to_string)
        );
    }

    #[test]
    fn test_validate_period_date_unreadable_period() {
        let service = Service::new(FakeRepository::default().failing(Call::ReadPeriod));

        assert!(matches!(
            pollster::block_on(service.validate_period_date("2020-02-02")),
            Err(ValidationError::Other(_))
        ));
    }

    #[rstest]
    #[case::lowest("1", Ok(Intensity::Spotting))]
    #[case::highest("4", Ok(Intensity::Heavy))]
    #[case::surrounded_by_whitespace(" 2 ", Ok(Intensity::Light))]
    #[case::below_range("0", Err("intensity must be in the range 1 to 4"))]
    #[case::above_range("5", Err("intensity must be in the range 1 to 4"))]
    #[case::not_a_number("abc", Err("intensity must be a positive whole number"))]
    fn test_validate_period_intensity(
        #[case] input: &str,
        #[case] expected: Result<Intensity, &str>,
    ) {
        assert_eq!(
            Service::new(FakeRepository::default())
                .validate_period_intensity(input)
                .map_err(|err| err.to_string()),
            expected.map_err(str::to_string)
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn test_cycles_are_positive_and_consecutive(gaps in prop::collection::vec(1i64..30, 1..30)) {
            let mut date = NaiveDate::default();
            let period = gaps
                .iter()
                .map(|gap| {
                    date += Duration::days(*gap);
                    Period {
                        date,
                        intensity: Intensity::Medium,
                    }
                })
                .collect::<Vec<_>>();

            let cycles = cycles(&period);

            let mut previous_begin = None;
            for cycle in &cycles {
                prop_assert!(cycle.length > Duration::days(0));
                if let Some(previous_begin) = previous_begin {
                    prop_assert!(cycle.begin > previous_begin);
                }
                previous_begin = Some(cycle.begin);
            }
        }

        // A single duration is excluded, because `Q1` and `Q3` are then taken from an empty half
        // and are zero.
        #[test]
        fn test_quartiles_are_ordered(lengths in prop::collection::vec(1i64..100, 2..30)) {
            let mut durations = lengths.iter().map(|l| Duration::days(*l)).collect::<Vec<_>>();
            durations.sort();

            prop_assert!(quartile(&durations, Quartile::Q1) <= quartile(&durations, Quartile::Q2));
            prop_assert!(quartile(&durations, Quartile::Q2) <= quartile(&durations, Quartile::Q3));
        }
    }
}
