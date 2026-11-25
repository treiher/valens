use std::{
    collections::{BTreeMap, BTreeSet},
    slice::Iter,
    str::FromStr,
};

use chrono::{Datelike, NaiveDate};
use derive_more::{Deref, Display};
use uuid::Uuid;

use crate::{
    Name, ReadError, RoutineID, SyncError, TrainingSession, TrainingSessionID, UpdateError,
    ValidationError,
};

#[allow(async_fn_in_trait)]
pub trait ScheduleService {
    async fn get_schedule(&self) -> Result<Schedule, ReadError>;
    async fn modify_schedule(&self, schedule: Schedule) -> Result<Schedule, UpdateError>;
}

#[allow(async_fn_in_trait)]
pub trait ScheduleRepository {
    async fn sync_schedule(&self) -> Result<Schedule, SyncError>;
    async fn read_schedule(&self) -> Result<Schedule, ReadError>;
    async fn replace_schedule(&self, schedule: Schedule) -> Result<Schedule, UpdateError>;
}

/// Weekly training plan consisting of rotations and slots on days of the week.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Schedule {
    rotations: BTreeMap<RotationID, Rotation>,
    entries: BTreeMap<Weekday, Vec<ScheduleSlot>>,
}

impl Schedule {
    /// Creates a schedule, ensuring that every rotation slot references a known rotation.
    pub fn new(
        rotations: BTreeMap<RotationID, Rotation>,
        entries: BTreeMap<Weekday, Vec<ScheduleSlot>>,
    ) -> Result<Self, ScheduleError> {
        if entries.values().flatten().any(|slot| {
            matches!(slot, ScheduleSlot::Rotation(rotation_id) if !rotations.contains_key(rotation_id))
        }) {
            return Err(ScheduleError::DanglingRotation);
        }

        Ok(Self { rotations, entries })
    }

    #[must_use]
    pub fn rotations(&self) -> &BTreeMap<RotationID, Rotation> {
        &self.rotations
    }

    #[must_use]
    pub fn entries(&self) -> &BTreeMap<Weekday, Vec<ScheduleSlot>> {
        &self.entries
    }

    /// Appends a slot to `weekday`, ensuring that a rotation slot references a known rotation.
    pub fn add_slot(&mut self, weekday: Weekday, slot: ScheduleSlot) -> Result<(), ScheduleError> {
        let index = self.entries.get(&weekday).map_or(0, Vec::len);
        self.insert_slot(weekday, index, slot)
    }

    /// Inserts a slot at `index` of `weekday`, ensuring that a rotation slot references a known
    /// rotation. An `index` beyond the last slot is clamped.
    pub fn insert_slot(
        &mut self,
        weekday: Weekday,
        index: usize,
        slot: ScheduleSlot,
    ) -> Result<(), ScheduleError> {
        if matches!(slot, ScheduleSlot::Rotation(rotation_id) if !self.rotations.contains_key(&rotation_id))
        {
            return Err(ScheduleError::DanglingRotation);
        }
        let slots = self.entries.entry(weekday).or_default();
        slots.insert(index.min(slots.len()), slot);
        Ok(())
    }

    /// Removes and returns the slot at `index` of `weekday`, or `None` if no such slot exists.
    pub fn remove_slot(&mut self, weekday: Weekday, index: usize) -> Option<ScheduleSlot> {
        let slots = self.entries.get_mut(&weekday)?;
        if index >= slots.len() {
            return None;
        }
        let slot = slots.remove(index);
        if slots.is_empty() {
            self.entries.remove(&weekday);
        }
        Some(slot)
    }

    /// Returns `name` parsed as a rotation name if no rotation other than `exclude` uses it.
    pub fn validate_rotation_name(
        &self,
        name: &str,
        exclude: Option<RotationID>,
    ) -> Result<Name, ValidationError> {
        match Name::new(name) {
            Ok(name) if self.rotation_name_taken(&name, exclude) => {
                Err(ValidationError::Conflict("name".to_string()))
            }
            Ok(name) => Ok(name),
            Err(err) => Err(ValidationError::Other(err.into())),
        }
    }

    /// Whether a rotation other than `exclude` already uses `name`.
    fn rotation_name_taken(&self, name: &Name, exclude: Option<RotationID>) -> bool {
        self.rotations
            .iter()
            .any(|(id, r)| Some(*id) != exclude && r.name == *name)
    }

    /// Adds a rotation or replaces the rotation with the same ID, ensuring that its name is not
    /// used by another rotation.
    pub fn insert_rotation(
        &mut self,
        rotation_id: RotationID,
        rotation: Rotation,
    ) -> Result<(), ScheduleError> {
        if self.rotation_name_taken(&rotation.name, Some(rotation_id)) {
            return Err(ScheduleError::DuplicateRotationName);
        }
        self.rotations.insert(rotation_id, rotation);
        Ok(())
    }

    /// Renames a rotation, ensuring that the name is not used by another rotation.
    pub fn rename_rotation(
        &mut self,
        rotation_id: RotationID,
        name: Name,
    ) -> Result<(), ScheduleError> {
        if self.rotation_name_taken(&name, Some(rotation_id)) {
            return Err(ScheduleError::DuplicateRotationName);
        }
        let Some(rotation) = self.rotations.get_mut(&rotation_id) else {
            return Err(ScheduleError::UnknownRotation);
        };
        rotation.name = name;
        Ok(())
    }

    /// Removes a rotation, ensuring that it is not referenced by any slot.
    pub fn remove_rotation(&mut self, rotation_id: RotationID) -> Result<(), ScheduleError> {
        if self
            .entries
            .values()
            .flatten()
            .any(|slot| *slot == ScheduleSlot::Rotation(rotation_id))
        {
            return Err(ScheduleError::RotationInUse);
        }
        self.rotations.remove(&rotation_id);
        Ok(())
    }

    /// Returns the entries of `date`: a session entry for every training session on `date` with
    /// at least one set, and a planned entry for every pending routine (see `pending_routines`).
    ///
    /// A session takes the first slot of the day resolving to its routine. The entries are
    /// ordered in progress, then planned, then completed. Within each group, entries with a slot
    /// come first in slot order, followed by the other sessions in the order of
    /// `training_sessions`.
    #[must_use]
    pub fn day_entries(
        &self,
        date: NaiveDate,
        training_sessions: &[TrainingSession],
    ) -> Vec<DayEntry> {
        let slots = self.resolved_slots(date, training_sessions);
        let completed = Self::completed_routines(date, training_sessions);
        let slot_of = |routine_id: RoutineID| {
            slots
                .iter()
                .position(|(_, r)| *r == routine_id)
                .map(|idx| (idx, slots[idx].0))
        };
        let sessions = training_sessions
            .iter()
            .filter(|t| t.date == date)
            .filter_map(|t| {
                let (recorded_sets, sets) = t.set_progress();
                if sets == 0 {
                    return None;
                }
                let slot = slot_of(t.routine_id);
                let group = if t.all_sets_recorded() { 2 } else { 0 };
                Some((
                    group,
                    slot.map(|(idx, _)| idx),
                    DayEntry::Session {
                        training_session_id: t.id,
                        routine_id: t.routine_id,
                        slot: slot.map(|(_, slot)| slot),
                        recorded_sets,
                        sets,
                    },
                ))
            });
        let planned = slots
            .iter()
            .enumerate()
            .filter(|(_, (_, routine_id))| !completed.contains(routine_id))
            .map(|(idx, &(slot, routine_id))| {
                (1, Some(idx), DayEntry::Planned { slot, routine_id })
            });
        let mut entries = sessions.chain(planned).collect::<Vec<_>>();
        entries.sort_by_key(|(group, slot_idx, _)| (*group, slot_idx.unwrap_or(usize::MAX)));
        entries.into_iter().map(|(_, _, entry)| entry).collect()
    }

    /// Returns the not yet completed planned routines of `date` in slot order.
    ///
    /// A planned routine is completed if a training session with its routine exists on `date`.
    /// One training session satisfies all slots resolving to the same routine.
    #[must_use]
    pub fn pending_routines(
        &self,
        date: NaiveDate,
        training_sessions: &[TrainingSession],
    ) -> Vec<(ScheduleSlot, RoutineID)> {
        let completed = Self::completed_routines(date, training_sessions);
        self.resolved_slots(date, training_sessions)
            .into_iter()
            .filter(|(_, routine_id)| !completed.contains(routine_id))
            .collect()
    }

    /// Returns the routines of the training sessions on `date`.
    fn completed_routines(
        date: NaiveDate,
        training_sessions: &[TrainingSession],
    ) -> BTreeSet<RoutineID> {
        training_sessions
            .iter()
            .filter(|t| t.date == date)
            .map(|t| t.routine_id)
            .collect()
    }

    /// Returns the slots of `date` with their resolved routines in slot order.
    ///
    /// Rotation slots resolve against the training sessions before `date`; repeated slots of the
    /// same rotation continue the sequence within the day. Slots of empty or unknown rotations
    /// are skipped.
    fn resolved_slots(
        &self,
        date: NaiveDate,
        training_sessions: &[TrainingSession],
    ) -> Vec<(ScheduleSlot, RoutineID)> {
        let Some(slots) = self.entries.get(&Weekday::from(date.weekday())) else {
            return vec![];
        };
        let mut current: BTreeMap<RotationID, RoutineID> = BTreeMap::new();
        let mut result = vec![];
        for slot in slots {
            let routine_id = match slot {
                ScheduleSlot::Routine(routine_id) => *routine_id,
                ScheduleSlot::Rotation(rotation_id) => {
                    let Some(rotation) = self.rotations.get(rotation_id) else {
                        continue;
                    };
                    let next = match current.get(rotation_id) {
                        Some(previous) => rotation.routine_after(*previous),
                        None => rotation
                            .next_routine_in(training_sessions.iter().filter(|t| t.date < date)),
                    };
                    let Some(routine_id) = next else {
                        continue;
                    };
                    current.insert(*rotation_id, routine_id);
                    routine_id
                }
            };
            result.push((*slot, routine_id));
        }
        result
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rotations.is_empty() && self.entries.is_empty()
    }

    /// Returns the routines referenced by any slot or rotation.
    #[must_use]
    pub fn routines(&self) -> BTreeSet<RoutineID> {
        self.rotations
            .values()
            .flat_map(|rotation| rotation.routines.iter().copied())
            .chain(self.entries.values().flatten().filter_map(|slot| {
                if let ScheduleSlot::Routine(routine_id) = slot {
                    Some(*routine_id)
                } else {
                    None
                }
            }))
            .collect()
    }
}

/// Sequence of routines trained in turns on the days the rotation is planned for.
#[derive(Debug, Clone, PartialEq)]
pub struct Rotation {
    pub name: Name,
    routines: Vec<RoutineID>,
}

impl Rotation {
    /// Creates a rotation, ensuring that it contains no duplicate routines.
    pub fn new(name: Name, routines: Vec<RoutineID>) -> Result<Self, RotationError> {
        if routines.iter().collect::<BTreeSet<_>>().len() < routines.len() {
            return Err(RotationError::DuplicateRoutine);
        }

        Ok(Self { name, routines })
    }

    #[must_use]
    pub fn routines(&self) -> &[RoutineID] {
        &self.routines
    }

    /// Returns the routine following the most recently trained routine that is currently a
    /// member of the rotation, wrapping around, the first routine if no training session
    /// matches the current membership, or `None` if the rotation is empty.
    fn next_routine_in<'a>(
        &self,
        training_sessions: impl Iterator<Item = &'a TrainingSession>,
    ) -> Option<RoutineID> {
        let first = *self.routines.first()?;
        Some(
            training_sessions
                .filter(|t| self.routines.contains(&t.routine_id))
                .max_by_key(|t| (t.date, t.id))
                .map_or(first, |t| self.routine_after(t.routine_id).unwrap_or(first)),
        )
    }

    fn routine_after(&self, routine_id: RoutineID) -> Option<RoutineID> {
        let first = *self.routines.first()?;
        Some(
            self.routines
                .iter()
                .position(|r| *r == routine_id)
                .map_or(first, |i| self.routines[(i + 1) % self.routines.len()]),
        )
    }
}

#[derive(Deref, Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RotationID(Uuid);

impl From<Uuid> for RotationID {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<u128> for RotationID {
    fn from(value: u128) -> Self {
        Self(Uuid::from_bytes(value.to_be_bytes()))
    }
}

impl FromStr for RotationID {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(Self)
    }
}

/// An entry of a day, as returned by `Schedule::day_entries`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DayEntry {
    /// A planned routine without a training session on the day.
    Planned {
        slot: ScheduleSlot,
        routine_id: RoutineID,
    },
    /// A training session of the day, with the set progress of [`TrainingSession::set_progress`].
    Session {
        training_session_id: TrainingSessionID,
        routine_id: RoutineID,
        slot: Option<ScheduleSlot>,
        recorded_sets: usize,
        sets: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScheduleSlot {
    Routine(RoutineID),
    Rotation(RotationID),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    pub fn iter() -> Iter<'static, Weekday> {
        static WEEKDAYS: [Weekday; 7] = [
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
            Weekday::Saturday,
            Weekday::Sunday,
        ];
        WEEKDAYS.iter()
    }
}

impl From<chrono::Weekday> for Weekday {
    fn from(value: chrono::Weekday) -> Self {
        match value {
            chrono::Weekday::Mon => Weekday::Monday,
            chrono::Weekday::Tue => Weekday::Tuesday,
            chrono::Weekday::Wed => Weekday::Wednesday,
            chrono::Weekday::Thu => Weekday::Thursday,
            chrono::Weekday::Fri => Weekday::Friday,
            chrono::Weekday::Sat => Weekday::Saturday,
            chrono::Weekday::Sun => Weekday::Sunday,
        }
    }
}

/// ISO 8601 weekday number, Monday = 1
impl From<Weekday> for u8 {
    fn from(value: Weekday) -> Self {
        match value {
            Weekday::Monday => 1,
            Weekday::Tuesday => 2,
            Weekday::Wednesday => 3,
            Weekday::Thursday => 4,
            Weekday::Friday => 5,
            Weekday::Saturday => 6,
            Weekday::Sunday => 7,
        }
    }
}

/// ISO 8601 weekday number, Monday = 1
impl TryFrom<u8> for Weekday {
    type Error = WeekdayError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Weekday::Monday),
            2 => Ok(Weekday::Tuesday),
            3 => Ok(Weekday::Wednesday),
            4 => Ok(Weekday::Thursday),
            5 => Ok(Weekday::Friday),
            6 => Ok(Weekday::Saturday),
            7 => Ok(Weekday::Sunday),
            _ => Err(WeekdayError::OutOfRange),
        }
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ScheduleError {
    #[error("schedule must not contain slots referencing unknown rotations")]
    DanglingRotation,
    #[error("rotation is used in the schedule")]
    RotationInUse,
    #[error("rotation with this name already exists")]
    DuplicateRotationName,
    #[error("unknown rotation")]
    UnknownRotation,
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum RotationError {
    #[error("rotation must not contain duplicate routines")]
    DuplicateRoutine,
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum WeekdayError {
    #[error("weekday must be in the range 1 to 7")]
    OutOfRange,
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use crate::{RPE, Reps, Side, Tempo, Time, TrainingSessionElement, Weight};

    use super::*;

    const PUSH: u128 = 1;
    const PULL: u128 = 2;
    const LEGS: u128 = 3;
    const UPPER: u128 = 4;
    const FULL: u128 = 5;
    const CORE: u128 = 6;

    #[test]
    fn test_rotation_new_empty() {
        let rotation = Rotation::new(Name::new("PPL").unwrap(), vec![]).unwrap();
        assert_eq!(rotation.routines(), []);
        assert_eq!(rotation.next_routine_in([].iter()), None);
    }

    #[test]
    fn test_rotation_new_duplicate_routine() {
        assert_eq!(
            Rotation::new(Name::new("PPL").unwrap(), vec![PUSH.into(), PUSH.into()]),
            Err(RotationError::DuplicateRoutine)
        );
    }

    #[test]
    fn test_rotation_routines() {
        assert_eq!(
            ppl_rotation().routines(),
            [PUSH.into(), PULL.into(), LEGS.into()]
        );
    }

    #[test]
    fn test_schedule_new_dangling_rotation() {
        assert_eq!(
            Schedule::new(
                BTreeMap::new(),
                BTreeMap::from([(Weekday::Monday, vec![ScheduleSlot::Rotation(1.into())])]),
            ),
            Err(ScheduleError::DanglingRotation)
        );
    }

    #[test]
    fn test_schedule_routines() {
        let schedule = Schedule::new(
            BTreeMap::from([(RotationID::from(1), ppl_rotation())]),
            BTreeMap::from([(
                Weekday::Monday,
                vec![
                    ScheduleSlot::Rotation(1.into()),
                    ScheduleSlot::Routine(UPPER.into()),
                ],
            )]),
        )
        .unwrap();
        assert_eq!(
            schedule.routines(),
            BTreeSet::from([PUSH.into(), PULL.into(), LEGS.into(), UPPER.into()])
        );
    }

    #[test]
    fn test_schedule_add_and_remove_slot() {
        let mut schedule = ppl_schedule();
        assert_eq!(
            schedule.add_slot(Weekday::Tuesday, ScheduleSlot::Routine(UPPER.into())),
            Ok(())
        );
        assert_eq!(
            schedule.entries()[&Weekday::Tuesday],
            vec![ScheduleSlot::Routine(UPPER.into())]
        );
        assert_eq!(
            schedule.remove_slot(Weekday::Tuesday, 0),
            Some(ScheduleSlot::Routine(UPPER.into()))
        );
        assert_eq!(schedule, ppl_schedule());
    }

    #[test]
    fn test_schedule_add_slot_dangling_rotation() {
        let mut schedule = ppl_schedule();
        assert_eq!(
            schedule.add_slot(Weekday::Tuesday, ScheduleSlot::Rotation(2.into())),
            Err(ScheduleError::DanglingRotation)
        );
        assert_eq!(schedule, ppl_schedule());
    }

    #[test]
    fn test_schedule_insert_slot_clamps_index() {
        let mut schedule = ppl_schedule();
        assert_eq!(
            schedule.insert_slot(Weekday::Monday, 9, ScheduleSlot::Routine(UPPER.into())),
            Ok(())
        );
        assert_eq!(
            schedule.entries()[&Weekday::Monday],
            vec![
                ScheduleSlot::Rotation(1.into()),
                ScheduleSlot::Routine(UPPER.into()),
            ]
        );
    }

    #[test]
    fn test_schedule_remove_slot_out_of_bounds() {
        let mut schedule = ppl_schedule();
        assert_eq!(schedule.remove_slot(Weekday::Monday, 1), None);
        assert_eq!(schedule.remove_slot(Weekday::Tuesday, 0), None);
        assert_eq!(schedule, ppl_schedule());
    }

    #[test]
    fn test_schedule_insert_and_remove_rotation() {
        let mut schedule = ppl_schedule();
        let rotation = Rotation::new(Name::new("U").unwrap(), vec![UPPER.into()]).unwrap();
        assert_eq!(schedule.insert_rotation(2.into(), rotation.clone()), Ok(()));
        assert_eq!(schedule.rotations()[&RotationID::from(2)], rotation);
        assert_eq!(schedule.remove_rotation(2.into()), Ok(()));
        assert_eq!(schedule, ppl_schedule());
    }

    #[test]
    fn test_schedule_insert_rotation_duplicate_name() {
        let mut schedule = ppl_schedule();
        let rotation = Rotation::new(ppl_rotation().name, vec![UPPER.into()]).unwrap();
        assert_eq!(
            schedule.insert_rotation(2.into(), rotation),
            Err(ScheduleError::DuplicateRotationName)
        );
        assert_eq!(schedule, ppl_schedule());
    }

    #[test]
    fn test_schedule_insert_rotation_replace_keeps_name() {
        let mut schedule = ppl_schedule();
        let rotation = Rotation::new(ppl_rotation().name, vec![UPPER.into()]).unwrap();
        assert_eq!(schedule.insert_rotation(1.into(), rotation.clone()), Ok(()));
        assert_eq!(schedule.rotations()[&RotationID::from(1)], rotation);
    }

    #[test]
    fn test_schedule_rename_rotation() {
        let mut schedule = ppl_schedule();
        let name = Name::new("U").unwrap();
        assert_eq!(schedule.rename_rotation(1.into(), name.clone()), Ok(()));
        assert_eq!(schedule.rotations()[&RotationID::from(1)].name, name);
    }

    #[test]
    fn test_schedule_rename_rotation_unchanged_name() {
        let mut schedule = ppl_schedule();
        assert_eq!(
            schedule.rename_rotation(1.into(), ppl_rotation().name),
            Ok(())
        );
        assert_eq!(schedule, ppl_schedule());
    }

    #[test]
    fn test_schedule_rename_rotation_duplicate_name() {
        let mut schedule = ppl_schedule();
        let rotation = Rotation::new(Name::new("U").unwrap(), vec![UPPER.into()]).unwrap();
        assert_eq!(schedule.insert_rotation(2.into(), rotation.clone()), Ok(()));
        assert_eq!(
            schedule.rename_rotation(2.into(), ppl_rotation().name),
            Err(ScheduleError::DuplicateRotationName)
        );
        assert_eq!(schedule.rotations()[&RotationID::from(2)], rotation);
    }

    #[test]
    fn test_schedule_rename_rotation_unknown_rotation() {
        let mut schedule = ppl_schedule();
        assert_eq!(
            schedule.rename_rotation(2.into(), Name::new("U").unwrap()),
            Err(ScheduleError::UnknownRotation)
        );
        assert_eq!(schedule, ppl_schedule());
    }

    #[test]
    fn test_schedule_validate_rotation_name() {
        let schedule = ppl_schedule();
        assert_eq!(
            schedule.validate_rotation_name("U", None).unwrap(),
            Name::new("U").unwrap()
        );
        assert_eq!(
            schedule
                .validate_rotation_name("PPL", None)
                .unwrap_err()
                .to_string(),
            "entry with this name already exists"
        );
        assert_eq!(
            schedule
                .validate_rotation_name("PPL", Some(1.into()))
                .unwrap(),
            Name::new("PPL").unwrap()
        );
        assert!(schedule.validate_rotation_name("", None).is_err());
    }

    #[test]
    fn test_schedule_remove_rotation_in_use() {
        let mut schedule = ppl_schedule();
        assert_eq!(
            schedule.remove_rotation(1.into()),
            Err(ScheduleError::RotationInUse)
        );
        assert_eq!(schedule, ppl_schedule());
    }

    #[test]
    fn test_next_routine_never_trained() {
        assert_eq!(ppl_rotation().next_routine_in([].iter()), Some(PUSH.into()));
    }

    #[test]
    fn test_next_routine_after_most_recent_session() {
        let training_sessions = [
            training_session(1, PUSH, date(2020, 3, 2)),
            training_session(2, PULL, date(2020, 3, 4)),
        ];
        assert_eq!(
            ppl_rotation().next_routine_in(training_sessions.iter()),
            Some(LEGS.into())
        );
    }

    #[test]
    fn test_next_routine_wrap_around() {
        let training_sessions = [training_session(1, LEGS, date(2020, 3, 6))];
        assert_eq!(
            ppl_rotation().next_routine_in(training_sessions.iter()),
            Some(PUSH.into())
        );
    }

    #[test]
    fn test_next_routine_advanced_by_ad_hoc_session() {
        let training_sessions = [
            training_session(1, PUSH, date(2020, 3, 2)),
            training_session(2, PULL, date(2020, 3, 3)),
        ];
        assert_eq!(
            ppl_rotation().next_routine_in(training_sessions.iter()),
            Some(LEGS.into())
        );
    }

    #[test]
    fn test_next_routine_ignores_sessions_of_non_members() {
        let training_sessions = [
            training_session(1, PUSH, date(2020, 3, 2)),
            training_session(2, UPPER, date(2020, 3, 4)),
        ];
        assert_eq!(
            ppl_rotation().next_routine_in(training_sessions.iter()),
            Some(PULL.into())
        );
    }

    #[test]
    fn test_next_routine_most_recent_member_removed() {
        let rotation =
            Rotation::new(Name::new("PL").unwrap(), vec![PUSH.into(), LEGS.into()]).unwrap();
        let training_sessions = [
            training_session(1, PUSH, date(2020, 3, 2)),
            training_session(2, PULL, date(2020, 3, 4)),
        ];
        assert_eq!(
            rotation.next_routine_in(training_sessions.iter()),
            Some(LEGS.into())
        );
    }

    #[test]
    fn test_pending_routines_resumes_after_skipped_day() {
        // P/P/L planned on Monday, Wednesday and Friday, Wednesday skipped
        let schedule = ppl_schedule();
        let training_sessions = [training_session(1, PUSH, date(2020, 3, 2))];
        assert_eq!(
            schedule.pending_routines(date(2020, 3, 6), &training_sessions),
            vec![(ScheduleSlot::Rotation(1.into()), PULL.into())]
        );
    }

    #[test]
    fn test_pending_routines_fixed_and_rotation_on_same_day() {
        let schedule = Schedule::new(
            BTreeMap::from([(RotationID::from(1), ppl_rotation())]),
            BTreeMap::from([(
                Weekday::Monday,
                vec![
                    ScheduleSlot::Rotation(1.into()),
                    ScheduleSlot::Routine(UPPER.into()),
                ],
            )]),
        )
        .unwrap();
        assert_eq!(
            schedule.pending_routines(date(2020, 3, 2), &[]),
            vec![
                (ScheduleSlot::Rotation(1.into()), PUSH.into()),
                (ScheduleSlot::Routine(UPPER.into()), UPPER.into()),
            ]
        );
    }

    #[test]
    fn test_pending_routines_completed_session_removes_entry() {
        let schedule = ppl_schedule();
        let training_sessions = [
            training_session(1, PUSH, date(2020, 3, 2)),
            training_session(2, PULL, date(2020, 3, 4)),
        ];
        assert_eq!(
            schedule.pending_routines(date(2020, 3, 4), &training_sessions),
            vec![]
        );
    }

    #[test]
    fn test_pending_routines_same_rotation_twice_advances_sequentially() {
        let schedule = Schedule::new(
            BTreeMap::from([(RotationID::from(1), ppl_rotation())]),
            BTreeMap::from([(
                Weekday::Monday,
                vec![
                    ScheduleSlot::Rotation(1.into()),
                    ScheduleSlot::Rotation(1.into()),
                ],
            )]),
        )
        .unwrap();
        assert_eq!(
            schedule.pending_routines(date(2020, 3, 2), &[]),
            vec![
                (ScheduleSlot::Rotation(1.into()), PUSH.into()),
                (ScheduleSlot::Rotation(1.into()), PULL.into()),
            ]
        );
        let training_sessions = [training_session(1, PUSH, date(2020, 3, 2))];
        assert_eq!(
            schedule.pending_routines(date(2020, 3, 2), &training_sessions),
            vec![(ScheduleSlot::Rotation(1.into()), PULL.into())]
        );
    }

    #[test]
    fn test_pending_routines_wrap_around() {
        let schedule = ppl_schedule();
        let training_sessions = [training_session(1, LEGS, date(2020, 3, 4))];
        assert_eq!(
            schedule.pending_routines(date(2020, 3, 6), &training_sessions),
            vec![(ScheduleSlot::Rotation(1.into()), PUSH.into())]
        );
    }

    #[test]
    fn test_pending_routines_skips_empty_rotation() {
        let schedule = Schedule::new(
            BTreeMap::from([(
                RotationID::from(1),
                Rotation::new(Name::new("PPL").unwrap(), vec![]).unwrap(),
            )]),
            BTreeMap::from([(
                Weekday::Monday,
                vec![
                    ScheduleSlot::Rotation(1.into()),
                    ScheduleSlot::Routine(UPPER.into()),
                ],
            )]),
        )
        .unwrap();
        assert_eq!(
            schedule.pending_routines(date(2020, 3, 2), &[]),
            vec![(ScheduleSlot::Routine(UPPER.into()), UPPER.into())]
        );
    }

    #[test]
    fn test_pending_routines_no_entries_for_weekday() {
        assert_eq!(
            ppl_schedule().pending_routines(date(2020, 3, 3), &[]),
            vec![]
        );
    }

    #[test]
    fn test_day_entries_planned_routine_without_session() {
        assert_eq!(
            ppl_schedule().day_entries(date(2020, 3, 2), &[]),
            vec![DayEntry::Planned {
                slot: ScheduleSlot::Rotation(1.into()),
                routine_id: PUSH.into(),
            }]
        );
    }

    #[rstest]
    #[case::in_progress(1)]
    #[case::completed(2)]
    fn test_day_entries_planned_routine_with_session(#[case] recorded_sets: usize) {
        let training_sessions = [session_with_sets(
            1,
            PUSH,
            date(2020, 3, 2),
            recorded_sets,
            2,
        )];
        assert_eq!(
            ppl_schedule().day_entries(date(2020, 3, 2), &training_sessions),
            vec![DayEntry::Session {
                training_session_id: 1.into(),
                routine_id: PUSH.into(),
                slot: Some(ScheduleSlot::Rotation(1.into())),
                recorded_sets,
                sets: 2,
            }]
        );
    }

    #[test]
    fn test_day_entries_sessions_without_slot() {
        let training_sessions = [
            session_with_sets(1, UPPER, date(2020, 3, 3), 0, 1),
            session_with_sets(2, 0, date(2020, 3, 3), 0, 1),
        ];
        assert_eq!(
            ppl_schedule().day_entries(date(2020, 3, 3), &training_sessions),
            vec![
                DayEntry::Session {
                    training_session_id: 1.into(),
                    routine_id: UPPER.into(),
                    slot: None,
                    recorded_sets: 0,
                    sets: 1,
                },
                DayEntry::Session {
                    training_session_id: 2.into(),
                    routine_id: RoutineID::nil(),
                    slot: None,
                    recorded_sets: 0,
                    sets: 1,
                },
            ]
        );
    }

    #[test]
    fn test_day_entries_session_of_rotation_routine_takes_rotation_slot() {
        let training_sessions = [
            training_session(1, PUSH, date(2020, 3, 2)),
            session_with_sets(2, PULL, date(2020, 3, 4), 0, 1),
        ];
        assert_eq!(
            ppl_schedule().day_entries(date(2020, 3, 4), &training_sessions),
            vec![DayEntry::Session {
                training_session_id: 2.into(),
                routine_id: PULL.into(),
                slot: Some(ScheduleSlot::Rotation(1.into())),
                recorded_sets: 0,
                sets: 1,
            }]
        );
    }

    #[test]
    fn test_day_entries_session_without_set_satisfies_slot() {
        let training_sessions = [training_session(1, PUSH, date(2020, 3, 2))];
        assert_eq!(
            ppl_schedule().day_entries(date(2020, 3, 2), &training_sessions),
            vec![]
        );
    }

    #[test]
    fn test_day_entries_ignores_sessions_of_other_days() {
        let training_sessions = [session_with_sets(1, UPPER, date(2020, 3, 1), 0, 1)];
        assert_eq!(
            ppl_schedule().day_entries(date(2020, 3, 2), &training_sessions),
            vec![DayEntry::Planned {
                slot: ScheduleSlot::Rotation(1.into()),
                routine_id: PUSH.into(),
            }]
        );
    }

    #[test]
    fn test_day_entries_order() {
        let schedule = Schedule::new(
            BTreeMap::new(),
            BTreeMap::from([(
                Weekday::Monday,
                [PUSH, PULL, LEGS, UPPER, FULL, CORE]
                    .map(|r| ScheduleSlot::Routine(r.into()))
                    .to_vec(),
            )]),
        )
        .unwrap();
        let training_sessions = [
            session_with_sets(1, 0, date(2020, 3, 2), 1, 1),
            session_with_sets(2, 0, date(2020, 3, 2), 0, 1),
            session_with_sets(3, CORE, date(2020, 3, 2), 1, 1),
            session_with_sets(4, UPPER, date(2020, 3, 2), 0, 1),
            session_with_sets(5, PUSH, date(2020, 3, 2), 1, 1),
            session_with_sets(6, PULL, date(2020, 3, 2), 0, 1),
        ];
        let entries = schedule.day_entries(date(2020, 3, 2), &training_sessions);
        assert_eq!(
            entries
                .iter()
                .map(|entry| match entry {
                    DayEntry::Planned { routine_id, .. } => (None, *routine_id),
                    DayEntry::Session {
                        training_session_id,
                        routine_id,
                        ..
                    } => (Some(*training_session_id), *routine_id),
                })
                .collect::<Vec<_>>(),
            vec![
                (Some(6.into()), PULL.into()),
                (Some(4.into()), UPPER.into()),
                (Some(2.into()), RoutineID::nil()),
                (None, LEGS.into()),
                (None, FULL.into()),
                (Some(5.into()), PUSH.into()),
                (Some(3.into()), CORE.into()),
                (Some(1.into()), RoutineID::nil()),
            ]
        );
    }

    #[test]
    fn test_day_entries_planned_routines_keep_their_slot_order() {
        let schedule = Schedule::new(
            BTreeMap::from([(RotationID::from(1), ppl_rotation())]),
            BTreeMap::from([(
                Weekday::Monday,
                vec![
                    ScheduleSlot::Rotation(1.into()),
                    ScheduleSlot::Routine(PULL.into()),
                    ScheduleSlot::Routine(PUSH.into()),
                ],
            )]),
        )
        .unwrap();
        assert_eq!(
            schedule.day_entries(date(2020, 3, 2), &[]),
            vec![
                DayEntry::Planned {
                    slot: ScheduleSlot::Rotation(1.into()),
                    routine_id: PUSH.into(),
                },
                DayEntry::Planned {
                    slot: ScheduleSlot::Routine(PULL.into()),
                    routine_id: PULL.into(),
                },
                DayEntry::Planned {
                    slot: ScheduleSlot::Routine(PUSH.into()),
                    routine_id: PUSH.into(),
                },
            ]
        );
    }

    #[rstest]
    #[case(date(2020, 3, 2), Weekday::Monday)]
    #[case(date(2020, 3, 3), Weekday::Tuesday)]
    #[case(date(2020, 3, 4), Weekday::Wednesday)]
    #[case(date(2020, 3, 5), Weekday::Thursday)]
    #[case(date(2020, 3, 6), Weekday::Friday)]
    #[case(date(2020, 3, 7), Weekday::Saturday)]
    #[case(date(2020, 3, 8), Weekday::Sunday)]
    fn test_weekday_from_chrono_weekday(#[case] date: NaiveDate, #[case] expected: Weekday) {
        assert_eq!(Weekday::from(date.weekday()), expected);
    }

    #[test]
    fn test_weekday_iter() {
        assert_eq!(
            Weekday::iter().copied().collect::<Vec<_>>(),
            [
                Weekday::Monday,
                Weekday::Tuesday,
                Weekday::Wednesday,
                Weekday::Thursday,
                Weekday::Friday,
                Weekday::Saturday,
                Weekday::Sunday
            ]
        );
    }

    #[test]
    fn test_weekday_number_round_trip() {
        for weekday in Weekday::iter() {
            assert_eq!(Weekday::try_from(u8::from(*weekday)), Ok(*weekday));
        }
    }

    #[rstest]
    #[case(0)]
    #[case(8)]
    fn test_weekday_try_from_out_of_range(#[case] number: u8) {
        assert_eq!(Weekday::try_from(number), Err(WeekdayError::OutOfRange));
    }

    fn ppl_rotation() -> Rotation {
        Rotation::new(
            Name::new("PPL").unwrap(),
            vec![PUSH.into(), PULL.into(), LEGS.into()],
        )
        .unwrap()
    }

    fn ppl_schedule() -> Schedule {
        Schedule::new(
            BTreeMap::from([(RotationID::from(1), ppl_rotation())]),
            BTreeMap::from([
                (Weekday::Monday, vec![ScheduleSlot::Rotation(1.into())]),
                (Weekday::Wednesday, vec![ScheduleSlot::Rotation(1.into())]),
                (Weekday::Friday, vec![ScheduleSlot::Rotation(1.into())]),
            ]),
        )
        .unwrap()
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn training_session(id: u128, routine_id: u128, date: NaiveDate) -> TrainingSession {
        TrainingSession {
            id: id.into(),
            routine_id: routine_id.into(),
            date,
            notes: String::new(),
            elements: vec![],
            exercise_notes: BTreeMap::new(),
            exercise_measurements: BTreeMap::new(),
        }
    }

    /// A training session with `sets` sets, the first `recorded_sets` of them with input
    fn session_with_sets(
        id: u128,
        routine_id: u128,
        date: NaiveDate,
        recorded_sets: usize,
        sets: usize,
    ) -> TrainingSession {
        TrainingSession {
            elements: (0..sets)
                .map(|i| TrainingSessionElement::Set {
                    exercise_id: 1.into(),
                    side: Side::Unset,
                    reps: Reps::new(u32::from(i < recorded_sets)).unwrap(),
                    time: Time::default(),
                    weight: Weight::default(),
                    rpe: RPE::default(),
                    target_reps: Reps::default(),
                    target_tempo: Tempo::default(),
                    target_weight: Weight::default(),
                    target_rpe: RPE::default(),
                    automatic: false,
                    countdown: false,
                })
                .collect(),
            ..training_session(id, routine_id, date)
        }
    }

    #[test]
    fn test_schedule_is_empty() {
        let mut schedule = Schedule::default();
        assert!(schedule.is_empty());

        schedule
            .add_slot(Weekday::Monday, ScheduleSlot::Routine(PUSH.into()))
            .unwrap();
        assert!(!schedule.is_empty());
    }

    #[test]
    fn test_schedule_pending_routines_skips_unknown_rotation() {
        let schedule = Schedule {
            rotations: BTreeMap::new(),
            entries: BTreeMap::from([(
                Weekday::Monday,
                vec![
                    ScheduleSlot::Rotation(1.into()),
                    ScheduleSlot::Routine(PUSH.into()),
                ],
            )]),
        };

        assert_eq!(
            schedule.pending_routines(date(2020, 3, 2), &[]),
            [(ScheduleSlot::Routine(PUSH.into()), RoutineID::from(PUSH))]
        );
    }

    #[test]
    fn test_rotation_id_from_str() {
        let id = RotationID::from(Uuid::from_u128(1));

        assert_eq!(id, RotationID::from(1u128));
        assert_eq!(RotationID::from_str(&id.to_string()), Ok(id));
    }
}
