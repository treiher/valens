use std::{
    collections::{BTreeMap, HashSet},
    ops::{Add, AddAssign, Mul},
    slice::Iter,
    str::FromStr,
};

use derive_more::Deref;
use uuid::Uuid;

use crate::{
    CreateError, DeleteError, Name, ReadError, SyncError, UpdateError, ValidationError, catalog,
};

#[allow(async_fn_in_trait)]
pub trait ExerciseService {
    async fn get_exercises(&self) -> Result<Vec<Exercise>, ReadError>;
    #[allow(clippy::too_many_arguments)]
    async fn create_exercise(
        &self,
        name: Name,
        muscles: Vec<ExerciseMuscle>,
        force: Option<Force>,
        mechanic: Option<Mechanic>,
        laterality: Option<Laterality>,
        assistance: Option<Assistance>,
        equipment: Vec<Equipment>,
        category: Option<Category>,
    ) -> Result<Exercise, CreateError>;
    async fn replace_exercise(&self, exercise: Exercise) -> Result<Exercise, UpdateError>;
    async fn delete_exercise(&self, id: ExerciseID) -> Result<(), DeleteError>;

    async fn validate_exercise_name(
        &self,
        name: &str,
        id: ExerciseID,
    ) -> Result<Name, ValidationError> {
        match Name::new(name) {
            Ok(name) => match self.get_exercises().await {
                Ok(exercises) => {
                    if exercises.iter().all(|e| e.id == id || e.name != name) {
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

    async fn get_exercise(&self, id: ExerciseID) -> Result<Option<Exercise>, ReadError> {
        Ok(self.get_exercises().await?.into_iter().find(|e| e.id == id))
    }
}

#[allow(async_fn_in_trait)]
pub trait ExerciseRepository {
    async fn sync_exercises(&self) -> Result<Vec<Exercise>, SyncError>;
    async fn read_exercises(&self) -> Result<Vec<Exercise>, ReadError>;
    #[allow(clippy::too_many_arguments)]
    async fn create_exercise(
        &self,
        name: Name,
        muscles: Vec<ExerciseMuscle>,
        force: Option<Force>,
        mechanic: Option<Mechanic>,
        laterality: Option<Laterality>,
        assistance: Option<Assistance>,
        equipment: Vec<Equipment>,
        category: Option<Category>,
    ) -> Result<Exercise, CreateError>;
    async fn replace_exercise(&self, exercise: Exercise) -> Result<Exercise, UpdateError>;
    async fn delete_exercise(&self, id: ExerciseID) -> Result<(), DeleteError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exercise {
    pub id: ExerciseID,
    pub name: Name,
    pub muscles: Vec<ExerciseMuscle>,
    pub force: Option<Force>,
    pub mechanic: Option<Mechanic>,
    pub laterality: Option<Laterality>,
    pub assistance: Option<Assistance>,
    pub equipment: Vec<Equipment>,
    pub category: Option<Category>,
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

/// The properties of an exercise, in the order in which they are passed to `create_exercise`.
pub type ExerciseProperties = (
    Option<Force>,
    Option<Mechanic>,
    Option<Laterality>,
    Option<Assistance>,
    Vec<Equipment>,
    Option<Category>,
);

impl From<&catalog::Exercise> for ExerciseProperties {
    fn from(value: &catalog::Exercise) -> Self {
        (
            Some(value.force),
            Some(value.mechanic),
            Some(value.laterality),
            Some(value.assistance),
            value.equipment.to_vec(),
            Some(value.category),
        )
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

impl FromStr for ExerciseID {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(Self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExerciseMuscle {
    pub muscle_id: MuscleID,
    pub stimulus: Stimulus,
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    #[error("stimulus must be 100 or less ({0} > 100)")]
    OutOfRange(u32),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum MuscleID {
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

    fn none_name() -> &'static str {
        "No Muscle"
    }

    fn name(self) -> &'static str {
        match self {
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
        MuscleID::iter()
            .find(|muscle_id| **muscle_id as u8 == value)
            .copied()
            .ok_or(MuscleIDError::Invalid)
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum MuscleIDError {
    #[error("invalid muscle ID")]
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Force {
    Push = 1,
    Pull = 2,
    Static = 3,
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

impl TryFrom<u8> for Force {
    type Error = ForceError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Force::iter()
            .find(|force| **force as u8 == value)
            .copied()
            .ok_or(ForceError::Invalid)
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ForceError {
    #[error("invalid force")]
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Mechanic {
    Compound = 1,
    Isolation = 2,
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

impl TryFrom<u8> for Mechanic {
    type Error = MechanicError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Mechanic::iter()
            .find(|mechanic| **mechanic as u8 == value)
            .copied()
            .ok_or(MechanicError::Invalid)
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum MechanicError {
    #[error("invalid mechanic")]
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Laterality {
    Bilateral = 1,
    Unilateral = 2,
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

impl TryFrom<u8> for Laterality {
    type Error = LateralityError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Laterality::iter()
            .find(|laterality| **laterality as u8 == value)
            .copied()
            .ok_or(LateralityError::Invalid)
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum LateralityError {
    #[error("invalid laterality")]
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Assistance {
    Unassisted = 1,
    Assisted = 2,
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

impl TryFrom<u8> for Assistance {
    type Error = AssistanceError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Assistance::iter()
            .find(|assistance| **assistance as u8 == value)
            .copied()
            .ok_or(AssistanceError::Invalid)
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum AssistanceError {
    #[error("invalid assistance")]
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Equipment {
    Barbell = 1,
    Box = 2,
    Cable = 3,
    Dumbbell = 4,
    ExerciseBall = 5,
    GymnasticRings = 6,
    Kettlebell = 7,
    Machine = 8,
    ParallelBars = 9,
    PullUpBar = 10,
    ResistanceBand = 11,
    Sliders = 12,
    TrapBar = 13,
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

    fn none_name() -> &'static str {
        "No Equipment"
    }

    fn name(self) -> &'static str {
        match self {
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

impl TryFrom<u8> for Equipment {
    type Error = EquipmentError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Equipment::iter()
            .find(|equipment| **equipment as u8 == value)
            .copied()
            .ok_or(EquipmentError::Invalid)
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum EquipmentError {
    #[error("invalid equipment")]
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Category {
    Strength = 1,
    Plyometrics = 2,
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

impl TryFrom<u8> for Category {
    type Error = CategoryError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Category::iter()
            .find(|category| **category as u8 == value)
            .copied()
            .ok_or(CategoryError::Invalid)
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum CategoryError {
    #[error("invalid category")]
    Invalid,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ExerciseFilter {
    pub name: String,
    pub muscles: HashSet<Option<MuscleID>>,
    pub force: HashSet<Option<Force>>,
    pub mechanic: HashSet<Option<Mechanic>>,
    pub laterality: HashSet<Option<Laterality>>,
    pub assistance: HashSet<Option<Assistance>>,
    pub equipment: HashSet<Option<Equipment>>,
    pub category: HashSet<Option<Category>>,
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
                        || self.muscles.iter().all(|m| match m {
                            Some(m) => e.muscle_stimulus().contains_key(m),
                            None => e.muscles.is_empty(),
                        }))
                    && (self.force.is_empty() || self.force.contains(&e.force))
                    && (self.mechanic.is_empty() || self.mechanic.contains(&e.mechanic))
                    && (self.laterality.is_empty() || self.laterality.contains(&e.laterality))
                    && (self.assistance.is_empty() || self.assistance.contains(&e.assistance))
                    && (self.equipment.is_empty()
                        || self.equipment.iter().any(|equipment| match equipment {
                            Some(equipment) => e.equipment.contains(equipment),
                            None => e.equipment.is_empty(),
                        }))
                    && (self.category.is_empty() || self.category.contains(&e.category))
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
                        || self.muscles.iter().all(|muscle| match muscle {
                            Some(muscle) => e.muscles.iter().any(|(m, _)| muscle == m),
                            None => e.muscles.is_empty(),
                        }))
                    && (self.force.is_empty() || self.force.contains(&Some(e.force)))
                    && (self.mechanic.is_empty() || self.mechanic.contains(&Some(e.mechanic)))
                    && (self.laterality.is_empty() || self.laterality.contains(&Some(e.laterality)))
                    && (self.assistance.is_empty() || self.assistance.contains(&Some(e.assistance)))
                    && (self.equipment.is_empty()
                        || self.equipment.iter().any(|equipment| match equipment {
                            Some(equipment) => e.equipment.contains(equipment),
                            None => e.equipment.is_empty(),
                        }))
                    && (self.category.is_empty() || self.category.contains(&Some(e.category)))
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
    pub fn muscle_list(&self) -> Vec<(Option<MuscleID>, bool)> {
        filter_list(&self.muscles)
    }

    #[must_use]
    pub fn force_list(&self) -> Vec<(Option<Force>, bool)> {
        filter_list(&self.force)
    }

    #[must_use]
    pub fn mechanic_list(&self) -> Vec<(Option<Mechanic>, bool)> {
        filter_list(&self.mechanic)
    }

    #[must_use]
    pub fn laterality_list(&self) -> Vec<(Option<Laterality>, bool)> {
        filter_list(&self.laterality)
    }

    #[must_use]
    pub fn assistance_list(&self) -> Vec<(Option<Assistance>, bool)> {
        filter_list(&self.assistance)
    }

    #[must_use]
    pub fn equipment_list(&self) -> Vec<(Option<Equipment>, bool)> {
        filter_list(&self.equipment)
    }

    #[must_use]
    pub fn category_list(&self) -> Vec<(Option<Category>, bool)> {
        filter_list(&self.category)
    }

    pub fn toggle_muscle(&mut self, muscle: Option<MuscleID>) {
        if self.muscles.contains(&muscle) {
            self.muscles.remove(&muscle);
        } else {
            if muscle.is_none() {
                self.muscles.clear();
            } else {
                self.muscles.remove(&None);
            }
            self.muscles.insert(muscle);
        }
    }

    pub fn toggle_force(&mut self, force: Option<Force>) {
        toggle(&mut self.force, force);
    }

    pub fn toggle_mechanic(&mut self, mechanic: Option<Mechanic>) {
        toggle(&mut self.mechanic, mechanic);
    }

    pub fn toggle_laterality(&mut self, laterality: Option<Laterality>) {
        toggle(&mut self.laterality, laterality);
    }

    pub fn toggle_assistance(&mut self, assistance: Option<Assistance>) {
        toggle(&mut self.assistance, assistance);
    }

    pub fn toggle_equipment(&mut self, equipment: Option<Equipment>) {
        toggle(&mut self.equipment, equipment);
    }

    pub fn toggle_category(&mut self, category: Option<Category>) {
        toggle(&mut self.category, category);
    }
}

fn filter_list<T: Property + Eq + std::hash::Hash + 'static>(
    selected: &HashSet<Option<T>>,
) -> Vec<(Option<T>, bool)> {
    T::iter()
        .map(|value| Some(*value))
        .chain([None])
        .map(|value| (value, selected.contains(&value)))
        .collect()
}

fn toggle<T: Eq + std::hash::Hash>(selected: &mut HashSet<T>, value: T) {
    if selected.contains(&value) {
        selected.remove(&value);
    } else {
        selected.insert(value);
    }
}

pub trait Property: Clone + Copy + Sized {
    fn iter() -> Iter<'static, Self>;
    fn name(self) -> &'static str;
    #[must_use]
    fn none_name() -> &'static str {
        "Not Set"
    }
}

#[must_use]
pub fn name_or_none<T: Property>(value: Option<T>) -> &'static str {
    match value {
        Some(value) => value.name(),
        None => T::none_name(),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    fn exercise(id: u128, name: &str, muscles: Vec<ExerciseMuscle>) -> Exercise {
        Exercise {
            id: id.into(),
            name: Name::new(name).unwrap(),
            muscles,
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        }
    }

    #[test]
    fn test_exercise_muscle_stimulus() {
        assert_eq!(
            exercise(
                1,
                "A",
                vec![
                    ExerciseMuscle {
                        muscle_id: MuscleID::Lats,
                        stimulus: Stimulus::PRIMARY,
                    },
                    ExerciseMuscle {
                        muscle_id: MuscleID::Traps,
                        stimulus: Stimulus::SECONDARY,
                    }
                ]
            )
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

    fn assert_distinct_names<T: Property + 'static>() {
        let mut names = HashSet::new();

        for value in T::iter().map(|value| Some(*value)).chain([None]) {
            let name = name_or_none(value);

            assert!(!name.is_empty());
            assert!(names.insert(name));
        }
    }

    #[test]
    fn test_exercise_properties_from_catalog_exercise() {
        assert_eq!(
            ExerciseProperties::from(
                &catalog::EXERCISES[&Name::new("Barbell Bench Press").unwrap()]
            ),
            (
                Some(Force::Push),
                Some(Mechanic::Compound),
                Some(Laterality::Bilateral),
                Some(Assistance::Unassisted),
                vec![Equipment::Barbell],
                Some(Category::Strength),
            )
        );
    }

    #[test]
    fn test_muscle_id_name() {
        assert_distinct_names::<MuscleID>();
    }

    #[test]
    fn test_muscle_id_description() {
        let mut descriptions = HashSet::new();

        for muscle in MuscleID::iter() {
            let description = muscle.description();

            assert!(description.is_empty() || !descriptions.contains(description));

            descriptions.insert(description);
        }
    }

    fn assert_try_from_u8<T>()
    where
        T: Property + TryFrom<u8> + Eq + std::hash::Hash + std::fmt::Debug + 'static,
    {
        let decoded = (0..=u8::MAX)
            .filter_map(|value| T::try_from(value).ok())
            .collect::<Vec<_>>();

        assert_eq!(
            decoded.iter().copied().collect::<HashSet<_>>(),
            T::iter().copied().collect::<HashSet<_>>()
        );
        assert_eq!(decoded.len(), T::iter().count());
        assert!(T::try_from(0).is_err());
    }

    #[test]
    fn test_muscle_id_try_from_u8() {
        assert_try_from_u8::<MuscleID>();
    }

    #[test]
    fn test_force_name() {
        assert_distinct_names::<Force>();
    }

    #[test]
    fn test_force_try_from_u8() {
        assert_try_from_u8::<Force>();
    }

    #[test]
    fn test_mechanic_name() {
        assert_distinct_names::<Mechanic>();
    }

    #[test]
    fn test_mechanic_try_from_u8() {
        assert_try_from_u8::<Mechanic>();
    }

    #[test]
    fn test_laterality_name() {
        assert_distinct_names::<Laterality>();
    }

    #[test]
    fn test_laterality_try_from_u8() {
        assert_try_from_u8::<Laterality>();
    }

    #[test]
    fn test_assistance_name() {
        assert_distinct_names::<Assistance>();
    }

    #[test]
    fn test_assistance_try_from_u8() {
        assert_try_from_u8::<Assistance>();
    }

    #[test]
    fn test_equipment_name() {
        assert_distinct_names::<Equipment>();
    }

    #[test]
    fn test_equipment_try_from_u8() {
        assert_try_from_u8::<Equipment>();
    }

    #[test]
    fn test_category_name() {
        assert_distinct_names::<Category>();
    }

    #[test]
    fn test_category_try_from_u8() {
        assert_try_from_u8::<Category>();
    }

    #[rstest]
    #[case::name_lower_case(
        ExerciseFilter { name: "push".into(), ..ExerciseFilter::default() },
        &[
            exercise(0, "Handstand Push Up", vec![]),
        ],
        &[exercise(0, "Handstand Push Up", vec![])]
    )]
    #[case::name_upper_case(
        ExerciseFilter { name: "PUSH".into(), ..ExerciseFilter::default() },
        &[
            exercise(0, "Handstand Push Up", vec![]),
        ],
        &[exercise(0, "Handstand Push Up", vec![])]
    )]
    #[case::no_muscles(
        ExerciseFilter { muscles: [None].into(), ..ExerciseFilter::default() },
        &[
            exercise(0, "Squat", vec![]),
            exercise(1, "Squat", vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY }]),
        ],
        &[exercise(0, "Squat", vec![])]
    )]
    #[case::muscles(
        ExerciseFilter { muscles: [Some(MuscleID::Pecs), Some(MuscleID::FrontDelts)].into(), ..ExerciseFilter::default() },
        &[
            exercise(0, "Squat", vec![]),
            exercise(1, "Squat", vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY }, ExerciseMuscle { muscle_id: MuscleID::FrontDelts, stimulus: Stimulus::SECONDARY }]),
        ],
        &[exercise(1, "Squat", vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY }, ExerciseMuscle { muscle_id: MuscleID::FrontDelts, stimulus: Stimulus::SECONDARY }])]
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
    #[case::force(ExerciseFilter { force: [Some(Force::Push)].into(), ..ExerciseFilter::default() }, true)]
    #[case::force_not_set(ExerciseFilter { force: [None].into(), ..ExerciseFilter::default() }, false)]
    #[case::mechanic(ExerciseFilter { mechanic: [Some(Mechanic::Compound)].into(), ..ExerciseFilter::default() }, true)]
    #[case::mechanic_not_set(ExerciseFilter { mechanic: [None].into(), ..ExerciseFilter::default() }, false)]
    #[case::laterality(ExerciseFilter { laterality: [Some(Laterality::Bilateral)].into(), ..ExerciseFilter::default() }, true)]
    #[case::laterality_not_set(ExerciseFilter { laterality: [None].into(), ..ExerciseFilter::default() }, false)]
    #[case::assistance(ExerciseFilter { assistance: [Some(Assistance::Assisted)].into(), ..ExerciseFilter::default() }, true)]
    #[case::assistance_not_set(ExerciseFilter { assistance: [None].into(), ..ExerciseFilter::default() }, false)]
    #[case::equipment(ExerciseFilter { equipment: [Some(Equipment::Barbell)].into(), ..ExerciseFilter::default() }, true)]
    #[case::equipment_not_set(ExerciseFilter { equipment: [None].into(), ..ExerciseFilter::default() }, false)]
    #[case::category(ExerciseFilter { category: [Some(Category::Strength)].into(), ..ExerciseFilter::default() }, true)]
    #[case::category_not_set(ExerciseFilter { category: [None].into(), ..ExerciseFilter::default() }, false)]
    fn test_exercise_filter_exercises_by_property(
        #[case] filter: ExerciseFilter,
        #[case] matches_exercise_with_properties: bool,
    ) {
        let without_properties = exercise(0, "A", vec![]);
        let with_properties = Exercise {
            force: Some(Force::Push),
            mechanic: Some(Mechanic::Compound),
            laterality: Some(Laterality::Bilateral),
            assistance: Some(Assistance::Assisted),
            equipment: vec![Equipment::Barbell],
            category: Some(Category::Strength),
            ..exercise(1, "B", vec![])
        };
        let exercises = [without_properties.clone(), with_properties.clone()];

        assert_eq!(
            filter.exercises(exercises.iter()),
            vec![if matches_exercise_with_properties {
                &with_properties
            } else {
                &without_properties
            }],
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
        ExerciseFilter { muscles: [None].into(), ..ExerciseFilter::default() },
        None
    )]
    #[case::muscles(
        ExerciseFilter { muscles: [Some(MuscleID::Lats), Some(MuscleID::Traps)].into(), ..ExerciseFilter::default() },
        Some("Band Pull Apart")
    )]
    #[case::equipment(
        ExerciseFilter { equipment: [Some(Equipment::Barbell)].into(), ..ExerciseFilter::default() },
        Some("Barbell Ab Rollout")
    )]
    #[case::no_equipment(
        ExerciseFilter { equipment: [None].into(), ..ExerciseFilter::default() },
        Some("Bench Dip")
    )]
    #[case::equipment(
        ExerciseFilter { equipment: [Some(Equipment::Barbell)].into(), ..ExerciseFilter::default() },
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

        filter.toggle_muscle(None);

        assert!(filter.muscle_list().contains(&(None, true)));
        assert!(
            filter
                .muscle_list()
                .into_iter()
                .filter(|(m, _)| m.is_some())
                .map(|(_, b)| b)
                .all(|b| !b)
        );

        filter.toggle_muscle(Some(MuscleID::Abs));

        assert!(filter.muscle_list().contains(&(Some(MuscleID::Abs), true)));
        assert!(!filter.muscle_list().contains(&(None, true)));

        filter.toggle_muscle(Some(MuscleID::Abs));

        assert!(filter.muscle_list().iter().map(|(_, b)| b).all(|b| !b));
    }

    #[test]
    fn test_exercise_filter_toggle_force() {
        let mut filter = ExerciseFilter::default();

        assert!(filter.force_list().iter().map(|(_, b)| b).all(|b| !b));

        filter.toggle_force(Some(Force::Push));

        assert!(filter.force_list().contains(&(Some(Force::Push), true)));
        assert!(
            filter
                .force_list()
                .into_iter()
                .filter(|(f, _)| *f != Some(Force::Push))
                .map(|(_, b)| b)
                .all(|b| !b)
        );

        filter.toggle_force(Some(Force::Push));

        assert!(filter.force_list().iter().map(|(_, b)| b).all(|b| !b));
    }

    #[test]
    fn test_exercise_filter_toggle_mechanic() {
        let mut filter = ExerciseFilter::default();

        assert!(filter.mechanic_list().iter().map(|(_, b)| b).all(|b| !b));

        filter.toggle_mechanic(Some(Mechanic::Compound));

        assert!(
            filter
                .mechanic_list()
                .contains(&(Some(Mechanic::Compound), true))
        );
        assert!(
            filter
                .mechanic_list()
                .into_iter()
                .filter(|(m, _)| *m != Some(Mechanic::Compound))
                .map(|(_, b)| b)
                .all(|b| !b)
        );

        filter.toggle_mechanic(Some(Mechanic::Compound));

        assert!(filter.mechanic_list().iter().map(|(_, b)| b).all(|b| !b));
    }

    #[test]
    fn test_exercise_filter_toggle_laterality() {
        let mut filter = ExerciseFilter::default();

        assert!(filter.laterality_list().iter().map(|(_, b)| b).all(|b| !b));

        filter.toggle_laterality(Some(Laterality::Bilateral));

        assert!(
            filter
                .laterality_list()
                .contains(&(Some(Laterality::Bilateral), true))
        );
        assert!(
            filter
                .laterality_list()
                .into_iter()
                .filter(|(l, _)| *l != Some(Laterality::Bilateral))
                .map(|(_, b)| b)
                .all(|b| !b)
        );

        filter.toggle_laterality(Some(Laterality::Bilateral));

        assert!(filter.laterality_list().iter().map(|(_, b)| b).all(|b| !b));
    }

    #[test]
    fn test_exercise_filter_toggle_assistance() {
        let mut filter = ExerciseFilter::default();

        assert!(filter.assistance_list().iter().map(|(_, b)| b).all(|b| !b));

        filter.toggle_assistance(Some(Assistance::Assisted));

        assert!(
            filter
                .assistance_list()
                .contains(&(Some(Assistance::Assisted), true))
        );
        assert!(
            filter
                .assistance_list()
                .into_iter()
                .filter(|(a, _)| *a != Some(Assistance::Assisted))
                .map(|(_, b)| b)
                .all(|b| !b)
        );

        filter.toggle_assistance(Some(Assistance::Assisted));

        assert!(filter.assistance_list().iter().map(|(_, b)| b).all(|b| !b));
    }

    #[test]
    fn test_exercise_filter_toggle_equipment() {
        let mut filter = ExerciseFilter::default();

        assert!(filter.equipment_list().iter().map(|(_, b)| b).all(|b| !b));

        filter.toggle_equipment(Some(Equipment::Barbell));

        assert!(
            filter
                .equipment_list()
                .contains(&(Some(Equipment::Barbell), true))
        );
        assert!(
            filter
                .equipment_list()
                .into_iter()
                .filter(|(e, _)| *e != Some(Equipment::Barbell))
                .map(|(_, b)| b)
                .all(|b| !b)
        );

        filter.toggle_equipment(Some(Equipment::Barbell));

        assert!(filter.equipment_list().iter().map(|(_, b)| b).all(|b| !b));
    }

    #[test]
    fn test_exercise_filter_toggle_category() {
        let mut filter = ExerciseFilter::default();

        assert!(filter.category_list().iter().map(|(_, b)| b).all(|b| !b));

        filter.toggle_category(Some(Category::Strength));

        assert!(
            filter
                .category_list()
                .contains(&(Some(Category::Strength), true))
        );
        assert!(
            filter
                .category_list()
                .into_iter()
                .filter(|(c, _)| *c != Some(Category::Strength))
                .map(|(_, b)| b)
                .all(|b| !b)
        );

        filter.toggle_category(Some(Category::Strength));

        assert!(filter.category_list().iter().map(|(_, b)| b).all(|b| !b));
    }
}
