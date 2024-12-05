use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use chrono::{Duration, NaiveDate};

use crate::domain;

pub mod rest;

#[async_trait(?Send)]
pub trait Storage {
    async fn request_session(&self, user_id: u32) -> Result<Session, String>;
    async fn initialize_session(&self) -> Result<Session, String>;
    async fn delete_session(&self) -> Result<(), String>;

    async fn read_version(&self) -> Result<String, String>;

    async fn read_users(&self) -> Result<Vec<User>, String>;
    async fn create_user(&self, user: NewUser) -> Result<User, String>;
    async fn replace_user(&self, user: User) -> Result<User, String>;
    async fn delete_user(&self, id: u32) -> Result<u32, String>;

    async fn read_body_weight(&self) -> Result<Vec<BodyWeight>, String>;
    async fn create_body_weight(&self, body_weight: BodyWeight) -> Result<BodyWeight, String>;
    async fn replace_body_weight(&self, body_weight: BodyWeight) -> Result<BodyWeight, String>;
    async fn delete_body_weight(&self, date: NaiveDate) -> Result<NaiveDate, String>;

    async fn read_body_fat(&self) -> Result<Vec<BodyFat>, String>;
    async fn create_body_fat(&self, body_fat: BodyFat) -> Result<BodyFat, String>;
    async fn replace_body_fat(&self, body_fat: BodyFat) -> Result<BodyFat, String>;
    async fn delete_body_fat(&self, date: NaiveDate) -> Result<NaiveDate, String>;

    async fn read_period(&self) -> Result<Vec<Period>, String>;
    async fn create_period(&self, period: Period) -> Result<Period, String>;
    async fn replace_period(&self, period: Period) -> Result<Period, String>;
    async fn delete_period(&self, date: NaiveDate) -> Result<NaiveDate, String>;

    async fn read_exercises(&self) -> Result<Vec<Exercise>, String>;
    async fn create_exercise(
        &self,
        name: String,
        muscles: Vec<ExerciseMuscle>,
    ) -> Result<Exercise, String>;
    async fn replace_exercise(&self, exercise: Exercise) -> Result<Exercise, String>;
    async fn delete_exercise(&self, id: u32) -> Result<u32, String>;

    async fn read_routines(&self) -> Result<Vec<Routine>, String>;
    async fn create_routine(
        &self,
        name: String,
        sections: Vec<RoutinePart>,
    ) -> Result<Routine, String>;
    async fn modify_routine(
        &self,
        id: u32,
        name: Option<String>,
        archived: Option<bool>,
        sections: Option<Vec<RoutinePart>>,
    ) -> Result<Routine, String>;
    async fn delete_routine(&self, id: u32) -> Result<u32, String>;

    async fn read_training_sessions(&self) -> Result<Vec<TrainingSession>, String>;
    async fn create_training_session(
        &self,
        routine_id: Option<u32>,
        date: NaiveDate,
        notes: String,
        elements: Vec<TrainingSessionElement>,
    ) -> Result<TrainingSession, String>;
    async fn modify_training_session(
        &self,
        id: u32,
        notes: Option<String>,
        elements: Option<Vec<TrainingSessionElement>>,
    ) -> Result<TrainingSession, String>;
    async fn delete_training_session(&self, id: u32) -> Result<u32, String>;
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Session {
    #[allow(dead_code)]
    pub id: u32,
    pub name: String,
    pub sex: u8,
}

#[derive(serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub sex: u8,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct NewUser {
    pub name: String,
    pub sex: u8,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct BodyWeight {
    pub date: NaiveDate,
    pub weight: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BodyFat {
    pub date: NaiveDate,
    pub chest: Option<u8>,
    pub abdominal: Option<u8>,
    pub thigh: Option<u8>,
    pub tricep: Option<u8>,
    pub subscapular: Option<u8>,
    pub suprailiac: Option<u8>,
    pub midaxillary: Option<u8>,
}

impl BodyFat {
    pub fn jp3(&self, sex: u8) -> Option<f32> {
        if sex == 0 {
            Some(Self::jackson_pollock(
                f32::from(self.tricep?) + f32::from(self.suprailiac?) + f32::from(self.thigh?),
                1.099_492_1,
                0.000_992_9,
                0.000_002_3,
                0.000_139_2,
            ))
        } else if sex == 1 {
            Some(Self::jackson_pollock(
                f32::from(self.chest?) + f32::from(self.abdominal?) + f32::from(self.thigh?),
                1.109_38,
                0.000_826_7,
                0.000_001_6,
                0.000_257_4,
            ))
        } else {
            None
        }
    }

    pub fn jp7(&self, sex: u8) -> Option<f32> {
        if sex == 0 {
            Some(Self::jackson_pollock(
                f32::from(self.chest?)
                    + f32::from(self.abdominal?)
                    + f32::from(self.thigh?)
                    + f32::from(self.tricep?)
                    + f32::from(self.subscapular?)
                    + f32::from(self.suprailiac?)
                    + f32::from(self.midaxillary?),
                1.097,
                0.000_469_71,
                0.000_000_56,
                0.000_128_28,
            ))
        } else if sex == 1 {
            Some(Self::jackson_pollock(
                f32::from(self.chest?)
                    + f32::from(self.abdominal?)
                    + f32::from(self.thigh?)
                    + f32::from(self.tricep?)
                    + f32::from(self.subscapular?)
                    + f32::from(self.suprailiac?)
                    + f32::from(self.midaxillary?),
                1.112,
                0.000_434_99,
                0.000_000_55,
                0.000_288_26,
            ))
        } else {
            None
        }
    }

    fn jackson_pollock(sum: f32, k0: f32, k1: f32, k2: f32, ka: f32) -> f32 {
        let age = 30.; // assume an age of 30
        (495. / (k0 - (k1 * sum) + (k2 * sum * sum) - (ka * age))) - 450.
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Period {
    pub date: NaiveDate,
    pub intensity: u8,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Exercise {
    pub id: u32,
    pub name: String,
    pub muscles: Vec<ExerciseMuscle>,
}

impl Exercise {
    pub fn muscle_stimulus(&self) -> BTreeMap<u8, u8> {
        self.muscles
            .iter()
            .map(|m| (m.muscle_id, m.stimulus))
            .collect()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ExerciseMuscle {
    pub muscle_id: u8,
    pub stimulus: u8,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Routine {
    pub id: u32,
    pub name: String,
    pub notes: Option<String>,
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

    pub fn stimulus_per_muscle(&self, exercises: &BTreeMap<u32, Exercise>) -> BTreeMap<u8, u32> {
        let mut result: BTreeMap<u8, u32> = domain::Muscle::iter()
            .map(|m| (domain::Muscle::id(*m), 0))
            .collect();
        for section in &self.sections {
            for (id, stimulus) in section.stimulus_per_muscle(exercises) {
                if result.contains_key(&id) {
                    *result.entry(id).or_insert(0) += stimulus;
                }
            }
        }
        result
    }

    pub fn exercises(&self) -> BTreeSet<u32> {
        self.sections
            .iter()
            .flat_map(RoutinePart::exercises)
            .collect::<BTreeSet<_>>()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum RoutinePart {
    RoutineSection {
        rounds: u32,
        parts: Vec<RoutinePart>,
    },
    RoutineActivity {
        exercise_id: Option<u32>,
        reps: u32,
        time: u32,
        weight: f32,
        rpe: f32,
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
                let r = if *reps > 0 { *reps } else { 1 };
                let t = if *time > 0 { *time } else { 4 };
                Duration::seconds(i64::from(r * t))
            }
        }
    }

    pub fn num_sets(&self) -> u32 {
        match self {
            RoutinePart::RoutineSection { rounds, parts } => {
                parts.iter().map(RoutinePart::num_sets).sum::<u32>() * *rounds
            }
            RoutinePart::RoutineActivity { exercise_id, .. } => exercise_id.is_some().into(),
        }
    }

    pub fn stimulus_per_muscle(&self, exercises: &BTreeMap<u32, Exercise>) -> BTreeMap<u8, u32> {
        match self {
            RoutinePart::RoutineSection { rounds, parts } => {
                let mut result: BTreeMap<u8, u32> = BTreeMap::new();
                for part in parts {
                    for (id, stimulus) in part.stimulus_per_muscle(exercises) {
                        *result.entry(id).or_insert(0) += stimulus * rounds;
                    }
                }
                result
            }
            RoutinePart::RoutineActivity { exercise_id, .. } => exercises
                .get(&exercise_id.unwrap_or_default())
                .map(|e| {
                    e.muscle_stimulus()
                        .iter()
                        .map(|(id, stimulus)| (*id, u32::from(*stimulus)))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    fn exercises(&self) -> BTreeSet<u32> {
        let mut result: BTreeSet<u32> = BTreeSet::new();
        match self {
            RoutinePart::RoutineSection { parts, .. } => {
                for p in parts {
                    result.extend(Self::exercises(p));
                }
            }
            RoutinePart::RoutineActivity { exercise_id, .. } => {
                if let Some(id) = exercise_id {
                    result.insert(*id);
                }
            }
        }
        result
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct TrainingSession {
    pub id: u32,
    pub routine_id: Option<u32>,
    pub date: NaiveDate,
    pub notes: Option<String>,
    pub elements: Vec<TrainingSessionElement>,
}

impl TrainingSession {
    pub fn exercises(&self) -> BTreeSet<u32> {
        self.elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { exercise_id, .. } => Some(*exercise_id),
                _ => None,
            })
            .collect::<BTreeSet<_>>()
    }

    pub fn avg_reps(&self) -> Option<f32> {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { reps, .. } => *reps,
                _ => None,
            })
            .collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            Some(sets.iter().sum::<u32>() as f32 / sets.len() as f32)
        }
    }

    pub fn avg_time(&self) -> Option<f32> {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { time, .. } => *time,
                _ => None,
            })
            .collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            Some(sets.iter().sum::<u32>() as f32 / sets.len() as f32)
        }
    }

    pub fn avg_weight(&self) -> Option<f32> {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { weight, .. } => *weight,
                _ => None,
            })
            .collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            Some(sets.iter().sum::<f32>() / sets.len() as f32)
        }
    }

    pub fn avg_rpe(&self) -> Option<f32> {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { rpe, .. } => *rpe,
                _ => None,
            })
            .collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            Some(sets.iter().sum::<f32>() / sets.len() as f32)
        }
    }

    pub fn load(&self) -> u32 {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set {
                    reps, time, rpe, ..
                } => Some(if let Some(rpe) = *rpe {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    if rpe > 5.0 {
                        (2.0_f32).powf(rpe - 5.0).round() as u32
                    } else {
                        1
                    }
                } else {
                    u32::from(reps.is_some() || time.is_some())
                }),
                _ => None,
            })
            .collect::<Vec<_>>();
        sets.iter().sum::<u32>()
    }

    pub fn set_volume(&self) -> u32 {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { rpe, .. } => {
                    if rpe.unwrap_or(10.0) >= 7.0 {
                        Some(1)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        sets.iter().sum::<u32>()
    }

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
                            Some((*reps as f32 * weight).round() as u32)
                        } else {
                            Some(*reps)
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        sets.iter().sum::<u32>()
    }

    pub fn tut(&self) -> Option<u32> {
        let sets = &self
            .elements
            .iter()
            .map(|e| match e {
                TrainingSessionElement::Set { reps, time, .. } => {
                    time.as_ref().map(|v| reps.unwrap_or(1) * v)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if sets.iter().all(Option::is_none) {
            return None;
        }
        Some(sets.iter().filter_map(|e| *e).sum::<u32>())
    }

    pub fn stimulus_per_muscle(&self, exercises: &BTreeMap<u32, Exercise>) -> BTreeMap<u8, u32> {
        let mut result: BTreeMap<u8, u32> = BTreeMap::new();
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
                    if *rpe < 7.0 {
                        continue;
                    }
                }
                if let Some(exercise) = exercises.get(exercise_id) {
                    for (id, stimulus) in &exercise.muscle_stimulus() {
                        *result.entry(*id).or_insert(0) += u32::from(*stimulus);
                    }
                }
            }
        }
        result
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum TrainingSessionElement {
    Set {
        exercise_id: u32,
        reps: Option<u32>,
        time: Option<u32>,
        weight: Option<f32>,
        rpe: Option<f32>,
        target_reps: Option<u32>,
        target_time: Option<u32>,
        target_weight: Option<f32>,
        target_rpe: Option<f32>,
        automatic: bool,
    },
    Rest {
        target_time: Option<u32>,
        automatic: bool,
    },
}
