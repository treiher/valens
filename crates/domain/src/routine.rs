use std::collections::{BTreeMap, BTreeSet};

use chrono::Duration;
use derive_more::Deref;
use uuid::Uuid;

use crate::{
    CreateError, DeleteError, Exercise, ExerciseID, MuscleID, Name, Property, RPE, ReadError, Reps,
    Stimulus, SyncError, Time, UpdateError, Weight,
};

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
}
