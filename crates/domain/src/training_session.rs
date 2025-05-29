use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

use chrono::{Local, NaiveDate};
use derive_more::Deref;
use uuid::Uuid;

use crate::{
    CreateError, DeleteError, Exercise, ExerciseID, MuscleID, RPE, ReadError, Reps, RoutineID,
    Stimulus, SyncError, Time, TrainingStats, UpdateError, ValidationError, Weight, training_stats,
};

#[allow(async_fn_in_trait)]
pub trait TrainingSessionService {
    async fn get_training_sessions(&self) -> Result<Vec<TrainingSession>, ReadError>;
    async fn create_training_session(
        &self,
        routine_id: RoutineID,
        date: NaiveDate,
        notes: String,
        elements: Vec<TrainingSessionElement>,
    ) -> Result<TrainingSession, CreateError>;
    async fn modify_training_session(
        &self,
        id: TrainingSessionID,
        notes: Option<String>,
        elements: Option<Vec<TrainingSessionElement>>,
    ) -> Result<TrainingSession, UpdateError>;
    async fn delete_training_session(
        &self,
        id: TrainingSessionID,
    ) -> Result<TrainingSessionID, DeleteError>;

    async fn validate_training_session_date(
        &self,
        date: &str,
    ) -> Result<NaiveDate, ValidationError> {
        match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            Ok(parsed_date) => {
                if parsed_date <= Local::now().date_naive() {
                    match self.get_training_sessions().await {
                        Ok(training_sessions) => {
                            if training_sessions.iter().all(|u| u.date != parsed_date) {
                                Ok(parsed_date)
                            } else {
                                Err(ValidationError::Conflict("date".to_string()))
                            }
                        }
                        Err(err) => Err(ValidationError::Other(err.into())),
                    }
                } else {
                    Err(ValidationError::Other(
                        "Date must not be in the future".into(),
                    ))
                }
            }
            Err(_) => Err(ValidationError::Other("Invalid date".into())),
        }
    }

    async fn get_training_session(
        &self,
        id: TrainingSessionID,
    ) -> Result<Option<TrainingSession>, ReadError> {
        Ok(self
            .get_training_sessions()
            .await?
            .into_iter()
            .find(|e| e.id == id))
    }

    async fn get_training_sessions_by_exercise_id(
        &self,
        id: ExerciseID,
    ) -> Result<Vec<TrainingSession>, ReadError> {
        Ok(self
            .get_training_sessions()
            .await?
            .into_iter()
            .filter(|t| t.exercises().contains(&id))
            .map(|t| TrainingSession {
                id: t.id,
                routine_id: t.routine_id,
                date: t.date,
                notes: t.notes.clone(),
                elements: t
                    .elements
                    .iter()
                    .filter(|e| match e {
                        TrainingSessionElement::Set { exercise_id, .. } => *exercise_id == id,
                        TrainingSessionElement::Rest { .. } => false,
                    })
                    .cloned()
                    .collect::<Vec<_>>(),
            })
            .collect::<Vec<_>>())
    }

    async fn get_training_sessions_by_routine_id(
        &self,
        id: RoutineID,
    ) -> Result<Vec<TrainingSession>, ReadError> {
        Ok(self
            .get_training_sessions()
            .await?
            .into_iter()
            .filter(|t| t.routine_id == id)
            .collect::<Vec<_>>())
    }

    fn get_training_stats(&self, training_sessions: &[TrainingSession]) -> TrainingStats {
        training_stats(&training_sessions.iter().collect::<Vec<_>>())
    }
}

#[allow(async_fn_in_trait)]
pub trait TrainingSessionRepository {
    async fn sync_training_sessions(&self) -> Result<Vec<TrainingSession>, SyncError>;
    async fn read_training_sessions(&self) -> Result<Vec<TrainingSession>, ReadError>;
    async fn create_training_session(
        &self,
        routine_id: RoutineID,
        date: NaiveDate,
        notes: String,
        elements: Vec<TrainingSessionElement>,
    ) -> Result<TrainingSession, CreateError>;
    async fn modify_training_session(
        &self,
        id: TrainingSessionID,
        notes: Option<String>,
        elements: Option<Vec<TrainingSessionElement>>,
    ) -> Result<TrainingSession, UpdateError>;
    async fn delete_training_session(
        &self,
        id: TrainingSessionID,
    ) -> Result<TrainingSessionID, DeleteError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrainingSession {
    pub id: TrainingSessionID,
    pub routine_id: RoutineID,
    pub date: NaiveDate,
    pub notes: String,
    pub elements: Vec<TrainingSessionElement>,
}

impl TrainingSession {
    #[must_use]
    pub fn exercises(&self) -> BTreeSet<ExerciseID> {
        self.elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { exercise_id, .. } => Some(*exercise_id),
                TrainingSessionElement::Rest { .. } => None,
            })
            .collect::<BTreeSet<_>>()
    }

    #[must_use]
    pub fn avg_reps(&self) -> Option<f32> {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { reps, .. } => *reps,
                TrainingSessionElement::Rest { .. } => None,
            })
            .collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            Some(sets.iter().map(|r| u32::from(*r)).sum::<u32>() as f32 / sets.len() as f32)
        }
    }

    #[must_use]
    pub fn avg_time(&self) -> Option<f32> {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { time, .. } => *time,
                TrainingSessionElement::Rest { .. } => None,
            })
            .collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            Some(sets.iter().map(|t| u32::from(*t)).sum::<u32>() as f32 / sets.len() as f32)
        }
    }

    #[must_use]
    pub fn avg_weight(&self) -> Option<f32> {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { weight, .. } => *weight,
                TrainingSessionElement::Rest { .. } => None,
            })
            .collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            Some(sets.iter().map(|w| f32::from(*w)).sum::<f32>() / sets.len() as f32)
        }
    }

    #[must_use]
    pub fn avg_rpe(&self) -> Option<RPE> {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { rpe, .. } => *rpe,
                TrainingSessionElement::Rest { .. } => None,
            })
            .collect::<Vec<_>>();
        RPE::avg(sets)
    }

    #[must_use]
    pub fn load(&self) -> u32 {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set {
                    reps, time, rpe, ..
                } => Some(if let Some(rpe) = *rpe {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    if rpe > RPE::FIVE {
                        (2.0_f32).powf(f32::from(rpe) - 5.0).round() as u32
                    } else {
                        1
                    }
                } else {
                    u32::from(reps.is_some() || time.is_some())
                }),
                TrainingSessionElement::Rest { .. } => None,
            })
            .collect::<Vec<_>>();
        sets.iter().sum::<u32>()
    }

    #[must_use]
    pub fn set_volume(&self) -> u32 {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set {
                    reps, time, rpe, ..
                } => {
                    if rpe.unwrap_or(RPE::TEN) >= RPE::SEVEN {
                        Some(u32::from(reps.is_some() || time.is_some()))
                    } else {
                        None
                    }
                }
                TrainingSessionElement::Rest { .. } => None,
            })
            .collect::<Vec<_>>();
        sets.iter().sum::<u32>()
    }

    #[must_use]
    pub fn volume_load(&self) -> u32 {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { reps, weight, .. } => {
                    if let Some(reps) = reps {
                        #[allow(
                            clippy::cast_possible_truncation,
                            clippy::cast_precision_loss,
                            clippy::cast_sign_loss
                        )]
                        if let Some(weight) = weight {
                            Some((u32::from(*reps) as f32 * f32::from(*weight)).round() as u32)
                        } else {
                            Some(u32::from(*reps))
                        }
                    } else {
                        None
                    }
                }
                TrainingSessionElement::Rest { .. } => None,
            })
            .collect::<Vec<_>>();
        sets.iter().sum::<u32>()
    }

    pub fn tut(&self) -> Option<u32> {
        let sets = &self
            .elements
            .iter()
            .map(|e| match e {
                TrainingSessionElement::Set { reps, time, .. } => time
                    .as_ref()
                    .map(|v| reps.unwrap_or(Reps::new(1).unwrap()) * *v),
                TrainingSessionElement::Rest { .. } => None,
            })
            .collect::<Vec<_>>();
        if sets.iter().all(Option::is_none) {
            return None;
        }
        Some(
            sets.iter()
                .map(|t| u32::from(t.unwrap_or_default()))
                .sum::<u32>(),
        )
    }

    #[must_use]
    pub fn stimulus_per_muscle(
        &self,
        exercises: &BTreeMap<ExerciseID, Exercise>,
    ) -> BTreeMap<MuscleID, Stimulus> {
        let mut result: BTreeMap<MuscleID, Stimulus> = BTreeMap::new();
        for element in &self.elements {
            if let TrainingSessionElement::Set {
                exercise_id,
                reps,
                time,
                rpe,
                ..
            } = element
            {
                if reps.is_none() && time.is_none() {
                    continue;
                }
                if let Some(rpe) = rpe {
                    if *rpe < RPE::SEVEN {
                        continue;
                    }
                }
                if let Some(exercise) = exercises.get(exercise_id) {
                    for (muscle_id, stimulus) in &exercise.muscle_stimulus() {
                        *result.entry(*muscle_id).or_insert(Stimulus::NONE) += *stimulus;
                    }
                }
            }
        }
        result
    }

    #[must_use]
    pub fn compute_sections(&self) -> Vec<TrainingSessionSection> {
        let mut sections = vec![];
        let mut idx = 0;

        while idx < self.elements.len() {
            let last = Self::find_last_of_section(self, idx);
            sections.push(TrainingSessionSection(self.elements[idx..=last].to_vec()));
            idx = last + 1;
        }

        sections
    }

    fn find_last_of_section(&self, element_idx: usize) -> usize {
        let mut last = Self::find_last_set_with_same_exercises(self, element_idx);

        assert!(element_idx <= last);

        if last + 1 < self.elements.len() {
            if let TrainingSessionElement::Rest { .. } = &self.elements[last + 1] {
                last += 1;
            }
        }

        last
    }

    fn find_last_set_with_same_exercises(&self, element_idx: usize) -> usize {
        let ids = next_consecutive_exercise_ids(&self.elements[element_idx..]);
        let mut last_idx = element_idx;

        if ids.is_empty() {
            return last_idx;
        }

        let mut ids_idx = 0;

        for (i, element) in self.elements.iter().enumerate().skip(last_idx) {
            if let TrainingSessionElement::Set { exercise_id, .. } = &element {
                if *exercise_id == ids[ids_idx] {
                    // Only consider sets that contain all exercises
                    if ids_idx == ids.len() - 1 {
                        last_idx = i;
                    }
                } else {
                    break;
                }
                ids_idx = (ids_idx + 1) % ids.len();
            }
        }

        last_idx
    }
}

#[derive(Deref, Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TrainingSessionID(Uuid);

impl TrainingSessionID {
    #[must_use]
    pub fn nil() -> Self {
        Self(Uuid::nil())
    }

    #[must_use]
    pub fn is_nil(&self) -> bool {
        self.0.is_nil()
    }
}

impl From<Uuid> for TrainingSessionID {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<u128> for TrainingSessionID {
    fn from(value: u128) -> Self {
        Self(Uuid::from_bytes(value.to_be_bytes()))
    }
}

impl FromStr for TrainingSessionID {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(Self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrainingSessionElement {
    Set {
        exercise_id: ExerciseID,
        reps: Option<Reps>,
        time: Option<Time>,
        weight: Option<Weight>,
        rpe: Option<RPE>,
        target_reps: Option<Reps>,
        target_time: Option<Time>,
        target_weight: Option<Weight>,
        target_rpe: Option<RPE>,
        automatic: bool,
    },
    Rest {
        target_time: Option<Time>,
        automatic: bool,
    },
}

impl TrainingSessionElement {
    #[must_use]
    pub fn to_string(&self, show_tut: bool, show_rpe: bool) -> String {
        match self {
            TrainingSessionElement::Set {
                reps,
                time,
                weight,
                rpe,
                ..
            } => {
                let mut parts = vec![];

                if let Some(reps) = reps {
                    if *reps > Reps::default() {
                        parts.push(reps.to_string());
                    }
                }

                if let Some(time) = time {
                    if show_tut && *time > Time::default() {
                        parts.push(format!("{time} s"));
                    }
                }

                if let Some(weight) = weight {
                    if *weight > Weight::default() {
                        parts.push(format!("{weight} kg"));
                    }
                }

                let mut result = parts.join(" × ");

                if let Some(rpe) = rpe {
                    if show_rpe && *rpe > RPE::ZERO {
                        result.push_str(&format!(" @ {rpe}"));
                    }
                }

                result
            }
            TrainingSessionElement::Rest { .. } => String::new(),
        }
    }
}

// TODO: replace Vec by mitsein::Vec1? https://github.com/olson-sean-k/mitsein
#[derive(Clone, Debug, PartialEq)]
pub struct TrainingSessionSection(Vec<TrainingSessionElement>);

impl TrainingSessionSection {
    pub fn elements(&self) -> &[TrainingSessionElement] {
        &self.0
    }

    pub fn exercise_ids(&self) -> Vec<ExerciseID> {
        next_consecutive_exercise_ids(&self.0)
    }
}

fn next_consecutive_exercise_ids(elements: &[TrainingSessionElement]) -> Vec<ExerciseID> {
    let mut exercise_ids = vec![];

    for element in elements {
        match element {
            TrainingSessionElement::Set { exercise_id, .. } => exercise_ids.push(*exercise_id),
            TrainingSessionElement::Rest { .. } => return exercise_ids,
        }
    }

    exercise_ids
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Local};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use crate::{ExerciseMuscle, Name};

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
                    reps: Some(Reps::new(10).unwrap()),
                    time: Some(Time::new(3).unwrap()),
                    weight: Some(Weight::new(30.0).unwrap()),
                    rpe: Some(RPE::EIGHT),
                    target_reps: Some(Reps::new(8).unwrap()),
                    target_time: Some(Time::new(4).unwrap()),
                    target_weight: Some(Weight::new(40.0).unwrap()),
                    target_rpe: Some(RPE::NINE),
                    automatic: false,
                },
                TrainingSessionElement::Rest {
                    target_time: Some(Time::new(60).unwrap()),
                    automatic: true,
                },
                TrainingSessionElement::Set {
                    exercise_id: 2.into(),
                    reps: Some(Reps::new(5).unwrap()),
                    time: Some(Time::new(4).unwrap()),
                    weight: None,
                    rpe: Some(RPE::FOUR),
                    target_reps: None,
                    target_time: None,
                    target_weight: None,
                    target_rpe: None,
                    automatic: false,
                },
                TrainingSessionElement::Rest {
                    target_time: Some(Time::new(60).unwrap()),
                    automatic: true,
                },
                TrainingSessionElement::Set {
                    exercise_id: 2.into(),
                    reps: None,
                    time: Some(Time::new(60).unwrap()),
                    weight: None,
                    rpe: None,
                    target_reps: None,
                    target_time: None,
                    target_weight: None,
                    target_rpe: None,
                    automatic: false,
                },
                TrainingSessionElement::Rest {
                    target_time: Some(Time::new(60).unwrap()),
                    automatic: true,
                },
            ],
        });

    static EMPTY_TRAINING_SESSION: std::sync::LazyLock<TrainingSession> =
        std::sync::LazyLock::new(|| {
            let mut training_session = TRAINING_SESSION.clone();
            training_session.elements = TRAINING_SESSION
                .elements
                .iter()
                .map(|e| match e {
                    TrainingSessionElement::Set {
                        exercise_id,
                        target_reps,
                        target_time,
                        target_weight,
                        target_rpe,
                        automatic,
                        ..
                    } => TrainingSessionElement::Set {
                        exercise_id: *exercise_id,
                        reps: None,
                        time: None,
                        weight: None,
                        rpe: None,
                        target_reps: *target_reps,
                        target_time: *target_time,
                        target_weight: *target_weight,
                        target_rpe: *target_rpe,
                        automatic: *automatic,
                    },
                    TrainingSessionElement::Rest { .. } => e.clone(),
                })
                .collect::<Vec<_>>();
            training_session
        });

    #[test]
    fn test_training_session_exercises() {
        assert_eq!(
            TRAINING_SESSION.exercises(),
            BTreeSet::from([1.into(), 2.into()])
        );
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, Some(7.5))]
    #[case(&*EMPTY_TRAINING_SESSION, None)]
    fn test_training_session_avg_reps(
        #[case] training_session: &TrainingSession,
        #[case] expected: Option<f32>,
    ) {
        assert_eq!(training_session.avg_reps(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, Some(22.333_334))]
    #[case(&*EMPTY_TRAINING_SESSION, None)]
    fn test_training_session_avg_time(
        #[case] training_session: &TrainingSession,
        #[case] expected: Option<f32>,
    ) {
        assert_eq!(training_session.avg_time(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, Some(30.0))]
    #[case(&*EMPTY_TRAINING_SESSION, None)]
    fn test_training_session_avg_weight(
        #[case] training_session: &TrainingSession,
        #[case] expected: Option<f32>,
    ) {
        assert_eq!(training_session.avg_weight(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, Some(RPE::SIX))]
    #[case(&*EMPTY_TRAINING_SESSION, None)]
    fn test_training_session_avg_rpe(
        #[case] training_session: &TrainingSession,
        #[case] expected: Option<RPE>,
    ) {
        assert_eq!(training_session.avg_rpe(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, 10)]
    #[case(&*EMPTY_TRAINING_SESSION, 0)]
    fn test_training_session_load(
        #[case] training_session: &TrainingSession,
        #[case] expected: u32,
    ) {
        assert_eq!(training_session.load(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, 2)]
    #[case(&*EMPTY_TRAINING_SESSION, 0)]
    fn test_training_session_set_volume(
        #[case] training_session: &TrainingSession,
        #[case] expected: u32,
    ) {
        assert_eq!(training_session.set_volume(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, 305)]
    #[case(&*EMPTY_TRAINING_SESSION, 0)]
    fn test_training_session_volume_load(
        #[case] training_session: &TrainingSession,
        #[case] expected: u32,
    ) {
        assert_eq!(training_session.volume_load(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, Some(110))]
    #[case(&*EMPTY_TRAINING_SESSION, None)]
    fn test_training_session_tut(
        #[case] training_session: &TrainingSession,
        #[case] expected: Option<u32>,
    ) {
        assert_eq!(training_session.tut(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, BTreeMap::from([(MuscleID::Pecs, Stimulus::PRIMARY), (MuscleID::FrontDelts, Stimulus::SECONDARY)]))]
    #[case(&*EMPTY_TRAINING_SESSION, BTreeMap::new())]
    fn test_training_session_stimulus_per_muscle(
        #[case] training_session: &TrainingSession,
        #[case] expected: BTreeMap<MuscleID, Stimulus>,
    ) {
        let exercises = BTreeMap::from([(
            1.into(),
            Exercise {
                id: 1.into(),
                name: Name::new("A").unwrap(),
                muscles: vec![
                    ExerciseMuscle {
                        muscle_id: MuscleID::Pecs,
                        stimulus: Stimulus::PRIMARY,
                    },
                    ExerciseMuscle {
                        muscle_id: MuscleID::FrontDelts,
                        stimulus: Stimulus::SECONDARY,
                    },
                ],
            },
        )]);
        assert_eq!(training_session.stimulus_per_muscle(&exercises), expected);
    }

    #[test]
    fn test_training_session_id_nil() {
        assert!(TrainingSessionID::nil().is_nil());
        assert_eq!(TrainingSessionID::nil(), TrainingSessionID::default());
    }

    #[test]
    fn test_training_session_compute_sections_empty() {
        assert_eq!(training_session(&[]).compute_sections(), vec![]);
    }

    #[test]
    fn test_training_session_compute_sections_simple() {
        assert_eq!(
            training_session(&[exercise(0, 0)]).compute_sections(),
            vec![section(&[exercise(0, 0)])]
        );
        assert_eq!(
            training_session(&[exercise(0, 0), exercise(1, 1)]).compute_sections(),
            vec![section(&[exercise(0, 0), exercise(1, 1)])]
        );
        assert_eq!(
            training_session(&[rest(0)]).compute_sections(),
            vec![section(&[rest(0)])]
        );
        assert_eq!(
            training_session(&[rest(0), rest(1)]).compute_sections(),
            vec![section(&[rest(0), rest(1)])]
        );
        assert_eq!(
            training_session(&[exercise(0, 0), rest(1)]).compute_sections(),
            vec![section(&[exercise(0, 0), rest(1)])]
        );
        assert_eq!(
            training_session(&[rest(0), exercise(1, 1)]).compute_sections(),
            vec![section(&[rest(0)]), section(&[exercise(1, 1)])]
        );
    }

    #[test]
    fn test_training_session_compute_sections_complex() {
        assert_eq!(
            training_session(&[
                exercise(0, 0),
                rest(0),
                exercise(1, 0),
                rest(1),
                exercise(2, 1),
                rest(2),
                exercise(4, 0),
                exercise(5, 2),
                rest(4),
                exercise(6, 0),
                exercise(7, 2),
                rest(5),
                exercise(8, 0),
                exercise(9, 2),
                rest(6),
                exercise(10, 0),
                exercise(11, 0),
                rest(7),
                exercise(12, 0),
                exercise(13, 0),
            ])
            .compute_sections(),
            vec![
                section(&[exercise(0, 0), rest(0), exercise(1, 0), rest(1)]),
                section(&[exercise(2, 1), rest(2)]),
                section(&[
                    exercise(4, 0),
                    exercise(5, 2),
                    rest(4),
                    exercise(6, 0),
                    exercise(7, 2),
                    rest(5),
                    exercise(8, 0),
                    exercise(9, 2),
                    rest(6),
                ]),
                section(&[
                    exercise(10, 0),
                    exercise(11, 0),
                    rest(7),
                    exercise(12, 0),
                    exercise(13, 0),
                ]),
            ]
        );
    }

    fn training_session(elements: &[TrainingSessionElement]) -> TrainingSession {
        TrainingSession {
            id: 1.into(),
            routine_id: 0.into(),
            date: *TODAY - Duration::days(10),
            notes: String::new(),
            elements: elements.to_vec(),
        }
    }

    fn exercise(entry_id: u32, exercise_id: u128) -> TrainingSessionElement {
        TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            reps: None,
            time: None,
            weight: None,
            rpe: None,
            target_reps: if entry_id > 0 {
                Some(Reps::new(entry_id).unwrap())
            } else {
                None
            },
            target_time: None,
            target_weight: None,
            target_rpe: None,
            automatic: false,
        }
    }

    fn rest(entry_id: u32) -> TrainingSessionElement {
        TrainingSessionElement::Rest {
            target_time: Some(Time::new(entry_id).unwrap()),
            automatic: true,
        }
    }

    fn section(elements: &[TrainingSessionElement]) -> TrainingSessionSection {
        TrainingSessionSection(elements.to_vec())
    }
}
