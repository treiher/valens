use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    iter,
    ops::{Range, RangeInclusive},
    str::FromStr,
};

use chrono::{Local, NaiveDate};
use derive_more::Deref;
use log::error;
use uuid::Uuid;

use crate::{
    CreateError, DeleteError, Exercise, ExerciseID, Laterality, MuscleID, RPE, ReadError, Reps,
    RoutineID, Side, Sides, Stimulus, SyncError, Tempo, Time, TrainingStats, UpdateError,
    ValidationError, Weight, one_rep_max, training::values_to_string, training_stats,
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
    /// recent to oldest. The group index of a set is that of `TrainingSession::group_indices`.
    ///
    /// Only training sessions for the same routine that occurred before the current one are taken
    /// into account. Sessions without a non-empty set for an exercise are skipped for that
    /// exercise, so the sets of an exercise that was left out of a session are still returned.
    fn get_recent_session_sets_by_exercise<'a>(
        &self,
        training_session: &TrainingSession,
        training_sessions: &'a [TrainingSession],
        limit: usize,
    ) -> HashMap<ExerciseID, Vec<RecentSessionSets<'a>>> {
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

        let mut result: HashMap<ExerciseID, Vec<RecentSessionSets<'a>>> = HashMap::new();
        let mut sets_by_exercise: HashMap<ExerciseID, Vec<(usize, &'a TrainingSessionElement)>> =
            HashMap::new();
        for earlier_training_session in earlier_training_sessions {
            let group_indices = earlier_training_session.group_indices();
            for (element_idx, element) in earlier_training_session.elements.iter().enumerate() {
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
                        .push((group_indices[&element_idx], element));
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

/// The date of an earlier training session and its non-empty sets of an exercise, each with its
/// group index in the session.
pub type RecentSessionSets<'a> = (NaiveDate, Vec<(usize, &'a TrainingSessionElement)>);

/// The sets of a recent session offered to the sets of one side.
#[derive(Debug, PartialEq)]
pub struct OfferedSets {
    sets: Vec<Set>,
    /// The group index of each set, `None` if the recent session holds no set with a side.
    group_indices: Option<Vec<usize>>,
}

impl OfferedSets {
    /// Selects the sets among the sets of a recent session, each with its group index, that are
    /// offered to a set of `side`.
    ///
    /// A set is offered the recent sets without a side and those of its side, a set without a side
    /// those of the left side.
    #[must_use]
    pub fn new(sets: &[(usize, &TrainingSessionElement)], side: Side) -> Self {
        let sets = sets
            .iter()
            .filter_map(|(group_index, element)| match element {
                TrainingSessionElement::Set { side, .. } => {
                    Some((*group_index, *side, element.set()?))
                }
                TrainingSessionElement::Rest { .. } => None,
            })
            .collect::<Vec<_>>();
        let has_sides = sets.iter().any(|(_, s, _)| *s != Side::Unset);
        let side = side.non_zero().unwrap_or(Side::Left);
        let offered = sets
            .into_iter()
            .filter(|(_, s, _)| *s == Side::Unset || *s == side)
            .collect::<Vec<_>>();
        Self {
            group_indices: has_sides.then(|| offered.iter().map(|(g, _, _)| *g).collect()),
            sets: offered.into_iter().map(|(_, _, set)| set).collect(),
        }
    }

    #[must_use]
    pub fn sets(&self) -> &[Set] {
        &self.sets
    }

    /// The index of the set corresponding to a set of `group_index` and `set_index`.
    ///
    /// It is the set of the same group, or in a recent session without sides the set at the same
    /// position.
    #[must_use]
    pub fn index(&self, group_index: usize, set_index: usize) -> Option<usize> {
        match &self.group_indices {
            Some(group_indices) => group_indices.iter().position(|g| *g == group_index),
            None => (set_index < self.sets.len()).then_some(set_index),
        }
    }
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

    /// The number of sets weighted by their rating, a set with a side counting half.
    #[must_use]
    pub fn load(&self) -> f32 {
        halved(
            self.elements
                .iter()
                .map(|element| halves(element) * set_load(element))
                .sum(),
        )
    }

    /// The number of sets that count for the muscle, a set with a side counting half.
    #[must_use]
    pub fn set_volume(&self) -> f32 {
        halved(
            self.elements
                .iter()
                .map(|element| halves(element) * u32::from(counts_for_muscle(element)))
                .sum(),
        )
    }

    /// The repetitions times the weight, a set with a side counting in full.
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

    /// The time under tension, a set with a side counting half, rounded half up.
    #[must_use]
    pub fn tut(&self) -> Option<u32> {
        let tuts = self
            .elements
            .iter()
            .filter_map(|element| Some(halves(element) * set_tut(element)?))
            .collect::<Vec<_>>();
        (!tuts.is_empty()).then(|| tuts.iter().sum::<u32>().div_ceil(2))
    }

    /// Whether any set records a time or prescribes a tempo.
    #[must_use]
    pub fn has_time(&self) -> bool {
        self.elements.iter().any(|e| match e {
            TrainingSessionElement::Set {
                time, target_tempo, ..
            } => time.non_zero().is_some() || target_tempo.non_zero().is_some(),
            TrainingSessionElement::Rest { .. } => false,
        })
    }

    /// Whether any set records or prescribes a rating of perceived exertion.
    #[must_use]
    pub fn has_rpe(&self) -> bool {
        self.elements.iter().any(|e| match e {
            TrainingSessionElement::Set {
                rpe, target_rpe, ..
            } => rpe.non_zero().is_some() || target_rpe.non_zero().is_some(),
            TrainingSessionElement::Rest { .. } => false,
        })
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

    /// The stimulus of the sets that count for the muscle, a set with a side counting half.
    #[must_use]
    pub fn stimulus_per_muscle(&self, exercises: &[Exercise]) -> BTreeMap<MuscleID, Stimulus> {
        let mut result: BTreeMap<MuscleID, Stimulus> = BTreeMap::new();
        for element in &self.elements {
            if !counts_for_muscle(element) {
                continue;
            }
            if let TrainingSessionElement::Set { exercise_id, .. } = element
                && let Some(exercise) = exercises.iter().find(|e| e.id == *exercise_id)
            {
                for (muscle_id, stimulus) in &exercise.muscle_stimulus() {
                    *result.entry(*muscle_id).or_insert(Stimulus::NONE) +=
                        *stimulus * halves(element);
                }
            }
        }
        result
            .into_iter()
            .map(|(muscle_id, stimulus)| (muscle_id, stimulus / 2))
            .collect()
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

    pub fn add_set(&mut self, element_idx: usize) -> ElementMap {
        let section_idx = self.section_idx(element_idx);
        let sections = self.compute_sections();
        let Some(section) = sections.get(section_idx) else {
            return ElementMap::unchanged(self.elements.len());
        };
        let rests = section
            .elements()
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
        for element in section.elements() {
            match element {
                TrainingSessionElement::Set {
                    exercise_id,
                    side,
                    target_reps,
                    target_tempo,
                    target_weight,
                    target_rpe,
                    automatic,
                    ..
                } => {
                    sets.push(TrainingSessionElement::Set {
                        exercise_id: *exercise_id,
                        side: *side,
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

        let mut section_elements =
            tagged(section.elements(), section_start(&sections, section_idx));
        if matches!(
            section.elements().last(),
            Some(TrainingSessionElement::Set { .. })
        ) {
            section_elements.extend(untagged([rest]));
            section_elements.extend(untagged(sets));
        } else {
            section_elements.extend(untagged(sets));
            section_elements.extend(untagged([rest]));
        }

        self.replace_section(&sections, section_idx, section_elements, "adding set")
    }

    /// Adds sets of `exercise_id` to every round of the section.
    ///
    /// The sets are a pair if the sets of the exercise already in a round carry a side, or if the
    /// round holds none and the laterality calls for a pair.
    pub fn add_exercise(
        &mut self,
        section_idx: usize,
        exercise_id: ExerciseID,
        laterality: Option<Laterality>,
    ) -> ElementMap {
        let sections = &self.compute_sections();
        let section = &sections[section_idx];
        let sides = Sides::for_laterality(laterality);

        let mut elements = vec![];
        let mut run_start = section_start(sections, section_idx);
        for run in section
            .elements()
            .split_inclusive(|element| matches!(element, TrainingSessionElement::Rest { .. }))
        {
            let (sets, rest) = match run.split_last() {
                Some((rest @ TrainingSessionElement::Rest { .. }, sets)) => (sets, Some(rest)),
                _ => (run, None),
            };
            elements.extend(tagged(sets, run_start));
            elements.extend(untagged(new_sets(
                exercise_id,
                sides_of(sets, exercise_id).unwrap_or(sides),
            )));
            elements.extend(rest.map(|rest| (Some(run_start + sets.len()), rest.clone())));
            run_start += run.len();
        }

        self.replace_section(sections, section_idx, elements, "adding exercise")
    }

    /// Replaces `exercise_id` in the section.
    ///
    /// For a unilateral replacement, each side-less set becomes a pair whose sides both take the
    /// values of the set. For a bilateral replacement, each pair becomes its first set with recorded
    /// values, or its first set if none has any, and no set keeps its side. Without a laterality,
    /// the sides are kept. Sets of the replacement that thereby directly follow each other with
    /// opposite sides form a pair.
    pub fn replace_exercise(
        &mut self,
        section_idx: usize,
        exercise_id: ExerciseID,
        replacement: ExerciseID,
        laterality: Option<Laterality>,
    ) -> ElementMap {
        let sections = self.compute_sections();
        let start = section_start(&sections, section_idx);
        let section = sections[section_idx].elements();

        let mut elements = vec![];
        for group in groups(section) {
            let old = &section[group.clone()];
            let new = if matches!(
                old[0],
                TrainingSessionElement::Set { exercise_id: id, .. } if id == exercise_id
            ) {
                replaced_group(old, replacement, laterality)
            } else {
                old.to_vec()
            };
            // The sets of a group take the places of the sets they replace in order, any further
            // set being new.
            let origins = group.map(|idx| Some(start + idx)).chain(iter::repeat(None));
            elements.extend(origins.zip(new));
        }

        self.replace_section(&sections, section_idx, elements, "replacing exercise")
    }

    pub fn remove_set(&mut self, section_idx: usize) -> ElementMap {
        let section = self.section_range(section_idx);
        let end = *section.end();
        let mut first_removed = end + 1;
        for i in section.rev() {
            if i != end && matches!(self.elements[i], TrainingSessionElement::Rest { .. }) {
                break;
            }
            first_removed = i;
        }
        let removed = first_removed..=end;
        let elements = tagged(&self.elements, 0)
            .into_iter()
            .enumerate()
            .filter(|(idx, _)| !removed.contains(idx))
            .map(|(_, element)| element)
            .collect();
        self.rebuild(elements, "removing set")
    }

    /// Removes the last group of sets of `exercise_id` from every run of the section, or the
    /// whole section if its first run holds a single group.
    pub fn remove_exercise(&mut self, section_idx: usize, exercise_id: ExerciseID) -> ElementMap {
        let sections = self.compute_sections();
        let section = &sections[section_idx];

        let mut elements: Vec<TaggedElement> = vec![];
        let ids = section.exercise_ids();
        if groups(&section.elements()[..ids.len()]).len() > 1 {
            let closes_round = ids.last() == Some(&exercise_id);
            let mut run_start = section_start(&sections, section_idx);
            for run in section
                .elements()
                .split_inclusive(|element| matches!(element, TrainingSessionElement::Rest { .. }))
            {
                let sets = run
                    .iter()
                    .take_while(|element| matches!(element, TrainingSessionElement::Set { .. }))
                    .count();
                let removed = groups(&run[..sets])
                    .into_iter()
                    .rfind(|group| {
                        matches!(run[group.start], TrainingSessionElement::Set { exercise_id: id, .. } if id == exercise_id)
                    })
                    .unwrap_or_default();
                let remaining = tagged(run, run_start)
                    .into_iter()
                    .enumerate()
                    .filter(|(idx, _)| !removed.contains(idx))
                    .map(|(_, element)| element)
                    .collect::<Vec<_>>();
                run_start += run.len();
                if remaining
                    .iter()
                    .any(|(_, element)| matches!(element, TrainingSessionElement::Set { .. }))
                {
                    elements.extend(remaining);
                    continue;
                }
                // Of the rests around an emptied run, the one closing the round is kept.
                if closes_round {
                    if matches!(
                        elements.last(),
                        Some((_, TrainingSessionElement::Rest { .. }))
                    ) {
                        elements.pop();
                    }
                    elements.extend(remaining);
                }
            }
        }

        self.replace_section(&sections, section_idx, elements, "removing exercise")
    }

    /// Appends sets of `exercise_id` to the session.
    ///
    /// The sets are a pair if the sets of the exercise in the last run of the session carry a
    /// side, or if the run holds none and the laterality calls for a pair.
    pub fn append_exercise(
        &mut self,
        exercise_id: ExerciseID,
        laterality: Option<Laterality>,
    ) -> ElementMap {
        let last_run = self
            .elements
            .rsplit(|element| matches!(element, TrainingSessionElement::Rest { .. }))
            .find(|run| !run.is_empty())
            .unwrap_or_default();
        let sides = sides_of(last_run, exercise_id).unwrap_or(Sides::for_laterality(laterality));
        let mut elements = tagged(&self.elements, 0);
        if let Some(TrainingSessionElement::Set { .. }) = self.elements.last() {
            elements.extend(untagged([TrainingSessionElement::Rest {
                target_time: Time::default(),
                automatic: true,
            }]));
        }
        elements.extend(untagged(new_sets(exercise_id, sides)));
        self.rebuild(elements, "appending exercise")
    }

    pub fn move_section_up(&mut self, section_idx: usize) -> ElementMap {
        if section_idx == 0 {
            return ElementMap::unchanged(self.elements.len());
        }
        let section = self.section_range(section_idx);
        debug_assert!(section.start() <= section.end());
        let previous_section = self.section_range(section_idx - 1);
        let mut elements = tagged(&self.elements, 0);
        let mut trailing_rest = 0;
        if section.end() + 1 == self.elements.len()
            && let Some(TrainingSessionElement::Set { .. }) = self.elements.last()
        {
            elements.extend(untagged([TrainingSessionElement::Rest {
                target_time: Time::default(),
                automatic: true,
            }]));
            trailing_rest += 1;
        }
        elements[*previous_section.start()..=*section.end() + trailing_rest]
            .rotate_right(section.end() - section.start() + trailing_rest + 1);
        self.rebuild(elements, "moving section up")
    }

    pub fn move_section_down(&mut self, section_idx: usize) -> ElementMap {
        let section = self.section_range(section_idx);
        if *section.end() + 1 == self.elements.len() {
            return ElementMap::unchanged(self.elements.len());
        }
        let subsequent_section = self.section_range(section_idx + 1);
        let section_len = section.end() - section.start() + 1;
        let subsequent_section_len = subsequent_section.end() - subsequent_section.start() + 1;
        let mut elements = tagged(&self.elements, 0);
        let mut trailing_rest = 0;
        if section.start() + section_len + subsequent_section_len == self.elements.len()
            && let Some(TrainingSessionElement::Set { .. }) = self.elements.last()
        {
            elements.extend(untagged([TrainingSessionElement::Rest {
                target_time: Time::default(),
                automatic: true,
            }]));
            trailing_rest += 1;
        }
        elements[*section.start()
            ..*section.start() + section_len + subsequent_section_len + trailing_rest]
            .rotate_right(subsequent_section_len + trailing_rest);
        self.rebuild(elements, "moving section down")
    }

    fn replace_section(
        &mut self,
        sections: &[TrainingSessionSection],
        section_idx: usize,
        elements: Vec<TaggedElement>,
        action: &str,
    ) -> ElementMap {
        let start = section_start(sections, section_idx);
        let end = start + sections[section_idx].elements().len();
        let mut all_elements = tagged(&self.elements[..start], 0);
        all_elements.extend(elements);
        all_elements.extend(tagged(&self.elements[end..], end));
        self.rebuild(all_elements, action)
    }

    /// Replaces the elements by `elements` and returns where the previous elements moved to.
    ///
    /// Each element is tagged with its previous index, or with `None` if it is new. Sections
    /// consisting only of rests are removed.
    fn rebuild(&mut self, elements: Vec<TaggedElement>, action: &str) -> ElementMap {
        let previous_len = self.elements.len();
        let (mut origins, elements): (Vec<_>, Vec<_>) = elements.into_iter().unzip();
        self.elements = elements;

        let sections = self.compute_sections();
        let has_set = |section: &TrainingSessionSection| {
            section
                .elements()
                .iter()
                .any(|e| matches!(e, TrainingSessionElement::Set { .. }))
        };
        if !sections.iter().all(has_set) {
            debug_assert!(
                false,
                "{action} resulted in a section consisting only of rest elements"
            );
            error!("{action} resulted in a section consisting only of rest elements");
            let mut remaining_origins = origins.into_iter();
            origins = vec![];
            for section in &sections {
                let section_origins = remaining_origins
                    .by_ref()
                    .take(section.elements().len())
                    .collect::<Vec<_>>();
                if has_set(section) {
                    origins.extend(section_origins);
                }
            }
            self.elements = sections
                .into_iter()
                .filter(has_set)
                .flat_map(|s| s.elements().to_vec())
                .collect();
        }

        ElementMap::new(previous_len, &origins)
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

    /// Returns the groups of `exercise_id` in one run of consecutive sets of the given section,
    /// in order, each group as the indices of its elements.
    ///
    /// A run is a maximal sequence of sets not interrupted by a rest. The run containing
    /// `element_idx` is returned, a rest counting towards the run following it. If `element_idx`
    /// lies outside the runs of the section, the first run is returned. A pair of sets forms one
    /// group and every other set a group of its own.
    #[must_use]
    pub fn run_element_indices(
        &self,
        section_idx: usize,
        exercise_id: ExerciseID,
        element_idx: usize,
    ) -> Vec<Vec<usize>> {
        if section_idx >= self.section_count() {
            return vec![];
        }

        let section = self.section_range(section_idx);
        let mut runs: Vec<Range<usize>> = vec![];
        let mut start = None;
        for idx in section.clone() {
            match self.elements[idx] {
                TrainingSessionElement::Set { .. } => {
                    start.get_or_insert(idx);
                }
                TrainingSessionElement::Rest { .. } => {
                    if let Some(start) = start.take() {
                        runs.push(start..idx);
                    }
                }
            }
        }
        if let Some(start) = start {
            runs.push(start..section.end() + 1);
        }

        let run = runs
            .iter()
            .find(|run| run.end > element_idx)
            .or_else(|| runs.first());

        run.map(|run| {
            groups(&self.elements[run.clone()])
                .into_iter()
                .map(|group| (group.start + run.start..group.end + run.start).collect::<Vec<_>>())
                .filter(|group| {
                    matches!(
                        self.elements[group[0]],
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

    /// For every set, the position of its group (see `groups`) among the groups of the same
    /// exercise, keyed by the index of the element. The two sets of a pair share a position.
    #[must_use]
    pub fn group_indices(&self) -> HashMap<usize, usize> {
        let mut counts: HashMap<ExerciseID, usize> = HashMap::new();
        groups(&self.elements)
            .into_iter()
            .filter_map(|group| match self.elements[group.start] {
                TrainingSessionElement::Set { exercise_id, .. } => {
                    let count = counts.entry(exercise_id).or_default();
                    let group_index = *count;
                    *count += 1;
                    Some(group.map(move |element_idx| (element_idx, group_index)))
                }
                TrainingSessionElement::Rest { .. } => None,
            })
            .flatten()
            .collect()
    }

    /// Returns the session reduced to the sets of `exercise_id`.
    #[must_use]
    pub fn restricted_to(&self, exercise_id: ExerciseID) -> TrainingSession {
        let elements = self
            .elements
            .iter()
            .filter(|element| {
                matches!(element, TrainingSessionElement::Set { exercise_id: id, .. } if *id == exercise_id)
            })
            .cloned()
            .collect();
        TrainingSession {
            id: self.id,
            routine_id: self.routine_id,
            date: self.date,
            notes: self.notes.clone(),
            elements,
            exercise_notes: self.exercise_notes.clone(),
        }
    }

    /// Returns the sets of `exercise_id` as rows of a history, grouped as by `groups`.
    #[must_use]
    pub fn set_history_rows(&self, exercise_id: ExerciseID) -> Vec<SetHistoryRow> {
        self.paired_sets()
            .into_iter()
            .filter(|group| {
                matches!(group[0], TrainingSessionElement::Set { exercise_id: id, .. } if id == exercise_id)
            })
            .filter_map(|group| match ElementGroup::of(group) {
                ElementGroup::Sides { left, right } => Some(SetHistoryRow::Sides {
                    left: left.and_then(TrainingSessionElement::set),
                    right: right.and_then(TrainingSessionElement::set),
                }),
                ElementGroup::Single(element) => element.set().map(SetHistoryRow::Combined),
            })
            .collect()
    }

    /// Whether the element at `element_idx` is the second set of a pair.
    #[must_use]
    pub fn completes_pair(&self, element_idx: usize) -> bool {
        groups(&self.elements)
            .iter()
            .any(|group| group.len() == 2 && group.end == element_idx + 1)
    }

    /// Returns the sets in order, grouped as by `groups`.
    fn paired_sets(&self) -> Vec<&[TrainingSessionElement]> {
        groups(&self.elements)
            .into_iter()
            .map(|group| &self.elements[group])
            .filter(|group| matches!(group[0], TrainingSessionElement::Set { .. }))
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

        // A section does not separate the sets of a pair.
        if let Some(group) = groups(&self.elements[element_idx..])
            .into_iter()
            .find(|group| group.contains(&(last - element_idx)))
        {
            last = element_idx + group.end - 1;
        }

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
}

/// Where each element of a session moved to by a change of its elements, by its previous index.
#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub struct ElementMap(Vec<ElementMove>);

impl ElementMap {
    fn new(previous_len: usize, origins: &[Option<usize>]) -> Self {
        let mut kept = vec![None; previous_len];
        for (idx, origin) in origins.iter().enumerate() {
            if let Some(origin) = origin {
                kept[*origin] = Some(idx);
            }
        }
        let mut moves = vec![ElementMove::Removed(0); previous_len];
        let mut following = origins.len();
        for (origin, kept) in kept.into_iter().enumerate().rev() {
            moves[origin] = if let Some(idx) = kept {
                following = following.min(idx);
                ElementMove::Kept(idx)
            } else {
                ElementMove::Removed(following)
            };
        }
        Self(moves)
    }

    fn unchanged(len: usize) -> Self {
        Self((0..len).map(ElementMove::Kept).collect())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementMove {
    /// The element is at the given index.
    Kept(usize),
    /// The element was removed. The given index is that of the first kept element that followed
    /// it, or the number of elements if there is none.
    Removed(usize),
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
        side: Side,
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

/// A row of the set history of an exercise.
#[derive(Debug, Clone, PartialEq)]
pub enum SetHistoryRow {
    /// The sets of a pair, or a set with a side whose other side is missing.
    Sides {
        left: Option<Set>,
        right: Option<Set>,
    },
    /// A set without a side.
    Combined(Set),
}

/// A group of elements as formed by `groups`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElementGroup<'a> {
    /// The sets of a pair, or a set with a side whose other side is missing.
    Sides {
        left: Option<&'a TrainingSessionElement>,
        right: Option<&'a TrainingSessionElement>,
    },
    /// A set without a side, or a rest.
    Single(&'a TrainingSessionElement),
}

impl<'a> ElementGroup<'a> {
    fn of(group: &'a [TrainingSessionElement]) -> Self {
        match group[0] {
            TrainingSessionElement::Set {
                side: Side::Left, ..
            } => Self::Sides {
                left: group.first(),
                right: group.get(1),
            },
            TrainingSessionElement::Set {
                side: Side::Right, ..
            } => Self::Sides {
                left: group.get(1),
                right: group.first(),
            },
            _ => Self::Single(&group[0]),
        }
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

    /// Returns the elements of the section grouped as by `groups`.
    #[must_use]
    pub fn groups(&self) -> Vec<ElementGroup<'_>> {
        groups(&self.0)
            .into_iter()
            .map(|group| ElementGroup::of(&self.0[group]))
            .collect()
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

/// Returns the highest estimated 1RM for `exercise_id` over the sessions dated within `dates`.
#[must_use]
pub fn best_one_rep_max_within(
    sessions: &[TrainingSession],
    exercise_id: ExerciseID,
    dates: RangeInclusive<NaiveDate>,
) -> Option<f32> {
    sessions
        .iter()
        .filter(|s| dates.contains(&s.date))
        .filter_map(|s| s.one_rep_max(exercise_id))
        .reduce(f32::max)
}

/// The number of halves of a set an element counts as, a set with a side counting as one half.
fn halves(element: &TrainingSessionElement) -> u32 {
    match element {
        TrainingSessionElement::Set {
            side: Side::Unset, ..
        }
        | TrainingSessionElement::Rest { .. } => 2,
        TrainingSessionElement::Set { .. } => 1,
    }
}

#[allow(clippy::cast_precision_loss)]
fn halved(halves: u32) -> f32 {
    halves as f32 / 2.
}

fn set_load(element: &TrainingSessionElement) -> u32 {
    match element {
        TrainingSessionElement::Set {
            reps, time, rpe, ..
        } => {
            if let Some(rpe) = rpe.non_zero() {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                if rpe > RPE::FIVE {
                    (2.0_f32).powf(f32::from(rpe) - 5.0).round() as u32
                } else {
                    1
                }
            } else {
                u32::from(reps.non_zero().is_some() || time.non_zero().is_some())
            }
        }
        TrainingSessionElement::Rest { .. } => 0,
    }
}

/// Whether a set records reps or a time with a rating that is unset or at least seven.
fn counts_for_muscle(element: &TrainingSessionElement) -> bool {
    match element {
        TrainingSessionElement::Set {
            reps, time, rpe, ..
        } => {
            rpe.non_zero().unwrap_or(RPE::TEN) >= RPE::SEVEN
                && (reps.non_zero().is_some() || time.non_zero().is_some())
        }
        TrainingSessionElement::Rest { .. } => false,
    }
}

fn set_tut(element: &TrainingSessionElement) -> Option<u32> {
    match element {
        TrainingSessionElement::Set { reps, time, .. } => time
            .non_zero()
            .map(|v| u32::from(reps.non_zero().unwrap_or(Reps::new(1).unwrap()) * v)),
        TrainingSessionElement::Rest { .. } => None,
    }
}

/// Splits elements into groups, two adjacent sets of the same exercise with opposite sides forming
/// a pair and every other element a group of its own. Pairs are formed from the start, so a set
/// belongs to the pair with its predecessor before one with its successor.
fn groups(run: &[TrainingSessionElement]) -> Vec<Range<usize>> {
    let mut result = vec![];
    let mut idx = 0;
    while idx < run.len() {
        let len = if let (
            TrainingSessionElement::Set {
                exercise_id: first,
                side: first_side,
                ..
            },
            Some(TrainingSessionElement::Set {
                exercise_id: second,
                side: second_side,
                ..
            }),
        ) = (&run[idx], run.get(idx + 1))
            && first == second
            && first_side.opposes(*second_side)
        {
            2
        } else {
            1
        };
        result.push(idx..idx + len);
        idx += len;
    }
    result
}

/// Returns the sets of `replacement` that replace `group`.
fn replaced_group(
    group: &[TrainingSessionElement],
    replacement: ExerciseID,
    laterality: Option<Laterality>,
) -> Vec<TrainingSessionElement> {
    let base = group
        .iter()
        .find(|element| !element.is_empty())
        .unwrap_or(&group[0]);
    let with_side = |side| {
        let mut element = base.clone();
        if let TrainingSessionElement::Set {
            exercise_id,
            side: element_side,
            ..
        } = &mut element
        {
            *exercise_id = replacement;
            *element_side = side;
        }
        element
    };
    match (laterality, group) {
        (
            Some(Laterality::Unilateral),
            [
                TrainingSessionElement::Set {
                    side: Side::Unset, ..
                },
            ],
        ) => vec![with_side(Side::Left), with_side(Side::Right)],
        (Some(Laterality::Bilateral), _) => vec![with_side(Side::Unset)],
        _ => group
            .iter()
            .cloned()
            .map(|mut element| {
                if let TrainingSessionElement::Set { exercise_id, .. } = &mut element {
                    *exercise_id = replacement;
                }
                element
            })
            .collect(),
    }
}

/// Returns how the sets of `exercise_id` among `elements` are performed, or `None` if there is no
/// such set.
fn sides_of(elements: &[TrainingSessionElement], exercise_id: ExerciseID) -> Option<Sides> {
    let sides = elements
        .iter()
        .filter_map(|element| match element {
            TrainingSessionElement::Set {
                exercise_id: id,
                side,
                ..
            } if *id == exercise_id => Some(*side),
            TrainingSessionElement::Set { .. } | TrainingSessionElement::Rest { .. } => None,
        })
        .collect::<Vec<_>>();
    if sides.is_empty() {
        None
    } else if sides.iter().any(|side| *side != Side::Unset) {
        Some(Sides::PerSide)
    } else {
        Some(Sides::Combined)
    }
}

/// An element tagged with its previous index, or with `None` if it is new.
type TaggedElement = (Option<usize>, TrainingSessionElement);

fn tagged(elements: &[TrainingSessionElement], start: usize) -> Vec<TaggedElement> {
    elements
        .iter()
        .enumerate()
        .map(|(idx, element)| (Some(start + idx), element.clone()))
        .collect()
}

fn untagged(
    elements: impl IntoIterator<Item = TrainingSessionElement>,
) -> impl Iterator<Item = TaggedElement> {
    elements.into_iter().map(|element| (None, element))
}

fn section_start(sections: &[TrainingSessionSection], section_idx: usize) -> usize {
    sections[..section_idx]
        .iter()
        .map(|section| section.elements().len())
        .sum()
}

fn new_sets(exercise_id: ExerciseID, sides: Sides) -> Vec<TrainingSessionElement> {
    let set = |side| TrainingSessionElement::Set {
        exercise_id,
        side,
        reps: Reps::default(),
        time: Time::default(),
        weight: Weight::default(),
        rpe: RPE::default(),
        target_reps: Reps::default(),
        target_tempo: Tempo::default(),
        target_weight: Weight::default(),
        target_rpe: RPE::default(),
        automatic: false,
    };
    sides.sides().iter().map(|side| set(*side)).collect()
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
    use assert_approx_eq::assert_approx_eq;
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
                    side: Side::Unset,
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
                    side: Side::Unset,
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
                    side: Side::Unset,
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
                        side: Side::Unset,
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
    #[case(&*TRAINING_SESSION, 10.0)]
    #[case(&*EMPTY_TRAINING_SESSION, 0.0)]
    fn test_training_session_load(
        #[case] training_session: &TrainingSession,
        #[case] expected: f32,
    ) {
        assert_approx_eq!(training_session.load(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, 2.0)]
    #[case(&*EMPTY_TRAINING_SESSION, 0.0)]
    fn test_training_session_set_volume(
        #[case] training_session: &TrainingSession,
        #[case] expected: f32,
    ) {
        assert_approx_eq!(training_session.set_volume(), expected);
    }

    #[test]
    fn test_training_session_set_volume_treats_unset_rpe_as_maximum() {
        let training_session =
            training_session(&[set(1, 5, 100.0, RPE::ZERO), set(1, 5, 100.0, RPE::FOUR)]);
        assert_approx_eq!(training_session.set_volume(), 1.0);
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
    #[case::nothing(Time::default(), Tempo::default(), false)]
    #[case::recorded_time(Time::new(30).unwrap(), Tempo::default(), true)]
    #[case::prescribed_tempo(Time::default(), Tempo::new(&[3, 1, 1, 0]).unwrap(), true)]
    fn test_training_session_has_time(
        #[case] time: Time,
        #[case] target_tempo: Tempo,
        #[case] expected: bool,
    ) {
        let TrainingSessionElement::Set {
            exercise_id,
            reps,
            weight,
            rpe,
            target_reps,
            target_weight,
            target_rpe,
            automatic,
            ..
        } = set(1, 5, 100.0, RPE::ZERO)
        else {
            unreachable!()
        };
        let training_session = training_session(&[
            rest(60),
            TrainingSessionElement::Set {
                exercise_id,
                side: Side::Unset,
                reps,
                time,
                weight,
                rpe,
                target_reps,
                target_tempo,
                target_weight,
                target_rpe,
                automatic,
            },
        ]);

        assert_eq!(training_session.has_time(), expected);
    }

    #[rstest]
    #[case::nothing(RPE::ZERO, RPE::ZERO, false)]
    #[case::recorded_rpe(RPE::EIGHT, RPE::ZERO, true)]
    #[case::prescribed_rpe(RPE::ZERO, RPE::NINE, true)]
    fn test_training_session_has_rpe(
        #[case] rpe: RPE,
        #[case] target_rpe: RPE,
        #[case] expected: bool,
    ) {
        let TrainingSessionElement::Set {
            exercise_id,
            reps,
            time,
            weight,
            target_reps,
            target_tempo,
            target_weight,
            automatic,
            ..
        } = set(1, 5, 100.0, RPE::ZERO)
        else {
            unreachable!()
        };
        let training_session = training_session(&[
            rest(60),
            TrainingSessionElement::Set {
                exercise_id,
                side: Side::Unset,
                reps,
                time,
                weight,
                rpe,
                target_reps,
                target_tempo,
                target_weight,
                target_rpe,
                automatic,
            },
        ]);

        assert_eq!(training_session.has_rpe(), expected);
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
                side: Side::Unset,
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
                    side: Side::Unset,
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
                    side: Side::Unset,
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
        let map = training_session.move_section_up(1);
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
        assert_eq!(*map, [2, 3, 0, 1, 4].map(ElementMove::Kept));
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
        training_session.add_exercise(0, 2.into(), None);
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
        training_session.add_exercise(2, 2.into(), None);
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
        training_session.add_exercise(0, 3.into(), None);
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
        training_session.add_exercise(1, 3.into(), None);
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
        training_session.add_exercise(0, 1.into(), None);
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
        training_session.replace_exercise(0, 0.into(), 2.into(), None);
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
        training_session.replace_exercise(2, 0.into(), 2.into(), None);
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
        training_session.replace_exercise(0, 0.into(), 3.into(), None);
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
        training_session.replace_exercise(0, 0.into(), 3.into(), None);
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
    fn test_training_session_replace_exercise_repeated_in_a_run() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 0),
            exercise(2, 1),
            rest(0),
            exercise(3, 0),
            exercise(4, 0),
            exercise(5, 1),
            rest(1),
        ]);
        training_session.replace_exercise(0, 1.into(), 2.into(), None);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                exercise(1, 0),
                exercise(2, 2),
                rest(0),
                exercise(3, 0),
                exercise(4, 0),
                exercise(5, 2),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_training_session_replace_exercise_in_a_round_split_by_a_rest() {
        let mut training_session = training_session(&[
            exercise(0, 0),
            exercise(1, 1),
            rest(0),
            exercise(2, 0),
            rest(1),
            exercise(3, 1),
            rest(2),
        ]);
        training_session.replace_exercise(0, 1.into(), 2.into(), None);
        assert_eq!(
            training_session.elements,
            vec![
                exercise(0, 0),
                exercise(1, 2),
                rest(0),
                exercise(2, 0),
                rest(1),
                exercise(3, 2),
                rest(2),
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
        training_session.replace_exercise(1, 2.into(), 3.into(), None);
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
        let map = training_session.remove_set(0);
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
        assert_eq!(
            *map,
            [
                ElementMove::Kept(0),
                ElementMove::Kept(1),
                ElementMove::Removed(2),
                ElementMove::Removed(2),
                ElementMove::Kept(2),
                ElementMove::Kept(3),
                ElementMove::Kept(4),
                ElementMove::Kept(5),
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
        let map = training_session.remove_set(1);
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
        assert_eq!(map[6..], [ElementMove::Removed(6), ElementMove::Removed(6)]);
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
        training_session.remove_exercise(0, 0.into());
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
        training_session.remove_exercise(2, 0.into());
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
        training_session.remove_exercise(0, 0.into());
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
        training_session.remove_exercise(0, 0.into());
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
        training_session.remove_exercise(1, 2.into());
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
        training_session.append_exercise(1.into(), None);
        assert_eq!(training_session.elements, vec![exercise(0, 1)]);
    }

    #[test]
    fn test_training_session_append_exercise_same() {
        let mut training_session = training_session(&[exercise(0, 1)]);
        training_session.append_exercise(1.into(), None);
        assert_eq!(
            training_session.elements,
            vec![exercise(0, 1), rest(0), exercise(0, 1)]
        );
    }

    #[test]
    fn test_training_session_append_exercise_different() {
        let mut training_session = training_session(&[exercise(0, 1)]);
        training_session.append_exercise(2.into(), None);
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

    fn grouped_recent_set(
        group_index: usize,
        side: Side,
        reps: u32,
    ) -> (usize, TrainingSessionElement) {
        (group_index, sided_set(1, side, reps, RPE::ZERO))
    }

    fn offered_sets(sets: &[(usize, TrainingSessionElement)], side: Side) -> OfferedSets {
        let sets = sets
            .iter()
            .map(|(group_index, element)| (*group_index, element))
            .collect::<Vec<_>>();
        OfferedSets::new(&sets, side)
    }

    fn reps_set(reps: u32) -> Set {
        sided_set(1, Side::Unset, reps, RPE::ZERO).set().unwrap()
    }

    #[test]
    fn test_offered_sets_of_a_set_with_a_side_are_the_recent_sets_of_its_side() {
        let offered = offered_sets(
            &[
                grouped_recent_set(0, Side::Left, 1),
                grouped_recent_set(0, Side::Right, 2),
                grouped_recent_set(1, Side::Left, 3),
                grouped_recent_set(1, Side::Right, 4),
            ],
            Side::Right,
        );

        assert_eq!(offered.sets(), [reps_set(2), reps_set(4)]);
        assert_eq!(offered.index(1, 3), Some(1));
    }

    #[test]
    fn test_offered_sets_of_a_set_with_a_side_in_a_recent_session_without_sides_follow_its_position()
     {
        let offered = offered_sets(
            &[
                grouped_recent_set(0, Side::Unset, 1),
                grouped_recent_set(1, Side::Unset, 2),
                grouped_recent_set(2, Side::Unset, 3),
                grouped_recent_set(3, Side::Unset, 4),
            ],
            Side::Left,
        );

        assert_eq!(
            offered.sets(),
            [reps_set(1), reps_set(2), reps_set(3), reps_set(4)]
        );
        assert_eq!(offered.index(1, 2), Some(2));
    }

    #[test]
    fn test_offered_sets_of_a_set_without_a_side_are_the_recent_left_sets() {
        let offered = offered_sets(
            &[
                grouped_recent_set(0, Side::Left, 1),
                grouped_recent_set(0, Side::Right, 2),
                grouped_recent_set(1, Side::Left, 3),
                grouped_recent_set(1, Side::Right, 4),
            ],
            Side::Unset,
        );

        assert_eq!(offered.sets(), [reps_set(1), reps_set(3)]);
        assert_eq!(offered.index(1, 1), Some(1));
    }

    #[rstest]
    #[case::group_missing(Side::Left, 1)]
    #[case::position_missing(Side::Unset, 1)]
    fn test_offered_sets_hold_no_set_corresponding_to_a_set_missing_in_the_recent_session(
        #[case] side: Side,
        #[case] index: usize,
    ) {
        let offered = offered_sets(&[grouped_recent_set(0, side, 1)], side);

        assert_eq!(offered.index(index, index), None);
    }

    #[test]
    fn test_training_session_compute_sections_keeps_a_pair_together() {
        let elements = [
            set(2, 5, 0.0, RPE::ZERO),
            sided_set(1, Side::Left, 5, RPE::ZERO),
            rest(60),
            set(2, 5, 0.0, RPE::ZERO),
            sided_set(1, Side::Left, 5, RPE::ZERO),
            sided_set(1, Side::Right, 5, RPE::ZERO),
        ];

        assert_eq!(
            training_session(&elements).compute_sections(),
            vec![section(&elements)]
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
    fn test_best_one_rep_max_within_ignores_sessions_outside_the_dates() {
        let session = |days_ago, weight| {
            let mut session = training_session(&[set(1, 1, weight, RPE::ZERO)]);
            session.date = *TODAY - Duration::days(days_ago);
            session
        };
        assert_eq!(
            best_one_rep_max_within(
                &[
                    session(40, 200.0),
                    session(30, 100.0),
                    session(10, 110.0),
                    session(0, 300.0)
                ],
                1.into(),
                *TODAY - Duration::days(30)..=*TODAY - Duration::days(10)
            ),
            Some(110.0)
        );
    }

    #[test]
    fn test_best_one_rep_max_within_takes_the_best_over_sessions_and_sets() {
        let mut a = training_session(&[set(1, 1, 100.0, RPE::ZERO), set(1, 1, 110.0, RPE::ZERO)]);
        a.date = *TODAY - Duration::days(5);
        let mut b = training_session(&[set(1, 1, 105.0, RPE::ZERO), set(2, 1, 300.0, RPE::ZERO)]);
        b.date = *TODAY;
        assert_eq!(
            best_one_rep_max_within(&[a, b], 1.into(), *TODAY - Duration::days(10)..=*TODAY),
            Some(110.0)
        );
    }

    #[test]
    fn test_best_one_rep_max_within_without_matching_sets() {
        let session = training_session(&[set(2, 5, 100.0, RPE::ZERO)]);
        assert_eq!(
            best_one_rep_max_within(&[session], 1.into(), *TODAY - Duration::days(10)..=*TODAY),
            None
        );
        assert_eq!(
            best_one_rep_max_within(&[], 1.into(), *TODAY - Duration::days(10)..=*TODAY),
            None
        );
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
    #[case::first_round(0, 1, 0, vec![vec![0]])]
    #[case::rest_belongs_to_the_following_round(0, 1, 1, vec![vec![2]])]
    #[case::second_round(0, 1, 2, vec![vec![2]])]
    #[case::trailing_rest_falls_back_to_the_first_round(0, 1, 3, vec![vec![0]])]
    #[case::before_the_section_falls_back_to_the_first_round(1, 2, 0, vec![vec![4]])]
    #[case::after_the_section_falls_back_to_the_first_round(0, 1, 99, vec![vec![0]])]
    fn test_training_session_run_element_indices(
        #[case] section_idx: usize,
        #[case] exercise_id: u128,
        #[case] element_idx: usize,
        #[case] expected: Vec<Vec<usize>>,
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
    #[case::first_exercise(1, 3, vec![vec![3]])]
    #[case::second_exercise(2, 0, vec![vec![1]])]
    fn test_training_session_run_element_indices_of_a_superset(
        #[case] exercise_id: u128,
        #[case] element_idx: usize,
        #[case] expected: Vec<Vec<usize>>,
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

        assert_eq!(
            ts.run_element_indices(0, 1.into(), 0),
            Vec::<Vec<usize>>::new()
        );
        assert_eq!(ts.run_element_indices(1, 1.into(), 1), vec![vec![1]]);
        assert_eq!(ts.run_element_indices(1, 1.into(), 2), vec![vec![3]]);
    }

    #[test]
    fn test_training_session_run_element_indices_beyond_the_last_section() {
        let ts = training_session(&[exercise(0, 1)]);

        assert_eq!(
            ts.run_element_indices(1, 1.into(), 0),
            Vec::<Vec<usize>>::new()
        );
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

    #[rstest]
    #[case::full_pair(
        &[sided_set(1, Side::Left, 5, RPE::EIGHT), sided_set(1, Side::Right, 5, RPE::EIGHT)],
        vec![vec![0, 1]]
    )]
    #[case::pair_whose_left_side_is_empty(
        &[sided_set(1, Side::Left, 0, RPE::ZERO), sided_set(1, Side::Right, 5, RPE::EIGHT)],
        vec![vec![0, 1]]
    )]
    #[case::right_set_directly_followed_by_its_left_set(
        &[sided_set(1, Side::Right, 5, RPE::EIGHT), sided_set(1, Side::Left, 5, RPE::EIGHT)],
        vec![vec![0, 1]]
    )]
    #[case::alternating_sides(
        &[
            sided_set(1, Side::Left, 5, RPE::ZERO),
            sided_set(1, Side::Right, 5, RPE::ZERO),
            sided_set(1, Side::Left, 6, RPE::ZERO),
            sided_set(1, Side::Right, 6, RPE::ZERO),
        ],
        vec![vec![0, 1], vec![2, 3]]
    )]
    #[case::two_right_sets_followed_by_a_left_set(
        &[
            sided_set(1, Side::Right, 5, RPE::ZERO),
            sided_set(1, Side::Right, 6, RPE::ZERO),
            sided_set(1, Side::Left, 6, RPE::ZERO),
        ],
        vec![vec![0], vec![1, 2]]
    )]
    #[case::sides_of_different_exercises(
        &[sided_set(1, Side::Left, 5, RPE::ZERO), sided_set(2, Side::Right, 5, RPE::ZERO)],
        vec![vec![0], vec![1]]
    )]
    #[case::sides_split_by_another_exercise(
        &[sided_set(1, Side::Left, 5, RPE::ZERO), set(2, 5, 0.0, RPE::ZERO), sided_set(1, Side::Right, 5, RPE::ZERO)],
        vec![vec![0], vec![1], vec![2]]
    )]
    #[case::sides_split_by_a_rest(
        &[sided_set(1, Side::Left, 5, RPE::ZERO), rest(60), sided_set(1, Side::Right, 5, RPE::ZERO)],
        vec![vec![0], vec![2]]
    )]
    #[case::two_left_sets_followed_by_two_right_sets(
        &[
            sided_set(1, Side::Left, 5, RPE::ZERO),
            sided_set(1, Side::Left, 6, RPE::ZERO),
            sided_set(1, Side::Right, 5, RPE::ZERO),
            sided_set(1, Side::Right, 6, RPE::ZERO),
        ],
        vec![vec![0], vec![1, 2], vec![3]]
    )]
    #[case::left_set_of_an_earlier_section(
        &[
            sided_set(1, Side::Left, 5, RPE::ZERO),
            set(2, 5, 0.0, RPE::ZERO),
            rest(60),
            sided_set(1, Side::Left, 5, RPE::ZERO),
            sided_set(1, Side::Right, 5, RPE::ZERO),
        ],
        vec![vec![0], vec![1], vec![3, 4]]
    )]
    #[case::side_less_sets(
        &[set(1, 5, 0.0, RPE::ZERO), set(1, 5, 0.0, RPE::ZERO)],
        vec![vec![0], vec![1]]
    )]
    fn test_training_session_paired_sets(
        #[case] elements: &[TrainingSessionElement],
        #[case] expected: Vec<Vec<usize>>,
    ) {
        let session = training_session(elements);

        assert_eq!(
            session
                .paired_sets()
                .iter()
                .map(|group| group
                    .iter()
                    .map(|set| session
                        .elements
                        .iter()
                        .position(|element| std::ptr::eq(element, set))
                        .unwrap())
                    .collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[rstest]
    #[case::full_pair(&[sided_set(1, Side::Left, 5, RPE::EIGHT), sided_set(1, Side::Right, 5, RPE::EIGHT)], 1.0)]
    #[case::pair_whose_left_side_is_empty(&[sided_set(1, Side::Left, 0, RPE::ZERO), sided_set(1, Side::Right, 5, RPE::EIGHT)], 0.5)]
    #[case::unpaired_right_set(&[sided_set(1, Side::Right, 5, RPE::EIGHT)], 0.5)]
    #[case::pair_with_one_side_below_seven(&[sided_set(1, Side::Left, 5, RPE::SIX), sided_set(1, Side::Right, 5, RPE::EIGHT)], 0.5)]
    #[case::pair_with_both_sides_below_seven(&[sided_set(1, Side::Left, 5, RPE::SIX), sided_set(1, Side::Right, 5, RPE::SIX)], 0.0)]
    #[case::sides_split_by_another_exercise(&[sided_set(1, Side::Left, 5, RPE::ZERO), set(2, 5, 0.0, RPE::ZERO), sided_set(1, Side::Right, 5, RPE::ZERO)], 2.0)]
    #[case::side_less_sets(&[set(1, 5, 0.0, RPE::ZERO), set(1, 5, 0.0, RPE::ZERO)], 2.0)]
    fn test_training_session_set_volume_of_sets_with_a_side(
        #[case] elements: &[TrainingSessionElement],
        #[case] expected: f32,
    ) {
        assert_approx_eq!(training_session(elements).set_volume(), expected);
    }

    #[test]
    fn test_training_session_stimulus_per_muscle_counts_a_set_with_a_side_half() {
        let exercises = [Exercise {
            id: 1.into(),
            name: Name::new("A").unwrap(),
            notes: String::new(),
            muscles: vec![ExerciseMuscle {
                muscle_id: MuscleID::Quads,
                stimulus: Stimulus::PRIMARY,
            }],
            force: None,
            mechanic: None,
            laterality: Some(Laterality::Unilateral),
            assistance: None,
            equipment: vec![],
            category: None,
        }];
        let session = training_session(&[
            sided_set(1, Side::Left, 5, RPE::EIGHT),
            sided_set(1, Side::Right, 5, RPE::EIGHT),
            sided_set(1, Side::Left, 5, RPE::SIX),
            sided_set(1, Side::Right, 5, RPE::EIGHT),
        ]);

        assert_eq!(
            session.stimulus_per_muscle(&exercises),
            BTreeMap::from([(MuscleID::Quads, Stimulus::PRIMARY * 3 / 2)])
        );
    }

    #[test]
    fn test_training_session_load_of_sets_with_a_side() {
        let session = training_session(&[
            sided_set(1, Side::Left, 5, RPE::EIGHT),
            sided_set(1, Side::Right, 5, RPE::NINE),
            sided_set(1, Side::Left, 5, RPE::ZERO),
        ]);

        assert_approx_eq!(session.load(), 12.5);
    }

    #[rstest]
    #[case::different_times(3, 4, Some(18))]
    #[case::empty_side(0, 4, Some(10))]
    #[case::no_time(0, 0, None)]
    fn test_training_session_tut_of_sets_with_a_side(
        #[case] left_time: u32,
        #[case] right_time: u32,
        #[case] expected: Option<u32>,
    ) {
        let timed = |side, time| TrainingSessionElement::Set {
            exercise_id: 1.into(),
            side,
            reps: Reps::new(5).unwrap(),
            time: Time::new(time).unwrap(),
            weight: Weight::default(),
            rpe: RPE::ZERO,
            target_reps: Reps::default(),
            target_tempo: Tempo::default(),
            target_weight: Weight::default(),
            target_rpe: RPE::default(),
            automatic: false,
        };
        let session =
            training_session(&[timed(Side::Left, left_time), timed(Side::Right, right_time)]);

        assert_eq!(session.tut(), expected);
    }

    #[test]
    fn test_training_session_group_indices_count_a_pair_once() {
        let session = training_session(&[
            sided_set(1, Side::Left, 5, RPE::ZERO),
            sided_set(1, Side::Right, 5, RPE::ZERO),
            set(2, 5, 0.0, RPE::ZERO),
            rest(60),
            sided_set(1, Side::Left, 5, RPE::ZERO),
            sided_set(1, Side::Right, 5, RPE::ZERO),
            set(1, 5, 0.0, RPE::ZERO),
            sided_set(1, Side::Right, 5, RPE::ZERO),
        ]);

        assert_eq!(
            session.group_indices(),
            HashMap::from([(0, 0), (1, 0), (2, 0), (4, 1), (5, 1), (6, 2), (7, 3)])
        );
    }

    #[rstest]
    #[case::unilateral(Some(Laterality::Unilateral), vec![sided_set(2, Side::Left, 0, RPE::ZERO), sided_set(2, Side::Right, 0, RPE::ZERO)])]
    #[case::bilateral(Some(Laterality::Bilateral), vec![set(2, 0, 0.0, RPE::ZERO)])]
    fn test_training_session_add_exercise_of_a_laterality(
        #[case] laterality: Option<Laterality>,
        #[case] added: Vec<TrainingSessionElement>,
    ) {
        let mut session = training_session(&[
            set(1, 5, 0.0, RPE::ZERO),
            rest(60),
            set(1, 5, 0.0, RPE::ZERO),
            rest(60),
        ]);

        session.add_exercise(0, 2.into(), laterality);

        let round = [vec![set(1, 5, 0.0, RPE::ZERO)], added, vec![rest(60)]].concat();
        assert_eq!(session.elements, [round.clone(), round].concat());
    }

    #[test]
    fn test_training_session_add_exercise_held_side_less() {
        let mut session = training_session(&[set(1, 5, 0.0, RPE::ZERO), rest(60)]);

        session.add_exercise(0, 1.into(), Some(Laterality::Unilateral));

        assert_eq!(
            session.elements,
            [
                set(1, 5, 0.0, RPE::ZERO),
                set(1, 0, 0.0, RPE::ZERO),
                rest(60)
            ]
        );
    }

    #[rstest]
    #[case::unilateral(Some(Laterality::Unilateral), vec![sided_set(2, Side::Left, 0, RPE::ZERO), sided_set(2, Side::Right, 0, RPE::ZERO)])]
    #[case::bilateral(Some(Laterality::Bilateral), vec![set(2, 0, 0.0, RPE::ZERO)])]
    fn test_training_session_append_exercise_of_a_laterality(
        #[case] laterality: Option<Laterality>,
        #[case] appended: Vec<TrainingSessionElement>,
    ) {
        let mut session = training_session(&[set(1, 5, 0.0, RPE::ZERO)]);

        session.append_exercise(2.into(), laterality);

        assert_eq!(
            session.elements,
            [vec![set(1, 5, 0.0, RPE::ZERO), rest(0)], appended].concat()
        );
    }

    #[test]
    fn test_training_session_append_exercise_held_side_less() {
        let mut session = training_session(&[set(1, 5, 0.0, RPE::ZERO), rest(60)]);

        session.append_exercise(1.into(), Some(Laterality::Unilateral));

        assert_eq!(
            session.elements,
            [
                set(1, 5, 0.0, RPE::ZERO),
                rest(60),
                set(1, 0, 0.0, RPE::ZERO)
            ]
        );
    }

    #[test]
    fn test_training_session_replace_exercise_of_side_less_sets_by_a_unilateral_exercise() {
        let target = |exercise_id: u128, side| TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            side,
            reps: Reps::new(5).unwrap(),
            time: Time::default(),
            weight: Weight::new(20.0).unwrap(),
            rpe: RPE::EIGHT,
            target_reps: Reps::new(6).unwrap(),
            target_tempo: Tempo::default(),
            target_weight: Weight::new(22.5).unwrap(),
            target_rpe: RPE::NINE,
            automatic: true,
        };
        let mut session = training_session(&[target(1, Side::Unset), rest(60)]);

        let moved_to =
            session.replace_exercise(0, 1.into(), 2.into(), Some(Laterality::Unilateral));

        assert_eq!(
            session.elements,
            [target(2, Side::Left), target(2, Side::Right), rest(60)]
        );
        assert_eq!(*moved_to, [ElementMove::Kept(0), ElementMove::Kept(2)]);
    }

    #[rstest]
    #[case::unilateral_after_an_unpaired_set(
        &[sided_set(1, Side::Right, 5, RPE::ZERO), set(1, 6, 0.0, RPE::ZERO), rest(60)],
        Some(Laterality::Unilateral),
        &[
            sided_set(2, Side::Right, 5, RPE::ZERO),
            sided_set(2, Side::Left, 6, RPE::ZERO),
            sided_set(2, Side::Right, 6, RPE::ZERO),
            rest(60),
        ]
    )]
    #[case::superset_partner_before(
        &[sided_set(2, Side::Right, 5, RPE::ZERO), sided_set(1, Side::Left, 6, RPE::ZERO), rest(60)],
        None,
        &[sided_set(2, Side::Right, 5, RPE::ZERO), sided_set(2, Side::Left, 6, RPE::ZERO), rest(60)]
    )]
    #[case::superset_partner_after(
        &[sided_set(1, Side::Left, 5, RPE::ZERO), sided_set(2, Side::Right, 6, RPE::ZERO), rest(60)],
        None,
        &[sided_set(2, Side::Left, 5, RPE::ZERO), sided_set(2, Side::Right, 6, RPE::ZERO), rest(60)]
    )]
    fn test_training_session_replace_exercise_pairs_adjacent_opposite_sides(
        #[case] elements: &[TrainingSessionElement],
        #[case] laterality: Option<Laterality>,
        #[case] expected: &[TrainingSessionElement],
    ) {
        let mut session = training_session(elements);

        session.replace_exercise(0, 1.into(), 2.into(), laterality);

        assert_eq!(session.elements, expected);
        assert!(session.completes_pair(1));
    }

    #[rstest]
    #[case::bilateral(
        Some(Laterality::Bilateral),
        &[sided_set(2, Side::Unset, 5, RPE::ZERO), rest(60)],
        &[ElementMove::Kept(0), ElementMove::Removed(1), ElementMove::Kept(1)]
    )]
    #[case::unset(
        None,
        &[sided_set(2, Side::Left, 5, RPE::ZERO), sided_set(2, Side::Right, 6, RPE::ZERO), rest(60)],
        &[ElementMove::Kept(0), ElementMove::Kept(1), ElementMove::Kept(2)]
    )]
    #[case::unilateral(
        Some(Laterality::Unilateral),
        &[sided_set(2, Side::Left, 5, RPE::ZERO), sided_set(2, Side::Right, 6, RPE::ZERO), rest(60)],
        &[ElementMove::Kept(0), ElementMove::Kept(1), ElementMove::Kept(2)]
    )]
    fn test_training_session_replace_exercise_of_a_pair(
        #[case] laterality: Option<Laterality>,
        #[case] expected: &[TrainingSessionElement],
        #[case] expected_moved_to: &[ElementMove],
    ) {
        let mut session = training_session(&[
            sided_set(1, Side::Left, 5, RPE::ZERO),
            sided_set(1, Side::Right, 6, RPE::ZERO),
            rest(60),
        ]);

        let moved_to = session.replace_exercise(0, 1.into(), 2.into(), laterality);

        assert_eq!(session.elements, expected);
        assert_eq!(*moved_to, expected_moved_to);
    }

    #[rstest]
    #[case::unilateral(Some(Laterality::Unilateral))]
    #[case::unset(None)]
    fn test_training_session_replace_exercise_of_an_unpaired_set_keeps_its_side(
        #[case] laterality: Option<Laterality>,
    ) {
        let mut session = training_session(&[sided_set(1, Side::Right, 5, RPE::ZERO), rest(60)]);

        session.replace_exercise(0, 1.into(), 2.into(), laterality);

        assert_eq!(
            session.elements,
            [sided_set(2, Side::Right, 5, RPE::ZERO), rest(60)]
        );
    }

    #[test]
    fn test_training_session_replace_exercise_moves_the_elements_of_later_sections() {
        let mut session = training_session(&[
            set(3, 5, 0.0, RPE::ZERO),
            rest(60),
            set(1, 5, 0.0, RPE::ZERO),
            rest(60),
            set(1, 6, 0.0, RPE::ZERO),
            rest(60),
            set(3, 5, 0.0, RPE::ZERO),
        ]);

        let moved_to =
            session.replace_exercise(1, 1.into(), 2.into(), Some(Laterality::Unilateral));

        assert_eq!(*moved_to, [0, 1, 2, 4, 5, 7, 8].map(ElementMove::Kept));
        assert_eq!(session.elements[8], set(3, 5, 0.0, RPE::ZERO));
    }

    #[test]
    fn test_training_session_replace_exercise_keeps_the_first_set_of_a_pair() {
        let mut session = training_session(&[
            sided_set(1, Side::Right, 5, RPE::ZERO),
            sided_set(1, Side::Left, 6, RPE::ZERO),
            rest(60),
        ]);

        session.replace_exercise(0, 1.into(), 2.into(), Some(Laterality::Bilateral));

        assert_eq!(
            session.elements,
            [sided_set(2, Side::Unset, 5, RPE::ZERO), rest(60)]
        );
    }

    #[test]
    fn test_training_session_replace_exercise_keeps_the_recorded_set_of_a_pair() {
        let mut session = training_session(&[
            sided_set(1, Side::Left, 0, RPE::ZERO),
            sided_set(1, Side::Right, 6, RPE::ZERO),
            rest(60),
        ]);

        session.replace_exercise(0, 1.into(), 2.into(), Some(Laterality::Bilateral));

        assert_eq!(
            session.elements,
            [sided_set(2, Side::Unset, 6, RPE::ZERO), rest(60)]
        );
    }

    #[test]
    fn test_training_session_replace_exercise_of_unpaired_sides_by_a_bilateral_exercise() {
        let mut session = training_session(&[
            sided_set(1, Side::Left, 5, RPE::ZERO),
            set(3, 5, 0.0, RPE::ZERO),
            sided_set(1, Side::Right, 6, RPE::ZERO),
            rest(60),
        ]);

        session.replace_exercise(0, 1.into(), 2.into(), Some(Laterality::Bilateral));

        assert_eq!(
            session.elements,
            [
                sided_set(2, Side::Unset, 5, RPE::ZERO),
                set(3, 5, 0.0, RPE::ZERO),
                sided_set(2, Side::Unset, 6, RPE::ZERO),
                rest(60)
            ]
        );
    }

    #[test]
    fn test_training_session_remove_exercise_of_a_pair_in_a_superset() {
        let mut session = training_session(&[
            sided_set(1, Side::Left, 5, RPE::ZERO),
            sided_set(1, Side::Right, 5, RPE::ZERO),
            set(2, 5, 0.0, RPE::ZERO),
            rest(60),
            sided_set(1, Side::Left, 5, RPE::ZERO),
            sided_set(1, Side::Right, 5, RPE::ZERO),
            set(2, 5, 0.0, RPE::ZERO),
            rest(60),
        ]);

        session.remove_exercise(0, 1.into());

        assert_eq!(
            session.elements,
            [
                set(2, 5, 0.0, RPE::ZERO),
                rest(60),
                set(2, 5, 0.0, RPE::ZERO),
                rest(60)
            ]
        );
    }

    #[test]
    fn test_training_session_remove_exercise_of_a_lone_pair_removes_the_section() {
        let mut session = training_session(&[
            set(2, 5, 0.0, RPE::ZERO),
            rest(60),
            sided_set(1, Side::Left, 5, RPE::ZERO),
            sided_set(1, Side::Right, 5, RPE::ZERO),
            rest(60),
        ]);

        session.remove_exercise(1, 1.into());

        assert_eq!(session.elements, [set(2, 5, 0.0, RPE::ZERO), rest(60)]);
    }

    #[test]
    fn test_training_session_add_set_of_a_per_side_section() {
        let mut session = training_session(&[
            sided_set(1, Side::Left, 5, RPE::ZERO),
            sided_set(1, Side::Right, 5, RPE::ZERO),
            rest(60),
        ]);

        session.add_set(0);

        assert_eq!(
            session.elements,
            [
                sided_set(1, Side::Left, 5, RPE::ZERO),
                sided_set(1, Side::Right, 5, RPE::ZERO),
                rest(60),
                sided_set(1, Side::Left, 0, RPE::ZERO),
                sided_set(1, Side::Right, 0, RPE::ZERO),
                rest(60),
            ]
        );
    }

    #[rstest]
    #[case::pair(1, vec![vec![0, 1], vec![3]])]
    #[case::superset_partner(2, vec![vec![2]])]
    fn test_training_session_run_element_indices_of_a_run_holding_a_pair(
        #[case] exercise_id: u128,
        #[case] expected: Vec<Vec<usize>>,
    ) {
        let session = training_session(&[
            sided_set(1, Side::Left, 5, RPE::ZERO),
            sided_set(1, Side::Right, 5, RPE::ZERO),
            set(2, 5, 0.0, RPE::ZERO),
            set(1, 5, 0.0, RPE::ZERO),
        ]);

        assert_eq!(
            session.run_element_indices(0, exercise_id.into(), 0),
            expected
        );
    }

    #[test]
    fn test_training_session_section_groups() {
        let elements = [
            sided_set(1, Side::Right, 5, RPE::ZERO),
            sided_set(1, Side::Left, 6, RPE::ZERO),
            sided_set(1, Side::Left, 7, RPE::ZERO),
            set(2, 5, 0.0, RPE::ZERO),
            sided_set(1, Side::Right, 8, RPE::ZERO),
            set(1, 5, 0.0, RPE::ZERO),
            set(1, 5, 0.0, RPE::ZERO),
            rest(60),
        ];

        assert_eq!(
            section(&elements).groups(),
            [
                ElementGroup::Sides {
                    left: Some(&elements[1]),
                    right: Some(&elements[0])
                },
                ElementGroup::Sides {
                    left: Some(&elements[2]),
                    right: None
                },
                ElementGroup::Single(&elements[3]),
                ElementGroup::Sides {
                    left: None,
                    right: Some(&elements[4])
                },
                ElementGroup::Single(&elements[5]),
                ElementGroup::Single(&elements[6]),
                ElementGroup::Single(&elements[7]),
            ]
        );
    }

    #[rstest]
    #[case::first_set_of_a_pair(0, false)]
    #[case::second_set_of_a_pair(1, true)]
    #[case::opposite_side_after_a_pair(2, false)]
    #[case::rest(3, false)]
    fn test_training_session_completes_pair(#[case] element_idx: usize, #[case] expected: bool) {
        let session = training_session(&[
            sided_set(1, Side::Right, 5, RPE::ZERO),
            sided_set(1, Side::Left, 6, RPE::ZERO),
            sided_set(1, Side::Right, 7, RPE::ZERO),
            rest(60),
        ]);

        assert_eq!(session.completes_pair(element_idx), expected);
    }

    #[test]
    fn test_training_session_set_history_rows() {
        let session = training_session(&[
            sided_set(1, Side::Left, 5, RPE::ZERO),
            sided_set(1, Side::Right, 6, RPE::ZERO),
            set(2, 5, 0.0, RPE::ZERO),
            rest(60),
            sided_set(1, Side::Left, 7, RPE::ZERO),
            rest(60),
            set(3, 5, 0.0, RPE::ZERO),
            rest(60),
            sided_set(1, Side::Right, 8, RPE::ZERO),
            rest(60),
            set(1, 9, 0.0, RPE::ZERO),
            rest(60),
            sided_set(1, Side::Right, 10, RPE::ZERO),
            sided_set(1, Side::Left, 11, RPE::ZERO),
        ]);
        let values = |reps| Set {
            reps: Reps::new(reps).unwrap(),
            time: Time::default(),
            weight: Weight::default(),
            rpe: RPE::ZERO,
        };

        assert_eq!(
            session.set_history_rows(1.into()),
            vec![
                SetHistoryRow::Sides {
                    left: Some(values(5)),
                    right: Some(values(6)),
                },
                SetHistoryRow::Sides {
                    left: Some(values(7)),
                    right: None,
                },
                SetHistoryRow::Sides {
                    left: None,
                    right: Some(values(8)),
                },
                SetHistoryRow::Combined(values(9)),
                SetHistoryRow::Sides {
                    left: Some(values(11)),
                    right: Some(values(10)),
                },
            ]
        );
    }

    #[test]
    fn test_training_session_restricted_to_keeps_the_sets_of_the_exercise() {
        let session = training_session(&[
            sided_set(1, Side::Left, 5, RPE::ZERO),
            sided_set(1, Side::Right, 6, RPE::ZERO),
            set(2, 5, 0.0, RPE::ZERO),
            sided_set(1, Side::Left, 7, RPE::ZERO),
            set(2, 5, 0.0, RPE::ZERO),
            sided_set(1, Side::Right, 8, RPE::ZERO),
            rest(60),
            set(1, 9, 0.0, RPE::ZERO),
        ]);

        let restricted = session.restricted_to(1.into());

        assert_eq!(
            restricted.elements,
            vec![
                sided_set(1, Side::Left, 5, RPE::ZERO),
                sided_set(1, Side::Right, 6, RPE::ZERO),
                sided_set(1, Side::Left, 7, RPE::ZERO),
                sided_set(1, Side::Right, 8, RPE::ZERO),
                set(1, 9, 0.0, RPE::ZERO),
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
            side: Side::Unset,
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
            side: Side::Unset,
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

    fn sided_set(exercise_id: u128, side: Side, reps: u32, rpe: RPE) -> TrainingSessionElement {
        TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            side,
            reps: Reps::new(reps).unwrap(),
            time: Time::default(),
            weight: Weight::default(),
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
                    (previous.date, vec![(0, &previous.elements[0])]),
                    (earlier.date, vec![(0, &earlier.elements[0])]),
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
            Some(&vec![(earlier.date, vec![(0, &earlier.elements[0])])])
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
                vec![(earlier.date, vec![(0, &earlier.elements[0])])]
            )])
        );
    }

    #[test]
    fn test_get_recent_session_sets_by_exercise_with_group_indices() {
        let service = Service::new(FakeRepository::default());
        let current = dated_training_session(1, 1, *TODAY, &[set(1, 5, 100.0, RPE::ZERO)]);
        let earlier = dated_training_session(
            2,
            1,
            *TODAY - Duration::days(7),
            &[
                sided_set(1, Side::Left, 3, RPE::ZERO),
                sided_set(1, Side::Right, 4, RPE::ZERO),
                set(1, 0, 0.0, RPE::ZERO),
                rest(60),
                sided_set(1, Side::Left, 5, RPE::ZERO),
            ],
        );
        let training_sessions = [current.clone(), earlier.clone()];

        assert_eq!(
            service.get_recent_session_sets_by_exercise(&current, &training_sessions, 3),
            HashMap::from([(
                ExerciseID::from(1u128),
                vec![(
                    earlier.date,
                    vec![
                        (0, &earlier.elements[0]),
                        (0, &earlier.elements[1]),
                        (2, &earlier.elements[4])
                    ]
                )]
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
            side: Side::Unset,
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
            side: Side::Unset,
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
        prop::collection::vec(prop::option::of((1u128..4, any::<bool>())), 1..10)
            .prop_map(|slots| {
                let mut elements: Vec<TrainingSessionElement> = vec![];
                for slot in slots {
                    match slot {
                        Some((exercise_id, false)) => {
                            elements.push(set(exercise_id, 5, 100.0, RPE::ZERO));
                        }
                        Some((exercise_id, true)) => {
                            elements.push(sided_set(exercise_id, Side::Left, 5, RPE::ZERO));
                            elements.push(sided_set(exercise_id, Side::Right, 5, RPE::ZERO));
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
        fn test_training_session_mutations_keep_a_set_in_every_section_and_map_the_elements(
            session in any_session(),
            operations in prop::collection::vec((0u8..8, 0usize..8, 0usize..8), 0..8),
        ) {
            let mut session = session;

            for (operation, a, b) in operations {
                let sections = session.compute_sections();
                if sections.is_empty() {
                    continue;
                }
                let section_idx = a % sections.len();
                let exercise_ids = sections[section_idx].exercise_ids();
                let laterality = [None, Some(Laterality::Bilateral), Some(Laterality::Unilateral)][a % 3];
                let previous = session.clone();
                let mut replaced = None;
                let map = match operation {
                    0 => session.add_set(b % session.elements.len()),
                    1 => session.add_exercise(section_idx, (b as u128 % 3 + 1).into(), laterality),
                    2 if !exercise_ids.is_empty() => {
                        let exercise_id = exercise_ids[b % exercise_ids.len()];
                        replaced = Some(exercise_id);
                        session.replace_exercise(section_idx, exercise_id, (a as u128 % 3 + 1).into(), laterality)
                    }
                    3 => session.remove_set(section_idx),
                    4 if !exercise_ids.is_empty() => session.remove_exercise(section_idx, exercise_ids[b % exercise_ids.len()]),
                    5 => session.move_section_up(section_idx),
                    6 => session.move_section_down(section_idx),
                    7 => session.append_exercise((b as u128 % 3 + 1).into(), laterality),
                    _ => continue,
                };

                prop_assert!(!has_rest_only_section(&session));
                prop_assert_eq!(map.len(), previous.elements.len());
                let mut kept = HashSet::new();
                for (idx, element) in previous.elements.iter().enumerate() {
                    match map[idx] {
                        ElementMove::Kept(new_idx) => {
                            prop_assert!(kept.insert(new_idx));
                            if !matches!(element, TrainingSessionElement::Set { exercise_id, .. } if Some(*exercise_id) == replaced) {
                                prop_assert_eq!(&session.elements[new_idx], element);
                            }
                        }
                        ElementMove::Removed(following) => {
                            prop_assert!(following <= session.elements.len());
                        }
                    }
                }
                if !matches!(operation, 5 | 6) {
                    let indices = map.iter().map(|m| match m {
                        ElementMove::Kept(idx) | ElementMove::Removed(idx) => *idx,
                    });
                    prop_assert!(indices.is_sorted());
                }
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn test_training_session_replace_exercise_by_a_unilateral_and_back_by_a_bilateral_exercise(
            session in any_session(),
            a in 0usize..8,
            b in 0usize..8,
        ) {
            let sections = session.compute_sections();
            let section_idx = a % sections.len();
            let exercise_ids = sections[section_idx].exercise_ids();
            prop_assume!(!exercise_ids.is_empty());
            let exercise_id = exercise_ids[b % exercise_ids.len()];
            prop_assume!(sections[section_idx].elements().iter().all(|element| !matches!(
                element,
                TrainingSessionElement::Set { exercise_id: id, side, .. } if *id == exercise_id && *side != Side::Unset
            )));
            let mut replaced = session.clone();

            replaced.replace_exercise(section_idx, exercise_id, 9.into(), Some(Laterality::Unilateral));
            replaced.replace_exercise(section_idx, 9.into(), exercise_id, Some(Laterality::Bilateral));

            prop_assert_eq!(replaced, session);
        }
    }

    #[rstest]
    #[case::opening_exercise(0, &[exercise(1, 1), exercise(2, 2), rest(90), exercise(5, 1), rest(31), exercise(6, 2), rest(91)])]
    #[case::middle_exercise(1, &[exercise(0, 0), exercise(2, 2), rest(90), exercise(4, 0), rest(30), exercise(6, 2), rest(91)])]
    #[case::closing_exercise(2, &[exercise(0, 0), exercise(1, 1), rest(90), exercise(4, 0), rest(30), exercise(5, 1), rest(91)])]
    fn test_training_session_remove_exercise_keeps_the_rest_closing_a_round(
        #[case] exercise_id: u128,
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

        session.remove_exercise(0, exercise_id.into());

        assert_eq!(session.elements, expected);
    }

    #[test]
    fn test_training_session_remove_exercise_repeated_in_a_run_removes_one_set_per_run() {
        let mut session = training_session(&[
            exercise(0, 0),
            exercise(1, 0),
            exercise(2, 1),
            rest(0),
            exercise(3, 0),
            exercise(4, 0),
            exercise(5, 1),
            rest(1),
        ]);

        session.remove_exercise(0, 0.into());

        assert_eq!(
            session.elements,
            [
                exercise(0, 0),
                exercise(2, 1),
                rest(0),
                exercise(3, 0),
                exercise(5, 1),
                rest(1),
            ]
        );
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

        session.remove_exercise(0, 2.into());

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
