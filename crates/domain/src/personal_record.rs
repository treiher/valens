use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    iter,
};

use chrono::{Duration, NaiveDate};

use crate::{
    Assistance, Exercise, ExerciseID, RPE, Reps, Side, Time, TrainingSession,
    TrainingSessionElement, TrainingSessionID, Weight, training::fewest_reps_for_one_rep_max,
    training_session::is_recorded,
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

/// The values a set needs for a recent record.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RecordTargets {
    pub reps: Option<Reps>,
    pub time: Option<Time>,
}

/// The records and the record sets of all exercises, a set without a side counting for the left
/// and the right side and an alternating set for the alternating side only.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PersonalRecords {
    /// The records of the left, the right and the alternating side.
    records: HashMap<(ExerciseID, Side, RecordValue), History>,
    record_sets: BTreeMap<TrainingSessionID, BTreeMap<usize, RecordKind>>,
    exclusions: HashMap<ExerciseID, Exclusion>,
    /// The exercises with a recorded set on the left or the right side.
    sided: HashSet<ExerciseID>,
    /// The session whose sets have targets, with the greatest value per value of the earlier
    /// sessions within its window per exercise and side.
    recent_records_before: Option<(TrainingSessionID, RecentRecordsBefore)>,
}

type RecentRecordsBefore = BTreeMap<(ExerciseID, Side), BTreeMap<RecordValue, f32>>;

impl PersonalRecords {
    #[must_use]
    pub fn new(training_sessions: &[TrainingSession], exercises: &[Exercise]) -> Self {
        Self::build(training_sessions, exercises, None)
    }

    /// The records like [`PersonalRecords::new`], providing the targets of the sets of the session
    /// with `session_id`.
    #[must_use]
    pub fn with_targets(
        training_sessions: &[TrainingSession],
        exercises: &[Exercise],
        session_id: TrainingSessionID,
    ) -> Self {
        Self::build(training_sessions, exercises, Some(session_id))
    }

    /// The all-time and the recent records of an exercise per side and value, the recent records
    /// including the sessions of `today`.
    ///
    /// The records of the left and the right side are without a side if the exercise has no
    /// recorded set on the left or the right side. The records of the alternating side are always
    /// kept under [`Side::Alternating`].
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
            .filter(|((id, side, _), _)| {
                *id == exercise_id && (has_sides || matches!(side, Side::Left | Side::Alternating))
            })
            .filter_map(|((_, side, value), history)| {
                Some((
                    (
                        if has_sides || *side == Side::Alternating {
                            *side
                        } else {
                            Side::Unset
                        },
                        *value,
                    ),
                    Records {
                        all_time: history.all_time?,
                        recent: history.recent(window_start),
                    },
                ))
            })
            .collect()
    }

    /// The values the set at `element_idx` of `session` needs on its side with its weight and RPE
    /// for a recent record.
    ///
    /// A target beats the recent records before the session and the sets of the session before
    /// the set, and reaches the sets after it. A set without a side takes the smaller target of
    /// both sides. A recorded set and the sets of any session other than the one given to
    /// [`PersonalRecords::with_targets`] have no targets.
    #[must_use]
    pub fn targets(&self, session: &TrainingSession, element_idx: usize) -> RecordTargets {
        let Some(
            element @ TrainingSessionElement::Set {
                exercise_id,
                side,
                weight,
                rpe,
                ..
            },
        ) = session.elements.get(element_idx)
        else {
            return RecordTargets::default();
        };
        let Some((session_id, recent_records_before)) = &self.recent_records_before else {
            return RecordTargets::default();
        };
        if *session_id != session.id || is_recorded(element) {
            return RecordTargets::default();
        }
        let exclusion = self.exclusion(*exercise_id);
        record_sides(*side)
            .iter()
            .map(|record_side| {
                let mut to_beat = recent_records_before
                    .get(&(*exercise_id, *record_side))
                    .cloned()
                    .unwrap_or_default();
                let mut to_reach: BTreeMap<RecordValue, f32> = BTreeMap::new();
                for (idx, other) in session.elements.iter().enumerate() {
                    if idx == element_idx
                        || !matches!(
                            other,
                            TrainingSessionElement::Set { exercise_id: id, side: s, .. }
                                if id == exercise_id && record_sides(*s).contains(record_side)
                        )
                    {
                        continue;
                    }
                    for (record_value, value) in values(other, exclusion) {
                        if idx > element_idx {
                            let to_reach = to_reach.entry(record_value).or_insert(value);
                            *to_reach = to_reach.max(value);
                        } else if let Some(to_beat) = to_beat.get_mut(&record_value) {
                            *to_beat = to_beat.max(value);
                        }
                    }
                }
                record_targets(&to_beat, &to_reach, *weight, *rpe, *side)
            })
            .reduce(|a, b| RecordTargets {
                reps: smaller(a.reps, b.reps),
                time: smaller(a.time, b.time),
            })
            .unwrap_or_default()
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

impl PersonalRecords {
    fn build(
        training_sessions: &[TrainingSession],
        exercises: &[Exercise],
        target_session_id: Option<TrainingSessionID>,
    ) -> Self {
        let mut sessions = training_sessions.iter().collect::<Vec<_>>();
        sessions.sort_by_key(|s| (s.date, s.id));

        let mut result = Self {
            exclusions: exercises.iter().map(|e| (e.id, Exclusion::of(e))).collect(),
            ..Self::default()
        };
        for session in sessions {
            let window_start = session.date - Duration::days(RECENT_RECORD_DAYS);
            if Some(session.id) == target_session_id {
                result.recent_records_before =
                    Some((session.id, result.recent_records(session, window_start)));
            }
            for element in &session.elements {
                if let TrainingSessionElement::Set {
                    exercise_id, side, ..
                } = element
                    && side.is_left_or_right()
                    && !values(element, result.exclusion(*exercise_id)).is_empty()
                {
                    result.sided.insert(*exercise_id);
                }
            }
            let mut record_sets = BTreeMap::new();
            for (key, (value, element_idx)) in session_bests(session, &result.exclusions) {
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

    /// The greatest value per value of the sessions added within the window starting at
    /// `window_start` per exercise of `session` and side.
    fn recent_records(
        &self,
        session: &TrainingSession,
        window_start: NaiveDate,
    ) -> RecentRecordsBefore {
        let mut result = BTreeMap::new();
        for exercise_id in session.exercises() {
            for side in [Side::Left, Side::Right, Side::Alternating] {
                let records = [
                    RecordValue::Reps,
                    RecordValue::Time,
                    RecordValue::Weight,
                    RecordValue::OneRepMax,
                ]
                .into_iter()
                .filter_map(|value| {
                    let history = self.records.get(&(exercise_id, side, value))?;
                    Some((value, history.recent(window_start)?.value))
                })
                .collect::<BTreeMap<_, _>>();
                if !records.is_empty() {
                    result.insert((exercise_id, side), records);
                }
            }
        }
        result
    }

    fn exclusion(&self, exercise_id: ExerciseID) -> Exclusion {
        self.exclusions
            .get(&exercise_id)
            .copied()
            .unwrap_or_default()
    }
}

/// The sides whose records a set on `side` takes part in.
fn record_sides(side: Side) -> &'static [Side] {
    match side {
        Side::Unset => &[Side::Left, Side::Right],
        Side::Left => &[Side::Left],
        Side::Right => &[Side::Right],
        Side::Alternating => &[Side::Alternating],
    }
}

fn smaller<T: PartialOrd>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(if b < a { b } else { a }),
        (a, b) => a.or(b),
    }
}

/// The targets of a set that has to beat `to_beat` and reach `to_reach`, holding only values with
/// a recent record in `to_beat`.
fn record_targets(
    to_beat: &BTreeMap<RecordValue, f32>,
    to_reach: &BTreeMap<RecordValue, f32>,
    weight: Weight,
    rpe: RPE,
    side: Side,
) -> RecordTargets {
    let is_record = |record_value: RecordValue, value: f32| {
        to_beat.get(&record_value).is_some_and(|b| value > *b)
            && to_reach.get(&record_value).is_none_or(|r| value >= *r)
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let next = |record_value: RecordValue| {
        let to_beat = to_beat.get(&record_value)?;
        let to_reach = to_reach.get(&record_value).copied().unwrap_or_default();
        Some((to_beat + 1.).max(to_reach) as u32)
    };
    match weight.non_zero() {
        Some(_) if !to_beat.contains_key(&RecordValue::OneRepMax) => RecordTargets::default(),
        Some(weight) if is_record(RecordValue::Weight, f32::from(weight)) => RecordTargets {
            reps: Reps::new(1).ok(),
            time: None,
        },
        Some(weight) => RecordTargets {
            reps: fewest_reps_for_one_rep_max(weight, rpe, side, |one_rep_max| {
                is_record(RecordValue::OneRepMax, one_rep_max)
            }),
            time: None,
        },
        None => RecordTargets {
            reps: next(RecordValue::Reps).and_then(|reps| Reps::new(reps).ok()),
            time: next(RecordValue::Time).and_then(|time| Time::new(time).ok()),
        },
    }
}

/// The values an exercise has no records of.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
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
        self.expire(window_start);
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

    /// Removes the sessions before `window_start`, which no later window holds.
    fn expire(&mut self, window_start: NaiveDate) {
        while self.window.front().is_some_and(|r| r.date < window_start) {
            self.window.pop_front();
        }
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
    fn test_alternating_sets_are_kept_apart_from_the_sides() {
        let records = personal_records(
            &[
                session(
                    1,
                    0,
                    &[sided_reps(Side::Left, 10), sided_reps(Side::Right, 10)],
                ),
                session(2, 1, &[sided_reps(Side::Alternating, 18)]),
                session(3, 2, &[sided_reps(Side::Alternating, 20)]),
            ],
            &[],
        );

        assert_eq!(records.record_sets(2.into()), BTreeMap::new());
        assert_eq!(
            records.record_sets(3.into()),
            BTreeMap::from([(0, RecordKind::AllTime)])
        );
        assert_eq!(
            values(&records, 2),
            BTreeMap::from([
                ((Side::Left, RecordValue::Reps), (10.0, Some(10.0))),
                ((Side::Right, RecordValue::Reps), (10.0, Some(10.0))),
                ((Side::Alternating, RecordValue::Reps), (20.0, Some(20.0))),
            ])
        );
    }

    #[rstest]
    #[case::without_a_side(&[reps(10)], &[Side::Unset, Side::Alternating])]
    #[case::per_side(
        &[sided_reps(Side::Left, 10), sided_reps(Side::Right, 10)],
        &[Side::Left, Side::Right, Side::Alternating]
    )]
    #[case::alternating_only(&[], &[Side::Alternating])]
    fn test_records_of_the_alternating_side_are_kept_under_their_side(
        #[case] earlier: &[TrainingSessionElement],
        #[case] expected: &[Side],
    ) {
        let records = personal_records(
            &[
                session(1, 0, earlier),
                session(2, 1, &[sided_reps(Side::Alternating, 18)]),
            ],
            &[],
        );

        assert_eq!(
            keys(&records, 1),
            expected
                .iter()
                .map(|side| (*side, RecordValue::Reps))
                .collect::<Vec<_>>()
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

    /// The targets of the set at `element_idx` of a session 10 days after `earlier`, holding
    /// `elements` with the set entered on `side` with `weight` and `rpe`.
    fn targets(
        earlier: &[TrainingSessionElement],
        elements: &[TrainingSessionElement],
        element_idx: usize,
        side: Side,
        weight: f32,
        rpe: RPE,
    ) -> RecordTargets {
        targets_with(earlier, elements, element_idx, side, weight, rpe, &[])
    }

    fn targets_with(
        earlier: &[TrainingSessionElement],
        elements: &[TrainingSessionElement],
        element_idx: usize,
        side: Side,
        weight: f32,
        rpe: RPE,
        exercises: &[Exercise],
    ) -> RecordTargets {
        let mut elements = elements.to_vec();
        if let TrainingSessionElement::Set {
            side: s,
            weight: w,
            rpe: r,
            ..
        } = &mut elements[element_idx]
        {
            *s = side;
            *w = Weight::new(weight).unwrap();
            *r = rpe;
        }
        let current = session(2, 10, &elements);
        PersonalRecords::with_targets(
            &[session(1, 0, earlier), current.clone()],
            exercises,
            current.id,
        )
        .targets(&current, element_idx)
    }

    fn reps_target(reps: u32) -> RecordTargets {
        RecordTargets {
            reps: Some(Reps::new(reps).unwrap()),
            time: None,
        }
    }

    fn empty() -> TrainingSessionElement {
        set(Side::Unset, 0, 0, 0.0, RPE::ZERO)
    }

    #[rstest]
    #[case::without_an_rpe(RPE::ZERO, 6)]
    #[case::with_an_rpe(RPE::EIGHT, 4)]
    fn test_reps_target_of_an_estimated_one_rep_max(#[case] rpe: RPE, #[case] expected: u32) {
        assert_eq!(
            targets(&[lift(5, 100.0)], &[empty()], 0, Side::Unset, 100.0, rpe),
            reps_target(expected)
        );
    }

    #[test]
    fn test_weight_too_light_for_any_number_of_reps_has_no_target() {
        assert_eq!(
            targets(
                &[lift(5, 100.0)],
                &[empty()],
                0,
                Side::Unset,
                10.0,
                RPE::ZERO
            ),
            RecordTargets::default()
        );
    }

    #[test]
    fn test_reps_and_time_targets_are_the_record_plus_one() {
        assert_eq!(
            targets(
                &[reps(10), hold(30)],
                &[empty()],
                0,
                Side::Unset,
                0.0,
                RPE::ZERO
            ),
            RecordTargets {
                reps: Some(Reps::new(11).unwrap()),
                time: Some(Time::new(31).unwrap()),
            }
        );
    }

    #[rstest]
    #[case::beating_a_set_before(&[reps(12), empty()], 1, 13)]
    #[case::reaching_a_set_after(&[empty(), reps(12)], 0, 12)]
    #[case::beating_before_and_reaching_after(&[reps(12), empty(), reps(14)], 1, 14)]
    #[case::below_the_recent_record(&[reps(8), empty()], 1, 11)]
    fn test_saved_sets_of_the_session(
        #[case] elements: &[TrainingSessionElement],
        #[case] element_idx: usize,
        #[case] expected: u32,
    ) {
        assert_eq!(
            targets(
                &[reps(10)],
                elements,
                element_idx,
                Side::Unset,
                0.0,
                RPE::ZERO
            )
            .reps,
            Some(Reps::new(expected).unwrap())
        );
    }

    #[rstest]
    #[case::left(Side::Left, Some(11))]
    #[case::right(Side::Right, Some(9))]
    #[case::without_a_side(Side::Unset, Some(9))]
    fn test_target_follows_the_side_of_the_set(#[case] side: Side, #[case] expected: Option<u32>) {
        assert_eq!(
            targets(
                &[sided_reps(Side::Left, 10), sided_reps(Side::Right, 8)],
                &[empty(), sided_reps(Side::Left, 5)],
                0,
                side,
                0.0,
                RPE::ZERO
            )
            .reps,
            expected.map(|reps| Reps::new(reps).unwrap())
        );
    }

    #[rstest]
    #[case::alternating(&[sided_reps(Side::Alternating, 18), set(Side::Alternating, 0, 30, 0.0, RPE::ZERO)], Some(19), Some(31))]
    #[case::without_a_side(&[reps(10), hold(30)], None, None)]
    #[case::left(&[sided_reps(Side::Left, 10)], None, None)]
    fn test_targets_of_an_alternating_set_come_from_alternating_sets(
        #[case] earlier: &[TrainingSessionElement],
        #[case] reps: Option<u32>,
        #[case] time: Option<u32>,
    ) {
        assert_eq!(
            targets(earlier, &[empty()], 0, Side::Alternating, 0.0, RPE::ZERO),
            RecordTargets {
                reps: reps.map(|reps| Reps::new(reps).unwrap()),
                time: time.map(|time| Time::new(time).unwrap()),
            }
        );
    }

    #[test]
    fn test_reps_target_of_an_estimated_one_rep_max_of_an_alternating_set_counts_both_sides() {
        assert_eq!(
            targets(
                &[set(Side::Alternating, 10, 0, 100.0, RPE::ZERO)],
                &[empty()],
                0,
                Side::Alternating,
                100.0,
                RPE::ZERO
            ),
            reps_target(11)
        );
    }

    #[rstest]
    #[case::earlier_session(&[reps(10)], &[empty()], 0, 11)]
    #[case::set_before(
        &[sided_reps(Side::Left, 10), sided_reps(Side::Right, 10)],
        &[reps(12), empty()],
        1,
        13
    )]
    fn test_target_of_a_side_counts_the_sets_without_a_side(
        #[case] earlier: &[TrainingSessionElement],
        #[case] elements: &[TrainingSessionElement],
        #[case] element_idx: usize,
        #[case] expected: u32,
    ) {
        assert_eq!(
            targets(earlier, elements, element_idx, Side::Left, 0.0, RPE::ZERO),
            RecordTargets {
                reps: Some(Reps::new(expected).unwrap()),
                time: None,
            }
        );
    }

    #[test]
    fn test_no_target_without_a_recent_record() {
        let current = session(2, 200, &[empty()]);
        let records = PersonalRecords::with_targets(
            &[session(1, 0, &[reps(10)]), current.clone()],
            &[],
            current.id,
        );

        assert_eq!(records.targets(&current, 0), RecordTargets::default());
    }

    #[test]
    fn test_no_target_in_another_session() {
        let current = session(2, 10, &[empty()]);
        let other = session(3, 10, &[empty()]);
        let sessions = [session(1, 0, &[reps(10)]), current.clone(), other.clone()];

        assert_eq!(
            PersonalRecords::with_targets(&sessions, &[], current.id).targets(&other, 0),
            RecordTargets::default()
        );
        assert_eq!(
            PersonalRecords::new(&sessions, &[]).targets(&current, 0),
            RecordTargets::default()
        );
    }

    #[test]
    fn test_targets_of_a_later_session_keep_the_recent_records_of_an_earlier_day() {
        let current = session(2, 190, &[empty()]);
        let records = PersonalRecords::with_targets(
            &[session(1, 0, &[reps(10)]), current.clone()],
            &[],
            current.id,
        );

        assert_eq!(
            values(&records, 100),
            BTreeMap::from([((Side::Unset, RecordValue::Reps), (10.0, Some(10.0)))])
        );
    }

    #[test]
    fn test_no_target_for_a_set_whose_saved_values_are_recorded() {
        assert_eq!(
            targets(&[reps(10)], &[hold(30)], 0, Side::Unset, 0.0, RPE::ZERO),
            RecordTargets::default()
        );
    }

    #[test]
    fn test_no_weighted_target_of_an_assisted_exercise() {
        let mut exercise = exercise(vec![]);
        exercise.assistance = Some(Assistance::Assisted);

        assert_eq!(
            targets_with(
                &[lift(5, 20.0)],
                &[empty()],
                0,
                Side::Unset,
                30.0,
                RPE::ZERO,
                &[exercise]
            ),
            RecordTargets::default()
        );
    }

    #[test]
    fn test_no_target_of_a_stretch_only_exercise() {
        let exercise = exercise(vec![ExerciseMuscle {
            muscle_id: MuscleID::Pecs,
            stimulus: Stimulus::NONE,
            stretch: Stimulus::PRIMARY,
        }]);

        assert_eq!(
            targets_with(
                &[hold(30)],
                &[empty()],
                0,
                Side::Unset,
                0.0,
                RPE::ZERO,
                &[exercise]
            ),
            RecordTargets::default()
        );
    }

    #[test]
    fn test_no_target_above_the_largest_reps_or_time() {
        assert_eq!(
            targets(
                &[reps(999), hold(999)],
                &[empty()],
                0,
                Side::Unset,
                0.0,
                RPE::ZERO
            ),
            RecordTargets::default()
        );
    }

    #[test]
    fn test_weight_above_the_recent_weight_record_gives_a_reps_target_of_one() {
        assert_eq!(
            targets(
                &[lift(5, 100.0)],
                &[empty()],
                0,
                Side::Unset,
                105.0,
                RPE::ZERO
            ),
            reps_target(1)
        );
    }

    #[rstest]
    #[case::saved_set_before(&[lift(3, 105.0), empty()], 1, 4)]
    #[case::saved_set_after(&[empty(), lift(3, 105.0)], 0, 1)]
    fn test_weight_equal_to_that_of_a_saved_set(
        #[case] elements: &[TrainingSessionElement],
        #[case] element_idx: usize,
        #[case] expected: u32,
    ) {
        assert_eq!(
            targets(
                &[lift(5, 100.0)],
                elements,
                element_idx,
                Side::Unset,
                105.0,
                RPE::ZERO
            ),
            reps_target(expected)
        );
    }

    #[test]
    fn test_weight_above_the_weight_record_of_a_weighted_hold_has_no_target() {
        assert_eq!(
            targets(
                &[set(Side::Unset, 0, 30, 20.0, RPE::ZERO)],
                &[empty()],
                0,
                Side::Unset,
                25.0,
                RPE::ZERO
            ),
            RecordTargets::default()
        );
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
            countdown: false,
        }
    }
}
