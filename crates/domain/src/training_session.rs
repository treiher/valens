use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    ops::RangeInclusive,
    str::FromStr,
};

use chrono::{Local, NaiveDate};
use derive_more::Deref;
use log::error;
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

    fn validate_training_session_date(&self, date: &str) -> Result<NaiveDate, ValidationError> {
        match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            Ok(parsed_date) => {
                if parsed_date <= Local::now().date_naive() {
                    Ok(parsed_date)
                } else {
                    Err(ValidationError::Other(
                        "Date must not be in the future".into(),
                    ))
                }
            }
            Err(_) => Err(ValidationError::Other("Invalid date".into())),
        }
    }

    fn get_training_stats(&self, training_sessions: &[TrainingSession]) -> TrainingStats {
        training_stats(&training_sessions.iter().collect::<Vec<_>>())
    }

    /// Returns all non-empty sets from the training session, grouped by exercise.
    fn get_sets_by_exercise<'a>(
        &self,
        training_session: &'a TrainingSession,
    ) -> HashMap<ExerciseID, Vec<&'a TrainingSessionElement>> {
        let mut result: HashMap<ExerciseID, Vec<&'a TrainingSessionElement>> = HashMap::new();
        for element in &training_session.elements {
            if let TrainingSessionElement::Set {
                exercise_id,
                reps,
                time,
                weight,
                rpe,
                ..
            } = element
            {
                if reps.is_some() || time.is_some() || weight.is_some() || rpe.is_some() {
                    result.entry(*exercise_id).or_default().push(element);
                }
            }
        }
        result
    }

    /// Returns all non-empty sets from the previous training session, grouped by exercise.
    ///
    /// The previous session is the most recent training session for the same routine that occurred
    /// before the current one. If no such session exists, this returns an empty map.
    fn get_previous_session_sets_by_exercise<'a>(
        &self,
        training_session: &TrainingSession,
        training_sessions: &'a [TrainingSession],
    ) -> HashMap<ExerciseID, Vec<&'a TrainingSessionElement>> {
        if let Some(previous_training_session) = training_sessions
            .iter()
            .filter(|t| {
                t.id != training_session.id
                    && t.date < training_session.date
                    && t.routine_id == training_session.routine_id
            })
            .max_by_key(|t| (t.date, t.id))
        {
            self.get_sets_by_exercise(previous_training_session)
        } else {
            HashMap::new()
        }
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
    pub fn stimulus_per_muscle(&self, exercises: &[Exercise]) -> BTreeMap<MuscleID, Stimulus> {
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
                if let Some(exercise) = exercises.iter().find(|e| e.id == *exercise_id) {
                    for (muscle_id, stimulus) in &exercise.muscle_stimulus() {
                        *result.entry(*muscle_id).or_insert(Stimulus::NONE) += *stimulus;
                    }
                }
            }
        }
        result
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.elements.iter().all(TrainingSessionElement::is_empty)
    }

    pub fn add_set(&mut self, element_idx: usize) {
        let section_idx = self.section_idx(element_idx);
        let sections = self.compute_sections();
        let Some(section) = sections.get(section_idx) else {
            return;
        };
        let mut section_elements = section.elements().to_vec();
        let rests = section_elements
            .iter()
            .filter(|e| matches!(e, TrainingSessionElement::Rest { .. }))
            .collect::<Vec<_>>();
        let rest = if let Some(rest) = rests.first() {
            (*rest).clone()
        } else {
            TrainingSessionElement::Rest {
                target_time: None,
                automatic: true,
            }
        };
        let mut sets = vec![];
        for element in &section_elements {
            match element {
                TrainingSessionElement::Set {
                    exercise_id,
                    target_reps,
                    target_time,
                    target_weight,
                    target_rpe,
                    automatic,
                    ..
                } => {
                    sets.push(TrainingSessionElement::Set {
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
                    });
                }
                TrainingSessionElement::Rest { .. } => {
                    break;
                }
            }
        }

        if matches!(
            section_elements.last(),
            Some(TrainingSessionElement::Set { .. })
        ) {
            section_elements.push(rest);
            section_elements.extend(sets);
        } else {
            section_elements.extend(sets);
            section_elements.push(rest);
        }

        self.replace_elements_of_section(&sections, section_idx, section_elements);
        self.ensure_sections_contain_set("adding set");
    }

    pub fn add_same_exercise(&mut self, section_idx: usize, exercise_idx: usize) {
        let section = &self.compute_sections()[section_idx];
        let exercise_id = section.exercise_ids()[exercise_idx];

        self.add_exercise(section_idx, exercise_id);
        self.ensure_sections_contain_set("adding same exercise");
    }

    pub fn add_exercise(&mut self, section_idx: usize, exercise_id: ExerciseID) {
        let sections = &self.compute_sections();
        let section = &sections[section_idx];
        println!("{section:?}");

        let mut elements = vec![];
        for element in section.elements() {
            let element = element.clone();
            match element {
                TrainingSessionElement::Set { .. } => {
                    elements.push(element);
                }
                TrainingSessionElement::Rest { .. } => {
                    elements.push(TrainingSessionElement::Set {
                        exercise_id,
                        reps: None,
                        time: None,
                        weight: None,
                        rpe: None,
                        target_reps: None,
                        target_time: None,
                        target_weight: None,
                        target_rpe: None,
                        automatic: false,
                    });
                    elements.push(element);
                }
            }
        }

        if let Some(TrainingSessionElement::Set { .. }) = section.elements().last() {
            elements.push(TrainingSessionElement::Set {
                exercise_id,
                reps: None,
                time: None,
                weight: None,
                rpe: None,
                target_reps: None,
                target_time: None,
                target_weight: None,
                target_rpe: None,
                automatic: false,
            });
        }

        self.replace_elements_of_section(sections, section_idx, elements);
        self.ensure_sections_contain_set("adding exercise");
    }

    pub fn replace_exercise(
        &mut self,
        section_idx: usize,
        exercise_idx: usize,
        exercise_id: ExerciseID,
    ) {
        let sections = self.compute_sections();
        let section = &sections[section_idx];
        let exercises = section.exercise_ids();
        let replace_all = exercise_idx == 0
            && exercises
                .first()
                .is_none_or(|first| exercises.iter().all(|id| id == first));

        let mut elements = vec![];
        let mut idx = 0;
        for element in section.elements() {
            match element {
                TrainingSessionElement::Set {
                    reps,
                    time,
                    weight,
                    rpe,
                    target_reps,
                    target_time,
                    target_weight,
                    target_rpe,
                    automatic,
                    ..
                } => {
                    if idx == exercise_idx || replace_all {
                        elements.push(TrainingSessionElement::Set {
                            exercise_id,
                            reps: *reps,
                            time: *time,
                            weight: *weight,
                            rpe: *rpe,
                            target_reps: *target_reps,
                            target_time: *target_time,
                            target_weight: *target_weight,
                            target_rpe: *target_rpe,
                            automatic: *automatic,
                        });
                    } else {
                        elements.push(element.clone());
                    }
                    idx += 1;
                }
                TrainingSessionElement::Rest { .. } => {
                    elements.push(element.clone());
                    idx = 0;
                }
            }
        }

        self.replace_elements_of_section(&sections, section_idx, elements);
        self.ensure_sections_contain_set("replacing exercise");
    }

    pub fn remove_set(&mut self, section_idx: usize) {
        let section = self.section_range(section_idx);
        let end = *section.end();
        for i in section.rev() {
            if i != end && matches!(self.elements[i], TrainingSessionElement::Rest { .. }) {
                break;
            }
            self.elements.remove(i);
        }
        self.ensure_sections_contain_set("removing set");
    }

    pub fn remove_exercise(&mut self, section_idx: usize, exercise_idx: usize) {
        let sections = self.compute_sections();
        let section = &sections[section_idx];

        let mut elements = vec![];
        if section.exercise_ids().len() > 1 {
            let id = section.exercise_ids()[exercise_idx];
            let mut removed = false;
            for element in section.elements().iter().rev() {
                match element {
                    TrainingSessionElement::Set { exercise_id, .. } => {
                        if !removed && *exercise_id == id {
                            removed = true;
                        } else {
                            elements.push(element.clone());
                        }
                    }
                    TrainingSessionElement::Rest { .. } => {
                        elements.push(element.clone());
                        removed = false;
                    }
                }
            }
        }
        elements.reverse();

        self.replace_elements_of_section(&sections, section_idx, elements);
        self.ensure_sections_contain_set("removing exercise");
    }

    pub fn append_exercise(&mut self, exercise_id: ExerciseID) {
        if let Some(TrainingSessionElement::Set { .. }) = self.elements.last() {
            self.elements.push(TrainingSessionElement::Rest {
                target_time: None,
                automatic: true,
            });
        }
        self.elements.push(TrainingSessionElement::Set {
            exercise_id,
            reps: None,
            time: None,
            weight: None,
            rpe: None,
            target_reps: None,
            target_time: None,
            target_weight: None,
            target_rpe: None,
            automatic: false,
        });
        self.ensure_sections_contain_set("appending exercise");
    }

    pub fn move_section_up(&mut self, section_idx: usize) {
        if section_idx == 0 {
            return;
        }
        let section = self.section_range(section_idx);
        debug_assert!(section.start() <= section.end());
        let previous_section = self.section_range(section_idx - 1);
        let mut trailing_rest = 0;
        if section.end() + 1 == self.elements.len() {
            if let Some(TrainingSessionElement::Set { .. }) = self.elements.last() {
                self.elements.push(TrainingSessionElement::Rest {
                    target_time: None,
                    automatic: true,
                });
                trailing_rest += 1;
            }
        }
        self.elements[*previous_section.start()..=*section.end() + trailing_rest]
            .rotate_right(section.end() - section.start() + trailing_rest + 1);
        self.ensure_sections_contain_set("moving section up");
    }

    pub fn move_section_down(&mut self, section_idx: usize) {
        let section = self.section_range(section_idx);
        if *section.end() + 1 == self.elements.len() {
            return;
        }
        let subsequent_section = self.section_range(section_idx + 1);
        let section_len = section.end() - section.start() + 1;
        let subsequent_section_len = subsequent_section.end() - subsequent_section.start() + 1;
        let mut trailing_rest = 0;
        if section.start() + section_len + subsequent_section_len == self.elements.len() {
            if let Some(TrainingSessionElement::Set { .. }) = self.elements.last() {
                self.elements.push(TrainingSessionElement::Rest {
                    target_time: None,
                    automatic: true,
                });
                trailing_rest += 1;
            }
        }
        self.elements[*section.start()
            ..*section.start() + section_len + subsequent_section_len + trailing_rest]
            .rotate_right(subsequent_section_len + trailing_rest);
        self.ensure_sections_contain_set("moving section down");
    }

    fn ensure_sections_contain_set(&mut self, action: &str) {
        let sections = self.compute_sections();
        let has_rest_only_section = sections.iter().any(|s| {
            !s.elements()
                .iter()
                .any(|e| matches!(e, TrainingSessionElement::Set { .. }))
        });
        if has_rest_only_section {
            debug_assert!(
                false,
                "{action} resulted in a section consisting only of rest elements"
            );
            error!("{action} resulted in a section consisting only of rest elements");
            self.elements = sections
                .into_iter()
                .filter(|s| {
                    s.elements()
                        .iter()
                        .any(|e| matches!(e, TrainingSessionElement::Set { .. }))
                })
                .flat_map(|s| s.elements().to_vec())
                .collect();
        }
    }

    #[must_use]
    fn section_idx(&self, element_idx: usize) -> usize {
        let mut section_idx = 0;
        let mut idx = 0;

        while idx < self.elements.len() {
            let last = Self::find_last_of_section(self, idx);
            if (idx..=last).contains(&element_idx) {
                break;
            }
            section_idx += 1;
            idx = last + 1;
        }

        section_idx
    }

    #[must_use]
    fn section_range(&self, section_idx: usize) -> RangeInclusive<usize> {
        let mut first = 0;
        let mut last = 0;
        let mut idx = 0;

        while first < self.elements.len() {
            last = Self::find_last_of_section(self, first);
            if idx == section_idx {
                break;
            }
            idx += 1;
            first = last + 1;
        }

        first..=last
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

        debug_assert!(element_idx <= last);

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

    fn replace_elements_of_section(
        &mut self,
        sections: &[TrainingSessionSection],
        section_idx: usize,
        elements: Vec<TrainingSessionElement>,
    ) {
        self.elements = sections[..section_idx]
            .iter()
            .flat_map(|section| section.elements().iter().cloned())
            .chain(elements)
            .chain(
                sections[section_idx + 1..]
                    .iter()
                    .flat_map(|section| section.elements().iter().cloned()),
            )
            .collect::<Vec<_>>();
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
    pub fn is_empty(&self) -> bool {
        match self {
            TrainingSessionElement::Set {
                reps,
                time,
                weight,
                rpe,
                ..
            } => {
                reps.is_none_or(|reps| reps == Reps::default())
                    && time.is_none_or(|time| time == Time::default())
                    && weight.is_none_or(|weight| weight == Weight::default())
                    && rpe.is_none_or(|rpe| rpe == RPE::default())
            }
            TrainingSessionElement::Rest { .. } => true,
        }
    }

    #[must_use]
    pub fn to_string(&self, show_tut: bool, show_rpe: bool) -> String {
        match self {
            TrainingSessionElement::Set {
                reps,
                time,
                weight,
                rpe,
                ..
            } => Set {
                reps: reps.unwrap_or_default(),
                time: time.unwrap_or_default(),
                weight: weight.unwrap_or_default(),
                rpe: rpe.unwrap_or_default(),
            }
            .to_string(show_tut, show_rpe),
            TrainingSessionElement::Rest { .. } => String::new(),
        }
    }

    #[must_use]
    pub fn target_to_string(&self, show_tut: bool, show_rpe: bool) -> String {
        match self {
            TrainingSessionElement::Set {
                target_reps,
                target_time,
                target_weight,
                target_rpe,
                ..
            } => Set {
                reps: target_reps.unwrap_or_default(),
                time: target_time.unwrap_or_default(),
                weight: target_weight.unwrap_or_default(),
                rpe: target_rpe.unwrap_or_default(),
            }
            .to_string(show_tut, show_rpe),
            TrainingSessionElement::Rest { .. } => String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Set {
    pub reps: Reps,
    pub time: Time,
    pub weight: Weight,
    pub rpe: RPE,
}

impl Set {
    #[must_use]
    pub fn to_string(&self, show_tut: bool, show_rpe: bool) -> String {
        let mut parts = vec![];

        if self.reps > Reps::default() {
            parts.push(self.reps.to_string());
        }

        if show_tut && self.time > Time::default() {
            parts.push(format!("{} s", self.time));
        }

        if self.weight > Weight::default() {
            parts.push(format!("{} kg", self.weight));
        }

        let mut result = parts.join(" × ");

        if show_rpe && self.rpe > RPE::ZERO {
            result.push_str(&format!(" @ {}", self.rpe));
        }

        result
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrainingSessionSection(Vec<TrainingSessionElement>);

impl TrainingSessionSection {
    #[must_use]
    pub fn elements(&self) -> &[TrainingSessionElement] {
        &self.0
    }

    #[must_use]
    pub fn exercise_ids(&self) -> Vec<ExerciseID> {
        next_consecutive_exercise_ids(&self.0)
    }

    /// Returns the number of occurrences of each exercise in a set.
    #[must_use]
    pub fn exercise_counts(&self) -> HashMap<ExerciseID, usize> {
        let mut counts = HashMap::new();

        for id in self.exercise_ids() {
            *counts.entry(id).or_insert(0) += 1;
        }

        counts
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
        let exercises = [Exercise {
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
        }];
        assert_eq!(training_session.stimulus_per_muscle(&exercises), expected);
    }

    #[test]
    fn test_training_session_id_nil() {
        assert!(TrainingSessionID::nil().is_nil());
        assert_eq!(TrainingSessionID::nil(), TrainingSessionID::default());
    }

    #[test]
    fn test_training_session_move_section_up_first() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 1),
            rest(1),
            exercise(2, 2),
            rest(2),
        ]);
        training_session.move_section_up(0);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(1, 1),
                rest(1),
                exercise(2, 2),
                rest(2),
            ]
        );
    }

    #[test]
    fn test_training_session_move_section_up_penultimate_without_trailing_rest() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 1),
            rest(1),
            exercise(2, 2),
        ]);
        training_session.move_section_up(1);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(1, 1),
                rest(1),
                exercise(0, 0),
                rest(0),
                exercise(2, 2),
            ]
        );
    }

    #[test]
    fn test_training_session_move_section_up_last() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 1),
            rest(1),
            exercise(2, 2),
            rest(2),
        ]);
        training_session.move_section_up(2);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(2, 2),
                rest(2),
                exercise(1, 1),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_training_session_move_section_up_last_without_trailing_rest() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 1),
            rest(1),
            exercise(2, 2),
        ]);
        training_session.move_section_up(2);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(2, 2),
                rest(0),
                exercise(1, 1),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_training_session_move_section_up_multiple_sets() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
            exercise(4, 2),
            rest(4),
            exercise(5, 2),
            rest(5),
        ]);
        training_session.move_section_up(1);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
                exercise(0, 0),
                rest(0),
                exercise(1, 0),
                rest(1),
                exercise(4, 2),
                rest(4),
                exercise(5, 2),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_move_section_up_supersets() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 1),
            rest(0),
            exercise(2, 0),
            exercise(3, 1),
            rest(1),
            exercise(4, 0),
            exercise(5, 2),
            rest(2),
            exercise(6, 0),
            exercise(7, 2),
            rest(3),
            exercise(8, 1),
            exercise(9, 2),
            rest(4),
            exercise(10, 1),
            exercise(11, 2),
            rest(5),
        ]);
        training_session.move_section_up(1);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(4, 0),
                exercise(5, 2),
                rest(2),
                exercise(6, 0),
                exercise(7, 2),
                rest(3),
                exercise(0, 0),
                exercise(1, 1),
                rest(0),
                exercise(2, 0),
                exercise(3, 1),
                rest(1),
                exercise(8, 1),
                exercise(9, 2),
                rest(4),
                exercise(10, 1),
                exercise(11, 2),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_move_section_down_first() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 1),
            rest(1),
            exercise(2, 2),
            rest(2),
        ]);
        training_session.move_section_down(0);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(1, 1),
                rest(1),
                exercise(0, 0),
                rest(0),
                exercise(2, 2),
                rest(2),
            ]
        );
    }

    #[test]
    fn test_training_session_move_section_down_penultimate() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 1),
            rest(1),
            exercise(2, 2),
            rest(2),
        ]);
        training_session.move_section_down(1);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(2, 2),
                rest(2),
                exercise(1, 1),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_training_session_move_section_down_penultimate_without_trailing_rest() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 1),
            rest(1),
            exercise(2, 2),
        ]);
        training_session.move_section_down(1);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(2, 2),
                rest(0),
                exercise(1, 1),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_training_session_move_section_down_last() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 1),
            rest(1),
            exercise(2, 2),
            rest(2),
        ]);
        training_session.move_section_down(2);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(1, 1),
                rest(1),
                exercise(2, 2),
                rest(2),
            ]
        );
    }

    #[test]
    fn test_training_session_move_section_down_multiple_sets() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
            exercise(4, 2),
            rest(4),
            exercise(5, 2),
            rest(5),
        ]);
        training_session.move_section_down(0);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
                exercise(0, 0),
                rest(0),
                exercise(1, 0),
                rest(1),
                exercise(4, 2),
                rest(4),
                exercise(5, 2),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_move_section_down_supersets() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 1),
            rest(0),
            exercise(2, 0),
            exercise(3, 1),
            rest(1),
            exercise(4, 0),
            exercise(5, 2),
            rest(2),
            exercise(6, 0),
            exercise(7, 2),
            rest(3),
            exercise(8, 1),
            exercise(9, 2),
            rest(4),
            exercise(10, 1),
            exercise(11, 2),
            rest(5),
        ]);
        training_session.move_section_down(1);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                exercise(1, 1),
                rest(0),
                exercise(2, 0),
                exercise(3, 1),
                rest(1),
                exercise(8, 1),
                exercise(9, 2),
                rest(4),
                exercise(10, 1),
                exercise(11, 2),
                rest(5),
                exercise(4, 0),
                exercise(5, 2),
                rest(2),
                exercise(6, 0),
                exercise(7, 2),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_training_session_add_set_first_set() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
        ]);
        training_session.add_set(0);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(1, 0),
                rest(1),
                exercise(0, 0),
                rest(0),
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_training_session_add_set_second_set() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
        ]);
        training_session.add_set(2);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(1, 0),
                rest(1),
                exercise(0, 0),
                rest(0),
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_training_session_add_set_penultimate_set() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
        ]);
        training_session.add_set(4);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(1, 0),
                rest(1),
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
                exercise(2, 1),
                rest(2),
            ]
        );
    }

    #[test]
    fn test_training_session_add_set_last_set() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
        ]);
        training_session.add_set(6);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(1, 0),
                rest(1),
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
                exercise(2, 1),
                rest(2),
            ]
        );
    }

    #[test]
    fn test_training_session_add_set_superset() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(4, 2),
            rest(0),
            exercise(1, 0),
            exercise(5, 2),
            rest(1),
        ]);
        training_session.add_set(0);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                exercise(4, 2),
                rest(0),
                exercise(1, 0),
                exercise(5, 2),
                rest(1),
                exercise(0, 0),
                exercise(4, 2),
                rest(0),
            ]
        );
    }

    #[test]
    fn test_training_session_add_set_no_rest_first_set() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 0),
            exercise(2, 1),
            exercise(3, 1),
        ]);
        training_session.add_set(0);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                exercise(1, 0),
                exercise(2, 1),
                exercise(3, 1),
                rest(0),
                exercise(0, 0),
                exercise(1, 0),
                exercise(2, 1),
                exercise(3, 1),
            ]
        );
    }

    #[test]
    fn test_training_session_add_set_no_rest_second_set() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 0),
            exercise(2, 1),
            exercise(3, 1),
        ]);
        training_session.add_set(1);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                exercise(1, 0),
                exercise(2, 1),
                exercise(3, 1),
                rest(0),
                exercise(0, 0),
                exercise(1, 0),
                exercise(2, 1),
                exercise(3, 1),
            ]
        );
    }

    #[test]
    fn test_training_session_add_set_no_rest_penultimate_set() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 0),
            exercise(2, 1),
            exercise(3, 1),
        ]);
        training_session.add_set(2);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                exercise(1, 0),
                exercise(2, 1),
                exercise(3, 1),
                rest(0),
                exercise(0, 0),
                exercise(1, 0),
                exercise(2, 1),
                exercise(3, 1),
            ]
        );
    }

    #[test]
    fn test_training_session_add_set_no_rest_last_set() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 0),
            exercise(2, 1),
            exercise(3, 1),
        ]);
        training_session.add_set(3);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                exercise(1, 0),
                exercise(2, 1),
                exercise(3, 1),
                rest(0),
                exercise(0, 0),
                exercise(1, 0),
                exercise(2, 1),
                exercise(3, 1),
            ]
        );
    }

    #[test]
    fn test_training_session_add_set_first_single_set() {
        let mut training_session = training_session(&[exercise(0, 0), rest(0), exercise(1, 1)]);
        training_session.add_set(0);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(0, 0),
                rest(0),
                exercise(1, 1),
            ]
        );
    }

    #[test]
    fn test_training_session_add_set_last_single_set() {
        let mut training_session = training_session(&[exercise(0, 0), rest(0), exercise(1, 1)]);
        training_session.add_set(2);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(1, 1),
                rest(0),
                exercise(1, 1),
            ]
        );
    }

    #[test]
    fn test_training_session_add_set_invalid_element_idx_out_of_range() {
        let mut training_session =
            training_session(&[exercise(0, 0), rest(0), exercise(1, 0), rest(1)]);
        training_session.add_set(4);
        assert_eq!(
            training_session.elements,
            vec![exercise(0, 0), rest(0), exercise(1, 0), rest(1),]
        );
    }

    #[test]
    fn test_training_session_add_same_exercise_first() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
            exercise(4, 0),
            rest(4),
            exercise(5, 0),
            rest(5),
        ]);
        training_session.add_same_exercise(0, 0);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                exercise(0, 0),
                rest(0),
                exercise(1, 0),
                exercise(0, 0),
                rest(1),
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
                exercise(4, 0),
                rest(4),
                exercise(5, 0),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_add_same_exercise_last() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
            exercise(4, 0),
            rest(4),
            exercise(5, 0),
            rest(5),
        ]);
        training_session.add_same_exercise(2, 0);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(1, 0),
                rest(1),
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
                exercise(4, 0),
                exercise(0, 0),
                rest(4),
                exercise(5, 0),
                exercise(0, 0),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_add_same_exercise_no_rest() {
        let mut training_session = training_session(&[exercise(0, 0)]);
        training_session.add_same_exercise(0, 0);
        assert_eq!(
            training_session.elements,
            vec![exercise(0, 0), exercise(0, 0)]
        );
    }

    #[test]
    fn test_training_session_add_exercise_first() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
            exercise(4, 0),
            rest(4),
            exercise(5, 0),
            rest(5),
        ]);
        training_session.add_exercise(0, 2.into());
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                exercise(0, 2),
                rest(0),
                exercise(1, 0),
                exercise(0, 2),
                rest(1),
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
                exercise(4, 0),
                rest(4),
                exercise(5, 0),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_add_exercise_second() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
            exercise(4, 0),
            rest(4),
            exercise(5, 0),
            rest(5),
        ]);
        training_session.add_exercise(2, 2.into());
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(1, 0),
                rest(1),
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
                exercise(4, 0),
                exercise(0, 2),
                rest(4),
                exercise(5, 0),
                exercise(0, 2),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_add_exercise_superset_first() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 1),
            rest(0),
            exercise(2, 0),
            exercise(3, 1),
            rest(1),
            exercise(4, 0),
            exercise(5, 2),
            rest(2),
            exercise(6, 0),
            exercise(7, 2),
            rest(3),
            exercise(8, 1),
            exercise(9, 2),
            rest(4),
            exercise(10, 1),
            exercise(11, 2),
            rest(5),
        ]);
        training_session.add_exercise(0, 3.into());
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                exercise(1, 1),
                exercise(0, 3),
                rest(0),
                exercise(2, 0),
                exercise(3, 1),
                exercise(0, 3),
                rest(1),
                exercise(4, 0),
                exercise(5, 2),
                rest(2),
                exercise(6, 0),
                exercise(7, 2),
                rest(3),
                exercise(8, 1),
                exercise(9, 2),
                rest(4),
                exercise(10, 1),
                exercise(11, 2),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_add_exercise_superset_second() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 1),
            rest(0),
            exercise(2, 0),
            exercise(3, 1),
            rest(1),
            exercise(4, 0),
            exercise(5, 2),
            rest(2),
            exercise(6, 0),
            exercise(7, 2),
            rest(3),
            exercise(8, 1),
            exercise(9, 2),
            rest(4),
            exercise(10, 1),
            exercise(11, 2),
            rest(5),
        ]);
        training_session.add_exercise(1, 3.into());
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                exercise(1, 1),
                rest(0),
                exercise(2, 0),
                exercise(3, 1),
                rest(1),
                exercise(4, 0),
                exercise(5, 2),
                exercise(0, 3),
                rest(2),
                exercise(6, 0),
                exercise(7, 2),
                exercise(0, 3),
                rest(3),
                exercise(8, 1),
                exercise(9, 2),
                rest(4),
                exercise(10, 1),
                exercise(11, 2),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_add_exercise_no_rest() {
        let mut training_session = training_session(&[exercise(0, 0)]);
        training_session.add_exercise(0, 1.into());
        assert_eq!(
            training_session.elements,
            vec![exercise(0, 0), exercise(0, 1)]
        );
    }

    #[test]
    fn test_training_session_replace_exercise_first() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
            exercise(4, 0),
            rest(4),
            exercise(5, 0),
            rest(5),
        ]);
        training_session.replace_exercise(0, 0, 2.into());
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 2),
                rest(0),
                exercise(1, 2),
                rest(1),
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
                exercise(4, 0),
                rest(4),
                exercise(5, 0),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_replace_exercise_last() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
            exercise(4, 0),
            rest(4),
            exercise(5, 0),
            rest(5),
        ]);
        training_session.replace_exercise(2, 0, 2.into());
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(1, 0),
                rest(1),
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
                exercise(4, 2),
                rest(4),
                exercise(5, 2),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_replace_exercise_superset_first_exercise() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 1),
            rest(0),
            exercise(2, 0),
            exercise(3, 1),
            rest(1),
            exercise(4, 0),
            exercise(5, 2),
            rest(2),
            exercise(6, 0),
            exercise(7, 2),
            rest(3),
            exercise(8, 1),
            exercise(9, 2),
            rest(4),
            exercise(10, 1),
            exercise(11, 2),
            rest(5),
        ]);
        training_session.replace_exercise(0, 0, 3.into());
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 3),
                exercise(1, 1),
                rest(0),
                exercise(2, 3),
                exercise(3, 1),
                rest(1),
                exercise(4, 0),
                exercise(5, 2),
                rest(2),
                exercise(6, 0),
                exercise(7, 2),
                rest(3),
                exercise(8, 1),
                exercise(9, 2),
                rest(4),
                exercise(10, 1),
                exercise(11, 2),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_replace_exercise_dropsets() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 0),
            rest(0),
            exercise(2, 0),
            exercise(3, 0),
            rest(1),
            exercise(4, 0),
            exercise(5, 2),
            rest(2),
            exercise(6, 0),
            exercise(7, 2),
            rest(3),
        ]);
        training_session.replace_exercise(0, 0, 3.into());
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 3),
                exercise(1, 3),
                rest(0),
                exercise(2, 3),
                exercise(3, 3),
                rest(1),
                exercise(4, 0),
                exercise(5, 2),
                rest(2),
                exercise(6, 0),
                exercise(7, 2),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_training_session_replace_exercise_superset_second_exercise() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 1),
            rest(0),
            exercise(2, 0),
            exercise(3, 1),
            rest(1),
            exercise(4, 0),
            exercise(5, 2),
            rest(2),
            exercise(6, 0),
            exercise(7, 2),
            rest(3),
            exercise(8, 1),
            exercise(9, 2),
            rest(4),
            exercise(10, 1),
            exercise(11, 2),
            rest(5),
        ]);
        training_session.replace_exercise(1, 1, 3.into());
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                exercise(1, 1),
                rest(0),
                exercise(2, 0),
                exercise(3, 1),
                rest(1),
                exercise(4, 0),
                exercise(5, 3),
                rest(2),
                exercise(6, 0),
                exercise(7, 3),
                rest(3),
                exercise(8, 1),
                exercise(9, 2),
                rest(4),
                exercise(10, 1),
                exercise(11, 2),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_remove_set_first() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
        ]);
        training_session.remove_set(0);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_training_session_remove_set_last() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
        ]);
        training_session.remove_set(1);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(1, 0),
                rest(1),
                exercise(2, 1),
                rest(2),
            ]
        );
    }

    #[test]
    fn test_training_session_remove_set_superset() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 2),
            rest(0),
            exercise(2, 0),
            exercise(3, 2),
            rest(1),
        ]);
        training_session.remove_set(0);
        assert_eq!(
            training_session.elements,
            vec![exercise(0, 0), exercise(1, 2), rest(0)]
        );
    }

    #[test]
    fn test_training_session_remove_set_no_rest() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 0),
            exercise(2, 1),
            exercise(3, 1),
        ]);
        training_session.remove_set(0);
        assert_eq!(training_session.elements, vec![]);
    }

    #[test]
    fn test_training_session_remove_set_first_single_set() {
        let mut training_session = training_session(&[exercise(0, 0), rest(0), exercise(1, 1)]);
        training_session.remove_set(0);
        assert_eq!(training_session.elements, vec![exercise(1, 1)]);
    }

    #[test]
    fn test_training_session_remove_set_last_single_set() {
        let mut training_session = training_session(&[exercise(0, 0), rest(0), exercise(1, 1)]);
        training_session.remove_set(1);
        assert_eq!(training_session.elements, vec![exercise(0, 0), rest(0)]);
    }

    #[test]
    fn test_training_session_remove_exercise_first() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
            exercise(4, 0),
            rest(4),
            exercise(5, 0),
            rest(5),
        ]);
        training_session.remove_exercise(0, 0);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
                exercise(4, 0),
                rest(4),
                exercise(5, 0),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_remove_exercise_last() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            rest(0),
            exercise(1, 0),
            rest(1),
            exercise(2, 1),
            rest(2),
            exercise(3, 1),
            rest(3),
            exercise(4, 0),
            rest(4),
            exercise(5, 0),
            rest(5),
        ]);
        training_session.remove_exercise(2, 0);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                rest(0),
                exercise(1, 0),
                rest(1),
                exercise(2, 1),
                rest(2),
                exercise(3, 1),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_training_session_remove_exercise_superset_first_exercise() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 1),
            rest(0),
            exercise(2, 0),
            exercise(3, 1),
            rest(1),
            exercise(4, 0),
            exercise(5, 2),
            rest(2),
            exercise(6, 0),
            exercise(7, 2),
            rest(3),
            exercise(8, 1),
            exercise(9, 2),
            rest(4),
            exercise(10, 1),
            exercise(11, 2),
            rest(5),
        ]);
        training_session.remove_exercise(0, 0);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(1, 1),
                rest(0),
                exercise(3, 1),
                rest(1),
                exercise(4, 0),
                exercise(5, 2),
                rest(2),
                exercise(6, 0),
                exercise(7, 2),
                rest(3),
                exercise(8, 1),
                exercise(9, 2),
                rest(4),
                exercise(10, 1),
                exercise(11, 2),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_remove_exercise_dropsets() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 0),
            rest(0),
            exercise(2, 0),
            exercise(3, 0),
            rest(1),
        ]);
        training_session.remove_exercise(0, 0);
        assert_eq!(
            training_session.elements,
            vec![exercise(0, 0), rest(0), exercise(2, 0), rest(1),]
        );
    }

    #[test]
    fn test_training_session_remove_exercise_superset_second_exercise() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 1),
            rest(0),
            exercise(2, 0),
            exercise(3, 1),
            rest(1),
            exercise(4, 0),
            exercise(5, 2),
            rest(2),
            exercise(6, 0),
            exercise(7, 2),
            rest(3),
            exercise(8, 1),
            exercise(9, 2),
            rest(4),
            exercise(10, 1),
            exercise(11, 2),
            rest(5),
        ]);
        training_session.remove_exercise(1, 1);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                exercise(1, 1),
                rest(0),
                exercise(2, 0),
                exercise(3, 1),
                rest(1),
                exercise(4, 0),
                rest(2),
                exercise(6, 0),
                rest(3),
                exercise(8, 1),
                exercise(9, 2),
                rest(4),
                exercise(10, 1),
                exercise(11, 2),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_training_session_append_exercise_empty() {
        let mut training_session = training_session(&[]);
        training_session.append_exercise(1.into());
        assert_eq!(training_session.elements, vec![exercise(0, 1)]);
    }

    #[test]
    fn test_training_session_append_exercise_same() {
        let mut training_session = training_session(&[exercise(0, 1)]);
        training_session.append_exercise(1.into());
        assert_eq!(
            training_session.elements,
            vec![exercise(0, 1), rest(0), exercise(0, 1)]
        );
    }

    #[test]
    fn test_training_session_append_exercise_different() {
        let mut training_session = training_session(&[exercise(0, 1)]);
        training_session.append_exercise(2.into());
        assert_eq!(
            training_session.elements,
            vec![exercise(0, 1), rest(0), exercise(0, 2)]
        );
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
            target_time: if entry_id > 0 {
                Some(Time::new(entry_id).unwrap())
            } else {
                None
            },
            automatic: true,
        }
    }

    fn section(elements: &[TrainingSessionElement]) -> TrainingSessionSection {
        TrainingSessionSection(elements.to_vec())
    }
}
