use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

use chrono::{Duration, NaiveDate};
use derive_more::{Deref, Display, From, Into};
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
        notes: String,
        sections: Vec<RoutinePart>,
    ) -> Result<Routine, CreateError>;
    async fn modify_routine(
        &self,
        id: RoutineID,
        name: Option<Name>,
        notes: Option<String>,
        archived: Option<bool>,
        sections: Option<Vec<RoutinePart>>,
    ) -> Result<Routine, UpdateError>;
    async fn delete_routine(&self, id: RoutineID) -> Result<(), DeleteError>;

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
        notes: String,
        sections: Vec<RoutinePart>,
    ) -> Result<Routine, CreateError>;
    async fn modify_routine(
        &self,
        id: RoutineID,
        name: Option<Name>,
        notes: Option<String>,
        archived: Option<bool>,
        sections: Option<Vec<RoutinePart>>,
    ) -> Result<Routine, UpdateError>;
    async fn delete_routine(&self, id: RoutineID) -> Result<(), DeleteError>;
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
    pub fn stimulus_per_muscle(&self, exercises: &[Exercise]) -> BTreeMap<MuscleID, Stimulus> {
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

    #[must_use]
    pub fn to_text(&self, exercises: &[Exercise], show_tut: bool, show_rpe: bool) -> String {
        let mut lines = vec![self.name.to_string()];
        if !self.notes.is_empty() {
            lines.push(String::new());
            lines.push(self.notes.clone());
        }
        for (i, section) in self.sections.iter().enumerate() {
            lines.push(String::new());
            let label = u8::try_from(i)
                .ok()
                .map(|n| char::from(b'A'.saturating_add(n)))
                .filter(char::is_ascii_uppercase)
                .map_or_else(|| format!("#{}", i + 1), |c| c.to_string());
            lines.extend(section.to_text_lines(&label, 0, exercises, show_tut, show_rpe));
        }
        lines.join("\n")
    }

    pub fn exercises(&self) -> BTreeSet<ExerciseID> {
        self.sections
            .iter()
            .flat_map(RoutinePart::exercises)
            .collect::<BTreeSet<_>>()
    }

    /// Returns the exercises used in training sessions of the routine that the routine no longer
    /// contains, sorted by name.
    pub fn previously_used_exercises<'a>(
        &self,
        training_sessions: &[TrainingSession],
        exercises: &'a [Exercise],
    ) -> Vec<&'a Exercise> {
        let used_exercise_ids = training_sessions
            .iter()
            .filter(|t| t.routine_id == self.id)
            .flat_map(TrainingSession::exercises)
            .collect::<BTreeSet<_>>();
        let mut result = (&used_exercise_ids - &self.exercises())
            .iter()
            .filter_map(|exercise_id| exercises.iter().find(|e| e.id == *exercise_id))
            .collect::<Vec<_>>();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    pub fn add_section(&mut self, path: &RoutinePartPath) {
        let new_section = RoutinePart::RoutineSection {
            rounds: Rounds::new(1).unwrap(),
            parts: vec![],
        };
        if path.is_empty() {
            self.sections.push(new_section);
        } else if let Some(RoutinePart::RoutineSection { parts, .. }) =
            Self::get_mut_part(&mut self.sections, path)
        {
            parts.push(new_section);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_section(&mut self, rounds: Option<Rounds>, path: &RoutinePartPath) {
        let new_rounds = rounds;
        if let Some(RoutinePart::RoutineSection { rounds, .. }) =
            Self::get_mut_part(&mut self.sections, path)
            && let Some(new_rounds) = new_rounds
        {
            *rounds = new_rounds;
        }
    }

    pub fn add_activity(&mut self, exercise_id: ExerciseID, path: &RoutinePartPath) {
        let new_activity = RoutinePart::RoutineActivity {
            exercise_id,
            reps: Reps::default(),
            time: if exercise_id.is_nil() {
                Time::new(60).unwrap()
            } else {
                Time::default()
            },
            weight: Weight::default(),
            rpe: RPE::ZERO,
            automatic: exercise_id.is_nil(),
        };
        if let Some(RoutinePart::RoutineSection { parts, .. }) =
            Self::get_mut_part(&mut self.sections, path)
        {
            parts.push(new_activity);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_activity(
        &mut self,
        exercise_id: Option<ExerciseID>,
        reps: Option<Reps>,
        time: Option<Time>,
        weight: Option<Weight>,
        rpe: Option<RPE>,
        automatic: Option<bool>,
        path: &RoutinePartPath,
    ) {
        let new_exercise_id = exercise_id;
        let new_reps = reps;
        let new_time = time;
        let new_weight = weight;
        let new_rpe = rpe;
        let new_automatic = automatic;
        if let Some(RoutinePart::RoutineActivity {
            exercise_id,
            reps,
            time,
            weight,
            rpe,
            automatic,
        }) = Self::get_mut_part(&mut self.sections, path)
        {
            if let Some(new_exercise_id) = new_exercise_id {
                *exercise_id = new_exercise_id;
            }
            if let Some(new_reps) = new_reps {
                *reps = new_reps;
            }
            if let Some(new_time) = new_time {
                *time = new_time;
            }
            if let Some(new_weight) = new_weight {
                *weight = new_weight;
            }
            if let Some(new_rpe) = new_rpe {
                *rpe = new_rpe;
            }
            if let Some(new_automatic) = new_automatic {
                *automatic = new_automatic;
            }
        }
    }

    /// Removes the part at `path`, leaving the routine unchanged if `path` does not resolve.
    pub fn remove_part(&mut self, path: &RoutinePartPath) {
        if let Some((parts, index)) = self.siblings_mut(path) {
            parts.remove(index);
        }
    }

    /// Moves the part at `path` one position towards the end of its list, wrapping around at the
    /// end and leaving the routine unchanged if `path` does not resolve.
    pub fn move_part_down(&mut self, path: &RoutinePartPath) {
        if let Some((parts, index)) = self.siblings_mut(path) {
            if index == parts.len() - 1 {
                parts.rotate_right(1);
            } else {
                parts.swap(index, index + 1);
            }
        }
    }

    /// Moves the part at `path` one position towards the start of its list, wrapping around at the
    /// start and leaving the routine unchanged if `path` does not resolve.
    pub fn move_part_up(&mut self, path: &RoutinePartPath) {
        if let Some((parts, index)) = self.siblings_mut(path) {
            if index == 0 {
                parts.rotate_left(1);
            } else {
                parts.swap(index, index - 1);
            }
        }
    }

    /// Returns the list containing the part at `path` together with the index of that part in the
    /// list, or `None` if `path` does not resolve.
    fn siblings_mut(&mut self, path: &RoutinePartPath) -> Option<(&mut Vec<RoutinePart>, usize)> {
        let (&index, parent) = path.split_first()?;
        let parts = if parent.is_empty() {
            &mut self.sections
        } else if let Some(RoutinePart::RoutineSection { parts, .. }) =
            Self::get_mut_part(&mut self.sections, parent)
        {
            parts
        } else {
            return None;
        };
        if index >= parts.len() {
            return None;
        }
        Some((parts, index))
    }

    /// Move the part at `source` into the section at `target_parent` at the insertion position
    /// `index`, with `index` referring to the parts before the removal of the moved part.
    ///
    /// The routine is left unchanged if `source` does not exist, if `target_parent` is not a
    /// section outside the moved part or if the moved part is an activity and `target_parent` is
    /// the top level, which may only contain sections.
    pub fn move_part(
        &mut self,
        source: &RoutinePartPath,
        target_parent: &RoutinePartPath,
        index: usize,
    ) {
        if let Some(sections) =
            Self::sections_with_moved_part(&self.sections, source, target_parent, index)
        {
            self.sections = sections;
        }
    }

    fn sections_with_moved_part(
        sections: &[RoutinePart],
        source: &RoutinePartPath,
        target_parent: &RoutinePartPath,
        index: usize,
    ) -> Option<Vec<RoutinePart>> {
        let (&source_index, source_parent) = source.split_first()?;
        if target_parent.ends_with(source) {
            return None;
        }
        let mut sections = sections.to_vec();
        let part = if source_parent.is_empty() {
            if source_index >= sections.len() {
                return None;
            }
            sections.remove(source_index)
        } else {
            let Some(RoutinePart::RoutineSection { parts, .. }) =
                Self::get_mut_part(&mut sections, source_parent)
            else {
                return None;
            };
            if source_index >= parts.len() {
                return None;
            }
            parts.remove(source_index)
        };
        if target_parent.is_empty() && matches!(part, RoutinePart::RoutineActivity { .. }) {
            return None;
        }
        // `target_parent` and `index` refer to the parts before the removal of the moved part
        let mut target_parent = target_parent.to_vec();
        let mut index = index;
        if target_parent[..] == *source_parent {
            if index > source_index {
                index -= 1;
            }
        } else if target_parent.ends_with(source_parent) {
            let position = target_parent.len() - source_parent.len() - 1;
            if target_parent[position] > source_index {
                target_parent[position] -= 1;
            }
        }
        if target_parent.is_empty() {
            let index = index.min(sections.len());
            sections.insert(index, part);
        } else {
            let Some(RoutinePart::RoutineSection { parts, .. }) =
                Self::get_mut_part(&mut sections, &target_parent)
            else {
                return None;
            };
            let index = index.min(parts.len());
            parts.insert(index, part);
        }
        Some(sections)
    }

    #[must_use]
    pub fn part(&self, path: &RoutinePartPath) -> Option<&RoutinePart> {
        Self::get_part(&self.sections, path)
    }

    fn get_part<'a>(sections: &'a [RoutinePart], path: &[usize]) -> Option<&'a RoutinePart> {
        if let Some(i) = path.last()
            && i < &sections.len()
        {
            let p = &sections[*i];
            if path.len() == 1 {
                return Some(p);
            }
            if let RoutinePart::RoutineSection { rounds: _, parts } = p {
                return Self::get_part(parts, &path[..path.len() - 1]);
            }
        }
        None
    }

    fn get_mut_part<'a>(
        sections: &'a mut [RoutinePart],
        path: &[usize],
    ) -> Option<&'a mut RoutinePart> {
        if let Some(i) = path.last()
            && i < &sections.len()
        {
            let p = &mut sections[*i];
            if path.len() == 1 {
                return Some(p);
            }
            if let RoutinePart::RoutineSection { rounds: _, parts } = p {
                return Self::get_mut_part(parts, &path[..path.len() - 1]);
            }
        }
        None
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

impl FromStr for RoutineID {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(Self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoutinePart {
    RoutineSection {
        rounds: Rounds,
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
                let r: u32 = (*rounds).into();
                parts.iter().map(RoutinePart::duration).sum::<Duration>()
                    * r.try_into().unwrap_or_default()
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
                let r: u32 = (*rounds).into();
                parts.iter().map(RoutinePart::num_sets).sum::<u32>() * r
            }
            RoutinePart::RoutineActivity { exercise_id, .. } => (!exercise_id.is_nil()).into(),
        }
    }

    #[must_use]
    pub fn stimulus_per_muscle(&self, exercises: &[Exercise]) -> BTreeMap<MuscleID, Stimulus> {
        match self {
            RoutinePart::RoutineSection { rounds, parts } => {
                let mut result: BTreeMap<MuscleID, Stimulus> = BTreeMap::new();
                for part in parts {
                    for (muscle_id, stimulus) in part.stimulus_per_muscle(exercises) {
                        let r: u32 = (*rounds).into();
                        *result.entry(muscle_id).or_insert(Stimulus::NONE) += stimulus * r;
                    }
                }
                result
            }
            RoutinePart::RoutineActivity { exercise_id, .. } => exercises
                .iter()
                .find(|e| e.id == *exercise_id)
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
                for _ in 0..(*rounds).into() {
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
                        target_time: *time,
                        automatic: *automatic,
                    }
                } else {
                    TrainingSessionElement::Set {
                        exercise_id: *exercise_id,
                        reps: Reps::default(),
                        time: Time::default(),
                        weight: Weight::default(),
                        rpe: RPE::default(),
                        target_reps: *reps,
                        target_time: *time,
                        target_weight: *weight,
                        target_rpe: *rpe,
                        automatic: *automatic,
                    }
                });
            }
        }
        result
    }

    fn to_text_lines(
        &self,
        label: &str,
        depth: usize,
        exercises: &[Exercise],
        show_tut: bool,
        show_rpe: bool,
    ) -> Vec<String> {
        use std::fmt::Write as _;
        let indent = "  ".repeat(depth);
        let mut lines = vec![];
        match self {
            RoutinePart::RoutineSection { rounds, parts } => {
                let r = u32::from(*rounds);
                let sets_label = if r == 1 { "set" } else { "sets" };
                lines.push(format!("{indent}[{label}] {r} {sets_label}"));
                let mut counter = 1;
                for part in parts {
                    match part {
                        RoutinePart::RoutineSection { .. } => {
                            let label = RoutinePart::part_label(label, counter);
                            counter += 1;
                            lines.extend(part.to_text_lines(
                                &label,
                                depth + 1,
                                exercises,
                                show_tut,
                                show_rpe,
                            ));
                            lines.push(String::new());
                        }
                        RoutinePart::RoutineActivity { exercise_id, .. }
                            if exercise_id.is_nil() =>
                        {
                            lines.extend(part.to_text_lines(
                                "",
                                depth + 1,
                                exercises,
                                show_tut,
                                show_rpe,
                            ));
                        }
                        RoutinePart::RoutineActivity { .. } => {
                            let label = RoutinePart::part_label(label, counter);
                            counter += 1;
                            lines.extend(part.to_text_lines(
                                &label,
                                depth + 1,
                                exercises,
                                show_tut,
                                show_rpe,
                            ));
                        }
                    }
                }
                if lines.last().is_some_and(String::is_empty) {
                    lines.pop();
                }
            }
            RoutinePart::RoutineActivity {
                exercise_id,
                reps,
                time,
                weight,
                rpe,
                ..
            } => {
                if exercise_id.is_nil() {
                    if *time > Time::default() {
                        lines.push(format!("{indent}Rest \u{2014} {time} s"));
                    } else {
                        lines.push(format!("{indent}Rest"));
                    }
                } else {
                    let name = exercises.iter().find(|e| e.id == *exercise_id).map_or_else(
                        || format!("Exercise#{}", **exercise_id),
                        |e| e.name.to_string(),
                    );
                    let mut parts = vec![];
                    if *reps > Reps::default() {
                        parts.push(reps.to_string());
                    }
                    if show_tut && *time > Time::default() {
                        parts.push(format!("{time} s"));
                    }
                    if *weight > Weight::default() {
                        parts.push(format!("{weight} kg"));
                    }
                    let mut targets = parts.join(" \u{00d7} ");
                    if show_rpe && *rpe > RPE::ZERO {
                        let _ = write!(targets, " @ {rpe}");
                    }
                    if targets.is_empty() {
                        lines.push(format!("{indent}{label} \u{2014} {name}"));
                    } else {
                        lines.push(format!(
                            "{indent}{label} \u{2014} {name} \u{2014} {targets}"
                        ));
                    }
                }
            }
        }
        lines
    }

    fn part_label(parent: &str, n: usize) -> String {
        if parent.ends_with(|c: char| c.is_ascii_digit()) {
            format!("{parent}.{n}")
        } else {
            format!("{parent}{n}")
        }
    }
}

#[derive(Deref, Debug, Default, Clone, From, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoutinePartPath(Vec<usize>);

#[derive(Debug, Display, Clone, Copy, Into, PartialEq, PartialOrd)]
pub struct Rounds(u32);

impl Rounds {
    pub fn new(value: u32) -> Result<Self, RoundsError> {
        if !(1..1000).contains(&value) {
            return Err(RoundsError::OutOfRange);
        }

        Ok(Self(value))
    }
}

impl Default for Rounds {
    fn default() -> Self {
        Rounds(1)
    }
}

impl TryFrom<&str> for Rounds {
    type Error = RoundsError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.parse::<u32>() {
            Ok(parsed_value) => Rounds::new(parsed_value),
            Err(_) => Err(RoundsError::ParseError),
        }
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum RoundsError {
    #[error("rounds must be in the range 1 to 999")]
    OutOfRange,
    #[error("rounds must be an integer")]
    ParseError,
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
    use proptest::prelude::*;
    use rstest::rstest;

    use crate::{
        ExerciseMuscle, Service,
        tests::{Call, FakeRepository},
    };

    use super::*;

    static ROUTINE: std::sync::LazyLock<Routine> = std::sync::LazyLock::new(|| Routine {
        id: 1.into(),
        name: Name::new("A").unwrap(),
        notes: String::from("B"),
        archived: false,
        sections: vec![
            RoutinePart::RoutineSection {
                rounds: Rounds::new(2).unwrap(),
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
                rounds: Rounds::new(2).unwrap(),
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

    static EXERCISES: std::sync::LazyLock<Vec<Exercise>> = std::sync::LazyLock::new(|| {
        vec![Exercise {
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
        }]
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
    fn test_routine_previously_used_exercises_excludes_exercises_of_routine() {
        let exercises = [named_exercise(1, "A"), named_exercise(3, "B")];

        assert_eq!(
            ROUTINE.previously_used_exercises(&[training_session_with(1, &[1, 3])], &exercises),
            [&exercises[1]]
        );
    }

    #[test]
    fn test_routine_previously_used_exercises_orders_by_name() {
        let exercises = [named_exercise(3, "C"), named_exercise(4, "B")];

        assert_eq!(
            ROUTINE.previously_used_exercises(&[training_session_with(1, &[3, 4])], &exercises),
            [&exercises[1], &exercises[0]]
        );
    }

    #[test]
    fn test_routine_previously_used_exercises_ignores_other_routines() {
        let exercises = [named_exercise(3, "B")];

        assert_eq!(
            ROUTINE.previously_used_exercises(&[training_session_with(2, &[3])], &exercises),
            [] as [&Exercise; 0]
        );
    }

    fn named_exercise(id: u128, name: &str) -> Exercise {
        Exercise {
            id: id.into(),
            name: Name::new(name).unwrap(),
            notes: String::new(),
            muscles: vec![],
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        }
    }

    fn training_session_with(routine_id: u128, exercise_ids: &[u128]) -> TrainingSession {
        TrainingSession {
            elements: exercise_ids
                .iter()
                .map(|exercise_id| TrainingSessionElement::Set {
                    exercise_id: (*exercise_id).into(),
                    reps: Reps::default(),
                    time: Time::default(),
                    weight: Weight::default(),
                    rpe: RPE::default(),
                    target_reps: Reps::default(),
                    target_time: Time::default(),
                    target_weight: Weight::default(),
                    target_rpe: RPE::default(),
                    automatic: false,
                })
                .collect(),
            ..training_session(1, routine_id, NaiveDate::default())
        }
    }

    #[test]
    fn test_routine_get_part_in_sections() {
        let mut sections = vec![
            RoutinePart::RoutineSection {
                rounds: Rounds::new(1).unwrap(),
                parts: vec![RoutinePart::RoutineActivity {
                    exercise_id: ExerciseID::nil(),
                    reps: Reps::new(1).unwrap(),
                    time: Time::new(2).unwrap(),
                    weight: Weight::new(4.0).unwrap(),
                    rpe: RPE::FIVE,
                    automatic: false,
                }],
            },
            RoutinePart::RoutineSection {
                rounds: Rounds::new(2).unwrap(),
                parts: vec![RoutinePart::RoutineActivity {
                    exercise_id: ExerciseID::nil(),
                    reps: Reps::new(2).unwrap(),
                    time: Time::new(3).unwrap(),
                    weight: Weight::new(5.0).unwrap(),
                    rpe: RPE::SIX,
                    automatic: false,
                }],
            },
        ];
        assert_eq!(
            *Routine::get_mut_part(&mut sections, &[0]).unwrap(),
            RoutinePart::RoutineSection {
                rounds: Rounds::new(1).unwrap(),
                parts: vec![RoutinePart::RoutineActivity {
                    exercise_id: ExerciseID::nil(),
                    reps: Reps::new(1).unwrap(),
                    time: Time::new(2).unwrap(),
                    weight: Weight::new(4.0).unwrap(),
                    rpe: RPE::FIVE,
                    automatic: false,
                }],
            }
        );
        assert_eq!(
            *Routine::get_mut_part(&mut sections, &[1]).unwrap(),
            RoutinePart::RoutineSection {
                rounds: Rounds::new(2).unwrap(),
                parts: vec![RoutinePart::RoutineActivity {
                    exercise_id: ExerciseID::nil(),
                    reps: Reps::new(2).unwrap(),
                    time: Time::new(3).unwrap(),
                    weight: Weight::new(5.0).unwrap(),
                    rpe: RPE::SIX,
                    automatic: false,
                }],
            }
        );
        assert!(Routine::get_mut_part(&mut sections, &[2]).is_none());
        assert_eq!(
            *Routine::get_mut_part(&mut sections, &[0, 0]).unwrap(),
            RoutinePart::RoutineActivity {
                exercise_id: ExerciseID::nil(),
                reps: Reps::new(1).unwrap(),
                time: Time::new(2).unwrap(),
                weight: Weight::new(4.0).unwrap(),
                rpe: RPE::FIVE,
                automatic: false,
            },
        );
        assert!(Routine::get_mut_part(&mut sections, &[1, 0]).is_none());
        assert_eq!(
            *Routine::get_mut_part(&mut sections, &[0, 1]).unwrap(),
            RoutinePart::RoutineActivity {
                exercise_id: ExerciseID::nil(),
                reps: Reps::new(2).unwrap(),
                time: Time::new(3).unwrap(),
                weight: Weight::new(5.0).unwrap(),
                rpe: RPE::SIX,
                automatic: false,
            },
        );
        assert!(Routine::get_mut_part(&mut sections, &[1, 1]).is_none());
    }

    #[test]
    fn test_routine_get_part_in_nested_sections() {
        let mut sections = vec![RoutinePart::RoutineSection {
            rounds: Rounds::new(1).unwrap(),
            parts: vec![
                RoutinePart::RoutineActivity {
                    exercise_id: ExerciseID::nil(),
                    reps: Reps::new(1).unwrap(),
                    time: Time::new(2).unwrap(),
                    weight: Weight::new(4.0).unwrap(),
                    rpe: RPE::FIVE,
                    automatic: false,
                },
                RoutinePart::RoutineSection {
                    rounds: Rounds::new(2).unwrap(),
                    parts: vec![RoutinePart::RoutineActivity {
                        exercise_id: ExerciseID::nil(),
                        reps: Reps::new(2).unwrap(),
                        time: Time::new(3).unwrap(),
                        weight: Weight::new(5.0).unwrap(),
                        rpe: RPE::SIX,
                        automatic: false,
                    }],
                },
            ],
        }];
        assert_eq!(
            *Routine::get_mut_part(&mut sections, &[0]).unwrap(),
            RoutinePart::RoutineSection {
                rounds: Rounds::new(1).unwrap(),
                parts: vec![
                    RoutinePart::RoutineActivity {
                        exercise_id: ExerciseID::nil(),
                        reps: Reps::new(1).unwrap(),
                        time: Time::new(2).unwrap(),
                        weight: Weight::new(4.0).unwrap(),
                        rpe: RPE::FIVE,
                        automatic: false,
                    },
                    RoutinePart::RoutineSection {
                        rounds: Rounds::new(2).unwrap(),
                        parts: vec![RoutinePart::RoutineActivity {
                            exercise_id: ExerciseID::nil(),
                            reps: Reps::new(2).unwrap(),
                            time: Time::new(3).unwrap(),
                            weight: Weight::new(5.0).unwrap(),
                            rpe: RPE::SIX,
                            automatic: false,
                        }],
                    },
                ],
            }
        );
        assert!(Routine::get_mut_part(&mut sections, &[1]).is_none());
        assert_eq!(
            *Routine::get_mut_part(&mut sections, &[0, 0]).unwrap(),
            RoutinePart::RoutineActivity {
                exercise_id: ExerciseID::nil(),
                reps: Reps::new(1).unwrap(),
                time: Time::new(2).unwrap(),
                weight: Weight::new(4.0).unwrap(),
                rpe: RPE::FIVE,
                automatic: false,
            },
        );
        assert_eq!(
            *Routine::get_mut_part(&mut sections, &[1, 0]).unwrap(),
            RoutinePart::RoutineSection {
                rounds: Rounds::new(2).unwrap(),
                parts: vec![RoutinePart::RoutineActivity {
                    exercise_id: ExerciseID::nil(),
                    reps: Reps::new(2).unwrap(),
                    time: Time::new(3).unwrap(),
                    weight: Weight::new(5.0).unwrap(),
                    rpe: RPE::SIX,
                    automatic: false,
                }],
            },
        );
        assert!(Routine::get_mut_part(&mut sections, &[2, 0]).is_none());
        assert!(Routine::get_mut_part(&mut sections, &[0, 0, 0]).is_none());
        assert_eq!(
            *Routine::get_mut_part(&mut sections, &[0, 1, 0]).unwrap(),
            RoutinePart::RoutineActivity {
                exercise_id: ExerciseID::nil(),
                reps: Reps::new(2).unwrap(),
                time: Time::new(3).unwrap(),
                weight: Weight::new(5.0).unwrap(),
                rpe: RPE::SIX,
                automatic: false,
            },
        );
        assert!(Routine::get_mut_part(&mut sections, &[1, 1, 0]).is_none());
        assert!(Routine::get_mut_part(&mut sections, &[0, 0, 1, 0]).is_none());
    }

    fn routine_with_sections(sections: Vec<RoutinePart>) -> Routine {
        Routine {
            id: 1.into(),
            name: Name::new("A").unwrap(),
            notes: String::new(),
            archived: false,
            sections,
        }
    }

    fn section(rounds: u32, parts: Vec<RoutinePart>) -> RoutinePart {
        RoutinePart::RoutineSection {
            rounds: Rounds::new(rounds).unwrap(),
            parts,
        }
    }

    fn activity(reps: u32) -> RoutinePart {
        RoutinePart::RoutineActivity {
            exercise_id: 1.into(),
            reps: Reps::new(reps).unwrap(),
            time: Time::default(),
            weight: Weight::default(),
            rpe: RPE::ZERO,
            automatic: false,
        }
    }

    #[test]
    fn test_routine_move_part_within_section() {
        let mut routine = routine_with_sections(vec![section(
            1,
            vec![activity(1), activity(2), activity(3)],
        )]);
        routine.move_part(&vec![0, 0].into(), &vec![0].into(), 2);
        assert_eq!(
            routine.sections,
            vec![section(1, vec![activity(2), activity(1), activity(3)])]
        );
        routine.move_part(&vec![2, 0].into(), &vec![0].into(), 0);
        assert_eq!(
            routine.sections,
            vec![section(1, vec![activity(3), activity(2), activity(1)])]
        );
    }

    #[test]
    fn test_routine_move_part_onto_itself_keeps_routine_unchanged() {
        let mut routine = routine_with_sections(vec![section(1, vec![activity(1), activity(2)])]);
        routine.move_part(&vec![0, 0].into(), &vec![0].into(), 0);
        routine.move_part(&vec![0, 0].into(), &vec![0].into(), 1);
        assert_eq!(
            routine.sections,
            vec![section(1, vec![activity(1), activity(2)])]
        );
    }

    #[test]
    fn test_routine_move_part_between_sections() {
        let mut routine = routine_with_sections(vec![
            section(1, vec![activity(1), activity(2)]),
            section(2, vec![activity(3)]),
        ]);
        routine.move_part(&vec![1, 0].into(), &vec![1].into(), 0);
        assert_eq!(
            routine.sections,
            vec![
                section(1, vec![activity(1)]),
                section(2, vec![activity(2), activity(3)]),
            ]
        );
    }

    #[test]
    fn test_routine_move_part_into_nested_section() {
        let mut routine = routine_with_sections(vec![section(
            1,
            vec![activity(1), section(2, vec![activity(2)])],
        )]);
        routine.move_part(&vec![0, 0].into(), &vec![1, 0].into(), 1);
        assert_eq!(
            routine.sections,
            vec![section(1, vec![section(2, vec![activity(2), activity(1)])])]
        );
    }

    #[test]
    fn test_routine_move_section_at_top_level() {
        let mut routine = routine_with_sections(vec![
            section(1, vec![activity(1)]),
            section(2, vec![activity(2)]),
        ]);
        routine.move_part(&vec![0].into(), &RoutinePartPath::default(), 2);
        assert_eq!(
            routine.sections,
            vec![section(2, vec![activity(2)]), section(1, vec![activity(1)])]
        );
    }

    #[test]
    fn test_routine_move_section_into_section() {
        let mut routine = routine_with_sections(vec![
            section(1, vec![activity(1)]),
            section(2, vec![activity(2)]),
        ]);
        routine.move_part(&vec![0].into(), &vec![1].into(), 0);
        assert_eq!(
            routine.sections,
            vec![section(2, vec![section(1, vec![activity(1)]), activity(2)])]
        );
    }

    #[test]
    fn test_routine_move_section_into_descendant_keeps_routine_unchanged() {
        let mut routine =
            routine_with_sections(vec![section(1, vec![section(2, vec![activity(1)])])]);
        routine.move_part(&vec![0].into(), &vec![0].into(), 0);
        routine.move_part(&vec![0].into(), &vec![0, 0].into(), 0);
        assert_eq!(
            routine.sections,
            vec![section(1, vec![section(2, vec![activity(1)])])]
        );
    }

    #[test]
    fn test_routine_move_activity_to_top_level_keeps_routine_unchanged() {
        let mut routine = routine_with_sections(vec![section(1, vec![activity(1)])]);
        routine.move_part(&vec![0, 0].into(), &RoutinePartPath::default(), 0);
        assert_eq!(routine.sections, vec![section(1, vec![activity(1)])]);
    }

    #[test]
    fn test_routine_move_part_into_activity_keeps_routine_unchanged() {
        let mut routine = routine_with_sections(vec![section(1, vec![activity(1), activity(2)])]);
        routine.move_part(&vec![0, 0].into(), &vec![1, 0].into(), 0);
        assert_eq!(
            routine.sections,
            vec![section(1, vec![activity(1), activity(2)])]
        );
    }

    #[test]
    fn test_routine_move_nonexistent_part_keeps_routine_unchanged() {
        let mut routine = routine_with_sections(vec![section(1, vec![activity(1)])]);
        routine.move_part(&RoutinePartPath::default(), &vec![0].into(), 0);
        routine.move_part(&vec![1].into(), &vec![0].into(), 0);
        routine.move_part(&vec![1, 0].into(), &vec![0].into(), 0);
        assert_eq!(routine.sections, vec![section(1, vec![activity(1)])]);
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
            exercise_notes: BTreeMap::new(),
        }
    }

    #[test]
    fn test_routine_to_text() {
        assert_eq!(
            ROUTINE.to_text(&EXERCISES, true, true),
            "A\n\nB\n\n[A] 2 sets\n  A1 \u{2014} A \u{2014} 10 \u{00d7} 2 s \u{00d7} 30 kg @ 10\n  Rest \u{2014} 60 s\n\n[B] 2 sets\n  B1 \u{2014} Exercise#00000000-0000-0000-0000-000000000002 \u{2014} 10\n  Rest \u{2014} 30 s"
        );
    }

    #[test]
    fn test_routine_to_text_without_notes() {
        let routine = Routine {
            id: 0.into(),
            name: Name::new("N").unwrap(),
            notes: String::new(),
            archived: false,
            sections: vec![RoutinePart::RoutineSection {
                rounds: Rounds::new(1).unwrap(),
                parts: vec![RoutinePart::RoutineActivity {
                    exercise_id: 1.into(),
                    reps: Reps::new(5).unwrap(),
                    time: Time::default(),
                    weight: Weight::default(),
                    rpe: RPE::ZERO,
                    automatic: false,
                }],
            }],
        };
        assert_eq!(
            routine.to_text(&EXERCISES, true, true),
            "N\n\n[A] 1 set\n  A1 \u{2014} A \u{2014} 5"
        );
    }

    #[test]
    fn test_routine_to_text_time_only_and_rest_no_time() {
        let routine = Routine {
            id: 0.into(),
            name: Name::new("T").unwrap(),
            notes: String::new(),
            archived: false,
            sections: vec![RoutinePart::RoutineSection {
                rounds: Rounds::new(2).unwrap(),
                parts: vec![
                    RoutinePart::RoutineActivity {
                        exercise_id: 1.into(),
                        reps: Reps::default(),
                        time: Time::new(30).unwrap(),
                        weight: Weight::default(),
                        rpe: RPE::ZERO,
                        automatic: false,
                    },
                    RoutinePart::RoutineActivity {
                        exercise_id: ExerciseID::nil(),
                        reps: Reps::default(),
                        time: Time::default(),
                        weight: Weight::default(),
                        rpe: RPE::ZERO,
                        automatic: false,
                    },
                ],
            }],
        };
        assert_eq!(
            routine.to_text(&EXERCISES, true, true),
            "T\n\n[A] 2 sets\n  A1 \u{2014} A \u{2014} 30 s\n  Rest"
        );
    }

    #[test]
    fn test_routine_to_text_nested() {
        let routine = Routine {
            id: 0.into(),
            name: Name::new("N").unwrap(),
            notes: String::new(),
            archived: false,
            sections: vec![RoutinePart::RoutineSection {
                rounds: Rounds::new(2).unwrap(),
                parts: vec![
                    RoutinePart::RoutineActivity {
                        exercise_id: 1.into(),
                        reps: Reps::new(5).unwrap(),
                        time: Time::default(),
                        weight: Weight::default(),
                        rpe: RPE::ZERO,
                        automatic: false,
                    },
                    RoutinePart::RoutineSection {
                        rounds: Rounds::new(3).unwrap(),
                        parts: vec![
                            RoutinePart::RoutineActivity {
                                exercise_id: 1.into(),
                                reps: Reps::new(8).unwrap(),
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
                                automatic: false,
                            },
                        ],
                    },
                ],
            }],
        };
        assert_eq!(
            routine.to_text(&EXERCISES, true, true),
            "N\n\n[A] 2 sets\n  A1 \u{2014} A \u{2014} 5\n  [A2] 3 sets\n    A2.1 \u{2014} A \u{2014} 8\n    Rest \u{2014} 30 s"
        );
    }

    #[test]
    fn test_routine_to_text_no_params() {
        let routine = Routine {
            id: 0.into(),
            name: Name::new("X").unwrap(),
            notes: String::new(),
            archived: false,
            sections: vec![RoutinePart::RoutineSection {
                rounds: Rounds::new(1).unwrap(),
                parts: vec![RoutinePart::RoutineActivity {
                    exercise_id: 1.into(),
                    reps: Reps::default(),
                    time: Time::default(),
                    weight: Weight::default(),
                    rpe: RPE::ZERO,
                    automatic: false,
                }],
            }],
        };
        assert_eq!(
            routine.to_text(&EXERCISES, true, true),
            "X\n\n[A] 1 set\n  A1 \u{2014} A"
        );
    }

    #[test]
    fn test_routine_to_text_hide_tut() {
        assert_eq!(
            ROUTINE.to_text(&EXERCISES, false, true),
            "A\n\nB\n\n[A] 2 sets\n  A1 \u{2014} A \u{2014} 10 \u{00d7} 30 kg @ 10\n  Rest \u{2014} 60 s\n\n[B] 2 sets\n  B1 \u{2014} Exercise#00000000-0000-0000-0000-000000000002 \u{2014} 10\n  Rest \u{2014} 30 s"
        );
    }

    #[test]
    fn test_routine_to_text_hide_rpe() {
        assert_eq!(
            ROUTINE.to_text(&EXERCISES, true, false),
            "A\n\nB\n\n[A] 2 sets\n  A1 \u{2014} A \u{2014} 10 \u{00d7} 2 s \u{00d7} 30 kg\n  Rest \u{2014} 60 s\n\n[B] 2 sets\n  B1 \u{2014} Exercise#00000000-0000-0000-0000-000000000002 \u{2014} 10\n  Rest \u{2014} 30 s"
        );
    }

    #[test]
    fn test_routine_to_text_many_sections() {
        let sections = (0..27)
            .map(|_| RoutinePart::RoutineSection {
                rounds: Rounds::new(1).unwrap(),
                parts: vec![],
            })
            .collect();
        let routine = Routine {
            id: 0.into(),
            name: Name::new("N").unwrap(),
            notes: String::new(),
            archived: false,
            sections,
        };
        let text = routine.to_text(&[], true, true);
        assert!(
            text.contains("[Z] 1 set"),
            "expected label Z for 26th section"
        );
        assert!(
            text.contains("[#27] 1 set"),
            "expected fallback label for 27th section"
        );
    }

    #[rstest]
    #[case::unused("C", Ok("C"))]
    #[case::used_by_the_routine_itself("A", Ok("A"))]
    #[case::used_by_another_routine("B", Err("entry with this name already exists"))]
    #[case::invalid("", Err("name must not be empty"))]
    fn test_validate_routine_name(#[case] input: &str, #[case] expected: Result<&str, &str>) {
        let service = Service::new(
            FakeRepository::default()
                .with_routines(vec![named_routine(1, "A"), named_routine(2, "B")]),
        );

        assert_eq!(
            pollster::block_on(service.validate_routine_name(input, 1.into()))
                .map(|name| name.to_string())
                .map_err(|err| err.to_string()),
            expected.map(str::to_string).map_err(str::to_string)
        );
    }

    #[test]
    fn test_validate_routine_name_unreadable_routines() {
        let service = Service::new(FakeRepository::default().failing(Call::ReadRoutines));

        assert!(matches!(
            pollster::block_on(service.validate_routine_name("A", 1.into())),
            Err(ValidationError::Other(_))
        ));
    }

    #[rstest]
    #[case::known(1, Some("A"))]
    #[case::unknown(2, None)]
    fn test_get_routine(#[case] id: u128, #[case] expected: Option<&str>) {
        let service =
            Service::new(FakeRepository::default().with_routines(vec![named_routine(1, "A")]));

        assert_eq!(
            pollster::block_on(service.get_routine(id.into()))
                .unwrap()
                .map(|r| r.name.to_string()),
            expected.map(str::to_string)
        );
    }

    fn named_routine(id: u128, name: &str) -> Routine {
        Routine {
            id: id.into(),
            name: Name::new(name).unwrap(),
            notes: String::new(),
            archived: false,
            sections: vec![],
        }
    }

    #[rstest]
    #[case::empty_path(RoutinePartPath::default())]
    #[case::section_out_of_range(vec![1].into())]
    #[case::part_out_of_range(vec![1, 0].into())]
    #[case::unknown_parent_section(vec![0, 1].into())]
    #[case::parent_is_an_activity(vec![0, 0, 0].into())]
    fn test_routine_mutate_unresolvable_path_keeps_routine_unchanged(
        #[case] path: RoutinePartPath,
    ) {
        let sections = vec![section(1, vec![activity(1)])];

        for mutate in [
            Routine::remove_part,
            Routine::move_part_down,
            Routine::move_part_up,
        ] {
            let mut routine = routine_with_sections(sections.clone());
            mutate(&mut routine, &path);
            assert_eq!(routine.sections, sections);
        }
    }

    #[test]
    fn test_routine_mutate_empty_routine_keeps_routine_unchanged() {
        for mutate in [
            Routine::remove_part,
            Routine::move_part_down,
            Routine::move_part_up,
        ] {
            let mut routine = routine_with_sections(vec![]);
            mutate(&mut routine, &vec![0].into());
            assert_eq!(routine.sections, vec![]);
        }
    }

    #[test]
    fn test_routine_add_section_at_top_level() {
        let mut routine = routine_with_sections(vec![section(1, vec![])]);

        routine.add_section(&RoutinePartPath::default());

        assert_eq!(
            routine.sections,
            vec![section(1, vec![]), section(1, vec![])]
        );
    }

    #[test]
    fn test_routine_add_section_into_section() {
        let mut routine = routine_with_sections(vec![section(2, vec![activity(1)])]);

        routine.add_section(&vec![0].into());

        assert_eq!(
            routine.sections,
            vec![section(2, vec![activity(1), section(1, vec![])])]
        );
    }

    #[rstest]
    #[case::unknown_section(vec![1].into())]
    #[case::activity(vec![0, 0].into())]
    fn test_routine_add_section_with_unresolvable_path(#[case] path: RoutinePartPath) {
        let sections = vec![section(2, vec![activity(1)])];
        let mut routine = routine_with_sections(sections.clone());

        routine.add_section(&path);

        assert_eq!(routine.sections, sections);
    }

    #[test]
    fn test_routine_update_section() {
        let mut routine = routine_with_sections(vec![section(1, vec![activity(1)])]);

        routine.update_section(Some(Rounds::new(3).unwrap()), &vec![0].into());

        assert_eq!(routine.sections, vec![section(3, vec![activity(1)])]);
    }

    #[rstest]
    #[case::no_rounds(None, vec![0].into())]
    #[case::unknown_section(Some(Rounds::new(3).unwrap()), vec![1].into())]
    #[case::activity(Some(Rounds::new(3).unwrap()), vec![0, 0].into())]
    fn test_routine_update_section_without_effect(
        #[case] rounds: Option<Rounds>,
        #[case] path: RoutinePartPath,
    ) {
        let sections = vec![section(1, vec![activity(1)])];
        let mut routine = routine_with_sections(sections.clone());

        routine.update_section(rounds, &path);

        assert_eq!(routine.sections, sections);
    }

    #[test]
    fn test_routine_add_activity() {
        let mut routine = routine_with_sections(vec![section(1, vec![])]);

        routine.add_activity(1.into(), &vec![0].into());

        assert_eq!(
            routine.sections,
            vec![section(
                1,
                vec![RoutinePart::RoutineActivity {
                    exercise_id: 1.into(),
                    reps: Reps::default(),
                    time: Time::default(),
                    weight: Weight::default(),
                    rpe: RPE::ZERO,
                    automatic: false,
                }]
            )]
        );
    }

    #[test]
    fn test_routine_add_activity_without_exercise_is_an_automatic_rest() {
        let mut routine = routine_with_sections(vec![section(1, vec![])]);

        routine.add_activity(ExerciseID::nil(), &vec![0].into());

        assert_eq!(
            routine.sections,
            vec![section(
                1,
                vec![RoutinePart::RoutineActivity {
                    exercise_id: ExerciseID::nil(),
                    reps: Reps::default(),
                    time: Time::new(60).unwrap(),
                    weight: Weight::default(),
                    rpe: RPE::ZERO,
                    automatic: true,
                }]
            )]
        );
    }

    #[rstest]
    #[case::top_level(RoutinePartPath::default())]
    #[case::unknown_section(vec![1].into())]
    #[case::activity(vec![0, 0].into())]
    fn test_routine_add_activity_with_unresolvable_path(#[case] path: RoutinePartPath) {
        let sections = vec![section(1, vec![activity(1)])];
        let mut routine = routine_with_sections(sections.clone());

        routine.add_activity(2.into(), &path);

        assert_eq!(routine.sections, sections);
    }

    #[rstest]
    #[case::exercise_id(
        Some(ExerciseID::from(2u128)), None, None, None, None, None,
        RoutinePart::RoutineActivity {
            exercise_id: 2.into(),
            reps: Reps::new(1).unwrap(),
            time: Time::new(2).unwrap(),
            weight: Weight::new(3.0).unwrap(),
            rpe: RPE::FOUR,
            automatic: false,
        },
    )]
    #[case::reps(
        None, Some(Reps::new(5).unwrap()), None, None, None, None,
        RoutinePart::RoutineActivity {
            exercise_id: 1.into(),
            reps: Reps::new(5).unwrap(),
            time: Time::new(2).unwrap(),
            weight: Weight::new(3.0).unwrap(),
            rpe: RPE::FOUR,
            automatic: false,
        },
    )]
    #[case::time(
        None, None, Some(Time::new(6).unwrap()), None, None, None,
        RoutinePart::RoutineActivity {
            exercise_id: 1.into(),
            reps: Reps::new(1).unwrap(),
            time: Time::new(6).unwrap(),
            weight: Weight::new(3.0).unwrap(),
            rpe: RPE::FOUR,
            automatic: false,
        },
    )]
    #[case::weight(
        None, None, None, Some(Weight::new(7.0).unwrap()), None, None,
        RoutinePart::RoutineActivity {
            exercise_id: 1.into(),
            reps: Reps::new(1).unwrap(),
            time: Time::new(2).unwrap(),
            weight: Weight::new(7.0).unwrap(),
            rpe: RPE::FOUR,
            automatic: false,
        },
    )]
    #[case::rpe(
        None, None, None, None, Some(RPE::EIGHT), None,
        RoutinePart::RoutineActivity {
            exercise_id: 1.into(),
            reps: Reps::new(1).unwrap(),
            time: Time::new(2).unwrap(),
            weight: Weight::new(3.0).unwrap(),
            rpe: RPE::EIGHT,
            automatic: false,
        },
    )]
    #[case::automatic(
        None, None, None, None, None, Some(true),
        RoutinePart::RoutineActivity {
            exercise_id: 1.into(),
            reps: Reps::new(1).unwrap(),
            time: Time::new(2).unwrap(),
            weight: Weight::new(3.0).unwrap(),
            rpe: RPE::FOUR,
            automatic: true,
        },
    )]
    #[case::nothing(
        None, None, None, None, None, None,
        RoutinePart::RoutineActivity {
            exercise_id: 1.into(),
            reps: Reps::new(1).unwrap(),
            time: Time::new(2).unwrap(),
            weight: Weight::new(3.0).unwrap(),
            rpe: RPE::FOUR,
            automatic: false,
        },
    )]
    fn test_routine_update_activity(
        #[case] exercise_id: Option<ExerciseID>,
        #[case] reps: Option<Reps>,
        #[case] time: Option<Time>,
        #[case] weight: Option<Weight>,
        #[case] rpe: Option<RPE>,
        #[case] automatic: Option<bool>,
        #[case] expected: RoutinePart,
    ) {
        let mut routine = routine_with_sections(vec![section(1, vec![full_activity()])]);

        routine.update_activity(
            exercise_id,
            reps,
            time,
            weight,
            rpe,
            automatic,
            &vec![0, 0].into(),
        );

        assert_eq!(routine.sections, vec![section(1, vec![expected])]);
    }

    #[rstest]
    #[case::unknown_activity(vec![1, 0].into())]
    #[case::section(vec![0].into())]
    fn test_routine_update_activity_with_unresolvable_path(#[case] path: RoutinePartPath) {
        let sections = vec![section(1, vec![full_activity()])];
        let mut routine = routine_with_sections(sections.clone());

        routine.update_activity(Some(2.into()), None, None, None, None, None, &path);

        assert_eq!(routine.sections, sections);
    }

    fn full_activity() -> RoutinePart {
        RoutinePart::RoutineActivity {
            exercise_id: 1.into(),
            reps: Reps::new(1).unwrap(),
            time: Time::new(2).unwrap(),
            weight: Weight::new(3.0).unwrap(),
            rpe: RPE::FOUR,
            automatic: false,
        }
    }

    #[test]
    fn test_routine_remove_section() {
        let mut routine =
            routine_with_sections(vec![section(1, vec![activity(1)]), section(2, vec![])]);

        routine.remove_part(&vec![0].into());

        assert_eq!(routine.sections, vec![section(2, vec![])]);
    }

    #[test]
    fn test_routine_remove_part_of_section() {
        let mut routine = routine_with_sections(vec![section(1, vec![activity(1), activity(2)])]);

        routine.remove_part(&vec![0, 0].into());

        assert_eq!(routine.sections, vec![section(1, vec![activity(2)])]);
    }

    #[rstest]
    #[case::down(Routine::move_part_down, vec![0].into(), [2, 1, 3])]
    #[case::down_wrapping_around(Routine::move_part_down, vec![2].into(), [3, 1, 2])]
    #[case::up(Routine::move_part_up, vec![1].into(), [2, 1, 3])]
    #[case::up_wrapping_around(Routine::move_part_up, vec![0].into(), [2, 3, 1])]
    fn test_routine_move_section(
        #[case] mutate: fn(&mut Routine, &RoutinePartPath),
        #[case] path: RoutinePartPath,
        #[case] expected: [u32; 3],
    ) {
        let mut routine =
            routine_with_sections((1..=3).map(|rounds| section(rounds, vec![])).collect());

        mutate(&mut routine, &path);

        assert_eq!(
            routine.sections,
            expected
                .iter()
                .map(|rounds| section(*rounds, vec![]))
                .collect::<Vec<_>>()
        );
    }

    #[rstest]
    #[case::down(Routine::move_part_down, vec![0, 0].into(), [2, 1, 3])]
    #[case::down_wrapping_around(Routine::move_part_down, vec![2, 0].into(), [3, 1, 2])]
    #[case::up(Routine::move_part_up, vec![1, 0].into(), [2, 1, 3])]
    #[case::up_wrapping_around(Routine::move_part_up, vec![0, 0].into(), [2, 3, 1])]
    fn test_routine_move_part_of_section(
        #[case] mutate: fn(&mut Routine, &RoutinePartPath),
        #[case] path: RoutinePartPath,
        #[case] expected: [u32; 3],
    ) {
        let mut routine = routine_with_sections(vec![section(1, (1..=3).map(activity).collect())]);

        mutate(&mut routine, &path);

        assert_eq!(
            routine.sections,
            vec![section(
                1,
                expected.iter().map(|reps| activity(*reps)).collect()
            )]
        );
    }

    #[test]
    fn test_routine_part_to_training_session_elements() {
        let part = section(
            2,
            vec![
                section(2, vec![activity(1)]),
                RoutinePart::RoutineActivity {
                    exercise_id: ExerciseID::nil(),
                    reps: Reps::default(),
                    time: Time::new(60).unwrap(),
                    weight: Weight::default(),
                    rpe: RPE::ZERO,
                    automatic: true,
                },
            ],
        );

        let set = TrainingSessionElement::Set {
            exercise_id: 1.into(),
            reps: Reps::default(),
            time: Time::default(),
            weight: Weight::default(),
            rpe: RPE::default(),
            target_reps: Reps::new(1).unwrap(),
            target_time: Time::default(),
            target_weight: Weight::default(),
            target_rpe: RPE::ZERO,
            automatic: false,
        };
        let rest = TrainingSessionElement::Rest {
            target_time: Time::new(60).unwrap(),
            automatic: true,
        };

        assert_eq!(
            part.to_training_session_elements(),
            vec![
                set.clone(),
                set.clone(),
                rest.clone(),
                set.clone(),
                set,
                rest
            ]
        );
    }

    #[test]
    fn test_routine_part_path() {
        let routine = routine_with_sections(vec![section(1, vec![activity(1)])]);

        assert_eq!(routine.part(&vec![0].into()), Some(&routine.sections[0]));
        assert_eq!(routine.part(&vec![0, 0].into()), Some(&activity(1)));
        assert_eq!(routine.part(&RoutinePartPath::default()), None);
        assert_eq!(routine.part(&vec![1].into()), None);
        assert_eq!(routine.part(&vec![1, 0].into()), None);
        assert_eq!(routine.part(&vec![0, 0, 0].into()), None);
    }

    #[test]
    fn test_rounds_default() {
        assert_eq!(Rounds::default(), Rounds::new(1).unwrap());
    }

    #[rstest]
    #[case::lowest("1", Ok(Rounds::new(1).unwrap()))]
    #[case::highest("999", Ok(Rounds::new(999).unwrap()))]
    #[case::zero("0", Err(RoundsError::OutOfRange))]
    #[case::above_range("1000", Err(RoundsError::OutOfRange))]
    #[case::not_a_number("abc", Err(RoundsError::ParseError))]
    fn test_rounds_try_from_str(
        #[case] input: &str,
        #[case] expected: Result<Rounds, RoundsError>,
    ) {
        assert_eq!(Rounds::try_from(input), expected);
    }

    #[test]
    fn test_routine_move_part_from_unknown_section_keeps_routine_unchanged() {
        let mut routine = routine_with_sections(vec![section(1, vec![activity(1)])]);

        routine.move_part(&vec![0, 1].into(), &vec![0].into(), 0);

        assert_eq!(routine.sections, vec![section(1, vec![activity(1)])]);
    }

    #[test]
    fn test_routine_id_from_str() {
        let id = RoutineID::from(Uuid::from_u128(1));

        assert_eq!(id, RoutineID::from(1u128));
        assert_eq!(RoutineID::from_str(&id.to_string()), Ok(id));
    }

    fn any_activity() -> impl Strategy<Value = RoutinePart> {
        (1u32..10).prop_map(activity)
    }

    fn any_routine() -> impl Strategy<Value = Routine> {
        let part = prop_oneof![
            any_activity(),
            (1u32..3, prop::collection::vec(any_activity(), 0..3))
                .prop_map(|(rounds, parts)| section(rounds, parts)),
        ];
        prop::collection::vec(
            (1u32..3, prop::collection::vec(part, 0..4))
                .prop_map(|(rounds, parts)| section(rounds, parts)),
            0..4,
        )
        .prop_map(routine_with_sections)
    }

    fn any_path() -> impl Strategy<Value = RoutinePartPath> {
        prop::collection::vec(0usize..4, 0..4).prop_map(RoutinePartPath::from)
    }

    fn activities(parts: &[RoutinePart]) -> Vec<String> {
        let mut result = vec![];
        for part in parts {
            match part {
                RoutinePart::RoutineSection { parts, .. } => result.extend(activities(parts)),
                RoutinePart::RoutineActivity { .. } => result.push(format!("{part:?}")),
            }
        }
        result.sort();
        result
    }

    fn siblings(routine: &Routine, path: &RoutinePartPath) -> usize {
        if path.len() == 1 {
            routine.sections.len()
        } else if let Some(RoutinePart::RoutineSection { parts, .. }) =
            routine.part(&path[1..].to_vec().into())
        {
            parts.len()
        } else {
            0
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn test_routine_mutations_ignore_unresolvable_paths(
            routine in any_routine(),
            path in any_path(),
        ) {
            if routine.part(&path).is_some() {
                return Ok(());
            }

            for mutate in [
                Routine::remove_part,
                Routine::move_part_down,
                Routine::move_part_up,
            ] {
                let mut mutated = routine.clone();
                mutate(&mut mutated, &path);
                prop_assert_eq!(&mutated, &routine);
            }
        }

        #[test]
        fn test_routine_move_part_down_is_undone_by_move_part_up(
            routine in any_routine(),
            path in any_path(),
        ) {
            if routine.part(&path).is_none() {
                return Ok(());
            }
            let last = siblings(&routine, &path) - 1;
            let mut moved_path = path.to_vec();
            moved_path[0] = if path[0] == last { 0 } else { path[0] + 1 };

            let mut mutated = routine.clone();
            mutated.move_part_down(&path);
            mutated.move_part_up(&moved_path.into());

            prop_assert_eq!(mutated, routine);
        }

        #[test]
        fn test_routine_moves_preserve_the_activities(
            routine in any_routine(),
            moves in prop::collection::vec((any::<bool>(), any_path()), 0..8),
        ) {
            let expected = activities(&routine.sections);
            let mut mutated = routine;

            for (down, path) in moves {
                if down {
                    mutated.move_part_down(&path);
                } else {
                    mutated.move_part_up(&path);
                }
            }

            prop_assert_eq!(activities(&mutated.sections), expected);
        }

        #[test]
        fn test_routine_remove_part_removes_one_sibling(
            routine in any_routine(),
            path in any_path(),
        ) {
            if routine.part(&path).is_none() {
                return Ok(());
            }
            let expected = siblings(&routine, &path) - 1;

            let mut mutated = routine;
            mutated.remove_part(&path);

            prop_assert_eq!(siblings(&mutated, &path), expected);
        }
    }
}
