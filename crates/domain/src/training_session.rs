use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    ops::RangeInclusive,
    str::FromStr,
};

use chrono::{Local, NaiveDate};
use derive_more::Deref;
use log::error;
use uuid::Uuid;

use crate::{
    CreateError, DeleteError, Exercise, ExerciseID, MuscleID, RPE, ReadError, Reps, RoutineID,
    Stimulus, SyncError, Tempo, Time, TrainingStats, UpdateError, ValidationError, Weight,
    one_rep_max, training::values_to_string, training_stats,
};

#[allow(async_fn_in_trait)]
pub trait TrainingSessionService {
    /// Returns all training sessions sorted by date in ascending order (oldest first).
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
        exercise_notes: Option<BTreeMap<ExerciseID, String>>,
    ) -> Result<TrainingSession, UpdateError>;
    async fn delete_training_session(&self, id: TrainingSessionID) -> Result<(), DeleteError>;

    fn validate_training_session_date(&self, date: &str) -> Result<NaiveDate, ValidationError> {
        match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            Ok(parsed_date) => {
                if parsed_date <= Local::now().date_naive() {
                    Ok(parsed_date)
                } else {
                    Err(ValidationError::Other(
                        "date must not be in the future".into(),
                    ))
                }
            }
            Err(_) => Err(ValidationError::Other("invalid date".into())),
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
            if let TrainingSessionElement::Set { exercise_id, .. } = element
                && !element.is_empty()
            {
                result.entry(*exercise_id).or_default().push(element);
            }
        }
        result
    }

    /// Returns the non-empty sets of up to `limit` earlier training sessions per exercise of the
    /// training session, together with the date of the session they belong to, ordered from most
    /// recent to oldest.
    ///
    /// Only training sessions for the same routine that occurred before the current one are taken
    /// into account. Sessions without a non-empty set for an exercise are skipped for that
    /// exercise, so the sets of an exercise that was left out of a session are still returned.
    fn get_recent_session_sets_by_exercise<'a>(
        &self,
        training_session: &TrainingSession,
        training_sessions: &'a [TrainingSession],
        limit: usize,
    ) -> HashMap<ExerciseID, Vec<(NaiveDate, Vec<&'a TrainingSessionElement>)>> {
        if limit == 0 {
            return HashMap::new();
        }

        let exercise_ids = training_session
            .elements
            .iter()
            .filter_map(|element| match element {
                TrainingSessionElement::Set { exercise_id, .. } => Some(*exercise_id),
                TrainingSessionElement::Rest { .. } => None,
            })
            .collect::<HashSet<_>>();

        let mut earlier_training_sessions = training_sessions
            .iter()
            .filter(|t| {
                t.id != training_session.id
                    && t.date < training_session.date
                    && t.routine_id == training_session.routine_id
            })
            .collect::<Vec<_>>();
        earlier_training_sessions.sort_by_key(|t| std::cmp::Reverse((t.date, t.id)));

        let mut result: HashMap<ExerciseID, Vec<(NaiveDate, Vec<&'a TrainingSessionElement>)>> =
            HashMap::new();
        let mut sets_by_exercise: HashMap<ExerciseID, Vec<&'a TrainingSessionElement>> =
            HashMap::new();
        for earlier_training_session in earlier_training_sessions {
            for element in &earlier_training_session.elements {
                if let TrainingSessionElement::Set { exercise_id, .. } = element
                    && !element.is_empty()
                    && exercise_ids.contains(exercise_id)
                    && result
                        .get(exercise_id)
                        .is_none_or(|sessions| sessions.len() < limit)
                {
                    sets_by_exercise
                        .entry(*exercise_id)
                        .or_default()
                        .push(element);
                }
            }
            for (exercise_id, sets) in sets_by_exercise.drain() {
                result
                    .entry(exercise_id)
                    .or_default()
                    .push((earlier_training_session.date, sets));
            }
            if exercise_ids.iter().all(|id| {
                result
                    .get(id)
                    .is_some_and(|sessions| sessions.len() >= limit)
            }) {
                break;
            }
        }
        result
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
        exercise_notes: Option<BTreeMap<ExerciseID, String>>,
    ) -> Result<TrainingSession, UpdateError>;
    async fn delete_training_session(&self, id: TrainingSessionID) -> Result<(), DeleteError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrainingSession {
    pub id: TrainingSessionID,
    pub routine_id: RoutineID,
    pub date: NaiveDate,
    pub notes: String,
    pub elements: Vec<TrainingSessionElement>,
    pub exercise_notes: BTreeMap<ExerciseID, String>,
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
                TrainingSessionElement::Set { reps, .. } => reps.non_zero(),
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
                TrainingSessionElement::Set { time, .. } => time.non_zero(),
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
                TrainingSessionElement::Set { weight, .. } => weight.non_zero(),
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
                TrainingSessionElement::Set { rpe, .. } => rpe.non_zero(),
                TrainingSessionElement::Rest { .. } => None,
            })
            .collect::<Vec<_>>();
        RPE::avg(sets)
    }

    #[must_use]
    pub fn estimated_max_reps(&self) -> Option<f32> {
        self.elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { reps, rpe, .. } => {
                    Some(reps.non_zero()?.including_rir(rpe.non_zero()?))
                }
                TrainingSessionElement::Rest { .. } => None,
            })
            .fold(None, |acc, v| Some(acc.map_or(v, |m: f32| m.max(v))))
    }

    #[must_use]
    pub fn load(&self) -> u32 {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set {
                    reps, time, rpe, ..
                } => Some(if let Some(rpe) = rpe.non_zero() {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    if rpe > RPE::FIVE {
                        (2.0_f32).powf(f32::from(rpe) - 5.0).round() as u32
                    } else {
                        1
                    }
                } else {
                    u32::from(reps.non_zero().is_some() || time.non_zero().is_some())
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
                    if rpe.non_zero().unwrap_or(RPE::TEN) >= RPE::SEVEN {
                        Some(u32::from(
                            reps.non_zero().is_some() || time.non_zero().is_some(),
                        ))
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
                    if let Some(reps) = reps.non_zero() {
                        #[allow(
                            clippy::cast_possible_truncation,
                            clippy::cast_precision_loss,
                            clippy::cast_sign_loss
                        )]
                        if let Some(weight) = weight.non_zero() {
                            Some((u32::from(reps) as f32 * f32::from(weight)).round() as u32)
                        } else {
                            Some(u32::from(reps))
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
                    .non_zero()
                    .map(|v| reps.non_zero().unwrap_or(Reps::new(1).unwrap()) * v),
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
    pub fn one_rep_max(&self, exercise_id: ExerciseID) -> Option<f32> {
        self.elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set {
                    exercise_id: set_exercise_id,
                    ..
                } if *set_exercise_id == exercise_id => e.one_rep_max(),
                TrainingSessionElement::Rest { .. } | TrainingSessionElement::Set { .. } => None,
            })
            .reduce(f32::max)
    }

    /// Returns the reps including RIR (i.e. the estimated reps to failure,
    /// rounded to the nearest integer) and weight of the set for
    /// `exercise_id` with the highest estimated 1RM in this session. Returns
    /// `None` if no set with both reps and weight exists for `exercise_id`.
    #[must_use]
    pub fn best_set_for_one_rep_max(&self, exercise_id: ExerciseID) -> Option<(Reps, Weight)> {
        self.elements
            .iter()
            .filter_map(|element| match element {
                TrainingSessionElement::Set {
                    exercise_id: id,
                    reps,
                    weight,
                    rpe,
                    ..
                } if *id == exercise_id => {
                    let reps = reps.non_zero()?;
                    let weight = weight.non_zero()?;
                    let one_rep_max = element.one_rep_max()?;
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let reps_including_rir = Reps::new(
                        reps.including_rir(rpe.non_zero().unwrap_or(RPE::TEN))
                            .round() as u32,
                    )
                    .ok()?;
                    Some((one_rep_max, reps_including_rir, weight))
                }
                TrainingSessionElement::Rest { .. } | TrainingSessionElement::Set { .. } => None,
            })
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, reps, weight)| (reps, weight))
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
                if reps.non_zero().is_none() && time.non_zero().is_none() {
                    continue;
                }
                if let Some(rpe) = rpe.non_zero()
                    && rpe < RPE::SEVEN
                {
                    continue;
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

    /// Returns `true` if every set in the session has recorded input.
    ///
    /// A session without sets is vacuously fully recorded.
    #[must_use]
    pub fn all_sets_recorded(&self) -> bool {
        self.elements
            .iter()
            .filter(|element| matches!(element, TrainingSessionElement::Set { .. }))
            .all(|element| !element.is_empty())
    }

    /// Retrieves the most recent distinct previous exercise notes for a given exercise.
    ///
    /// This method returns up to three distinct previous exercise notes for the specified
    /// exercise, sourced from past training sessions.
    ///
    /// # Key Behaviors
    ///
    /// - Session filtering: Only considers training sessions with `date < self.date`.
    /// - Empty notes skipped: Ignores sessions with empty notes.
    /// - Current note excluded: Filters out any notes matching the current exercise note.
    /// - Deduplication: Returns only distinct note texts (if the same note appears in
    ///   multiple sessions, only the most recent is included).
    /// - Ordering: Results are sorted by date in descending order (newest first).
    /// - Result limit: Returns at most 3 notes.
    #[must_use]
    pub fn previous_exercise_notes(
        &self,
        exercise_id: ExerciseID,
        training_sessions: &[TrainingSession],
    ) -> Vec<PreviousExerciseNote> {
        let current_note = self
            .exercise_notes
            .get(&exercise_id)
            .map_or("", |note| note.trim());
        let mut notes = training_sessions
            .iter()
            .filter(|session| session.id != self.id && session.date < self.date)
            .filter_map(|session| {
                let note = session.exercise_notes.get(&exercise_id)?.trim().to_string();
                if note.is_empty() {
                    return None;
                }
                Some(PreviousExerciseNote {
                    date: session.date,
                    routine_id: session.routine_id,
                    note,
                })
            })
            .collect::<Vec<_>>();
        notes.sort_by_key(|note| std::cmp::Reverse(note.date));

        let mut seen_notes = HashSet::new();
        notes
            .into_iter()
            .filter(|previous_note| previous_note.note != current_note)
            .filter(|previous_note| seen_notes.insert(previous_note.note.clone()))
            .take(3)
            .collect()
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
                target_time: Time::default(),
                automatic: true,
            }
        };
        let mut sets = vec![];
        for element in &section_elements {
            match element {
                TrainingSessionElement::Set {
                    exercise_id,
                    target_reps,
                    target_tempo,
                    target_weight,
                    target_rpe,
                    automatic,
                    ..
                } => {
                    sets.push(TrainingSessionElement::Set {
                        exercise_id: *exercise_id,
                        reps: Reps::default(),
                        time: Time::default(),
                        weight: Weight::default(),
                        rpe: RPE::default(),
                        target_reps: *target_reps,
                        target_tempo: *target_tempo,
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
                        reps: Reps::default(),
                        time: Time::default(),
                        weight: Weight::default(),
                        rpe: RPE::default(),
                        target_reps: Reps::default(),
                        target_tempo: Tempo::default(),
                        target_weight: Weight::default(),
                        target_rpe: RPE::default(),
                        automatic: false,
                    });
                    elements.push(element);
                }
            }
        }

        if let Some(TrainingSessionElement::Set { .. }) = section.elements().last() {
            elements.push(TrainingSessionElement::Set {
                exercise_id,
                reps: Reps::default(),
                time: Time::default(),
                weight: Weight::default(),
                rpe: RPE::default(),
                target_reps: Reps::default(),
                target_tempo: Tempo::default(),
                target_weight: Weight::default(),
                target_rpe: RPE::default(),
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
                    target_tempo,
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
                            target_tempo: *target_tempo,
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
        let ids = section.exercise_ids();
        if ids.len() > 1 {
            let id = ids[exercise_idx];
            let closes_round = ids.last() == Some(&id);
            for run in section
                .elements()
                .split_inclusive(|element| matches!(element, TrainingSessionElement::Rest { .. }))
            {
                let removed_idx = run.iter().rposition(|element| {
                    matches!(element, TrainingSessionElement::Set { exercise_id, .. } if *exercise_id == id)
                });
                let remaining = run
                    .iter()
                    .enumerate()
                    .filter(|(idx, _)| Some(*idx) != removed_idx)
                    .map(|(_, element)| element.clone())
                    .collect::<Vec<_>>();
                if remaining
                    .iter()
                    .any(|element| matches!(element, TrainingSessionElement::Set { .. }))
                {
                    elements.extend(remaining);
                    continue;
                }
                // Of the rests around an emptied run, the one closing the round is kept.
                if closes_round {
                    if matches!(elements.last(), Some(TrainingSessionElement::Rest { .. })) {
                        elements.pop();
                    }
                    elements.extend(remaining);
                }
            }
        }

        self.replace_elements_of_section(&sections, section_idx, elements);
        self.ensure_sections_contain_set("removing exercise");
    }

    pub fn append_exercise(&mut self, exercise_id: ExerciseID) {
        if let Some(TrainingSessionElement::Set { .. }) = self.elements.last() {
            self.elements.push(TrainingSessionElement::Rest {
                target_time: Time::default(),
                automatic: true,
            });
        }
        self.elements.push(TrainingSessionElement::Set {
            exercise_id,
            reps: Reps::default(),
            time: Time::default(),
            weight: Weight::default(),
            rpe: RPE::default(),
            target_reps: Reps::default(),
            target_tempo: Tempo::default(),
            target_weight: Weight::default(),
            target_rpe: RPE::default(),
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
        if section.end() + 1 == self.elements.len()
            && let Some(TrainingSessionElement::Set { .. }) = self.elements.last()
        {
            self.elements.push(TrainingSessionElement::Rest {
                target_time: Time::default(),
                automatic: true,
            });
            trailing_rest += 1;
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
        if section.start() + section_len + subsequent_section_len == self.elements.len()
            && let Some(TrainingSessionElement::Set { .. }) = self.elements.last()
        {
            self.elements.push(TrainingSessionElement::Rest {
                target_time: Time::default(),
                automatic: true,
            });
            trailing_rest += 1;
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
    pub fn section_idx(&self, element_idx: usize) -> usize {
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

    /// Like [`Self::section_idx`], but the last element of a section maps to the *next* section.
    /// Returns the section count once `element_idx` is at or past the final element.
    #[must_use]
    pub fn section_idx_lookahead(&self, element_idx: usize) -> usize {
        self.section_idx(element_idx.saturating_add(1))
    }

    #[must_use]
    fn section_count(&self) -> usize {
        let mut count = 0;
        let mut idx = 0;

        while idx < self.elements.len() {
            idx = Self::find_last_of_section(self, idx) + 1;
            count += 1;
        }

        count
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

    /// Returns the exercise at the given position of the given computed section, if any.
    #[must_use]
    pub fn exercise_id_at(&self, section_idx: usize, exercise_idx: usize) -> Option<ExerciseID> {
        self.compute_sections()
            .get(section_idx)
            .and_then(|section| section.exercise_ids().get(exercise_idx).copied())
    }

    /// Returns the indices of the elements of `exercise_id` in one run of consecutive sets of the
    /// given section, in order.
    ///
    /// A run is a maximal sequence of sets not interrupted by a rest. The run containing
    /// `element_idx` is returned, a rest counting towards the run following it. If `element_idx`
    /// lies outside the runs of the section, the first run is returned.
    #[must_use]
    pub fn run_element_indices(
        &self,
        section_idx: usize,
        exercise_id: ExerciseID,
        element_idx: usize,
    ) -> Vec<usize> {
        if section_idx >= self.section_count() {
            return vec![];
        }

        let mut runs: Vec<Vec<usize>> = vec![];
        let mut run = vec![];
        for idx in self.section_range(section_idx) {
            match self.elements[idx] {
                TrainingSessionElement::Set { .. } => run.push(idx),
                TrainingSessionElement::Rest { .. } => {
                    if !run.is_empty() {
                        runs.push(std::mem::take(&mut run));
                    }
                }
            }
        }
        if !run.is_empty() {
            runs.push(run);
        }

        let run = runs
            .iter()
            .find(|run| run.last().is_some_and(|last| *last >= element_idx))
            .or_else(|| runs.first());

        run.map(|run| {
            run.iter()
                .copied()
                .filter(|idx| {
                    matches!(
                        self.elements[*idx],
                        TrainingSessionElement::Set { exercise_id: id, .. } if id == exercise_id
                    )
                })
                .collect()
        })
        .unwrap_or_default()
    }

    /// For every set, its position among the sets of the same exercise, keyed by the index of the
    /// element.
    #[must_use]
    pub fn set_indices(&self) -> HashMap<usize, usize> {
        let mut counts: HashMap<ExerciseID, usize> = HashMap::new();
        self.elements
            .iter()
            .enumerate()
            .filter_map(|(element_idx, element)| match element {
                TrainingSessionElement::Set { exercise_id, .. } => {
                    let count = counts.entry(*exercise_id).or_default();
                    let set_index = *count;
                    *count += 1;
                    Some((element_idx, set_index))
                }
                TrainingSessionElement::Rest { .. } => None,
            })
            .collect()
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

        if last + 1 < self.elements.len()
            && let TrainingSessionElement::Rest { .. } = &self.elements[last + 1]
        {
            last += 1;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviousExerciseNote {
    pub date: NaiveDate,
    pub routine_id: RoutineID,
    pub note: String,
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
        reps: Reps,
        time: Time,
        weight: Weight,
        rpe: RPE,
        target_reps: Reps,
        target_tempo: Tempo,
        target_weight: Weight,
        target_rpe: RPE,
        automatic: bool,
    },
    Rest {
        target_time: Time,
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
                *reps == Reps::default()
                    && *time == Time::default()
                    && *weight == Weight::default()
                    && *rpe == RPE::default()
            }
            TrainingSessionElement::Rest { .. } => true,
        }
    }

    #[must_use]
    pub fn set(&self) -> Option<Set> {
        match self {
            TrainingSessionElement::Set {
                reps,
                time,
                weight,
                rpe,
                ..
            } => Some(Set {
                reps: *reps,
                time: *time,
                weight: *weight,
                rpe: *rpe,
            }),
            TrainingSessionElement::Rest { .. } => None,
        }
    }

    #[must_use]
    pub fn one_rep_max(&self) -> Option<f32> {
        match self {
            TrainingSessionElement::Set {
                reps, weight, rpe, ..
            } => {
                let reps = reps.non_zero()?;
                let weight = weight.non_zero()?;
                Some(one_rep_max(
                    reps.including_rir(rpe.non_zero().unwrap_or(RPE::TEN)),
                    f32::from(weight),
                ))
            }
            TrainingSessionElement::Rest { .. } => None,
        }
    }

    #[must_use]
    pub fn to_string(&self, show_tut: bool, show_rpe: bool) -> String {
        self.set()
            .map(|set| set.to_string(show_tut, show_rpe))
            .unwrap_or_default()
    }

    #[must_use]
    pub fn target_to_string(&self, show_rpe: bool) -> String {
        match self {
            TrainingSessionElement::Set {
                target_reps,
                target_tempo,
                target_weight,
                target_rpe,
                ..
            } => values_to_string(
                target_reps.non_zero(),
                target_weight.non_zero(),
                if show_rpe {
                    target_rpe.non_zero()
                } else {
                    None
                },
                &target_tempo.to_string(),
            ),
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
        values_to_string(
            self.reps.non_zero(),
            self.weight.non_zero(),
            if show_rpe { self.rpe.non_zero() } else { None },
            &match self.time.non_zero() {
                Some(time) if show_tut => format!("{time} s"),
                _ => String::new(),
            },
        )
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

/// Returns the reps including RIR and weight of the best set (by estimated
/// 1RM) for `exercise_id` in the most recent training session that contains
/// a completed set for that exercise. Sessions are compared by date,
/// breaking ties by ID.
#[must_use]
pub fn most_recent_best_set_for_one_rep_max(
    sessions: &[TrainingSession],
    exercise_id: ExerciseID,
) -> Option<(Reps, Weight)> {
    sessions
        .iter()
        .filter(|s| s.one_rep_max(exercise_id).is_some())
        .max_by_key(|s| (s.date, s.id))
        .and_then(|s| s.best_set_for_one_rep_max(exercise_id))
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
    use proptest::prelude::*;
    use rstest::rstest;

    use crate::{ExerciseMuscle, Name, Service, tests::FakeRepository};

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
                    reps: Reps::new(10).unwrap(),
                    time: Time::new(3).unwrap(),
                    weight: Weight::new(30.0).unwrap(),
                    rpe: RPE::EIGHT,
                    target_reps: Reps::new(8).unwrap(),
                    target_tempo: Tempo::new(&[4]).unwrap(),
                    target_weight: Weight::new(40.0).unwrap(),
                    target_rpe: RPE::NINE,
                    automatic: false,
                },
                TrainingSessionElement::Rest {
                    target_time: Time::new(60).unwrap(),
                    automatic: true,
                },
                TrainingSessionElement::Set {
                    exercise_id: 2.into(),
                    reps: Reps::new(5).unwrap(),
                    time: Time::new(4).unwrap(),
                    weight: Weight::default(),
                    rpe: RPE::FOUR,
                    target_reps: Reps::default(),
                    target_tempo: Tempo::default(),
                    target_weight: Weight::default(),
                    target_rpe: RPE::default(),
                    automatic: false,
                },
                TrainingSessionElement::Rest {
                    target_time: Time::new(60).unwrap(),
                    automatic: true,
                },
                TrainingSessionElement::Set {
                    exercise_id: 2.into(),
                    reps: Reps::default(),
                    time: Time::new(60).unwrap(),
                    weight: Weight::default(),
                    rpe: RPE::default(),
                    target_reps: Reps::default(),
                    target_tempo: Tempo::default(),
                    target_weight: Weight::default(),
                    target_rpe: RPE::default(),
                    automatic: false,
                },
                TrainingSessionElement::Rest {
                    target_time: Time::new(60).unwrap(),
                    automatic: true,
                },
            ],
            exercise_notes: BTreeMap::new(),
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
                        target_tempo,
                        target_weight,
                        target_rpe,
                        automatic,
                        ..
                    } => TrainingSessionElement::Set {
                        exercise_id: *exercise_id,
                        reps: Reps::default(),
                        time: Time::default(),
                        weight: Weight::default(),
                        rpe: RPE::default(),
                        target_reps: *target_reps,
                        target_tempo: *target_tempo,
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
    #[case(&training_session(&[set(1, 5, 30.0, RPE::ZERO), set(1, 5, 50.0, RPE::ZERO)]), Some(40.0))]
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

    #[test]
    fn test_training_session_avg_rpe_excludes_unset_rpe() {
        let training_session =
            training_session(&[set(1, 5, 100.0, RPE::ZERO), set(1, 5, 100.0, RPE::EIGHT)]);
        assert_eq!(training_session.avg_rpe(), Some(RPE::EIGHT));
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, Some(12.0))]
    #[case(&*EMPTY_TRAINING_SESSION, None)]
    fn test_training_session_estimated_max_reps(
        #[case] training_session: &TrainingSession,
        #[case] expected: Option<f32>,
    ) {
        assert_eq!(training_session.estimated_max_reps(), expected);
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

    #[test]
    fn test_training_session_set_volume_treats_unset_rpe_as_maximum() {
        let training_session =
            training_session(&[set(1, 5, 100.0, RPE::ZERO), set(1, 5, 100.0, RPE::FOUR)]);
        assert_eq!(training_session.set_volume(), 1);
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
    #[case(&*TRAINING_SESSION, Some(40.8))]
    #[case(&*EMPTY_TRAINING_SESSION, None)]
    fn test_training_session_one_rep_max(
        #[case] training_session: &TrainingSession,
        #[case] expected: Option<f32>,
    ) {
        assert_eq!(
            training_session
                .one_rep_max(1.into())
                .map(|v| (v * 10.0).round() / 10.0),
            expected
        );
    }

    #[test]
    fn test_training_session_one_rep_max_not_present() {
        assert_eq!(TRAINING_SESSION.one_rep_max(99.into()), None);
    }

    #[test]
    fn test_training_session_one_rep_max_no_rpe() {
        let training_session = TrainingSession {
            id: 1.into(),
            routine_id: 0.into(),
            date: *TODAY,
            notes: String::new(),
            elements: vec![TrainingSessionElement::Set {
                exercise_id: 1.into(),
                reps: Reps::new(30).unwrap(),
                time: Time::default(),
                weight: Weight::new(100.0).unwrap(),
                rpe: RPE::default(),
                target_reps: Reps::default(),
                target_tempo: Tempo::default(),
                target_weight: Weight::default(),
                target_rpe: RPE::default(),
                automatic: false,
            }],
            exercise_notes: BTreeMap::new(),
        };
        assert_eq!(
            training_session
                .one_rep_max(1.into())
                .map(|v| (v * 10.0).round() / 10.0),
            Some(183.6)
        );
    }

    #[test]
    fn test_training_session_one_rep_max_picks_best_set() {
        let training_session = TrainingSession {
            id: 1.into(),
            routine_id: 0.into(),
            date: *TODAY,
            notes: String::new(),
            elements: vec![
                TrainingSessionElement::Set {
                    exercise_id: 1.into(),
                    reps: Reps::new(10).unwrap(),
                    time: Time::default(),
                    weight: Weight::new(100.0).unwrap(),
                    rpe: RPE::default(),
                    target_reps: Reps::default(),
                    target_tempo: Tempo::default(),
                    target_weight: Weight::default(),
                    target_rpe: RPE::default(),
                    automatic: false,
                },
                TrainingSessionElement::Set {
                    exercise_id: 1.into(),
                    reps: Reps::new(8).unwrap(),
                    time: Time::default(),
                    weight: Weight::new(105.0).unwrap(),
                    rpe: RPE::default(),
                    target_reps: Reps::default(),
                    target_tempo: Tempo::default(),
                    target_weight: Weight::default(),
                    target_rpe: RPE::default(),
                    automatic: false,
                },
            ],
            exercise_notes: BTreeMap::new(),
        };
        assert_eq!(
            training_session
                .one_rep_max(1.into())
                .map(|v| (v * 10.0).round() / 10.0),
            Some(129.2)
        );
    }

    #[test]
    fn test_training_session_best_set_for_one_rep_max_picks_highest_estimated() {
        let training_session =
            training_session(&[set(1, 10, 100.0, RPE::ZERO), set(1, 8, 105.0, RPE::ZERO)]);
        assert_eq!(
            training_session.best_set_for_one_rep_max(1.into()),
            Some((Reps::new(10).unwrap(), Weight::new(100.0).unwrap()))
        );
    }

    #[test]
    fn test_training_session_best_set_for_one_rep_max_includes_rir_in_reps() {
        let training_session = training_session(&[set(1, 5, 100.0, RPE::EIGHT)]);
        assert_eq!(
            training_session.best_set_for_one_rep_max(1.into()),
            Some((Reps::new(7).unwrap(), Weight::new(100.0).unwrap()))
        );
    }

    #[test]
    fn test_training_session_best_set_for_one_rep_max_ignores_other_exercises() {
        let training_session =
            training_session(&[set(2, 20, 200.0, RPE::ZERO), set(1, 5, 100.0, RPE::ZERO)]);
        assert_eq!(
            training_session.best_set_for_one_rep_max(1.into()),
            Some((Reps::new(5).unwrap(), Weight::new(100.0).unwrap()))
        );
    }

    #[test]
    fn test_training_session_best_set_for_one_rep_max_ignores_incomplete_sets() {
        let training_session =
            training_session(&[set(1, 0, 100.0, RPE::ZERO), set(1, 5, 0.0, RPE::ZERO)]);
        assert_eq!(training_session.best_set_for_one_rep_max(1.into()), None);
    }

    #[test]
    fn test_training_session_best_set_for_one_rep_max_no_set_for_exercise() {
        let training_session = training_session(&[set(2, 5, 100.0, RPE::ZERO)]);
        assert_eq!(training_session.best_set_for_one_rep_max(1.into()), None);
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
            notes: String::new(),
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
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        }];
        assert_eq!(training_session.stimulus_per_muscle(&exercises), expected);
    }

    #[rstest]
    #[case(RPE::ZERO, BTreeMap::from([(MuscleID::Pecs, Stimulus::PRIMARY)]))]
    #[case(RPE::SEVEN, BTreeMap::from([(MuscleID::Pecs, Stimulus::PRIMARY)]))]
    #[case(RPE::SIX, BTreeMap::new())]
    #[case(RPE::FOUR, BTreeMap::new())]
    fn test_training_session_stimulus_per_muscle_with_unset_rpe(
        #[case] rpe: RPE,
        #[case] expected: BTreeMap<MuscleID, Stimulus>,
    ) {
        let training_session = training_session(&[set(1, 5, 100.0, rpe)]);
        let exercises = [Exercise {
            id: 1.into(),
            name: Name::new("A").unwrap(),
            notes: String::new(),
            muscles: vec![ExerciseMuscle {
                muscle_id: MuscleID::Pecs,
                stimulus: Stimulus::PRIMARY,
            }],
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        }];
        assert_eq!(training_session.stimulus_per_muscle(&exercises), expected);
    }

    #[rstest]
    #[case(set(1, 0, 0.0, RPE::ZERO), true)]
    #[case(set(1, 0, 0.0, RPE::FOUR), false)]
    #[case(set(1, 5, 0.0, RPE::ZERO), false)]
    fn test_training_session_element_is_empty(
        #[case] element: TrainingSessionElement,
        #[case] expected: bool,
    ) {
        assert_eq!(element.is_empty(), expected);
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

    #[rstest]
    #[case(0, 0, Some(ExerciseID::from(1_u128)))]
    #[case(0, 1, Some(ExerciseID::from(2_u128)))]
    #[case(1, 0, Some(ExerciseID::from(3_u128)))]
    #[case(0, 2, None)]
    #[case(2, 0, None)]
    fn test_training_session_exercise_id_at(
        #[case] section_idx: usize,
        #[case] exercise_idx: usize,
        #[case] expected: Option<ExerciseID>,
    ) {
        let training_session =
            training_session(&[exercise(0, 1), exercise(1, 2), rest(0), exercise(2, 3)]);

        assert_eq!(
            training_session.exercise_id_at(section_idx, exercise_idx),
            expected
        );
    }

    #[test]
    fn test_training_session_compute_sections_empty() {
        assert_eq!(training_session(&[]).compute_sections(), vec![]);
    }

    #[test]
    fn test_training_session_set_indices_number_the_sets_of_each_exercise_separately() {
        let session = training_session(&[
            exercise(0, 0),
            exercise(1, 1),
            rest(1),
            exercise(2, 0),
            exercise(3, 1),
        ]);

        assert_eq!(
            session.set_indices(),
            HashMap::from([(0, 0), (1, 0), (3, 1), (4, 1)])
        );
    }

    #[test]
    fn test_training_session_set_indices_of_a_session_without_sets_are_empty() {
        assert_eq!(training_session(&[rest(0)]).set_indices(), HashMap::new());
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

    #[test]
    fn test_training_session_previous_exercise_notes_returns_distinct_notes_ordered_newest_first_limited_to_three()
     {
        let exercise_id = ExerciseID::from(1_u128);
        let current = training_session_with_exercise_notes(
            10,
            1,
            NaiveDate::from_ymd_opt(2026, 5, 16).unwrap(),
            &[(exercise_id, "current")],
        );
        let sessions = vec![
            current.clone(),
            training_session_with_exercise_notes(
                9,
                1,
                NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
                &[(exercise_id, "note 1")],
            ),
            training_session_with_exercise_notes(
                8,
                2,
                NaiveDate::from_ymd_opt(2026, 5, 14).unwrap(),
                &[(exercise_id, "note 2")],
            ),
            training_session_with_exercise_notes(
                7,
                3,
                NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(),
                &[(exercise_id, "note 3")],
            ),
            training_session_with_exercise_notes(
                6,
                2,
                NaiveDate::from_ymd_opt(2026, 5, 12).unwrap(),
                &[(exercise_id, "note 4")],
            ),
        ];

        let result = current.previous_exercise_notes(exercise_id, &sessions);

        assert_eq!(
            result,
            vec![
                PreviousExerciseNote {
                    date: NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
                    routine_id: RoutineID::from(1_u128),
                    note: "note 1".to_string(),
                },
                PreviousExerciseNote {
                    date: NaiveDate::from_ymd_opt(2026, 5, 14).unwrap(),
                    routine_id: RoutineID::from(2_u128),
                    note: "note 2".to_string(),
                },
                PreviousExerciseNote {
                    date: NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(),
                    routine_id: RoutineID::from(3_u128),
                    note: "note 3".to_string(),
                },
            ],
        );
    }

    #[test]
    fn test_training_session_previous_exercise_notes_deduplicates_keeping_most_recent() {
        let exercise_id = ExerciseID::from(1_u128);
        let current = training_session_with_exercise_notes(
            10,
            1,
            NaiveDate::from_ymd_opt(2026, 5, 16).unwrap(),
            &[],
        );
        let sessions = vec![
            current.clone(),
            training_session_with_exercise_notes(
                9,
                1,
                NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
                &[(exercise_id, "repeat")],
            ),
            training_session_with_exercise_notes(
                8,
                2,
                NaiveDate::from_ymd_opt(2026, 5, 14).unwrap(),
                &[(exercise_id, "other")],
            ),
            training_session_with_exercise_notes(
                7,
                3,
                NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(),
                &[(exercise_id, "repeat")],
            ),
        ];

        let result = current.previous_exercise_notes(exercise_id, &sessions);

        assert_eq!(
            result
                .into_iter()
                .map(|n| (n.date, n.note))
                .collect::<Vec<_>>(),
            vec![
                (
                    NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
                    "repeat".to_string(),
                ),
                (
                    NaiveDate::from_ymd_opt(2026, 5, 14).unwrap(),
                    "other".to_string(),
                ),
            ],
        );
    }

    #[test]
    fn test_training_session_previous_exercise_notes_excludes_current_note_after_trimming() {
        let exercise_id = ExerciseID::from(1_u128);
        let current = training_session_with_exercise_notes(
            10,
            1,
            NaiveDate::from_ymd_opt(2026, 5, 16).unwrap(),
            &[(exercise_id, "  shared  ")],
        );
        let sessions = vec![
            current.clone(),
            training_session_with_exercise_notes(
                9,
                1,
                NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
                &[(exercise_id, "shared")],
            ),
            training_session_with_exercise_notes(
                8,
                2,
                NaiveDate::from_ymd_opt(2026, 5, 14).unwrap(),
                &[(exercise_id, "other")],
            ),
        ];

        let result = current.previous_exercise_notes(exercise_id, &sessions);

        assert_eq!(
            result.into_iter().map(|n| n.note).collect::<Vec<_>>(),
            vec!["other".to_string()],
        );
    }

    #[test]
    fn test_training_session_previous_exercise_notes_excludes_self_same_date_and_future_sessions() {
        let exercise_id = ExerciseID::from(1_u128);
        let current = training_session_with_exercise_notes(
            10,
            1,
            NaiveDate::from_ymd_opt(2026, 5, 16).unwrap(),
            &[(exercise_id, "current")],
        );
        let sessions = vec![
            current.clone(),
            training_session_with_exercise_notes(
                11,
                1,
                NaiveDate::from_ymd_opt(2026, 5, 17).unwrap(),
                &[(exercise_id, "future")],
            ),
            training_session_with_exercise_notes(
                12,
                1,
                NaiveDate::from_ymd_opt(2026, 5, 16).unwrap(),
                &[(exercise_id, "same day")],
            ),
            training_session_with_exercise_notes(
                9,
                1,
                NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
                &[(exercise_id, "past")],
            ),
        ];

        let result = current.previous_exercise_notes(exercise_id, &sessions);

        assert_eq!(
            result.into_iter().map(|n| n.note).collect::<Vec<_>>(),
            vec!["past".to_string()],
        );
    }

    #[test]
    fn test_training_session_previous_exercise_notes_skips_sessions_whose_note_is_empty_after_trimming()
     {
        let exercise_id = ExerciseID::from(1_u128);
        let current = training_session_with_exercise_notes(
            10,
            1,
            NaiveDate::from_ymd_opt(2026, 5, 16).unwrap(),
            &[],
        );
        let sessions = vec![
            current.clone(),
            training_session_with_exercise_notes(
                9,
                1,
                NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
                &[(exercise_id, "")],
            ),
            training_session_with_exercise_notes(
                8,
                2,
                NaiveDate::from_ymd_opt(2026, 5, 14).unwrap(),
                &[(exercise_id, "   ")],
            ),
            training_session_with_exercise_notes(
                7,
                3,
                NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(),
                &[(exercise_id, "kept")],
            ),
        ];

        let result = current.previous_exercise_notes(exercise_id, &sessions);

        assert_eq!(
            result.into_iter().map(|n| n.note).collect::<Vec<_>>(),
            vec!["kept".to_string()],
        );
    }

    #[test]
    fn test_training_session_previous_exercise_notes_ignores_other_exercises() {
        let exercise_id = ExerciseID::from(1_u128);
        let other_exercise = ExerciseID::from(2_u128);
        let current = training_session_with_exercise_notes(
            10,
            1,
            NaiveDate::from_ymd_opt(2026, 5, 16).unwrap(),
            &[],
        );
        let sessions = vec![
            current.clone(),
            training_session_with_exercise_notes(
                9,
                1,
                NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
                &[(other_exercise, "other exercise")],
            ),
            training_session_with_exercise_notes(
                8,
                2,
                NaiveDate::from_ymd_opt(2026, 5, 14).unwrap(),
                &[(exercise_id, "matching")],
            ),
        ];

        let result = current.previous_exercise_notes(exercise_id, &sessions);

        assert_eq!(
            result.into_iter().map(|n| n.note).collect::<Vec<_>>(),
            vec!["matching".to_string()],
        );
    }

    #[test]
    fn test_most_recent_best_set_for_one_rep_max_picks_latest_session_by_date() {
        let mut older = training_session(&[set(1, 3, 150.0, RPE::ZERO)]);
        older.date = *TODAY - Duration::days(7);
        let mut newer = training_session(&[set(1, 8, 100.0, RPE::ZERO)]);
        newer.date = *TODAY - Duration::days(1);
        assert_eq!(
            most_recent_best_set_for_one_rep_max(&[older, newer], 1.into()),
            Some((Reps::new(8).unwrap(), Weight::new(100.0).unwrap()))
        );
    }

    #[test]
    fn test_most_recent_best_set_for_one_rep_max_breaks_ties_by_id() {
        let a = training_session(&[set(1, 5, 100.0, RPE::ZERO)]);
        let mut b = training_session(&[set(1, 3, 120.0, RPE::ZERO)]);
        b.id = 2.into();
        assert_eq!(
            most_recent_best_set_for_one_rep_max(&[a, b], 1.into()),
            Some((Reps::new(3).unwrap(), Weight::new(120.0).unwrap()))
        );
    }

    #[test]
    fn test_most_recent_best_set_for_one_rep_max_skips_sessions_without_exercise() {
        let mut with_other_exercise = training_session(&[set(2, 10, 200.0, RPE::ZERO)]);
        with_other_exercise.date = *TODAY;
        let with_target_exercise = training_session(&[set(1, 5, 100.0, RPE::ZERO)]);
        assert_eq!(
            most_recent_best_set_for_one_rep_max(
                &[with_other_exercise, with_target_exercise],
                1.into()
            ),
            Some((Reps::new(5).unwrap(), Weight::new(100.0).unwrap()))
        );
    }

    #[test]
    fn test_most_recent_best_set_for_one_rep_max_empty() {
        assert_eq!(most_recent_best_set_for_one_rep_max(&[], 1.into()), None);
    }

    #[test]
    fn test_section_idx_lookahead_empty() {
        let ts = training_session(&[]);
        assert_eq!(ts.section_idx_lookahead(0), 0);
        assert_eq!(ts.section_idx_lookahead(7), 0);
    }

    #[test]
    fn test_section_idx_lookahead_matches_section_idx_within_section() {
        let ts = training_session(&[
            set(1, 5, 100.0, RPE::ZERO),
            set(1, 5, 100.0, RPE::ZERO),
            rest(60),
            set(2, 8, 50.0, RPE::ZERO),
            set(2, 8, 50.0, RPE::ZERO),
        ]);
        assert_eq!(ts.section_idx_lookahead(0), 0);
        assert_eq!(ts.section_idx_lookahead(3), 1);
    }

    #[test]
    fn test_section_idx_lookahead_at_section_boundary_advances() {
        let ts = training_session(&[
            set(1, 5, 100.0, RPE::ZERO),
            set(1, 5, 100.0, RPE::ZERO),
            rest(60),
            set(2, 8, 50.0, RPE::ZERO),
            set(2, 8, 50.0, RPE::ZERO),
        ]);
        assert_eq!(ts.section_idx(2), 0);
        assert_eq!(ts.section_idx_lookahead(2), 1);
    }

    #[test]
    fn test_section_idx_lookahead_past_end_returns_section_count() {
        let ts = training_session(&[
            set(1, 5, 100.0, RPE::ZERO),
            rest(60),
            set(2, 8, 50.0, RPE::ZERO),
        ]);
        assert_eq!(ts.section_idx_lookahead(2), 2);
        assert_eq!(ts.section_idx_lookahead(99), 2);
    }

    #[rstest]
    #[case::first_round(0, 1, 0, vec![0])]
    #[case::rest_belongs_to_the_following_round(0, 1, 1, vec![2])]
    #[case::second_round(0, 1, 2, vec![2])]
    #[case::trailing_rest_falls_back_to_the_first_round(0, 1, 3, vec![0])]
    #[case::before_the_section_falls_back_to_the_first_round(1, 2, 0, vec![4])]
    #[case::after_the_section_falls_back_to_the_first_round(0, 1, 99, vec![0])]
    fn test_training_session_run_element_indices(
        #[case] section_idx: usize,
        #[case] exercise_id: u128,
        #[case] element_idx: usize,
        #[case] expected: Vec<usize>,
    ) {
        let ts = training_session(&[
            exercise(0, 1),
            rest(0),
            exercise(1, 1),
            rest(1),
            exercise(2, 2),
        ]);

        assert_eq!(
            ts.run_element_indices(section_idx, exercise_id.into(), element_idx),
            expected
        );
    }

    #[rstest]
    #[case::first_exercise(1, 3, vec![3])]
    #[case::second_exercise(2, 0, vec![1])]
    fn test_training_session_run_element_indices_of_a_superset(
        #[case] exercise_id: u128,
        #[case] element_idx: usize,
        #[case] expected: Vec<usize>,
    ) {
        let ts = training_session(&[
            exercise(0, 1),
            exercise(1, 2),
            rest(0),
            exercise(2, 1),
            exercise(3, 2),
        ]);

        assert_eq!(
            ts.run_element_indices(0, exercise_id.into(), element_idx),
            expected
        );
    }

    #[test]
    fn test_training_session_run_element_indices_skips_a_leading_rest() {
        let ts = training_session(&[rest(0), exercise(1, 1), rest(1), exercise(2, 1)]);

        assert_eq!(ts.run_element_indices(0, 1.into(), 0), Vec::<usize>::new());
        assert_eq!(ts.run_element_indices(1, 1.into(), 1), vec![1]);
        assert_eq!(ts.run_element_indices(1, 1.into(), 2), vec![3]);
    }

    #[test]
    fn test_training_session_run_element_indices_beyond_the_last_section() {
        let ts = training_session(&[exercise(0, 1)]);

        assert_eq!(ts.run_element_indices(1, 1.into(), 0), Vec::<usize>::new());
    }

    #[test]
    fn test_training_session_all_sets_recorded() {
        assert!(training_session(&[set(1, 5, 100.0, RPE::ZERO), rest(60)]).all_sets_recorded());
        assert!(
            !training_session(&[set(1, 5, 100.0, RPE::ZERO), set(2, 0, 0.0, RPE::ZERO)])
                .all_sets_recorded()
        );
        assert!(training_session(&[rest(60)]).all_sets_recorded());
        assert!(training_session(&[]).all_sets_recorded());
    }

    fn training_session(elements: &[TrainingSessionElement]) -> TrainingSession {
        TrainingSession {
            id: 1.into(),
            routine_id: 0.into(),
            date: *TODAY - Duration::days(10),
            notes: String::new(),
            elements: elements.to_vec(),
            exercise_notes: BTreeMap::new(),
        }
    }

    fn training_session_with_exercise_notes(
        id: u128,
        routine_id: u128,
        date: NaiveDate,
        exercise_notes: &[(ExerciseID, &str)],
    ) -> TrainingSession {
        TrainingSession {
            id: TrainingSessionID::from(id),
            routine_id: RoutineID::from(routine_id),
            date,
            notes: String::new(),
            elements: vec![],
            exercise_notes: exercise_notes
                .iter()
                .map(|(id, n)| (*id, (*n).to_string()))
                .collect(),
        }
    }

    fn exercise(entry_id: u32, exercise_id: u128) -> TrainingSessionElement {
        TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            reps: Reps::default(),
            time: Time::default(),
            weight: Weight::default(),
            rpe: RPE::default(),
            target_reps: Reps::new(entry_id).unwrap(),
            target_tempo: Tempo::default(),
            target_weight: Weight::default(),
            target_rpe: RPE::default(),
            automatic: false,
        }
    }

    fn rest(entry_id: u32) -> TrainingSessionElement {
        TrainingSessionElement::Rest {
            target_time: Time::new(entry_id).unwrap(),
            automatic: true,
        }
    }

    fn set(exercise_id: u128, reps: u32, weight: f32, rpe: RPE) -> TrainingSessionElement {
        TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            reps: Reps::new(reps).unwrap(),
            time: Time::default(),
            weight: Weight::new(weight).unwrap(),
            rpe,
            target_reps: Reps::default(),
            target_tempo: Tempo::default(),
            target_weight: Weight::default(),
            target_rpe: RPE::default(),
            automatic: false,
        }
    }

    fn section(elements: &[TrainingSessionElement]) -> TrainingSessionSection {
        TrainingSessionSection(elements.to_vec())
    }

    #[rstest]
    #[case::today(TODAY.to_string(), Ok(TODAY.to_string()))]
    #[case::in_the_future("2999-01-01".to_string(), Err("date must not be in the future"))]
    #[case::unparsable("02/02/2020".to_string(), Err("invalid date"))]
    fn test_validate_training_session_date(
        #[case] input: String,
        #[case] expected: Result<String, &str>,
    ) {
        assert_eq!(
            Service::new(FakeRepository::default())
                .validate_training_session_date(&input)
                .map(|date| date.to_string())
                .map_err(|err| err.to_string()),
            expected.map_err(str::to_string)
        );
    }

    #[test]
    fn test_get_training_stats() {
        let service = Service::new(FakeRepository::default());
        let training_session = TrainingSession {
            date: *TODAY,
            ..training_session(&[set(1, 5, 100.0, RPE::ZERO), set(1, 5, 100.0, RPE::ZERO)])
        };

        let stats = service.get_training_stats(&[training_session]);

        assert_eq!(stats.short_term_load, [(*TODAY, 2.0)]);
        assert_eq!(stats.long_term_load, []);
    }

    #[test]
    fn test_get_sets_by_exercise() {
        let service = Service::new(FakeRepository::default());
        let training_session = training_session(&[
            set(1, 5, 100.0, RPE::ZERO),
            rest(60),
            set(1, 3, 100.0, RPE::ZERO),
            set(2, 0, 0.0, RPE::ZERO),
        ]);

        let sets = service.get_sets_by_exercise(&training_session);

        assert_eq!(
            sets,
            HashMap::from([(
                ExerciseID::from(1u128),
                vec![&training_session.elements[0], &training_session.elements[2]]
            )])
        );
    }

    #[test]
    fn test_get_recent_session_sets_by_exercise() {
        let service = Service::new(FakeRepository::default());
        let current = dated_training_session(1, 1, *TODAY, &[set(1, 5, 100.0, RPE::ZERO)]);
        let previous = dated_training_session(
            2,
            1,
            *TODAY - Duration::days(7),
            &[set(1, 3, 90.0, RPE::ZERO)],
        );
        let earlier = dated_training_session(
            3,
            1,
            *TODAY - Duration::days(14),
            &[set(1, 2, 80.0, RPE::ZERO)],
        );
        let training_sessions = [
            earlier.clone(),
            dated_training_session(6, 1, *TODAY, &[set(1, 1, 70.0, RPE::ZERO)]),
            previous.clone(),
            dated_training_session(
                4,
                2,
                *TODAY - Duration::days(1),
                &[set(1, 4, 95.0, RPE::ZERO)],
            ),
            dated_training_session(
                5,
                1,
                *TODAY + Duration::days(1),
                &[set(1, 6, 110.0, RPE::ZERO)],
            ),
            current.clone(),
        ];

        assert_eq!(
            service.get_recent_session_sets_by_exercise(&current, &training_sessions, 3),
            HashMap::from([(
                ExerciseID::from(1u128),
                vec![
                    (previous.date, vec![&previous.elements[0]]),
                    (earlier.date, vec![&earlier.elements[0]]),
                ]
            )])
        );
    }

    #[test]
    fn test_get_recent_session_sets_by_exercise_skipping_sessions_without_exercise() {
        let service = Service::new(FakeRepository::default());
        let current = dated_training_session(1, 1, *TODAY, &[set(1, 5, 100.0, RPE::ZERO)]);
        let earlier = dated_training_session(
            3,
            1,
            *TODAY - Duration::days(14),
            &[set(1, 2, 80.0, RPE::ZERO)],
        );
        let training_sessions = [
            current.clone(),
            dated_training_session(
                2,
                1,
                *TODAY - Duration::days(7),
                &[set(2, 3, 90.0, RPE::ZERO), set(1, 0, 0.0, RPE::ZERO)],
            ),
            earlier.clone(),
        ];

        assert_eq!(
            service
                .get_recent_session_sets_by_exercise(&current, &training_sessions, 3)
                .get(&ExerciseID::from(1u128)),
            Some(&vec![(earlier.date, vec![&earlier.elements[0]])])
        );
    }

    #[test]
    fn test_get_recent_session_sets_by_exercise_ignoring_exercises_of_other_sessions() {
        let service = Service::new(FakeRepository::default());
        let current = dated_training_session(1, 1, *TODAY, &[set(1, 5, 100.0, RPE::ZERO)]);
        let earlier = dated_training_session(
            2,
            1,
            *TODAY - Duration::days(7),
            &[set(1, 3, 90.0, RPE::ZERO), set(2, 4, 60.0, RPE::ZERO)],
        );
        let training_sessions = [current.clone(), earlier.clone()];

        assert_eq!(
            service.get_recent_session_sets_by_exercise(&current, &training_sessions, 3),
            HashMap::from([(
                ExerciseID::from(1u128),
                vec![(earlier.date, vec![&earlier.elements[0]])]
            )])
        );
    }

    #[test]
    fn test_get_recent_session_sets_by_exercise_limited_to_limit_sessions() {
        let service = Service::new(FakeRepository::default());
        let current = dated_training_session(1, 1, *TODAY, &[set(1, 5, 100.0, RPE::ZERO)]);
        let training_sessions = (2u32..6)
            .map(|i| {
                dated_training_session(
                    u128::from(i),
                    1,
                    *TODAY - Duration::days(i64::from(i)),
                    &[set(1, 3, 90.0, RPE::ZERO)],
                )
            })
            .collect::<Vec<_>>();

        let sessions = service.get_recent_session_sets_by_exercise(&current, &training_sessions, 3);

        assert_eq!(
            sessions[&ExerciseID::from(1u128)]
                .iter()
                .map(|(date, _)| *date)
                .collect::<Vec<_>>(),
            vec![
                *TODAY - Duration::days(2),
                *TODAY - Duration::days(3),
                *TODAY - Duration::days(4),
            ]
        );
    }

    #[test]
    fn test_get_recent_session_sets_by_exercise_without_earlier_session() {
        let service = Service::new(FakeRepository::default());
        let current = dated_training_session(1, 1, *TODAY, &[set(1, 5, 100.0, RPE::ZERO)]);

        assert_eq!(
            service.get_recent_session_sets_by_exercise(
                &current,
                std::slice::from_ref(&current),
                3
            ),
            HashMap::new()
        );
    }

    #[test]
    fn test_get_recent_session_sets_by_exercise_without_limit() {
        let service = Service::new(FakeRepository::default());
        let current = dated_training_session(1, 1, *TODAY, &[set(1, 5, 100.0, RPE::ZERO)]);
        let earlier = dated_training_session(
            2,
            1,
            *TODAY - Duration::days(1),
            &[set(1, 3, 90.0, RPE::ZERO)],
        );

        assert_eq!(
            service.get_recent_session_sets_by_exercise(&current, &[current.clone(), earlier], 0),
            HashMap::new()
        );
    }

    fn dated_training_session(
        id: u128,
        routine_id: u128,
        date: NaiveDate,
        elements: &[TrainingSessionElement],
    ) -> TrainingSession {
        TrainingSession {
            id: id.into(),
            routine_id: routine_id.into(),
            date,
            ..training_session(elements)
        }
    }

    #[rstest]
    #[case::all_shown(true, true, "10 × 30 kg @ 8 (3 s)")]
    #[case::without_tut(false, true, "10 × 30 kg @ 8")]
    #[case::without_rpe(true, false, "10 × 30 kg (3 s)")]
    #[case::without_tut_and_rpe(false, false, "10 × 30 kg")]
    fn test_set_to_string(#[case] show_tut: bool, #[case] show_rpe: bool, #[case] expected: &str) {
        let set = Set {
            reps: Reps::new(10).unwrap(),
            time: Time::new(3).unwrap(),
            weight: Weight::new(30.0).unwrap(),
            rpe: RPE::EIGHT,
        };

        assert_eq!(set.to_string(show_tut, show_rpe), expected);
    }

    #[test]
    fn test_set_to_string_without_time() {
        let set = Set {
            reps: Reps::new(10).unwrap(),
            time: Time::default(),
            weight: Weight::new(30.0).unwrap(),
            rpe: RPE::EIGHT,
        };

        assert_eq!(set.to_string(true, true), "10 × 30 kg @ 8");
    }

    #[test]
    fn test_set_to_string_of_a_hold() {
        let set = Set {
            reps: Reps::default(),
            time: Time::new(60).unwrap(),
            weight: Weight::default(),
            rpe: RPE::EIGHT,
        };

        assert_eq!(set.to_string(true, true), "60 s @ 8");
    }

    #[test]
    fn test_set_to_string_without_values() {
        let set = Set {
            reps: Reps::default(),
            time: Time::default(),
            weight: Weight::default(),
            rpe: RPE::ZERO,
        };

        assert_eq!(set.to_string(true, true), "");
    }

    #[test]
    fn test_set_to_string_with_only_an_rpe() {
        let set = Set {
            reps: Reps::default(),
            time: Time::default(),
            weight: Weight::default(),
            rpe: RPE::EIGHT,
        };

        assert_eq!(set.to_string(true, true), "@ 8");
    }

    #[test]
    fn test_training_session_element_to_string() {
        let element = TrainingSessionElement::Set {
            exercise_id: 1.into(),
            reps: Reps::new(10).unwrap(),
            time: Time::new(3).unwrap(),
            weight: Weight::new(30.0).unwrap(),
            rpe: RPE::EIGHT,
            target_reps: Reps::new(8).unwrap(),
            target_tempo: Tempo::new(&[4]).unwrap(),
            target_weight: Weight::new(40.0).unwrap(),
            target_rpe: RPE::NINE,
            automatic: false,
        };

        assert_eq!(element.to_string(true, true), "10 × 30 kg @ 8 (3 s)");
        assert_eq!(element.target_to_string(true), "8 × 40 kg @ 9 (4 s)");
    }

    #[test]
    fn test_training_session_element_target_to_string_of_a_subdivided_tempo() {
        let element = TrainingSessionElement::Set {
            exercise_id: 1.into(),
            reps: Reps::default(),
            time: Time::default(),
            weight: Weight::default(),
            rpe: RPE::default(),
            target_reps: Reps::new(8).unwrap(),
            target_tempo: Tempo::new(&[3, 1, 1, 0]).unwrap(),
            target_weight: Weight::new(40.0).unwrap(),
            target_rpe: RPE::NINE,
            automatic: false,
        };

        assert_eq!(element.target_to_string(true), "8 × 40 kg @ 9 (3·1·1·0)");
        assert_eq!(element.target_to_string(false), "8 × 40 kg (3·1·1·0)");
    }

    #[test]
    fn test_training_session_element_to_string_of_rest() {
        assert_eq!(rest(60).to_string(true, true), "");
        assert_eq!(rest(60).target_to_string(true), "");
    }

    #[test]
    fn test_training_session_is_empty() {
        assert!(training_session(&[]).is_empty());
        assert!(training_session(&[set(1, 0, 0.0, RPE::ZERO), rest(60)]).is_empty());
        assert!(!training_session(&[set(1, 5, 0.0, RPE::ZERO)]).is_empty());
    }

    #[test]
    fn test_training_session_element_is_empty_of_rest() {
        assert!(rest(60).is_empty());
    }

    #[test]
    fn test_training_session_element_one_rep_max() {
        assert!(set(1, 5, 100.0, RPE::TEN).one_rep_max().is_some());
        assert_eq!(set(1, 0, 100.0, RPE::TEN).one_rep_max(), None);
        assert_eq!(set(1, 5, 0.0, RPE::TEN).one_rep_max(), None);
        assert_eq!(rest(60).one_rep_max(), None);
    }

    #[test]
    fn test_training_session_section_exercise_counts() {
        let section = section(&[
            set(1, 5, 100.0, RPE::ZERO),
            set(2, 5, 50.0, RPE::ZERO),
            set(1, 5, 100.0, RPE::ZERO),
        ]);

        assert_eq!(
            section.exercise_counts(),
            HashMap::from([(ExerciseID::from(1u128), 2), (2.into(), 1)])
        );
    }

    #[test]
    fn test_training_session_id_from_str() {
        let id = TrainingSessionID::from(uuid::Uuid::from_u128(1));

        assert_eq!(id, TrainingSessionID::from(1u128));
        assert!(!id.is_nil());
        assert_eq!(TrainingSessionID::from_str(&id.to_string()), Ok(id));
    }

    fn any_session() -> impl Strategy<Value = TrainingSession> {
        prop::collection::vec(prop::option::of(1u128..4), 1..10)
            .prop_map(|slots| {
                let mut elements: Vec<TrainingSessionElement> = vec![];
                for slot in slots {
                    match slot {
                        Some(exercise_id) => {
                            elements.push(set(exercise_id, 5, 100.0, RPE::ZERO));
                        }
                        None => {
                            if matches!(elements.last(), Some(TrainingSessionElement::Set { .. })) {
                                elements.push(rest(60));
                            }
                        }
                    }
                }
                training_session(&elements)
            })
            .prop_filter("session without sets", |session| {
                !session.elements.is_empty()
            })
    }

    fn has_rest_only_section(session: &TrainingSession) -> bool {
        session.compute_sections().iter().any(|section| {
            !section
                .elements()
                .iter()
                .any(|element| matches!(element, TrainingSessionElement::Set { .. }))
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn test_training_session_mutations_keep_a_set_in_every_section(
            session in any_session(),
            operations in prop::collection::vec((0u8..7, 0usize..8, 0usize..8), 0..8),
        ) {
            let mut session = session;

            for (operation, a, b) in operations {
                let sections = session.compute_sections();
                if sections.is_empty() {
                    continue;
                }
                let section_idx = a % sections.len();
                let exercises = sections[section_idx].exercise_ids().len();
                match operation {
                    0 => session.add_set(b % session.elements.len()),
                    1 => session.add_exercise(section_idx, (b as u128 % 3 + 1).into()),
                    2 if exercises > 0 => session.add_same_exercise(section_idx, b % exercises),
                    3 => session.remove_set(section_idx),
                    4 if exercises > 0 => session.remove_exercise(section_idx, b % exercises),
                    5 => session.move_section_up(section_idx),
                    6 => session.move_section_down(section_idx),
                    _ => {}
                }

                prop_assert!(!has_rest_only_section(&session));
            }
        }
    }

    #[rstest]
    #[case::opening_exercise(0, &[exercise(1, 1), exercise(2, 2), rest(90), exercise(5, 1), rest(31), exercise(6, 2), rest(91)])]
    #[case::middle_exercise(1, &[exercise(0, 0), exercise(2, 2), rest(90), exercise(4, 0), rest(30), exercise(6, 2), rest(91)])]
    #[case::closing_exercise(2, &[exercise(0, 0), exercise(1, 1), rest(90), exercise(4, 0), rest(30), exercise(5, 1), rest(91)])]
    fn test_training_session_remove_exercise_keeps_the_rest_closing_a_round(
        #[case] exercise_idx: usize,
        #[case] expected: &[TrainingSessionElement],
    ) {
        let mut session = training_session(&[
            exercise(0, 0),
            exercise(1, 1),
            exercise(2, 2),
            rest(90),
            exercise(4, 0),
            rest(30),
            exercise(5, 1),
            rest(31),
            exercise(6, 2),
            rest(91),
        ]);

        session.remove_exercise(0, exercise_idx);

        assert_eq!(session.elements, expected);
    }

    #[test]
    fn test_training_session_remove_exercise_drops_the_rest_of_an_emptied_round() {
        let mut session = training_session(&[
            set(1, 5, 100.0, RPE::ZERO),
            set(2, 5, 100.0, RPE::ZERO),
            rest(60),
            set(1, 5, 100.0, RPE::ZERO),
            rest(60),
            set(2, 5, 100.0, RPE::ZERO),
            rest(60),
        ]);

        session.remove_exercise(0, 1);

        assert_eq!(
            session.elements,
            [
                set(1, 5, 100.0, RPE::ZERO),
                rest(60),
                set(1, 5, 100.0, RPE::ZERO),
                rest(60),
            ]
        );
    }

    #[test]
    fn test_training_session_move_section_down_of_an_inner_section() {
        let mut session = training_session(&[
            set(1, 5, 100.0, RPE::ZERO),
            rest(60),
            set(2, 5, 100.0, RPE::ZERO),
            rest(60),
            set(3, 5, 100.0, RPE::ZERO),
            rest(60),
            set(4, 5, 100.0, RPE::ZERO),
        ]);

        session.move_section_down(2);

        assert_eq!(
            session.elements,
            [
                set(1, 5, 100.0, RPE::ZERO),
                rest(60),
                set(2, 5, 100.0, RPE::ZERO),
                rest(60),
                set(4, 5, 100.0, RPE::ZERO),
                rest(0),
                set(3, 5, 100.0, RPE::ZERO),
                rest(60),
            ]
        );
    }
}
