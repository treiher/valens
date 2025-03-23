use chrono::NaiveDate;
use gloo_net::http::Request;
use serde_json::{Map, json};
use thiserror::Error;
use valens_domain as domain;

#[derive(Clone)]
pub struct REST;

impl domain::SessionRepository for REST {
    async fn request_session(&self, user_id: domain::UserID) -> Result<domain::User, String> {
        let r: User = fetch(
            Request::post("api/session")
                .json(&json!({ "id": user_id.as_u128() }))
                .expect("serialization failed"),
        )
        .await?;
        r.try_into()
            .map_err(|err: domain::NameError| err.to_string())
    }

    async fn initialize_session(&self) -> Result<domain::User, String> {
        let r: User = fetch(Request::get("api/session").build().unwrap()).await?;
        r.try_into()
            .map_err(|err: domain::NameError| err.to_string())
    }

    async fn delete_session(&self) -> Result<(), String> {
        fetch_no_content(Request::delete("api/session").build().unwrap(), ()).await
    }
}

impl domain::VersionRepository for REST {
    async fn read_version(&self) -> Result<String, String> {
        fetch(Request::get("api/version").build().unwrap()).await
    }
}

impl domain::UserRepository for REST {
    async fn read_users(&self) -> Result<Vec<domain::User>, String> {
        let r: Vec<User> = fetch(Request::get("api/users").build().unwrap()).await?;
        r.into_iter()
            .map(|user| {
                domain::User::try_from(user).map_err(|err: domain::NameError| err.to_string())
            })
            .collect::<Result<Vec<domain::User>, String>>()
    }

    async fn create_user(
        &self,
        name: domain::Name,
        sex: domain::Sex,
    ) -> Result<domain::User, String> {
        let r: User = fetch(
            Request::post("api/users")
                .json(&UserData {
                    name: name.to_string(),
                    sex: sex as u8,
                })
                .expect("serialization failed"),
        )
        .await?;
        r.try_into()
            .map_err(|err: domain::NameError| err.to_string())
    }

    async fn replace_user(&self, user: domain::User) -> Result<domain::User, String> {
        let r: User = fetch(
            Request::put(&format!("api/users/{}", user.id.as_u128()))
                .json(&UserData::from(user))
                .expect("serialization failed"),
        )
        .await?;
        r.try_into()
            .map_err(|err: domain::NameError| err.to_string())
    }

    async fn delete_user(&self, id: domain::UserID) -> Result<domain::UserID, String> {
        fetch_no_content(
            Request::delete(&format!("api/users/{}", id.as_u128()))
                .build()
                .unwrap(),
            id,
        )
        .await
    }
}

impl domain::BodyWeightRepository for REST {
    async fn read_body_weight(&self) -> Result<Vec<domain::BodyWeight>, String> {
        let r: Vec<BodyWeight> = fetch(Request::get("api/body_weight").build().unwrap()).await?;
        Ok(r.into_iter().map(domain::BodyWeight::from).collect())
    }

    async fn create_body_weight(
        &self,
        body_weight: domain::BodyWeight,
    ) -> Result<domain::BodyWeight, String> {
        let r: BodyWeight = fetch(
            Request::post("api/body_weight")
                .json(&BodyWeight::from(body_weight))
                .expect("serialization failed"),
        )
        .await?;
        Ok(r.into())
    }

    async fn replace_body_weight(
        &self,
        body_weight: domain::BodyWeight,
    ) -> Result<domain::BodyWeight, String> {
        let r: BodyWeight = fetch(
            Request::put(&format!("api/body_weight/{}", body_weight.date))
                .json(&json!(&BodyWeightData::from(body_weight)))
                .expect("serialization failed"),
        )
        .await?;
        Ok(r.into())
    }

    async fn delete_body_weight(&self, date: NaiveDate) -> Result<NaiveDate, String> {
        fetch_no_content(
            Request::delete(&format!("api/body_weight/{date}"))
                .build()
                .unwrap(),
            date,
        )
        .await
    }
}

impl domain::BodyFatRepository for REST {
    async fn read_body_fat(&self) -> Result<Vec<domain::BodyFat>, String> {
        let r: Vec<BodyFat> = fetch(Request::get("api/body_fat").build().unwrap()).await?;
        Ok(r.into_iter().map(domain::BodyFat::from).collect())
    }

    async fn create_body_fat(&self, body_fat: domain::BodyFat) -> Result<domain::BodyFat, String> {
        let r: BodyFat = fetch(
            Request::post("api/body_fat")
                .json(&BodyFat::from(body_fat))
                .expect("serialization failed"),
        )
        .await?;
        Ok(r.into())
    }

    async fn replace_body_fat(&self, body_fat: domain::BodyFat) -> Result<domain::BodyFat, String> {
        let r: BodyFat = fetch(
            Request::put(&format!("api/body_fat/{}", body_fat.date))
                .json(&BodyFatData::from(body_fat))
                .expect("serialization failed"),
        )
        .await?;
        Ok(r.into())
    }

    async fn delete_body_fat(&self, date: NaiveDate) -> Result<NaiveDate, String> {
        fetch_no_content(
            Request::delete(&format!("api/body_fat/{date}"))
                .build()
                .unwrap(),
            date,
        )
        .await
    }
}

impl domain::PeriodRepository for REST {
    async fn read_period(&self) -> Result<Vec<domain::Period>, String> {
        let r: Vec<Period> = fetch(Request::get("api/period").build().unwrap()).await?;
        r.into_iter()
            .map(|p| {
                domain::Period::try_from(p).map_err(|err: domain::IntensityError| err.to_string())
            })
            .collect::<Result<Vec<domain::Period>, String>>()
    }

    async fn create_period(&self, period: domain::Period) -> Result<domain::Period, String> {
        let r: Period = fetch(
            Request::post("api/period")
                .json(&Period::from(period))
                .expect("serialization failed"),
        )
        .await?;
        r.try_into()
            .map_err(|err: domain::IntensityError| err.to_string())
    }

    async fn replace_period(&self, period: domain::Period) -> Result<domain::Period, String> {
        let r: Period = fetch(
            Request::put(&format!("api/period/{}", period.date))
                .json(&json!(&PeriodData::from(period)))
                .expect("serialization failed"),
        )
        .await?;
        r.try_into()
            .map_err(|err: domain::IntensityError| err.to_string())
    }

    async fn delete_period(&self, date: NaiveDate) -> Result<NaiveDate, String> {
        fetch_no_content(
            Request::delete(&format!("api/period/{date}"))
                .build()
                .unwrap(),
            date,
        )
        .await
    }
}

impl domain::ExerciseRepository for REST {
    async fn read_exercises(&self) -> Result<Vec<domain::Exercise>, String> {
        let r: Vec<Exercise> = fetch(Request::get("api/exercises").build().unwrap()).await?;
        r.into_iter()
            .map(|exercise| {
                domain::Exercise::try_from(exercise).map_err(|err: ExerciseError| err.to_string())
            })
            .collect::<Result<Vec<domain::Exercise>, String>>()
    }

    async fn create_exercise(
        &self,
        name: domain::Name,
        muscles: Vec<domain::ExerciseMuscle>,
    ) -> Result<domain::Exercise, String> {
        let r: Exercise = fetch(
            Request::post("api/exercises")
                .json(&json!(&ExerciseData {
                    name: name.to_string(),
                    muscles: muscles.into_iter().map(ExerciseMuscle::from).collect()
                }))
                .expect("serialization failed"),
        )
        .await?;
        r.try_into().map_err(|err: ExerciseError| err.to_string())
    }

    async fn replace_exercise(
        &self,
        exercise: domain::Exercise,
    ) -> Result<domain::Exercise, String> {
        let r: Exercise = fetch(
            Request::put(&format!("api/exercises/{}", exercise.id.as_u128()))
                .json(&Exercise::from(exercise))
                .expect("serialization failed"),
        )
        .await?;
        r.try_into().map_err(|err: ExerciseError| err.to_string())
    }

    async fn delete_exercise(&self, id: domain::ExerciseID) -> Result<domain::ExerciseID, String> {
        fetch_no_content(
            Request::delete(&format!("api/exercises/{}", id.as_u128()))
                .build()
                .unwrap(),
            id,
        )
        .await
    }
}

impl domain::RoutineRepository for REST {
    async fn read_routines(&self) -> Result<Vec<domain::Routine>, String> {
        let r: Vec<Routine> = fetch(Request::get("api/routines").build().unwrap()).await?;
        r.into_iter()
            .map(|routine| {
                domain::Routine::try_from(routine).map_err(|err: domain::NameError| err.to_string())
            })
            .collect::<Result<Vec<domain::Routine>, String>>()
    }

    async fn create_routine(
        &self,
        name: domain::Name,
        sections: Vec<domain::RoutinePart>,
    ) -> Result<domain::Routine, String> {
        let r: Routine = fetch(
            Request::post("api/routines")
                .json(&RoutineData {
                    name: name.to_string(),
                    notes: None,
                    archived: false,
                    sections: sections.into_iter().map(RoutinePart::from).collect(),
                })
                .expect("serialization failed"),
        )
        .await?;
        r.try_into()
            .map_err(|err: domain::NameError| err.to_string())
    }

    async fn modify_routine(
        &self,
        id: domain::RoutineID,
        name: Option<domain::Name>,
        archived: Option<bool>,
        sections: Option<Vec<domain::RoutinePart>>,
    ) -> Result<domain::Routine, String> {
        let mut content = Map::new();
        if let Some(name) = name {
            content.insert("name".into(), json!(name.to_string()));
        }
        if let Some(archived) = archived {
            content.insert("archived".into(), json!(archived));
        }
        if let Some(sections) = sections {
            content.insert(
                "sections".into(),
                json!(
                    sections
                        .into_iter()
                        .map(RoutinePart::from)
                        .collect::<Vec<_>>()
                ),
            );
        }
        let r: Routine = fetch(
            Request::patch(&format!("api/routines/{}", id.as_u128()))
                .json(&content)
                .expect("serialization failed"),
        )
        .await?;
        r.try_into()
            .map_err(|err: domain::NameError| err.to_string())
    }

    async fn delete_routine(&self, id: domain::RoutineID) -> Result<domain::RoutineID, String> {
        fetch_no_content(
            Request::delete(&format!("api/routines/{}", id.as_u128()))
                .build()
                .unwrap(),
            id,
        )
        .await
    }
}

impl domain::TrainingSessionRepository for REST {
    async fn read_training_sessions(&self) -> Result<Vec<domain::TrainingSession>, String> {
        let r: Vec<TrainingSession> = fetch(Request::get("api/workouts").build().unwrap()).await?;
        Ok(r.into_iter().map(domain::TrainingSession::from).collect())
    }

    async fn create_training_session(
        &self,
        routine_id: domain::RoutineID,
        date: NaiveDate,
        notes: String,
        elements: Vec<domain::TrainingSessionElement>,
    ) -> Result<domain::TrainingSession, String> {
        let r: TrainingSession = fetch(
            Request::post("api/workouts")
                .json(&TrainingSessionData {
                    routine_id: if routine_id.is_nil() {
                        None
                    } else {
                        Some(routine_id.as_u128())
                    },
                    date,
                    notes: Some(notes),
                    elements: elements
                        .into_iter()
                        .map(TrainingSessionElement::from)
                        .collect(),
                })
                .expect("serialization failed"),
        )
        .await?;
        Ok(r.into())
    }

    async fn modify_training_session(
        &self,
        id: domain::TrainingSessionID,
        notes: Option<String>,
        elements: Option<Vec<domain::TrainingSessionElement>>,
    ) -> Result<domain::TrainingSession, String> {
        let mut content = Map::new();
        if let Some(notes) = notes {
            content.insert("notes".into(), json!(notes));
        }
        if let Some(elements) = elements {
            content.insert(
                "elements".into(),
                json!(
                    elements
                        .into_iter()
                        .map(TrainingSessionElement::from)
                        .collect::<Vec<_>>()
                ),
            );
        }
        let r: TrainingSession = fetch(
            Request::patch(&format!("api/workouts/{}", id.as_u128()))
                .json(&content)
                .expect("serialization failed"),
        )
        .await?;
        Ok(r.into())
    }

    async fn delete_training_session(
        &self,
        id: domain::TrainingSessionID,
    ) -> Result<domain::TrainingSessionID, String> {
        fetch_no_content(
            Request::delete(&format!("api/workouts/{}", id.as_u128()))
                .build()
                .unwrap(),
            id,
        )
        .await
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: u128,
    pub name: String,
    pub sex: u8,
}

impl From<domain::User> for User {
    fn from(value: domain::User) -> Self {
        Self {
            id: value.id.as_u128(),
            name: value.name.to_string(),
            sex: value.sex as u8,
        }
    }
}

impl TryFrom<User> for domain::User {
    type Error = domain::NameError;

    fn try_from(value: User) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id.into(),
            name: domain::Name::new(&value.name)?,
            sex: value.sex.into(),
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct UserData {
    pub name: String,
    pub sex: u8,
}

impl From<domain::User> for UserData {
    fn from(value: domain::User) -> Self {
        Self {
            name: value.name.to_string(),
            sex: value.sex as u8,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct BodyWeight {
    pub date: NaiveDate,
    pub weight: f32,
}

impl From<domain::BodyWeight> for BodyWeight {
    fn from(value: domain::BodyWeight) -> Self {
        Self {
            date: value.date,
            weight: value.weight,
        }
    }
}

impl From<BodyWeight> for domain::BodyWeight {
    fn from(value: BodyWeight) -> Self {
        Self {
            date: value.date,
            weight: value.weight,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct BodyWeightData {
    pub weight: f32,
}

impl From<domain::BodyWeight> for BodyWeightData {
    fn from(value: domain::BodyWeight) -> Self {
        Self {
            weight: value.weight,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
struct BodyFat {
    pub date: NaiveDate,
    pub chest: Option<u8>,
    pub abdominal: Option<u8>,
    pub thigh: Option<u8>,
    pub tricep: Option<u8>,
    pub subscapular: Option<u8>,
    pub suprailiac: Option<u8>,
    pub midaxillary: Option<u8>,
}

impl From<domain::BodyFat> for BodyFat {
    fn from(value: domain::BodyFat) -> Self {
        Self {
            date: value.date,
            chest: value.chest,
            abdominal: value.abdominal,
            thigh: value.thigh,
            tricep: value.tricep,
            subscapular: value.subscapular,
            suprailiac: value.suprailiac,
            midaxillary: value.midaxillary,
        }
    }
}

impl From<BodyFat> for domain::BodyFat {
    fn from(value: BodyFat) -> Self {
        Self {
            date: value.date,
            chest: value.chest,
            abdominal: value.abdominal,
            thigh: value.thigh,
            tricep: value.tricep,
            subscapular: value.subscapular,
            suprailiac: value.suprailiac,
            midaxillary: value.midaxillary,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
struct BodyFatData {
    pub chest: Option<u8>,
    pub abdominal: Option<u8>,
    pub thigh: Option<u8>,
    pub tricep: Option<u8>,
    pub subscapular: Option<u8>,
    pub suprailiac: Option<u8>,
    pub midaxillary: Option<u8>,
}

impl From<domain::BodyFat> for BodyFatData {
    fn from(value: domain::BodyFat) -> Self {
        Self {
            chest: value.chest,
            abdominal: value.abdominal,
            thigh: value.thigh,
            tricep: value.tricep,
            subscapular: value.subscapular,
            suprailiac: value.suprailiac,
            midaxillary: value.midaxillary,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Period {
    pub date: NaiveDate,
    pub intensity: u8,
}

impl From<domain::Period> for Period {
    fn from(value: domain::Period) -> Self {
        Self {
            date: value.date,
            intensity: value.intensity as u8,
        }
    }
}

impl TryFrom<Period> for domain::Period {
    type Error = domain::IntensityError;

    fn try_from(value: Period) -> Result<Self, Self::Error> {
        Ok(Self {
            date: value.date,
            intensity: domain::Intensity::try_from(value.intensity)?,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PeriodData {
    pub intensity: u8,
}

impl From<domain::Period> for PeriodData {
    fn from(value: domain::Period) -> Self {
        Self {
            intensity: value.intensity as u8,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Exercise {
    pub id: u128,
    pub name: String,
    pub muscles: Vec<ExerciseMuscle>,
}

impl From<domain::Exercise> for Exercise {
    fn from(value: domain::Exercise) -> Self {
        Self {
            id: value.id.as_u128(),
            name: value.name.to_string(),
            muscles: value
                .muscles
                .into_iter()
                .map(ExerciseMuscle::from)
                .collect(),
        }
    }
}

impl TryFrom<Exercise> for domain::Exercise {
    type Error = ExerciseError;

    fn try_from(value: Exercise) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id.into(),
            name: domain::Name::new(&value.name)?,
            muscles: value
                .muscles
                .into_iter()
                .map(|m| domain::ExerciseMuscle::try_from(m).map_err(From::from))
                .collect::<Result<Vec<domain::ExerciseMuscle>, ExerciseError>>()?,
        })
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum ExerciseError {
    #[error(transparent)]
    InvalidName(#[from] domain::NameError),
    #[error(transparent)]
    InvalidMuscle(#[from] domain::MuscleIDError),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ExerciseData {
    pub name: String,
    pub muscles: Vec<ExerciseMuscle>,
}

impl From<domain::Exercise> for ExerciseData {
    fn from(value: domain::Exercise) -> Self {
        Self {
            name: value.name.to_string(),
            muscles: value
                .muscles
                .into_iter()
                .map(ExerciseMuscle::from)
                .collect(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ExerciseMuscle {
    pub muscle_id: u8,
    pub stimulus: u32,
}

impl From<domain::ExerciseMuscle> for ExerciseMuscle {
    fn from(value: domain::ExerciseMuscle) -> Self {
        Self {
            muscle_id: value.muscle_id as u8,
            stimulus: *value.stimulus,
        }
    }
}

impl TryFrom<ExerciseMuscle> for domain::ExerciseMuscle {
    type Error = domain::MuscleIDError;

    fn try_from(value: ExerciseMuscle) -> Result<Self, Self::Error> {
        let muscle_id = domain::MuscleID::try_from(value.muscle_id)?;
        Ok(Self {
            muscle_id,
            stimulus: domain::Stimulus::new(value.stimulus).unwrap_or(domain::Stimulus::PRIMARY),
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Routine {
    pub id: u128,
    pub name: String,
    pub notes: Option<String>,
    pub archived: bool,
    pub sections: Vec<RoutinePart>,
}

impl From<domain::Routine> for Routine {
    fn from(value: domain::Routine) -> Self {
        Self {
            id: value.id.as_u128(),
            name: value.name.to_string(),
            notes: Some(value.notes),
            archived: value.archived,
            sections: value.sections.into_iter().map(RoutinePart::from).collect(),
        }
    }
}

impl TryFrom<Routine> for domain::Routine {
    type Error = domain::NameError;

    fn try_from(value: Routine) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id.into(),
            name: domain::Name::new(&value.name)?,
            notes: value.notes.unwrap_or_default(),
            archived: value.archived,
            sections: value
                .sections
                .into_iter()
                .map(domain::RoutinePart::from)
                .collect(),
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct RoutineData {
    pub name: String,
    pub notes: Option<String>,
    pub archived: bool,
    pub sections: Vec<RoutinePart>,
}

impl From<domain::Routine> for RoutineData {
    fn from(value: domain::Routine) -> Self {
        Self {
            name: value.name.to_string(),
            notes: Some(value.notes),
            archived: value.archived,
            sections: value.sections.into_iter().map(RoutinePart::from).collect(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum RoutinePart {
    RoutineSection {
        rounds: u32,
        parts: Vec<RoutinePart>,
    },
    RoutineActivity {
        exercise_id: Option<u64>,
        reps: u32,
        time: u32,
        weight: f32,
        rpe: f32,
        automatic: bool,
    },
}

impl From<domain::RoutinePart> for RoutinePart {
    fn from(value: domain::RoutinePart) -> Self {
        match value {
            domain::RoutinePart::RoutineSection { rounds, parts } => RoutinePart::RoutineSection {
                rounds,
                parts: parts.into_iter().map(RoutinePart::from).collect(),
            },
            domain::RoutinePart::RoutineActivity {
                exercise_id,
                reps,
                time,
                weight,
                rpe,
                automatic,
            } => RoutinePart::RoutineActivity {
                #[allow(clippy::cast_possible_truncation)]
                exercise_id: if exercise_id.is_nil() {
                    None
                } else {
                    Some(exercise_id.as_u128() as u64)
                },
                reps: u32::from(reps),
                time: u32::from(time),
                weight: f32::from(weight),
                rpe: f32::from(rpe),
                automatic,
            },
        }
    }
}

impl From<RoutinePart> for domain::RoutinePart {
    fn from(value: RoutinePart) -> Self {
        match value {
            RoutinePart::RoutineSection { rounds, parts } => domain::RoutinePart::RoutineSection {
                rounds,
                parts: parts.into_iter().map(domain::RoutinePart::from).collect(),
            },
            RoutinePart::RoutineActivity {
                exercise_id,
                reps,
                time,
                weight,
                rpe,
                automatic,
            } => domain::RoutinePart::RoutineActivity {
                exercise_id: exercise_id
                    .map(u128::from)
                    .map(domain::ExerciseID::from)
                    .unwrap_or_default(),
                reps: domain::Reps::new(reps).unwrap_or_default(),
                time: domain::Time::new(time).unwrap_or_default(),
                weight: domain::Weight::new(weight).unwrap_or_default(),
                rpe: domain::RPE::new(rpe).unwrap_or_default(),
                automatic,
            },
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct TrainingSession {
    pub id: u128,
    pub routine_id: Option<u128>,
    pub date: NaiveDate,
    pub notes: Option<String>,
    pub elements: Vec<TrainingSessionElement>,
}

impl From<domain::TrainingSession> for TrainingSession {
    fn from(value: domain::TrainingSession) -> Self {
        Self {
            id: value.id.as_u128(),
            routine_id: if value.routine_id.is_nil() {
                None
            } else {
                Some(value.routine_id.as_u128())
            },
            date: value.date,
            notes: Some(value.notes),
            elements: value
                .elements
                .into_iter()
                .map(TrainingSessionElement::from)
                .collect(),
        }
    }
}

impl From<TrainingSession> for domain::TrainingSession {
    fn from(value: TrainingSession) -> Self {
        Self {
            id: value.id.into(),
            routine_id: value.routine_id.unwrap_or_default().into(),
            date: value.date,
            notes: value.notes.unwrap_or_default(),
            elements: value
                .elements
                .into_iter()
                .map(domain::TrainingSessionElement::from)
                .collect(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct TrainingSessionData {
    pub routine_id: Option<u128>,
    pub date: NaiveDate,
    pub notes: Option<String>,
    pub elements: Vec<TrainingSessionElement>,
}

impl From<domain::TrainingSession> for TrainingSessionData {
    fn from(value: domain::TrainingSession) -> Self {
        Self {
            routine_id: if value.routine_id.is_nil() {
                None
            } else {
                Some(value.routine_id.as_u128())
            },
            date: value.date,
            notes: Some(value.notes),
            elements: value
                .elements
                .into_iter()
                .map(TrainingSessionElement::from)
                .collect(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum TrainingSessionElement {
    Set {
        exercise_id: u64,
        reps: Option<u32>,
        time: Option<u32>,
        weight: Option<f32>,
        rpe: Option<f32>,
        target_reps: Option<u32>,
        target_time: Option<u32>,
        target_weight: Option<f32>,
        target_rpe: Option<f32>,
        automatic: bool,
    },
    Rest {
        target_time: Option<u32>,
        automatic: bool,
    },
}

impl From<domain::TrainingSessionElement> for TrainingSessionElement {
    fn from(value: domain::TrainingSessionElement) -> Self {
        match value {
            domain::TrainingSessionElement::Set {
                exercise_id,
                reps,
                time,
                weight,
                rpe,
                target_reps,
                target_time,
                target_weight,
                target_rpe,
                automatic,
            } => TrainingSessionElement::Set {
                #[allow(clippy::cast_possible_truncation)]
                exercise_id: exercise_id.as_u128() as u64,
                reps: reps.map(From::from),
                time: time.map(From::from),
                weight: weight.map(From::from),
                rpe: rpe.map(From::from),
                target_reps: target_reps.map(From::from),
                target_time: target_time.map(From::from),
                target_weight: target_weight.map(From::from),
                target_rpe: target_rpe.map(From::from),
                automatic,
            },
            domain::TrainingSessionElement::Rest {
                target_time,
                automatic,
            } => TrainingSessionElement::Rest {
                target_time: target_time.map(From::from),
                automatic,
            },
        }
    }
}

impl From<TrainingSessionElement> for domain::TrainingSessionElement {
    fn from(value: TrainingSessionElement) -> Self {
        match value {
            TrainingSessionElement::Set {
                exercise_id,
                reps,
                time,
                weight,
                rpe,
                target_reps,
                target_time,
                target_weight,
                target_rpe,
                automatic,
            } => domain::TrainingSessionElement::Set {
                exercise_id: u128::from(exercise_id).into(),
                reps: reps.map(|r| domain::Reps::new(r).unwrap_or_default()),
                time: time.map(|t| domain::Time::new(t).unwrap_or_default()),
                weight: weight.map(|t| domain::Weight::new(t).unwrap_or_default()),
                rpe: rpe.and_then(|rpe| domain::RPE::new(rpe).ok()),
                target_reps: target_reps.map(|r| domain::Reps::new(r).unwrap_or_default()),
                target_time: target_time.map(|t| domain::Time::new(t).unwrap_or_default()),
                target_weight: target_weight.map(|t| domain::Weight::new(t).unwrap_or_default()),
                target_rpe: target_rpe.and_then(|rpe| domain::RPE::new(rpe).ok()),
                automatic,
            },
            TrainingSessionElement::Rest {
                target_time,
                automatic,
            } => domain::TrainingSessionElement::Rest {
                target_time: target_time.map(|t| domain::Time::new(t).unwrap_or_default()),
                automatic,
            },
        }
    }
}

async fn fetch<T>(request: Request) -> Result<T, String>
where
    T: 'static + for<'de> serde::Deserialize<'de>,
{
    match request.send().await {
        Ok(response) => {
            if response.ok() {
                match response.json::<T>().await {
                    Ok(data) => Ok(data),
                    Err(error) => Err(format!("deserialization failed: {error:?}")),
                }
            } else {
                Err(format!("{} {}", response.status(), response.status_text()))
            }
        }
        Err(_) => Err("no connection".into()),
    }
}

async fn fetch_no_content<T>(request: Request, result: T) -> Result<T, String> {
    match request.send().await {
        Ok(response) => {
            if response.ok() {
                Ok(result)
            } else {
                Err(format!("{} {}", response.status(), response.status_text()))
            }
        }
        Err(_) => Err("no connection".into()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Local, NaiveDate};
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use serde_json::json;

    use super::*;

    static TODAY: std::sync::LazyLock<NaiveDate> =
        std::sync::LazyLock::new(|| Local::now().date_naive());

    static ROUTINE: std::sync::LazyLock<domain::Routine> =
        std::sync::LazyLock::new(|| domain::Routine {
            id: 1.into(),
            name: domain::Name::new("A").unwrap(),
            notes: String::from("B"),
            archived: false,
            sections: vec![
                domain::RoutinePart::RoutineSection {
                    rounds: 2,
                    parts: vec![
                        domain::RoutinePart::RoutineActivity {
                            exercise_id: 1.into(),
                            reps: domain::Reps::new(10).unwrap(),
                            time: domain::Time::new(2).unwrap(),
                            weight: domain::Weight::new(30.0).unwrap(),
                            rpe: domain::RPE::TEN,
                            automatic: false,
                        },
                        domain::RoutinePart::RoutineActivity {
                            exercise_id: domain::ExerciseID::nil(),
                            reps: domain::Reps::default(),
                            time: domain::Time::new(60).unwrap(),
                            weight: domain::Weight::default(),
                            rpe: domain::RPE::ZERO,
                            automatic: true,
                        },
                    ],
                },
                domain::RoutinePart::RoutineSection {
                    rounds: 2,
                    parts: vec![
                        domain::RoutinePart::RoutineActivity {
                            exercise_id: 2.into(),
                            reps: domain::Reps::new(10).unwrap(),
                            time: domain::Time::default(),
                            weight: domain::Weight::default(),
                            rpe: domain::RPE::ZERO,
                            automatic: false,
                        },
                        domain::RoutinePart::RoutineActivity {
                            exercise_id: domain::ExerciseID::nil(),
                            reps: domain::Reps::default(),
                            time: domain::Time::new(30).unwrap(),
                            weight: domain::Weight::default(),
                            rpe: domain::RPE::ZERO,
                            automatic: true,
                        },
                    ],
                },
            ],
        });

    static TRAINING_SESSION: std::sync::LazyLock<domain::TrainingSession> =
        std::sync::LazyLock::new(|| domain::TrainingSession {
            id: 1.into(),
            routine_id: 2.into(),
            date: *TODAY - Duration::days(10),
            notes: String::from("A"),
            elements: vec![
                domain::TrainingSessionElement::Set {
                    exercise_id: 1.into(),
                    reps: Some(domain::Reps::new(10).unwrap()),
                    time: Some(domain::Time::new(3).unwrap()),
                    weight: Some(domain::Weight::new(30.0).unwrap()),
                    rpe: Some(domain::RPE::EIGHT),
                    target_reps: Some(domain::Reps::new(8).unwrap()),
                    target_time: Some(domain::Time::new(4).unwrap()),
                    target_weight: Some(domain::Weight::new(40.0).unwrap()),
                    target_rpe: Some(domain::RPE::NINE),
                    automatic: false,
                },
                domain::TrainingSessionElement::Rest {
                    target_time: Some(domain::Time::new(60).unwrap()),
                    automatic: true,
                },
                domain::TrainingSessionElement::Set {
                    exercise_id: 2.into(),
                    reps: Some(domain::Reps::new(5).unwrap()),
                    time: Some(domain::Time::new(4).unwrap()),
                    weight: None,
                    rpe: Some(domain::RPE::FOUR),
                    target_reps: None,
                    target_time: None,
                    target_weight: None,
                    target_rpe: None,
                    automatic: false,
                },
                domain::TrainingSessionElement::Rest {
                    target_time: Some(domain::Time::new(60).unwrap()),
                    automatic: true,
                },
                domain::TrainingSessionElement::Set {
                    exercise_id: 2.into(),
                    reps: None,
                    time: Some(domain::Time::new(60).unwrap()),
                    weight: None,
                    rpe: None,
                    target_reps: None,
                    target_time: None,
                    target_weight: None,
                    target_rpe: None,
                    automatic: false,
                },
                domain::TrainingSessionElement::Rest {
                    target_time: Some(domain::Time::new(60).unwrap()),
                    automatic: true,
                },
            ],
        });

    #[test]
    fn test_user_try_from() {
        let user = domain::User {
            id: (2u128.pow(64) - 1).into(),
            name: domain::Name::new("A").unwrap(),
            sex: domain::Sex::FEMALE,
        };
        assert_eq!(domain::User::try_from(User::from(user.clone())), Ok(user));
    }

    #[rstest]
    #[case(domain::Sex::FEMALE)]
    #[case(domain::Sex::MALE)]
    fn test_user_serde(#[case] sex: domain::Sex) {
        let obj: User = domain::User {
            id: (2u128.pow(64) - 1).into(),
            name: domain::Name::new("A").unwrap(),
            sex,
        }
        .into();
        let serialized = json!(obj);
        let deserialized: User = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, obj);
    }

    #[test]
    fn test_body_weight_from() {
        let body_weight = domain::BodyWeight {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            weight: 80.0,
        };
        assert_eq!(
            domain::BodyWeight::from(BodyWeight::from(body_weight.clone())),
            body_weight
        );
    }

    #[test]
    fn test_body_weight_serde() {
        let obj: BodyWeight = domain::BodyWeight {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            weight: 80.0,
        }
        .into();
        let serialized = json!(obj);
        let deserialized: BodyWeight = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, obj);
    }

    #[test]
    fn test_body_fat_from() {
        let body_fat = domain::BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: Some(1),
            abdominal: Some(2),
            thigh: Some(3),
            tricep: Some(4),
            subscapular: Some(5),
            suprailiac: Some(6),
            midaxillary: Some(7),
        };
        assert_eq!(
            domain::BodyFat::from(BodyFat::from(body_fat.clone())),
            body_fat
        );
    }

    #[test]
    fn test_body_fat_serde() {
        let obj = domain::BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: Some(1),
            abdominal: Some(2),
            thigh: Some(3),
            tricep: Some(4),
            subscapular: Some(5),
            suprailiac: Some(6),
            midaxillary: Some(7),
        }
        .into();
        let serialized = json!(obj);
        let deserialized: BodyFat = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, obj);
    }

    #[test]
    fn test_period_try_from() {
        let period = domain::Period {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            intensity: domain::Intensity::Light,
        };
        assert_eq!(
            domain::Period::try_from(Period::from(period.clone())),
            Ok(period)
        );
    }

    #[test]
    fn test_period_serde() {
        let obj = domain::Period {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            intensity: domain::Intensity::Light,
        }
        .into();
        let serialized = json!(obj);
        let deserialized: Period = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, obj);
    }

    #[test]
    fn test_exercise_try_from() {
        let exercise = domain::Exercise {
            id: 1.into(),
            name: domain::Name::new("A").unwrap(),
            muscles: vec![domain::ExerciseMuscle {
                muscle_id: domain::MuscleID::Abs,
                stimulus: domain::Stimulus::PRIMARY,
            }],
        };
        assert_eq!(
            domain::Exercise::try_from(Exercise::from(exercise.clone())),
            Ok(exercise)
        );
    }

    #[test]
    fn test_exercise_serde() {
        let obj = domain::Exercise {
            id: 1.into(),
            name: domain::Name::new("A").unwrap(),
            muscles: vec![domain::ExerciseMuscle {
                muscle_id: domain::MuscleID::Abs,
                stimulus: domain::Stimulus::PRIMARY,
            }],
        }
        .into();
        let serialized = json!(obj);
        let deserialized: Exercise = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, obj);
    }

    #[test]
    fn test_routine_try_from() {
        assert_eq!(
            domain::Routine::try_from(Routine::from(ROUTINE.clone())),
            Ok(ROUTINE.clone())
        );
    }

    #[test]
    fn test_routine_serde() {
        let obj = Routine::from(ROUTINE.clone());
        let serialized = json!(obj);
        let deserialized: Routine = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, obj);
    }

    #[test]
    fn test_training_session_from() {
        assert_eq!(
            domain::TrainingSession::from(TrainingSession::from(TRAINING_SESSION.clone())),
            TRAINING_SESSION.clone()
        );
    }

    #[test]
    fn test_training_session_serde() {
        let obj = TrainingSession::from(TRAINING_SESSION.clone());
        let serialized = json!(obj);
        let deserialized: TrainingSession = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, obj);
    }
}
