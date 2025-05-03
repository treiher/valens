use std::collections::{BTreeMap, BTreeSet};

use chrono::{Duration, NaiveDate};
use derive_more::Deref;
use uuid::Uuid;

use crate::{
    CreateError, DeleteError, Exercise, ExerciseID, MuscleID, Name, Property, RPE, ReadError, Reps,
    Stimulus, SyncError, Time, TrainingSession, TrainingSessionElement, UpdateError,
    ValidationError, Weight,
};

#[allow(async_fn_in_trait)]
pub trait RoutineService {
    async fn get_routines(&self) -> Result<Vec<Routine>, ReadError>;
    async fn create_routine(
        &self,
        name: Name,
        sections: Vec<RoutinePart>,
    ) -> Result<Routine, CreateError>;
    async fn modify_routine(
        &self,
        id: RoutineID,
        name: Option<Name>,
        archived: Option<bool>,
        sections: Option<Vec<RoutinePart>>,
    ) -> Result<Routine, UpdateError>;
    async fn delete_routine(&self, id: RoutineID) -> Result<RoutineID, DeleteError>;

    async fn validate_routine_name(
        &self,
        name: &str,
        id: RoutineID,
    ) -> Result<Name, ValidationError> {
        match Name::new(name) {
            Ok(name) => match self.get_routines().await {
                Ok(routines) => {
                    if routines.iter().all(|r| r.id == id || r.name != name) {
                        Ok(name)
                    } else {
                        Err(ValidationError::Conflict("name".to_string()))
                    }
                }
                Err(err) => Err(ValidationError::Other(err.into())),
            },
            Err(err) => Err(ValidationError::Other(err.into())),
        }
    }

    async fn get_routine(&self, id: RoutineID) -> Result<Option<Routine>, ReadError> {
        Ok(self.get_routines().await?.into_iter().find(|e| e.id == id))
    }
}

#[allow(async_fn_in_trait)]
pub trait RoutineRepository {
    async fn sync_routines(&self) -> Result<Vec<Routine>, SyncError>;
    async fn read_routines(&self) -> Result<Vec<Routine>, ReadError>;
    async fn create_routine(
        &self,
        name: Name,
        sections: Vec<RoutinePart>,
    ) -> Result<Routine, CreateError>;
    async fn modify_routine(
        &self,
        id: RoutineID,
        name: Option<Name>,
        archived: Option<bool>,
        sections: Option<Vec<RoutinePart>>,
    ) -> Result<Routine, UpdateError>;
    async fn delete_routine(&self, id: RoutineID) -> Result<RoutineID, DeleteError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Routine {
    pub id: RoutineID,
    pub name: Name,
    pub notes: String,
    pub archived: bool,
    pub sections: Vec<RoutinePart>,
}

impl Routine {
    pub fn duration(&self) -> Duration {
        self.sections.iter().map(RoutinePart::duration).sum()
    }

    pub fn num_sets(&self) -> u32 {
        self.sections.iter().map(RoutinePart::num_sets).sum()
    }

    #[must_use]
    pub fn stimulus_per_muscle(
        &self,
        exercises: &BTreeMap<ExerciseID, Exercise>,
    ) -> BTreeMap<MuscleID, Stimulus> {
        let mut result: BTreeMap<MuscleID, Stimulus> =
            MuscleID::iter().map(|m| (*m, Stimulus::NONE)).collect();
        for section in &self.sections {
            for (muscle_id, stimulus) in section.stimulus_per_muscle(exercises) {
                if result.contains_key(&muscle_id) {
                    *result.entry(muscle_id).or_insert(Stimulus::NONE) += stimulus;
                }
            }
        }
        result
    }

    pub fn exercises(&self) -> BTreeSet<ExerciseID> {
        self.sections
            .iter()
            .flat_map(RoutinePart::exercises)
            .collect::<BTreeSet<_>>()
    }
}

#[derive(Deref, Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoutineID(Uuid);

impl RoutineID {
    #[must_use]
    pub fn nil() -> Self {
        Self(Uuid::nil())
    }

    #[must_use]
    pub fn is_nil(&self) -> bool {
        self.0.is_nil()
    }
}

impl From<Uuid> for RoutineID {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<u128> for RoutineID {
    fn from(value: u128) -> Self {
        Self(Uuid::from_bytes(value.to_be_bytes()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoutinePart {
    RoutineSection {
        rounds: u32,
        parts: Vec<RoutinePart>,
    },
    RoutineActivity {
        exercise_id: ExerciseID,
        reps: Reps,
        time: Time,
        weight: Weight,
        rpe: RPE,
        automatic: bool,
    },
}

impl RoutinePart {
    pub fn duration(&self) -> Duration {
        match self {
            RoutinePart::RoutineSection { rounds, parts } => {
                parts.iter().map(RoutinePart::duration).sum::<Duration>()
                    * (*rounds).try_into().unwrap_or(1)
            }
            RoutinePart::RoutineActivity { reps, time, .. } => {
                let r = if *reps > Reps::default() {
                    *reps
                } else {
                    Reps::new(1).unwrap()
                };
                let t = if *time > Time::default() {
                    *time
                } else {
                    Time::new(4).unwrap()
                };
                Duration::seconds(i64::from(r * t))
            }
        }
    }

    pub fn num_sets(&self) -> u32 {
        match self {
            RoutinePart::RoutineSection { rounds, parts } => {
                parts.iter().map(RoutinePart::num_sets).sum::<u32>() * *rounds
            }
            RoutinePart::RoutineActivity { exercise_id, .. } => (!exercise_id.is_nil()).into(),
        }
    }

    #[must_use]
    pub fn stimulus_per_muscle(
        &self,
        exercises: &BTreeMap<ExerciseID, Exercise>,
    ) -> BTreeMap<MuscleID, Stimulus> {
        match self {
            RoutinePart::RoutineSection { rounds, parts } => {
                let mut result: BTreeMap<MuscleID, Stimulus> = BTreeMap::new();
                for part in parts {
                    for (muscle_id, stimulus) in part.stimulus_per_muscle(exercises) {
                        *result.entry(muscle_id).or_insert(Stimulus::NONE) += stimulus * *rounds;
                    }
                }
                result
            }
            RoutinePart::RoutineActivity { exercise_id, .. } => exercises
                .get(exercise_id)
                .map(|e| {
                    e.muscle_stimulus()
                        .iter()
                        .map(|(muscle_id, stimulus)| (*muscle_id, *stimulus))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    fn exercises(&self) -> BTreeSet<ExerciseID> {
        let mut result: BTreeSet<ExerciseID> = BTreeSet::new();
        match self {
            RoutinePart::RoutineSection { parts, .. } => {
                for p in parts {
                    result.extend(Self::exercises(p));
                }
            }
            RoutinePart::RoutineActivity { exercise_id, .. } => {
                if !exercise_id.is_nil() {
                    result.insert(*exercise_id);
                }
            }
        }
        result
    }

    #[must_use]
    pub fn to_training_session_elements(&self) -> Vec<TrainingSessionElement> {
        let mut result = vec![];
        match self {
            RoutinePart::RoutineSection { rounds, parts, .. } => {
                for _ in 0..*rounds {
                    for p in parts {
                        for s in p.to_training_session_elements() {
                            result.push(s);
                        }
                    }
                }
            }
            RoutinePart::RoutineActivity {
                exercise_id,
                reps,
                time,
                weight,
                rpe,
                automatic,
            } => {
                result.push(if exercise_id.is_nil() {
                    TrainingSessionElement::Rest {
                        target_time: if *time > Time::default() {
                            Some(*time)
                        } else {
                            None
                        },
                        automatic: *automatic,
                    }
                } else {
                    TrainingSessionElement::Set {
                        exercise_id: *exercise_id,
                        reps: None,
                        time: None,
                        weight: None,
                        rpe: None,
                        target_reps: if *reps > Reps::default() {
                            Some(*reps)
                        } else {
                            None
                        },
                        target_time: if *time > Time::default() {
                            Some(*time)
                        } else {
                            None
                        },
                        target_weight: if *weight > Weight::default() {
                            Some(*weight)
                        } else {
                            None
                        },
                        target_rpe: if *rpe > RPE::ZERO { Some(*rpe) } else { None },
                        automatic: *automatic,
                    }
                });
            }
        }
        result
    }
}

pub fn routines_sorted_by_last_use(
    routines: &[Routine],
    training_sessions: &[TrainingSession],
    filter: impl Fn(&Routine) -> bool,
) -> Vec<Routine> {
    let mut map: BTreeMap<RoutineID, NaiveDate> = BTreeMap::new();
    for routine_id in routines.iter().filter(|r| filter(r)).map(|r| r.id) {
        #[allow(clippy::cast_possible_truncation)]
        map.insert(
            routine_id,
            NaiveDate::MIN + Duration::days(routine_id.as_u128() as i64),
        );
    }
    for training_session in training_sessions {
        let routine_id = training_session.routine_id;
        if routines.iter().any(|r| r.id == routine_id)
            && map.contains_key(&routine_id)
            && training_session.date > map[&routine_id]
        {
            map.insert(routine_id, training_session.date);
        }
    }
    let mut list: Vec<_> = map.iter().collect();
    list.sort_by(|a, b| a.1.cmp(b.1).reverse());
    list.iter()
        .filter_map(|(routine_id, _)| routines.iter().find(|r| r.id == **routine_id))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::ExerciseMuscle;

    use super::*;

    static ROUTINE: std::sync::LazyLock<Routine> = std::sync::LazyLock::new(|| Routine {
        id: 1.into(),
        name: Name::new("A").unwrap(),
        notes: String::from("B"),
        archived: false,
        sections: vec![
            RoutinePart::RoutineSection {
                rounds: 2,
                parts: vec![
                    RoutinePart::RoutineActivity {
                        exercise_id: 1.into(),
                        reps: Reps::new(10).unwrap(),
                        time: Time::new(2).unwrap(),
                        weight: Weight::new(30.0).unwrap(),
                        rpe: RPE::TEN,
                        automatic: false,
                    },
                    RoutinePart::RoutineActivity {
                        exercise_id: ExerciseID::nil(),
                        reps: Reps::default(),
                        time: Time::new(60).unwrap(),
                        weight: Weight::default(),
                        rpe: RPE::ZERO,
                        automatic: true,
                    },
                ],
            },
            RoutinePart::RoutineSection {
                rounds: 2,
                parts: vec![
                    RoutinePart::RoutineActivity {
                        exercise_id: 2.into(),
                        reps: Reps::new(10).unwrap(),
                        time: Time::default(),
                        weight: Weight::default(),
                        rpe: RPE::ZERO,
                        automatic: false,
                    },
                    RoutinePart::RoutineActivity {
                        exercise_id: ExerciseID::nil(),
                        reps: Reps::default(),
                        time: Time::new(30).unwrap(),
                        weight: Weight::default(),
                        rpe: RPE::ZERO,
                        automatic: true,
                    },
                ],
            },
        ],
    });

    static EXERCISES: std::sync::LazyLock<BTreeMap<ExerciseID, Exercise>> =
        std::sync::LazyLock::new(|| {
            BTreeMap::from([(
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
            )])
        });

    #[test]
    fn test_routine_duration() {
        assert_eq!(ROUTINE.duration(), Duration::seconds(300));
    }

    #[test]
    fn test_routine_num_sets() {
        assert_eq!(ROUTINE.num_sets(), 4);
    }

    #[test]
    fn test_routine_stimulus_per_muscle() {
        assert_eq!(
            ROUTINE.stimulus_per_muscle(&EXERCISES),
            BTreeMap::from([
                (MuscleID::Neck, Stimulus::NONE),
                (MuscleID::Pecs, Stimulus::PRIMARY * 2),
                (MuscleID::Traps, Stimulus::NONE),
                (MuscleID::Lats, Stimulus::NONE),
                (MuscleID::FrontDelts, Stimulus::PRIMARY),
                (MuscleID::SideDelts, Stimulus::NONE),
                (MuscleID::RearDelts, Stimulus::NONE),
                (MuscleID::Biceps, Stimulus::NONE),
                (MuscleID::Triceps, Stimulus::NONE),
                (MuscleID::Forearms, Stimulus::NONE),
                (MuscleID::Abs, Stimulus::NONE),
                (MuscleID::ErectorSpinae, Stimulus::NONE),
                (MuscleID::Glutes, Stimulus::NONE),
                (MuscleID::Abductors, Stimulus::NONE),
                (MuscleID::Quads, Stimulus::NONE),
                (MuscleID::Hamstrings, Stimulus::NONE),
                (MuscleID::Adductors, Stimulus::NONE),
                (MuscleID::Calves, Stimulus::NONE),
            ])
        );
    }

    #[test]
    fn test_routine_exercises() {
        assert_eq!(ROUTINE.exercises(), BTreeSet::from([1.into(), 2.into()]));
    }

    #[test]
    fn test_routine_id_nil() {
        assert!(RoutineID::nil().is_nil());
        assert_eq!(RoutineID::nil(), RoutineID::default());
    }

    #[test]
    fn test_sort_routines_by_last_use() {
        let routines = [routine(1), routine(2), routine(3), routine(4)];
        let training_sessions = [
            training_session(1, 3, NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            training_session(2, 2, NaiveDate::from_ymd_opt(2020, 3, 3).unwrap()),
            training_session(3, 3, NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
        ];
        assert_eq!(
            routines_sorted_by_last_use(&routines, &training_sessions, |_| true),
            vec![routine(2), routine(3), routine(4), routine(1)]
        );
    }

    #[test]
    fn test_sort_routines_by_last_use_empty() {
        let routines = [];
        let training_sessions = [];
        assert_eq!(
            routines_sorted_by_last_use(&routines, &training_sessions, |_| true),
            vec![]
        );
    }

    #[test]
    fn test_sort_routines_by_last_use_missing_routines() {
        let routines = [routine(1), routine(2)];
        let training_sessions = [
            training_session(1, 3, NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            training_session(2, 2, NaiveDate::from_ymd_opt(2020, 3, 3).unwrap()),
            training_session(3, 3, NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
        ];
        assert_eq!(
            routines_sorted_by_last_use(&routines, &training_sessions, |_| true),
            vec![routine(2), routine(1)]
        );
    }

    #[test]
    fn test_sort_routines_by_last_use_filter() {
        let routines = [routine(1), routine(2), routine(3), routine(4)];
        let training_sessions = [
            training_session(1, 3, NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            training_session(2, 2, NaiveDate::from_ymd_opt(2020, 3, 3).unwrap()),
            training_session(3, 3, NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
        ];
        assert_eq!(
            routines_sorted_by_last_use(&routines, &training_sessions, |r| r.id > 2.into()),
            vec![routine(3), routine(4)]
        );
    }

    fn routine(id: u128) -> Routine {
        Routine {
            id: id.into(),
            name: Name::new(&id.to_string()).unwrap(),
            notes: String::new(),
            archived: false,
            sections: vec![],
        }
    }

    fn training_session(id: u128, routine_id: u128, date: NaiveDate) -> TrainingSession {
        TrainingSession {
            id: id.into(),
            routine_id: RoutineID::from(routine_id),
            date,
            notes: String::new(),
            elements: vec![],
        }
    }
}
