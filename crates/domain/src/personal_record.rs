use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    iter,
};

use chrono::{Duration, NaiveDate};

use crate::{
    Assistance, Exercise, ExerciseID, Side, TrainingSession, TrainingSessionElement,
    TrainingSessionID, training_session::is_recorded,
};

/// The number of days before the date of a session within which earlier sessions count for a
/// recent record.
const RECENT_RECORD_DAYS: i64 = 182;

/// The value of a set a record is kept for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecordValue {
    Reps,
    Time,
    Weight,
    OneRepMax,
}

/// The kind of a record set, an all-time record set ranking above a recent one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecordKind {
    Recent,
    AllTime,
}

/// The greatest value with the first session holding it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Record {
    pub value: f32,
    pub session_id: TrainingSessionID,
    pub date: NaiveDate,
}

/// The all-time and the recent record of a value and side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Records {
    pub all_time: Record,
    pub recent: Option<Record>,
}

/// The records and the record sets of all exercises, a set without a side counting for both
/// sides.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PersonalRecords {
    /// The records of the left and the right side.
    records: HashMap<(ExerciseID, Side, RecordValue), History>,
    record_sets: BTreeMap<TrainingSessionID, BTreeMap<usize, RecordKind>>,
    /// The exercises with a recorded set on a side.
    sided: HashSet<ExerciseID>,
}

impl PersonalRecords {
    #[must_use]
    pub fn new(training_sessions: &[TrainingSession], exercises: &[Exercise]) -> Self {
        let exclusions = exercises
            .iter()
            .map(|e| (e.id, Exclusion::of(e)))
            .collect::<HashMap<_, _>>();
        let mut sessions = training_sessions.iter().collect::<Vec<_>>();
        sessions.sort_by_key(|s| (s.date, s.id));

        let mut result = Self::default();
        for session in sessions {
            let window_start = session.date - Duration::days(RECENT_RECORD_DAYS);
            for element in &session.elements {
                if let TrainingSessionElement::Set {
                    exercise_id, side, ..
                } = element
                    && *side != Side::Unset
                    && !values(
                        element,
                        exclusions.get(exercise_id).copied().unwrap_or_default(),
                    )
                    .is_empty()
                {
                    result.sided.insert(*exercise_id);
                }
            }
            let mut record_sets = BTreeMap::new();
            for (key, (value, element_idx)) in session_bests(session, &exclusions) {
                let record = Record {
                    value,
                    session_id: session.id,
                    date: session.date,
                };
                let history = result.records.entry(key).or_default();
                if let Some(kind) = history.add(record, window_start) {
                    record_sets
                        .entry(element_idx)
                        .and_modify(|k: &mut RecordKind| *k = (*k).max(kind))
                        .or_insert(kind);
                }
            }
            if !record_sets.is_empty() {
                result.record_sets.insert(session.id, record_sets);
            }
        }
        result
    }

    /// The all-time and the recent records of an exercise per side and value, the recent records
    /// including the sessions of `today`.
    ///
    /// The records are without a side if the exercise has no recorded set on a side.
    #[must_use]
    pub fn records(
        &self,
        exercise_id: ExerciseID,
        today: NaiveDate,
    ) -> BTreeMap<(Side, RecordValue), Records> {
        let window_start = today - Duration::days(RECENT_RECORD_DAYS);
        let has_sides = self.sided.contains(&exercise_id);
        self.records
            .iter()
            .filter(|((id, side, _), _)| *id == exercise_id && (has_sides || *side == Side::Left))
            .filter_map(|((_, side, value), history)| {
                Some((
                    (if has_sides { *side } else { Side::Unset }, *value),
                    Records {
                        all_time: history.all_time?,
                        recent: history.recent(window_start),
                    },
                ))
            })
            .collect()
    }

    /// The record sets of a session as element indices with their kind.
    #[must_use]
    pub fn record_sets(&self, session_id: TrainingSessionID) -> BTreeMap<usize, RecordKind> {
        self.record_sets
            .get(&session_id)
            .cloned()
            .unwrap_or_default()
    }
}

/// The sides whose records a set on `side` takes part in.
fn record_sides(side: Side) -> &'static [Side] {
    match side {
        Side::Unset => &[Side::Left, Side::Right],
        Side::Left => &[Side::Left],
        Side::Right => &[Side::Right],
    }
}

/// The values an exercise has no records of.
#[derive(Debug, Clone, Copy, Default)]
struct Exclusion {
    weighted: bool,
    all: bool,
}

impl Exclusion {
    fn of(exercise: &Exercise) -> Self {
        Self {
            weighted: exercise.assistance == Some(Assistance::Assisted),
            all: exercise.is_stretch_only(),
        }
    }
}

/// The records of an exercise, side and value up to the last session added.
#[derive(Debug, Clone, Default, PartialEq)]
struct History {
    all_time: Option<Record>,
    /// The session bests that may still become the greatest of a window, with values that do not
    /// increase from front to back, so the front is the earliest session holding the greatest
    /// value.
    window: VecDeque<Record>,
}

impl History {
    /// Adds the best of a session, which follows all sessions added before, and returns the kind
    /// of record it sets.
    fn add(&mut self, record: Record, window_start: NaiveDate) -> Option<RecordKind> {
        while self.window.front().is_some_and(|r| r.date < window_start) {
            self.window.pop_front();
        }
        let kind = match (self.all_time, self.window.front()) {
            (Some(all_time), _) if record.value > all_time.value => Some(RecordKind::AllTime),
            (_, Some(recent)) if record.value > recent.value => Some(RecordKind::Recent),
            _ => None,
        };
        if self.all_time.is_none() || kind == Some(RecordKind::AllTime) {
            self.all_time = Some(record);
        }
        while self.window.back().is_some_and(|r| r.value < record.value) {
            self.window.pop_back();
        }
        self.window.push_back(record);
        kind
    }

    fn recent(&self, window_start: NaiveDate) -> Option<Record> {
        self.window.iter().find(|r| r.date >= window_start).copied()
    }
}

/// The greatest value of a session per exercise, record side and value, with the index of the
/// first element holding it.
fn session_bests(
    session: &TrainingSession,
    exclusions: &HashMap<ExerciseID, Exclusion>,
) -> BTreeMap<(ExerciseID, Side, RecordValue), (f32, usize)> {
    let mut result: BTreeMap<_, (f32, usize)> = BTreeMap::new();
    for (element_idx, element) in session.elements.iter().enumerate() {
        let TrainingSessionElement::Set {
            exercise_id, side, ..
        } = element
        else {
            continue;
        };
        let exclusion = exclusions.get(exercise_id).copied().unwrap_or_default();
        for (record_value, value) in values(element, exclusion) {
            for record_side in record_sides(*side) {
                result
                    .entry((*exercise_id, *record_side, record_value))
                    .and_modify(|best| {
                        if value > best.0 {
                            *best = (value, element_idx);
                        }
                    })
                    .or_insert((value, element_idx));
            }
        }
    }
    result
}

/// The values of a set that take part in the records.
fn values(element: &TrainingSessionElement, exclusion: Exclusion) -> Vec<(RecordValue, f32)> {
    let TrainingSessionElement::Set {
        reps, time, weight, ..
    } = element
    else {
        return vec![];
    };
    if exclusion.all || !is_recorded(element) {
        return vec![];
    }
    #[allow(clippy::cast_precision_loss)]
    match (weight.non_zero(), reps.non_zero()) {
        (Some(_), _) if exclusion.weighted => vec![],
        (Some(weight), _) => iter::once((RecordValue::Weight, f32::from(weight)))
            .chain(
                element
                    .one_rep_max()
                    .map(|one_rep_max| (RecordValue::OneRepMax, one_rep_max)),
            )
            .collect(),
        (None, Some(reps)) => vec![(RecordValue::Reps, u32::from(reps) as f32)],
        (None, None) => time
            .non_zero()
            .map(|time| (RecordValue::Time, u32::from(time) as f32))
            .into_iter()
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use crate::{ExerciseMuscle, MuscleID, Name, RPE, Reps, RoutineID, Stimulus, Time, Weight};

    use super::*;

    const EXERCISE_ID: u128 = 1;

    #[test]
    fn test_set_with_only_a_weight_takes_no_part() {
        let records = personal_records(
            &[
                session(1, 0, &[lift(5, 40.0)]),
                session(2, 1, &[lift(0, 100.0)]),
            ],
            &[],
        );

        assert_eq!(records.record_sets(2.into()), BTreeMap::new());
        assert_eq!(
            values(&records, 1),
            BTreeMap::from([
                ((Side::Unset, RecordValue::Weight), (40.0, Some(40.0))),
                ((Side::Unset, RecordValue::OneRepMax), (45.0, Some(45.0))),
            ])
        );
    }

    #[rstest]
    #[case::reps(set(Side::Unset, 5, 0, 50.0, RPE::ZERO))]
    #[case::reps_and_time(set(Side::Unset, 5, 3, 50.0, RPE::ZERO))]
    fn test_weighted_set_sets_a_weight_and_an_estimated_one_rep_max_record(
        #[case] element: TrainingSessionElement,
    ) {
        let records = personal_records(&[session(1, 0, &[element])], &[]);

        assert_eq!(
            keys(&records, 0),
            [
                (Side::Unset, RecordValue::Weight),
                (Side::Unset, RecordValue::OneRepMax)
            ]
        );
    }

    #[test]
    fn test_weighted_hold_sets_a_weight_record_only() {
        let records = personal_records(
            &[session(1, 0, &[set(Side::Unset, 0, 30, 20.0, RPE::ZERO)])],
            &[],
        );

        assert_eq!(keys(&records, 0), [(Side::Unset, RecordValue::Weight)]);
    }

    #[test]
    fn test_estimated_one_rep_max_and_weight_records_are_set_separately() {
        let records = personal_records(
            &[
                session(1, 0, &[lift(5, 100.0)]),
                session(2, 1, &[lift(10, 90.0)]),
                session(3, 2, &[lift(1, 105.0)]),
            ],
            &[],
        );

        assert_eq!(
            records.record_sets(2.into()),
            BTreeMap::from([(0, RecordKind::AllTime)])
        );
        assert_eq!(
            records.record_sets(3.into()),
            BTreeMap::from([(0, RecordKind::AllTime)])
        );
        let all_time = records.records(EXERCISE_ID.into(), date(2));
        assert_eq!(
            all_time[&(Side::Unset, RecordValue::Weight)]
                .all_time
                .session_id,
            3.into()
        );
        assert_eq!(
            all_time[&(Side::Unset, RecordValue::OneRepMax)]
                .all_time
                .session_id,
            2.into()
        );
    }

    #[test]
    fn test_rpe_raises_the_estimated_one_rep_max() {
        let records = personal_records(
            &[
                session(1, 0, &[set(Side::Unset, 5, 0, 100.0, RPE::TEN)]),
                session(2, 1, &[set(Side::Unset, 5, 0, 100.0, RPE::EIGHT)]),
            ],
            &[],
        );

        assert_eq!(
            records.record_sets(2.into()),
            BTreeMap::from([(0, RecordKind::AllTime)])
        );
        assert_approx_eq!(
            values(&records, 1)[&(Side::Unset, RecordValue::OneRepMax)].0,
            crate::one_rep_max(7.0, 100.0)
        );
    }

    #[test]
    fn test_set_with_reps_and_a_time_sets_no_time_record() {
        let records = personal_records(
            &[session(1, 0, &[set(Side::Unset, 5, 3, 0.0, RPE::ZERO)])],
            &[],
        );

        assert_eq!(keys(&records, 0), [(Side::Unset, RecordValue::Reps)]);
    }

    #[test]
    fn test_each_value_has_its_own_record() {
        let records = personal_records(
            &[
                session(1, 0, &[reps(10), hold(30)]),
                session(2, 1, &[reps(8), hold(40)]),
            ],
            &[],
        );

        assert_eq!(
            records.record_sets(2.into()),
            BTreeMap::from([(1, RecordKind::AllTime)])
        );
        assert_eq!(
            values(&records, 1),
            BTreeMap::from([
                ((Side::Unset, RecordValue::Reps), (10.0, Some(10.0))),
                ((Side::Unset, RecordValue::Time), (40.0, Some(40.0))),
            ])
        );
    }

    #[test]
    fn test_tie_does_not_beat_a_record() {
        let records = personal_records(
            &[session(1, 0, &[reps(10)]), session(2, 1, &[reps(10)])],
            &[],
        );

        assert_eq!(records.record_sets(2.into()), BTreeMap::new());
        assert_eq!(
            records.records(EXERCISE_ID.into(), date(1))[&(Side::Unset, RecordValue::Reps)]
                .all_time
                .session_id,
            1.into()
        );
    }

    #[test]
    fn test_set_below_an_rpe_of_seven_sets_a_record() {
        let records = personal_records(
            &[
                session(1, 0, &[reps(10)]),
                session(2, 1, &[set(Side::Unset, 11, 0, 0.0, RPE::FIVE)]),
            ],
            &[],
        );

        assert_eq!(
            records.record_sets(2.into()),
            BTreeMap::from([(0, RecordKind::AllTime)])
        );
    }

    #[test]
    fn test_sides_are_kept_apart() {
        let records = personal_records(
            &[
                session(
                    1,
                    0,
                    &[sided_reps(Side::Left, 10), sided_reps(Side::Right, 8)],
                ),
                session(
                    2,
                    1,
                    &[sided_reps(Side::Left, 9), sided_reps(Side::Right, 9)],
                ),
            ],
            &[],
        );

        assert_eq!(
            records.record_sets(2.into()),
            BTreeMap::from([(1, RecordKind::AllTime)])
        );
        assert_eq!(
            values(&records, 1),
            BTreeMap::from([
                ((Side::Left, RecordValue::Reps), (10.0, Some(10.0))),
                ((Side::Right, RecordValue::Reps), (9.0, Some(9.0))),
            ])
        );
    }

    #[test]
    fn test_set_without_a_side_counts_for_both_sides() {
        let records = personal_records(
            &[
                session(1, 0, &[reps(12)]),
                session(
                    2,
                    1,
                    &[sided_reps(Side::Left, 11), sided_reps(Side::Right, 13)],
                ),
            ],
            &[],
        );

        assert_eq!(
            records.record_sets(2.into()),
            BTreeMap::from([(1, RecordKind::AllTime)])
        );
        assert_eq!(
            values(&records, 1),
            BTreeMap::from([
                ((Side::Left, RecordValue::Reps), (12.0, Some(12.0))),
                ((Side::Right, RecordValue::Reps), (13.0, Some(13.0))),
            ])
        );
    }

    #[test]
    fn test_set_without_a_side_beating_one_side_is_a_record_set() {
        let records = personal_records(
            &[
                session(
                    1,
                    0,
                    &[sided_reps(Side::Left, 10), sided_reps(Side::Right, 14)],
                ),
                session(2, 1, &[reps(12)]),
            ],
            &[],
        );

        assert_eq!(
            records.record_sets(2.into()),
            BTreeMap::from([(0, RecordKind::AllTime)])
        );
        assert_eq!(
            values(&records, 1),
            BTreeMap::from([
                ((Side::Left, RecordValue::Reps), (12.0, Some(12.0))),
                ((Side::Right, RecordValue::Reps), (14.0, Some(14.0))),
            ])
        );
    }

    #[test]
    fn test_records_are_per_side_from_the_first_recorded_set_on_a_side() {
        let records = personal_records(
            &[
                session(1, 0, &[reps(10)]),
                session(2, 1, &[sided_reps(Side::Left, 0)]),
            ],
            &[],
        );

        assert_eq!(keys(&records, 1), [(Side::Unset, RecordValue::Reps)]);
    }

    #[test]
    fn test_first_session_marks_no_set_while_holding_the_records() {
        let records = personal_records(&[session(1, 0, &[reps(10)])], &[]);

        assert_eq!(records.record_sets(1.into()), BTreeMap::new());
        assert_eq!(
            records.records(EXERCISE_ID.into(), date(0)),
            BTreeMap::from([(
                (Side::Unset, RecordValue::Reps),
                Records {
                    all_time: record(10.0, 1, 0),
                    recent: Some(record(10.0, 1, 0)),
                }
            )])
        );
    }

    #[test]
    fn test_first_value_of_a_kind_in_a_later_session_marks_no_set() {
        let records = personal_records(
            &[session(1, 0, &[reps(10)]), session(2, 1, &[hold(30)])],
            &[],
        );

        assert_eq!(records.record_sets(2.into()), BTreeMap::new());
        assert_eq!(
            values(&records, 1)[&(Side::Unset, RecordValue::Time)],
            (30.0, Some(30.0))
        );
    }

    #[test]
    fn test_session_improving_twice_marks_its_best_set_only() {
        let records = personal_records(
            &[
                session(1, 0, &[hold(6)]),
                session(2, 1, &[hold(7), hold(8)]),
            ],
            &[],
        );

        assert_eq!(
            records.record_sets(2.into()),
            BTreeMap::from([(1, RecordKind::AllTime)])
        );
    }

    #[test]
    fn test_first_of_two_sets_equal_to_the_session_best_is_marked() {
        let records = personal_records(
            &[
                session(1, 0, &[hold(6)]),
                session(2, 1, &[hold(8), hold(8)]),
            ],
            &[],
        );

        assert_eq!(
            records.record_sets(2.into()),
            BTreeMap::from([(0, RecordKind::AllTime)])
        );
    }

    #[test]
    fn test_sessions_of_the_same_date_are_ordered_by_id() {
        let records = personal_records(
            &[session(2, 0, &[reps(12)]), session(1, 0, &[reps(10)])],
            &[],
        );

        assert_eq!(records.record_sets(1.into()), BTreeMap::new());
        assert_eq!(
            records.record_sets(2.into()),
            BTreeMap::from([(0, RecordKind::AllTime)])
        );
    }

    #[test]
    fn test_recent_record_set_below_the_all_time_record() {
        let records = personal_records(
            &[
                session(1, 0, &[reps(20)]),
                session(2, 300, &[reps(10)]),
                session(3, 310, &[reps(12)]),
            ],
            &[],
        );

        assert_eq!(records.record_sets(2.into()), BTreeMap::new());
        assert_eq!(
            records.record_sets(3.into()),
            BTreeMap::from([(0, RecordKind::Recent)])
        );
        assert_eq!(
            records.records(EXERCISE_ID.into(), date(310)),
            BTreeMap::from([(
                (Side::Unset, RecordValue::Reps),
                Records {
                    all_time: record(20.0, 1, 0),
                    recent: Some(record(12.0, 3, 310)),
                }
            )])
        );
    }

    #[test]
    fn test_set_beating_both_records_is_marked_all_time() {
        let records = personal_records(
            &[
                session(1, 0, &[reps(10)]),
                session(2, 300, &[reps(8)]),
                session(3, 310, &[reps(12)]),
            ],
            &[],
        );

        assert_eq!(
            records.record_sets(3.into()),
            BTreeMap::from([(0, RecordKind::AllTime)])
        );
    }

    #[test]
    fn test_set_with_an_all_time_weight_and_a_recent_estimated_one_rep_max_record_is_marked_all_time()
     {
        let records = personal_records(
            &[
                session(1, 0, &[lift(10, 150.0)]),
                session(2, 300, &[lift(5, 100.0)]),
                session(3, 310, &[lift(1, 160.0)]),
            ],
            &[],
        );

        assert_eq!(
            records.record_sets(3.into()),
            BTreeMap::from([(0, RecordKind::AllTime)])
        );
        let records = records.records(EXERCISE_ID.into(), date(310));
        assert_eq!(
            records[&(Side::Unset, RecordValue::Weight)].all_time,
            record(160.0, 3, 310)
        );
        assert_eq!(
            records[&(Side::Unset, RecordValue::OneRepMax)].recent,
            Some(record(160.0, 3, 310))
        );
        assert_eq!(
            records[&(Side::Unset, RecordValue::OneRepMax)]
                .all_time
                .session_id,
            1.into()
        );
    }

    #[rstest]
    #[case::session_182_days_before(182, Some(RecordKind::Recent))]
    #[case::break_longer_than_the_window(183, None)]
    fn test_window_of_recent_records(#[case] day: i64, #[case] expected: Option<RecordKind>) {
        let records = personal_records(
            &[
                session(1, -100, &[reps(20)]),
                session(2, 0, &[reps(10)]),
                session(3, day, &[reps(11)]),
            ],
            &[],
        );

        assert_eq!(
            records.record_sets(3.into()),
            expected
                .map(|kind| BTreeMap::from([(0, kind)]))
                .unwrap_or_default()
        );
    }

    #[rstest]
    #[case::session_of_today(300, Some(8.0))]
    #[case::session_182_days_before(482, Some(8.0))]
    #[case::session_183_days_before(483, None)]
    fn test_recent_records_end_today(#[case] today: i64, #[case] expected: Option<f32>) {
        let records = personal_records(
            &[session(1, 0, &[reps(10)]), session(2, 300, &[reps(8)])],
            &[],
        );

        assert_eq!(
            values(&records, today),
            BTreeMap::from([((Side::Unset, RecordValue::Reps), (10.0, expected))])
        );
    }

    #[test]
    fn test_earlier_session_of_the_same_date_lies_within_the_window() {
        let records = personal_records(
            &[
                session(1, 0, &[reps(20)]),
                session(2, 200, &[reps(10)]),
                session(3, 200, &[reps(11)]),
            ],
            &[],
        );

        assert_eq!(
            records.record_sets(3.into()),
            BTreeMap::from([(0, RecordKind::Recent)])
        );
    }

    #[test]
    fn test_recent_record_held_by_two_sessions_is_that_of_the_first() {
        let records = personal_records(
            &[
                session(1, 0, &[reps(20)]),
                session(2, 200, &[reps(10)]),
                session(3, 210, &[reps(10)]),
            ],
            &[],
        );

        assert_eq!(
            records.records(EXERCISE_ID.into(), date(210))[&(Side::Unset, RecordValue::Reps)]
                .recent,
            Some(record(10.0, 2, 200))
        );
    }

    #[test]
    fn test_assisted_exercise_has_no_weight_or_estimated_one_rep_max_record() {
        let mut exercise = exercise(vec![]);
        exercise.assistance = Some(Assistance::Assisted);
        let records = personal_records(
            &[
                session(1, 0, &[lift(10, 20.0), reps(10)]),
                session(2, 1, &[lift(12, 30.0), reps(11)]),
            ],
            &[exercise],
        );

        assert_eq!(keys(&records, 1), [(Side::Unset, RecordValue::Reps)]);
        assert_eq!(
            records.record_sets(2.into()),
            BTreeMap::from([(1, RecordKind::AllTime)])
        );
    }

    #[test]
    fn test_stretch_only_exercise_has_no_records() {
        let exercise = exercise(vec![ExerciseMuscle {
            muscle_id: MuscleID::Pecs,
            stimulus: Stimulus::NONE,
            stretch: Stimulus::PRIMARY,
        }]);
        let records = personal_records(
            &[session(1, 0, &[hold(30)]), session(2, 1, &[hold(40)])],
            &[exercise],
        );

        assert_eq!(keys(&records, 1), []);
        assert_eq!(records.record_sets(2.into()), BTreeMap::new());
    }

    fn personal_records(sessions: &[TrainingSession], exercises: &[Exercise]) -> PersonalRecords {
        PersonalRecords::new(sessions, exercises)
    }

    fn keys(records: &PersonalRecords, today: i64) -> Vec<(Side, RecordValue)> {
        records
            .records(EXERCISE_ID.into(), date(today))
            .into_keys()
            .collect()
    }

    fn values(
        records: &PersonalRecords,
        today: i64,
    ) -> BTreeMap<(Side, RecordValue), (f32, Option<f32>)> {
        records
            .records(EXERCISE_ID.into(), date(today))
            .into_iter()
            .map(|(key, r)| (key, (r.all_time.value, r.recent.map(|r| r.value))))
            .collect()
    }

    fn record(value: f32, session_id: u128, day: i64) -> Record {
        Record {
            value,
            session_id: session_id.into(),
            date: date(day),
        }
    }

    fn date(day: i64) -> NaiveDate {
        NaiveDate::from_ymd_opt(2025, 1, 1).unwrap() + Duration::days(day)
    }

    fn session(id: u128, day: i64, elements: &[TrainingSessionElement]) -> TrainingSession {
        TrainingSession {
            id: id.into(),
            routine_id: RoutineID::nil(),
            date: date(day),
            notes: String::new(),
            elements: elements.to_vec(),
            exercise_notes: BTreeMap::new(),
            exercise_measurements: BTreeMap::new(),
        }
    }

    fn exercise(muscles: Vec<ExerciseMuscle>) -> Exercise {
        Exercise {
            id: EXERCISE_ID.into(),
            name: Name::new("A").unwrap(),
            notes: String::new(),
            muscles,
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        }
    }

    fn reps(reps: u32) -> TrainingSessionElement {
        set(Side::Unset, reps, 0, 0.0, RPE::ZERO)
    }

    fn sided_reps(side: Side, reps: u32) -> TrainingSessionElement {
        set(side, reps, 0, 0.0, RPE::ZERO)
    }

    fn hold(time: u32) -> TrainingSessionElement {
        set(Side::Unset, 0, time, 0.0, RPE::ZERO)
    }

    fn lift(reps: u32, weight: f32) -> TrainingSessionElement {
        set(Side::Unset, reps, 0, weight, RPE::ZERO)
    }

    fn set(side: Side, reps: u32, time: u32, weight: f32, rpe: RPE) -> TrainingSessionElement {
        TrainingSessionElement::Set {
            exercise_id: EXERCISE_ID.into(),
            side,
            reps: Reps::new(reps).unwrap(),
            time: Time::new(time).unwrap(),
            weight: Weight::new(weight).unwrap(),
            rpe,
            target_reps: Reps::default(),
            target_tempo: crate::Tempo::default(),
            target_weight: Weight::default(),
            target_rpe: RPE::default(),
            automatic: false,
        }
    }
}
