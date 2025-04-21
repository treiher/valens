use std::{
    collections::{BTreeMap, HashSet},
    ops::{Add, AddAssign, Mul},
    slice::Iter,
};

use derive_more::Deref;
use uuid::Uuid;

use crate::{CreateError, DeleteError, Name, ReadError, SyncError, UpdateError, catalog};

#[allow(async_fn_in_trait)]
pub trait ExerciseRepository {
    async fn sync_exercises(&self) -> Result<Vec<Exercise>, SyncError>;
    async fn read_exercises(&self) -> Result<Vec<Exercise>, ReadError>;
    async fn create_exercise(
        &self,
        name: Name,
        muscles: Vec<ExerciseMuscle>,
    ) -> Result<Exercise, CreateError>;
    async fn replace_exercise(&self, exercise: Exercise) -> Result<Exercise, UpdateError>;
    async fn delete_exercise(&self, id: ExerciseID) -> Result<ExerciseID, DeleteError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exercise {
    pub id: ExerciseID,
    pub name: Name,
    pub muscles: Vec<ExerciseMuscle>,
}

impl Exercise {
    #[must_use]
    pub fn muscle_stimulus(&self) -> BTreeMap<MuscleID, Stimulus> {
        self.muscles
            .iter()
            .map(|m| (m.muscle_id, m.stimulus))
            .collect()
    }
}

#[derive(Deref, Debug, Default, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExerciseID(Uuid);

impl ExerciseID {
    #[must_use]
    pub fn nil() -> Self {
        Self(Uuid::nil())
    }

    #[must_use]
    pub fn is_nil(&self) -> bool {
        self.0.is_nil()
    }
}

impl From<Uuid> for ExerciseID {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<u128> for ExerciseID {
    fn from(value: u128) -> Self {
        Self(Uuid::from_bytes(value.to_be_bytes()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExerciseMuscle {
    pub muscle_id: MuscleID,
    pub stimulus: Stimulus,
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stimulus(u32);

impl Stimulus {
    pub const PRIMARY: Stimulus = Stimulus(100);
    pub const SECONDARY: Stimulus = Stimulus(50);
    pub const NONE: Stimulus = Stimulus(0);

    pub fn new(value: u32) -> Result<Self, StimulusError> {
        if value > 100 {
            return Err(StimulusError::OutOfRange(value));
        }
        Ok(Self(value))
    }
}

impl Add for Stimulus {
    type Output = Stimulus;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Stimulus {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self(self.0 + rhs.0);
    }
}

impl Mul<u32> for Stimulus {
    type Output = Stimulus;

    fn mul(self, rhs: u32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum StimulusError {
    #[error("Stimulus must be 100 or less ({0} > 100)")]
    OutOfRange(u32),
}

#[derive(Clone, Copy, Default, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum MuscleID {
    #[default]
    None = 0,
    // Neck
    Neck = 1,
    // Chest
    Pecs = 11,
    // Back
    Traps = 21,
    Lats = 22,
    // Shoulders
    FrontDelts = 31,
    SideDelts = 32,
    RearDelts = 33,
    // Upper arms
    Biceps = 41,
    Triceps = 42,
    // Forearms
    Forearms = 51,
    // Waist
    Abs = 61,
    ErectorSpinae = 62,
    // Hips
    Glutes = 71,
    Abductors = 72,
    // Thighs
    Quads = 81,
    Hamstrings = 82,
    Adductors = 83,
    // Calves
    Calves = 91,
}

impl Property for MuscleID {
    fn iter() -> Iter<'static, MuscleID> {
        static MUSCLES: [MuscleID; 18] = [
            MuscleID::Neck,
            MuscleID::Pecs,
            MuscleID::Traps,
            MuscleID::Lats,
            MuscleID::FrontDelts,
            MuscleID::SideDelts,
            MuscleID::RearDelts,
            MuscleID::Biceps,
            MuscleID::Triceps,
            MuscleID::Forearms,
            MuscleID::Abs,
            MuscleID::ErectorSpinae,
            MuscleID::Glutes,
            MuscleID::Abductors,
            MuscleID::Quads,
            MuscleID::Hamstrings,
            MuscleID::Adductors,
            MuscleID::Calves,
        ];
        MUSCLES.iter()
    }

    fn iter_filter() -> Iter<'static, MuscleID> {
        static MUSCLES: [MuscleID; 19] = [
            MuscleID::Neck,
            MuscleID::Pecs,
            MuscleID::Traps,
            MuscleID::Lats,
            MuscleID::FrontDelts,
            MuscleID::SideDelts,
            MuscleID::RearDelts,
            MuscleID::Biceps,
            MuscleID::Triceps,
            MuscleID::Forearms,
            MuscleID::Abs,
            MuscleID::ErectorSpinae,
            MuscleID::Glutes,
            MuscleID::Abductors,
            MuscleID::Quads,
            MuscleID::Hamstrings,
            MuscleID::Adductors,
            MuscleID::Calves,
            MuscleID::None,
        ];
        MUSCLES.iter()
    }

    #[must_use]
    fn name(self) -> &'static str {
        match self {
            MuscleID::None => "No Muscle",
            MuscleID::Neck => "Neck",
            MuscleID::Pecs => "Pecs",
            MuscleID::Traps => "Traps",
            MuscleID::Lats => "Lats",
            MuscleID::FrontDelts => "Front Delts",
            MuscleID::SideDelts => "Side Delts",
            MuscleID::RearDelts => "Rear Delts",
            MuscleID::Biceps => "Biceps",
            MuscleID::Triceps => "Triceps",
            MuscleID::Forearms => "Forearms",
            MuscleID::Abs => "Abs",
            MuscleID::ErectorSpinae => "Erector Spinae",
            MuscleID::Glutes => "Glutes",
            MuscleID::Abductors => "Abductors",
            MuscleID::Quads => "Quads",
            MuscleID::Hamstrings => "Hamstrings",
            MuscleID::Adductors => "Adductors",
            MuscleID::Calves => "Calves",
        }
    }
}

impl MuscleID {
    #[must_use]
    pub fn description(self) -> &'static str {
        #[allow(clippy::match_same_arms)]
        match self {
            MuscleID::None => "",
            MuscleID::Neck => "",
            MuscleID::Pecs => "Chest",
            MuscleID::Traps => "Upper back",
            MuscleID::Lats => "Sides of back",
            MuscleID::FrontDelts => "Anterior shoulders",
            MuscleID::SideDelts => "Mid shoulders",
            MuscleID::RearDelts => "Posterior shoulders",
            MuscleID::Biceps => "Front of upper arms",
            MuscleID::Triceps => "Back of upper arms",
            MuscleID::Forearms => "",
            MuscleID::Abs => "Belly",
            MuscleID::ErectorSpinae => "Lower back and spine",
            MuscleID::Glutes => "Buttocks",
            MuscleID::Abductors => "Outside of hips",
            MuscleID::Quads => "Front of thighs",
            MuscleID::Hamstrings => "Back of thighs",
            MuscleID::Adductors => "Inner thighs",
            MuscleID::Calves => "Back of lower legs",
        }
    }
}

impl TryFrom<u8> for MuscleID {
    type Error = MuscleIDError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            x if x == MuscleID::Neck as u8 => Ok(MuscleID::Neck),
            x if x == MuscleID::Pecs as u8 => Ok(MuscleID::Pecs),
            x if x == MuscleID::Traps as u8 => Ok(MuscleID::Traps),
            x if x == MuscleID::Lats as u8 => Ok(MuscleID::Lats),
            x if x == MuscleID::FrontDelts as u8 => Ok(MuscleID::FrontDelts),
            x if x == MuscleID::SideDelts as u8 => Ok(MuscleID::SideDelts),
            x if x == MuscleID::RearDelts as u8 => Ok(MuscleID::RearDelts),
            x if x == MuscleID::Biceps as u8 => Ok(MuscleID::Biceps),
            x if x == MuscleID::Triceps as u8 => Ok(MuscleID::Triceps),
            x if x == MuscleID::Forearms as u8 => Ok(MuscleID::Forearms),
            x if x == MuscleID::Abs as u8 => Ok(MuscleID::Abs),
            x if x == MuscleID::ErectorSpinae as u8 => Ok(MuscleID::ErectorSpinae),
            x if x == MuscleID::Glutes as u8 => Ok(MuscleID::Glutes),
            x if x == MuscleID::Abductors as u8 => Ok(MuscleID::Abductors),
            x if x == MuscleID::Quads as u8 => Ok(MuscleID::Quads),
            x if x == MuscleID::Hamstrings as u8 => Ok(MuscleID::Hamstrings),
            x if x == MuscleID::Adductors as u8 => Ok(MuscleID::Adductors),
            x if x == MuscleID::Calves as u8 => Ok(MuscleID::Calves),
            _ => Err(MuscleIDError::Invalid),
        }
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum MuscleIDError {
    #[error("Invalid muscle ID")]
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Force {
    Push,
    Pull,
    Static,
}

impl Property for Force {
    fn iter() -> Iter<'static, Force> {
        static FORCE: [Force; 3] = [Force::Push, Force::Pull, Force::Static];
        FORCE.iter()
    }

    fn name(self) -> &'static str {
        match self {
            Force::Push => "Push",
            Force::Pull => "Pull",
            Force::Static => "Static",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Mechanic {
    Compound,
    Isolation,
}

impl Property for Mechanic {
    fn iter() -> Iter<'static, Mechanic> {
        static MECHANIC: [Mechanic; 2] = [Mechanic::Compound, Mechanic::Isolation];
        MECHANIC.iter()
    }

    fn name(self) -> &'static str {
        match self {
            Mechanic::Compound => "Compound",
            Mechanic::Isolation => "Isolation",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Laterality {
    Bilateral,
    Unilateral,
}

impl Property for Laterality {
    fn iter() -> Iter<'static, Laterality> {
        static LATERALITY: [Laterality; 2] = [Laterality::Bilateral, Laterality::Unilateral];
        LATERALITY.iter()
    }

    fn name(self) -> &'static str {
        match self {
            Laterality::Bilateral => "Bilateral",
            Laterality::Unilateral => "Unilateral",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Assistance {
    Unassisted,
    Assisted,
}

impl Property for Assistance {
    fn iter() -> Iter<'static, Assistance> {
        static ASSISTANCE: [Assistance; 2] = [Assistance::Unassisted, Assistance::Assisted];
        ASSISTANCE.iter()
    }

    fn name(self) -> &'static str {
        match self {
            Assistance::Unassisted => "Unassisted",
            Assistance::Assisted => "Assisted",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Equipment {
    None,
    Barbell,
    Box,
    Cable,
    Dumbbell,
    ExerciseBall,
    GymnasticRings,
    Kettlebell,
    Machine,
    ParallelBars,
    PullUpBar,
    ResistanceBand,
    Sliders,
    TrapBar,
}

impl Property for Equipment {
    fn iter() -> Iter<'static, Equipment> {
        static EQUIPMENT: [Equipment; 13] = [
            Equipment::Barbell,
            Equipment::Box,
            Equipment::Cable,
            Equipment::Dumbbell,
            Equipment::ExerciseBall,
            Equipment::GymnasticRings,
            Equipment::Kettlebell,
            Equipment::Machine,
            Equipment::ParallelBars,
            Equipment::PullUpBar,
            Equipment::ResistanceBand,
            Equipment::Sliders,
            Equipment::TrapBar,
        ];
        EQUIPMENT.iter()
    }

    fn iter_filter() -> Iter<'static, Equipment> {
        static EQUIPMENT: [Equipment; 14] = [
            Equipment::Barbell,
            Equipment::Box,
            Equipment::Cable,
            Equipment::Dumbbell,
            Equipment::ExerciseBall,
            Equipment::GymnasticRings,
            Equipment::Kettlebell,
            Equipment::Machine,
            Equipment::ParallelBars,
            Equipment::PullUpBar,
            Equipment::ResistanceBand,
            Equipment::Sliders,
            Equipment::TrapBar,
            Equipment::None,
        ];
        EQUIPMENT.iter()
    }

    fn name(self) -> &'static str {
        match self {
            Equipment::None => "No Equipment",
            Equipment::Barbell => "Barbell",
            Equipment::Box => "Box",
            Equipment::Cable => "Cable",
            Equipment::Dumbbell => "Dumbbell",
            Equipment::ExerciseBall => "Exercise Ball",
            Equipment::GymnasticRings => "Gymnastic Rings",
            Equipment::Kettlebell => "Kettlebell",
            Equipment::Machine => "Machine",
            Equipment::ParallelBars => "Parallel Bars",
            Equipment::PullUpBar => "Pull Up Bar",
            Equipment::ResistanceBand => "Resistance Band",
            Equipment::Sliders => "Sliders",
            Equipment::TrapBar => "Trap Bar",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Category {
    Strength,
    Plyometrics,
}

impl Property for Category {
    fn iter() -> Iter<'static, Category> {
        static CATEGORY: [Category; 2] = [Category::Strength, Category::Plyometrics];
        CATEGORY.iter()
    }

    fn name(self) -> &'static str {
        match self {
            Category::Strength => "Strength",
            Category::Plyometrics => "Plyometrics",
        }
    }
}

#[derive(Default, PartialEq)]
pub struct ExerciseFilter {
    pub name: String,
    pub muscles: HashSet<MuscleID>,
    pub force: HashSet<Force>,
    pub mechanic: HashSet<Mechanic>,
    pub laterality: HashSet<Laterality>,
    pub assistance: HashSet<Assistance>,
    pub equipment: HashSet<Equipment>,
    pub category: HashSet<Category>,
}

impl ExerciseFilter {
    #[must_use]
    pub fn exercises<'a>(
        &self,
        exercises: impl Iterator<Item = &'a Exercise>,
    ) -> Vec<&'a Exercise> {
        exercises
            .filter(|e| {
                e.name
                    .as_ref()
                    .to_lowercase()
                    .contains(self.name.to_lowercase().trim())
                    && (self.muscles.is_empty()
                        || self.muscles.iter().all(|m| {
                            if *m == MuscleID::None {
                                e.muscles.is_empty()
                            } else {
                                e.muscle_stimulus().contains_key(m)
                            }
                        }))
                    && self.force.is_empty()
                    && self.mechanic.is_empty()
                    && self.laterality.is_empty()
                    && self.equipment.is_empty()
                    && self.category.is_empty()
            })
            .collect()
    }

    #[must_use]
    pub fn catalog(&self) -> BTreeMap<&'static Name, &'static catalog::Exercise> {
        catalog::EXERCISES
            .values()
            .filter(|e| {
                e.name
                    .as_ref()
                    .to_lowercase()
                    .contains(self.name.to_lowercase().trim())
                    && (self.muscles.is_empty()
                        || self.muscles.iter().all(|muscle| {
                            if *muscle == MuscleID::None {
                                e.muscles.is_empty()
                            } else {
                                e.muscles.iter().any(|(m, _)| muscle == m)
                            }
                        }))
                    && (self.force.is_empty() || self.force.contains(&e.force))
                    && (self.mechanic.is_empty() || self.mechanic.contains(&e.mechanic))
                    && (self.laterality.is_empty() || self.laterality.contains(&e.laterality))
                    && (self.assistance.is_empty() || self.assistance.contains(&e.assistance))
                    && (self.equipment.is_empty()
                        || self.equipment.iter().any(|equipment| {
                            if *equipment == Equipment::None {
                                e.equipment.is_empty()
                            } else {
                                e.equipment.iter().any(|e| equipment == e)
                            }
                        }))
                    && (self.category.is_empty() || self.category.contains(&e.category))
            })
            .map(|e| (&e.name, e))
            .collect()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.name.trim().is_empty()
            && self.muscles.is_empty()
            && self.force.is_empty()
            && self.mechanic.is_empty()
            && self.laterality.is_empty()
            && self.assistance.is_empty()
            && self.equipment.is_empty()
            && self.category.is_empty()
    }

    #[must_use]
    pub fn muscle_list(&self) -> Vec<(MuscleID, bool)> {
        MuscleID::iter_filter()
            .map(|m| (*m, self.muscles.contains(m)))
            .collect::<Vec<_>>()
    }

    #[must_use]
    pub fn force_list(&self) -> Vec<(Force, bool)> {
        Force::iter_filter()
            .map(|f| (*f, self.force.contains(f)))
            .collect::<Vec<_>>()
    }

    #[must_use]
    pub fn mechanic_list(&self) -> Vec<(Mechanic, bool)> {
        Mechanic::iter_filter()
            .map(|m| (*m, self.mechanic.contains(m)))
            .collect::<Vec<_>>()
    }

    #[must_use]
    pub fn laterality_list(&self) -> Vec<(Laterality, bool)> {
        Laterality::iter_filter()
            .map(|l| (*l, self.laterality.contains(l)))
            .collect::<Vec<_>>()
    }

    #[must_use]
    pub fn assistance_list(&self) -> Vec<(Assistance, bool)> {
        Assistance::iter_filter()
            .map(|l| (*l, self.assistance.contains(l)))
            .collect::<Vec<_>>()
    }

    #[must_use]
    pub fn equipment_list(&self) -> Vec<(Equipment, bool)> {
        Equipment::iter_filter()
            .map(|e| (*e, self.equipment.contains(e)))
            .collect::<Vec<_>>()
    }

    #[must_use]
    pub fn category_list(&self) -> Vec<(Category, bool)> {
        Category::iter_filter()
            .map(|c| (*c, self.category.contains(c)))
            .collect::<Vec<_>>()
    }

    pub fn toggle_muscle(&mut self, muscle: MuscleID) {
        if self.muscles.contains(&muscle) {
            self.muscles.remove(&muscle);
        } else {
            if muscle == MuscleID::None {
                self.muscles.clear();
            } else {
                self.muscles.remove(&MuscleID::None);
            }
            self.muscles.insert(muscle);
        }
    }

    pub fn toggle_force(&mut self, force: Force) {
        if self.force.contains(&force) {
            self.force.remove(&force);
        } else {
            self.force.insert(force);
        }
    }

    pub fn toggle_mechanic(&mut self, mechanic: Mechanic) {
        if self.mechanic.contains(&mechanic) {
            self.mechanic.remove(&mechanic);
        } else {
            self.mechanic.insert(mechanic);
        }
    }

    pub fn toggle_laterality(&mut self, laterality: Laterality) {
        if self.laterality.contains(&laterality) {
            self.laterality.remove(&laterality);
        } else {
            self.laterality.insert(laterality);
        }
    }

    pub fn toggle_assistance(&mut self, assistance: Assistance) {
        if self.assistance.contains(&assistance) {
            self.assistance.remove(&assistance);
        } else {
            self.assistance.insert(assistance);
        }
    }

    pub fn toggle_equipment(&mut self, equipment: Equipment) {
        if self.equipment.contains(&equipment) {
            self.equipment.remove(&equipment);
        } else {
            self.equipment.insert(equipment);
        }
    }

    pub fn toggle_category(&mut self, category: Category) {
        if self.category.contains(&category) {
            self.category.remove(&category);
        } else {
            self.category.insert(category);
        }
    }
}

pub trait Property: Clone + Copy + Sized {
    fn iter() -> Iter<'static, Self>;
    fn iter_filter() -> Iter<'static, Self> {
        Self::iter()
    }
    fn name(self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_exercise_muscle_stimulus() {
        assert_eq!(
            Exercise {
                id: 1.into(),
                name: Name::new("A").unwrap(),
                muscles: vec![
                    ExerciseMuscle {
                        muscle_id: MuscleID::Lats,
                        stimulus: Stimulus::PRIMARY,
                    },
                    ExerciseMuscle {
                        muscle_id: MuscleID::Traps,
                        stimulus: Stimulus::SECONDARY,
                    }
                ],
            }
            .muscle_stimulus(),
            BTreeMap::from([
                (MuscleID::Lats, Stimulus::PRIMARY),
                (MuscleID::Traps, Stimulus::SECONDARY)
            ])
        );
    }

    #[rstest]
    #[case(0, Ok(Stimulus::NONE))]
    #[case(50, Ok(Stimulus::SECONDARY))]
    #[case(100, Ok(Stimulus::PRIMARY))]
    #[case(101, Err(StimulusError::OutOfRange(101)))]
    fn test_stimulus_new(#[case] value: u32, #[case] expected: Result<Stimulus, StimulusError>) {
        assert_eq!(Stimulus::new(value), expected);
    }

    #[test]
    fn test_stimulus_add() {
        assert_eq!(
            Stimulus::NONE + Stimulus::SECONDARY + Stimulus::PRIMARY,
            Stimulus(150)
        );
    }

    #[test]
    fn test_muscle_id_iter() {
        assert!(
            !MuscleID::iter()
                .collect::<Vec<_>>()
                .contains(&&MuscleID::None)
        );
    }

    #[test]
    fn test_muscle_id_name() {
        let mut names = HashSet::new();

        for muscle in MuscleID::iter_filter() {
            let name = muscle.name();

            assert!(!name.is_empty());
            assert!(!names.contains(name));

            names.insert(name);
        }
    }

    #[test]
    fn test_muscle_id_description() {
        let mut descriptions = HashSet::new();

        for muscle in MuscleID::iter_filter() {
            let description = muscle.description();

            assert!(description.is_empty() || !descriptions.contains(description));

            descriptions.insert(description);
        }
    }

    #[test]
    fn test_muscle_id_try_from_u8() {
        for muscle_id in MuscleID::iter() {
            assert_eq!(MuscleID::try_from(*muscle_id as u8), Ok(*muscle_id));
        }

        assert_eq!(MuscleID::try_from(0), Err(MuscleIDError::Invalid));
    }

    #[test]
    fn test_force_name() {
        let mut names = HashSet::new();

        for force in Force::iter_filter() {
            let name = force.name();

            assert!(!name.is_empty());
            assert!(!names.contains(name));

            names.insert(name);
        }
    }

    #[test]
    fn test_mechanic_name() {
        let mut names = HashSet::new();

        for mechanic in Mechanic::iter_filter() {
            let name = mechanic.name();

            assert!(!name.is_empty());
            assert!(!names.contains(name));

            names.insert(name);
        }
    }

    #[test]
    fn test_laterality_name() {
        let mut names = HashSet::new();

        for laterality in Laterality::iter_filter() {
            let name = laterality.name();

            assert!(!name.is_empty());
            assert!(!names.contains(name));

            names.insert(name);
        }
    }

    #[test]
    fn test_assistance_name() {
        let mut names = HashSet::new();

        for assistance in Assistance::iter_filter() {
            let name = assistance.name();

            assert!(!name.is_empty());
            assert!(!names.contains(name));

            names.insert(name);
        }
    }

    #[test]
    fn test_equipment_iter() {
        assert!(
            !Equipment::iter()
                .collect::<Vec<_>>()
                .contains(&&Equipment::None)
        );
    }

    #[test]
    fn test_equipment_name() {
        let mut names = HashSet::new();

        for equipment in Equipment::iter_filter() {
            let name = equipment.name();

            assert!(!name.is_empty());
            assert!(!names.contains(name));

            names.insert(name);
        }
    }

    #[test]
    fn test_category_name() {
        let mut names = HashSet::new();

        for category in Category::iter_filter() {
            let name = category.name();

            assert!(!name.is_empty());
            assert!(!names.contains(name));

            names.insert(name);
        }
    }

    #[rstest]
    #[case::name_lower_case(
        ExerciseFilter { name: "push".into(), ..ExerciseFilter::default() },
        &[
            Exercise { id: 0.into(), name: Name::new("Handstand Push Up").unwrap(), muscles: vec![] },
        ],
        &[Exercise { id: 0.into(), name: Name::new("Handstand Push Up").unwrap(), muscles: vec![] }]
    )]
    #[case::name_upper_case(
        ExerciseFilter { name: "PUSH".into(), ..ExerciseFilter::default() },
        &[
            Exercise { id: 0.into(), name: Name::new("Handstand Push Up").unwrap(), muscles: vec![] },
        ],
        &[Exercise { id: 0.into(), name: Name::new("Handstand Push Up").unwrap(), muscles: vec![] }]
    )]
    #[case::no_muscles(
        ExerciseFilter { muscles: [MuscleID::None].into(), ..ExerciseFilter::default() },
        &[
            Exercise { id: 0.into(), name: Name::new("Squat").unwrap(), muscles: vec![] },
            Exercise { id: 1.into(), name: Name::new("Squat").unwrap(), muscles: vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY }] },
        ],
        &[Exercise { id: 0.into(), name: Name::new("Squat").unwrap(), muscles: vec![] }]
    )]
    #[case::muscles(
        ExerciseFilter { muscles: [MuscleID::Pecs, MuscleID::FrontDelts].into(), ..ExerciseFilter::default() },
        &[
            Exercise { id: 0.into(), name: Name::new("Squat").unwrap(), muscles: vec![] },
            Exercise { id: 1.into(), name: Name::new("Squat").unwrap(), muscles: vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY }, ExerciseMuscle { muscle_id: MuscleID::FrontDelts, stimulus: Stimulus::SECONDARY }] },
        ],
        &[Exercise { id: 1.into(), name: Name::new("Squat").unwrap(), muscles: vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY }, ExerciseMuscle { muscle_id: MuscleID::FrontDelts, stimulus: Stimulus::SECONDARY }] }]
    )]
    fn test_exercise_filter_exercises(
        #[case] filter: ExerciseFilter,
        #[case] exercises: &[Exercise],
        #[case] expected: &[Exercise],
    ) {
        assert_eq!(
            filter.exercises(exercises.iter()),
            expected.iter().collect::<Vec<_>>(),
        );
    }

    #[rstest]
    #[case::name_lower_case(
        ExerciseFilter { name: "push".into(), ..ExerciseFilter::default() },
        Some("Decline Push Up")
    )]
    #[case::name_upper_case(
        ExerciseFilter { name: "PUSH".into(), ..ExerciseFilter::default() },
        Some("Decline Push Up")
    )]
    #[case::no_muscles(
        ExerciseFilter { muscles: [MuscleID::None].into(), ..ExerciseFilter::default() },
        None
    )]
    #[case::muscles(
        ExerciseFilter { muscles: [MuscleID::Lats, MuscleID::Traps].into(), ..ExerciseFilter::default() },
        Some("Band Pull Apart")
    )]
    #[case::equipment(
        ExerciseFilter { equipment: [Equipment::Barbell].into(), ..ExerciseFilter::default() },
        Some("Barbell Ab Rollout")
    )]
    #[case::no_equipment(
        ExerciseFilter { equipment: [Equipment::None].into(), ..ExerciseFilter::default() },
        Some("Bench Dip")
    )]
    #[case::equipment(
        ExerciseFilter { equipment: [Equipment::Barbell].into(), ..ExerciseFilter::default() },
        Some("Barbell Ab Rollout")
    )]
    fn test_exercise_catalog(
        #[case] filter: ExerciseFilter,
        #[case] expected_first_name: Option<&str>,
    ) {
        assert_eq!(
            filter.catalog().first_entry().map(|e| (*e.key()).clone()),
            expected_first_name.map(|name| Name::new(name).unwrap()),
        );
    }

    #[test]
    fn test_exercise_filter_is_empty() {
        assert!(ExerciseFilter::default().is_empty());
    }

    #[test]
    fn test_exercise_filter_toggle_muscle() {
        let mut filter = ExerciseFilter::default();

        assert!(filter.muscle_list().iter().map(|(_, b)| b).all(|b| !b));

        filter.toggle_muscle(MuscleID::None);

        assert!(filter.muscle_list().contains(&(MuscleID::None, true)));
        assert!(
            filter
                .muscle_list()
                .into_iter()
                .filter(|(m, _)| *m != MuscleID::None)
                .map(|(_, b)| b)
                .all(|b| !b)
        );

        filter.toggle_muscle(MuscleID::Abs);

        assert!(filter.muscle_list().contains(&(MuscleID::Abs, true)));
        assert!(!filter.muscle_list().contains(&(MuscleID::None, true)));

        filter.toggle_muscle(MuscleID::Abs);

        assert!(filter.muscle_list().iter().map(|(_, b)| b).all(|b| !b));
    }

    #[test]
    fn test_exercise_filter_toggle_force() {
        let mut filter = ExerciseFilter::default();

        assert!(filter.force_list().iter().map(|(_, b)| b).all(|b| !b));

        filter.toggle_force(Force::Push);

        assert!(filter.force_list().contains(&(Force::Push, true)));
        assert!(
            filter
                .force_list()
                .into_iter()
                .filter(|(f, _)| *f != Force::Push)
                .map(|(_, b)| b)
                .all(|b| !b)
        );

        filter.toggle_force(Force::Push);

        assert!(filter.force_list().iter().map(|(_, b)| b).all(|b| !b));
    }

    #[test]
    fn test_exercise_filter_toggle_mechanic() {
        let mut filter = ExerciseFilter::default();

        assert!(filter.mechanic_list().iter().map(|(_, b)| b).all(|b| !b));

        filter.toggle_mechanic(Mechanic::Compound);

        assert!(filter.mechanic_list().contains(&(Mechanic::Compound, true)));
        assert!(
            filter
                .mechanic_list()
                .into_iter()
                .filter(|(m, _)| *m != Mechanic::Compound)
                .map(|(_, b)| b)
                .all(|b| !b)
        );

        filter.toggle_mechanic(Mechanic::Compound);

        assert!(filter.mechanic_list().iter().map(|(_, b)| b).all(|b| !b));
    }

    #[test]
    fn test_exercise_filter_toggle_laterality() {
        let mut filter = ExerciseFilter::default();

        assert!(filter.laterality_list().iter().map(|(_, b)| b).all(|b| !b));

        filter.toggle_laterality(Laterality::Bilateral);

        assert!(
            filter
                .laterality_list()
                .contains(&(Laterality::Bilateral, true))
        );
        assert!(
            filter
                .laterality_list()
                .into_iter()
                .filter(|(l, _)| *l != Laterality::Bilateral)
                .map(|(_, b)| b)
                .all(|b| !b)
        );

        filter.toggle_laterality(Laterality::Bilateral);

        assert!(filter.laterality_list().iter().map(|(_, b)| b).all(|b| !b));
    }

    #[test]
    fn test_exercise_filter_toggle_assistance() {
        let mut filter = ExerciseFilter::default();

        assert!(filter.assistance_list().iter().map(|(_, b)| b).all(|b| !b));

        filter.toggle_assistance(Assistance::Assisted);

        assert!(
            filter
                .assistance_list()
                .contains(&(Assistance::Assisted, true))
        );
        assert!(
            filter
                .assistance_list()
                .into_iter()
                .filter(|(a, _)| *a != Assistance::Assisted)
                .map(|(_, b)| b)
                .all(|b| !b)
        );

        filter.toggle_assistance(Assistance::Assisted);

        assert!(filter.assistance_list().iter().map(|(_, b)| b).all(|b| !b));
    }

    #[test]
    fn test_exercise_filter_toggle_equipment() {
        let mut filter = ExerciseFilter::default();

        assert!(filter.equipment_list().iter().map(|(_, b)| b).all(|b| !b));

        filter.toggle_equipment(Equipment::Barbell);

        assert!(
            filter
                .equipment_list()
                .contains(&(Equipment::Barbell, true))
        );
        assert!(
            filter
                .equipment_list()
                .into_iter()
                .filter(|(e, _)| *e != Equipment::Barbell)
                .map(|(_, b)| b)
                .all(|b| !b)
        );

        filter.toggle_equipment(Equipment::Barbell);

        assert!(filter.equipment_list().iter().map(|(_, b)| b).all(|b| !b));
    }

    #[test]
    fn test_exercise_filter_toggle_category() {
        let mut filter = ExerciseFilter::default();

        assert!(filter.category_list().iter().map(|(_, b)| b).all(|b| !b));

        filter.toggle_category(Category::Strength);

        assert!(filter.category_list().contains(&(Category::Strength, true)));
        assert!(
            filter
                .category_list()
                .into_iter()
                .filter(|(c, _)| *c != Category::Strength)
                .map(|(_, b)| b)
                .all(|b| !b)
        );

        filter.toggle_category(Category::Strength);

        assert!(filter.category_list().iter().map(|(_, b)| b).all(|b| !b));
    }
}
