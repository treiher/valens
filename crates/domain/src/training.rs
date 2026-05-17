use chrono::{Duration, Local, NaiveDate};
use derive_more::{Display, Into};
use std::{
    collections::BTreeMap,
    fmt,
    hash::{Hash, Hasher},
    iter::zip,
    ops::Mul,
};

use crate::TrainingSession;

#[derive(Debug, Default, Display, Clone, Copy, Into, PartialEq, Eq, PartialOrd, Hash)]
pub struct Reps(u32);

impl Reps {
    pub fn new(value: u32) -> Result<Self, RepsError> {
        if !(0..1000).contains(&value) {
            return Err(RepsError::OutOfRange);
        }

        Ok(Self(value))
    }

    #[must_use]
    pub fn including_rir(self, rpe: RPE) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        let reps = self.0 as f32;
        reps + f32::from(RIR::from(rpe))
    }
}

impl TryFrom<&str> for Reps {
    type Error = RepsError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.parse::<u32>() {
            Ok(parsed_value) => Reps::new(parsed_value),
            Err(_) => Err(RepsError::ParseError),
        }
    }
}

impl Mul<Time> for Reps {
    type Output = Time;

    fn mul(self, rhs: Time) -> Self::Output {
        Time(self.0 * rhs.0)
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum RepsError {
    #[error("Reps must be in the range 0 to 999")]
    OutOfRange,
    #[error("Reps must be an integer")]
    ParseError,
}

#[derive(Debug, Default, Display, Clone, Copy, Into, PartialEq, Eq, PartialOrd, Hash)]
pub struct Time(u32);

impl Time {
    pub fn new(value: u32) -> Result<Self, TimeError> {
        if !(0..1000).contains(&value) {
            return Err(TimeError::OutOfRange);
        }

        Ok(Self(value))
    }
}

impl From<Time> for i64 {
    fn from(value: Time) -> Self {
        i64::from(value.0)
    }
}

impl TryFrom<&str> for Time {
    type Error = TimeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.parse::<u32>() {
            Ok(parsed_value) => Time::new(parsed_value),
            Err(_) => Err(TimeError::ParseError),
        }
    }
}

impl Mul<Reps> for Time {
    type Output = Time;

    fn mul(self, rhs: Reps) -> Self::Output {
        Time(self.0 * rhs.0)
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum TimeError {
    #[error("Time must be in the range 0 to 999 s")]
    OutOfRange,
    #[error("Time must be an integer")]
    ParseError,
}

#[derive(Debug, Default, Display, Clone, Copy, Into, PartialOrd)]
pub struct Weight(f32);

impl Weight {
    pub fn new(value: f32) -> Result<Self, WeightError> {
        if !(0.0..1000.0).contains(&value) {
            return Err(WeightError::OutOfRange);
        }

        if (value * 10.0 % 1.0).abs() > f32::EPSILON {
            return Err(WeightError::InvalidResolution);
        }

        Ok(Self(value))
    }
}

impl PartialEq for Weight {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for Weight {}

impl Hash for Weight {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl TryFrom<&str> for Weight {
    type Error = WeightError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.replace(',', ".").parse::<f32>() {
            Ok(parsed_value) => Weight::new(parsed_value),
            Err(_) => Err(WeightError::ParseError),
        }
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum WeightError {
    #[error("Weight must be a multiple of 0.1 kg")]
    InvalidResolution,
    #[error("Weight must be in the range 0.0 to 999.9 kg")]
    OutOfRange,
    #[error("Weight must be a decimal")]
    ParseError,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct RPE(u8);

impl RPE {
    pub const ZERO: RPE = RPE(0);
    pub const ONE: RPE = RPE(10);
    pub const TWO: RPE = RPE(20);
    pub const THREE: RPE = RPE(30);
    pub const FOUR: RPE = RPE(40);
    pub const FIVE: RPE = RPE(50);
    pub const SIX: RPE = RPE(60);
    pub const SEVEN: RPE = RPE(70);
    pub const EIGHT: RPE = RPE(80);
    pub const NINE: RPE = RPE(90);
    pub const TEN: RPE = RPE(100);

    pub fn new(value: f32) -> Result<Self, RPEError> {
        if !(0.0..=10.0).contains(&value) {
            return Err(RPEError::OutOfRange);
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = (value * 10.0) as u8;

        if !v.is_multiple_of(5) {
            return Err(RPEError::InvalidResolution);
        }

        Ok(Self(v))
    }

    #[must_use]
    pub fn avg(values: &[RPE]) -> Option<RPE> {
        if values.is_empty() {
            None
        } else {
            #[allow(clippy::cast_possible_truncation)]
            Some(RPE(
                (values.iter().map(|rpe| rpe.0 as usize).sum::<usize>() / values.len()) as u8,
            ))
        }
    }
}

impl From<RPE> for f32 {
    fn from(value: RPE) -> Self {
        f32::from(value.0) / 10.0
    }
}

impl TryFrom<&str> for RPE {
    type Error = RPEError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.replace(',', ".").parse::<f32>() {
            Ok(parsed_value) => RPE::new(parsed_value),
            Err(_) => Err(RPEError::ParseError),
        }
    }
}

impl fmt::Display for RPE {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", f32::from(*self))
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum RPEError {
    #[error("RPE must be in the range 0.0 to 10.0")]
    OutOfRange,
    #[error("RPE must be a multiple of 0.5")]
    InvalidResolution,
    #[error("RPE must be a decimal")]
    ParseError,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct RIR(u8);

impl From<RPE> for RIR {
    fn from(value: RPE) -> Self {
        Self(100 - value.0)
    }
}

impl From<RIR> for f32 {
    fn from(value: RIR) -> Self {
        f32::from(value.0) / 10.0
    }
}

impl fmt::Display for RIR {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", f32::from(*self))
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct TrainingStats {
    pub short_term_load: Vec<(NaiveDate, f32)>,
    pub long_term_load: Vec<(NaiveDate, f32)>,
}

impl TrainingStats {
    pub const LOAD_RATIO_LOW: f32 = 0.8;
    pub const LOAD_RATIO_HIGH: f32 = 1.5;

    #[must_use]
    pub fn load_ratio(&self) -> Option<f32> {
        let long_term_load = self.long_term_load.last().map_or(0., |(_, l)| *l);
        if long_term_load > 0. {
            let short_term_load = self.short_term_load.last().map_or(0., |(_, l)| *l);
            Some(short_term_load / long_term_load)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.short_term_load.clear();
        self.long_term_load.clear();
    }
}

#[must_use]
pub fn training_stats(training_sessions: &[&TrainingSession]) -> TrainingStats {
    let short_term_load = weighted_sum_of_load(training_sessions, 7);
    let long_term_load = average_weighted_sum_of_load(&short_term_load, 28);
    TrainingStats {
        short_term_load,
        long_term_load,
    }
}

fn weighted_sum_of_load(
    training_sessions: &[&TrainingSession],
    window_size: usize,
) -> Vec<(NaiveDate, f32)> {
    let mut result: BTreeMap<NaiveDate, f32> = BTreeMap::new();

    let today = Local::now().date_naive();
    let mut day = training_sessions.first().map_or(today, |t| t.date);
    while day <= today {
        result.insert(day, 0.0);
        day += Duration::days(1);
    }

    for t in training_sessions {
        #[allow(clippy::cast_precision_loss)]
        result
            .entry(t.date)
            .and_modify(|e| *e += t.load() as f32)
            .or_insert(t.load() as f32);
    }

    #[allow(clippy::cast_precision_loss)]
    let weighting: Vec<f32> = (0..window_size)
        .map(|i| 1. - 1. / window_size as f32 * i as f32)
        .collect();
    let mut window: Vec<f32> = (0..window_size).map(|_| 0.).collect();

    result
        .into_iter()
        .map(|(date, load)| {
            window.rotate_right(1);
            window[0] = load;
            (
                date,
                zip(&window, &weighting)
                    .map(|(load, weight)| load * weight)
                    .sum(),
            )
        })
        .collect()
}

fn average_weighted_sum_of_load(
    weighted_sum_of_load: &[(NaiveDate, f32)],
    window_size: usize,
) -> Vec<(NaiveDate, f32)> {
    #[allow(clippy::cast_precision_loss)]
    weighted_sum_of_load
        .windows(window_size)
        .map(|window| {
            (
                window.last().unwrap().0,
                window.iter().map(|(_, l)| l).sum::<f32>() / window_size as f32,
            )
        })
        .collect::<Vec<_>>()
}

/// Calculates the one-rep max using a hybrid formula.
///
/// The Brzycki formula is used for 2-6 reps, the Desgorces formula for 7-19
/// reps, and the Wathan formula for 20+ reps.
#[must_use]
pub fn one_rep_max(reps: f32, weight: f32) -> f32 {
    if reps <= 1.0 {
        weight
    } else if reps <= 6.0 {
        // Brzycki
        weight * 36.0 / (37.0 - reps)
    } else if reps <= 19.0 {
        // Desgorces
        100.0 * weight / (83.7677 * (-0.0338 * reps).exp() + 17.6846)
    } else {
        // Wathan
        100.0 * weight / (48.8 + 53.8 * (-0.075 * reps).exp())
    }
}

/// Returns the number of reps achievable at a given percentage of the one-rep
/// max (inverse of [`one_rep_max`]).
#[must_use]
pub fn reps_for_percentage(percentage: f32) -> f32 {
    if percentage >= 100.0 {
        return 1.0;
    }
    let r_brzycki = 37.0 - 36.0 * percentage / 100.0;
    if r_brzycki <= 6.0 {
        return r_brzycki;
    }
    let r_desgorces = -((percentage - 17.6846) / 83.7677).ln() / 0.0338;
    if r_desgorces <= 19.0 {
        return r_desgorces;
    }
    -((percentage - 48.8) / 53.8).ln() / 0.075
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use crate::TrainingSessionElement;

    use super::*;

    static TODAY: std::sync::LazyLock<NaiveDate> =
        std::sync::LazyLock::new(|| Local::now().date_naive());

    static TRAINING_SESSION: std::sync::LazyLock<TrainingSession> =
        std::sync::LazyLock::new(|| TrainingSession {
            id: 1.into(),
            routine_id: 2.into(),
            date: *TODAY - Duration::days(10),
            notes: String::from("A"),
            elements: vec![
                TrainingSessionElement::Set {
                    exercise_id: 1.into(),
                    reps: Some(Reps(10)),
                    time: Some(Time(3)),
                    weight: Some(Weight(30.0)),
                    rpe: Some(RPE::EIGHT),
                    target_reps: Some(Reps(8)),
                    target_time: Some(Time(4)),
                    target_weight: Some(Weight(40.0)),
                    target_rpe: Some(RPE::NINE),
                    automatic: false,
                },
                TrainingSessionElement::Rest {
                    target_time: Some(Time(60)),
                    automatic: true,
                },
                TrainingSessionElement::Set {
                    exercise_id: 2.into(),
                    reps: Some(Reps(5)),
                    time: Some(Time(4)),
                    weight: None,
                    rpe: Some(RPE::FOUR),
                    target_reps: None,
                    target_time: None,
                    target_weight: None,
                    target_rpe: None,
                    automatic: false,
                },
                TrainingSessionElement::Rest {
                    target_time: Some(Time(60)),
                    automatic: true,
                },
                TrainingSessionElement::Set {
                    exercise_id: 2.into(),
                    reps: None,
                    time: Some(Time(60)),
                    weight: None,
                    rpe: None,
                    target_reps: None,
                    target_time: None,
                    target_weight: None,
                    target_rpe: None,
                    automatic: false,
                },
                TrainingSessionElement::Rest {
                    target_time: Some(Time(60)),
                    automatic: true,
                },
            ],
            exercise_notes: BTreeMap::new(),
        });

    #[rstest]
    #[case(0, Ok(Reps(0)))]
    #[case(999, Ok(Reps(999)))]
    #[case(1000, Err(RepsError::OutOfRange))]
    fn test_reps_new(#[case] input: u32, #[case] expected: Result<Reps, RepsError>) {
        assert_eq!(Reps::new(input), expected);
    }

    #[rstest]
    #[case("0", Ok(Reps(0)))]
    #[case("999", Ok(Reps(999)))]
    #[case("1000", Err(RepsError::OutOfRange))]
    #[case("4.", Err(RepsError::ParseError))]
    #[case("", Err(RepsError::ParseError))]
    fn test_reps_from_str(#[case] input: &str, #[case] expected: Result<Reps, RepsError>) {
        assert_eq!(Reps::try_from(input), expected);
    }

    #[rstest]
    fn test_reps_mul_time() {
        assert_eq!(Reps(2) * Time(4), Time(8));
    }

    #[rstest]
    #[case(Reps(8), "8")]
    fn test_reps_display(#[case] input: Reps, #[case] expected: &str) {
        assert_eq!(input.to_string(), expected);
    }

    #[rstest]
    #[case(Reps(5), RPE::TEN, 5.0)]
    #[case(Reps(5), RPE::EIGHT, 7.0)]
    #[case(Reps(5), RPE::ZERO, 15.0)]
    fn test_reps_including_rir(#[case] reps: Reps, #[case] rpe: RPE, #[case] expected: f32) {
        assert_approx_eq!(reps.including_rir(rpe), expected);
    }

    #[rstest]
    #[case(0, Ok(Time(0)))]
    #[case(999, Ok(Time(999)))]
    #[case(1000, Err(TimeError::OutOfRange))]
    fn test_time_new(#[case] input: u32, #[case] expected: Result<Time, TimeError>) {
        assert_eq!(Time::new(input), expected);
    }

    #[rstest]
    #[case("0", Ok(Time(0)))]
    #[case("999", Ok(Time(999)))]
    #[case("1000", Err(TimeError::OutOfRange))]
    #[case("4.", Err(TimeError::ParseError))]
    #[case("", Err(TimeError::ParseError))]
    fn test_time_from_str(#[case] input: &str, #[case] expected: Result<Time, TimeError>) {
        assert_eq!(Time::try_from(input), expected);
    }

    #[rstest]
    fn test_time_mul_reps() {
        assert_eq!(Time(2) * Reps(4), Time(8));
    }

    #[rstest]
    #[case(Time(8), "8")]
    fn test_time_display(#[case] input: Time, #[case] expected: &str) {
        assert_eq!(input.to_string(), expected);
    }

    #[rstest]
    #[case(0.0, Ok(Weight(0.0)))]
    #[case(999.9, Ok(Weight(999.9)))]
    #[case(1000.0, Err(WeightError::OutOfRange))]
    #[case(1.23, Err(WeightError::InvalidResolution))]
    fn test_weight_new(#[case] input: f32, #[case] expected: Result<Weight, WeightError>) {
        assert_eq!(Weight::new(input), expected);
    }

    #[rstest]
    #[case("2.0", Ok(Weight(2.0)))]
    #[case("4.", Ok(Weight(4.0)))]
    #[case("8", Ok(Weight(8.0)))]
    #[case("1000", Err(WeightError::OutOfRange))]
    #[case("", Err(WeightError::ParseError))]
    fn test_weight_from_str(#[case] input: &str, #[case] expected: Result<Weight, WeightError>) {
        assert_eq!(Weight::try_from(input), expected);
    }

    #[rstest]
    #[case(Weight(2.0), "2")]
    #[case(Weight(8.4), "8.4")]
    fn test_weight_display(#[case] input: Weight, #[case] expected: &str) {
        assert_eq!(input.to_string(), expected);
    }

    #[rstest]
    #[case(0.0, Ok(RPE::ZERO))]
    #[case(8.0, Ok(RPE::EIGHT))]
    #[case(9.5, Ok(RPE(95)))]
    #[case(10.0, Ok(RPE::TEN))]
    fn test_rpe_new(#[case] input: f32, #[case] expected: Result<RPE, RPEError>) {
        assert_eq!(RPE::new(input), expected);
    }

    #[rstest]
    #[case("2.0", Ok(RPE::TWO))]
    #[case("4.", Ok(RPE::FOUR))]
    #[case("8", Ok(RPE::EIGHT))]
    #[case("11", Err(RPEError::OutOfRange))]
    #[case("9.2", Err(RPEError::InvalidResolution))]
    #[case("", Err(RPEError::ParseError))]
    fn test_rpe_from_str(#[case] input: &str, #[case] expected: Result<RPE, RPEError>) {
        assert_eq!(RPE::try_from(input), expected);
    }

    #[rstest]
    #[case(RPE::EIGHT, "8")]
    #[case(RPE(95), "9.5")]
    fn test_rpe_display(#[case] input: RPE, #[case] expected: &str) {
        assert_eq!(input.to_string(), expected);
    }

    #[rstest]
    #[case(RPE(0), RIR(100))]
    #[case(RPE(50), RIR(50))]
    #[case(RPE(80), RIR(20))]
    #[case(RPE(100), RIR(0))]
    fn test_rir_from_rpe(#[case] rpe: RPE, #[case] expected: RIR) {
        assert_eq!(RIR::from(rpe), expected);
    }

    #[rstest]
    #[case(RIR(20), "2")]
    #[case(RIR(25), "2.5")]
    fn test_rir_display(#[case] input: RIR, #[case] expected: &str) {
        assert_eq!(input.to_string(), expected);
    }

    #[rstest]
    #[case::no_load_ratio(vec![], vec![], None)]
    #[case::load_ratio(
        vec![(from_num_days(0), 12.0), (from_num_days(1), 10.0)],
        vec![(from_num_days(0), 10.0), (from_num_days(1), 8.0)],
        Some(1.25)
    )]
    fn test_training_stats_load_ratio(
        #[case] short_term_load: Vec<(NaiveDate, f32)>,
        #[case] long_term_load: Vec<(NaiveDate, f32)>,
        #[case] expected: Option<f32>,
    ) {
        assert_eq!(
            TrainingStats {
                short_term_load,
                long_term_load,
            }
            .load_ratio(),
            expected
        );
    }

    #[test]
    fn test_training_stats_clear() {
        let mut training_stats = TrainingStats {
            short_term_load: vec![(from_num_days(0), 10.0)],
            long_term_load: vec![(from_num_days(0), 8.0)],
        };

        assert!(!training_stats.short_term_load.is_empty());
        assert!(!training_stats.long_term_load.is_empty());

        training_stats.clear();

        assert!(training_stats.short_term_load.is_empty());
        assert!(training_stats.long_term_load.is_empty());
    }

    #[rstest]
    #[case::no_sessions(&[], vec![(*TODAY, 0.0)], vec![])]
    #[case::one_session(
        &[&*TRAINING_SESSION],
        vec![
            (*TODAY - Duration::days(10), 10.0),
            (*TODAY - Duration::days(9), 8.571_428),
            (*TODAY - Duration::days(8), 7.142_857_6),
            (*TODAY - Duration::days(7), 5.714_285_4),
            (*TODAY - Duration::days(6), 4.285_714),
            (*TODAY - Duration::days(5), 2.857_142_7),
            (*TODAY - Duration::days(4), 1.428_570_7),
            (*TODAY - Duration::days(3), 0.0),
            (*TODAY - Duration::days(2), 0.0),
            (*TODAY - Duration::days(1), 0.0),
            (*TODAY, 0.0),
        ],
        vec![]
    )]
    fn test_training_stats(
        #[case] training_sessions: &[&TrainingSession],
        #[case] short_term_load: Vec<(NaiveDate, f32)>,
        #[case] long_term_load: Vec<(NaiveDate, f32)>,
    ) {
        assert_eq!(
            training_stats(training_sessions),
            TrainingStats {
                short_term_load,
                long_term_load
            }
        );
    }

    #[rstest]
    #[case::no_load(&[], 2, vec![])]
    #[case::load(
        &[
            (from_num_days(0), 10.0),
            (from_num_days(1), 8.0),
            (from_num_days(2), 6.0),
            (from_num_days(3), 4.0),
            (from_num_days(4), 2.0),
            (from_num_days(5), 0.0),
            (from_num_days(6), 0.0),
            (from_num_days(7), 0.0),
        ],
        3,
        vec![
            (from_num_days(2), 8.0),
            (from_num_days(3), 6.0),
            (from_num_days(4), 4.0),
            (from_num_days(5), 2.0),
            (from_num_days(6), 0.666_666_7),
            (from_num_days(7), 0.0),
        ]
    )]
    fn test_average_weighted_sum_of_load(
        #[case] weighted_sum_of_load: &[(NaiveDate, f32)],
        #[case] window_size: usize,
        #[case] expected: Vec<(NaiveDate, f32)>,
    ) {
        assert_eq!(
            average_weighted_sum_of_load(weighted_sum_of_load, window_size),
            expected
        );
    }

    #[rstest]
    #[case(1.0, 100.0, 100.0)]
    #[case(5.0, 100.0, 112.5)]
    #[case(10.0, 100.0, 129.2)]
    #[case(30.0, 100.0, 183.6)]
    fn test_one_rep_max(#[case] reps: f32, #[case] weight: f32, #[case] expected: f32) {
        assert_approx_eq!(one_rep_max(reps, weight), expected, 0.05);
    }

    #[rstest]
    #[case(100.0, 1.0)]
    #[case(97.2, 2.0)]
    #[case(86.1, 6.0)]
    #[case(77.4, 10.0)]
    #[case(61.7, 19.0)]
    #[case(60.8, 20.0)]
    #[case(54.5, 30.0)]
    fn test_reps_for_percentage(#[case] percentage: f32, #[case] expected: f32) {
        assert_approx_eq!(reps_for_percentage(percentage), expected, 0.1);
    }

    #[rstest]
    #[case(150.0)]
    #[case(105.0)]
    fn test_reps_for_percentage_clamped_above_one_rep_max(#[case] percentage: f32) {
        assert_approx_eq!(reps_for_percentage(percentage), 1.0);
    }

    #[rstest]
    #[case(1.0)]
    #[case(2.5)]
    #[case(5.0)]
    #[case(6.0)]
    #[case(7.0)]
    #[case(12.0)]
    #[case(19.0)]
    #[case(20.0)]
    #[case(30.0)]
    #[case(50.0)]
    fn test_one_rep_max_and_reps_for_percentage_are_inverses(#[case] reps: f32) {
        let weight = 100.0;
        let one_rm = one_rep_max(reps, weight);
        let percentage = 100.0 * weight / one_rm;
        assert_approx_eq!(reps_for_percentage(percentage), reps, 0.05);
    }

    fn from_num_days(days: i32) -> NaiveDate {
        NaiveDate::from_num_days_from_ce_opt(days).unwrap()
    }
}
