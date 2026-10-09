use std::{
    collections::{BTreeMap, HashSet},
    ops::{Add, AddAssign, Div, Mul},
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
        notes: String,
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
        match self.get_exercises().await {
            Ok(exercises) => validate_exercise_name(name, id, &exercises),
            Err(err) => Err(ValidationError::Other(err.into())),
        }
    }

    async fn get_exercise(&self, id: ExerciseID) -> Result<Option<Exercise>, ReadError> {
        Ok(self.get_exercises().await?.into_iter().find(|e| e.id == id))
    }
}

/// Validates the name of the exercise `id`, which must not be used by any other of `exercises`.
pub fn validate_exercise_name(
    name: &str,
    id: ExerciseID,
    exercises: &[Exercise],
) -> Result<Name, ValidationError> {
    let name = Name::new(name).map_err(|err| ValidationError::Other(err.into()))?;
    if exercises.iter().all(|e| e.id == id || e.name != name) {
        Ok(name)
    } else {
        Err(ValidationError::Conflict("name".to_string()))
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
        notes: String,
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
    pub notes: String,
    pub muscles: Vec<ExerciseMuscle>,
    pub force: Option<Force>,
    pub mechanic: Option<Mechanic>,
    pub laterality: Option<Laterality>,
    pub assistance: Option<Assistance>,
    pub equipment: Vec<Equipment>,
    pub category: Option<Category>,
}

impl Exercise {
    /// The worked muscles with their stimulus.
    #[must_use]
    pub fn muscle_stimulus(&self) -> BTreeMap<MuscleID, Stimulus> {
        self.muscles
            .iter()
            .filter(|m| m.stimulus > Stimulus::NONE)
            .map(|m| (m.muscle_id, m.stimulus))
            .collect()
    }

    /// The stretched muscles with their stretch.
    #[must_use]
    pub fn muscle_stretch(&self) -> BTreeMap<MuscleID, Stimulus> {
        self.muscles
            .iter()
            .filter(|m| m.stretch > Stimulus::NONE)
            .map(|m| (m.muscle_id, m.stretch))
            .collect()
    }

    /// Whether a property other than the equipment is unset or no muscle is worked or stretched.
    ///
    /// No equipment is a valid value of a bodyweight exercise.
    #[must_use]
    pub fn misses_value(&self) -> bool {
        self.force.is_none()
            || self.mechanic.is_none()
            || self.laterality.is_none()
            || self.assistance.is_none()
            || self.category.is_none()
            || self
                .muscles
                .iter()
                .all(|m| m.stimulus == Stimulus::NONE && m.stretch == Stimulus::NONE)
    }

    /// Whether the exercise stretches at least one muscle and works none.
    #[must_use]
    pub fn is_stretch_only(&self) -> bool {
        self.muscles.iter().any(|m| m.stretch > Stimulus::NONE)
            && self.muscles.iter().all(|m| m.stimulus == Stimulus::NONE)
    }
}

/// The IDs of the stretch-only exercises.
#[derive(Deref, Debug, Default, Clone, PartialEq, Eq)]
pub struct StretchOnlyExercises(HashSet<ExerciseID>);

impl StretchOnlyExercises {
    #[must_use]
    pub fn new(exercises: &[Exercise]) -> Self {
        Self(
            exercises
                .iter()
                .filter(|e| e.is_stretch_only())
                .map(|e| e.id)
                .collect(),
        )
    }

    /// The exercises among these that stretch `muscle`.
    #[must_use]
    pub fn stretching(&self, muscle: MuscleID, exercises: &[Exercise]) -> Self {
        Self(
            exercises
                .iter()
                .filter(|e| self.contains(&e.id) && e.muscle_stretch().contains_key(&muscle))
                .map(|e| e.id)
                .collect(),
        )
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
            muscles: merge_muscles(value.muscles.iter().copied(), value.stretch.iter().copied()),
            force: Some(value.force),
            mechanic: Some(value.mechanic),
            laterality: Some(value.laterality),
            assistance: Some(value.assistance),
            equipment: value.equipment.to_vec(),
            category: Some(value.category),
        }
    }
}

/// Merge worked and stretched muscles into one entry per muscle, ordered by muscle.
fn merge_muscles(
    stimulus: impl Iterator<Item = (MuscleID, Stimulus)>,
    stretch: impl Iterator<Item = (MuscleID, Stimulus)>,
) -> Vec<ExerciseMuscle> {
    let mut muscles = BTreeMap::<MuscleID, ExerciseMuscle>::new();
    for (muscle_id, stimulus) in stimulus {
        muscles
            .entry(muscle_id)
            .or_insert_with(|| ExerciseMuscle::new(muscle_id))
            .stimulus = stimulus;
    }
    for (muscle_id, stretch) in stretch {
        muscles
            .entry(muscle_id)
            .or_insert_with(|| ExerciseMuscle::new(muscle_id))
            .stretch = stretch;
    }
    muscles.into_values().collect()
}

/// How the catalog exercise of an update was determined.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogMatch {
    Exact,
    /// The name equals an alias of the catalog exercise.
    Alias,
    /// The name starts with the name or an alias of the catalog exercise, followed by a space.
    Prefix,
    /// The catalog exercise was chosen explicitly instead of matched by name.
    Chosen,
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
///
/// A muscle carries its stimulus, or its stretch if it is a stretched muscle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyValue {
    pub name: &'static str,
    pub stimulus: Option<Stimulus>,
}

/// The values of a property before and after an update.
///
/// The worked and the stretched muscles are changed separately.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyChange {
    pub property: ExerciseProperty,
    pub stretched: bool,
    pub before: Vec<PropertyValue>,
    pub after: Vec<PropertyValue>,
}

/// An exercise updated from a catalog exercise.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogUpdate {
    pub exercise: Exercise,
    pub catalog_name: &'static Name,
    pub catalog_match: CatalogMatch,
    pub changes: Vec<PropertyChange>,
}

impl CatalogUpdate {
    /// Returns whether the update has changes and its catalog exercise was not matched by a prefix
    /// of the name.
    #[must_use]
    pub fn is_preselected(&self) -> bool {
        self.has_changes()
            && matches!(
                self.catalog_match,
                CatalogMatch::Exact | CatalogMatch::Alias | CatalogMatch::Chosen
            )
    }

    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }
}

/// Determine the updates from the catalog for all given exercises, sorted by exercise name.
///
/// The catalog exercise chosen for an exercise replaces the catalog exercise its name matches. An
/// update without changes is only kept if its catalog exercise was chosen.
#[must_use]
pub fn catalog_updates(
    exercises: &[Exercise],
    mode: CatalogUpdateMode,
    chosen: &BTreeMap<ExerciseID, &'static Name>,
) -> Vec<CatalogUpdate> {
    let mut updates = exercises
        .iter()
        .filter_map(|exercise| catalog_update(exercise, mode, chosen.get(&exercise.id).copied()))
        .filter(|update| update.catalog_match == CatalogMatch::Chosen || update.has_changes())
        .collect::<Vec<_>>();
    updates.sort_by(|a, b| a.exercise.name.cmp(&b.exercise.name));
    updates
}

/// Determine the update of an exercise from the chosen catalog exercise, or otherwise from the
/// catalog exercise its name matches.
///
/// Returns `None` if no catalog exercise is chosen and the name of the exercise matches none.
#[must_use]
pub fn catalog_update(
    exercise: &Exercise,
    mode: CatalogUpdateMode,
    chosen: Option<&'static Name>,
) -> Option<CatalogUpdate> {
    let (catalog_name, catalog_exercise, catalog_match) = match chosen
        .and_then(|name| catalog::exercise(name).map(|e| (name, e, CatalogMatch::Chosen)))
    {
        Some(chosen) => chosen,
        None => match_catalog(&exercise.name)?,
    };
    let updated_exercise = updated_exercise(exercise, catalog_exercise, mode);
    let changes = changes(exercise, &updated_exercise);

    Some(CatalogUpdate {
        exercise: updated_exercise,
        catalog_name,
        catalog_match,
        changes,
    })
}

/// Determine the exercises whose name matches no catalog exercise, for which no catalog exercise
/// is chosen and which miss a value, sorted by name.
#[must_use]
pub fn exercises_to_choose_for<'a>(
    exercises: &'a [Exercise],
    chosen: &BTreeMap<ExerciseID, &'static Name>,
) -> Vec<&'a Exercise> {
    let mut exercises = exercises
        .iter()
        .filter(|exercise| {
            !chosen.contains_key(&exercise.id)
                && match_catalog(&exercise.name).is_none()
                && exercise.misses_value()
        })
        .collect::<Vec<_>>();
    exercises.sort_by(|a, b| a.name.cmp(&b.name));
    exercises
}

/// Determine a search term for catalog exercises similar to an exercise of the given name.
///
/// This is the longest catalog name or alias the name ends with, following a space, or otherwise
/// the catalog name or alias the name matches, each spelled as in the catalog.
#[must_use]
pub fn catalog_search_term(name: &Name) -> String {
    let name = normalized_name(name.as_ref());
    CATALOG_NAMES
        .iter()
        .filter(|candidate| {
            name.strip_suffix(candidate.name.as_str())
                .is_some_and(|rest| rest.ends_with(' '))
        })
        .max_by_key(|candidate| candidate.name.len())
        .or_else(|| best_match(&name, &CATALOG_NAMES).map(|(candidate, _)| candidate))
        .map(|candidate| candidate.spelling.to_string())
        .unwrap_or_default()
}

/// Find the catalog exercise a name matches.
fn match_catalog(name: &Name) -> Option<(&'static Name, &'static catalog::Exercise, CatalogMatch)> {
    best_match(&normalized_name(name.as_ref()), &CATALOG_NAMES).map(|(candidate, catalog_match)| {
        let (catalog_name, catalog_exercise) = candidate.entry;
        (catalog_name, catalog_exercise, catalog_match)
    })
}

/// Find the best match of a normalized name among candidates of normalized names or aliases.
///
/// An exact name or alias is preferred over the longest name or alias the name starts with,
/// followed by a space.
fn best_match<'a, T>(
    name: &str,
    candidates: &'a [CatalogCandidate<T>],
) -> Option<(&'a CatalogCandidate<T>, CatalogMatch)> {
    let mut prefix_match: Option<&CatalogCandidate<T>> = None;

    for candidate in candidates {
        if name == candidate.name {
            return Some((
                candidate,
                if candidate.is_alias {
                    CatalogMatch::Alias
                } else {
                    CatalogMatch::Exact
                },
            ));
        }

        if name
            .strip_prefix(candidate.name.as_str())
            .is_some_and(|rest| rest.starts_with(' '))
            && prefix_match.is_none_or(|best| best.name.len() < candidate.name.len())
        {
            prefix_match = Some(candidate);
        }
    }

    prefix_match.map(|candidate| (candidate, CatalogMatch::Prefix))
}

/// A name or alias of a catalog exercise.
struct CatalogCandidate<T> {
    name: String,
    spelling: &'static str,
    is_alias: bool,
    entry: T,
}

type CatalogEntry = (&'static Name, &'static catalog::Exercise);

/// Catalog names and aliases, normalized and paired with the entry they belong to.
static CATALOG_NAMES: LazyLock<Vec<CatalogCandidate<CatalogEntry>>> = LazyLock::new(|| {
    let exercises: &'static BTreeMap<Name, catalog::Exercise> = &catalog::EXERCISES;
    exercises
        .iter()
        .flat_map(|(name, exercise)| {
            std::iter::once(CatalogCandidate {
                name: normalized_name(name.as_ref()),
                spelling: name.as_ref().as_str(),
                is_alias: false,
                entry: (name, exercise),
            })
            .chain(exercise.aliases.iter().map(move |alias| CatalogCandidate {
                name: normalized_name(alias),
                spelling: alias,
                is_alias: true,
                entry: (name, exercise),
            }))
        })
        .collect()
});

/// Normalize a name for comparison, ignoring case and treating a hyphen like a space.
pub(crate) fn normalized_name(name: &str) -> String {
    name.to_lowercase().replace('-', " ")
}

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
            muscles: filled_muscles(&exercise.muscles, &muscles),
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

/// Fill the worked muscles from the catalog if none is worked and the stretched muscles if none is
/// stretched.
fn filled_muscles(
    muscles: &[ExerciseMuscle],
    catalog_muscles: &[ExerciseMuscle],
) -> Vec<ExerciseMuscle> {
    let source = |fill: bool| if fill { catalog_muscles } else { muscles };
    let stimulus = source(muscles.iter().all(|m| m.stimulus == Stimulus::NONE));
    let stretch = source(muscles.iter().all(|m| m.stretch == Stimulus::NONE));
    merge_muscles(
        stimulus
            .iter()
            .filter(|m| m.stimulus > Stimulus::NONE)
            .map(|m| (m.muscle_id, m.stimulus)),
        stretch
            .iter()
            .filter(|m| m.stretch > Stimulus::NONE)
            .map(|m| (m.muscle_id, m.stretch)),
    )
}

fn changes(before: &Exercise, after: &Exercise) -> Vec<PropertyChange> {
    ExerciseProperty::iter()
        .flat_map(|property| {
            let kinds: &[bool] = if *property == ExerciseProperty::Muscles {
                &[false, true]
            } else {
                &[false]
            };
            kinds.iter().filter_map(|stretched| {
                change(
                    *property,
                    *stretched,
                    property_values(before, *property, *stretched),
                    property_values(after, *property, *stretched),
                )
            })
        })
        .collect()
}

fn change(
    property: ExerciseProperty,
    stretched: bool,
    before: Vec<PropertyValue>,
    after: Vec<PropertyValue>,
) -> Option<PropertyChange> {
    if before == after {
        None
    } else {
        Some(PropertyChange {
            property,
            stretched,
            before,
            after,
        })
    }
}

fn property_values(
    exercise: &Exercise,
    property: ExerciseProperty,
    stretched: bool,
) -> Vec<PropertyValue> {
    match property {
        ExerciseProperty::Muscles if stretched => muscle_values(&exercise.muscles, |m| m.stretch),
        ExerciseProperty::Muscles => muscle_values(&exercise.muscles, |m| m.stimulus),
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

/// Determine the values of the muscles with a nonzero `level`, ordered by `MuscleID::iter`.
fn muscle_values(
    muscles: &[ExerciseMuscle],
    level: fn(&ExerciseMuscle) -> Stimulus,
) -> Vec<PropertyValue> {
    values(
        MuscleID::iter()
            .filter_map(|muscle_id| muscles.iter().find(|m| m.muscle_id == *muscle_id))
            .filter(|muscle| level(muscle) > Stimulus::NONE)
            .map(|muscle| PropertyValue {
                name: muscle.muscle_id.name(),
                stimulus: Some(level(muscle)),
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

/// A muscle of an exercise, which is worked, stretched or both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExerciseMuscle {
    pub muscle_id: MuscleID,
    pub stimulus: Stimulus,
    pub stretch: Stimulus,
}

impl ExerciseMuscle {
    fn new(muscle_id: MuscleID) -> Self {
        Self {
            muscle_id,
            stimulus: Stimulus::NONE,
            stretch: Stimulus::NONE,
        }
    }
}

/// The degree to which a muscle is worked or stretched by an exercise.
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

    /// The stimulus as a fraction of the primary stimulus.
    #[must_use]
    pub fn fraction(self) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        let value = self.0 as f32;
        value / 100.
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

impl Div<u32> for Stimulus {
    type Output = Stimulus;

    fn div(self, rhs: u32) -> Self::Output {
        Self(self.0 / rhs)
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
    HipFlexors = 73,
    HipRotators = 74,
    // Thighs
    Quads = 81,
    Hamstrings = 82,
    Adductors = 83,
    // Calves
    Calves = 91,
}

impl Property for MuscleID {
    fn iter() -> Iter<'static, MuscleID> {
        static MUSCLES: [MuscleID; 20] = [
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
            MuscleID::HipFlexors,
            MuscleID::HipRotators,
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
            MuscleID::HipFlexors => "Hip Flexors",
            MuscleID::HipRotators => "Hip Rotators",
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
            MuscleID::HipFlexors => "Front of hips",
            MuscleID::HipRotators => "Deep hip",
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
    Flexibility = 3,
}

impl Property for Category {
    fn iter() -> Iter<'static, Category> {
        static CATEGORY: [Category; 3] = [
            Category::Strength,
            Category::Plyometrics,
            Category::Flexibility,
        ];
        CATEGORY.iter()
    }

    fn name(self) -> &'static str {
        match self {
            Category::Strength => "Strength",
            Category::Plyometrics => "Plyometrics",
            Category::Flexibility => "Flexibility",
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
    pub stretch: HashSet<(MuscleID, StimulusLevel)>,
    pub force: HashSet<Option<Force>>,
    pub mechanic: HashSet<Option<Mechanic>>,
    pub laterality: HashSet<Option<Laterality>>,
    pub assistance: HashSet<Option<Assistance>>,
    pub equipment: HashSet<Option<Equipment>>,
    pub category: HashSet<Option<Category>>,
}

impl ExerciseFilter {
    /// A filter which matches exercises with the same primary worked muscles, or with the same
    /// primary stretched muscles if the exercise has no primary worked muscle.
    #[must_use]
    pub fn primary_muscles_of(exercise: &Exercise) -> Self {
        let primary = |stimulus: Stimulus| {
            StimulusLevel::from_stimulus(stimulus) == Some(StimulusLevel::Primary)
        };
        let muscles = exercise
            .muscles
            .iter()
            .filter(|m| primary(m.stimulus))
            .map(|m| Some((m.muscle_id, StimulusLevel::Primary)))
            .collect::<HashSet<_>>();
        let stretch = if muscles.is_empty() {
            exercise
                .muscles
                .iter()
                .filter(|m| primary(m.stretch))
                .map(|m| (m.muscle_id, StimulusLevel::Primary))
                .collect()
        } else {
            HashSet::new()
        };
        Self {
            muscles,
            stretch,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn exercises<'a>(
        &self,
        exercises: impl Iterator<Item = &'a Exercise>,
    ) -> Vec<&'a Exercise> {
        let search_term = normalized_name(self.name.trim());
        exercises
            .filter(|e| {
                normalized_name(e.name.as_ref()).contains(&search_term)
                    && self.matches_muscles(
                        |muscle, level| {
                            e.muscles.iter().any(|em| {
                                em.muscle_id == muscle
                                    && StimulusLevel::from_stimulus(em.stimulus) == Some(level)
                            })
                        },
                        |muscle, level| {
                            e.muscles.iter().any(|em| {
                                em.muscle_id == muscle
                                    && StimulusLevel::from_stimulus(em.stretch) == Some(level)
                            })
                        },
                        e.muscles.is_empty(),
                    )
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
        let search_term = normalized_name(self.name.trim());
        CATALOG_NAMES
            .iter()
            .filter(|candidate| candidate.name.contains(&search_term))
            .map(|candidate| candidate.entry.1)
            .filter(|e| {
                self.matches_muscles(
                    |muscle, level| contains_muscle(e.muscles, muscle, level),
                    |muscle, level| contains_muscle(e.stretch, muscle, level),
                    e.muscles.is_empty() && e.stretch.is_empty(),
                ) && (self.force.is_empty() || self.force.contains(&Some(e.force)))
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
        ExerciseProperties {
            muscles: merge_muscles(
                self.muscles
                    .iter()
                    .filter_map(|m| m.map(|(muscle_id, level)| (muscle_id, level.stimulus()))),
                self.stretch
                    .iter()
                    .map(|(muscle_id, level)| (*muscle_id, level.stimulus())),
            ),
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
            && self.stretch.is_empty()
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
    pub fn stretch_list(&self) -> Vec<(MuscleID, Option<StimulusLevel>)> {
        MuscleID::iter()
            .map(|muscle| (*muscle, self.stretch_level(*muscle)))
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

    /// Cycle a worked muscle through no level, primary and secondary, or toggle "Not Set".
    ///
    /// "Not Set" excludes the worked and the stretched muscles.
    pub fn toggle_muscle(&mut self, muscle: Option<MuscleID>) {
        let Some(muscle) = muscle else {
            if !self.muscles.remove(&None) {
                self.muscles.clear();
                self.stretch.clear();
                self.muscles.insert(None);
            }
            return;
        };
        let level = next_level(self.muscle_level(muscle));
        self.clear_muscle(Some(muscle));
        if let Some(level) = level {
            self.clear_muscle(None);
            self.muscles.insert(Some((muscle, level)));
        }
    }

    /// Cycle a stretched muscle through no level, primary and secondary.
    ///
    /// Selecting a stretched muscle removes "Not Set".
    pub fn toggle_stretch(&mut self, muscle: MuscleID) {
        let level = next_level(self.stretch_level(muscle));
        self.clear_stretch(muscle);
        if let Some(level) = level {
            self.clear_muscle(None);
            self.stretch.insert((muscle, level));
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

    /// Remove a stretched muscle from the selection.
    pub fn clear_stretch(&mut self, muscle: MuscleID) {
        self.stretch.retain(|(m, _)| *m != muscle);
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

    /// Whether an exercise has every selected worked and stretched muscle at the selected level.
    fn matches_muscles(
        &self,
        worked: impl Fn(MuscleID, StimulusLevel) -> bool,
        stretched: impl Fn(MuscleID, StimulusLevel) -> bool,
        without_muscles: bool,
    ) -> bool {
        self.muscles.iter().all(|m| match m {
            Some((muscle, level)) => worked(*muscle, *level),
            None => without_muscles,
        }) && self
            .stretch
            .iter()
            .all(|(muscle, level)| stretched(*muscle, *level))
    }

    fn muscle_level(&self, muscle: MuscleID) -> Option<StimulusLevel> {
        self.muscles.iter().find_map(|m| match m {
            Some((m, level)) if *m == muscle => Some(*level),
            _ => None,
        })
    }

    fn stretch_level(&self, muscle: MuscleID) -> Option<StimulusLevel> {
        self.stretch
            .iter()
            .find_map(|(m, level)| (*m == muscle).then_some(*level))
    }
}

fn next_level(level: Option<StimulusLevel>) -> Option<StimulusLevel> {
    match level {
        None => Some(StimulusLevel::Primary),
        Some(StimulusLevel::Primary) => Some(StimulusLevel::Secondary),
        Some(StimulusLevel::Secondary) => None,
    }
}

fn contains_muscle(
    muscles: &[(MuscleID, Stimulus)],
    muscle: MuscleID,
    level: StimulusLevel,
) -> bool {
    muscles
        .iter()
        .any(|(m, stimulus)| *m == muscle && StimulusLevel::from_stimulus(*stimulus) == Some(level))
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

    use crate::{
        Service,
        tests::{Call, FakeRepository},
    };

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
                        stretch: Stimulus::NONE,
                    },
                    ExerciseMuscle {
                        muscle_id: MuscleID::Traps,
                        stimulus: Stimulus::SECONDARY,
                        stretch: Stimulus::NONE,
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

    #[test]
    fn test_exercise_muscle_stimulus_and_stretch_leave_out_the_other_kind() {
        let exercise = exercise(
            1,
            "A",
            vec![
                muscle(
                    MuscleID::ErectorSpinae,
                    Stimulus::PRIMARY,
                    Stimulus::SECONDARY,
                ),
                muscle(MuscleID::Hamstrings, Stimulus::NONE, Stimulus::PRIMARY),
                muscle(MuscleID::Glutes, Stimulus::SECONDARY, Stimulus::NONE),
            ],
        );

        assert_eq!(
            exercise.muscle_stimulus(),
            BTreeMap::from([
                (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
                (MuscleID::Glutes, Stimulus::SECONDARY),
            ])
        );
        assert_eq!(
            exercise.muscle_stretch(),
            BTreeMap::from([
                (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
                (MuscleID::Hamstrings, Stimulus::PRIMARY),
            ])
        );
    }

    #[rstest]
    #[case::stretch(vec![muscle(MuscleID::Hamstrings, Stimulus::NONE, Stimulus::PRIMARY)], true)]
    #[case::hybrid(
        vec![
            muscle(MuscleID::ErectorSpinae, Stimulus::PRIMARY, Stimulus::NONE),
            muscle(MuscleID::Hamstrings, Stimulus::NONE, Stimulus::PRIMARY),
        ],
        false
    )]
    #[case::worked(vec![muscle(MuscleID::Pecs, Stimulus::PRIMARY, Stimulus::NONE)], false)]
    #[case::no_muscles(vec![], false)]
    fn test_exercise_is_stretch_only(#[case] muscles: Vec<ExerciseMuscle>, #[case] expected: bool) {
        assert_eq!(exercise(1, "A", muscles).is_stretch_only(), expected);
    }

    #[test]
    fn test_stretch_only_exercises_stretching_a_muscle() {
        let exercises = [
            exercise(
                1,
                "A",
                vec![muscle(
                    MuscleID::Hamstrings,
                    Stimulus::NONE,
                    Stimulus::PRIMARY,
                )],
            ),
            exercise(
                2,
                "B",
                vec![muscle(MuscleID::Calves, Stimulus::NONE, Stimulus::PRIMARY)],
            ),
            exercise(
                3,
                "C",
                vec![
                    muscle(MuscleID::ErectorSpinae, Stimulus::PRIMARY, Stimulus::NONE),
                    muscle(MuscleID::Hamstrings, Stimulus::NONE, Stimulus::PRIMARY),
                ],
            ),
        ];

        assert_eq!(
            *StretchOnlyExercises::new(&exercises).stretching(MuscleID::Hamstrings, &exercises),
            HashSet::from([1.into()])
        );
    }

    #[test]
    fn test_merge_muscles() {
        assert_eq!(
            merge_muscles(
                [
                    (MuscleID::Hamstrings, Stimulus::SECONDARY),
                    (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
                ]
                .into_iter(),
                [
                    (MuscleID::Hamstrings, Stimulus::PRIMARY),
                    (MuscleID::Calves, Stimulus::SECONDARY),
                ]
                .into_iter(),
            ),
            vec![
                muscle(MuscleID::ErectorSpinae, Stimulus::PRIMARY, Stimulus::NONE),
                muscle(MuscleID::Hamstrings, Stimulus::SECONDARY, Stimulus::PRIMARY),
                muscle(MuscleID::Calves, Stimulus::NONE, Stimulus::SECONDARY),
            ]
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
                        stretch: Stimulus::NONE,
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
    #[case("Russian Twist", None)]
    #[case("Bench Press", Some(("Barbell Bench Press", CatalogMatch::Alias)))]
    #[case("bench press", Some(("Barbell Bench Press", CatalogMatch::Alias)))]
    #[case("Overhead Press", Some(("Barbell Shoulder Press", CatalogMatch::Alias)))]
    #[case("Military Press", Some(("Barbell Shoulder Press", CatalogMatch::Alias)))]
    #[case("Bench Press (paused)", Some(("Barbell Bench Press", CatalogMatch::Prefix)))]
    #[case("Pull-Up", Some(("Pull Up", CatalogMatch::Exact)))]
    #[case("Single Leg Glute Bridge", Some(("Single-Leg Glute Bridge", CatalogMatch::Exact)))]
    #[case("Pull-Up (weighted)", Some(("Pull Up", CatalogMatch::Prefix)))]
    #[case("Close Grip Bench Press", Some(("Barbell Close-Grip Bench Press", CatalogMatch::Alias)))]
    #[case("Front Squat", Some(("Barbell Front Squat", CatalogMatch::Alias)))]
    #[case("Pistol Squat", None)]
    fn test_catalog_update_match(
        #[case] name: &str,
        #[case] expected: Option<(&str, CatalogMatch)>,
    ) {
        let update = catalog_update(
            &exercise(1, name, vec![]),
            CatalogUpdateMode::FillMissing,
            None,
        );

        assert_eq!(
            update
                .as_ref()
                .map(|update| (update.catalog_name.as_ref().as_str(), update.catalog_match)),
            expected
        );
    }

    #[rstest]
    #[case("Dip", true)]
    #[case("Bench Press", true)]
    #[case("Dip (weighted)", false)]
    fn test_catalog_update_is_preselected(#[case] name: &str, #[case] expected: bool) {
        let update = catalog_update(
            &exercise(1, name, vec![]),
            CatalogUpdateMode::FillMissing,
            None,
        )
        .unwrap();

        assert_eq!(update.is_preselected(), expected);
    }

    #[rstest]
    #[case::exact_name("a b", Some(("A B", CatalogMatch::Exact)))]
    #[case::exact_alias_over_shorter_name("c d", Some(("X", CatalogMatch::Alias)))]
    #[case::longest_prefix_of_name("a b e", Some(("A B", CatalogMatch::Prefix)))]
    #[case::longest_prefix_of_alias("c d e", Some(("X", CatalogMatch::Prefix)))]
    #[case::prefix_without_space("ab", None)]
    fn test_best_match(#[case] name: &str, #[case] expected: Option<(&str, CatalogMatch)>) {
        let candidates = [
            candidate("a", false, "A"),
            candidate("c", false, "C"),
            candidate("a b", false, "A B"),
            candidate("c d", true, "X"),
        ];

        assert_eq!(
            best_match(name, &candidates)
                .map(|(candidate, catalog_match)| (candidate.entry, catalog_match)),
            expected
        );
    }

    fn candidate(
        name: &'static str,
        is_alias: bool,
        entry: &'static str,
    ) -> CatalogCandidate<&'static str> {
        CatalogCandidate {
            name: name.to_string(),
            spelling: name,
            is_alias,
            entry,
        }
    }

    fn catalog_name(name: &str) -> &'static Name {
        catalog::EXERCISES
            .get_key_value(&Name::new(name).unwrap())
            .unwrap()
            .0
    }

    #[rstest]
    #[case(CatalogUpdateMode::FillMissing)]
    #[case(CatalogUpdateMode::ReplaceAll)]
    fn test_catalog_update_of_exercise_without_properties(#[case] mode: CatalogUpdateMode) {
        let update = catalog_update(&exercise(1, "Dip", vec![]), mode, None).unwrap();

        assert_eq!(update.exercise, catalog_exercise("Dip", 1));
        assert_eq!(
            update
                .changes
                .iter()
                .map(|change| change.property)
                .collect::<Vec<_>>(),
            vec![
                ExerciseProperty::Muscles,
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
            update.changes[2],
            PropertyChange {
                property: ExerciseProperty::Force,
                stretched: false,
                before: vec![value(Force::none_name())],
                after: vec![value(Force::Push.name())],
            }
        );
        assert_eq!(
            update.changes[0],
            PropertyChange {
                property: ExerciseProperty::Muscles,
                stretched: false,
                before: vec![value(MuscleID::none_name())],
                after: vec![
                    muscle_value(MuscleID::Pecs, Stimulus::PRIMARY),
                    muscle_value(MuscleID::FrontDelts, Stimulus::PRIMARY),
                    muscle_value(MuscleID::Triceps, Stimulus::PRIMARY),
                ],
            }
        );
        assert_eq!(
            update.changes[1],
            PropertyChange {
                property: ExerciseProperty::Muscles,
                stretched: true,
                before: vec![value(MuscleID::none_name())],
                after: vec![
                    muscle_value(MuscleID::Pecs, Stimulus::SECONDARY),
                    muscle_value(MuscleID::FrontDelts, Stimulus::SECONDARY),
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
                    stretch: Stimulus::PRIMARY,
                }],
            )
        };

        let update = catalog_update(&exercise, CatalogUpdateMode::FillMissing, None).unwrap();

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
    fn test_catalog_update_fill_missing_fills_muscles_per_role() {
        let name = "Barbell Romanian Deadlift";
        let catalog = catalog_exercise(name, 1);
        let worked = ExerciseMuscle {
            muscle_id: MuscleID::Biceps,
            stimulus: Stimulus::PRIMARY,
            stretch: Stimulus::NONE,
        };
        let stretched = ExerciseMuscle {
            muscle_id: MuscleID::Calves,
            stimulus: Stimulus::NONE,
            stretch: Stimulus::SECONDARY,
        };

        let only_worked = catalog_update(
            &exercise(1, name, vec![worked.clone()]),
            CatalogUpdateMode::FillMissing,
            None,
        )
        .unwrap()
        .exercise;
        let only_stretched = catalog_update(
            &exercise(1, name, vec![stretched.clone()]),
            CatalogUpdateMode::FillMissing,
            None,
        )
        .unwrap()
        .exercise;

        assert_eq!(
            only_worked.muscle_stimulus(),
            BTreeMap::from([(worked.muscle_id, worked.stimulus)])
        );
        assert_eq!(only_worked.muscle_stretch(), catalog.muscle_stretch());
        assert_eq!(only_stretched.muscle_stimulus(), catalog.muscle_stimulus());
        assert_eq!(
            only_stretched.muscle_stretch(),
            BTreeMap::from([(stretched.muscle_id, stretched.stretch)])
        );
    }

    #[test]
    fn test_catalog_update_replace_all_clears_values_absent_from_catalog() {
        let exercise = Exercise {
            equipment: vec![Equipment::Barbell],
            ..catalog_exercise("Squat", 1)
        };

        let update = catalog_update(&exercise, CatalogUpdateMode::ReplaceAll, None).unwrap();

        assert_eq!(update.exercise, catalog_exercise("Squat", 1));
        assert_eq!(
            update.changes,
            vec![PropertyChange {
                property: ExerciseProperty::Equipment,
                stretched: false,
                before: vec![value(Equipment::Barbell.name())],
                after: vec![value(Equipment::none_name())],
            }]
        );
    }

    #[test]
    fn test_catalog_updates_ignore_order_of_equipment() {
        let exercise = Exercise {
            equipment: vec![Equipment::ParallelBars, Equipment::ResistanceBand],
            ..catalog_exercise("Band-Assisted Dip", 1)
        };

        assert_eq!(
            catalog_updates(&[exercise], CatalogUpdateMode::ReplaceAll, &BTreeMap::new()),
            vec![]
        );
    }

    #[test]
    fn test_catalog_update_detects_changed_stimulus() {
        let mut exercise = catalog_exercise("Dip", 1);
        exercise.muscles[0].stimulus = Stimulus::SECONDARY;

        let update = catalog_update(&exercise, CatalogUpdateMode::ReplaceAll, None).unwrap();

        assert_eq!(update.exercise, catalog_exercise("Dip", 1));
        assert_eq!(
            update.changes,
            vec![PropertyChange {
                property: ExerciseProperty::Muscles,
                stretched: false,
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

    #[test]
    fn test_catalog_update_detects_changed_stretch() {
        let mut exercise = catalog_exercise("Dip", 1);
        exercise.muscles[0].stretch = Stimulus::PRIMARY;
        exercise
            .muscles
            .push(muscle(MuscleID::Biceps, Stimulus::NONE, Stimulus::PRIMARY));

        let update = catalog_update(&exercise, CatalogUpdateMode::ReplaceAll, None).unwrap();

        assert_eq!(update.exercise, catalog_exercise("Dip", 1));
        assert_eq!(
            update.changes,
            vec![PropertyChange {
                property: ExerciseProperty::Muscles,
                stretched: true,
                before: vec![
                    muscle_value(MuscleID::Pecs, Stimulus::PRIMARY),
                    muscle_value(MuscleID::FrontDelts, Stimulus::SECONDARY),
                    muscle_value(MuscleID::Biceps, Stimulus::PRIMARY),
                ],
                after: vec![
                    muscle_value(MuscleID::Pecs, Stimulus::SECONDARY),
                    muscle_value(MuscleID::FrontDelts, Stimulus::SECONDARY),
                ],
            }]
        );
    }

    #[test]
    fn test_catalog_update_lists_the_stretched_muscles_after_the_worked_muscles() {
        let mut exercise = catalog_exercise("Dip", 1);
        exercise.muscles[0] = muscle(MuscleID::Pecs, Stimulus::SECONDARY, Stimulus::PRIMARY);

        let update = catalog_update(&exercise, CatalogUpdateMode::ReplaceAll, None).unwrap();

        assert_eq!(
            update
                .changes
                .iter()
                .map(|c| (c.property, c.stretched))
                .collect::<Vec<_>>(),
            [
                (ExerciseProperty::Muscles, false),
                (ExerciseProperty::Muscles, true)
            ]
        );
    }

    #[rstest]
    #[case(CatalogUpdateMode::FillMissing)]
    #[case(CatalogUpdateMode::ReplaceAll)]
    fn test_catalog_updates_of_unchanged_exercise(#[case] mode: CatalogUpdateMode) {
        assert_eq!(
            catalog_updates(&[catalog_exercise("Dip", 1)], mode, &BTreeMap::new()),
            vec![]
        );
    }

    #[rstest]
    #[case(CatalogUpdateMode::FillMissing)]
    #[case(CatalogUpdateMode::ReplaceAll)]
    fn test_catalog_update_of_unchanged_exercise_matching_by_name(#[case] mode: CatalogUpdateMode) {
        let update = catalog_update(&catalog_exercise("Dip", 1), mode, None).unwrap();

        assert_eq!(update.catalog_match, CatalogMatch::Exact);
        assert!(!update.has_changes());
        assert!(!update.is_preselected());
    }

    #[test]
    fn test_catalog_update_without_match_or_choice() {
        assert_eq!(
            catalog_update(
                &exercise(1, "Russian Twist", vec![]),
                CatalogUpdateMode::FillMissing,
                None
            ),
            None
        );
    }

    #[test]
    fn test_catalog_updates_choice_replaces_match_by_name() {
        let updates = catalog_updates(
            &[exercise(1, "Squat", vec![])],
            CatalogUpdateMode::FillMissing,
            &BTreeMap::from([(1.into(), catalog_name("Barbell Squat"))]),
        );

        assert_eq!(
            updates
                .iter()
                .map(|update| (
                    update.catalog_name.as_ref().as_str(),
                    update.catalog_match,
                    update.is_preselected()
                ))
                .collect::<Vec<_>>(),
            vec![("Barbell Squat", CatalogMatch::Chosen, true)]
        );
        assert_eq!(updates[0].exercise.equipment, vec![Equipment::Barbell]);
    }

    #[test]
    fn test_catalog_updates_keep_chosen_update_without_changes() {
        let updates = catalog_updates(
            &[catalog_exercise("Dip", 1)],
            CatalogUpdateMode::ReplaceAll,
            &BTreeMap::from([(1.into(), catalog_name("Dip"))]),
        );

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].catalog_match, CatalogMatch::Chosen);
        assert!(!updates[0].has_changes());
        assert!(!updates[0].is_preselected());
    }

    #[test]
    fn test_exercises_to_choose_for() {
        let complete = Exercise {
            name: Name::new("Complete").unwrap(),
            ..catalog_exercise("Dip", 1)
        };
        let without_equipment = Exercise {
            name: Name::new("Without Equipment").unwrap(),
            equipment: vec![],
            ..catalog_exercise("Dip", 2)
        };
        let without_muscles = Exercise {
            name: Name::new("Without Muscles").unwrap(),
            muscles: vec![],
            ..catalog_exercise("Dip", 3)
        };
        let without_force = Exercise {
            name: Name::new("Without Force").unwrap(),
            force: None,
            ..catalog_exercise("Dip", 4)
        };
        let chosen = exercise(5, "Chosen", vec![]);
        let matching = exercise(6, "Squat", vec![]);
        let exercises = [
            without_muscles.clone(),
            complete,
            without_equipment,
            without_force.clone(),
            chosen,
            matching,
        ];

        assert_eq!(
            exercises_to_choose_for(
                &exercises,
                &BTreeMap::from([(5.into(), catalog_name("Dip"))])
            ),
            vec![&without_force, &without_muscles]
        );
    }

    #[rstest]
    #[case::ending_in_name("Front Squat", "Squat")]
    #[case::ending_in_longest_name("Paused Barbell Squat", "Barbell Squat")]
    #[case::ending_in_alias("Paused Bench Press", "Bench Press")]
    #[case::exact_name("Squat", "Squat")]
    #[case::prefix("Dip (weighted)", "Dip")]
    #[case::hyphen("Weighted Chin-Up", "Chin Up")]
    #[case::none("Russian Twist", "")]
    fn test_catalog_search_term(#[case] exercise_name: &str, #[case] expected: &str) {
        assert_eq!(
            catalog_search_term(&Name::new(exercise_name).unwrap()),
            expected
        );
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
            &BTreeMap::new(),
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
    #[case::no_muscles(vec![], [].into(), [].into())]
    #[case::only_secondary_muscles(
        vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::SECONDARY, stretch: Stimulus::NONE }],
        [].into(),
        [].into()
    )]
    #[case::primary_and_secondary_muscles(
        vec![
            ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY, stretch: Stimulus::NONE },
            ExerciseMuscle { muscle_id: MuscleID::Triceps, stimulus: Stimulus::PRIMARY, stretch: Stimulus::NONE },
            ExerciseMuscle { muscle_id: MuscleID::FrontDelts, stimulus: Stimulus::SECONDARY, stretch: Stimulus::NONE },
        ],
        [
            Some((MuscleID::Pecs, StimulusLevel::Primary)),
            Some((MuscleID::Triceps, StimulusLevel::Primary)),
        ].into(),
        [].into()
    )]
    #[case::stretch_only(
        vec![
            muscle(MuscleID::Hamstrings, Stimulus::NONE, Stimulus::PRIMARY),
            muscle(MuscleID::ErectorSpinae, Stimulus::NONE, Stimulus::SECONDARY),
        ],
        [].into(),
        [(MuscleID::Hamstrings, StimulusLevel::Primary)].into()
    )]
    #[case::hybrid_with_secondary_worked_muscles(
        vec![
            muscle(MuscleID::ErectorSpinae, Stimulus::SECONDARY, Stimulus::NONE),
            muscle(MuscleID::Adductors, Stimulus::NONE, Stimulus::PRIMARY),
        ],
        [].into(),
        [(MuscleID::Adductors, StimulusLevel::Primary)].into()
    )]
    #[case::primary_worked_and_stretched_muscle(
        vec![
            muscle(MuscleID::Pecs, Stimulus::PRIMARY, Stimulus::PRIMARY),
            muscle(MuscleID::FrontDelts, Stimulus::SECONDARY, Stimulus::NONE),
        ],
        [Some((MuscleID::Pecs, StimulusLevel::Primary))].into(),
        [].into()
    )]
    fn test_exercise_filter_primary_muscles_of(
        #[case] muscles: Vec<ExerciseMuscle>,
        #[case] expected_muscles: HashSet<Option<(MuscleID, StimulusLevel)>>,
        #[case] expected_stretch: HashSet<(MuscleID, StimulusLevel)>,
    ) {
        let exercise = exercise(0, "A", muscles);

        let filter = ExerciseFilter::primary_muscles_of(&exercise);

        assert_eq!(
            filter,
            ExerciseFilter {
                muscles: expected_muscles,
                stretch: expected_stretch,
                ..ExerciseFilter::default()
            }
        );
        assert_eq!(filter.exercises([&exercise].into_iter()), vec![&exercise]);
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
            exercise(1, "Squat", vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY, stretch: Stimulus::NONE }]),
        ],
        &[exercise(0, "Squat", vec![])]
    )]
    #[case::muscles(
        ExerciseFilter { muscles: [Some((MuscleID::Pecs, StimulusLevel::Primary)), Some((MuscleID::FrontDelts, StimulusLevel::Secondary))].into(), ..ExerciseFilter::default() },
        &[
            exercise(0, "Squat", vec![]),
            exercise(1, "Squat", vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY, stretch: Stimulus::NONE }, ExerciseMuscle { muscle_id: MuscleID::FrontDelts, stimulus: Stimulus::SECONDARY, stretch: Stimulus::NONE }]),
        ],
        &[exercise(1, "Squat", vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY, stretch: Stimulus::NONE }, ExerciseMuscle { muscle_id: MuscleID::FrontDelts, stimulus: Stimulus::SECONDARY, stretch: Stimulus::NONE }])]
    )]
    #[case::muscle_at_other_level(
        ExerciseFilter { muscles: [Some((MuscleID::Pecs, StimulusLevel::Secondary))].into(), ..ExerciseFilter::default() },
        &[
            exercise(0, "Squat", vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY, stretch: Stimulus::NONE }]),
            exercise(1, "Squat", vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::SECONDARY, stretch: Stimulus::NONE }]),
        ],
        &[exercise(1, "Squat", vec![ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::SECONDARY, stretch: Stimulus::NONE }])]
    )]
    #[case::worked_and_stretched_muscles(
        ExerciseFilter {
            muscles: [Some((MuscleID::ErectorSpinae, StimulusLevel::Primary))].into(),
            stretch: [(MuscleID::Hamstrings, StimulusLevel::Primary)].into(),
            ..ExerciseFilter::default()
        },
        &[
            exercise(0, "Jefferson Curl", vec![muscle(MuscleID::ErectorSpinae, Stimulus::PRIMARY, Stimulus::NONE), muscle(MuscleID::Hamstrings, Stimulus::NONE, Stimulus::PRIMARY)]),
            exercise(1, "Back Extension", vec![muscle(MuscleID::ErectorSpinae, Stimulus::PRIMARY, Stimulus::NONE), muscle(MuscleID::Hamstrings, Stimulus::SECONDARY, Stimulus::NONE)]),
            exercise(2, "Forward Fold", vec![muscle(MuscleID::Hamstrings, Stimulus::NONE, Stimulus::PRIMARY)]),
        ],
        &[exercise(0, "Jefferson Curl", vec![muscle(MuscleID::ErectorSpinae, Stimulus::PRIMARY, Stimulus::NONE), muscle(MuscleID::Hamstrings, Stimulus::NONE, Stimulus::PRIMARY)])]
    )]
    #[case::stretched_muscle_at_other_level(
        ExerciseFilter { stretch: [(MuscleID::Hamstrings, StimulusLevel::Secondary)].into(), ..ExerciseFilter::default() },
        &[
            exercise(0, "A", vec![muscle(MuscleID::Hamstrings, Stimulus::SECONDARY, Stimulus::PRIMARY)]),
            exercise(1, "B", vec![muscle(MuscleID::Hamstrings, Stimulus::PRIMARY, Stimulus::SECONDARY)]),
        ],
        &[exercise(1, "B", vec![muscle(MuscleID::Hamstrings, Stimulus::PRIMARY, Stimulus::SECONDARY)])]
    )]
    #[case::name_ignoring_case_and_hyphens(
        ExerciseFilter { name: " pull up ".into(), ..ExerciseFilter::default() },
        &[exercise(0, "Pull-Up", vec![]), exercise(1, "Push-Up", vec![])],
        &[exercise(0, "Pull-Up", vec![])]
    )]
    #[case::name_with_hyphen(
        ExerciseFilter { name: "Pull-up".into(), ..ExerciseFilter::default() },
        &[exercise(0, "Pull Up", vec![]), exercise(1, "Push Up", vec![])],
        &[exercise(0, "Pull Up", vec![])]
    )]
    #[case::no_muscles_excludes_stretched_only_exercise(
        ExerciseFilter { muscles: [None].into(), ..ExerciseFilter::default() },
        &[
            exercise(0, "A", vec![]),
            exercise(1, "B", vec![muscle(MuscleID::Hamstrings, Stimulus::NONE, Stimulus::PRIMARY)]),
        ],
        &[exercise(0, "A", vec![])]
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
        Some("Archer Push Up")
    )]
    #[case::name_upper_case(
        ExerciseFilter { name: "PUSH".into(), ..ExerciseFilter::default() },
        Some("Archer Push Up")
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
        Some("90-90 Hip Stretch")
    )]
    #[case::equipment(
        ExerciseFilter { equipment: [Some(Equipment::Barbell)].into(), ..ExerciseFilter::default() },
        Some("Barbell Ab Rollout")
    )]
    #[case::stretched_muscle(
        ExerciseFilter { stretch: [(MuscleID::HipRotators, StimulusLevel::Primary)].into(), ..ExerciseFilter::default() },
        Some("90-90 Hip Stretch")
    )]
    #[case::stretched_muscle_of_a_stretch(
        ExerciseFilter { name: "couch".into(), stretch: [(MuscleID::HipFlexors, StimulusLevel::Primary)].into(), ..ExerciseFilter::default() },
        Some("Couch Stretch")
    )]
    #[case::no_muscles_excludes_a_stretch(
        ExerciseFilter { name: "couch".into(), muscles: [None].into(), ..ExerciseFilter::default() },
        None
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
    #[case::alias("overhead press", "Barbell Shoulder Press")]
    #[case::second_alias("Military", "Barbell Shoulder Press")]
    #[case::hyphen("pull-up", "Pull Up")]
    #[case::hyphen_of_catalog_name("single leg glute", "Single-Leg Glute Bridge")]
    fn test_exercise_catalog_by_name(#[case] name: &str, #[case] expected_name: &str) {
        let filter = ExerciseFilter {
            name: name.into(),
            ..ExerciseFilter::default()
        };

        assert!(
            filter
                .catalog()
                .contains_key(&Name::new(expected_name).unwrap())
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
                ExerciseMuscle { muscle_id: MuscleID::Pecs, stimulus: Stimulus::PRIMARY, stretch: Stimulus::NONE },
                ExerciseMuscle { muscle_id: MuscleID::Triceps, stimulus: Stimulus::SECONDARY, stretch: Stimulus::NONE },
            ],
            ..ExerciseProperties::default()
        }
    )]
    #[case::stretch(
        ExerciseFilter {
            muscles: [Some((MuscleID::ErectorSpinae, StimulusLevel::Primary))].into(),
            stretch: [
                (MuscleID::Hamstrings, StimulusLevel::Primary),
                (MuscleID::ErectorSpinae, StimulusLevel::Secondary),
            ].into(),
            ..ExerciseFilter::default()
        },
        ExerciseProperties {
            muscles: vec![
                muscle(MuscleID::ErectorSpinae, Stimulus::PRIMARY, Stimulus::SECONDARY),
                muscle(MuscleID::Hamstrings, Stimulus::NONE, Stimulus::PRIMARY),
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
    fn test_exercise_filter_toggle_stretch() {
        let mut filter = ExerciseFilter::default();

        assert!(filter.stretch_list().iter().all(|(_, l)| l.is_none()));

        filter.toggle_muscle(None);
        filter.toggle_stretch(MuscleID::Hamstrings);

        assert!(!filter.muscles_not_set());
        assert!(
            filter
                .stretch_list()
                .contains(&(MuscleID::Hamstrings, Some(StimulusLevel::Primary)))
        );
        assert!(filter.muscle_list().iter().all(|(_, l)| l.is_none()));

        filter.toggle_stretch(MuscleID::Hamstrings);

        assert!(
            filter
                .stretch_list()
                .contains(&(MuscleID::Hamstrings, Some(StimulusLevel::Secondary)))
        );

        filter.toggle_muscle(None);

        assert!(filter.muscles_not_set());
        assert!(filter.stretch_list().iter().all(|(_, l)| l.is_none()));

        filter.toggle_stretch(MuscleID::Hamstrings);
        filter.toggle_stretch(MuscleID::Hamstrings);
        filter.toggle_stretch(MuscleID::Hamstrings);

        assert!(filter.stretch_list().iter().all(|(_, l)| l.is_none()));
    }

    #[test]
    fn test_exercise_filter_clear_stretch() {
        let mut filter = ExerciseFilter {
            stretch: [
                (MuscleID::Hamstrings, StimulusLevel::Primary),
                (MuscleID::Calves, StimulusLevel::Secondary),
            ]
            .into(),
            ..ExerciseFilter::default()
        };

        filter.clear_stretch(MuscleID::Hamstrings);

        assert_eq!(
            filter.stretch,
            [(MuscleID::Calves, StimulusLevel::Secondary)].into()
        );
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
            notes: String::new(),
            muscles,
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        }
    }

    fn muscle(muscle_id: MuscleID, stimulus: Stimulus, stretch: Stimulus) -> ExerciseMuscle {
        ExerciseMuscle {
            muscle_id,
            stimulus,
            stretch,
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
            notes: String::new(),
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

    #[rstest]
    #[case::unused("C", Ok("C"))]
    #[case::used_by_the_exercise_itself("A", Ok("A"))]
    #[case::used_by_another_exercise("B", Err("entry with this name already exists"))]
    #[case::invalid("", Err("name must not be empty"))]
    fn test_validate_exercise_name(#[case] input: &str, #[case] expected: Result<&str, &str>) {
        let service = Service::new(
            FakeRepository::default()
                .with_exercises(vec![exercise(1, "A", vec![]), exercise(2, "B", vec![])]),
        );

        assert_eq!(
            pollster::block_on(service.validate_exercise_name(input, 1.into()))
                .map(|name| name.to_string())
                .map_err(|err| err.to_string()),
            expected.map(str::to_string).map_err(str::to_string)
        );
    }

    #[test]
    fn test_validate_exercise_name_unreadable_exercises() {
        let service = Service::new(FakeRepository::default().failing(Call::ReadExercises));

        assert!(matches!(
            pollster::block_on(service.validate_exercise_name("A", 1.into())),
            Err(ValidationError::Other(_))
        ));
    }

    #[rstest]
    #[case::known(1, Some("A"))]
    #[case::unknown(2, None)]
    fn test_get_exercise(#[case] id: u128, #[case] expected: Option<&str>) {
        let service =
            Service::new(FakeRepository::default().with_exercises(vec![exercise(1, "A", vec![])]));

        assert_eq!(
            pollster::block_on(service.get_exercise(id.into()))
                .unwrap()
                .map(|e| e.name.to_string()),
            expected.map(str::to_string)
        );
    }

    #[rstest]
    #[case::empty(ExerciseFilter::default(), true)]
    #[case::name(ExerciseFilter { name: "push".into(), ..ExerciseFilter::default() }, false)]
    #[case::blank_name(ExerciseFilter { name: "  ".into(), ..ExerciseFilter::default() }, true)]
    #[case::muscles(
        ExerciseFilter {
            muscles: [Some((MuscleID::Pecs, StimulusLevel::Primary))].into(),
            ..ExerciseFilter::default()
        },
        false
    )]
    #[case::stretch(
        ExerciseFilter {
            stretch: [(MuscleID::Hamstrings, StimulusLevel::Primary)].into(),
            ..ExerciseFilter::default()
        },
        false
    )]
    #[case::force(
        ExerciseFilter { force: [Some(Force::Push)].into(), ..ExerciseFilter::default() },
        false
    )]
    #[case::mechanic(
        ExerciseFilter {
            mechanic: [Some(Mechanic::Compound)].into(),
            ..ExerciseFilter::default()
        },
        false
    )]
    #[case::laterality(
        ExerciseFilter {
            laterality: [Some(Laterality::Bilateral)].into(),
            ..ExerciseFilter::default()
        },
        false
    )]
    #[case::assistance(
        ExerciseFilter {
            assistance: [Some(Assistance::Assisted)].into(),
            ..ExerciseFilter::default()
        },
        false
    )]
    #[case::equipment(
        ExerciseFilter {
            equipment: [Some(Equipment::Barbell)].into(),
            ..ExerciseFilter::default()
        },
        false
    )]
    #[case::category(
        ExerciseFilter {
            category: [Some(Category::Strength)].into(),
            ..ExerciseFilter::default()
        },
        false
    )]
    fn test_exercise_filter_is_empty(#[case] filter: ExerciseFilter, #[case] expected: bool) {
        assert_eq!(filter.is_empty(), expected);
    }

    #[test]
    fn test_exercise_filter_muscle_list_only_marks_selected_muscles() {
        let filter = ExerciseFilter {
            muscles: [Some((MuscleID::Pecs, StimulusLevel::Primary))].into(),
            ..ExerciseFilter::default()
        };

        let levels = filter.muscle_list().into_iter().collect::<BTreeMap<_, _>>();

        assert_eq!(levels[&MuscleID::Pecs], Some(StimulusLevel::Primary));
        assert_eq!(levels[&MuscleID::Lats], None);
    }

    #[test]
    fn test_exercise_id_from_str() {
        let id = ExerciseID::from(Uuid::from_u128(1));

        assert_eq!(id, ExerciseID::from(1u128));
        assert!(!id.is_nil());
        assert_eq!(ExerciseID::from_str(&id.to_string()), Ok(id));
    }
}
