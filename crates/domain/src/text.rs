//! Text form of routines and the schedule.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Write as _},
};

use crate::{
    CreateError, Exercise, ExerciseID, ExerciseProperties, Name, RPE, ReadError, Recoverable, Reps,
    Rotation, RotationID, Rounds, Routine, RoutineID, RoutinePart, Schedule, ScheduleError,
    ScheduleSlot, SetPacing, Sides, Tempo, Time, Unreachable, UpdateError, ValidationError,
    Weekday, Weight, catalog,
    exercise::{catalog_exercise_by_name, normalized_name, singular_name},
    training::values_to_string,
    validate_routine_name,
};

const SEPARATOR: &str = " \u{2014} ";
/// The separator and the forms of it typed without the em dash.
const SEPARATORS: [&str; 3] = [SEPARATOR, " \u{2013} ", " - "];
const ROUTINE: &str = "Routine";
const ROTATION: &str = "Rotation";
const SCHEDULE: &str = "Schedule";

impl Routine {
    /// Writes the routine as text, naming each exercise as in `exercises`.
    #[must_use]
    pub fn to_text(&self, exercises: &[Exercise]) -> String {
        routine_lines(self, &|id| exercise_name(id, exercises)).join("\n")
    }
}

impl Schedule {
    /// Writes the schedule as text, followed by the text of each routine it uses, in the order of
    /// their first use.
    ///
    /// Days without a slot are left out. A routine missing from `routines` is written as a
    /// placeholder and has no text of its own.
    #[must_use]
    pub fn to_text(&self, routines: &[Routine], exercises: &[Exercise]) -> String {
        let mut used = vec![];
        let mut reference = |routine_id: RoutineID| {
            if !used.contains(&routine_id) {
                used.push(routine_id);
            }
            routine_reference(routine_id, routines)
        };

        let mut week = vec![];
        for weekday in Weekday::iter() {
            let Some(slots) = self.entries().get(weekday).filter(|s| !s.is_empty()) else {
                continue;
            };
            week.push(weekday.to_string());
            for slot in slots {
                match slot {
                    ScheduleSlot::Routine(routine_id) => {
                        week.push(format!("- {}", reference(*routine_id)));
                    }
                    ScheduleSlot::Rotation(rotation_id) => {
                        if let Some(rotation) = self.rotations().get(rotation_id) {
                            week.push(format!("- {ROTATION}{SEPARATOR}{}", rotation.name));
                        }
                    }
                }
            }
        }

        let mut rotations = self.rotations().values().collect::<Vec<_>>();
        rotations.sort_by(|a, b| a.name.cmp(&b.name));

        let mut blocks = vec![SCHEDULE.to_string()];
        if !week.is_empty() {
            blocks.push(week.join("\n"));
        }
        for rotation in rotations {
            let mut lines = vec![format!("{ROTATION}{SEPARATOR}{}", rotation.name)];
            for routine_id in rotation.routines() {
                lines.push(format!("- {}", reference(*routine_id)));
            }
            blocks.push(lines.join("\n"));
        }
        for routine_id in used {
            if let Some(routine) = routines.iter().find(|r| r.id == routine_id) {
                blocks.push(routine.to_text(exercises));
            }
        }
        blocks.join("\n\n")
    }
}

/// Names a routine in a slot or a rotation, prefixing a name that would otherwise be read as
/// naming a rotation or a routine by its prefix.
fn routine_reference(routine_id: RoutineID, routines: &[Routine]) -> String {
    match routines.iter().find(|r| r.id == routine_id) {
        Some(routine) if starts_with_kind(routine.name.as_ref()) => {
            format!("{ROUTINE}{SEPARATOR}{}", routine.name)
        }
        Some(routine) => routine.name.to_string(),
        None => format!("{ROUTINE}#{}", *routine_id),
    }
}

/// Whether `name` starts with `Routine` or `Rotation` followed by a separator.
fn starts_with_kind(name: &str) -> bool {
    [ROUTINE, ROTATION]
        .iter()
        .any(|kind| strip_kind(name, kind).is_some())
}

/// Returns the remainder of `line` after `kind` and a separator, which may lack its trailing space
/// at the end of the line.
fn strip_kind<'a>(line: &'a str, kind: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(kind)?;
    SEPARATORS.iter().find_map(|separator| {
        rest.strip_prefix(separator)
            .or_else(|| (rest == separator.trim_end()).then_some(""))
    })
}

fn exercise_name(exercise_id: ExerciseID, exercises: &[Exercise]) -> String {
    exercises.iter().find(|e| e.id == exercise_id).map_or_else(
        || format!("Exercise#{}", *exercise_id),
        |e| e.name.to_string(),
    )
}

fn routine_lines(routine: &Routine, exercise_name: &dyn Fn(ExerciseID) -> String) -> Vec<String> {
    let mut lines = vec![format!("{ROUTINE}{SEPARATOR}{}", routine.name)];
    if !routine.notes.is_empty() {
        lines.push(String::new());
        lines.extend(routine.notes.split('\n').map(ToString::to_string));
    }
    for (i, section) in routine.sections.iter().enumerate() {
        lines.push(String::new());
        lines.extend(part_lines(section, &section_label(i), 0, exercise_name));
    }
    lines
}

/// The label of the top-level section at `index`: a letter, or its number from the 27th section
/// on.
fn section_label(index: usize) -> String {
    u8::try_from(index)
        .ok()
        .map(|n| char::from(b'A'.saturating_add(n)))
        .filter(char::is_ascii_uppercase)
        .map_or_else(|| format!("#{}", index + 1), |c| c.to_string())
}

fn part_label(parent: &str, n: usize) -> String {
    if parent.ends_with(|c: char| c.is_ascii_digit()) {
        format!("{parent}.{n}")
    } else {
        format!("{parent}{n}")
    }
}

fn part_lines(
    part: &RoutinePart,
    label: &str,
    depth: usize,
    exercise_name: &dyn Fn(ExerciseID) -> String,
) -> Vec<String> {
    let indent = "  ".repeat(depth);
    let mut lines = vec![];
    match part {
        RoutinePart::RoutineSection { rounds, parts } => {
            let r = u32::from(*rounds);
            let sets_label = if r == 1 { "set" } else { "sets" };
            lines.push(format!("{indent}[{label}] {r} {sets_label}"));
            let mut counter = 1;
            for part in parts {
                match part {
                    RoutinePart::RoutineSection { .. } => {
                        let label = part_label(label, counter);
                        counter += 1;
                        lines.extend(part_lines(part, &label, depth + 1, exercise_name));
                        lines.push(String::new());
                    }
                    RoutinePart::RoutineActivity { exercise_id, .. } if exercise_id.is_nil() => {
                        lines.extend(part_lines(part, "", depth + 1, exercise_name));
                    }
                    RoutinePart::RoutineActivity { .. } => {
                        let label = part_label(label, counter);
                        counter += 1;
                        lines.extend(part_lines(part, &label, depth + 1, exercise_name));
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
            tempo,
            weight,
            rpe,
            automatic,
            countdown,
            sides,
        } => {
            let line = if exercise_id.is_nil() {
                let mut line = format!("{indent}Rest");
                if let Some(tempo) = tempo.non_zero() {
                    let _ = write!(line, "{SEPARATOR}{tempo}");
                }
                if !automatic {
                    let _ = write!(line, "{SEPARATOR}manual");
                }
                line
            } else {
                let name = exercise_name(*exercise_id);
                let targets = values_to_string(
                    reps.non_zero(),
                    weight.non_zero(),
                    rpe.non_zero(),
                    &tempo_text(*tempo, reps.non_zero().is_some()),
                );
                let mut line = format!("{indent}{label}{SEPARATOR}{name}");
                if !targets.is_empty() {
                    let _ = write!(line, "{SEPARATOR}{targets}");
                }
                if *sides == Sides::PerSide {
                    let _ = write!(line, "{SEPARATOR}per side");
                }
                if *countdown && SetPacing::countdown_applies(*tempo) {
                    let _ = write!(line, "{SEPARATOR}{COUNTDOWN}");
                }
                if *automatic {
                    let _ = write!(line, "{SEPARATOR}auto-start");
                }
                line
            };
            lines.push(line);
        }
    }
    lines
}

/// Writes a tempo following reps with its unit per repetition, as a bare time could be read as
/// the duration of the set.
fn tempo_text(tempo: Tempo, follows_reps: bool) -> String {
    if follows_reps && tempo.non_zero().is_some() {
        let phases = tempo
            .phases()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\u{00b7}");
        format!("{phases} s/rep")
    } else {
        tempo.to_string()
    }
}

/// A text read into routines and a schedule, which refer to exercises, routines and rotations by
/// name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedText {
    pub routines: Vec<ParsedRoutine>,
    pub schedule: Option<ParsedSchedule>,
}

impl ParsedText {
    /// The exercise names of all activities, each once, in the order of their first appearance.
    #[must_use]
    pub fn exercise_names(&self) -> Vec<&Name> {
        let mut names = vec![];
        for routine in &self.routines {
            for part in &routine.sections {
                part.collect_exercise_names(&mut names);
            }
        }
        names
    }

    /// The exercise names that match none of `exercises`, each once, in the order of their first
    /// appearance.
    ///
    /// Names that match each other are returned once, in their first spelling.
    #[must_use]
    pub fn unknown_exercises(&self, exercises: &[Exercise]) -> Vec<Name> {
        let mut result: Vec<Name> = vec![];
        for name in self.exercise_names() {
            if resolve_exercise(name, exercises).is_none()
                && !result
                    .iter()
                    .any(|n| singular_name(n.as_ref()) == singular_name(name.as_ref()))
            {
                result.push(name.clone());
            }
        }
        result
    }

    /// The errors of routine references in the schedule naming neither a routine of the text nor
    /// one of `routines`.
    #[must_use]
    pub fn unknown_routines(&self, routines: &[Routine]) -> Vec<TextError> {
        let Some(schedule) = &self.schedule else {
            return vec![];
        };
        let mut references = schedule
            .entries
            .values()
            .flatten()
            .filter_map(|slot| match slot {
                ParsedSlot::Routine(reference) => Some(reference),
                ParsedSlot::Rotation(_) => None,
            })
            .chain(schedule.rotations.iter().flat_map(|r| &r.routines))
            .filter(|reference| {
                !self.routines.iter().any(|r| r.name == reference.name)
                    && !routines.iter().any(|r| r.name == reference.name)
            })
            .collect::<Vec<_>>();
        references.sort_by_key(|reference| reference.line);
        references
            .into_iter()
            .map(|reference| {
                TextError::at(
                    reference.line,
                    format!("unknown routine \"{}\"", reference.name),
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedRoutine {
    pub line: usize,
    pub name: Name,
    pub notes: String,
    pub sections: Vec<ParsedPart>,
}

impl ParsedRoutine {
    /// The sections of the routine, with each exercise identified by `exercise_id`.
    pub fn sections(&self, exercise_id: &impl Fn(&Name) -> ExerciseID) -> Vec<RoutinePart> {
        self.sections
            .iter()
            .map(|part| part.to_routine_part(exercise_id))
            .collect()
    }

    /// Compares the routine with the routine of its name in `routines`, identifying each exercise
    /// as [`ImportService::import_text`] does with `choices`.
    #[must_use]
    pub fn change(
        &self,
        routines: &[Routine],
        exercises: &[Exercise],
        choices: &ImportChoices,
    ) -> RoutineChange {
        let Some(existing) = routines.iter().find(|r| r.name == self.name) else {
            return RoutineChange::New;
        };
        // An exercise that the import would create gets an unused ID, so that it differs from
        // every exercise of the existing routine.
        let used = existing.exercises();
        let plan = ExercisePlan::new(
            &self.exercise_names(),
            exercises,
            choices,
            (1..).map(ExerciseID::from).filter(|id| !used.contains(id)),
        );
        let sections = self.sections(&|name| plan.exercise_id(name));
        if self.matches(existing, &sections) {
            return RoutineChange::Unchanged;
        }
        let imported = Routine {
            sections,
            notes: self.notes.clone(),
            ..existing.clone()
        };
        let lines = |routine: &Routine| {
            let exercise_name = |id: ExerciseID| {
                plan.created
                    .iter()
                    .find(|created| created.id == id)
                    .map_or_else(|| exercise_name(id, exercises), |e| e.name.to_string())
            };
            routine_lines(routine, &exercise_name)
                .into_iter()
                .skip(1)
                .skip_while(String::is_empty)
                .collect::<Vec<_>>()
        };
        // Blank lines only separate sections, so adding or removing one changes nothing.
        let changes = line_changes(&lines(existing), &lines(&imported))
            .into_iter()
            .filter(|change| match change {
                LineChange::Kept(_) => true,
                LineChange::Removed(line) | LineChange::Added(line) => !line.is_empty(),
            })
            .collect();
        RoutineChange::Changed(changes)
    }

    /// Whether `routine` has `sections` and the notes of this routine, ignoring the trailing
    /// whitespace of each line, which the parser drops.
    pub(crate) fn matches(&self, routine: &Routine, sections: &[RoutinePart]) -> bool {
        routine
            .notes
            .split('\n')
            .map(str::trim_end)
            .eq(self.notes.split('\n'))
            && routine.sections == sections
    }

    fn exercise_names(&self) -> Vec<&Name> {
        let mut names = vec![];
        for part in &self.sections {
            part.collect_exercise_names(&mut names);
        }
        names
    }
}

/// How an imported routine differs from the existing routine of its name.
#[derive(Debug, Clone, PartialEq)]
pub enum RoutineChange {
    /// No routine has the name.
    New,
    Unchanged,
    /// The lines of the text of the existing routine changed into those of the imported one,
    /// without the line of the name and without added or removed blank lines.
    Changed(Vec<LineChange>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LineChange {
    Kept(String),
    Removed(String),
    Added(String),
}

/// The changes turning `old` into `new`, keeping a longest common subsequence of lines.
fn line_changes(old: &[String], new: &[String]) -> Vec<LineChange> {
    // `common[i][j]` is the length of a longest common subsequence of `old[i..]` and `new[j..]`.
    let mut common = vec![vec![0; new.len() + 1]; old.len() + 1];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            common[i][j] = if old[i] == new[j] {
                common[i + 1][j + 1] + 1
            } else {
                common[i + 1][j].max(common[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut changes = vec![];
    while i < old.len() || j < new.len() {
        if i < old.len() && j < new.len() && old[i] == new[j] {
            changes.push(LineChange::Kept(old[i].clone()));
            i += 1;
            j += 1;
        } else if i < old.len() && (j == new.len() || common[i + 1][j] >= common[i][j + 1]) {
            changes.push(LineChange::Removed(old[i].clone()));
            i += 1;
        } else {
            changes.push(LineChange::Added(new[j].clone()));
            j += 1;
        }
    }
    changes
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedPart {
    Section {
        rounds: Rounds,
        parts: Vec<ParsedPart>,
    },
    /// An activity of the named exercise, or a rest without an exercise.
    Activity {
        exercise: Option<Name>,
        reps: Reps,
        tempo: Tempo,
        weight: Weight,
        rpe: RPE,
        automatic: bool,
        countdown: bool,
        sides: Sides,
    },
}

impl ParsedPart {
    fn collect_exercise_names<'a>(&'a self, names: &mut Vec<&'a Name>) {
        match self {
            ParsedPart::Section { parts, .. } => {
                for part in parts {
                    part.collect_exercise_names(names);
                }
            }
            ParsedPart::Activity {
                exercise: Some(name),
                ..
            } => {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
            ParsedPart::Activity { exercise: None, .. } => {}
        }
    }

    fn to_routine_part(&self, exercise_id: &impl Fn(&Name) -> ExerciseID) -> RoutinePart {
        match self {
            ParsedPart::Section { rounds, parts } => RoutinePart::RoutineSection {
                rounds: *rounds,
                parts: parts
                    .iter()
                    .map(|part| part.to_routine_part(exercise_id))
                    .collect(),
            },
            ParsedPart::Activity {
                exercise,
                reps,
                tempo,
                weight,
                rpe,
                automatic,
                countdown,
                sides,
            } => RoutinePart::RoutineActivity {
                exercise_id: exercise.as_ref().map_or_else(ExerciseID::nil, exercise_id),
                reps: *reps,
                tempo: *tempo,
                weight: *weight,
                rpe: *rpe,
                automatic: *automatic,
                // The flag is kept only where it takes effect, as in `Routine::update_activity`.
                countdown: *countdown && exercise.is_some() && SetPacing::countdown_applies(*tempo),
                sides: *sides,
            },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedSchedule {
    pub line: usize,
    pub entries: BTreeMap<Weekday, Vec<ParsedSlot>>,
    pub rotations: Vec<ParsedRotation>,
}

impl ParsedSchedule {
    /// Builds the schedule, identifying each routine by `routine_id` and keeping the identifier of
    /// each rotation of `existing` with the name of a rotation of the text.
    ///
    /// Other rotations get identifiers following the highest identifier in use.
    pub fn to_schedule(
        &self,
        routine_id: impl Fn(&Name) -> Option<RoutineID>,
        existing: &Schedule,
    ) -> Result<Schedule, ImportError> {
        let resolve = |reference: &Reference| {
            routine_id(&reference.name)
                .ok_or_else(|| ImportError::UnknownRoutine(reference.name.clone()))
        };
        let mut next_id = existing
            .rotations()
            .keys()
            .map(|id| id.as_u128())
            .max()
            .unwrap_or(0);
        let mut rotations = BTreeMap::new();
        let mut rotation_ids = BTreeMap::new();
        for rotation in &self.rotations {
            let rotation_id = existing
                .rotations()
                .iter()
                .find(|(_, r)| r.name == rotation.name)
                .map_or_else(
                    || {
                        next_id += 1;
                        RotationID::from(next_id)
                    },
                    |(id, _)| *id,
                );
            let routines = rotation
                .routines
                .iter()
                .map(resolve)
                .collect::<Result<Vec<_>, _>>()?;
            rotations.insert(
                rotation_id,
                Rotation::new(rotation.name.clone(), routines)
                    .map_err(|err| ImportError::ReplaceSchedule(UpdateError::Other(err.into())))?,
            );
            rotation_ids.insert(rotation.name.clone(), rotation_id);
        }
        let mut entries = BTreeMap::new();
        for (weekday, slots) in &self.entries {
            let slots = slots
                .iter()
                .map(|slot| match slot {
                    ParsedSlot::Routine(reference) => resolve(reference).map(ScheduleSlot::Routine),
                    ParsedSlot::Rotation(reference) => rotation_ids
                        .get(&reference.name)
                        .map(|id| ScheduleSlot::Rotation(*id))
                        .ok_or_else(|| {
                            ImportError::ReplaceSchedule(UpdateError::Other(
                                ScheduleError::DanglingRotation.into(),
                            ))
                        }),
                })
                .collect::<Result<Vec<_>, _>>()?;
            if !slots.is_empty() {
                entries.insert(*weekday, slots);
            }
        }
        Schedule::new(rotations, entries)
            .map_err(|err| ImportError::ReplaceSchedule(UpdateError::Other(err.into())))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedSlot {
    Routine(Reference),
    Rotation(Reference),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedRotation {
    pub line: usize,
    pub name: Name,
    pub routines: Vec<Reference>,
}

/// A routine or rotation named on a line of the text.
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub line: usize,
    pub name: Name,
}

/// An error in a text, on the line of the given number counted from one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextError {
    pub line: Option<usize>,
    pub message: String,
}

impl TextError {
    fn at(line: usize, message: impl Into<String>) -> Self {
        Self {
            line: Some(line),
            message: message.into(),
        }
    }
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "line {line}: {}", self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

/// Reads the routines and the schedule of a text, ignoring any text outside of them.
pub fn parse_text(text: &str) -> Result<ParsedText, Vec<TextError>> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut result = ParsedText::default();
    let mut errors = vec![];
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if let Some(name) = strip_kind(line, ROUTINE) {
            let end = block_end(&lines, i + 1);
            match parse_routine(&lines, i, end, name) {
                Ok(routine) => {
                    if result.routines.iter().any(|r| r.name == routine.name) {
                        errors.push(TextError::at(
                            i + 1,
                            format!("routine \"{}\" appears twice", routine.name),
                        ));
                    }
                    result.routines.push(routine);
                }
                Err(routine_errors) => errors.extend(routine_errors),
            }
            i = end;
        } else if line == SCHEDULE {
            let end = block_end(&lines, i + 1);
            if result.schedule.is_some() {
                errors.push(TextError::at(i + 1, "schedule appears twice"));
            }
            match parse_schedule(&lines, i, end) {
                Ok(schedule) => result.schedule = Some(schedule),
                Err(schedule_errors) => errors.extend(schedule_errors),
            }
            i = end;
        } else {
            if starts_block(&undecorated(line)) {
                errors.push(decoration_error(i));
            }
            i += 1;
        }
    }
    if errors.is_empty() && result.routines.is_empty() && result.schedule.is_none() {
        errors.push(TextError {
            line: None,
            message: "text contains no routine and no schedule".to_string(),
        });
    }
    if errors.is_empty() {
        Ok(result)
    } else {
        Err(errors)
    }
}

/// The index of the line ending the block whose body starts at `start`: the start of the next
/// block, a Markdown fence or the end of the text.
fn block_end(lines: &[&str], start: usize) -> usize {
    (start..lines.len())
        .find(|&i| {
            let line = lines[i].trim();
            starts_block(line) || line.starts_with(FENCE)
        })
        .unwrap_or(lines.len())
}

const FENCE: &str = "```";

fn starts_block(line: &str) -> bool {
    line == SCHEDULE || strip_kind(line, ROUTINE).is_some()
}

/// Removes a leading heading marker, a leading list marker and emphasis.
fn undecorated(line: &str) -> String {
    let mut line = line.trim();
    let heading = line.trim_start_matches('#');
    if heading.len() < line.len() && heading.starts_with(' ') {
        line = heading.trim_start();
    }
    if let Some(item) = list_item(line) {
        line = item;
    }
    line.replace("**", "")
        .replace("__", "")
        .trim_matches(['*', '_'])
        .trim()
        .to_string()
}

fn decoration_error(index: usize) -> TextError {
    TextError::at(index + 1, "unexpected Markdown formatting")
}

fn list_item(line: &str) -> Option<&str> {
    line.strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .map(str::trim)
}

fn parse_routine(
    lines: &[&str],
    start: usize,
    end: usize,
    name: &str,
) -> Result<ParsedRoutine, Vec<TextError>> {
    let mut errors = vec![];
    let name = Name::new(name).map_err(|err| TextError::at(start + 1, err.to_string()));

    let mut i = start + 1;
    let mut notes = vec![];
    while i < end && !starts_section(lines[i].trim()) {
        let line = lines[i].trim();
        let undecorated = undecorated(line);
        if undecorated != line && (starts_block(&undecorated) || starts_labeled_part(&undecorated))
        {
            errors.push(decoration_error(i));
        } else if starts_labeled_part(line) {
            errors.push(TextError::at(i + 1, "activity outside of a section"));
        } else {
            notes.push(lines[i].trim_end());
        }
        i += 1;
    }
    while notes.first().is_some_and(|l| l.is_empty()) {
        notes.remove(0);
    }
    while notes.last().is_some_and(|l| l.is_empty()) {
        notes.pop();
    }

    let mut part_lines = vec![];
    let mut blank_before = false;
    for (index, raw) in lines.iter().enumerate().take(end).skip(i) {
        let line = raw.trim();
        if line.is_empty() {
            blank_before = true;
            continue;
        }
        let undecorated = undecorated(line);
        if undecorated != line && (starts_block(&undecorated) || starts_part(&undecorated)) {
            errors.push(decoration_error(index));
            continue;
        }
        match parse_part_line(line) {
            Some(Ok(kind)) => {
                part_lines.push(PartLine {
                    line: index + 1,
                    indent: raw.len() - raw.trim_start().len(),
                    blank_before,
                    kind,
                });
                blank_before = false;
            }
            Some(Err(message)) => errors.push(TextError::at(index + 1, message)),
            None if lacks_separator_after_label(line) => errors.push(TextError::at(
                index + 1,
                format!("label must be followed by \"{}\"", SEPARATOR.trim()),
            )),
            None if lacks_label(line) => {
                errors.push(TextError::at(index + 1, "activity must start with a label"));
            }
            None => {}
        }
    }

    let sections = nest(part_lines).unwrap_or_else(|nesting_errors| {
        errors.extend(nesting_errors);
        vec![]
    });

    match name {
        Ok(name) if errors.is_empty() => Ok(ParsedRoutine {
            line: start + 1,
            name,
            notes: notes.join("\n"),
            sections,
        }),
        Ok(_) => Err(errors),
        Err(error) => {
            errors.insert(0, error);
            Err(errors)
        }
    }
}

struct PartLine {
    line: usize,
    indent: usize,
    blank_before: bool,
    kind: PartKind,
}

enum PartKind {
    Section { label: String, rounds: Rounds },
    Activity { label: String, part: ParsedPart },
    Rest(ParsedPart),
}

/// Builds the sections from the part lines, placing each part by the depth of its label, or by
/// its indentation if its label is outside the numbering scheme or it is a rest.
///
/// In a routine without any indentation, a rest belongs to the innermost open section, and a blank
/// line before it closes a nested section.
fn nest(part_lines: Vec<PartLine>) -> Result<Vec<ParsedPart>, Vec<TextError>> {
    struct Open {
        rounds: Rounds,
        parts: Vec<ParsedPart>,
    }

    fn close(stack: &mut Vec<Open>, sections: &mut Vec<ParsedPart>, len: usize) {
        while stack.len() > len {
            let Some(open) = stack.pop() else { break };
            let section = ParsedPart::Section {
                rounds: open.rounds,
                parts: open.parts,
            };
            match stack.last_mut() {
                Some(parent) => parent.parts.push(section),
                None => sections.push(section),
            }
        }
    }

    let indented = part_lines.iter().any(|p| p.indent > 0);
    let mut errors = vec![];
    let mut sections = vec![];
    let mut stack: Vec<Open> = vec![];
    let mut history: Vec<(usize, usize)> = vec![];

    for part_line in part_lines {
        let (label, is_section) = match &part_line.kind {
            PartKind::Section { label, .. } => (Some(label.as_str()), true),
            PartKind::Activity { label, .. } => (Some(label.as_str()), false),
            PartKind::Rest(_) => (None, false),
        };
        let depth = match label.and_then(label_depth) {
            Some(depth) => depth,
            None if indented => depth_from_indent(part_line.indent, &history),
            None if is_section => 1,
            None => {
                let innermost = if part_line.blank_before && stack.len() > 1 {
                    stack.len() - 1
                } else {
                    stack.len()
                };
                innermost + 1
            }
        };
        if depth == 1 && !is_section {
            errors.push(TextError::at(
                part_line.line,
                "activity outside of a section",
            ));
            continue;
        }
        if depth - 1 > stack.len() {
            errors.push(TextError::at(
                part_line.line,
                "part is more than one level below the enclosing section",
            ));
            continue;
        }
        history.push((part_line.indent, depth));
        close(&mut stack, &mut sections, depth - 1);
        match part_line.kind {
            PartKind::Section { rounds, .. } => stack.push(Open {
                rounds,
                parts: vec![],
            }),
            PartKind::Activity { part, .. } | PartKind::Rest(part) => {
                if let Some(parent) = stack.last_mut() {
                    parent.parts.push(part);
                }
            }
        }
    }
    close(&mut stack, &mut sections, 0);

    if errors.is_empty() {
        Ok(sections)
    } else {
        Err(errors)
    }
}

/// The depth of a label of the numbering scheme: a letter or `#` and a number at the top level,
/// with a number appended for each further level, preceded by `.` if the previous level ends in a
/// digit.
fn label_depth(label: &str) -> Option<usize> {
    let (mut rest, mut ends_in_digit) = if let Some(rest) = label.strip_prefix('#') {
        let digits = leading_digits(rest);
        if digits == 0 {
            return None;
        }
        (&rest[digits..], true)
    } else {
        if !label.chars().next()?.is_ascii_alphabetic() {
            return None;
        }
        (&label[1..], false)
    };
    let mut depth = 1;
    while !rest.is_empty() {
        if ends_in_digit {
            rest = rest.strip_prefix('.')?;
        }
        let digits = leading_digits(rest);
        if digits == 0 {
            return None;
        }
        rest = &rest[digits..];
        depth += 1;
        ends_in_digit = true;
    }
    Some(depth)
}

fn leading_digits(text: &str) -> usize {
    text.len() - text.trim_start_matches(|c: char| c.is_ascii_digit()).len()
}

/// The depth of a part relative to the indentation and depth of the preceding parts.
fn depth_from_indent(indent: usize, history: &[(usize, usize)]) -> usize {
    match history.last() {
        None => 1,
        Some(&(previous_indent, previous_depth)) if indent > previous_indent => previous_depth + 1,
        _ => history
            .iter()
            .rev()
            .find(|(i, _)| *i <= indent)
            .map_or(1, |&(i, depth)| if i == indent { depth } else { depth + 1 }),
    }
}

/// Whether a line starts like a part: a section, a rest or a single word followed by a separator.
fn starts_part(line: &str) -> bool {
    starts_section(line) || parse_part_line(line).is_some()
}

/// Whether a line starts like a section or like a part whose label belongs to the numbering
/// scheme.
fn starts_labeled_part(line: &str) -> bool {
    starts_section(line)
        || (parse_part_line(line).is_some() && label_depth(segments(line)[0].text(line)).is_some())
}

/// Whether a line starts with a label of a nested part followed by a space instead of a separator,
/// such as `A1 Squat — 5` or `A1. Squat — 5`.
fn lacks_separator_after_label(line: &str) -> bool {
    segments(line).len() > 1
        && line.split_whitespace().next().is_some_and(|word| {
            label_depth(word.trim_end_matches(['.', ':', ')'])).is_some_and(|depth| depth > 1)
        })
}

/// Whether a line starts with a label in brackets followed by a number.
fn starts_section(line: &str) -> bool {
    line.strip_prefix('[')
        .and_then(|rest| rest.split_once(']'))
        .is_some_and(|(_, rest)| rest.trim_start().starts_with(|c: char| c.is_ascii_digit()))
}

/// Whether a line has a separator but its first segment, containing a space, cannot be a label,
/// such as `Bench Press — 5`.
fn lacks_label(line: &str) -> bool {
    let segments = segments(line);
    segments.len() > 1 && segments[0].text(line).contains(char::is_whitespace)
}

/// Parses a line starting like a part, or returns `None` for any other line.
fn parse_part_line(line: &str) -> Option<Result<PartKind, String>> {
    if line.eq_ignore_ascii_case(REST) {
        return Some(Ok(PartKind::Rest(rest(Tempo::default(), true))));
    }
    if starts_section(line) {
        return Some(parse_section(line));
    }
    let segments = segments(line);
    let label = segments.first()?.text(line);
    if segments.len() < 2 || label.is_empty() || label.contains(char::is_whitespace) {
        return None;
    }
    if label.eq_ignore_ascii_case(REST) {
        return Some(parse_rest(line, &segments[1..]));
    }
    Some(
        parse_activity(line, &segments).map(|part| PartKind::Activity {
            label: label.to_string(),
            part,
        }),
    )
}

const REST: &str = "Rest";

fn parse_section(line: &str) -> Result<PartKind, String> {
    let (label, rest) = line
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']'))
        .unwrap_or_default();
    match rest.split_whitespace().collect::<Vec<_>>()[..] {
        [rounds, "set" | "sets" | "round" | "rounds"] => Ok(PartKind::Section {
            label: label.trim().to_string(),
            rounds: Rounds::try_from(rounds).map_err(|err| err.to_string())?,
        }),
        _ => Err("section must be followed by a number of sets".to_string()),
    }
}

fn parse_rest(line: &str, segments: &[Segment]) -> Result<PartKind, String> {
    let mut automatic = true;
    let mut end = segments.len();
    while end > 0 {
        match segments[end - 1].text(line) {
            "manual" => automatic = false,
            "auto-start" => {}
            PER_SIDE => return Err("rest must not be performed per side".to_string()),
            _ => break,
        }
        end -= 1;
    }
    let tempo = match &segments[..end] {
        [] => Tempo::default(),
        [segment] if has_values_form(segment.text(line)) => {
            match parse_values(segment.text(line))? {
                Values {
                    reps: None,
                    weight: None,
                    rpe: None,
                    tempo,
                } if tempo.phases().len() <= 1 => tempo,
                _ => return Err("rest must have no values other than a time".to_string()),
            }
        }
        _ => return Err("rest must have no values other than a time".to_string()),
    };
    Ok(PartKind::Rest(rest(tempo, automatic)))
}

fn rest(tempo: Tempo, automatic: bool) -> ParsedPart {
    ParsedPart::Activity {
        exercise: None,
        reps: Reps::default(),
        tempo,
        weight: Weight::default(),
        rpe: RPE::ZERO,
        automatic,
        countdown: false,
        sides: Sides::Combined,
    }
}

const PER_SIDE: &str = "per side";
const COUNTDOWN: &str = "countdown";

/// Parses an activity line, reading its segments from the right: the suffixes, then the values if
/// an exercise name precedes them, and the remainder as the exercise name.
fn parse_activity(line: &str, segments: &[Segment]) -> Result<ParsedPart, String> {
    let mut automatic = false;
    let mut countdown = false;
    let mut sides = Sides::Combined;
    let mut end = segments.len();
    while end > 1 {
        match segments[end - 1].text(line) {
            "auto-start" => automatic = true,
            "manual" => {}
            COUNTDOWN => countdown = true,
            PER_SIDE => sides = Sides::PerSide,
            _ => break,
        }
        end -= 1;
    }
    let mut values = Values::default();
    if end > 2 && has_values_form(segments[end - 1].text(line)) {
        values = parse_values(segments[end - 1].text(line))?;
        end -= 1;
    }
    if end < 2 {
        return Err("exercise name is missing".to_string());
    }
    // A label outside the numbering scheme followed only by values, such as `Plank — 30 s`, is
    // more likely an exercise name than a label.
    if end == 2
        && label_depth(segments[0].text(line)).is_none()
        && has_values_form(segments[1].text(line))
    {
        return Err("activity must start with a label".to_string());
    }
    let name = &line[segments[1].start..segments[end - 1].end];
    if name.trim().eq_ignore_ascii_case(REST) {
        return Err("rest must not have a label".to_string());
    }
    Ok(ParsedPart::Activity {
        exercise: Some(Name::new(name).map_err(|err| err.to_string())?),
        reps: values.reps.unwrap_or_default(),
        tempo: values.tempo,
        weight: values.weight.unwrap_or_default(),
        rpe: values.rpe.unwrap_or_default(),
        automatic,
        countdown,
        sides,
    })
}

/// The text between two separators, as a byte range of the line.
struct Segment {
    start: usize,
    end: usize,
}

impl Segment {
    fn text<'a>(&self, line: &'a str) -> &'a str {
        line[self.start..self.end].trim()
    }
}

/// Splits a line at each separator, including a separator at its end that lost its trailing
/// space.
///
/// A line containing the em dash separator is only split at em dashes, so that a hyphen or an en
/// dash surrounded by spaces belongs to the exercise name.
fn segments(line: &str) -> Vec<Segment> {
    let separators = if line.contains(SEPARATOR) || line.ends_with(SEPARATOR.trim_end()) {
        &SEPARATORS[..1]
    } else {
        &SEPARATORS[..]
    };
    let mut segments = vec![];
    let mut start = 0;
    let mut position = 0;
    while position < line.len() {
        let rest = &line[position..];
        let separator = separators.iter().find_map(|separator| {
            if rest.starts_with(separator) {
                Some(separator.len())
            } else if rest == separator.trim_end() {
                Some(rest.len())
            } else {
                None
            }
        });
        if let Some(len) = separator {
            segments.push(Segment {
                start,
                end: position,
            });
            position += len;
            start = position;
        } else {
            position += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    segments.push(Segment {
        start,
        end: line.len(),
    });
    segments
}

#[derive(Default)]
struct Values {
    reps: Option<Reps>,
    weight: Option<Weight>,
    rpe: Option<RPE>,
    tempo: Tempo,
}

/// Whether a segment consists only of numbers, units and the symbols of values.
fn has_values_form(segment: &str) -> bool {
    segment.contains(|c: char| c.is_ascii_digit())
        && segment
            .replace("s/rep", "")
            .replace("kg", "")
            .replace('s', "")
            .chars()
            .all(|c| {
                c.is_ascii_digit() || c.is_whitespace() || ".,-@()*xX\u{00b7}\u{00d7}".contains(c)
            })
}

/// Parses the values of an activity: reps and a weight joined by a multiplication sign, an RPE
/// following `@` and a tempo in parentheses, or a tempo standing on its own without reps and
/// weight.
fn parse_values(segment: &str) -> Result<Values, String> {
    let mut text = segment.trim();
    let mut values = Values::default();

    let mut parenthesized = None;
    if let Some(stripped) = text.strip_suffix(')') {
        let open = stripped
            .rfind('(')
            .ok_or_else(|| "parenthesis is not opened".to_string())?;
        parenthesized = Some(stripped[open + 1..].trim());
        text = stripped[..open].trim();
    }

    if let Some((before, after)) = text.split_once('@') {
        values.rpe = Some(RPE::try_from(after.trim()).map_err(|err| err.to_string())?);
        text = before.trim();
    }

    if let Some((reps, weight)) = text.split_once(['\u{00d7}', 'x', 'X', '*']) {
        values.reps = Some(parse_reps(reps)?);
        values.weight = Some(parse_weight(weight)?);
    } else if text.ends_with("kg") {
        values.weight = Some(parse_weight(text)?);
    } else if !text.is_empty() {
        if text.contains(['\u{00b7}', '-', 's']) || text.matches('.').count() > 1 {
            values.tempo = parse_tempo(text, TempoPosition::Standalone)?;
        } else {
            values.reps = Some(parse_reps(text)?);
        }
    }

    if let Some(parenthesized) = parenthesized {
        let position = if values.reps.is_some() {
            TempoPosition::AfterReps
        } else if values.weight.is_some() {
            TempoPosition::AfterWeight
        } else {
            return Err("time in parentheses must follow reps or a weight".to_string());
        };
        values.tempo = parse_tempo(parenthesized, position)?;
    }

    Ok(values)
}

fn parse_reps(text: &str) -> Result<Reps, String> {
    Reps::try_from(text.trim()).map_err(|err| err.to_string())
}

fn parse_weight(text: &str) -> Result<Weight, String> {
    let text = text
        .trim()
        .strip_suffix("kg")
        .ok_or_else(|| "weight must be followed by \"kg\"".to_string())?;
    Weight::try_from(text.trim()).map_err(|err| err.to_string())
}

#[derive(Clone, Copy, PartialEq)]
enum TempoPosition {
    Standalone,
    AfterReps,
    AfterWeight,
}

/// Parses a tempo, accepting `.` and `-` in place of `·`.
///
/// A single `.` between two numbers is read as a decimal, and two phases joined by `-` standing
/// on their own as a range, both of which are rejected, as is a bare ` s` after several phases.
fn parse_tempo(text: &str, position: TempoPosition) -> Result<Tempo, String> {
    let (body, unit) = if let Some(body) = text.strip_suffix("s/rep") {
        (body.trim(), Some("s/rep"))
    } else if let Some(body) = text.strip_suffix('s') {
        (body.trim(), Some("s"))
    } else {
        (text.trim(), None)
    };
    let is_decimal = body.matches('.').count() == 1 && !body.contains(['\u{00b7}', '-']);
    let phases = if is_decimal {
        vec![body]
    } else {
        body.split(['\u{00b7}', '.', '-']).collect::<Vec<_>>()
    };
    if phases.len() > 1 && unit == Some("s") {
        return Err("tempo of several phases must not be followed by \"s\"".to_string());
    }
    if phases.len() == 1 && position == TempoPosition::AfterReps && unit != Some("s/rep") {
        return Err("time following reps must be followed by \"s/rep\"".to_string());
    }
    if phases.len() == 2 && position == TempoPosition::Standalone && body.contains('-') {
        return Err("two phases joined by \"-\" must be joined by \"\u{00b7}\"".to_string());
    }
    let phases = phases
        .into_iter()
        .map(|phase| {
            Time::try_from(phase.trim())
                .map(u32::from)
                .map_err(|err| err.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Tempo::new(&phases).map_err(|err| err.to_string())
}

fn parse_schedule(
    lines: &[&str],
    start: usize,
    end: usize,
) -> Result<ParsedSchedule, Vec<TextError>> {
    let mut schedule = ParsedSchedule {
        line: start + 1,
        ..ParsedSchedule::default()
    };
    let mut errors = vec![];
    let mut days = BTreeSet::new();
    let mut heading = Heading::Schedule;
    let mut directly_below = true;

    for (index, raw) in lines.iter().enumerate().take(end).skip(start + 1) {
        let line = raw.trim();
        let number = index + 1;
        if line.is_empty() {
            directly_below = false;
            continue;
        }
        if let Some(item) = list_item(line).or_else(|| (line == "-" || line == "*").then_some("")) {
            directly_below = true;
            if let Err(error) = add_list_item(&mut schedule, &heading, item, number) {
                errors.push(error);
            }
            continue;
        }
        if let Some(weekday) = Weekday::iter().find(|w| w.to_string() == line) {
            directly_below = true;
            if !days.insert(*weekday) {
                errors.push(TextError::at(number, format!("{weekday} appears twice")));
            }
            heading = Heading::Day(*weekday);
            continue;
        }
        if let Some(name) = strip_kind(line, ROTATION) {
            directly_below = true;
            match Name::new(name) {
                Ok(name) => {
                    if schedule.rotations.iter().any(|r| r.name == name) {
                        errors.push(TextError::at(
                            number,
                            format!("rotation \"{name}\" appears twice"),
                        ));
                    }
                    schedule.rotations.push(ParsedRotation {
                        line: number,
                        name,
                        routines: vec![],
                    });
                    heading = Heading::Rotation(schedule.rotations.len() - 1);
                }
                Err(err) => errors.push(TextError::at(number, err.to_string())),
            }
            continue;
        }
        if starts_block(&undecorated(line)) {
            errors.push(decoration_error(index));
        } else if directly_below {
            errors.push(TextError::at(
                number,
                "line must be a weekday, a rotation or a list item starting with \"- \"",
            ));
        } else if starts_with_weekday(line) {
            errors.push(TextError::at(
                number,
                "weekday must stand on a line of its own, followed by its routines as list items",
            ));
        }
    }

    errors.extend(undefined_rotations(&schedule));
    if errors.is_empty() {
        Ok(schedule)
    } else {
        errors.sort_by_key(|error| error.line);
        Err(errors)
    }
}

/// The heading of the schedule block that list items belong to.
enum Heading {
    Schedule,
    Day(Weekday),
    Rotation(usize),
}

fn add_list_item(
    schedule: &mut ParsedSchedule,
    heading: &Heading,
    item: &str,
    line: usize,
) -> Result<(), TextError> {
    match heading {
        Heading::Schedule => Err(TextError::at(
            line,
            "list item before the first weekday or rotation",
        )),
        Heading::Day(weekday) => {
            let slot = parse_slot(item, line)?;
            schedule.entries.entry(*weekday).or_default().push(slot);
            Ok(())
        }
        Heading::Rotation(index) => {
            let reference = parse_rotation_routine(item, line)?;
            let rotation = &mut schedule.rotations[*index];
            if rotation.routines.iter().any(|r| r.name == reference.name) {
                return Err(TextError::at(
                    line,
                    format!(
                        "routine \"{}\" appears twice in rotation \"{}\"",
                        reference.name, rotation.name
                    ),
                ));
            }
            rotation.routines.push(reference);
            Ok(())
        }
    }
}

/// The errors of slots naming a rotation the schedule does not define.
fn undefined_rotations(schedule: &ParsedSchedule) -> Vec<TextError> {
    schedule
        .entries
        .values()
        .flatten()
        .filter_map(|slot| match slot {
            ParsedSlot::Rotation(reference)
                if !schedule.rotations.iter().any(|r| r.name == reference.name) =>
            {
                Some(TextError::at(
                    reference.line,
                    format!("unknown rotation \"{}\"", reference.name),
                ))
            }
            ParsedSlot::Rotation(_) | ParsedSlot::Routine(_) => None,
        })
        .collect()
}

fn parse_slot(item: &str, line: usize) -> Result<ParsedSlot, TextError> {
    if let Some(name) = strip_kind(item, ROTATION) {
        return reference(name, line).map(ParsedSlot::Rotation);
    }
    reference(strip_kind(item, ROUTINE).unwrap_or(item), line).map(ParsedSlot::Routine)
}

fn parse_rotation_routine(item: &str, line: usize) -> Result<Reference, TextError> {
    if strip_kind(item, ROTATION).is_some() {
        return Err(TextError::at(line, "rotation must not contain a rotation"));
    }
    reference(strip_kind(item, ROUTINE).unwrap_or(item), line)
}

fn reference(name: &str, line: usize) -> Result<Reference, TextError> {
    Name::new(name)
        .map(|name| Reference { line, name })
        .map_err(|err| TextError::at(line, err.to_string()))
}

/// Whether the first word of a line is a weekday, ignoring Markdown emphasis, punctuation and
/// case.
fn starts_with_weekday(line: &str) -> bool {
    let word = line
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .split(|c: char| !c.is_alphabetic())
        .next()
        .unwrap_or_default();
    Weekday::iter().any(|w| w.to_string().eq_ignore_ascii_case(word))
}

/// Finds the exercise of `exercises` a name refers to.
///
/// An exact spelling is preferred over a match ignoring case, hyphens and a plural on the last
/// word, which must be unique. Without such a match, the name is mapped to a catalog exercise by
/// an exact name or alias, and an exercise is found if it is the only one mapped to the same
/// catalog exercise.
#[must_use]
pub fn resolve_exercise(name: &Name, exercises: &[Exercise]) -> Option<ExerciseID> {
    if let Some(exercise) = exercises.iter().find(|e| e.name == *name) {
        return Some(exercise.id);
    }
    for key in [normalized_name, singular_name] {
        let matching = exercises
            .iter()
            .filter(|e| key(e.name.as_ref()) == key(name.as_ref()))
            .collect::<Vec<_>>();
        match matching[..] {
            [exercise] => return Some(exercise.id),
            [] => {}
            _ => return None,
        }
    }
    let (catalog_name, _) = catalog_exercise_by_name(name.as_ref())?;
    match exercises
        .iter()
        .filter(|e| {
            catalog_exercise_by_name(e.name.as_ref()).is_some_and(|(n, _)| n == catalog_name)
        })
        .collect::<Vec<_>>()[..]
    {
        [exercise] => Some(exercise.id),
        _ => None,
    }
}

/// Validates the name under which the imported routine `routine` is saved as a copy.
///
/// The name must be used neither by one of `routines` nor by another routine the import creates:
/// a routine of the text that collides with none of `routines`, or a copy in `copies`.
pub fn validate_copy_name(
    input: &str,
    routine: &Name,
    text: &ParsedText,
    routines: &[Routine],
    copies: &BTreeMap<Name, Name>,
) -> Result<Name, ValidationError> {
    let name = validate_routine_name(input, RoutineID::nil(), routines)?;
    let created = text
        .routines
        .iter()
        .map(|r| &r.name)
        .filter(|n| !routines.iter().any(|r| r.name == **n))
        .chain(
            copies
                .iter()
                .filter(|(original, _)| *original != routine)
                .map(|(_, copy)| copy),
        );
    if created.into_iter().any(|n| *n == name) {
        Err(ValidationError::Conflict("name".to_string()))
    } else {
        Ok(name)
    }
}

/// The first name of the form "`routine` (n)" with n ≥ 2 that [`validate_copy_name`] accepts.
///
/// The name of `routine` is shortened if the suffix would exceed the maximum length of a name.
#[must_use]
pub fn free_copy_name(
    routine: &Name,
    text: &ParsedText,
    routines: &[Routine],
    copies: &BTreeMap<Name, Name>,
) -> Name {
    let mut n = 2;
    loop {
        let suffix = format!(" ({n})");
        let mut base = routine.as_ref().as_str();
        while base.len() + suffix.len() > Name::MAX_LEN {
            base = base[..base.floor_char_boundary(base.len() - 1)].trim_end();
        }
        if let Ok(name) =
            validate_copy_name(&format!("{base}{suffix}"), routine, text, routines, copies)
        {
            return name;
        }
        n += 1;
    }
}

/// The decisions on names of a text that collide with or match none of the existing ones.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportChoices {
    /// The choice for an exercise name. A name without a choice is resolved as
    /// [`ImportChoices::exercise`] describes.
    pub exercises: BTreeMap<Name, ExerciseChoice>,
    /// The name of the copy for a routine whose name is already used. A routine without a copy
    /// replaces the routine of its name.
    pub copies: BTreeMap<Name, Name>,
}

impl ImportChoices {
    /// What an import makes of the exercise name `name`: the choice for it or a name matching it,
    /// otherwise the one of `exercises` it matches, otherwise the catalog exercise it matches,
    /// otherwise a new exercise.
    ///
    /// A catalog exercise whose name one of `exercises` has is that exercise.
    #[must_use]
    pub fn exercise(&self, name: &Name, exercises: &[Exercise]) -> ExerciseChoice {
        let choice = self
            .exercises
            .iter()
            .find(|(n, _)| singular_name(n.as_ref()) == singular_name(name.as_ref()))
            .map(|(_, choice)| choice.clone())
            .or_else(|| resolve_exercise(name, exercises).map(ExerciseChoice::Existing))
            .or_else(|| {
                catalog_exercise_by_name(name.as_ref())
                    .map(|(catalog_name, _)| ExerciseChoice::Catalog(catalog_name.clone()))
            })
            .unwrap_or(ExerciseChoice::New);
        match choice {
            ExerciseChoice::Catalog(catalog_name) => exercises
                .iter()
                .find(|e| e.name == catalog_name)
                .map_or(ExerciseChoice::Catalog(catalog_name), |e| {
                    ExerciseChoice::Existing(e.id)
                }),
            choice => choice,
        }
    }
}

/// The exercises an import creates for the exercise names of a text, and the exercise each name
/// refers to.
pub(crate) struct ExercisePlan {
    /// The exercises to create, in order, each with a placeholder ID.
    pub created: Vec<Exercise>,
    ids: BTreeMap<Name, ExerciseID>,
}

impl ExercisePlan {
    /// Plans the exercises for `names` as [`ImportChoices::exercise`] chooses them, a name
    /// matching an exercise created for an earlier name referring to it. The placeholder IDs are
    /// taken from `ids`, skipping those of `exercises`.
    pub fn new(
        names: &[&Name],
        exercises: &[Exercise],
        choices: &ImportChoices,
        ids: impl Iterator<Item = ExerciseID>,
    ) -> Self {
        let mut ids = ids.filter(|id| !exercises.iter().any(|e| e.id == *id));
        let mut all = exercises.to_vec();
        let mut plan = Self {
            created: vec![],
            ids: BTreeMap::new(),
        };
        for name in names {
            let (new_name, properties) = match choices.exercise(name, &all) {
                ExerciseChoice::Existing(id) => {
                    plan.ids.insert((*name).clone(), id);
                    continue;
                }
                ExerciseChoice::Catalog(catalog_name) => {
                    let properties = catalog::exercise(&catalog_name)
                        .map(ExerciseProperties::from)
                        .unwrap_or_default();
                    (catalog_name, properties)
                }
                ExerciseChoice::New => {
                    if let Some(id) = resolve_exercise(name, &all) {
                        plan.ids.insert((*name).clone(), id);
                        continue;
                    }
                    ((*name).clone(), ExerciseProperties::default())
                }
            };
            let exercise = Exercise {
                id: ids.next().unwrap_or_default(),
                name: new_name,
                notes: String::new(),
                muscles: properties.muscles,
                force: properties.force,
                mechanic: properties.mechanic,
                laterality: properties.laterality,
                assistance: properties.assistance,
                equipment: properties.equipment,
                category: properties.category,
            };
            plan.ids.insert((*name).clone(), exercise.id);
            all.push(exercise.clone());
            plan.created.push(exercise);
        }
        plan
    }

    /// The ID of the exercise `name` refers to, or the nil ID for a name that was not planned.
    pub fn exercise_id(&self, name: &Name) -> ExerciseID {
        self.ids.get(name).copied().unwrap_or_default()
    }
}

/// What an exercise name of a text becomes in an import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExerciseChoice {
    Existing(ExerciseID),
    /// A new exercise with the name and the properties of the catalog exercise of this name.
    Catalog(Name),
    /// A new exercise without properties, named as in the text.
    New,
}

impl ExerciseChoice {
    /// The name of the exercise created for the exercise name `name`, or `None` if none is
    /// created.
    #[must_use]
    pub fn created_name(&self, name: &Name) -> Option<Name> {
        match self {
            ExerciseChoice::Existing(_) => None,
            ExerciseChoice::Catalog(catalog_name) => Some(catalog_name.clone()),
            ExerciseChoice::New => Some(name.clone()),
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait ImportService {
    /// Saves the routines and the schedule of a text: creates its unknown exercises, then creates
    /// or replaces its routines, then replaces the schedule if the text has one. A routine
    /// replacing an equal routine is not saved.
    ///
    /// A failing step stops the import, keeping the steps saved before it.
    async fn import_text(
        &self,
        text: &ParsedText,
        choices: &ImportChoices,
    ) -> Result<(), ImportError>;
}

#[derive(thiserror::Error, Debug)]
pub enum ImportError {
    #[error(transparent)]
    Read(#[from] ReadError),
    #[error("{1}")]
    CreateExercise(Name, CreateError),
    #[error("{1}")]
    CreateRoutine(Name, CreateError),
    #[error("{1}")]
    ReplaceRoutine(Name, UpdateError),
    #[error("unknown routine \"{0}\"")]
    UnknownRoutine(Name),
    #[error(transparent)]
    ReplaceSchedule(UpdateError),
}

impl ImportError {
    /// The step that failed, as an infinitive phrase.
    #[must_use]
    pub fn action(&self) -> String {
        match self {
            ImportError::Read(_) => "import text".to_string(),
            ImportError::CreateExercise(name, _) => format!("create exercise \"{name}\""),
            ImportError::CreateRoutine(name, _) => format!("create routine \"{name}\""),
            ImportError::ReplaceRoutine(name, _) => format!("replace routine \"{name}\""),
            ImportError::UnknownRoutine(_) | ImportError::ReplaceSchedule(_) => {
                "replace schedule".to_string()
            }
        }
    }
}

impl Recoverable for ImportError {
    fn recoverable(&self) -> bool {
        match self {
            ImportError::Read(err) => err.recoverable(),
            ImportError::CreateExercise(_, err) | ImportError::CreateRoutine(_, err) => {
                err.recoverable()
            }
            ImportError::ReplaceRoutine(_, err) | ImportError::ReplaceSchedule(err) => {
                err.recoverable()
            }
            ImportError::UnknownRoutine(_) => false,
        }
    }
}

impl Unreachable for ImportError {
    fn unreachable(&self) -> bool {
        match self {
            ImportError::Read(err) => err.unreachable(),
            ImportError::CreateExercise(_, err) | ImportError::CreateRoutine(_, err) => {
                err.unreachable()
            }
            ImportError::ReplaceRoutine(_, err) | ImportError::ReplaceSchedule(err) => {
                err.unreachable()
            }
            ImportError::UnknownRoutine(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use pretty_assertions::assert_eq;
    use proptest::prelude::*;
    use rstest::rstest;

    use crate::{
        ExerciseMuscle, ExerciseProperties, ExerciseRepository, MuscleID, RoutineRepository,
        ScheduleRepository, Service, Stimulus,
        tests::{Call, FakeRepository},
    };

    use super::*;

    #[test]
    fn test_routine_to_text() {
        assert_eq!(
            routine(1, "A", "B", example_sections()).to_text(&exercises()),
            "Routine \u{2014} A\n\nB\n\n[A] 2 sets\n  A1 \u{2014} A \u{2014} 10 \u{00d7} 30 kg @ 10 (2 s/rep)\n  Rest \u{2014} 60 s\n\n[B] 2 sets\n  B1 \u{2014} Exercise#00000000-0000-0000-0000-000000000002 \u{2014} 10\n  Rest \u{2014} 30 s"
        );
    }

    #[test]
    fn test_routine_to_text_without_notes() {
        assert_eq!(
            routine(0, "N", "", vec![section(1, vec![activity(1, 5)])]).to_text(&exercises()),
            "Routine \u{2014} N\n\n[A] 1 set\n  A1 \u{2014} A \u{2014} 5"
        );
    }

    #[rstest]
    #[case::single_phase_after_reps(8, &[2], 0.0, "8 (2 s/rep)")]
    #[case::several_phases_after_reps(8, &[3, 1, 1, 0], 80.0, "8 \u{00d7} 80 kg (3\u{00b7}1\u{00b7}1\u{00b7}0 s/rep)")]
    #[case::single_phase_after_weight(0, &[60], 40.0, "40 kg (60 s)")]
    #[case::hold(0, &[30], 0.0, "30 s")]
    #[case::several_phases_without_reps(0, &[5, 10], 0.0, "5\u{00b7}10")]
    fn test_routine_to_text_tempo(
        #[case] reps: u32,
        #[case] phases: &[u32],
        #[case] weight: f32,
        #[case] expected: &str,
    ) {
        let routine = routine(
            0,
            "N",
            "",
            vec![section(
                1,
                vec![RoutinePart::RoutineActivity {
                    exercise_id: 1.into(),
                    reps: Reps::new(reps).unwrap(),
                    tempo: Tempo::new(phases).unwrap(),
                    weight: Weight::new(weight).unwrap(),
                    rpe: RPE::ZERO,
                    automatic: false,
                    countdown: false,
                    sides: Sides::Combined,
                }],
            )],
        );

        assert_eq!(
            routine.to_text(&exercises()).lines().last(),
            Some(format!("  A1 \u{2014} A \u{2014} {expected}").as_str())
        );
    }

    #[test]
    fn test_routine_to_text_includes_rpe() {
        let routine = routine(
            0,
            "N",
            "",
            vec![section(
                1,
                vec![RoutinePart::RoutineActivity {
                    exercise_id: 1.into(),
                    reps: Reps::default(),
                    tempo: Tempo::new(&[60]).unwrap(),
                    weight: Weight::default(),
                    rpe: RPE::EIGHT,
                    automatic: false,
                    countdown: false,
                    sides: Sides::Combined,
                }],
            )],
        );

        assert_eq!(
            routine.to_text(&exercises()).lines().last(),
            Some("  A1 \u{2014} A \u{2014} 60 s @ 8")
        );
    }

    #[test]
    fn test_routine_to_text_time_only_and_rest_without_time() {
        let routine = routine(
            0,
            "T",
            "",
            vec![section(
                2,
                vec![
                    RoutinePart::RoutineActivity {
                        exercise_id: 1.into(),
                        reps: Reps::default(),
                        tempo: Tempo::new(&[30]).unwrap(),
                        weight: Weight::default(),
                        rpe: RPE::ZERO,
                        automatic: false,
                        countdown: false,
                        sides: Sides::Combined,
                    },
                    rest(&[], true),
                ],
            )],
        );

        assert_eq!(
            routine.to_text(&exercises()),
            "Routine \u{2014} T\n\n[A] 2 sets\n  A1 \u{2014} A \u{2014} 30 s\n  Rest"
        );
    }

    #[test]
    fn test_routine_to_text_nested() {
        let routine = routine(
            0,
            "N",
            "",
            vec![section(
                2,
                vec![
                    activity(1, 5),
                    section(3, vec![activity(1, 8), rest(&[30], true)]),
                    rest(&[90], true),
                ],
            )],
        );

        assert_eq!(
            routine.to_text(&exercises()),
            "Routine \u{2014} N\n\n[A] 2 sets\n  A1 \u{2014} A \u{2014} 5\n  [A2] 3 sets\n    A2.1 \u{2014} A \u{2014} 8\n    Rest \u{2014} 30 s\n\n  Rest \u{2014} 90 s"
        );
    }

    #[test]
    fn test_routine_to_text_without_values() {
        assert_eq!(
            routine(0, "X", "", vec![section(1, vec![activity(1, 0)])]).to_text(&exercises()),
            "Routine \u{2014} X\n\n[A] 1 set\n  A1 \u{2014} A"
        );
    }

    #[rstest]
    #[case::per_side_without_values(0, &[], Sides::PerSide, false, false, "A \u{2014} per side")]
    #[case::per_side_with_values(
        10,
        &[],
        Sides::PerSide,
        false,
        false,
        "A \u{2014} 10 \u{2014} per side"
    )]
    #[case::auto_start(10, &[], Sides::Combined, true, false, "A \u{2014} 10 \u{2014} auto-start")]
    #[case::per_side_and_auto_start(
        0,
        &[],
        Sides::PerSide,
        true,
        false,
        "A \u{2014} per side \u{2014} auto-start"
    )]
    #[case::auto_start_without_countdown(
        10,
        &[3],
        Sides::Combined,
        true,
        false,
        "A \u{2014} 10 (3 s/rep) \u{2014} auto-start"
    )]
    #[case::countdown(
        10,
        &[3],
        Sides::Combined,
        false,
        true,
        "A \u{2014} 10 (3 s/rep) \u{2014} countdown"
    )]
    #[case::per_side_countdown_and_auto_start(
        10,
        &[3],
        Sides::PerSide,
        true,
        true,
        "A \u{2014} 10 (3 s/rep) \u{2014} per side \u{2014} countdown \u{2014} auto-start"
    )]
    #[case::hold_with_the_flag(
        0,
        &[30],
        Sides::Combined,
        false,
        true,
        "A \u{2014} 30 s \u{2014} countdown"
    )]
    #[case::hold_without_the_flag(0, &[30], Sides::Combined, false, false, "A \u{2014} 30 s")]
    fn test_routine_to_text_suffixes_of_activity(
        #[case] reps: u32,
        #[case] phases: &[u32],
        #[case] sides: Sides,
        #[case] automatic: bool,
        #[case] countdown: bool,
        #[case] expected: &str,
    ) {
        let routine = routine(
            0,
            "N",
            "",
            vec![section(
                1,
                vec![RoutinePart::RoutineActivity {
                    exercise_id: 1.into(),
                    reps: Reps::new(reps).unwrap(),
                    tempo: Tempo::new(phases).unwrap(),
                    weight: Weight::default(),
                    rpe: RPE::ZERO,
                    automatic,
                    countdown,
                    sides,
                }],
            )],
        );

        assert_eq!(
            routine.to_text(&exercises()).lines().last(),
            Some(format!("  A1 \u{2014} {expected}").as_str())
        );
    }

    #[rstest]
    #[case::automatic_with_time(&[60], true, "Rest \u{2014} 60 s")]
    #[case::manual_with_time(&[60], false, "Rest \u{2014} 60 s \u{2014} manual")]
    #[case::manual_without_time(&[], false, "Rest \u{2014} manual")]
    fn test_routine_to_text_suffix_of_rest(
        #[case] phases: &[u32],
        #[case] automatic: bool,
        #[case] expected: &str,
    ) {
        let routine = routine(0, "N", "", vec![section(1, vec![rest(phases, automatic)])]);

        assert_eq!(
            routine.to_text(&exercises()).lines().last(),
            Some(format!("  {expected}").as_str())
        );
    }

    #[test]
    fn test_routine_to_text_many_sections() {
        let routine = routine(0, "N", "", (0..27).map(|_| section(1, vec![])).collect());

        let text = routine.to_text(&[]);

        assert!(text.contains("\n[Z] 1 set"), "{text}");
        assert!(text.contains("\n[#27] 1 set"), "{text}");
    }

    #[test]
    fn test_routine_to_text_numbers_sibling_sections() {
        let routine = routine(
            0,
            "N",
            "",
            vec![section(
                1,
                vec![
                    section(1, vec![activity(1, 1)]),
                    section(1, vec![activity(1, 2)]),
                ],
            )],
        );

        let text = routine.to_text(&exercises());

        assert!(text.contains("[A1] 1 set"), "{text}");
        assert!(text.contains("[A2] 1 set"), "{text}");
    }

    #[test]
    fn test_routine_to_text_does_not_number_rests() {
        let routine = routine(
            0,
            "N",
            "",
            vec![section(
                1,
                vec![activity(1, 1), rest(&[60], true), activity(1, 2)],
            )],
        );

        let text = routine.to_text(&exercises());

        assert!(text.contains("A1 \u{2014} A \u{2014} 1"), "{text}");
        assert!(text.contains("A2 \u{2014} A \u{2014} 2"), "{text}");
    }

    #[test]
    fn test_schedule_to_text() {
        let schedule = schedule(
            vec![(1, "Upper/Lower", vec![2, 3])],
            vec![
                (Weekday::Monday, vec![ScheduleSlot::Routine(1.into())]),
                (Weekday::Wednesday, vec![ScheduleSlot::Rotation(1.into())]),
                (
                    Weekday::Friday,
                    vec![
                        ScheduleSlot::Routine(4.into()),
                        ScheduleSlot::Routine(5.into()),
                    ],
                ),
            ],
        );
        let routines = vec![
            routine(1, "Push Day", "", vec![]),
            routine(2, "Upper Day", "", vec![]),
            routine(3, "Lower Day", "", vec![]),
            routine(4, "Monday", "", vec![]),
            routine(5, "Mobility", "", vec![section(1, vec![activity(1, 5)])]),
        ];

        assert_eq!(
            schedule.to_text(&routines, &exercises()),
            "Schedule\n\n\
             Monday\n- Push Day\nWednesday\n- Rotation \u{2014} Upper/Lower\nFriday\n- Monday\n- Mobility\n\n\
             Rotation \u{2014} Upper/Lower\n- Upper Day\n- Lower Day\n\n\
             Routine \u{2014} Push Day\n\n\
             Routine \u{2014} Monday\n\n\
             Routine \u{2014} Mobility\n\n[A] 1 set\n  A1 \u{2014} A \u{2014} 5\n\n\
             Routine \u{2014} Upper Day\n\n\
             Routine \u{2014} Lower Day"
        );
    }

    #[test]
    fn test_schedule_to_text_empty() {
        assert_eq!(Schedule::default().to_text(&[], &[]), "Schedule");
    }

    #[test]
    fn test_schedule_to_text_rotations_ordered_by_name() {
        let schedule = schedule(vec![(1, "B", vec![]), (2, "A", vec![])], vec![]);

        assert_eq!(
            schedule.to_text(&[], &[]),
            "Schedule\n\nRotation \u{2014} A\n\nRotation \u{2014} B"
        );
    }

    #[test]
    fn test_schedule_to_text_slot_sharing_a_name_with_a_rotation() {
        let schedule = schedule(
            vec![(1, "A", vec![1])],
            vec![(
                Weekday::Monday,
                vec![
                    ScheduleSlot::Routine(1.into()),
                    ScheduleSlot::Rotation(1.into()),
                ],
            )],
        );

        assert_eq!(
            schedule.to_text(&[routine(1, "A", "", vec![])], &[]),
            "Schedule\n\nMonday\n- A\n- Rotation \u{2014} A\n\nRotation \u{2014} A\n- A\n\nRoutine \u{2014} A"
        );
    }

    #[rstest]
    #[case::rotation_em_dash("Rotation \u{2014} X")]
    #[case::rotation_hyphen("Rotation - X")]
    #[case::rotation_en_dash("Rotation \u{2013} X")]
    #[case::routine_em_dash("Routine \u{2014} X")]
    #[case::routine_hyphen("Routine - X")]
    #[case::routine_en_dash("Routine \u{2013} X")]
    fn test_schedule_to_text_prefixes_a_name_starting_with_a_kind(#[case] name: &str) {
        let schedule = schedule(
            vec![(1, "R", vec![1])],
            vec![(Weekday::Monday, vec![ScheduleSlot::Routine(1.into())])],
        );

        let text = schedule.to_text(&[routine(1, name, "", vec![])], &[]);

        assert_eq!(
            text.lines()
                .filter(|line| line.starts_with("- R"))
                .collect::<Vec<_>>(),
            vec![format!("- Routine \u{2014} {name}"); 2]
        );
    }

    #[rstest]
    #[case::without_separator("Routine X")]
    #[case::without_spaces("Rotation-X")]
    fn test_schedule_to_text_does_not_prefix_other_names(#[case] name: &str) {
        let schedule = schedule(
            vec![],
            vec![(Weekday::Monday, vec![ScheduleSlot::Routine(1.into())])],
        );

        let text = schedule.to_text(&[routine(1, name, "", vec![])], &[]);

        assert!(text.contains(&format!("\n- {name}\n")), "{text}");
    }

    #[test]
    fn test_schedule_to_text_slot_of_unknown_routine() {
        let schedule = schedule(
            vec![(1, "R", vec![2])],
            vec![(Weekday::Monday, vec![ScheduleSlot::Routine(1.into())])],
        );

        assert_eq!(
            schedule.to_text(&[], &[]),
            "Schedule\n\nMonday\n- Routine#00000000-0000-0000-0000-000000000001\n\nRotation \u{2014} R\n- Routine#00000000-0000-0000-0000-000000000002"
        );
    }

    #[test]
    fn test_parse_routine() {
        let parsed = parse_text(
            "Routine — Push Day\n\
             \n\
             Heavy day.\n\
             \n\
             [A] 3 sets\n  \
               A1 — A — 8 × 60 kg @ 8 (3·1·1·0 s/rep)\n  \
               A2 — B — 30 s — per side\n  \
               Rest — 90 s\n  \
               [A3] 2 sets\n    \
                 A3.1 — A — 10 @ 9 — auto-start\n\
             \n  \
               Rest — manual\n\
             \n\
             [B] 1 set\n  \
               B1 — B — 40 kg (60 s)",
        )
        .unwrap();

        assert_eq!(parsed.schedule, None);
        let [routine] = &parsed.routines[..] else {
            panic!("{parsed:?}");
        };
        assert_eq!(routine.line, 1);
        assert_eq!(routine.name, Name::new("Push Day").unwrap());
        assert_eq!(routine.notes, "Heavy day.");
        assert_eq!(routine.sections(&exercise_id), push_day_sections());
    }

    #[test]
    fn test_parse_routine_without_indentation() {
        assert_eq!(
            sections(
                "Routine — Push Day\n\
                 \n\
                 [A] 3 sets\n\
                 A1 — A — 8 × 60 kg @ 8 (3·1·1·0 s/rep)\n\
                 A2 — B — 30 s — per side\n\
                 Rest — 90 s\n\
                 [A3] 2 sets\n\
                 A3.1 — A — 10 @ 9 — auto-start\n\
                 \n\
                 Rest — manual\n\
                 \n\
                 [B] 1 set\n\
                 B1 — B — 40 kg (60 s)"
            ),
            push_day_sections()
        );
    }

    #[test]
    fn test_parse_ignores_text_around_blocks() {
        let parsed = parse_text(
            "Here is the updated routine:\n\
             \n\
             ```\n\
             Routine — X\n\
             \n\
             [A] 1 set\n  \
               A1 — A — 5\n\
             ```\n\
             \n\
             I lowered the reps — let me know.",
        )
        .unwrap();

        assert_eq!(parsed.routines.len(), 1);
        assert_eq!(parsed.routines[0].notes, "");
        assert_eq!(
            parsed.routines[0].sections(&exercise_id),
            vec![section(1, vec![activity(1, 5)])]
        );
    }

    #[rstest]
    #[case::indented(
        "[A] 1 set\n  [A1] 2 sets\n    A1.1 — A — 5\n  Rest — 60 s",
        vec![section(1, vec![section(2, vec![activity(1, 5)]), rest(&[60], true)])]
    )]
    #[case::unindented(
        "[A] 1 set\n[A1] 2 sets\nA1.1 — A — 5\nRest — 60 s",
        vec![section(1, vec![section(2, vec![activity(1, 5), rest(&[60], true)])])]
    )]
    #[case::unindented_after_blank_line(
        "[A] 1 set\n[A1] 2 sets\nA1.1 — A — 5\n\nRest — 60 s",
        vec![section(1, vec![section(2, vec![activity(1, 5)]), rest(&[60], true)])]
    )]
    fn test_parse_rest_after_nested_section(
        #[case] body: &str,
        #[case] expected: Vec<RoutinePart>,
    ) {
        assert_eq!(sections(&format!("Routine — X\n\n{body}")), expected);
    }

    #[test]
    fn test_parse_indented_and_unindented_routine_in_one_text() {
        let parsed = parse_text(
            "Routine — I\n\
             \n\
             [A] 1 set\n  \
               [A1] 1 set\n    \
                 A1.1 — A\n  \
               Rest — 60 s\n\
             \n\
             Routine — U\n\
             \n\
             [A] 1 set\n\
             [A1] 1 set\n\
             A1.1 — A\n\
             \n\
             Rest — 60 s",
        )
        .unwrap();

        let expected = vec![section(
            1,
            vec![section(1, vec![activity(1, 0)]), rest(&[60], true)],
        )];
        assert_eq!(parsed.routines[0].sections(&exercise_id), expected);
        assert_eq!(parsed.routines[1].sections(&exercise_id), expected);
    }

    #[test]
    fn test_parse_ignores_numbering() {
        assert_eq!(
            sections("Routine — X\n\n[A] 1 set\n  A3 — A — 5\n  A1 — B — 6\n[C] 2 sets\n  C7 — A"),
            vec![
                section(1, vec![activity(1, 5), activity(2, 6)]),
                section(2, vec![activity(1, 0)])
            ]
        );
    }

    #[rstest]
    #[case::indented(
        "[Warm-up] 1 set\n  WU1 — A — 5\n  [Mobility] 2 sets\n    MO1 — B\n  WU2 — A — 6\n[A] 1 set\n  A1 — B",
        vec![
            section(1, vec![activity(1, 5), section(2, vec![activity(2, 0)]), activity(1, 6)]),
            section(1, vec![activity(2, 0)]),
        ]
    )]
    #[case::unindented(
        "[Warm-up] 1 set\nWU1 — A — 5\n[Mobility] 2 sets\nMO1 — B",
        vec![section(1, vec![activity(1, 5)]), section(2, vec![activity(2, 0)])]
    )]
    #[case::numbered_from_27th_section(
        "[#27] 1 set\n#27.1 — A — 5\n[#27.1] 2 sets\n#27.1.1 — B",
        vec![section(1, vec![activity(1, 5), section(2, vec![activity(2, 0)])])]
    )]
    fn test_parse_labels(#[case] body: &str, #[case] expected: Vec<RoutinePart>) {
        assert_eq!(sections(&format!("Routine — X\n\n{body}")), expected);
    }

    #[test]
    fn test_parse_ignores_stray_lines_between_sections() {
        assert_eq!(
            sections(
                "Routine — X\n\n[A] 1 set\n  A1 — A — 5\n\n### Upper body\nNow some pulling:\n\n[B] 1 set\n  B1 — B — 6"
            ),
            vec![
                section(1, vec![activity(1, 5)]),
                section(1, vec![activity(2, 6)])
            ]
        );
    }

    #[rstest]
    #[case::round("[A] 1 round", 1)]
    #[case::rounds("[A] 2 rounds", 2)]
    #[case::set("[A] 1 set", 1)]
    #[case::sets("[A] 3 sets", 3)]
    fn test_parse_section(#[case] line: &str, #[case] rounds: u32) {
        assert_eq!(
            sections(&format!("Routine — X\n\n{line}")),
            vec![section(rounds, vec![])]
        );
    }

    #[test]
    fn test_parse_ascii_alternatives() {
        assert_eq!(
            sections(
                "Routine - X\n\
                 \n\
                 [A] 2 sets\n  \
                   A1 - A - 8 x 60 kg @ 8 (3.1.1.0 s/rep)\n  \
                   A2 – B – 8 X 60 kg (3-1-1-0)\n  \
                   A3 - A - 8*60kg (2-4 s/rep)\n  \
                   Rest – 60 s - manual"
            ),
            vec![section(
                2,
                vec![
                    full_activity(1, 8, 60.0, 8.0, &[3, 1, 1, 0], false, Sides::Combined),
                    full_activity(2, 8, 60.0, 0.0, &[3, 1, 1, 0], false, Sides::Combined),
                    full_activity(1, 8, 60.0, 0.0, &[2, 4], false, Sides::Combined),
                    rest(&[60], false),
                ]
            )]
        );
    }

    #[rstest]
    #[case::several_phases_after_reps_without_unit("8 (3·1·1·0)", 8, 0.0, 0.0, &[3, 1, 1, 0])]
    #[case::single_phase_after_reps("8 (2 s/rep)", 8, 0.0, 0.0, &[2])]
    #[case::several_phases_after_reps("8 (5·10 s/rep)", 8, 0.0, 0.0, &[5, 10])]
    #[case::two_phases_joined_by_hyphen_after_reps("8 (2-4 s/rep)", 8, 0.0, 0.0, &[2, 4])]
    #[case::time_after_weight("40 kg (60 s)", 0, 40.0, 0.0, &[60])]
    #[case::decimal_weight("22,5 kg", 0, 22.5, 0.0, &[])]
    #[case::hold("30 s", 0, 0.0, 0.0, &[30])]
    #[case::hold_with_rpe("60 s @ 8", 0, 0.0, 8.0, &[60])]
    #[case::rpe("@ 8.5", 0, 0.0, 8.5, &[])]
    #[case::tempo("5·10", 0, 0.0, 0.0, &[5, 10])]
    #[case::tempo_with_dots("3.1.1.0", 0, 0.0, 0.0, &[3, 1, 1, 0])]
    #[case::tempo_with_hyphens("3-1-1-0", 0, 0.0, 0.0, &[3, 1, 1, 0])]
    fn test_parse_values(
        #[case] values: &str,
        #[case] reps: u32,
        #[case] weight: f32,
        #[case] rpe: f32,
        #[case] phases: &[u32],
    ) {
        assert_eq!(
            sections(&format!("Routine — X\n\n[A] 1 set\n  A1 — A — {values}")),
            vec![section(
                1,
                vec![full_activity(
                    1,
                    reps,
                    weight,
                    rpe,
                    phases,
                    false,
                    Sides::Combined
                )]
            )]
        );
    }

    #[rstest]
    #[case::name_with_hyphen("90-90 Hip Stretch — 5", "90-90 Hip Stretch")]
    #[case::name_with_separator("Curl - Hammer — 5", "Curl - Hammer")]
    #[case::name_ending_in_number_of_sets("Bench Press — 3 sets", "Bench Press — 3 sets")]
    #[case::name_ending_in_values("Dead Hang - 30 s", "Dead Hang - 30 s")]
    #[case::name_like_reps("8", "8")]
    #[case::name_like_time("21s", "21s")]
    #[case::name_like_reps_and_weight("5x5", "5x5")]
    fn test_parse_exercise_name(#[case] line: &str, #[case] expected: &str) {
        let parsed = parse_text(&format!("Routine — X\n\n[A] 1 set\n  A1 — {line}")).unwrap();

        assert_eq!(parsed.exercise_names(), vec![&Name::new(expected).unwrap()]);
    }

    #[rstest]
    #[case::auto_start("10 (3 s/rep) — auto-start", 10, &[3], true, false)]
    #[case::countdown("10 (3 s/rep) — countdown", 10, &[3], false, true)]
    #[case::countdown_and_auto_start("10 (3 s/rep) — countdown — auto-start", 10, &[3], true, true)]
    #[case::auto_start_and_countdown("10 (3 s/rep) — auto-start — countdown", 10, &[3], true, true)]
    #[case::countdown_of_a_hold("30 s — countdown", 0, &[30], false, true)]
    #[case::countdown_without_a_tempo("10 — countdown", 10, &[], false, false)]
    fn test_parse_suffixes_of_activity(
        #[case] values: &str,
        #[case] reps: u32,
        #[case] phases: &[u32],
        #[case] automatic: bool,
        #[case] countdown: bool,
    ) {
        assert_eq!(
            sections(&format!("Routine — X\n\n[A] 1 set\n  A1 — A — {values}")),
            vec![section(
                1,
                vec![RoutinePart::RoutineActivity {
                    exercise_id: 1.into(),
                    reps: Reps::new(reps).unwrap(),
                    tempo: Tempo::new(phases).unwrap(),
                    weight: Weight::default(),
                    rpe: RPE::ZERO,
                    automatic,
                    countdown,
                    sides: Sides::Combined,
                }]
            )]
        );
    }

    #[rstest]
    #[case::without_time("Rest", rest(&[], true))]
    #[case::manual_without_time("Rest — manual", rest(&[], false))]
    #[case::automatic("Rest — 60 s — auto-start", rest(&[60], true))]
    fn test_parse_rest(#[case] line: &str, #[case] expected: RoutinePart) {
        assert_eq!(
            sections(&format!("Routine — X\n\n[A] 1 set\n  {line}")),
            vec![section(1, vec![expected])]
        );
    }

    #[rstest]
    #[case::bare_single_time_after_reps(
        "A1 — A — 8 (30 s)",
        "time following reps must be followed by \"s/rep\""
    )]
    #[case::decimal_time_after_reps("A1 — A — 8 (1.5 s/rep)", "time must be an integer")]
    #[case::decimal_time_without_reps("A1 — A — 2.5 s", "time must be an integer")]
    #[case::several_phases_with_bare_unit(
        "A1 — A — 30-45 s",
        "tempo of several phases must not be followed by \"s\""
    )]
    #[case::range_without_reps(
        "A1 — A — 8-12",
        "two phases joined by \"-\" must be joined by \"·\""
    )]
    #[case::weight_without_unit("A1 — A — 8 × 60", "weight must be followed by \"kg\"")]
    #[case::reps_out_of_range("A1 — A — 1000", "reps must be in the range 0 to 999")]
    #[case::rpe_out_of_range("A1 — A — @ 11", "RPE must be in the range 0.0 to 10.0")]
    #[case::too_many_phases(
        "A1 — A — 8 (2·0·0·0·1 s/rep)",
        "tempo must not have more than 4 phases"
    )]
    #[case::parenthesized_time_without_reps(
        "A1 — A — (60 s)",
        "time in parentheses must follow reps or a weight"
    )]
    #[case::label_without_separator("A1 Squat — 5", "label must be followed by \"—\"")]
    #[case::label_with_period_without_separator("A1. Squat — 5", "label must be followed by \"—\"")]
    #[case::only_suffixes("A1 — per side", "exercise name is missing")]
    #[case::name_too_long(
        "A1 — AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "name must be 64 characters or fewer (65 > 64)"
    )]
    #[case::rest_with_reps("Rest — 8", "rest must have no values other than a time")]
    #[case::rest_with_rpe("Rest — 60 s @ 8", "rest must have no values other than a time")]
    #[case::rest_with_tempo("Rest — 5·10", "rest must have no values other than a time")]
    #[case::rest_with_text("Rest — as needed", "rest must have no values other than a time")]
    #[case::rest_per_side("Rest — 60 s — per side", "rest must not be performed per side")]
    #[case::rest_with_countdown(
        "Rest — 60 s — countdown",
        "rest must have no values other than a time"
    )]
    #[case::rest_with_label("A2 — Rest — 60 s", "rest must not have a label")]
    #[case::rest_with_label_without_time("A2 — rest", "rest must not have a label")]
    #[case::part_outside_section("A — A — 5", "activity outside of a section")]
    #[case::multi_word_exercise_without_label(
        "Bench Press — 5 × 60 kg",
        "activity must start with a label"
    )]
    #[case::exercise_without_label("Plank — 30 s", "activity must start with a label")]
    #[case::exercise_without_label_per_side(
        "Plank — 30 s — per side",
        "activity must start with a label"
    )]
    #[case::part_skipping_level(
        "A1.1 — A — 5",
        "part is more than one level below the enclosing section"
    )]
    #[case::rounds_out_of_range("[A1] 0 sets", "rounds must be in the range 1 to 999")]
    #[case::section_without_sets("[A1] 2 setz", "section must be followed by a number of sets")]
    #[case::bulleted_activity("- A1 — A — 8", "unexpected Markdown formatting")]
    #[case::bold_activity("**A1** — A — 8", "unexpected Markdown formatting")]
    fn test_parse_error_in_routine(#[case] line: &str, #[case] message: &str) {
        assert_eq!(
            errors(&format!("Routine — X\n\n[A] 1 set\n  {line}")),
            vec![format!("line 4: {message}")]
        );
    }

    #[test]
    fn test_parse_notes() {
        let parsed = parse_text(
            "Routine — X\n\
             \n\
             Rest — as needed\n\
             \n\
             Monday is heavy.\n\
             \n\
             [A] 1 set",
        )
        .unwrap();

        assert_eq!(
            parsed.routines[0].notes,
            "Rest — as needed\n\nMonday is heavy."
        );
    }

    #[test]
    fn test_parse_notes_with_list_item_like_a_part() {
        let parsed = parse_text("Routine — X\n\n- Tip - keep the elbows in\n\n[A] 1 set").unwrap();

        assert_eq!(parsed.routines[0].notes, "- Tip - keep the elbows in");
    }

    #[rstest]
    #[case::routine_without_name("Routine — \n\n[A] 1 set", "line 1: name must not be empty")]
    #[case::decorated_part_in_notes(
        "Routine — X\n- A1 — A — 5",
        "line 2: unexpected Markdown formatting"
    )]
    #[case::part_in_notes(
        "Routine — X\n\nA1 — A — 5\n\n[A] 1 set",
        "line 3: activity outside of a section"
    )]
    #[case::decorated_routine(
        "### Routine — Push Day\n\n[A] 1 set",
        "line 1: unexpected Markdown formatting"
    )]
    #[case::decorated_schedule(
        "## Schedule\n\nMonday\n- A",
        "line 1: unexpected Markdown formatting"
    )]
    #[case::duplicate_routine("Routine — A\n\nRoutine — A", "line 3: routine \"A\" appears twice")]
    #[case::duplicate_schedule("Schedule\n\nSchedule", "line 3: schedule appears twice")]
    #[case::without_block("Nothing to import here.", "text contains no routine and no schedule")]
    fn test_parse_error(#[case] text: &str, #[case] expected: &str) {
        assert_eq!(errors(text), vec![expected.to_string()]);
    }

    #[test]
    fn test_parse_schedule() {
        let parsed = parse_text(
            "Schedule\n\
             \n\
             Monday\n\
             - Push Day\n\
             Wednesday\n\
             - Rotation — Upper/Lower\n\
             Friday\n\
             - Monday\n\
             - Mobility\n\
             \n\
             Rotation — Upper/Lower\n\
             - Upper Day\n\
             - Lower Day",
        )
        .unwrap();

        assert_eq!(
            parsed.schedule,
            Some(ParsedSchedule {
                line: 1,
                entries: BTreeMap::from([
                    (Weekday::Monday, vec![routine_slot(4, "Push Day")]),
                    (
                        Weekday::Wednesday,
                        vec![ParsedSlot::Rotation(reference(6, "Upper/Lower"))]
                    ),
                    (
                        Weekday::Friday,
                        vec![routine_slot(8, "Monday"), routine_slot(9, "Mobility")]
                    ),
                ]),
                rotations: vec![ParsedRotation {
                    line: 11,
                    name: Name::new("Upper/Lower").unwrap(),
                    routines: vec![reference(12, "Upper Day"), reference(13, "Lower Day")],
                }],
            })
        );
    }

    #[rstest]
    #[case::blank_lines_and_asterisks(
        "Schedule\n\nMonday\n\n* A\n\nTuesday\n  - B\n\nLet me know if this works.",
        vec![(Weekday::Monday, vec!["A"]), (Weekday::Tuesday, vec!["B"])]
    )]
    #[case::routine_prefix(
        "Schedule\n\nMonday\n- Routine — Rotation — X\n- Routine - Y\n- Schedule\n- Rest",
        vec![(Weekday::Monday, vec!["Rotation — X", "Y", "Schedule", "Rest"])]
    )]
    #[case::fenced(
        "```\nSchedule\n\nMonday\n- A\n```\nDone.",
        vec![(Weekday::Monday, vec!["A"])]
    )]
    #[case::empty_week("Schedule", vec![])]
    fn test_parse_schedule_slots(#[case] text: &str, #[case] expected: Vec<(Weekday, Vec<&str>)>) {
        let parsed = parse_text(text).unwrap();

        assert_eq!(
            parsed
                .schedule
                .unwrap()
                .entries
                .into_iter()
                .map(|(weekday, slots)| (
                    weekday,
                    slots
                        .into_iter()
                        .map(|slot| match slot {
                            ParsedSlot::Routine(reference) => reference.name.to_string(),
                            ParsedSlot::Rotation(reference) => format!("@{}", reference.name),
                        })
                        .collect::<Vec<_>>()
                ))
                .collect::<Vec<_>>(),
            expected
                .into_iter()
                .map(|(weekday, slots)| (
                    weekday,
                    slots
                        .into_iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                ))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_parse_rotation_with_routine_prefix() {
        let parsed = parse_text("Schedule\n\nRotation — R\n- Routine – Rotation - Z").unwrap();

        assert_eq!(
            parsed.schedule.unwrap().rotations[0].routines,
            vec![reference(4, "Rotation - Z")]
        );
    }

    #[test]
    fn test_parse_routine_block_directly_after_last_slot() {
        let parsed =
            parse_text("Schedule\n\nMonday\n- A\nRoutine — A\n\nMonday is heavy.\n\n[A] 1 set")
                .unwrap();

        assert_eq!(
            parsed.schedule.unwrap().entries,
            BTreeMap::from([(Weekday::Monday, vec![routine_slot(4, "A")])])
        );
        assert_eq!(parsed.routines[0].name, Name::new("A").unwrap());
        assert_eq!(parsed.routines[0].notes, "Monday is heavy.");
    }

    #[rstest]
    #[case::unmarked_line_below_slot(
        "Schedule\n\nMonday\n- A\nTuesday — B",
        "line 5: line must be a weekday, a rotation or a list item starting with \"- \""
    )]
    #[case::unmarked_line_below_heading(
        "Schedule\n\nMonday\nA",
        "line 4: line must be a weekday, a rotation or a list item starting with \"- \""
    )]
    #[case::weekday_after_blank_line(
        "Schedule\n\nMonday\n- A\n\nTuesday — B",
        "line 6: weekday must stand on a line of its own, followed by its routines as list items"
    )]
    #[case::emphasized_weekday_after_blank_line(
        "Schedule\n\nMonday\n- A\n\n**Tuesday**",
        "line 6: weekday must stand on a line of its own, followed by its routines as list items"
    )]
    #[case::list_item_before_heading(
        "Schedule\n\n- A",
        "line 3: list item before the first weekday or rotation"
    )]
    #[case::duplicate_weekday(
        "Schedule\n\nMonday\n- A\nMonday\n- B",
        "line 5: Monday appears twice"
    )]
    #[case::duplicate_rotation(
        "Schedule\n\nRotation — R\n\nRotation — R",
        "line 5: rotation \"R\" appears twice"
    )]
    #[case::duplicate_routine_in_rotation(
        "Schedule\n\nRotation — R\n- A\n- A",
        "line 5: routine \"A\" appears twice in rotation \"R\""
    )]
    #[case::unknown_rotation(
        "Schedule\n\nMonday\n- Rotation — R",
        "line 4: unknown rotation \"R\""
    )]
    #[case::rotation_in_rotation(
        "Schedule\n\nRotation — R\n- Rotation — S",
        "line 4: rotation must not contain a rotation"
    )]
    #[case::empty_slot("Schedule\n\nMonday\n- ", "line 4: name must not be empty")]
    #[case::decorated_routine(
        "Schedule\n\nMonday\n- A\n\n**Routine — A**",
        "line 6: unexpected Markdown formatting"
    )]
    fn test_parse_error_in_schedule(#[case] text: &str, #[case] expected: &str) {
        assert_eq!(errors(text), vec![expected.to_string()]);
    }

    #[test]
    fn test_unknown_routines() {
        let parsed =
            parse_text("Schedule\n\nMonday\n- A\n- C\n\nRotation — R\n- B\n- D\n\nRoutine — A")
                .unwrap();

        assert_eq!(
            parsed
                .unknown_routines(&[routine(1, "B", "", vec![])])
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![
                "line 5: unknown routine \"C\"".to_string(),
                "line 9: unknown routine \"D\"".to_string()
            ]
        );
    }

    #[test]
    fn test_unknown_exercises() {
        let parsed = parse_text(
            "Routine — X\n\n[A] 1 set\n  A1 — Dips\n  A2 — dips\n  A3 — Bench Press\n  A4 — Mystery",
        )
        .unwrap();

        assert_eq!(
            parsed.unknown_exercises(&[exercise(1, "Barbell Bench Press")]),
            vec![Name::new("Dips").unwrap(), Name::new("Mystery").unwrap()]
        );
    }

    #[rstest]
    #[case::exact(&["Dip"], "Dip", Some(1))]
    #[case::case_and_hyphen(&["Pull-Up"], "pull up", Some(1))]
    #[case::plural(&["Dip"], "Dips", Some(1))]
    #[case::plural_of_user_exercise(&["Band Pull-Aparts"], "Band Pull Apart", Some(1))]
    #[case::catalog_alias(&["Barbell Bench Press"], "Bench Press", Some(1))]
    #[case::exact_spelling_among_several(&["Dip", "dip"], "dip", Some(2))]
    #[case::several_spellings(&["Dip", "dip"], "DIP", None)]
    #[case::exact_spelling_over_plural(&["Dip", "Dips"], "Dips", Some(2))]
    #[case::several_catalog_matches(&["Barbell Bench Press", "barbell bench press"], "Bench Press", None)]
    #[case::prefix(&["Dip"], "Dip (weighted)", None)]
    #[case::unknown(&["Dip"], "Squat", None)]
    fn test_resolve_exercise(
        #[case] names: &[&str],
        #[case] name: &str,
        #[case] expected: Option<u128>,
    ) {
        let exercises = names
            .iter()
            .zip(1..)
            .map(|(name, id)| exercise(id, name))
            .collect::<Vec<_>>();

        assert_eq!(
            resolve_exercise(&Name::new(name).unwrap(), &exercises),
            expected.map(ExerciseID::from)
        );
    }

    #[rstest]
    #[case::unused("C", None, Ok("C"))]
    #[case::used_by_existing_routine("A", None, Err("entry with this name already exists"))]
    #[case::used_by_new_routine("B", None, Err("entry with this name already exists"))]
    #[case::used_by_other_copy("C", Some("C"), Err("entry with this name already exists"))]
    #[case::invalid("", None, Err("name must not be empty"))]
    fn test_validate_copy_name(
        #[case] input: &str,
        #[case] other_copy: Option<&str>,
        #[case] expected: Result<&str, &str>,
    ) {
        let parsed = parse_text("Routine — A\n\nRoutine — B\n\nRoutine — D").unwrap();
        let copies = other_copy
            .map(|copy| (Name::new("D").unwrap(), Name::new(copy).unwrap()))
            .into_iter()
            .collect();

        assert_eq!(
            validate_copy_name(
                input,
                &Name::new("A").unwrap(),
                &parsed,
                &[routine(1, "A", "", vec![]), routine(2, "D", "", vec![])],
                &copies,
            )
            .map(|name| name.to_string())
            .map_err(|err| err.to_string()),
            expected
                .map(ToString::to_string)
                .map_err(ToString::to_string)
        );
    }

    #[rstest]
    #[case::free("A", &[], None, "A (2)")]
    #[case::used_by_existing_routine("A", &["A (2)"], None, "A (3)")]
    #[case::used_by_new_routine("A", &[], Some("Routine — A (2)"), "A (3)")]
    #[case::too_long(&"a".repeat(64), &[], None, &format!("{} (2)", "a".repeat(60)))]
    #[case::too_long_cut_at_space(&format!("{} bc", "a".repeat(59)), &[], None, &format!("{} (2)", "a".repeat(59)))]
    #[case::too_long_multibyte(&"ä".repeat(32), &[], None, &format!("{} (2)", "ä".repeat(30)))]
    fn test_free_copy_name(
        #[case] name: &str,
        #[case] existing: &[&str],
        #[case] new: Option<&str>,
        #[case] expected: &str,
    ) {
        let name = Name::new(name).unwrap();
        let text = [format!("Routine — {name}")]
            .into_iter()
            .chain(new.map(ToString::to_string))
            .collect::<Vec<_>>()
            .join("\n\n");
        let parsed = parse_text(&text).unwrap();
        let routines = [name.as_ref().as_str()]
            .iter()
            .chain(existing)
            .zip(1..)
            .map(|(name, id)| routine(id, name, "", vec![]))
            .collect::<Vec<_>>();

        assert_eq!(
            free_copy_name(&name, &parsed, &routines, &BTreeMap::new()).to_string(),
            expected
        );
    }

    #[test]
    fn test_free_copy_name_skips_names_of_other_copies() {
        let parsed = parse_text("Routine — A\n\nRoutine — D").unwrap();
        let copies = BTreeMap::from([(Name::new("D").unwrap(), Name::new("A (2)").unwrap())]);

        assert_eq!(
            free_copy_name(
                &Name::new("A").unwrap(),
                &parsed,
                &[routine(1, "A", "", vec![]), routine(2, "D", "", vec![])],
                &copies,
            )
            .to_string(),
            "A (3)"
        );
    }

    #[rstest]
    #[case::existing_exercise(&[], "dip", ExerciseChoice::Existing(1.into()))]
    #[case::catalog_exercise(&[], "Bench Press", catalog_choice("Barbell Bench Press"))]
    #[case::unknown(&[], "Mystery", ExerciseChoice::New)]
    #[case::chosen(&[("Mystery", ExerciseChoice::Existing(2.into()))], "Mystery", ExerciseChoice::Existing(2.into()))]
    #[case::chosen_for_matching_name(&[("Mystery", ExerciseChoice::New)], "Mysteries", ExerciseChoice::New)]
    #[case::new_for_catalog_exercise(&[("Bench Press", ExerciseChoice::New)], "Bench Press", ExerciseChoice::New)]
    #[case::chosen_catalog_exercise(&[("Mystery", catalog_choice("Squat"))], "Mystery", catalog_choice("Squat"))]
    #[case::catalog_exercise_of_existing_name(&[("Mystery", catalog_choice("Dip"))], "Mystery", ExerciseChoice::Existing(1.into()))]
    fn test_import_choices_exercise(
        #[case] chosen: &[(&str, ExerciseChoice)],
        #[case] name: &str,
        #[case] expected: ExerciseChoice,
    ) {
        let choices = ImportChoices {
            exercises: chosen
                .iter()
                .map(|(name, choice)| (Name::new(name).unwrap(), choice.clone()))
                .collect(),
            ..ImportChoices::default()
        };

        assert_eq!(
            choices.exercise(
                &Name::new(name).unwrap(),
                &[exercise(1, "Dip"), exercise(2, "Pull-Up")]
            ),
            expected
        );
    }

    #[test]
    fn test_change_of_routine_names_exercise_created_from_catalog_as_in_catalog() {
        let parsed = parse_text(
            "Routine — R

[A] 1 set
  A1 — Bench Press — 5",
        )
        .unwrap();

        assert_eq!(
            parsed.routines[0].change(
                &[routine(1, "R", "", vec![])],
                &exercises(),
                &ImportChoices::default()
            ),
            RoutineChange::Changed(vec![
                LineChange::Added("[A] 1 set".to_string()),
                LineChange::Added("  A1 — Barbell Bench Press — 5".to_string()),
            ])
        );
    }

    #[test]
    fn test_change_of_routine_names_matching_new_exercises_like_the_first() {
        let parsed = parse_text(
            "Routine — R

[A] 1 set
  A1 — Mystery Lift — 5
  A2 — mystery lifts — 5",
        )
        .unwrap();

        assert_eq!(
            parsed.routines[0].change(
                &[routine(1, "R", "", vec![])],
                &exercises(),
                &ImportChoices::default()
            ),
            RoutineChange::Changed(vec![
                LineChange::Added("[A] 1 set".to_string()),
                LineChange::Added("  A1 — Mystery Lift — 5".to_string()),
                LineChange::Added("  A2 — Mystery Lift — 5".to_string()),
            ])
        );
    }

    #[test]
    fn test_change_of_routine_compares_notes_by_line() {
        let parsed = parse_text("Routine — R\n\nA\nX\nC").unwrap();

        assert_eq!(
            parsed.routines[0].change(
                &[routine(1, "R", "A\nB\nC", vec![])],
                &exercises(),
                &ImportChoices::default()
            ),
            RoutineChange::Changed(vec![
                LineChange::Kept("A".to_string()),
                LineChange::Removed("B".to_string()),
                LineChange::Added("X".to_string()),
                LineChange::Kept("C".to_string()),
            ])
        );
    }

    #[test]
    fn test_change_of_routine_without_existing_routine() {
        let parsed = parse_text("Routine — R\n\n[A] 1 set\n  A1 — A — 5").unwrap();

        assert_eq!(
            parsed.routines[0].change(
                &[routine(1, "S", "", vec![])],
                &exercises(),
                &ImportChoices::default()
            ),
            RoutineChange::New
        );
    }

    #[rstest]
    #[case::same_spelling("Pull-Up", ImportChoices::default())]
    #[case::matching_spelling("pull ups", ImportChoices::default())]
    #[case::chosen_exercise(
        "Mystery",
        ImportChoices {
            exercises: BTreeMap::from([(
                Name::new("Mystery").unwrap(),
                ExerciseChoice::Existing(2.into())
            )]),
            ..ImportChoices::default()
        }
    )]
    fn test_change_of_equal_routine(#[case] exercise_name: &str, #[case] choices: ImportChoices) {
        let parsed = parse_text(&format!(
            "Routine — R\n\nNotes\n\n[A] 1 set\n  A1 — {exercise_name} — 5"
        ))
        .unwrap();

        assert_eq!(
            parsed.routines[0].change(
                &[routine(
                    1,
                    "R",
                    "Notes",
                    vec![section(1, vec![activity(2, 5)])]
                )],
                &[exercise(1, "Dip"), exercise(2, "Pull-Up")],
                &choices
            ),
            RoutineChange::Unchanged
        );
    }

    #[test]
    fn test_change_of_routine_with_trailing_whitespace_in_notes() {
        let parsed =
            parse_text("Routine — R\n\nHeavy day \nFocus\n\n[A] 1 set\n  A1 — Dip — 5").unwrap();

        assert_eq!(
            parsed.routines[0].change(
                &[routine(
                    1,
                    "R",
                    "Heavy day \nFocus",
                    vec![section(1, vec![activity(1, 5)])]
                )],
                &[exercise(1, "Dip")],
                &ImportChoices::default()
            ),
            RoutineChange::Unchanged
        );
    }

    #[test]
    fn test_change_of_changed_routine() {
        let parsed =
            parse_text("Routine — R\n\nnew\n\n[A] 1 set\n  A1 — A — 6\n  A2 — Mystery — 5")
                .unwrap();

        assert_eq!(
            parsed.routines[0].change(
                &[routine(
                    1,
                    "R",
                    "old",
                    vec![section(1, vec![activity(1, 5)])]
                )],
                &exercises(),
                &ImportChoices::default()
            ),
            RoutineChange::Changed(vec![
                LineChange::Removed("old".to_string()),
                LineChange::Added("new".to_string()),
                LineChange::Kept(String::new()),
                LineChange::Kept("[A] 1 set".to_string()),
                LineChange::Removed("  A1 — A — 5".to_string()),
                LineChange::Added("  A1 — A — 6".to_string()),
                LineChange::Added("  A2 — Mystery — 5".to_string()),
            ])
        );
    }

    #[test]
    fn test_change_of_routine_leaves_out_blank_lines() {
        let parsed = parse_text("Routine — R\n\nNotes\n\n[A] 1 set\n  A1 — A — 5").unwrap();

        assert_eq!(
            parsed.routines[0].change(
                &[routine(1, "R", "", vec![section(1, vec![activity(1, 5)])])],
                &exercises(),
                &ImportChoices::default()
            ),
            RoutineChange::Changed(vec![
                LineChange::Added("Notes".to_string()),
                LineChange::Kept("[A] 1 set".to_string()),
                LineChange::Kept("  A1 — A — 5".to_string()),
            ])
        );
    }

    #[test]
    fn test_change_of_routine_with_new_exercise_named_like_an_unknown_id() {
        let parsed = parse_text("Routine — R\n\n[A] 1 set\n  A1 — Mystery — 5").unwrap();

        assert!(matches!(
            parsed.routines[0].change(
                &[routine(1, "R", "", vec![section(1, vec![activity(2, 5)])])],
                &exercises(),
                &ImportChoices::default()
            ),
            RoutineChange::Changed(_)
        ));
    }

    #[test]
    fn test_import_text_keeps_unchanged_routine() {
        let repository = FakeRepository::default()
            .with_exercises(exercises())
            .with_routines(vec![routine(
                1,
                "R",
                "",
                vec![section(1, vec![activity(1, 5)])],
            )]);
        let parsed = parse_text("Routine — R\n\n[A] 1 set\n  A1 — A — 5").unwrap();

        import(&repository, &parsed, &ImportChoices::default()).unwrap();

        assert!(!repository.calls().contains(&Call::ModifyRoutine));
    }

    #[test]
    fn test_import_text_creates_unknown_exercises() {
        let repository =
            FakeRepository::default().with_exercises(vec![exercise(1, "Barbell Bench Press")]);
        let parsed = parse_text(
            "Routine — New\n\n[A] 1 set\n  A1 — Dips — 10\n  A2 — dips — 8\n  A3 — Bench Press — 5\n  A4 — Mystery — 5",
        )
        .unwrap();

        import(&repository, &parsed, &ImportChoices::default()).unwrap();

        let exercises = pollster::block_on(repository.read_exercises()).unwrap();
        assert_eq!(
            exercises
                .iter()
                .map(|e| (e.id, e.name.to_string()))
                .collect::<Vec<_>>(),
            vec![
                (1.into(), "Barbell Bench Press".to_string()),
                (2.into(), "Dip".to_string()),
                (3.into(), "Mystery".to_string()),
            ]
        );
        assert_eq!(
            ExerciseProperties::from(&exercises[1]),
            ExerciseProperties::from(
                crate::catalog::EXERCISES
                    .get(&Name::new("Dip").unwrap())
                    .unwrap()
            )
        );
        assert_eq!(
            ExerciseProperties::from(&exercises[2]),
            ExerciseProperties::default()
        );
        let routines = pollster::block_on(repository.read_routines()).unwrap();
        assert_eq!(
            routines[0].sections,
            vec![section(
                1,
                vec![
                    activity(2, 10),
                    activity(2, 8),
                    activity(1, 5),
                    activity(3, 5)
                ]
            )]
        );
    }

    #[test]
    fn test_import_text_creates_chosen_catalog_exercise_once() {
        let repository = FakeRepository::default();
        let parsed = parse_text(
            "Routine — New\n\n[A] 1 set\n  A1 — Mystery — 10\n  A2 — Enigma — 8\n  A3 — Bench Press — 5",
        )
        .unwrap();

        import(
            &repository,
            &parsed,
            &ImportChoices {
                exercises: BTreeMap::from([
                    (Name::new("Mystery").unwrap(), catalog_choice("Squat")),
                    (Name::new("Enigma").unwrap(), catalog_choice("Squat")),
                    (Name::new("Bench Press").unwrap(), ExerciseChoice::New),
                ]),
                ..ImportChoices::default()
            },
        )
        .unwrap();

        let exercises = pollster::block_on(repository.read_exercises()).unwrap();
        assert_eq!(
            exercises
                .iter()
                .map(|e| (e.name.to_string(), ExerciseProperties::from(e)))
                .collect::<Vec<_>>(),
            vec![
                (
                    "Squat".to_string(),
                    ExerciseProperties::from(
                        crate::catalog::exercise(&Name::new("Squat").unwrap()).unwrap()
                    )
                ),
                ("Bench Press".to_string(), ExerciseProperties::default()),
            ]
        );
        assert_eq!(
            pollster::block_on(repository.read_routines()).unwrap()[0].sections,
            vec![section(
                1,
                vec![activity(1, 10), activity(1, 8), activity(2, 5)]
            )]
        );
    }

    #[test]
    fn test_import_text_uses_chosen_exercise() {
        let repository =
            FakeRepository::default().with_exercises(vec![exercise(1, "Dip"), exercise(2, "dip")]);
        let parsed = parse_text("Routine — New\n\n[A] 1 set\n  A1 — DIP — 10").unwrap();

        import(
            &repository,
            &parsed,
            &ImportChoices {
                exercises: BTreeMap::from([(
                    Name::new("DIP").unwrap(),
                    ExerciseChoice::Existing(2.into()),
                )]),
                ..ImportChoices::default()
            },
        )
        .unwrap();

        assert_eq!(
            pollster::block_on(repository.read_routines()).unwrap()[0].sections,
            vec![section(1, vec![activity(2, 10)])]
        );
        assert_eq!(
            pollster::block_on(repository.read_exercises())
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn test_import_text_replaces_routine() {
        let repository = FakeRepository::default()
            .with_exercises(exercises())
            .with_routines(vec![Routine {
                archived: true,
                ..routine(1, "R", "old", vec![section(1, vec![])])
            }]);
        let parsed = parse_text("Routine — R\n\nnew\n\n[A] 2 sets\n  A1 — A — 5").unwrap();

        import(&repository, &parsed, &ImportChoices::default()).unwrap();

        assert_eq!(
            pollster::block_on(repository.read_routines()).unwrap(),
            vec![Routine {
                archived: true,
                ..routine(1, "R", "new", vec![section(2, vec![activity(1, 5)])])
            }]
        );
    }

    #[test]
    fn test_import_text_copies_routine_and_refers_to_copy_in_schedule() {
        let repository = FakeRepository::default()
            .with_exercises(exercises())
            .with_routines(vec![routine(1, "R", "old", vec![])]);
        let parsed = parse_text("Schedule\n\nMonday\n- R\n\nRoutine — R\n\nnew").unwrap();

        import(
            &repository,
            &parsed,
            &ImportChoices {
                copies: BTreeMap::from([(Name::new("R").unwrap(), Name::new("R2").unwrap())]),
                ..ImportChoices::default()
            },
        )
        .unwrap();

        assert_eq!(
            pollster::block_on(repository.read_routines()).unwrap(),
            vec![
                routine(1, "R", "old", vec![]),
                routine(2, "R2", "new", vec![])
            ]
        );
        assert_eq!(
            pollster::block_on(repository.read_schedule()).unwrap(),
            schedule(
                vec![],
                vec![(Weekday::Monday, vec![ScheduleSlot::Routine(2.into())])]
            )
        );
    }

    #[test]
    fn test_import_text_replaces_schedule() {
        let repository = FakeRepository::default()
            .with_routines(vec![
                routine(1, "A", "", vec![]),
                routine(2, "B", "", vec![]),
            ])
            .with_schedule(schedule(
                vec![(5, "R", vec![1]), (7, "Old", vec![2])],
                vec![
                    (Weekday::Tuesday, vec![ScheduleSlot::Routine(1.into())]),
                    (Weekday::Friday, vec![ScheduleSlot::Rotation(7.into())]),
                ],
            ));
        let parsed = parse_text(
            "Schedule\n\nMonday\n- Rotation — S\n- Rotation — R\n- C\n\nRotation — R\n- B\n- A\n\nRotation — S\n- A\n\nRoutine — C",
        )
        .unwrap();

        import(&repository, &parsed, &ImportChoices::default()).unwrap();

        assert_eq!(
            pollster::block_on(repository.read_schedule()).unwrap(),
            schedule(
                vec![(5, "R", vec![2, 1]), (8, "S", vec![1])],
                vec![(
                    Weekday::Monday,
                    vec![
                        ScheduleSlot::Rotation(8.into()),
                        ScheduleSlot::Rotation(5.into()),
                        ScheduleSlot::Routine(3.into()),
                    ]
                )]
            )
        );
    }

    #[test]
    fn test_import_text_without_schedule_keeps_schedule() {
        let repository = FakeRepository::default();
        let parsed = parse_text("Routine — A").unwrap();

        import(&repository, &parsed, &ImportChoices::default()).unwrap();

        assert!(!repository.calls().contains(&Call::ReadSchedule));
        assert!(!repository.calls().contains(&Call::ReplaceSchedule));
    }

    #[rstest]
    #[case::create_exercise(Call::CreateExercise, "create exercise \"Mystery\"", 0, 0)]
    #[case::create_routine(Call::CreateRoutine, "create routine \"New\"", 1, 0)]
    #[case::modify_routine(Call::ModifyRoutine, "replace routine \"R\"", 1, 1)]
    #[case::replace_schedule(Call::ReplaceSchedule, "replace schedule", 1, 2)]
    fn test_import_text_stops_at_failed_step(
        #[case] call: Call,
        #[case] action: &str,
        #[case] created_exercises: usize,
        #[case] created_routines: usize,
    ) {
        let repository = FakeRepository::default()
            .with_routines(vec![routine(1, "R", "", vec![])])
            .failing(call);
        let parsed = parse_text(
            "Schedule\n\nMonday\n- New\n\nRoutine — New\n\n[A] 1 set\n  A1 — Mystery\n\nRoutine — R\n\nNotes\n\nRoutine — Other",
        )
        .unwrap();

        let error = import(&repository, &parsed, &ImportChoices::default()).unwrap_err();

        assert_eq!(error.action(), action);
        assert_eq!(
            pollster::block_on(repository.read_exercises())
                .unwrap()
                .len(),
            created_exercises
        );
        assert_eq!(
            pollster::block_on(repository.read_routines())
                .unwrap()
                .len()
                - 1,
            created_routines
        );
        assert_eq!(
            pollster::block_on(repository.read_schedule()).unwrap(),
            Schedule::default()
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn test_line_changes_turn_old_into_new(
            old in prop::collection::vec("[ab]", 0..8),
            new in prop::collection::vec("[ab]", 0..8),
        ) {
            let changes = line_changes(&old, &new);
            let kept_or = |removed: bool| {
                changes
                    .iter()
                    .filter_map(|change| match change {
                        LineChange::Kept(line) => Some(line.clone()),
                        LineChange::Removed(line) if removed => Some(line.clone()),
                        LineChange::Added(line) if !removed => Some(line.clone()),
                        LineChange::Removed(_) | LineChange::Added(_) => None,
                    })
                    .collect::<Vec<_>>()
            };

            prop_assert_eq!(kept_or(true), old);
            prop_assert_eq!(kept_or(false), new);
        }

        #[test]
        fn test_routine_round_trip(
            (exercises, routines) in any_exercises_and_routines()
        ) {
            for routine in &routines {
                let parsed = parse_text(&routine.to_text(&exercises))
                    .map_err(|errors| TestCaseError::fail(format!("{errors:?}")))?;

                prop_assert_eq!(parsed.schedule, None);
                prop_assert_eq!(parsed.routines.len(), 1);
                let parsed_routine = &parsed.routines[0];
                prop_assert_eq!(&parsed_routine.name, &routine.name);
                prop_assert_eq!(&parsed_routine.notes, &routine.notes);
                prop_assert_eq!(
                    parsed_routine.sections(&|name| exercise_id_in(&exercises, name)),
                    routine.sections.clone()
                );
            }
        }

        #[test]
        fn test_schedule_round_trip(
            (exercises, routines, schedule) in any_schedule()
        ) {
            let parsed = parse_text(&schedule.to_text(&routines, &exercises))
                .map_err(|errors| TestCaseError::fail(format!("{errors:?}")))?;

            for parsed_routine in &parsed.routines {
                let routine = routines.iter().find(|r| r.name == parsed_routine.name).unwrap();
                prop_assert_eq!(&parsed_routine.notes, &routine.notes);
                prop_assert_eq!(
                    parsed_routine.sections(&|name| exercise_id_in(&exercises, name)),
                    routine.sections.clone()
                );
            }
            prop_assert_eq!(
                parsed.routines.iter().map(|r| r.name.clone()).collect::<BTreeSet<_>>(),
                routines
                    .iter()
                    .filter(|r| schedule.routines().contains(&r.id))
                    .map(|r| r.name.clone())
                    .collect::<BTreeSet<_>>()
            );
            let rebuilt = parsed
                .schedule
                .unwrap()
                .to_schedule(
                    |name| routines.iter().find(|r| r.name == *name).map(|r| r.id),
                    &schedule,
                )
                .unwrap();
            prop_assert_eq!(rebuilt, schedule);
        }
    }

    fn any_exercises_and_routines() -> impl Strategy<Value = (Vec<Exercise>, Vec<Routine>)> {
        (
            prop::sample::subsequence(
                vec![
                    "Bench Press",
                    "90-90 Hip Stretch",
                    "Curl - Hammer",
                    "Row – Cable",
                    "Dip — Weighted",
                    "Rest Pause Squat",
                    "Dead Hang - 30 s",
                    "21s",
                    "5x5",
                    "X",
                ],
                3,
            ),
            prop::sample::subsequence(
                vec![
                    "Push Day",
                    "Monday",
                    "Schedule",
                    "Rest",
                    "- Dash",
                    "Rotation — R",
                    "Rotation - R",
                    "Rotation – R",
                    "Routine — X",
                    "Routine - X",
                    "Routine – X",
                ],
                1..5,
            ),
        )
            .prop_flat_map(|(exercise_names, routine_names)| {
                let exercises = exercise_names
                    .into_iter()
                    .zip(1..)
                    .map(|(name, id)| exercise(id, name))
                    .collect::<Vec<_>>();
                let routines = routine_names
                    .into_iter()
                    .zip(1..)
                    .map(|(name, id)| {
                        (
                            prop::sample::select(vec![
                                "",
                                "Heavy day.",
                                "Monday is heavy.\n\nKeep the rests.",
                                "Tip — keep the elbows in",
                                "- Tip - keep the elbows in",
                            ]),
                            prop::collection::vec(any_section(2), 0..4),
                        )
                            .prop_map(move |(notes, sections)| routine(id, name, notes, sections))
                    })
                    .collect::<Vec<_>>();
                (Just(exercises), routines)
            })
    }

    fn any_schedule() -> impl Strategy<Value = (Vec<Exercise>, Vec<Routine>, Schedule)> {
        any_exercises_and_routines().prop_flat_map(|(exercises, routines)| {
            let routine_ids = routines.iter().map(|r| r.id).collect::<Vec<_>>();
            let rotation_names = vec!["Upper/Lower", "Monday", "Routine — R", "- Rotation"];
            (
                Just(exercises),
                Just(routines),
                prop::sample::subsequence(rotation_names, 0..3).prop_flat_map({
                    let routine_ids = routine_ids.clone();
                    move |names| {
                        names
                            .into_iter()
                            .map(|name| {
                                prop::sample::subsequence(
                                    routine_ids.clone(),
                                    0..=routine_ids.len(),
                                )
                                .prop_map(move |routines| (name, routines))
                            })
                            .collect::<Vec<_>>()
                    }
                }),
            )
                .prop_flat_map(move |(exercises, routines, rotations)| {
                    let rotation_count = rotations.len();
                    let slot = prop_oneof![
                        prop::sample::select(routine_ids.clone()).prop_map(ScheduleSlot::Routine),
                        (0..rotation_count.max(1)).prop_map(move |index| if rotation_count == 0 {
                            ScheduleSlot::Routine(RoutineID::nil())
                        } else {
                            ScheduleSlot::Rotation(RotationID::from(index as u128 + 1))
                        }),
                    ];
                    (
                        Just(exercises),
                        Just(routines),
                        Just(rotations),
                        prop::collection::vec(prop::collection::vec(slot, 0..3), 7),
                    )
                })
                .prop_map(|(exercises, routines, rotations, days)| {
                    let rotations = rotations
                        .into_iter()
                        .zip(1u128..)
                        .map(|((name, routine_ids), id)| {
                            (
                                RotationID::from(id),
                                Rotation::new(Name::new(name).unwrap(), routine_ids).unwrap(),
                            )
                        })
                        .collect();
                    let entries = Weekday::iter()
                        .copied()
                        .zip(days)
                        .map(|(weekday, slots)| {
                            (
                                weekday,
                                slots
                                    .into_iter()
                                    .filter(|slot| *slot != ScheduleSlot::Routine(RoutineID::nil()))
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .filter(|(_, slots)| !slots.is_empty())
                        .collect();
                    (
                        exercises,
                        routines,
                        Schedule::new(rotations, entries).unwrap(),
                    )
                })
        })
    }

    fn any_section(depth: u32) -> BoxedStrategy<RoutinePart> {
        let part = if depth == 0 {
            any_leaf().boxed()
        } else {
            prop_oneof![3 => any_leaf(), 1 => any_section(depth - 1)].boxed()
        };
        (1u32..4, prop::collection::vec(part, 0..4))
            .prop_map(|(rounds, parts)| section(rounds, parts))
            .boxed()
    }

    fn any_leaf() -> impl Strategy<Value = RoutinePart> {
        prop_oneof![
            (
                1u128..=3,
                0u32..20,
                prop::sample::select(vec![0.0f32, 2.5, 60.0]),
                prop::sample::select(vec![0.0f32, 8.0, 8.5]),
                any_tempo(),
                any::<bool>(),
                any::<bool>(),
                any::<bool>(),
            )
                .prop_map(
                    |(exercise_id, reps, weight, rpe, tempo, automatic, countdown, per_side)| {
                        let reps = Reps::new(reps).unwrap();
                        RoutinePart::RoutineActivity {
                            exercise_id: exercise_id.into(),
                            reps,
                            tempo,
                            weight: Weight::new(weight).unwrap(),
                            rpe: RPE::new(rpe).unwrap(),
                            automatic,
                            // The text keeps the flag only where it takes effect.
                            countdown: countdown && SetPacing::countdown_applies(tempo),
                            sides: if per_side {
                                Sides::PerSide
                            } else {
                                Sides::Combined
                            },
                        }
                    }
                ),
            (prop::option::of(1u32..300), any::<bool>())
                .prop_map(|(time, automatic)| { rest(time.as_slice(), automatic) }),
        ]
    }

    fn any_tempo() -> impl Strategy<Value = Tempo> {
        prop::collection::vec(0u32..12, 0..=4)
            .prop_filter_map("valid tempo", |phases| Tempo::new(&phases).ok())
    }

    fn exercise_id_in(exercises: &[Exercise], name: &Name) -> ExerciseID {
        exercises
            .iter()
            .find(|e| e.name == *name)
            .map_or_else(ExerciseID::nil, |e| e.id)
    }

    fn import(
        repository: &FakeRepository,
        text: &ParsedText,
        choices: &ImportChoices,
    ) -> Result<(), ImportError> {
        pollster::block_on(Service::new(repository.clone()).import_text(text, choices))
    }

    fn sections(text: &str) -> Vec<RoutinePart> {
        parse_text(text)
            .unwrap_or_else(|errors| panic!("{errors:?}"))
            .routines[0]
            .sections(&exercise_id)
    }

    fn errors(text: &str) -> Vec<String> {
        parse_text(text)
            .unwrap_err()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn exercise_id(name: &Name) -> ExerciseID {
        match name.as_ref().as_str() {
            "A" => 1.into(),
            "B" => 2.into(),
            _ => 9.into(),
        }
    }

    fn push_day_sections() -> Vec<RoutinePart> {
        vec![
            section(
                3,
                vec![
                    full_activity(1, 8, 60.0, 8.0, &[3, 1, 1, 0], false, Sides::Combined),
                    full_activity(2, 0, 0.0, 0.0, &[30], false, Sides::PerSide),
                    rest(&[90], true),
                    section(
                        2,
                        vec![full_activity(1, 10, 0.0, 9.0, &[], true, Sides::Combined)],
                    ),
                    rest(&[], false),
                ],
            ),
            section(
                1,
                vec![full_activity(
                    2,
                    0,
                    40.0,
                    0.0,
                    &[60],
                    false,
                    Sides::Combined,
                )],
            ),
        ]
    }

    fn full_activity(
        exercise_id: u128,
        reps: u32,
        weight: f32,
        rpe: f32,
        phases: &[u32],
        automatic: bool,
        sides: Sides,
    ) -> RoutinePart {
        RoutinePart::RoutineActivity {
            exercise_id: exercise_id.into(),
            reps: Reps::new(reps).unwrap(),
            tempo: Tempo::new(phases).unwrap(),
            weight: Weight::new(weight).unwrap(),
            rpe: RPE::new(rpe).unwrap(),
            automatic,
            countdown: false,
            sides,
        }
    }

    fn routine_slot(line: usize, name: &str) -> ParsedSlot {
        ParsedSlot::Routine(reference(line, name))
    }

    fn reference(line: usize, name: &str) -> Reference {
        Reference {
            line,
            name: Name::new(name).unwrap(),
        }
    }

    fn catalog_choice(name: &str) -> ExerciseChoice {
        ExerciseChoice::Catalog(Name::new(name).unwrap())
    }

    fn exercise(id: u128, name: &str) -> Exercise {
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

    fn routine(id: u128, name: &str, notes: &str, sections: Vec<RoutinePart>) -> Routine {
        Routine {
            id: id.into(),
            name: Name::new(name).unwrap(),
            notes: notes.to_string(),
            archived: false,
            sections,
        }
    }

    fn example_sections() -> Vec<RoutinePart> {
        vec![
            section(
                2,
                vec![
                    RoutinePart::RoutineActivity {
                        exercise_id: 1.into(),
                        reps: Reps::new(10).unwrap(),
                        tempo: Tempo::new(&[2]).unwrap(),
                        weight: Weight::new(30.0).unwrap(),
                        rpe: RPE::TEN,
                        automatic: false,
                        countdown: false,
                        sides: Sides::Combined,
                    },
                    rest(&[60], true),
                ],
            ),
            section(
                2,
                vec![
                    RoutinePart::RoutineActivity {
                        exercise_id: 2.into(),
                        reps: Reps::new(10).unwrap(),
                        tempo: Tempo::default(),
                        weight: Weight::default(),
                        rpe: RPE::ZERO,
                        automatic: false,
                        countdown: false,
                        sides: Sides::Combined,
                    },
                    rest(&[30], true),
                ],
            ),
        ]
    }

    fn section(rounds: u32, parts: Vec<RoutinePart>) -> RoutinePart {
        RoutinePart::RoutineSection {
            rounds: Rounds::new(rounds).unwrap(),
            parts,
        }
    }

    fn activity(exercise_id: u128, reps: u32) -> RoutinePart {
        RoutinePart::RoutineActivity {
            exercise_id: exercise_id.into(),
            reps: Reps::new(reps).unwrap(),
            tempo: Tempo::default(),
            weight: Weight::default(),
            rpe: RPE::ZERO,
            automatic: false,
            countdown: false,
            sides: Sides::Combined,
        }
    }

    fn rest(phases: &[u32], automatic: bool) -> RoutinePart {
        RoutinePart::RoutineActivity {
            exercise_id: ExerciseID::nil(),
            reps: Reps::default(),
            tempo: Tempo::new(phases).unwrap(),
            weight: Weight::default(),
            rpe: RPE::ZERO,
            automatic,
            countdown: false,
            sides: Sides::Combined,
        }
    }

    fn schedule(
        rotations: Vec<(u128, &str, Vec<u128>)>,
        entries: Vec<(Weekday, Vec<ScheduleSlot>)>,
    ) -> Schedule {
        Schedule::new(
            rotations
                .into_iter()
                .map(|(id, name, routines)| {
                    (
                        RotationID::from(id),
                        Rotation::new(
                            Name::new(name).unwrap(),
                            routines.into_iter().map(RoutineID::from).collect(),
                        )
                        .unwrap(),
                    )
                })
                .collect(),
            entries.into_iter().collect::<BTreeMap<_, _>>(),
        )
        .unwrap()
    }

    fn exercises() -> Vec<Exercise> {
        vec![Exercise {
            id: 1.into(),
            name: Name::new("A").unwrap(),
            notes: String::new(),
            muscles: vec![ExerciseMuscle {
                muscle_id: MuscleID::Pecs,
                stimulus: Stimulus::PRIMARY,
                stretch: Stimulus::NONE,
            }],
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        }]
    }
}
