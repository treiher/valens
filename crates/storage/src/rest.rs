use chrono::NaiveDate;
use gloo_net;
use serde_json::{Map, json};
use thiserror::Error;
use valens_domain as domain;

#[derive(Clone)]
pub struct REST<S: SendRequest> {
    pub sender: S,
}

impl REST<GlooNetSendRequest> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sender: GlooNetSendRequest,
        }
    }
}

impl Default for REST<GlooNetSendRequest> {
    #[must_use]
    fn default() -> Self {
        Self::new()
    }
}

impl<S: SendRequest> REST<S> {
    async fn fetch<T>(&self, request: gloo_net::http::Request) -> Result<T, FetchError>
    where
        T: 'static + for<'de> serde::Deserialize<'de>,
    {
        match self.sender.send_request(request).await {
            Ok(response) => {
                if response.ok() {
                    match response.json::<T>().await {
                        Ok(data) => Ok(data),
                        Err(err) => Err(anyhow::anyhow!(err)
                            .context("deserialization failed")
                            .into()),
                    }
                } else {
                    Err(FetchError::Response {
                        status: response.status(),
                        status_text: response.status_text(),
                        body: response.text().await.ok(),
                    })
                }
            }
            Err(gloo_net::Error::JsError(_) | gloo_net::Error::GlooError(_)) => {
                Err(FetchError::NoConnection)
            }
            Err(err) => Err(anyhow::anyhow!(err).into()),
        }
    }

    async fn fetch_no_content<T>(
        &self,
        request: gloo_net::http::Request,
        result: T,
    ) -> Result<T, FetchError> {
        match self.sender.send_request(request).await {
            Ok(response) => {
                if response.ok() {
                    Ok(result)
                } else {
                    Err(FetchError::Response {
                        status: response.status(),
                        status_text: response.status_text(),
                        body: response.text().await.ok(),
                    })
                }
            }
            Err(gloo_net::Error::JsError(_) | gloo_net::Error::GlooError(_)) => {
                Err(FetchError::NoConnection)
            }
            Err(err) => Err(anyhow::anyhow!(err).into()),
        }
    }
}

#[derive(Error, Debug)]
enum FetchError {
    #[error("no connection")]
    NoConnection,
    #[error("{status} {status_text}{}", body.as_ref().map(|body| format!(": {body}")).unwrap_or_default())]
    Response {
        status: u16,
        status_text: String,
        body: Option<String>,
    },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<FetchError> for domain::StorageError {
    fn from(value: FetchError) -> Self {
        match value {
            FetchError::NoConnection => domain::StorageError::NoConnection,
            FetchError::Response { .. } => domain::StorageError::Other(value.into()),
            FetchError::Other(err) => domain::StorageError::Other(err.into()),
        }
    }
}

impl From<FetchError> for domain::ReadError {
    fn from(value: FetchError) -> Self {
        domain::ReadError::Storage(value.into())
    }
}

impl From<FetchError> for domain::CreateError {
    fn from(value: FetchError) -> Self {
        match value {
            FetchError::Response { status: 409, .. } => domain::CreateError::Conflict,
            _ => domain::CreateError::Storage(value.into()),
        }
    }
}

impl From<FetchError> for domain::UpdateError {
    fn from(value: FetchError) -> Self {
        match value {
            FetchError::Response { status: 409, .. } => domain::UpdateError::Conflict,
            _ => domain::UpdateError::Storage(value.into()),
        }
    }
}

impl From<FetchError> for domain::DeleteError {
    fn from(value: FetchError) -> Self {
        domain::DeleteError::Storage(value.into())
    }
}

impl<S: SendRequest> domain::SessionRepository for REST<S> {
    async fn request_session(
        &self,
        user_id: domain::UserID,
    ) -> Result<domain::User, domain::ReadError> {
        let r: User = self
            .fetch(
                gloo_net::http::Request::post("api/session")
                    .json(&json!({ "id": user_id.as_u128() }))
                    .expect("serialization failed"),
            )
            .await?;
        Ok(r.try_into().map_err(Box::from)?)
    }

    async fn initialize_session(&self) -> Result<domain::User, domain::ReadError> {
        let r: User = self
            .fetch(gloo_net::http::Request::get("api/session").build().unwrap())
            .await?;
        Ok(r.try_into().map_err(Box::from)?)
    }

    async fn delete_session(&self) -> Result<(), domain::DeleteError> {
        Ok(self
            .fetch_no_content(
                gloo_net::http::Request::delete("api/session")
                    .build()
                    .unwrap(),
                (),
            )
            .await?)
    }
}

impl<S: SendRequest> domain::VersionRepository for REST<S> {
    async fn read_version(&self) -> Result<String, domain::ReadError> {
        Ok(self
            .fetch(gloo_net::http::Request::get("api/version").build().unwrap())
            .await?)
    }
}

impl<S: SendRequest> domain::UserRepository for REST<S> {
    async fn read_users(&self) -> Result<Vec<domain::User>, domain::ReadError> {
        let r: Vec<User> = self
            .fetch(gloo_net::http::Request::get("api/users").build().unwrap())
            .await?;
        Ok(r.into_iter()
            .map(|user| domain::User::try_from(user).map_err(Box::from))
            .collect::<Result<Vec<domain::User>, _>>()?)
    }

    async fn create_user(
        &self,
        name: domain::Name,
        sex: domain::Sex,
    ) -> Result<domain::User, domain::CreateError> {
        let r: User = self
            .fetch(
                gloo_net::http::Request::post("api/users")
                    .json(&UserData {
                        name: name.to_string(),
                        sex: sex as u8,
                    })
                    .expect("serialization failed"),
            )
            .await?;
        Ok(r.try_into().map_err(Box::from)?)
    }

    async fn replace_user(&self, user: domain::User) -> Result<domain::User, domain::UpdateError> {
        let r: User = self
            .fetch(
                gloo_net::http::Request::put(&format!("api/users/{}", user.id.as_u128()))
                    .json(&UserData::from(user))
                    .expect("serialization failed"),
            )
            .await?;
        Ok(r.try_into().map_err(Box::from)?)
    }

    async fn delete_user(&self, id: domain::UserID) -> Result<domain::UserID, domain::DeleteError> {
        Ok(self
            .fetch_no_content(
                gloo_net::http::Request::delete(&format!("api/users/{}", id.as_u128()))
                    .build()
                    .unwrap(),
                id,
            )
            .await?)
    }
}

impl<S: SendRequest> domain::BodyWeightRepository for REST<S> {
    async fn sync_body_weight(&self) -> Result<Vec<domain::BodyWeight>, domain::SyncError> {
        Ok(self.read_body_weight().await?)
    }

    async fn read_body_weight(&self) -> Result<Vec<domain::BodyWeight>, domain::ReadError> {
        let r: Vec<BodyWeight> = self
            .fetch(
                gloo_net::http::Request::get("api/body_weight")
                    .build()
                    .unwrap(),
            )
            .await?;
        Ok(r.into_iter().map(domain::BodyWeight::from).collect())
    }

    async fn create_body_weight(
        &self,
        body_weight: domain::BodyWeight,
    ) -> Result<domain::BodyWeight, domain::CreateError> {
        let r: BodyWeight = self
            .fetch(
                gloo_net::http::Request::post("api/body_weight")
                    .json(&BodyWeight::from(body_weight))
                    .expect("serialization failed"),
            )
            .await?;
        Ok(r.into())
    }

    async fn replace_body_weight(
        &self,
        body_weight: domain::BodyWeight,
    ) -> Result<domain::BodyWeight, domain::UpdateError> {
        let r: BodyWeight = self
            .fetch(
                gloo_net::http::Request::put(&format!("api/body_weight/{}", body_weight.date))
                    .json(&json!(&BodyWeightData::from(body_weight)))
                    .expect("serialization failed"),
            )
            .await?;
        Ok(r.into())
    }

    async fn delete_body_weight(&self, date: NaiveDate) -> Result<NaiveDate, domain::DeleteError> {
        Ok(self
            .fetch_no_content(
                gloo_net::http::Request::delete(&format!("api/body_weight/{date}"))
                    .build()
                    .unwrap(),
                date,
            )
            .await?)
    }
}

impl<S: SendRequest> domain::BodyFatRepository for REST<S> {
    async fn sync_body_fat(&self) -> Result<Vec<domain::BodyFat>, domain::SyncError> {
        Ok(self.read_body_fat().await?)
    }

    async fn read_body_fat(&self) -> Result<Vec<domain::BodyFat>, domain::ReadError> {
        let r: Vec<BodyFat> = self
            .fetch(
                gloo_net::http::Request::get("api/body_fat")
                    .build()
                    .unwrap(),
            )
            .await?;
        Ok(r.into_iter().map(domain::BodyFat::from).collect())
    }

    async fn create_body_fat(
        &self,
        body_fat: domain::BodyFat,
    ) -> Result<domain::BodyFat, domain::CreateError> {
        let r: BodyFat = self
            .fetch(
                gloo_net::http::Request::post("api/body_fat")
                    .json(&BodyFat::from(body_fat))
                    .expect("serialization failed"),
            )
            .await?;
        Ok(r.into())
    }

    async fn replace_body_fat(
        &self,
        body_fat: domain::BodyFat,
    ) -> Result<domain::BodyFat, domain::UpdateError> {
        let r: BodyFat = self
            .fetch(
                gloo_net::http::Request::put(&format!("api/body_fat/{}", body_fat.date))
                    .json(&BodyFatData::from(body_fat))
                    .expect("serialization failed"),
            )
            .await?;
        Ok(r.into())
    }

    async fn delete_body_fat(&self, date: NaiveDate) -> Result<NaiveDate, domain::DeleteError> {
        Ok(self
            .fetch_no_content(
                gloo_net::http::Request::delete(&format!("api/body_fat/{date}"))
                    .build()
                    .unwrap(),
                date,
            )
            .await?)
    }
}

impl<S: SendRequest> domain::PeriodRepository for REST<S> {
    async fn sync_period(&self) -> Result<Vec<domain::Period>, domain::SyncError> {
        Ok(self.read_period().await?)
    }

    async fn read_period(&self) -> Result<Vec<domain::Period>, domain::ReadError> {
        let r: Vec<Period> = self
            .fetch(gloo_net::http::Request::get("api/period").build().unwrap())
            .await?;
        Ok(r.into_iter()
            .map(|p| domain::Period::try_from(p).map_err(Box::from))
            .collect::<Result<Vec<domain::Period>, _>>()?)
    }

    async fn create_period(
        &self,
        period: domain::Period,
    ) -> Result<domain::Period, domain::CreateError> {
        let r: Period = self
            .fetch(
                gloo_net::http::Request::post("api/period")
                    .json(&Period::from(period))
                    .expect("serialization failed"),
            )
            .await?;
        Ok(r.try_into().map_err(Box::from)?)
    }

    async fn replace_period(
        &self,
        period: domain::Period,
    ) -> Result<domain::Period, domain::UpdateError> {
        let r: Period = self
            .fetch(
                gloo_net::http::Request::put(&format!("api/period/{}", period.date))
                    .json(&json!(&PeriodData::from(period)))
                    .expect("serialization failed"),
            )
            .await?;
        Ok(r.try_into().map_err(Box::from)?)
    }

    async fn delete_period(&self, date: NaiveDate) -> Result<NaiveDate, domain::DeleteError> {
        Ok(self
            .fetch_no_content(
                gloo_net::http::Request::delete(&format!("api/period/{date}"))
                    .build()
                    .unwrap(),
                date,
            )
            .await?)
    }
}

impl<S: SendRequest> domain::ExerciseRepository for REST<S> {
    async fn sync_exercises(&self) -> Result<Vec<domain::Exercise>, domain::SyncError> {
        Ok(self.read_exercises().await?)
    }

    async fn read_exercises(&self) -> Result<Vec<domain::Exercise>, domain::ReadError> {
        let r: Vec<Exercise> = self
            .fetch(
                gloo_net::http::Request::get("api/exercises")
                    .build()
                    .unwrap(),
            )
            .await?;
        Ok(r.into_iter()
            .map(|exercise| domain::Exercise::try_from(exercise).map_err(Box::from))
            .collect::<Result<Vec<domain::Exercise>, _>>()?)
    }

    async fn create_exercise(
        &self,
        name: domain::Name,
        muscles: Vec<domain::ExerciseMuscle>,
    ) -> Result<domain::Exercise, domain::CreateError> {
        let r: Exercise = self
            .fetch(
                gloo_net::http::Request::post("api/exercises")
                    .json(&json!(&ExerciseData {
                        name: name.to_string(),
                        muscles: muscles.into_iter().map(ExerciseMuscle::from).collect()
                    }))
                    .expect("serialization failed"),
            )
            .await?;
        Ok(r.try_into().map_err(Box::from)?)
    }

    async fn replace_exercise(
        &self,
        exercise: domain::Exercise,
    ) -> Result<domain::Exercise, domain::UpdateError> {
        let r: Exercise = self
            .fetch(
                gloo_net::http::Request::put(&format!("api/exercises/{}", exercise.id.as_u128()))
                    .json(&Exercise::from(exercise))
                    .expect("serialization failed"),
            )
            .await?;
        Ok(r.try_into().map_err(Box::from)?)
    }

    async fn delete_exercise(
        &self,
        id: domain::ExerciseID,
    ) -> Result<domain::ExerciseID, domain::DeleteError> {
        Ok(self
            .fetch_no_content(
                gloo_net::http::Request::delete(&format!("api/exercises/{}", id.as_u128()))
                    .build()
                    .unwrap(),
                id,
            )
            .await?)
    }
}

impl<S: SendRequest> domain::RoutineRepository for REST<S> {
    async fn sync_routines(&self) -> Result<Vec<domain::Routine>, domain::SyncError> {
        Ok(self.read_routines().await?)
    }

    async fn read_routines(&self) -> Result<Vec<domain::Routine>, domain::ReadError> {
        let r: Vec<Routine> = self
            .fetch(
                gloo_net::http::Request::get("api/routines")
                    .build()
                    .unwrap(),
            )
            .await?;
        Ok(r.into_iter()
            .map(|routine| domain::Routine::try_from(routine).map_err(Box::from))
            .collect::<Result<Vec<domain::Routine>, _>>()?)
    }

    async fn create_routine(
        &self,
        name: domain::Name,
        sections: Vec<domain::RoutinePart>,
    ) -> Result<domain::Routine, domain::CreateError> {
        let r: Routine = self
            .fetch(
                gloo_net::http::Request::post("api/routines")
                    .json(&RoutineData {
                        name: name.to_string(),
                        notes: None,
                        archived: false,
                        sections: sections.into_iter().map(RoutinePart::from).collect(),
                    })
                    .expect("serialization failed"),
            )
            .await?;
        Ok(r.try_into().map_err(Box::from)?)
    }

    async fn modify_routine(
        &self,
        id: domain::RoutineID,
        name: Option<domain::Name>,
        archived: Option<bool>,
        sections: Option<Vec<domain::RoutinePart>>,
    ) -> Result<domain::Routine, domain::UpdateError> {
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
        let r: Routine = self
            .fetch(
                gloo_net::http::Request::patch(&format!("api/routines/{}", id.as_u128()))
                    .json(&content)
                    .expect("serialization failed"),
            )
            .await?;
        Ok(r.try_into().map_err(Box::from)?)
    }

    async fn delete_routine(
        &self,
        id: domain::RoutineID,
    ) -> Result<domain::RoutineID, domain::DeleteError> {
        Ok(self
            .fetch_no_content(
                gloo_net::http::Request::delete(&format!("api/routines/{}", id.as_u128()))
                    .build()
                    .unwrap(),
                id,
            )
            .await?)
    }
}

impl<S: SendRequest> domain::TrainingSessionRepository for REST<S> {
    async fn sync_training_sessions(
        &self,
    ) -> Result<Vec<domain::TrainingSession>, domain::SyncError> {
        Ok(self.read_training_sessions().await?)
    }

    async fn read_training_sessions(
        &self,
    ) -> Result<Vec<domain::TrainingSession>, domain::ReadError> {
        let r: Vec<TrainingSession> = self
            .fetch(
                gloo_net::http::Request::get("api/workouts")
                    .build()
                    .unwrap(),
            )
            .await?;
        Ok(r.into_iter().map(domain::TrainingSession::from).collect())
    }

    async fn create_training_session(
        &self,
        routine_id: domain::RoutineID,
        date: NaiveDate,
        notes: String,
        elements: Vec<domain::TrainingSessionElement>,
    ) -> Result<domain::TrainingSession, domain::CreateError> {
        let r: TrainingSession = self
            .fetch(
                gloo_net::http::Request::post("api/workouts")
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
    ) -> Result<domain::TrainingSession, domain::UpdateError> {
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
        let r: TrainingSession = self
            .fetch(
                gloo_net::http::Request::patch(&format!("api/workouts/{}", id.as_u128()))
                    .json(&content)
                    .expect("serialization failed"),
            )
            .await?;
        Ok(r.into())
    }

    async fn delete_training_session(
        &self,
        id: domain::TrainingSessionID,
    ) -> Result<domain::TrainingSessionID, domain::DeleteError> {
        Ok(self
            .fetch_no_content(
                gloo_net::http::Request::delete(&format!("api/workouts/{}", id.as_u128()))
                    .build()
                    .unwrap(),
                id,
            )
            .await?)
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
pub struct BodyFat {
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
                rounds: u32::from(rounds),
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
                rounds: domain::Rounds::new(rounds).unwrap_or_default(),
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

pub trait SendRequest: Send + Sync + 'static {
    #[allow(async_fn_in_trait)]
    async fn send_request(
        &self,
        request: gloo_net::http::Request,
    ) -> Result<gloo_net::http::Response, gloo_net::Error>;
}

#[derive(Clone)]
pub struct GlooNetSendRequest;

impl SendRequest for GlooNetSendRequest {
    async fn send_request(
        &self,
        request: gloo_net::http::Request,
    ) -> Result<gloo_net::http::Response, gloo_net::Error> {
        request.send().await
    }
}

#[cfg(test)]
mod tests {

    use chrono::NaiveDate;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use serde_json::json;

    use crate::tests::data::{
        BODY_FAT, BODY_FAT_2, BODY_FATS, BODY_WEIGHT, BODY_WEIGHT_2, BODY_WEIGHTS, EXERCISE,
        EXERCISE_2, EXERCISES, PERIOD, PERIOD_2, PERIODS, ROUTINE, ROUTINE_2, ROUTINES,
        TRAINING_SESSION, TRAINING_SESSION_2, TRAINING_SESSIONS, USER, USER_2, USERS,
    };

    use super::*;

    #[test]
    fn test_user_try_from() {
        assert_eq!(
            domain::User::try_from(User::from(USER.clone())),
            Ok(USER.clone())
        );
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
        assert_eq!(
            domain::BodyWeight::from(BodyWeight::from(BODY_WEIGHT)),
            BODY_WEIGHT
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
        assert_eq!(domain::BodyFat::from(BodyFat::from(BODY_FAT)), BODY_FAT);
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
        assert_eq!(domain::Period::try_from(Period::from(PERIOD)), Ok(PERIOD));
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
        assert_eq!(
            domain::Exercise::try_from(Exercise::from(EXERCISE.clone())),
            Ok(EXERCISE.clone())
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

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    mod wasm {
        use std::sync::{Arc, Mutex};

        use pretty_assertions::assert_eq;
        use valens_domain::{
            BodyFatRepository, BodyWeightRepository, ExerciseRepository, PeriodRepository,
            RoutineRepository, SessionRepository, TrainingSessionRepository, UserRepository,
            VersionRepository,
        };
        use wasm_bindgen_test::wasm_bindgen_test;

        use super::*;

        #[wasm_bindgen_test]
        async fn test_request_session() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&User::from(USER.clone())),
                ))
                .request_session(USER.id)
                .await
                .unwrap(),
                USER.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_initialize_session() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&User::from(USER.clone())),
                ))
                .initialize_session()
                .await
                .unwrap(),
                USER.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_session() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_session()
                .await
                .unwrap(),
                ()
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_version() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&json!("0.1.2")),
                ))
                .read_version()
                .await
                .unwrap(),
                "0.1.2".to_string()
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_users() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&[User::from(USER.clone()), User::from(USER_2.clone())]),
                ))
                .read_users()
                .await
                .unwrap(),
                USERS.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_user() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&User::from(USER.clone())),
                ))
                .create_user(USER.name.clone(), USER.sex)
                .await
                .unwrap(),
                USER.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_replace_user() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&User::from(USER.clone())),
                ))
                .replace_user(USER.clone())
                .await
                .unwrap(),
                USER.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_user() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_user(USER.id)
                .await
                .unwrap(),
                USER.id
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_body_weight() {
            assert_eq!(
                rest_with_response(Some(gloo_net::http::Response::builder().status(200).json(
                    &[
                        BodyWeight::from(BODY_WEIGHT),
                        BodyWeight::from(BODY_WEIGHT_2),
                    ]
                )))
                .read_body_weight()
                .await
                .unwrap(),
                BODY_WEIGHTS.to_vec()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_body_weight() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&BodyWeight::from(BODY_WEIGHT)),
                ))
                .create_body_weight(BODY_WEIGHT)
                .await
                .unwrap(),
                BODY_WEIGHT
            );
        }

        #[wasm_bindgen_test]
        async fn test_replace_body_weight() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&BodyWeight::from(BODY_WEIGHT)),
                ))
                .replace_body_weight(BODY_WEIGHT)
                .await
                .unwrap(),
                BODY_WEIGHT
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_body_weight() {
            let date = NaiveDate::from_ymd_opt(2020, 2, 2).unwrap();
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_body_weight(date)
                .await
                .unwrap(),
                date
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_body_fat() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&[BodyFat::from(BODY_FAT), BodyFat::from(BODY_FAT_2)]),
                ))
                .read_body_fat()
                .await
                .unwrap(),
                BODY_FATS.to_vec()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_body_fat() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&BodyFat::from(BODY_FAT)),
                ))
                .create_body_fat(BODY_FAT)
                .await
                .unwrap(),
                BODY_FAT
            );
        }

        #[wasm_bindgen_test]
        async fn test_replace_body_fat() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&BodyFat::from(BODY_FAT)),
                ))
                .replace_body_fat(BODY_FAT)
                .await
                .unwrap(),
                BODY_FAT
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_body_fat() {
            let date = NaiveDate::from_ymd_opt(2020, 2, 2).unwrap();
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_body_fat(date)
                .await
                .unwrap(),
                date
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_period() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&[Period::from(PERIOD), Period::from(PERIOD_2)]),
                ))
                .read_period()
                .await
                .unwrap(),
                PERIODS.to_vec()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_period() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&Period::from(PERIOD)),
                ))
                .create_period(PERIOD)
                .await
                .unwrap(),
                PERIOD
            );
        }

        #[wasm_bindgen_test]
        async fn test_replace_period() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&Period::from(PERIOD)),
                ))
                .replace_period(PERIOD)
                .await
                .unwrap(),
                PERIOD
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_period() {
            let date = NaiveDate::from_ymd_opt(2020, 2, 2).unwrap();
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_period(date)
                .await
                .unwrap(),
                date
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_exercises() {
            assert_eq!(
                rest_with_response(Some(gloo_net::http::Response::builder().status(200).json(
                    &[
                        Exercise::from(EXERCISE.clone()),
                        Exercise::from(EXERCISE_2.clone()),
                    ]
                )))
                .read_exercises()
                .await
                .unwrap(),
                EXERCISES.to_vec()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_exercise() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&Exercise::from(EXERCISE.clone())),
                ))
                .create_exercise(EXERCISE.name.clone(), EXERCISE.muscles.clone())
                .await
                .unwrap(),
                EXERCISE.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_replace_exercise() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&Exercise::from(EXERCISE.clone())),
                ))
                .replace_exercise(EXERCISE.clone())
                .await
                .unwrap(),
                EXERCISE.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_exercise() {
            let id = 1.into();
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_exercise(id)
                .await
                .unwrap(),
                id
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_routines() {
            assert_eq!(
                rest_with_response(Some(gloo_net::http::Response::builder().status(200).json(
                    &[
                        Routine::from(ROUTINE.clone()),
                        Routine::from(ROUTINE_2.clone()),
                    ]
                )))
                .read_routines()
                .await
                .unwrap(),
                ROUTINES.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_routine() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&Routine::from(ROUTINE.clone())),
                ))
                .create_routine(ROUTINE.name.clone(), ROUTINE.sections.clone())
                .await
                .unwrap(),
                ROUTINE.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_modify_routine() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&Routine::from(ROUTINE.clone())),
                ))
                .modify_routine(
                    ROUTINE.id,
                    Some(ROUTINE.name.clone()),
                    Some(ROUTINE.archived),
                    Some(ROUTINE.sections.clone())
                )
                .await
                .unwrap(),
                ROUTINE.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_routine() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_routine(ROUTINE.id)
                .await
                .unwrap(),
                ROUTINE.id
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_training_sessions() {
            assert_eq!(
                rest_with_response(Some(gloo_net::http::Response::builder().status(200).json(
                    &[
                        TrainingSession::from(TRAINING_SESSION.clone()),
                        TrainingSession::from(TRAINING_SESSION_2.clone()),
                    ]
                )))
                .read_training_sessions()
                .await
                .unwrap(),
                TRAINING_SESSIONS.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_training_session() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&TrainingSession::from(TRAINING_SESSION.clone())),
                ))
                .create_training_session(
                    TRAINING_SESSION.routine_id,
                    TRAINING_SESSION.date,
                    TRAINING_SESSION.notes.clone(),
                    TRAINING_SESSION.elements.clone()
                )
                .await
                .unwrap(),
                TRAINING_SESSION.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_modify_training_session() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&TrainingSession::from(TRAINING_SESSION.clone())),
                ))
                .modify_training_session(
                    TRAINING_SESSION.id,
                    Some(TRAINING_SESSION.notes.clone()),
                    Some(TRAINING_SESSION.elements.clone())
                )
                .await
                .unwrap(),
                TRAINING_SESSION.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_training_session() {
            assert_eq!(
                rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_training_session(TRAINING_SESSION.id)
                .await
                .unwrap(),
                TRAINING_SESSION.id
            );
        }

        fn rest_with_response(
            response: Option<Result<gloo_net::http::Response, gloo_net::Error>>,
        ) -> REST<MockSendRequest> {
            let sender = MockSendRequest {
                #[allow(clippy::arc_with_non_send_sync)]
                request: Arc::new(Mutex::new(None)),
                #[allow(clippy::arc_with_non_send_sync)]
                response: Arc::new(Mutex::new(response)),
            };
            REST { sender }
        }

        struct MockSendRequest {
            request: Arc<Mutex<Option<gloo_net::http::Request>>>,
            response: Arc<Mutex<Option<Result<gloo_net::http::Response, gloo_net::Error>>>>,
        }

        unsafe impl Send for MockSendRequest {}
        unsafe impl Sync for MockSendRequest {}

        impl SendRequest for MockSendRequest {
            async fn send_request(
                &self,
                request: gloo_net::http::Request,
            ) -> Result<gloo_net::http::Response, gloo_net::Error> {
                *self.request.lock().unwrap() = Some(request);
                (*self.response.lock().unwrap())
                    .take()
                    .expect("no response set")
            }
        }
    }
}
