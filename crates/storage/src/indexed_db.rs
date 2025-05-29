#![allow(clippy::missing_errors_doc)]

use chrono::NaiveDate;
use indexed_db_futures::{
    DeserialiseFromJs, KeyPath, SerialiseToJs, database::Database, error::OpenDbError, prelude::*,
    primitive::TryToJs, transaction::TransactionMode,
};
use log::debug;
use strum::AsRefStr;
use thiserror;
use uuid::Uuid;
use valens_domain as domain;

#[derive(Clone)]
pub struct IndexedDB;

impl IndexedDB {
    async fn open(&self) -> Result<Database, OpenDbError> {
        Database::open("valens")
            .with_version(1u8)
            .with_on_blocked(|event| {
                debug!("upgrade of database blocked: {event:?}");
                Ok(())
            })
            .with_on_upgrade_needed(|event, db| {
                #[allow(clippy::single_match)]
                match (event.old_version(), event.new_version()) {
                    (0.0, Some(1.0)) => {
                        db.create_object_store(Store::App).build()?;
                        db.create_object_store(Store::BodyWeight)
                            .with_key_path(KeyPath::One("date"))
                            .build()?;
                        db.create_object_store(Store::BodyFat)
                            .with_key_path(KeyPath::One("date"))
                            .build()?;
                        db.create_object_store(Store::Period)
                            .with_key_path(KeyPath::One("date"))
                            .build()?;
                        db.create_object_store(Store::Exercises)
                            .with_key_path(KeyPath::One("id"))
                            .build()?;
                        db.create_object_store(Store::Routines)
                            .with_key_path(KeyPath::One("id"))
                            .build()?;
                        db.create_object_store(Store::TrainingSessions)
                            .with_key_path(KeyPath::One("id"))
                            .build()?;
                    }
                    _ => {}
                }
                Ok(())
            })
            .await
    }

    pub async fn get<K, R, V>(&self, object_store: Store, key: &K) -> Result<R, String>
    where
        K: serde::Serialize + TryToJs,
        R: TryFrom<V>,
        V: for<'de> serde::Deserialize<'de>,
        <R as TryFrom<V>>::Error: std::error::Error,
    {
        async {
            let db = IndexedDB.open().await?;
            let transaction = db
                .transaction(object_store.as_ref())
                .with_mode(TransactionMode::Readonly)
                .build()?;
            let store = transaction.object_store(object_store.as_ref())?;
            Ok(R::try_from(
                store
                    .get(key)
                    .serde()?
                    .await?
                    .ok_or(IndexedDBError::ObjectNotFound)?,
            )?)
        }
        .await
        .map_err(|err: Box<dyn std::error::Error>| err.to_string())
    }

    pub async fn get_all<R, V>(
        &self,
        object_store: Store,
    ) -> Result<Vec<R>, Box<dyn std::error::Error>>
    where
        R: TryFrom<V>,
        V: DeserialiseFromJs,
        <R as TryFrom<V>>::Error: std::error::Error + 'static,
    {
        async {
            let db = IndexedDB.open().await?;
            let transaction = db
                .transaction(object_store.as_ref())
                .with_mode(TransactionMode::Readonly)
                .build()?;
            let store = transaction.object_store(object_store.as_ref())?;
            let mut r = vec![];
            for e in store.get_all().serde()?.await? {
                r.push(R::try_from(e?)?);
            }
            Ok(r)
        }
        .await
    }

    pub async fn add<V: serde::Serialize, R>(
        &self,
        object_store: Store,
        value: V,
        result: R,
    ) -> Result<R, Box<dyn std::error::Error>> {
        async {
            let db = self.open().await?;
            let transaction = db
                .transaction(object_store.as_ref())
                .with_mode(TransactionMode::Readwrite)
                .build()?;
            let store = transaction.object_store(object_store.as_ref())?;
            store.add(value).serde()?.await?;
            transaction.commit().await?;
            Ok(result)
        }
        .await
    }

    pub async fn put<V: serde::Serialize, R>(
        &self,
        object_store: Store,
        value: V,
        result: R,
    ) -> Result<R, Box<dyn std::error::Error>> {
        async {
            let db = self.open().await?;
            let transaction = db
                .transaction(object_store.as_ref())
                .with_mode(TransactionMode::Readwrite)
                .build()?;
            let store = transaction.object_store(object_store.as_ref())?;
            store.put(value).serde()?.await?;
            transaction.commit().await?;
            Ok(result)
        }
        .await
    }

    pub async fn replace_all<'a, V: 'a, T: From<&'a V> + serde::Serialize, R>(
        &self,
        object_store: Store,
        values: &'a [V],
        result: R,
    ) -> Result<R, String> {
        async {
            let db = self.open().await?;
            let transaction = db
                .transaction(object_store.as_ref())
                .with_mode(TransactionMode::Readwrite)
                .build()?;
            let store = transaction.object_store(object_store.as_ref())?;
            store.clear()?;
            for value in values {
                store.put(T::from(value)).serde()?.await?;
            }
            transaction.commit().await?;
            Ok(result)
        }
        .await
        .map_err(|err: Box<dyn std::error::Error>| err.to_string())
    }

    pub async fn delete<K: serde::Serialize + SerialiseToJs + TryToJs, R>(
        &self,
        object_store: Store,
        key: K,
        result: R,
    ) -> Result<R, Box<dyn std::error::Error>> {
        async {
            let db = self.open().await?;
            let transaction = db
                .transaction(object_store.as_ref())
                .with_mode(TransactionMode::Readwrite)
                .build()?;
            let store = transaction.object_store(object_store.as_ref())?;
            store.delete(key).serde()?.await?;
            transaction.commit().await?;
            Ok(result)
        }
        .await
    }

    pub async fn clear_app_data(&self) -> Result<(), String> {
        async {
            let db = IndexedDB.open().await?;
            for os in [Store::App] {
                let transaction = db
                    .transaction(os.as_ref())
                    .with_mode(TransactionMode::Readwrite)
                    .build()?;
                let store = transaction.object_store(os.as_ref())?;
                store.clear()?.await?;
                transaction.commit().await?;
            }
            Ok(())
        }
        .await
        .map_err(|err: Box<dyn std::error::Error>| err.to_string())
    }

    pub async fn clear_session_dependent_data(&self) -> Result<(), Box<dyn std::error::Error>> {
        async {
            let db = IndexedDB.open().await?;
            for os in [
                Store::BodyWeight,
                Store::BodyFat,
                Store::Period,
                Store::Exercises,
                Store::Routines,
                Store::TrainingSessions,
            ] {
                let transaction = db
                    .transaction(os.as_ref())
                    .with_mode(TransactionMode::Readwrite)
                    .build()?;
                let store = transaction.object_store(os.as_ref())?;
                store.clear()?.await?;
                transaction.commit().await?;
            }
            Ok(())
        }
        .await
    }

    pub async fn write_session(&self, user: &domain::User) -> Result<(), String> {
        async {
            let db = self.open().await?;
            let transaction = db
                .transaction(Store::App.as_ref())
                .with_mode(TransactionMode::Readwrite)
                .build()?;
            let store = transaction.object_store(Store::App.as_ref())?;
            store
                .put(User::from(user))
                .with_key("session".to_string())
                .serde()?
                .await?;
            transaction.commit().await?;
            Ok(())
        }
        .await
        .map_err(|err: Box<dyn std::error::Error>| err.to_string())
    }

    pub async fn write_body_weight(
        &self,
        body_weight: &[domain::BodyWeight],
    ) -> Result<(), String> {
        IndexedDB
            .replace_all::<_, BodyWeight, _>(Store::BodyWeight, body_weight, ())
            .await
    }

    pub async fn write_body_fat(&self, body_fat: &[domain::BodyFat]) -> Result<(), String> {
        IndexedDB
            .replace_all::<_, BodyFat, _>(Store::BodyFat, body_fat, ())
            .await
    }

    pub async fn write_period(&self, period: &[domain::Period]) -> Result<(), String> {
        IndexedDB
            .replace_all::<_, Period, _>(Store::Period, period, ())
            .await
    }

    pub async fn write_exercises(&self, exercises: &[domain::Exercise]) -> Result<(), String> {
        IndexedDB
            .replace_all::<_, Exercise, _>(Store::Exercises, exercises, ())
            .await
    }

    pub async fn write_routines(&self, routines: &[domain::Routine]) -> Result<(), String> {
        IndexedDB
            .replace_all::<_, Routine, _>(Store::Routines, routines, ())
            .await
    }

    pub async fn write_training_sessions(
        &self,
        training_sessions: &[domain::TrainingSession],
    ) -> Result<(), String> {
        IndexedDB
            .replace_all::<_, TrainingSession, _>(Store::TrainingSessions, training_sessions, ())
            .await
    }
}

impl domain::SessionRepository for IndexedDB {
    async fn request_session(&self, _: domain::UserID) -> Result<domain::User, domain::ReadError> {
        panic!("unsupported")
    }

    async fn initialize_session(&self) -> Result<domain::User, domain::ReadError> {
        let user = async {
            let db = IndexedDB.open().await?;
            let transaction = db
                .transaction(Store::App.as_ref())
                .with_mode(TransactionMode::Readwrite)
                .build()?;
            let store = transaction.object_store(Store::App.as_ref())?;
            let user: Option<User> = store.get("session").serde()?.await?;
            Ok::<Option<User>, Box<dyn std::error::Error>>(user)
        }
        .await?;

        Ok(user
            .ok_or(domain::ReadError::Storage(domain::StorageError::NoSession))?
            .try_into()
            .map_err(Box::from)?)
    }

    async fn delete_session(&self) -> Result<(), domain::DeleteError> {
        async {
            let db = IndexedDB.open().await?;
            let transaction = db
                .transaction(Store::App.as_ref())
                .with_mode(TransactionMode::Readwrite)
                .build()?;
            let store = transaction.object_store(Store::App.as_ref())?;
            store.delete("session").serde()?.await?;
            transaction.commit().await?;
            Ok(())
        }
        .await
        .map_err(domain::DeleteError::Other)
    }
}

impl domain::UserRepository for IndexedDB {
    async fn read_users(&self) -> Result<Vec<domain::User>, domain::ReadError> {
        panic!("unsupported")
    }

    async fn create_user(
        &self,
        _name: domain::Name,
        _sex: domain::Sex,
    ) -> Result<domain::User, domain::CreateError> {
        panic!("unsupported")
    }

    async fn replace_user(&self, _: domain::User) -> Result<domain::User, domain::UpdateError> {
        panic!("unsupported")
    }

    async fn delete_user(&self, _: domain::UserID) -> Result<domain::UserID, domain::DeleteError> {
        panic!("unsupported")
    }
}

impl domain::BodyWeightRepository for IndexedDB {
    async fn sync_body_weight(&self) -> Result<Vec<domain::BodyWeight>, domain::SyncError> {
        panic!("unsupported")
    }

    async fn read_body_weight(&self) -> Result<Vec<domain::BodyWeight>, domain::ReadError> {
        Ok(IndexedDB
            .get_all::<domain::BodyWeight, BodyWeight>(Store::BodyWeight)
            .await?)
    }

    async fn create_body_weight(
        &self,
        body_weight: domain::BodyWeight,
    ) -> Result<domain::BodyWeight, domain::CreateError> {
        Ok(IndexedDB
            .add(
                Store::BodyWeight,
                BodyWeight::from(&body_weight),
                body_weight,
            )
            .await?)
    }

    async fn replace_body_weight(
        &self,
        body_weight: domain::BodyWeight,
    ) -> Result<domain::BodyWeight, domain::UpdateError> {
        Ok(IndexedDB
            .put(
                Store::BodyWeight,
                BodyWeight::from(&body_weight),
                body_weight,
            )
            .await?)
    }

    async fn delete_body_weight(&self, date: NaiveDate) -> Result<NaiveDate, domain::DeleteError> {
        Ok(IndexedDB
            .delete(Store::BodyWeight, date.to_string(), date)
            .await?)
    }
}

impl domain::BodyFatRepository for IndexedDB {
    async fn sync_body_fat(&self) -> Result<Vec<domain::BodyFat>, domain::SyncError> {
        panic!("unsupported")
    }

    async fn read_body_fat(&self) -> Result<Vec<domain::BodyFat>, domain::ReadError> {
        Ok(IndexedDB
            .get_all::<domain::BodyFat, BodyFat>(Store::BodyFat)
            .await?)
    }

    async fn create_body_fat(
        &self,
        body_fat: domain::BodyFat,
    ) -> Result<domain::BodyFat, domain::CreateError> {
        Ok(IndexedDB
            .add(Store::BodyFat, BodyFat::from(&body_fat), body_fat)
            .await?)
    }

    async fn replace_body_fat(
        &self,
        body_fat: domain::BodyFat,
    ) -> Result<domain::BodyFat, domain::UpdateError> {
        Ok(IndexedDB
            .put(Store::BodyFat, BodyFat::from(&body_fat), body_fat)
            .await?)
    }

    async fn delete_body_fat(&self, date: NaiveDate) -> Result<NaiveDate, domain::DeleteError> {
        Ok(IndexedDB
            .delete(Store::BodyFat, date.to_string(), date)
            .await?)
    }
}

impl domain::PeriodRepository for IndexedDB {
    async fn sync_period(&self) -> Result<Vec<domain::Period>, domain::SyncError> {
        panic!("unsupported")
    }

    async fn read_period(&self) -> Result<Vec<domain::Period>, domain::ReadError> {
        Ok(IndexedDB
            .get_all::<domain::Period, Period>(Store::Period)
            .await?)
    }

    async fn create_period(
        &self,
        period: domain::Period,
    ) -> Result<domain::Period, domain::CreateError> {
        Ok(IndexedDB
            .add(Store::Period, Period::from(&period), period)
            .await?)
    }

    async fn replace_period(
        &self,
        period: domain::Period,
    ) -> Result<domain::Period, domain::UpdateError> {
        Ok(IndexedDB
            .put(Store::Period, Period::from(&period), period)
            .await?)
    }

    async fn delete_period(&self, date: NaiveDate) -> Result<NaiveDate, domain::DeleteError> {
        Ok(IndexedDB
            .delete(Store::Period, date.to_string(), date)
            .await?)
    }
}

impl domain::ExerciseRepository for IndexedDB {
    async fn sync_exercises(&self) -> Result<Vec<domain::Exercise>, domain::SyncError> {
        panic!("unsupported")
    }

    async fn read_exercises(&self) -> Result<Vec<domain::Exercise>, domain::ReadError> {
        Ok(IndexedDB
            .get_all::<domain::Exercise, Exercise>(Store::Exercises)
            .await?)
    }

    async fn create_exercise(
        &self,
        _name: domain::Name,
        _muscles: Vec<domain::ExerciseMuscle>,
    ) -> Result<domain::Exercise, domain::CreateError> {
        panic!("unsupported")
    }

    async fn replace_exercise(
        &self,
        exercise: domain::Exercise,
    ) -> Result<domain::Exercise, domain::UpdateError> {
        Ok(IndexedDB
            .put(Store::Exercises, Exercise::from(&exercise), exercise)
            .await?)
    }

    async fn delete_exercise(
        &self,
        id: domain::ExerciseID,
    ) -> Result<domain::ExerciseID, domain::DeleteError> {
        Ok(IndexedDB
            .delete(Store::Exercises, id.to_string(), id)
            .await?)
    }
}

impl domain::RoutineRepository for IndexedDB {
    async fn sync_routines(&self) -> Result<Vec<domain::Routine>, domain::SyncError> {
        panic!("unsupported")
    }

    async fn read_routines(&self) -> Result<Vec<domain::Routine>, domain::ReadError> {
        Ok(IndexedDB
            .get_all::<domain::Routine, Routine>(Store::Routines)
            .await?)
    }

    async fn create_routine(
        &self,
        _name: domain::Name,
        _sections: Vec<domain::RoutinePart>,
    ) -> Result<domain::Routine, domain::CreateError> {
        panic!("unsupported")
    }

    async fn modify_routine(
        &self,
        id: domain::RoutineID,
        name: Option<domain::Name>,
        archived: Option<bool>,
        sections: Option<Vec<domain::RoutinePart>>,
    ) -> Result<domain::Routine, domain::UpdateError> {
        let mut routine = IndexedDB
            .get::<String, domain::Routine, Routine>(Store::Routines, &id.to_string())
            .await
            .map_err(Box::from)?;

        if let Some(name) = name {
            routine.name = name;
        }
        if let Some(archived) = archived {
            routine.archived = archived;
        }
        if let Some(sections) = sections {
            routine.sections = sections;
        }

        Ok(IndexedDB
            .put(Store::Routines, Routine::from(&routine), routine)
            .await?)
    }

    async fn delete_routine(
        &self,
        id: domain::RoutineID,
    ) -> Result<domain::RoutineID, domain::DeleteError> {
        Ok(IndexedDB
            .delete(Store::Routines, id.to_string(), id)
            .await?)
    }
}

impl domain::TrainingSessionRepository for IndexedDB {
    async fn sync_training_sessions(
        &self,
    ) -> Result<Vec<domain::TrainingSession>, domain::SyncError> {
        panic!("unsupported")
    }

    async fn read_training_sessions(
        &self,
    ) -> Result<Vec<domain::TrainingSession>, domain::ReadError> {
        Ok(IndexedDB
            .get_all::<domain::TrainingSession, TrainingSession>(Store::TrainingSessions)
            .await?)
    }

    async fn create_training_session(
        &self,
        _routine_id: domain::RoutineID,
        _date: NaiveDate,
        _notes: String,
        _elements: Vec<domain::TrainingSessionElement>,
    ) -> Result<domain::TrainingSession, domain::CreateError> {
        panic!("unsupported")
    }

    async fn modify_training_session(
        &self,
        id: domain::TrainingSessionID,
        notes: Option<String>,
        elements: Option<Vec<domain::TrainingSessionElement>>,
    ) -> Result<domain::TrainingSession, domain::UpdateError> {
        let mut training_session = IndexedDB
            .get::<String, domain::TrainingSession, TrainingSession>(
                Store::TrainingSessions,
                &id.to_string(),
            )
            .await
            .map_err(Box::from)?;

        if let Some(notes) = notes {
            training_session.notes = notes;
        }
        if let Some(elements) = elements {
            training_session.elements = elements;
        }

        Ok(IndexedDB
            .put(
                Store::TrainingSessions,
                TrainingSession::from(&training_session),
                training_session,
            )
            .await?)
    }

    async fn delete_training_session(
        &self,
        id: domain::TrainingSessionID,
    ) -> Result<domain::TrainingSessionID, domain::DeleteError> {
        Ok(IndexedDB
            .delete(Store::TrainingSessions, id.to_string(), id)
            .await?)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum IndexedDBError {
    #[error("object not found")]
    ObjectNotFound,
    #[error(transparent)]
    IDBError(#[from] indexed_db_futures::error::Error),
    #[error(transparent)]
    IDBOpenDBError(#[from] indexed_db_futures::error::OpenDbError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(AsRefStr)]
pub enum Store {
    #[strum(serialize = "app")]
    App,
    #[strum(serialize = "body_weight")]
    BodyWeight,
    #[strum(serialize = "body_fat")]
    BodyFat,
    #[strum(serialize = "period")]
    Period,
    #[strum(serialize = "exercises")]
    Exercises,
    #[strum(serialize = "routines")]
    Routines,
    #[strum(serialize = "training_sessions")]
    TrainingSessions,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub sex: u8,
}

impl From<domain::User> for User {
    fn from(value: domain::User) -> Self {
        Self::from(&value)
    }
}

impl From<&domain::User> for User {
    fn from(value: &domain::User) -> Self {
        Self {
            id: *value.id,
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct BodyWeight {
    pub date: NaiveDate,
    pub weight: f32,
}

impl From<domain::BodyWeight> for BodyWeight {
    fn from(value: domain::BodyWeight) -> Self {
        Self::from(&value)
    }
}

impl From<&domain::BodyWeight> for BodyWeight {
    fn from(value: &domain::BodyWeight) -> Self {
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
        Self::from(&value)
    }
}

impl From<&domain::BodyFat> for BodyFat {
    fn from(value: &domain::BodyFat) -> Self {
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
pub struct Period {
    pub date: NaiveDate,
    pub intensity: u8,
}

impl From<domain::Period> for Period {
    fn from(value: domain::Period) -> Self {
        Self::from(&value)
    }
}

impl From<&domain::Period> for Period {
    fn from(value: &domain::Period) -> Self {
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
pub struct Exercise {
    pub id: Uuid,
    pub name: String,
    pub muscles: Vec<ExerciseMuscle>,
}

impl From<domain::Exercise> for Exercise {
    fn from(value: domain::Exercise) -> Self {
        Self {
            id: *value.id,
            name: value.name.to_string(),
            muscles: value
                .muscles
                .into_iter()
                .map(ExerciseMuscle::from)
                .collect(),
        }
    }
}

impl From<&domain::Exercise> for Exercise {
    fn from(value: &domain::Exercise) -> Self {
        Self {
            id: *value.id,
            name: value.name.to_string(),
            muscles: value
                .muscles
                .iter()
                .cloned()
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

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ExerciseError {
    #[error(transparent)]
    InvalidName(#[from] domain::NameError),
    #[error(transparent)]
    InvalidMuscle(#[from] domain::MuscleIDError),
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
    pub id: Uuid,
    pub name: String,
    pub notes: String,
    pub archived: bool,
    pub sections: Vec<RoutinePart>,
}

impl From<domain::Routine> for Routine {
    fn from(value: domain::Routine) -> Self {
        Self {
            id: *value.id,
            name: value.name.to_string(),
            notes: value.notes,
            archived: value.archived,
            sections: value.sections.into_iter().map(RoutinePart::from).collect(),
        }
    }
}

impl From<&domain::Routine> for Routine {
    fn from(value: &domain::Routine) -> Self {
        Self {
            id: *value.id,
            name: value.name.to_string(),
            notes: value.notes.clone(),
            archived: value.archived,
            sections: value
                .sections
                .iter()
                .cloned()
                .map(RoutinePart::from)
                .collect(),
        }
    }
}

impl TryFrom<Routine> for domain::Routine {
    type Error = domain::NameError;

    fn try_from(value: Routine) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id.into(),
            name: domain::Name::new(&value.name)?,
            notes: value.notes,
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
#[serde(untagged)]
pub enum RoutinePart {
    RoutineSection {
        rounds: u32,
        parts: Vec<RoutinePart>,
    },
    RoutineActivity {
        exercise_id: Option<Uuid>,
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
                    Some(*exercise_id)
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
pub struct TrainingSessions(Vec<TrainingSession>);

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct TrainingSession {
    pub id: Uuid,
    pub routine_id: Uuid,
    pub date: NaiveDate,
    pub notes: String,
    pub elements: Vec<TrainingSessionElement>,
}

impl From<domain::TrainingSession> for TrainingSession {
    fn from(value: domain::TrainingSession) -> Self {
        Self {
            id: *value.id,
            routine_id: *value.routine_id,
            date: value.date,
            notes: value.notes,
            elements: value
                .elements
                .into_iter()
                .map(TrainingSessionElement::from)
                .collect(),
        }
    }
}

impl From<&domain::TrainingSession> for TrainingSession {
    fn from(value: &domain::TrainingSession) -> Self {
        Self {
            id: *value.id,
            routine_id: *value.routine_id,
            date: value.date,
            notes: value.notes.clone(),
            elements: value
                .elements
                .iter()
                .cloned()
                .map(TrainingSessionElement::from)
                .collect(),
        }
    }
}

impl From<TrainingSession> for domain::TrainingSession {
    fn from(value: TrainingSession) -> Self {
        Self {
            id: value.id.into(),
            routine_id: value.routine_id.into(),
            date: value.date,
            notes: value.notes,
            elements: value
                .elements
                .into_iter()
                .map(domain::TrainingSessionElement::from)
                .collect(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum TrainingSessionElement {
    Set {
        exercise_id: Uuid,
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
                exercise_id: *exercise_id,
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
                exercise_id: exercise_id.into(),
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

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use serde_json::json;

    use crate::tests::data::{
        BODY_FAT, BODY_FATS, BODY_WEIGHT, BODY_WEIGHTS, EXERCISE, EXERCISES, PERIOD, PERIODS,
        ROUTINE, ROUTINES, TRAINING_SESSION, TRAINING_SESSIONS, USER,
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
            domain::BodyWeight::from(BodyWeight::from(BODY_WEIGHT.clone())),
            BODY_WEIGHT.clone()
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
        use pretty_assertions::assert_eq;
        use valens_domain::{
            BodyFatRepository, BodyWeightRepository, ExerciseRepository, PeriodRepository,
            RoutineRepository, SessionRepository, TrainingSessionRepository, UserRepository,
        };
        use wasm_bindgen_test::wasm_bindgen_test;

        use super::*;

        #[wasm_bindgen_test]
        #[should_panic]
        async fn test_request_session() {
            let _ = IndexedDB.request_session(USER.id).await;
        }

        #[wasm_bindgen_test]
        async fn test_initialize_session() {
            reset().await;

            assert!(matches!(
                IndexedDB.initialize_session().await,
                Err(domain::ReadError::Storage(domain::StorageError::NoSession))
            ));

            IndexedDB.write_session(&USER).await.unwrap();

            assert_eq!(IndexedDB.initialize_session().await.unwrap(), USER.clone());
        }

        #[wasm_bindgen_test]
        async fn test_delete_session() {
            reset().await;

            IndexedDB.write_session(&USER).await.unwrap();

            assert_eq!(IndexedDB.delete_session().await.unwrap(), ());

            assert!(matches!(
                IndexedDB.initialize_session().await,
                Err(domain::ReadError::Storage(domain::StorageError::NoSession))
            ));
        }

        #[wasm_bindgen_test]
        async fn test_delete_session_non_existing() {
            reset().await;

            assert_eq!(IndexedDB.delete_session().await.unwrap(), ());
        }

        #[wasm_bindgen_test]
        #[should_panic]
        async fn test_read_users() {
            let _ = IndexedDB.read_users().await;
        }

        #[wasm_bindgen_test]
        #[should_panic]
        async fn test_create_user() {
            let _ = IndexedDB.create_user(USER.name.clone(), USER.sex).await;
        }

        #[wasm_bindgen_test]
        #[should_panic]
        async fn test_replace_user() {
            let _ = IndexedDB.replace_user(USER.clone()).await;
        }

        #[wasm_bindgen_test]
        #[should_panic]
        async fn test_delete_user() {
            let _ = IndexedDB.delete_user(USER.id).await;
        }

        #[wasm_bindgen_test]
        async fn test_read_body_weight() {
            reset().await;
            init_session().await;

            assert_eq!(IndexedDB.read_body_weight().await.unwrap(), vec![]);

            IndexedDB.write_body_weight(BODY_WEIGHTS).await.unwrap();

            assert_eq!(
                IndexedDB.read_body_weight().await.unwrap(),
                BODY_WEIGHTS.to_vec()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_body_weight() {
            reset().await;
            init_session().await;

            assert_eq!(IndexedDB.read_body_weight().await.unwrap(), vec![]);

            assert_eq!(
                IndexedDB.create_body_weight(BODY_WEIGHT).await.unwrap(),
                BODY_WEIGHT
            );

            assert_eq!(
                IndexedDB.read_body_weight().await.unwrap(),
                vec![BODY_WEIGHT]
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_body_weight_conflict() {
            reset().await;
            init_session().await;

            IndexedDB.write_body_weight(&[BODY_WEIGHT]).await.unwrap();

            assert_eq!(
                IndexedDB.read_body_weight().await.unwrap(),
                vec![BODY_WEIGHT]
            );

            let mut body_weight = BODY_WEIGHT;
            body_weight.weight += 1.0;

            assert!(
                IndexedDB
                    .create_body_weight(body_weight.clone())
                    .await
                    .unwrap_err()
                    .to_string()
                    .starts_with("ConstraintError: ")
            );

            assert_eq!(
                IndexedDB.read_body_weight().await.unwrap(),
                vec![BODY_WEIGHT]
            );
        }

        #[wasm_bindgen_test]
        async fn test_replace_body_weight() {
            reset().await;
            init_session().await;

            IndexedDB.write_body_weight(&[BODY_WEIGHT]).await.unwrap();

            assert_eq!(
                IndexedDB.read_body_weight().await.unwrap(),
                vec![BODY_WEIGHT]
            );

            let mut body_weight = BODY_WEIGHT;
            body_weight.weight += 1.0;

            assert_eq!(
                IndexedDB
                    .replace_body_weight(body_weight.clone())
                    .await
                    .unwrap(),
                body_weight.clone()
            );

            assert_eq!(
                IndexedDB.read_body_weight().await.unwrap(),
                vec![body_weight]
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_body_weight() {
            reset().await;
            init_session().await;

            IndexedDB.write_body_weight(&[BODY_WEIGHT]).await.unwrap();

            assert_eq!(
                IndexedDB.read_body_weight().await.unwrap(),
                vec![BODY_WEIGHT]
            );

            assert_eq!(
                IndexedDB
                    .delete_body_weight(BODY_WEIGHT.date)
                    .await
                    .unwrap(),
                BODY_WEIGHT.date
            );

            assert_eq!(IndexedDB.read_body_weight().await.unwrap(), vec![]);
        }

        #[wasm_bindgen_test]
        async fn test_delete_body_weight_non_existing() {
            reset().await;
            init_session().await;

            assert_eq!(
                IndexedDB
                    .delete_body_weight(BODY_WEIGHT.date)
                    .await
                    .unwrap(),
                BODY_WEIGHT.date
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_body_fat() {
            reset().await;
            init_session().await;

            assert_eq!(IndexedDB.read_body_fat().await.unwrap(), vec![]);

            IndexedDB.write_body_fat(BODY_FATS).await.unwrap();

            assert_eq!(IndexedDB.read_body_fat().await.unwrap(), BODY_FATS.to_vec());
        }

        #[wasm_bindgen_test]
        async fn test_create_body_fat() {
            reset().await;
            init_session().await;

            assert_eq!(IndexedDB.read_body_fat().await.unwrap(), vec![]);

            assert_eq!(IndexedDB.create_body_fat(BODY_FAT).await.unwrap(), BODY_FAT);

            assert_eq!(IndexedDB.read_body_fat().await.unwrap(), vec![BODY_FAT]);
        }

        #[wasm_bindgen_test]
        async fn test_create_body_fat_conflict() {
            reset().await;
            init_session().await;

            IndexedDB.write_body_fat(&[BODY_FAT]).await.unwrap();

            assert_eq!(IndexedDB.read_body_fat().await.unwrap(), vec![BODY_FAT]);

            let mut body_fat = BODY_FAT;
            body_fat.chest = body_fat.chest.map(|v| v + 1);

            assert!(
                IndexedDB
                    .create_body_fat(body_fat.clone())
                    .await
                    .unwrap_err()
                    .to_string()
                    .starts_with("ConstraintError: ")
            );

            assert_eq!(IndexedDB.read_body_fat().await.unwrap(), vec![BODY_FAT]);
        }

        #[wasm_bindgen_test]
        async fn test_replace_body_fat() {
            reset().await;
            init_session().await;

            IndexedDB.write_body_fat(&[BODY_FAT]).await.unwrap();

            assert_eq!(IndexedDB.read_body_fat().await.unwrap(), vec![BODY_FAT]);

            let mut body_fat = BODY_FAT;
            body_fat.chest = body_fat.chest.map(|v| v + 1);

            assert_eq!(
                IndexedDB.replace_body_fat(body_fat.clone()).await.unwrap(),
                body_fat.clone()
            );

            assert_eq!(IndexedDB.read_body_fat().await.unwrap(), vec![body_fat]);
        }

        #[wasm_bindgen_test]
        async fn test_delete_body_fat() {
            reset().await;
            init_session().await;

            IndexedDB.write_body_fat(&[BODY_FAT]).await.unwrap();

            assert_eq!(IndexedDB.read_body_fat().await.unwrap(), vec![BODY_FAT]);

            assert_eq!(
                IndexedDB.delete_body_fat(BODY_FAT.date).await.unwrap(),
                BODY_FAT.date
            );

            assert_eq!(IndexedDB.read_body_fat().await.unwrap(), vec![]);
        }

        #[wasm_bindgen_test]
        async fn test_delete_body_fat_non_existing() {
            reset().await;
            init_session().await;

            assert_eq!(
                IndexedDB.delete_body_fat(BODY_FAT.date).await.unwrap(),
                BODY_FAT.date
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_period() {
            reset().await;
            init_session().await;

            assert_eq!(IndexedDB.read_period().await.unwrap(), vec![]);

            IndexedDB.write_period(PERIODS).await.unwrap();

            assert_eq!(IndexedDB.read_period().await.unwrap(), PERIODS.to_vec());
        }

        #[wasm_bindgen_test]
        async fn test_create_period() {
            reset().await;
            init_session().await;

            assert_eq!(IndexedDB.read_period().await.unwrap(), vec![]);

            assert_eq!(IndexedDB.create_period(PERIOD).await.unwrap(), PERIOD);

            assert_eq!(IndexedDB.read_period().await.unwrap(), vec![PERIOD]);
        }

        #[wasm_bindgen_test]
        async fn test_create_period_conflict() {
            reset().await;
            init_session().await;

            IndexedDB.write_period(&[PERIOD]).await.unwrap();

            assert_eq!(IndexedDB.read_period().await.unwrap(), vec![PERIOD]);

            let mut period = PERIOD;
            period.intensity = domain::Intensity::Heavy;

            assert!(
                IndexedDB
                    .create_period(period.clone())
                    .await
                    .unwrap_err()
                    .to_string()
                    .starts_with("ConstraintError: ")
            );

            assert_eq!(IndexedDB.read_period().await.unwrap(), vec![PERIOD]);
        }

        #[wasm_bindgen_test]
        async fn test_replace_period() {
            reset().await;
            init_session().await;

            IndexedDB.write_period(&[PERIOD]).await.unwrap();

            assert_eq!(IndexedDB.read_period().await.unwrap(), vec![PERIOD]);

            let mut period = PERIOD;
            period.intensity = domain::Intensity::Heavy;

            assert_eq!(
                IndexedDB.replace_period(period.clone()).await.unwrap(),
                period.clone()
            );

            assert_eq!(IndexedDB.read_period().await.unwrap(), vec![period]);
        }

        #[wasm_bindgen_test]
        async fn test_delete_period() {
            reset().await;
            init_session().await;

            IndexedDB.write_period(&[PERIOD]).await.unwrap();

            assert_eq!(IndexedDB.read_period().await.unwrap(), vec![PERIOD]);

            assert_eq!(
                IndexedDB.delete_period(PERIOD.date).await.unwrap(),
                PERIOD.date
            );

            assert_eq!(IndexedDB.read_period().await.unwrap(), vec![]);
        }

        #[wasm_bindgen_test]
        async fn test_delete_period_non_existing() {
            reset().await;
            init_session().await;

            assert_eq!(
                IndexedDB.delete_period(PERIOD.date).await.unwrap(),
                PERIOD.date
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_exercises() {
            reset().await;
            init_session().await;

            assert_eq!(IndexedDB.read_exercises().await.unwrap(), vec![]);

            IndexedDB.write_exercises(&EXERCISES.clone()).await.unwrap();

            assert_eq!(IndexedDB.read_exercises().await.unwrap(), EXERCISES.clone());
        }

        #[wasm_bindgen_test]
        #[should_panic]
        async fn test_create_exercise() {
            reset().await;
            init_session().await;

            assert_eq!(IndexedDB.read_exercises().await.unwrap(), vec![]);

            assert_eq!(
                IndexedDB
                    .create_exercise(EXERCISE.name.clone(), EXERCISE.muscles.clone())
                    .await
                    .unwrap(),
                EXERCISE.clone()
            );

            assert_eq!(
                IndexedDB.read_exercises().await.unwrap(),
                vec![EXERCISE.clone()]
            );
        }

        #[wasm_bindgen_test]
        async fn test_replace_exercise() {
            reset().await;
            init_session().await;

            IndexedDB
                .write_exercises(&[EXERCISE.clone()])
                .await
                .unwrap();

            assert_eq!(
                IndexedDB.read_exercises().await.unwrap(),
                vec![EXERCISE.clone()]
            );

            let mut exercise = EXERCISE.clone();
            exercise.name = domain::Name::new("C").unwrap();

            assert_eq!(
                IndexedDB.replace_exercise(exercise.clone()).await.unwrap(),
                exercise.clone()
            );

            assert_eq!(IndexedDB.read_exercises().await.unwrap(), vec![exercise]);
        }

        #[wasm_bindgen_test]
        async fn test_delete_exercise() {
            reset().await;
            init_session().await;

            IndexedDB
                .write_exercises(&[EXERCISE.clone()])
                .await
                .unwrap();

            assert_eq!(
                IndexedDB.read_exercises().await.unwrap(),
                vec![EXERCISE.clone()]
            );

            assert_eq!(
                IndexedDB.delete_exercise(EXERCISE.id).await.unwrap(),
                EXERCISE.id
            );

            assert_eq!(IndexedDB.read_exercises().await.unwrap(), vec![]);
        }

        #[wasm_bindgen_test]
        async fn test_delete_exercise_non_existing() {
            reset().await;
            init_session().await;

            assert_eq!(
                IndexedDB.delete_exercise(EXERCISE.id).await.unwrap(),
                EXERCISE.id
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_routines() {
            reset().await;
            init_session().await;

            assert_eq!(IndexedDB.read_routines().await.unwrap(), vec![]);

            IndexedDB.write_routines(&ROUTINES.clone()).await.unwrap();

            assert_eq!(IndexedDB.read_routines().await.unwrap(), ROUTINES.clone());
        }

        #[wasm_bindgen_test]
        #[should_panic]
        async fn test_create_routine() {
            reset().await;
            init_session().await;

            assert_eq!(IndexedDB.read_routines().await.unwrap(), vec![]);

            assert_eq!(
                IndexedDB
                    .create_routine(ROUTINE.name.clone(), ROUTINE.sections.clone())
                    .await
                    .unwrap(),
                ROUTINE.clone()
            );

            assert_eq!(
                IndexedDB.read_routines().await.unwrap(),
                vec![ROUTINE.clone()]
            );
        }

        #[wasm_bindgen_test]
        async fn test_modify_routine() {
            reset().await;
            init_session().await;

            IndexedDB.write_routines(&[ROUTINE.clone()]).await.unwrap();

            assert_eq!(
                IndexedDB.read_routines().await.unwrap(),
                vec![ROUTINE.clone()]
            );

            let mut routine = ROUTINE.clone();
            routine.name = domain::Name::new("C").unwrap();
            routine.archived = true;
            routine.sections = vec![];

            assert_eq!(
                IndexedDB
                    .modify_routine(
                        routine.id,
                        Some(routine.name.clone()),
                        Some(routine.archived),
                        Some(routine.sections.clone())
                    )
                    .await
                    .unwrap(),
                routine.clone()
            );

            assert_eq!(IndexedDB.read_routines().await.unwrap(), vec![routine]);
        }

        #[wasm_bindgen_test]
        async fn test_delete_routine() {
            reset().await;
            init_session().await;

            IndexedDB.write_routines(&[ROUTINE.clone()]).await.unwrap();

            assert_eq!(
                IndexedDB.read_routines().await.unwrap(),
                vec![ROUTINE.clone()]
            );

            assert_eq!(
                IndexedDB.delete_routine(ROUTINE.id).await.unwrap(),
                ROUTINE.id
            );

            assert_eq!(IndexedDB.read_routines().await.unwrap(), vec![]);
        }

        #[wasm_bindgen_test]
        async fn test_delete_routine_non_existing() {
            reset().await;
            init_session().await;

            assert_eq!(
                IndexedDB.delete_routine(ROUTINE.id).await.unwrap(),
                ROUTINE.id
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_training_sessions() {
            reset().await;
            init_session().await;

            assert_eq!(IndexedDB.read_training_sessions().await.unwrap(), vec![]);

            IndexedDB
                .write_training_sessions(&TRAINING_SESSIONS.clone())
                .await
                .unwrap();

            assert_eq!(
                IndexedDB.read_training_sessions().await.unwrap(),
                TRAINING_SESSIONS.clone()
            );
        }

        #[wasm_bindgen_test]
        #[should_panic]
        async fn test_create_training_session() {
            reset().await;
            init_session().await;

            assert_eq!(IndexedDB.read_training_sessions().await.unwrap(), vec![]);

            assert_eq!(
                IndexedDB
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

            assert_eq!(
                IndexedDB.read_training_sessions().await.unwrap(),
                vec![TRAINING_SESSION.clone()]
            );
        }

        #[wasm_bindgen_test]
        async fn test_modify_training_session() {
            reset().await;
            init_session().await;

            IndexedDB
                .write_training_sessions(&[TRAINING_SESSION.clone()])
                .await
                .unwrap();

            assert_eq!(
                IndexedDB.read_training_sessions().await.unwrap(),
                vec![TRAINING_SESSION.clone()]
            );

            let mut training_session = TRAINING_SESSION.clone();
            training_session.notes = "C".to_string();
            training_session.elements = vec![];

            assert_eq!(
                IndexedDB
                    .modify_training_session(
                        training_session.id,
                        Some(training_session.notes.clone()),
                        Some(training_session.elements.clone())
                    )
                    .await
                    .unwrap(),
                training_session.clone()
            );

            assert_eq!(
                IndexedDB.read_training_sessions().await.unwrap(),
                vec![training_session]
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_training_session() {
            reset().await;
            init_session().await;

            IndexedDB
                .write_training_sessions(&[TRAINING_SESSION.clone()])
                .await
                .unwrap();

            assert_eq!(
                IndexedDB.read_training_sessions().await.unwrap(),
                vec![TRAINING_SESSION.clone()]
            );

            assert_eq!(
                IndexedDB
                    .delete_training_session(TRAINING_SESSION.id)
                    .await
                    .unwrap(),
                TRAINING_SESSION.id
            );

            assert_eq!(IndexedDB.read_training_sessions().await.unwrap(), vec![]);
        }

        #[wasm_bindgen_test]
        async fn test_delete_training_session_non_existing() {
            reset().await;
            init_session().await;

            assert_eq!(
                IndexedDB
                    .delete_training_session(TRAINING_SESSION.id)
                    .await
                    .unwrap(),
                TRAINING_SESSION.id
            );
        }

        async fn init_session() {
            IndexedDB.write_session(&USER).await.unwrap();
        }

        async fn reset() {
            IndexedDB.clear_app_data().await.unwrap();
            IndexedDB.clear_session_dependent_data().await.unwrap();
        }
    }
}
