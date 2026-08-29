use std::{
    collections::{BTreeMap, HashSet},
    ops::{Add, AddAssign, Mul},
    slice::Iter,
    str::FromStr,
    sync::LazyLock,
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

/// A property of an exercise.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExerciseProperty {
    Muscles,
    Force,
    Mechanic,
    Laterality,
    Assistance,
    Category,
    Equipment,
}

impl Property for ExerciseProperty {
    fn iter() -> Iter<'static, ExerciseProperty> {
        static PROPERTIES: [ExerciseProperty; 7] = [
            ExerciseProperty::Muscles,
            ExerciseProperty::Force,
            ExerciseProperty::Mechanic,
            ExerciseProperty::Laterality,
            ExerciseProperty::Assistance,
            ExerciseProperty::Category,
            ExerciseProperty::Equipment,
        ];
        PROPERTIES.iter()
    }

    fn name(self) -> &'static str {
        match self {
            ExerciseProperty::Muscles => "Muscles",
            ExerciseProperty::Force => "Force",
            ExerciseProperty::Mechanic => "Mechanic",
            ExerciseProperty::Laterality => "Laterality",
            ExerciseProperty::Assistance => "Assistance",
            ExerciseProperty::Category => "Category",
            ExerciseProperty::Equipment => "Equipment",
        }
    }
}

/// The properties of an exercise apart from its name.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExerciseProperties {
    pub muscles: Vec<ExerciseMuscle>,
    pub force: Option<Force>,
    pub mechanic: Option<Mechanic>,
    pub laterality: Option<Laterality>,
    pub assistance: Option<Assistance>,
    pub equipment: Vec<Equipment>,
    pub category: Option<Category>,
}

impl From<&Exercise> for ExerciseProperties {
    fn from(value: &Exercise) -> Self {
        ExerciseProperties {
            muscles: value.muscles.clone(),
            force: value.force,
            mechanic: value.mechanic,
            laterality: value.laterality,
            assistance: value.assistance,
            equipment: value.equipment.clone(),
            category: value.category,
        }
    }
}

impl From<&catalog::Exercise> for ExerciseProperties {
    fn from(value: &catalog::Exercise) -> Self {
        ExerciseProperties {
            muscles: value
                .muscles
                .iter()
                .map(|(muscle_id, stimulus)| ExerciseMuscle {
                    muscle_id: *muscle_id,
                    stimulus: *stimulus,
                })
                .collect(),
            force: Some(value.force),
            mechanic: Some(value.mechanic),
            laterality: Some(value.laterality),
            assistance: Some(value.assistance),
            equipment: value.equipment.to_vec(),
            category: Some(value.category),
        }
    }
}

/// How the name of an exercise matched the name of a catalog exercise.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogMatch {
    Exact,
    Prefix,
}

/// Which values of an exercise an update from the catalog writes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogUpdateMode {
    /// Set only values that are unset or empty.
    FillMissing,
    /// Set all values, including clearing values the catalog exercise does not have.
    ReplaceAll,
}

/// A single value of a property, or the absence of any value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyValue {
    pub name: &'static str,
    pub stimulus: Option<Stimulus>,
}

/// The values of a property before and after an update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyChange {
    pub property: ExerciseProperty,
    pub before: Vec<PropertyValue>,
    pub after: Vec<PropertyValue>,
}

/// An exercise updated from the catalog exercise its name matched.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogUpdate {
    pub exercise: Exercise,
    pub catalog_name: &'static Name,
    pub catalog_match: CatalogMatch,
    pub changes: Vec<PropertyChange>,
}

/// Determine the updates from the catalog for all given exercises, sorted by exercise name.
#[must_use]
pub fn catalog_updates(exercises: &[Exercise], mode: CatalogUpdateMode) -> Vec<CatalogUpdate> {
    let mut updates = exercises
        .iter()
        .filter_map(|exercise| catalog_update(exercise, mode))
        .collect::<Vec<_>>();
    updates.sort_by(|a, b| a.exercise.name.cmp(&b.exercise.name));
    updates
}

/// Determine the update from the catalog for an exercise.
///
/// Returns `None` if the name of the exercise matches no catalog exercise or if the update would
/// change nothing.
#[must_use]
pub fn catalog_update(exercise: &Exercise, mode: CatalogUpdateMode) -> Option<CatalogUpdate> {
    let (catalog_name, catalog_exercise, catalog_match) = match_catalog(&exercise.name)?;
    let updated_exercise = updated_exercise(exercise, catalog_exercise, mode);
    let changes = changes(exercise, &updated_exercise);

    if changes.is_empty() {
        return None;
    }

    Some(CatalogUpdate {
        exercise: updated_exercise,
        catalog_name,
        catalog_match,
        changes,
    })
}

/// Find the catalog exercise a name matches, preferring the exact name over the longest name the
/// name starts with, followed by a space.
fn match_catalog(name: &Name) -> Option<(&'static Name, &'static catalog::Exercise, CatalogMatch)> {
    let name = name.as_ref().to_lowercase();
    let mut prefix_match: Option<(usize, &'static Name, &'static catalog::Exercise)> = None;

    for (candidate, catalog_name, catalog_exercise) in CATALOG_NAMES.iter() {
        if name == *candidate {
            return Some((catalog_name, catalog_exercise, CatalogMatch::Exact));
        }

        if name
            .strip_prefix(candidate.as_str())
            .is_some_and(|rest| rest.starts_with(' '))
            && prefix_match.is_none_or(|(len, _, _)| len < candidate.len())
        {
            prefix_match = Some((candidate.len(), catalog_name, catalog_exercise));
        }
    }

    prefix_match.map(|(_, name, exercise)| (name, exercise, CatalogMatch::Prefix))
}

/// Catalog names in lowercase, paired with the entry they belong to.
static CATALOG_NAMES: LazyLock<Vec<(String, &'static Name, &'static catalog::Exercise)>> =
    LazyLock::new(|| {
        let exercises: &'static BTreeMap<Name, catalog::Exercise> = &catalog::EXERCISES;
        exercises
            .iter()
            .map(|(name, exercise)| (name.as_ref().to_lowercase(), name, exercise))
            .collect()
    });

fn updated_exercise(
    exercise: &Exercise,
    catalog_exercise: &catalog::Exercise,
    mode: CatalogUpdateMode,
) -> Exercise {
    let ExerciseProperties {
        muscles,
        force,
        mechanic,
        laterality,
        assistance,
        equipment,
        category,
    } = ExerciseProperties::from(catalog_exercise);

    match mode {
        CatalogUpdateMode::FillMissing => Exercise {
            muscles: if exercise.muscles.is_empty() {
                muscles
            } else {
                exercise.muscles.clone()
            },
            force: exercise.force.or(force),
            mechanic: exercise.mechanic.or(mechanic),
            laterality: exercise.laterality.or(laterality),
            assistance: exercise.assistance.or(assistance),
            equipment: if exercise.equipment.is_empty() {
                equipment
            } else {
                exercise.equipment.clone()
            },
            category: exercise.category.or(category),
            ..exercise.clone()
        },
        CatalogUpdateMode::ReplaceAll => Exercise {
            muscles,
            force,
            mechanic,
            laterality,
            assistance,
            equipment,
            category,
            ..exercise.clone()
        },
    }
}

fn changes(before: &Exercise, after: &Exercise) -> Vec<PropertyChange> {
    ExerciseProperty::iter()
        .filter_map(|property| {
            change(
                *property,
                property_values(before, *property),
                property_values(after, *property),
            )
        })
        .collect()
}

fn change(
    property: ExerciseProperty,
    before: Vec<PropertyValue>,
    after: Vec<PropertyValue>,
) -> Option<PropertyChange> {
    if before == after {
        None
    } else {
        Some(PropertyChange {
            property,
            before,
            after,
        })
    }
}

fn property_values(exercise: &Exercise, property: ExerciseProperty) -> Vec<PropertyValue> {
    match property {
        ExerciseProperty::Muscles => muscle_values(&exercise.muscles),
        ExerciseProperty::Force => scalar_values(exercise.force),
        ExerciseProperty::Mechanic => scalar_values(exercise.mechanic),
        ExerciseProperty::Laterality => scalar_values(exercise.laterality),
        ExerciseProperty::Assistance => scalar_values(exercise.assistance),
        ExerciseProperty::Category => scalar_values(exercise.category),
        ExerciseProperty::Equipment => equipment_values(&exercise.equipment),
    }
}

fn scalar_values<T: Property>(value: Option<T>) -> Vec<PropertyValue> {
    vec![PropertyValue {
        name: name_or_none(value),
        stimulus: None,
    }]
}

/// Determine the values of a set of equipment, ordered by `Equipment::iter`.
fn equipment_values(equipment: &[Equipment]) -> Vec<PropertyValue> {
    values(
        Equipment::iter()
            .filter(|value| equipment.contains(value))
            .map(|value| PropertyValue {
                name: value.name(),
                stimulus: None,
            })
            .collect(),
        Equipment::none_name(),
    )
}

/// Determine the values of a set of muscles, ordered by `MuscleID::iter`.
fn muscle_values(muscles: &[ExerciseMuscle]) -> Vec<PropertyValue> {
    values(
        MuscleID::iter()
            .filter_map(|muscle_id| muscles.iter().find(|m| m.muscle_id == *muscle_id))
            .map(|muscle| PropertyValue {
                name: muscle.muscle_id.name(),
                stimulus: Some(muscle.stimulus),
            })
            .collect(),
        MuscleID::none_name(),
    )
}

fn values(values: Vec<PropertyValue>, none_name: &'static str) -> Vec<PropertyValue> {
    if values.is_empty() {
        vec![PropertyValue {
            name: none_name,
            stimulus: None,
        }]
    } else {
        values
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
pub enum StimulusLevel {
    Secondary,
    Primary,
}

impl StimulusLevel {
    #[must_use]
    pub fn from_stimulus(stimulus: Stimulus) -> Option<Self> {
        if stimulus == Stimulus::NONE {
            None
        } else if stimulus >= Stimulus::PRIMARY {
            Some(StimulusLevel::Primary)
        } else {
            Some(StimulusLevel::Secondary)
        }
    }

    #[must_use]
    pub fn stimulus(self) -> Stimulus {
        match self {
            StimulusLevel::Secondary => Stimulus::SECONDARY,
            StimulusLevel::Primary => Stimulus::PRIMARY,
        }
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            StimulusLevel::Secondary => "Secondary",
            StimulusLevel::Primary => "Primary",
        }
    }
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
    pub muscles: HashSet<Option<(MuscleID, StimulusLevel)>>,
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
                            Some((muscle, level)) => e.muscles.iter().any(|em| {
                                em.muscle_id == *muscle
                                    && StimulusLevel::from_stimulus(em.stimulus) == Some(*level)
                            }),
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
                        || self.muscles.iter().all(|m| match m {
                            Some((muscle, level)) => e.muscles.iter().any(|(m, stimulus)| {
                                m == muscle
                                    && StimulusLevel::from_stimulus(*stimulus) == Some(*level)
                            }),
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

    /// The property values an exercise needs to have to match the filter.
    ///
    /// A property with several selected values yields the first value in the order of the property.
    /// Equipment yields a single value, which suffices because an exercise matches the filter if it
    /// has any of the selected equipment.
    #[must_use]
    pub fn exercise_properties(&self) -> ExerciseProperties {
        let mut muscles = self
            .muscles
            .iter()
            .filter_map(|m| {
                m.map(|(muscle_id, level)| ExerciseMuscle {
                    muscle_id,
                    stimulus: level.stimulus(),
                })
            })
            .collect::<Vec<_>>();
        muscles.sort_by_key(|m| m.muscle_id);

        ExerciseProperties {
            muscles,
            force: first_selected(&self.force),
            mechanic: first_selected(&self.mechanic),
            laterality: first_selected(&self.laterality),
            assistance: first_selected(&self.assistance),
            equipment: first_selected(&self.equipment).into_iter().collect(),
            category: first_selected(&self.category),
        }
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
    pub fn muscle_list(&self) -> Vec<(MuscleID, Option<StimulusLevel>)> {
        MuscleID::iter()
            .map(|muscle| (*muscle, self.muscle_level(*muscle)))
            .collect()
    }

    #[must_use]
    pub fn muscles_not_set(&self) -> bool {
        self.muscles.contains(&None)
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

    /// Cycle a muscle through no level, primary and secondary, or toggle "Not Set".
    ///
    /// "Not Set" and the muscles are mutually exclusive.
    pub fn toggle_muscle(&mut self, muscle: Option<MuscleID>) {
        let Some(muscle) = muscle else {
            if !self.muscles.remove(&None) {
                self.muscles.clear();
                self.muscles.insert(None);
            }
            return;
        };
        let level = match self.muscle_level(muscle) {
            None => Some(StimulusLevel::Primary),
            Some(StimulusLevel::Primary) => Some(StimulusLevel::Secondary),
            Some(StimulusLevel::Secondary) => None,
        };
        self.clear_muscle(Some(muscle));
        if let Some(level) = level {
            self.clear_muscle(None);
            self.muscles.insert(Some((muscle, level)));
        }
    }

    /// Remove a muscle or "Not Set" from the selection.
    pub fn clear_muscle(&mut self, muscle: Option<MuscleID>) {
        match muscle {
            Some(muscle) => self
                .muscles
                .retain(|m| !matches!(m, Some((m, _)) if *m == muscle)),
            None => {
                self.muscles.remove(&None);
            }
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

    fn muscle_level(&self, muscle: MuscleID) -> Option<StimulusLevel> {
        self.muscles.iter().find_map(|m| match m {
            Some((m, level)) if *m == muscle => Some(*level),
            _ => None,
        })
    }
}

fn first_selected<T: Property + Eq + std::hash::Hash + 'static>(
    selected: &HashSet<Option<T>>,
) -> Option<T> {
    T::iter()
        .find(|value| selected.contains(&Some(**value)))
        .copied()
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

    #[rstest]
    #[case(0, None)]
    #[case(1, Some(StimulusLevel::Secondary))]
    #[case(99, Some(StimulusLevel::Secondary))]
    #[case(100, Some(StimulusLevel::Primary))]
    fn test_stimulus_level_from_stimulus(
        #[case] value: u32,
        #[case] expected: Option<StimulusLevel>,
    ) {
        assert_eq!(
            StimulusLevel::from_stimulus(Stimulus::new(value).unwrap()),
            expected
        );
    }

    #[rstest]
    #[case(StimulusLevel::Secondary, Stimulus::SECONDARY)]
    #[case(StimulusLevel::Primary, Stimulus::PRIMARY)]
    fn test_stimulus_level_stimulus(#[case] level: StimulusLevel, #[case] expected: Stimulus) {
        assert_eq!(level.stimulus(), expected);
        assert_eq!(StimulusLevel::from_stimulus(level.stimulus()), Some(level));
    }

    #[test]
    fn test_stimulus_level_name() {
        assert_ne!(
            StimulusLevel::Primary.name(),
            StimulusLevel::Secondary.name()
        );
    }

    #[test]
    fn test_stimulus_add() {
        assert_eq!(
            Stimulus::NONE + Stimulus::SECONDARY + Stimulus::PRIMARY,
            Stimulus(150)
        );
    }

    #[test]
    fn test_exercise_properties_from_exercise() {
        let exercise = catalog_exercise("Barbell Bench Press", 1);

        assert_eq!(
            ExerciseProperties::from(&exercise),
            ExerciseProperties::from(
                &catalog::EXERCISES[&Name::new("Barbell Bench Press").unwrap()]
            )
        );
    }

    #[test]
    fn test_exercise_properties_from_catalog_exercise() {
        let catalog_exercise = &catalog::EXERCISES[&Name::new("Barbell Bench Press").unwrap()];

        assert_eq!(
            ExerciseProperties::from(catalog_exercise),
            ExerciseProperties {
                muscles: catalog_exercise
                    .muscles
                    .iter()
                    .map(|(muscle_id, stimulus)| ExerciseMuscle {
                        muscle_id: *muscle_id,
                        stimulus: *stimulus,
                    })
                    .collect(),
                force: Some(Force::Push),
                mechanic: Some(Mechanic::Compound),
                laterality: Some(Laterality::Bilateral),
                assistance: Some(Assistance::Unassisted),
                equipment: vec![Equipment::Barbell],
                category: Some(Category::Strength),
            }
        );
    }

    #[rstest]
    #[case("Dip", Some(("Dip", CatalogMatch::Exact)))]
    #[case("dIP", Some(("Dip", CatalogMatch::Exact)))]
    #[case("Dip (weighted)", Some(("Dip", CatalogMatch::Prefix)))]
    #[case("Dipping", None)]
    #[case("Squat Jump", Some(("Squat Jump", CatalogMatch::Exact)))]
    #[case("Squat Jump (weighted)", Some(("Squat Jump", CatalogMatch::Prefix)))]
    #[case("Squat Twist", Some(("Squat", CatalogMatch::Prefix)))]
    #[case("Sit Up", None)]
    fn test_catalog_update_match(
        #[case] name: &str,
        #[case] expected: Option<(&str, CatalogMatch)>,
    ) {
        let update = catalog_update(&exercise(1, name, vec![]), CatalogUpdateMode::FillMissing);

        assert_eq!(
            update
                .as_ref()
                .map(|update| (update.catalog_name.as_ref().as_str(), update.catalog_match)),
            expected
        );
    }

    #[rstest]
    #[case(CatalogUpdateMode::FillMissing)]
    #[case(CatalogUpdateMode::ReplaceAll)]
    fn test_catalog_update_of_exercise_without_properties(#[case] mode: CatalogUpdateMode) {
        let update = catalog_update(&exercise(1, "Dip", vec![]), mode).unwrap();

        assert_eq!(update.exercise, catalog_exercise("Dip", 1));
        assert_eq!(
            update
                .changes
                .iter()
                .map(|change| change.property)
                .collect::<Vec<_>>(),
            vec![
                ExerciseProperty::Muscles,
                ExerciseProperty::Force,
                ExerciseProperty::Mechanic,
                ExerciseProperty::Laterality,
                ExerciseProperty::Assistance,
                ExerciseProperty::Category,
                ExerciseProperty::Equipment,
            ]
        );
        assert_eq!(
            update.changes[1],
            PropertyChange {
                property: ExerciseProperty::Force,
                before: vec![value(Force::none_name())],
                after: vec![value(Force::Push.name())],
            }
        );
        assert_eq!(
            update.changes[0],
            PropertyChange {
                property: ExerciseProperty::Muscles,
                before: vec![value(MuscleID::none_name())],
                after: vec![
                    muscle_value(MuscleID::Pecs, Stimulus::PRIMARY),
                    muscle_value(MuscleID::FrontDelts, Stimulus::PRIMARY),
                    muscle_value(MuscleID::Triceps, Stimulus::PRIMARY),
                ],
            }
        );
    }

    #[test]
    fn test_catalog_update_fill_missing_keeps_existing_values() {
        let exercise = Exercise {
            force: Some(Force::Pull),
            equipment: vec![Equipment::Dumbbell],
            ..exercise(
                1,
                "Dip",
                vec![ExerciseMuscle {
                    muscle_id: MuscleID::Biceps,
                    stimulus: Stimulus::PRIMARY,
                }],
            )
        };

        let update = catalog_update(&exercise, CatalogUpdateMode::FillMissing).unwrap();

        assert_eq!(
            update.exercise,
            Exercise {
                mechanic: Some(Mechanic::Compound),
                laterality: Some(Laterality::Bilateral),
                assistance: Some(Assistance::Unassisted),
                category: Some(Category::Strength),
                ..exercise
            }
        );
        assert_eq!(
            update
                .changes
                .iter()
                .map(|change| change.property)
                .collect::<Vec<_>>(),
            vec![
                ExerciseProperty::Mechanic,
                ExerciseProperty::Laterality,
                ExerciseProperty::Assistance,
                ExerciseProperty::Category,
            ]
        );
    }

    #[test]
    fn test_catalog_update_replace_all_clears_values_absent_from_catalog() {
        let exercise = Exercise {
            equipment: vec![Equipment::Barbell],
            ..catalog_exercise("Squat", 1)
        };

        let update = catalog_update(&exercise, CatalogUpdateMode::ReplaceAll).unwrap();

        assert_eq!(update.exercise, catalog_exercise("Squat", 1));
        assert_eq!(
            update.changes,
            vec![PropertyChange {
                property: ExerciseProperty::Equipment,
                before: vec![value(Equipment::Barbell.name())],
                after: vec![value(Equipment::none_name())],
            }]
        );
    }

    #[test]
    fn test_catalog_update_ignores_order_of_equipment() {
        let exercise = Exercise {
            equipment: vec![Equipment::PullUpBar, Equipment::ResistanceBand],
            ..catalog_exercise("Band-Assisted Dip", 1)
        };

        assert_eq!(
            catalog_update(&exercise, CatalogUpdateMode::ReplaceAll),
            None
        );
    }

    #[test]
    fn test_catalog_update_detects_changed_stimulus() {
        let exercise = Exercise {
            muscles: vec![
                ExerciseMuscle {
                    muscle_id: MuscleID::Pecs,
                    stimulus: Stimulus::SECONDARY,
                },
                ExerciseMuscle {
                    muscle_id: MuscleID::FrontDelts,
                    stimulus: Stimulus::PRIMARY,
                },
                ExerciseMuscle {
                    muscle_id: MuscleID::Triceps,
                    stimulus: Stimulus::PRIMARY,
                },
            ],
            ..catalog_exercise("Dip", 1)
        };

        let update = catalog_update(&exercise, CatalogUpdateMode::ReplaceAll).unwrap();

        assert_eq!(update.exercise, catalog_exercise("Dip", 1));
        assert_eq!(
            update.changes,
            vec![PropertyChange {
                property: ExerciseProperty::Muscles,
                before: vec![
                    muscle_value(MuscleID::Pecs, Stimulus::SECONDARY),
                    muscle_value(MuscleID::FrontDelts, Stimulus::PRIMARY),
                    muscle_value(MuscleID::Triceps, Stimulus::PRIMARY),
                ],
                after: vec![
                    muscle_value(MuscleID::Pecs, Stimulus::PRIMARY),
                    muscle_value(MuscleID::FrontDelts, Stimulus::PRIMARY),
                    muscle_value(MuscleID::Triceps, Stimulus::PRIMARY),
                ],
            }]
        );
    }

    #[rstest]
    #[case(CatalogUpdateMode::FillMissing)]
    #[case(CatalogUpdateMode::ReplaceAll)]
    fn test_catalog_update_of_unchanged_exercise(#[case] mode: CatalogUpdateMode) {
        assert_eq!(catalog_update(&catalog_exercise("Dip", 1), mode), None);
    }

    #[test]
    fn test_catalog_updates_sorted_by_name() {
        let updates = catalog_updates(
            &[
                exercise(1, "Squat", vec![]),
                catalog_exercise("Dip", 2),
                exercise(3, "Lunge", vec![]),
            ],
            CatalogUpdateMode::FillMissing,
        );

        assert_eq!(
            updates
                .iter()
                .map(|update| update.exercise.name.as_ref().as_str())
                .collect::<Vec<_>>(),
            vec!["Lunge", "Squat"]
        );
    }

    #[test]
    fn test_exercise_property_name() {
        assert_distinct_value_names::<ExerciseProperty>();
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
        ExerciseFilter { muscles: [Some((MuscleID::Pecs, StimulusLevel::Primary)), Some((MuscleID::FrontDelts, StimulusLevel::Secondary))].into(), ..ExerciseFilter::default() },
        &[
            exercise(0, "Squat", vec![]),
            exercise(1, "Squat", vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY }, ExerciseMuscle { muscle_id: MuscleID::FrontDelts, stimulus: Stimulus::SECONDARY }]),
        ],
        &[exercise(1, "Squat", vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY }, ExerciseMuscle { muscle_id: MuscleID::FrontDelts, stimulus: Stimulus::SECONDARY }])]
    )]
    #[case::muscle_at_other_level(
        ExerciseFilter { muscles: [Some((MuscleID::Pecs, StimulusLevel::Secondary))].into(), ..ExerciseFilter::default() },
        &[
            exercise(0, "Squat", vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY }]),
            exercise(1, "Squat", vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::SECONDARY }]),
        ],
        &[exercise(1, "Squat", vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::SECONDARY }])]
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
        ExerciseFilter { muscles: [Some((MuscleID::Lats, StimulusLevel::Secondary)), Some((MuscleID::Traps, StimulusLevel::Secondary))].into(), ..ExerciseFilter::default() },
        Some("Band Pull Apart")
    )]
    #[case::muscles_at_different_levels(
        ExerciseFilter { muscles: [Some((MuscleID::RearDelts, StimulusLevel::Primary)), Some((MuscleID::Lats, StimulusLevel::Secondary))].into(), ..ExerciseFilter::default() },
        Some("Band Pull Apart")
    )]
    #[case::muscle_at_other_level(
        ExerciseFilter { muscles: [Some((MuscleID::Lats, StimulusLevel::Primary)), Some((MuscleID::Traps, StimulusLevel::Secondary))].into(), ..ExerciseFilter::default() },
        Some("Band-Assisted Pull Up")
    )]
    #[case::muscle_at_both_levels(
        ExerciseFilter { muscles: [Some((MuscleID::Lats, StimulusLevel::Primary)), Some((MuscleID::Lats, StimulusLevel::Secondary))].into(), ..ExerciseFilter::default() },
        None
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

    #[rstest]
    #[case::empty(ExerciseFilter::default(), ExerciseProperties::default())]
    #[case::not_set(
        ExerciseFilter {
            muscles: [None].into(),
            force: [None].into(),
            equipment: [None].into(),
            ..ExerciseFilter::default()
        },
        ExerciseProperties::default()
    )]
    #[case::muscles(
        ExerciseFilter {
            muscles: [
                Some((MuscleID::Pecs, StimulusLevel::Primary)),
                Some((MuscleID::Triceps, StimulusLevel::Secondary)),
            ].into(),
            ..ExerciseFilter::default()
        },
        ExerciseProperties {
            muscles: vec![
                ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY },
                ExerciseMuscle { muscle_id: MuscleID::Triceps, stimulus: Stimulus::SECONDARY },
            ],
            ..ExerciseProperties::default()
        }
    )]
    #[case::properties(
        ExerciseFilter {
            force: [Some(Force::Push)].into(),
            mechanic: [Some(Mechanic::Compound)].into(),
            laterality: [Some(Laterality::Bilateral)].into(),
            assistance: [Some(Assistance::Unassisted)].into(),
            equipment: [Some(Equipment::Barbell)].into(),
            category: [Some(Category::Strength)].into(),
            ..ExerciseFilter::default()
        },
        ExerciseProperties {
            force: Some(Force::Push),
            mechanic: Some(Mechanic::Compound),
            laterality: Some(Laterality::Bilateral),
            assistance: Some(Assistance::Unassisted),
            equipment: vec![Equipment::Barbell],
            category: Some(Category::Strength),
            ..ExerciseProperties::default()
        }
    )]
    #[case::several_values_per_property(
        ExerciseFilter {
            force: [Some(Force::Pull), Some(Force::Push), None].into(),
            equipment: [Some(Equipment::Dumbbell), Some(Equipment::Barbell), None].into(),
            ..ExerciseFilter::default()
        },
        ExerciseProperties {
            force: Some(Force::Push),
            equipment: vec![Equipment::Barbell],
            ..ExerciseProperties::default()
        }
    )]
    fn test_exercise_filter_exercise_properties(
        #[case] filter: ExerciseFilter,
        #[case] expected: ExerciseProperties,
    ) {
        assert_eq!(filter.exercise_properties(), expected);

        let ExerciseProperties {
            muscles,
            force,
            mechanic,
            laterality,
            assistance,
            equipment,
            category,
        } = expected;
        let exercise = Exercise {
            muscles,
            force,
            mechanic,
            laterality,
            assistance,
            equipment,
            category,
            ..exercise(0, "Exercise", vec![])
        };

        assert_eq!(filter.exercises([&exercise].into_iter()), vec![&exercise]);
    }

    #[test]
    fn test_exercise_filter_is_empty() {
        assert!(ExerciseFilter::default().is_empty());
    }

    #[test]
    fn test_exercise_filter_toggle_muscle() {
        let mut filter = ExerciseFilter::default();

        assert!(!filter.muscles_not_set());
        assert!(filter.muscle_list().iter().all(|(_, l)| l.is_none()));

        filter.toggle_muscle(None);

        assert!(filter.muscles_not_set());
        assert!(filter.muscle_list().iter().all(|(_, l)| l.is_none()));

        filter.toggle_muscle(Some(MuscleID::Abs));

        assert!(!filter.muscles_not_set());
        assert!(
            filter
                .muscle_list()
                .contains(&(MuscleID::Abs, Some(StimulusLevel::Primary)))
        );

        filter.toggle_muscle(Some(MuscleID::Abs));

        assert!(
            filter
                .muscle_list()
                .contains(&(MuscleID::Abs, Some(StimulusLevel::Secondary)))
        );

        filter.toggle_muscle(None);

        assert!(filter.muscles_not_set());
        assert!(filter.muscle_list().iter().all(|(_, l)| l.is_none()));

        filter.toggle_muscle(None);

        assert!(!filter.muscles_not_set());

        filter.toggle_muscle(Some(MuscleID::Abs));
        filter.toggle_muscle(Some(MuscleID::Abs));
        filter.toggle_muscle(Some(MuscleID::Abs));

        assert!(filter.muscle_list().iter().all(|(_, l)| l.is_none()));
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

    fn assert_distinct_names<T: Property + 'static>() {
        assert_distinct(
            T::iter()
                .map(|value| name_or_none(Some(*value)))
                .chain([T::none_name()]),
        );
    }

    fn assert_distinct_value_names<T: Property + 'static>() {
        assert_distinct(T::iter().map(|value| value.name()));
    }

    fn assert_distinct(names: impl Iterator<Item = &'static str>) {
        let mut distinct_names = HashSet::new();

        for name in names {
            assert!(!name.is_empty());
            assert!(distinct_names.insert(name));
        }
    }

    fn catalog_exercise(name: &str, id: u128) -> Exercise {
        let ExerciseProperties {
            muscles,
            force,
            mechanic,
            laterality,
            assistance,
            equipment,
            category,
        } = ExerciseProperties::from(&catalog::EXERCISES[&Name::new(name).unwrap()]);
        Exercise {
            id: id.into(),
            name: Name::new(name).unwrap(),
            muscles,
            force,
            mechanic,
            laterality,
            assistance,
            equipment,
            category,
        }
    }

    fn value(name: &'static str) -> PropertyValue {
        PropertyValue {
            name,
            stimulus: None,
        }
    }

    fn muscle_value(muscle_id: MuscleID, stimulus: Stimulus) -> PropertyValue {
        PropertyValue {
            name: muscle_id.name(),
            stimulus: Some(stimulus),
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
}
