//! Cached REST
//!
//! The `REST` server acts as the authoritative data source, while `IndexedDB` serves as a local
//! cache for user-specific data. Data modifications are only possible if an active connection to
//! the server is available. Once the server has accepted a modification, a failure to update the
//! local cache is only logged; the cache is corrected by the next synchronization. Modifications
//! deliberately leave the stored `ETag` untouched, so a dropped cache write is always followed by a
//! full download instead of a `304`.

use chrono::NaiveDate;
use log::error;
use valens_domain::{self as domain, SessionRepository};

use super::{
    indexed_db::{IndexedDB, Store},
    rest::{Conditional, GlooNetSendRequest, REST, SendRequest},
};

macro_rules! sync {
    ($self:ident, $read:ident, $write:ident, $read_back:ident, $name:literal) => {{
        let etag = IndexedDB.read_etag($name).await.ok().flatten();
        match $self.rest.$read(etag.as_deref()).await {
            Ok(Conditional::Modified { data, etag }) => {
                // Persist the ETag only after the data it describes is cached, so a later 304
                // never serves stale data behind a current ETag.
                if let Err(err) = IndexedDB.$write(&data).await {
                    error!("failed to write {} into IDB: {err}", $name);
                } else if let Some(etag) = etag
                    && let Err(err) = IndexedDB.write_etag($name, &etag).await
                {
                    error!("failed to write {} etag into IDB: {err}", $name);
                }
                Ok(data)
            }
            // Reuse the cached data the server confirmed is still current.
            Ok(Conditional::NotModified) => Ok(IndexedDB.$read_back().await?),
            Err(err) => Err(err.into()),
        }
    }};
}

macro_rules! create {
    ($self: ident, $create: ident, $replace: ident, $name: literal, $($arg:expr),*) => {{
        let result = $self.rest.$create($($arg),*).await?;
        if let Err(err) = IndexedDB.$replace(result.clone()).await {
            error!("failed to update {} in IDB: {err}", $name);
        }
        Ok(result)
    }};
}

macro_rules! execute {
    ($self: ident, $method: ident, $name: literal $(, $arg:expr)*) => {{
        let result = $self.rest.$method($($arg.clone()),*).await?;
        if let Err(err) = IndexedDB.$method($($arg),*).await {
            error!("failed to update {} in IDB: {err}", $name);
        }
        Ok(result)
    }};
}

#[derive(Clone, Copy)]
pub struct CachedREST<S: SendRequest> {
    pub rest: REST<S>,
}

impl CachedREST<GlooNetSendRequest> {
    #[must_use]
    pub const fn new() -> Self {
        Self { rest: REST::new() }
    }
}

impl Default for CachedREST<GlooNetSendRequest> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: SendRequest> domain::SessionRepository for CachedREST<S> {
    async fn request_session(&self, name: domain::Name) -> Result<domain::User, domain::ReadError> {
        let rest_result = self.rest.request_session(name).await;
        if let Ok(ref user) = rest_result
            && let Err(err) = IndexedDB.write_session(user).await
        {
            error!("failed to write session into IDB: {err}");
        }

        rest_result
    }

    async fn initialize_session(&self) -> Result<domain::User, domain::ReadError> {
        IndexedDB.initialize_session().await
    }

    async fn sync_session(&self) -> Result<Option<domain::User>, domain::SyncError> {
        if let Some(user) = self.rest.sync_session().await? {
            if let Err(err) = IndexedDB.write_session(&user).await {
                error!("failed to write session into IDB: {err}");
            }
            Ok(Some(user))
        } else {
            // A missing session on the server means the user is signed out
            if let Err(err) = IndexedDB.delete_session().await {
                error!("failed to update session in IDB: {err}");
            }
            if let Err(err) = IndexedDB.clear_session_dependent_data().await {
                error!("failed to update session-dependent data in IDB: {err}");
            }
            Ok(None)
        }
    }

    async fn delete_session(&self) -> Result<domain::SignOut, domain::DeleteError> {
        self.rest.delete_session().await?;
        let mut sign_out = domain::SignOut::Complete;
        if let Err(err) = IndexedDB.delete_session().await {
            error!("failed to update session in IDB: {err}");
            sign_out = domain::SignOut::DataRetained;
        }
        if let Err(err) = IndexedDB.clear_session_dependent_data().await {
            error!("failed to update session-dependent data in IDB: {err}");
            sign_out = domain::SignOut::DataRetained;
        }
        Ok(sign_out)
    }
}

impl<S: SendRequest> domain::AuthRepository for CachedREST<S> {
    async fn read_auth_methods(&self) -> Result<Vec<domain::AuthMethod>, domain::ReadError> {
        self.rest.read_auth_methods().await
    }

    async fn login_with_passkey(&self) -> Result<domain::User, domain::ReadError> {
        let rest_result = self.rest.login_with_passkey().await;
        if let Ok(ref user) = rest_result
            && let Err(err) = IndexedDB.write_session(user).await
        {
            error!("failed to write session into IDB: {err}");
        }

        rest_result
    }

    async fn register_passkey(&self) -> Result<domain::Passkey, domain::CreateError> {
        self.rest.register_passkey().await
    }

    async fn read_passkeys(
        &self,
        user_id: domain::UserID,
    ) -> Result<Vec<domain::Passkey>, domain::ReadError> {
        self.rest.read_passkeys(user_id).await
    }

    async fn rename_passkey(
        &self,
        user_id: domain::UserID,
        id: domain::PasskeyID,
        label: domain::Name,
    ) -> Result<domain::Passkey, domain::UpdateError> {
        self.rest.rename_passkey(user_id, id, label).await
    }

    async fn delete_passkey(
        &self,
        user_id: domain::UserID,
        id: domain::PasskeyID,
    ) -> Result<(), domain::DeleteError> {
        self.rest.delete_passkey(user_id, id).await
    }

    async fn create_login_link(
        &self,
        user_id: domain::UserID,
    ) -> Result<String, domain::CreateError> {
        self.rest.create_login_link(user_id).await
    }

    async fn redeem_login_link(&self, token: String) -> Result<domain::User, domain::ReadError> {
        let rest_result = self.rest.redeem_login_link(token).await;
        if let Ok(ref user) = rest_result
            && let Err(err) = IndexedDB.write_session(user).await
        {
            error!("failed to write session into IDB: {err}");
        }

        rest_result
    }
}

impl<S: SendRequest> domain::VersionRepository for CachedREST<S> {
    async fn read_version(&self) -> Result<String, domain::ReadError> {
        self.rest.read_version().await
    }
}

impl<S: SendRequest> domain::UserRepository for CachedREST<S> {
    async fn read_users(&self) -> Result<Vec<domain::User>, domain::ReadError> {
        self.rest.read_users().await
    }

    async fn create_user(
        &self,
        name: domain::Name,
        sex: domain::Sex,
        height: Option<u8>,
        role: domain::Role,
    ) -> Result<domain::User, domain::CreateError> {
        self.rest.create_user(name, sex, height, role).await
    }

    async fn replace_user(&self, user: domain::User) -> Result<domain::User, domain::UpdateError> {
        let user = self.rest.replace_user(user).await?;
        if let Ok(session_user) = IndexedDB.initialize_session().await
            && session_user.id == user.id
            && let Err(err) = IndexedDB.write_session(&user).await
        {
            error!("failed to write session into IDB: {err}");
        }
        Ok(user)
    }

    async fn update_user(
        &self,
        id: domain::UserID,
        name: domain::Name,
        sex: domain::Sex,
        height: Option<u8>,
    ) -> Result<domain::User, domain::UpdateError> {
        let user = self.rest.update_user(id, name, sex, height).await?;
        if let Ok(session_user) = IndexedDB.initialize_session().await
            && session_user.id == user.id
            && let Err(err) = IndexedDB.write_session(&user).await
        {
            error!("failed to write session into IDB: {err}");
        }
        Ok(user)
    }

    async fn delete_user(&self, id: domain::UserID) -> Result<(), domain::DeleteError> {
        self.rest.delete_user(id).await
    }
}

impl<S: SendRequest> domain::BodyWeightRepository for CachedREST<S> {
    async fn sync_body_weight(&self) -> Result<Vec<domain::BodyWeight>, domain::SyncError> {
        sync!(
            self,
            read_body_weight_conditional,
            write_body_weight,
            read_body_weight,
            "body weight"
        )
    }

    async fn read_body_weight(&self) -> Result<Vec<domain::BodyWeight>, domain::ReadError> {
        IndexedDB.read_body_weight().await
    }

    async fn create_body_weight(
        &self,
        body_weight: domain::BodyWeight,
    ) -> Result<domain::BodyWeight, domain::CreateError> {
        create!(
            self,
            create_body_weight,
            replace_body_weight,
            "body weight",
            body_weight
        )
    }

    async fn replace_body_weight(
        &self,
        body_weight: domain::BodyWeight,
    ) -> Result<domain::BodyWeight, domain::UpdateError> {
        execute!(self, replace_body_weight, "body weight", body_weight)
    }

    async fn delete_body_weight(&self, date: NaiveDate) -> Result<(), domain::DeleteError> {
        execute!(self, delete_body_weight, "body weight", date)
    }
}

impl<S: SendRequest> domain::BodyFatRepository for CachedREST<S> {
    async fn sync_body_fat(&self) -> Result<Vec<domain::BodyFat>, domain::SyncError> {
        sync!(
            self,
            read_body_fat_conditional,
            write_body_fat,
            read_body_fat,
            "body fat"
        )
    }

    async fn read_body_fat(&self) -> Result<Vec<domain::BodyFat>, domain::ReadError> {
        IndexedDB.read_body_fat().await
    }

    async fn create_body_fat(
        &self,
        body_fat: domain::BodyFat,
    ) -> Result<domain::BodyFat, domain::CreateError> {
        create!(
            self,
            create_body_fat,
            replace_body_fat,
            "body fat",
            body_fat
        )
    }

    async fn replace_body_fat(
        &self,
        body_fat: domain::BodyFat,
    ) -> Result<domain::BodyFat, domain::UpdateError> {
        execute!(self, replace_body_fat, "body fat", body_fat)
    }

    async fn delete_body_fat(&self, date: NaiveDate) -> Result<(), domain::DeleteError> {
        execute!(self, delete_body_fat, "body fat", date)
    }
}

impl<S: SendRequest> domain::PeriodRepository for CachedREST<S> {
    async fn sync_period(&self) -> Result<Vec<domain::Period>, domain::SyncError> {
        sync!(
            self,
            read_period_conditional,
            write_period,
            read_period,
            "period"
        )
    }

    async fn read_period(&self) -> Result<Vec<domain::Period>, domain::ReadError> {
        IndexedDB.read_period().await
    }

    async fn create_period(
        &self,
        period: domain::Period,
    ) -> Result<domain::Period, domain::CreateError> {
        create!(self, create_period, replace_period, "period", period)
    }

    async fn replace_period(
        &self,
        period: domain::Period,
    ) -> Result<domain::Period, domain::UpdateError> {
        execute!(self, replace_period, "period", period)
    }

    async fn delete_period(&self, date: NaiveDate) -> Result<(), domain::DeleteError> {
        execute!(self, delete_period, "period", date)
    }
}

impl<S: SendRequest> domain::ExerciseRepository for CachedREST<S> {
    async fn sync_exercises(&self) -> Result<Vec<domain::Exercise>, domain::SyncError> {
        sync!(
            self,
            read_exercises_conditional,
            write_exercises,
            read_exercises,
            "exercises"
        )
    }

    async fn read_exercises(&self) -> Result<Vec<domain::Exercise>, domain::ReadError> {
        IndexedDB.read_exercises().await
    }

    async fn create_exercise(
        &self,
        name: domain::Name,
        muscles: Vec<domain::ExerciseMuscle>,
        force: Option<domain::Force>,
        mechanic: Option<domain::Mechanic>,
        laterality: Option<domain::Laterality>,
        assistance: Option<domain::Assistance>,
        equipment: Vec<domain::Equipment>,
        category: Option<domain::Category>,
    ) -> Result<domain::Exercise, domain::CreateError> {
        create!(
            self,
            create_exercise,
            replace_exercise,
            "exercise",
            name,
            muscles,
            force,
            mechanic,
            laterality,
            assistance,
            equipment,
            category
        )
    }

    async fn replace_exercise(
        &self,
        exercise: domain::Exercise,
    ) -> Result<domain::Exercise, domain::UpdateError> {
        execute!(self, replace_exercise, "exercise", exercise)
    }

    async fn delete_exercise(&self, id: domain::ExerciseID) -> Result<(), domain::DeleteError> {
        execute!(self, delete_exercise, "exercise", id)
    }
}

impl<S: SendRequest> domain::RoutineRepository for CachedREST<S> {
    async fn sync_routines(&self) -> Result<Vec<domain::Routine>, domain::SyncError> {
        sync!(
            self,
            read_routines_conditional,
            write_routines,
            read_routines,
            "routines"
        )
    }

    async fn read_routines(&self) -> Result<Vec<domain::Routine>, domain::ReadError> {
        IndexedDB.read_routines().await
    }

    async fn create_routine(
        &self,
        name: domain::Name,
        sections: Vec<domain::RoutinePart>,
    ) -> Result<domain::Routine, domain::CreateError> {
        let routine = self.rest.create_routine(name, sections).await?;
        if let Err(err) = IndexedDB
            .put(
                Store::Routines,
                super::indexed_db::Routine::from(&routine),
                (),
            )
            .await
        {
            error!("failed to update routine in IDB: {err}");
        }
        Ok(routine)
    }

    async fn modify_routine(
        &self,
        id: domain::RoutineID,
        name: Option<domain::Name>,
        archived: Option<bool>,
        sections: Option<Vec<domain::RoutinePart>>,
    ) -> Result<domain::Routine, domain::UpdateError> {
        execute!(
            self,
            modify_routine,
            "routine",
            id,
            name,
            archived,
            sections
        )
    }

    async fn delete_routine(&self, id: domain::RoutineID) -> Result<(), domain::DeleteError> {
        execute!(self, delete_routine, "routine", id)
    }
}

impl<S: SendRequest> domain::ScheduleRepository for CachedREST<S> {
    async fn sync_schedule(&self) -> Result<domain::Schedule, domain::SyncError> {
        sync!(
            self,
            read_schedule_conditional,
            write_schedule,
            read_schedule,
            "schedule"
        )
    }

    async fn read_schedule(&self) -> Result<domain::Schedule, domain::ReadError> {
        IndexedDB.read_schedule().await
    }

    async fn replace_schedule(
        &self,
        schedule: domain::Schedule,
    ) -> Result<domain::Schedule, domain::UpdateError> {
        execute!(self, replace_schedule, "schedule", schedule)
    }
}

impl<S: SendRequest> domain::TrainingSessionRepository for CachedREST<S> {
    async fn sync_training_sessions(
        &self,
    ) -> Result<Vec<domain::TrainingSession>, domain::SyncError> {
        sync!(
            self,
            read_training_sessions_conditional,
            write_training_sessions,
            read_training_sessions,
            "training sessions"
        )
    }

    async fn read_training_sessions(
        &self,
    ) -> Result<Vec<domain::TrainingSession>, domain::ReadError> {
        IndexedDB.read_training_sessions().await
    }

    async fn create_training_session(
        &self,
        routine_id: domain::RoutineID,
        date: NaiveDate,
        notes: String,
        elements: Vec<domain::TrainingSessionElement>,
    ) -> Result<domain::TrainingSession, domain::CreateError> {
        let training_session = self
            .rest
            .create_training_session(routine_id, date, notes, elements)
            .await?;
        if let Err(err) = IndexedDB
            .put(
                Store::TrainingSessions,
                super::indexed_db::TrainingSession::from(&training_session),
                (),
            )
            .await
        {
            error!("failed to update training session in IDB: {err}");
        }
        Ok(training_session)
    }

    async fn modify_training_session(
        &self,
        id: domain::TrainingSessionID,
        notes: Option<String>,
        elements: Option<Vec<domain::TrainingSessionElement>>,
        exercise_notes: Option<std::collections::BTreeMap<domain::ExerciseID, String>>,
    ) -> Result<domain::TrainingSession, domain::UpdateError> {
        execute!(
            self,
            modify_training_session,
            "training session",
            id,
            notes,
            elements,
            exercise_notes
        )
    }

    async fn delete_training_session(
        &self,
        id: domain::TrainingSessionID,
    ) -> Result<(), domain::DeleteError> {
        execute!(self, delete_training_session, "training session", id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    mod wasm {
        use std::sync::{Arc, Mutex};

        use pretty_assertions::assert_eq;
        use serde_json::json;
        use valens_domain::{
            BodyFatRepository, BodyWeightRepository, ExerciseRepository, PeriodRepository,
            RoutineRepository, ScheduleRepository, SessionRepository, TrainingSessionRepository,
            UserRepository, VersionRepository,
        };
        use wasm_bindgen_test::wasm_bindgen_test;

        use crate::{
            rest,
            tests::data::{
                BODY_FAT, BODY_FATS, BODY_WEIGHT, BODY_WEIGHTS, EXERCISE, EXERCISES, PERIOD,
                PERIODS, ROUTINE, ROUTINES, SCHEDULE, TRAINING_SESSION, TRAINING_SESSIONS, USER,
                USER_2, USERS,
            },
        };

        use super::*;

        #[wasm_bindgen_test]
        async fn test_request_session() {
            reset_cache().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .request_session(USER.name.clone())
                    .await,
                Err(domain::ReadError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert!(matches!(
                IndexedDB.initialize_session().await,
                Err(domain::ReadError::Storage(domain::StorageError::NoSession))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::User::from(USER.clone())),
                ))
                .request_session(USER.name.clone())
                .await
                .unwrap(),
                USER.clone()
            );

            assert_eq!(IndexedDB.initialize_session().await.unwrap(), USER.clone());

            assert!(matches!(
                cached_rest_with_response(None)
                    .request_session(USER_2.name.clone())
                    .await,
                Err(domain::ReadError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(IndexedDB.initialize_session().await.unwrap(), USER.clone());
        }

        #[wasm_bindgen_test]
        async fn test_initialize_session() {
            reset_cache().await;

            assert!(matches!(
                cached_rest_with_response(None).initialize_session().await,
                Err(domain::ReadError::Storage(domain::StorageError::NoSession))
            ));

            IndexedDB.write_session(&USER).await.unwrap();

            assert_eq!(
                cached_rest_with_response(None)
                    .initialize_session()
                    .await
                    .unwrap(),
                USER.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_sync_session() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None).sync_session().await,
                Err(domain::SyncError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(IndexedDB.initialize_session().await.unwrap(), USER.clone());

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::User::from(USER_2.clone())),
                ))
                .sync_session()
                .await
                .unwrap(),
                Some(USER_2.clone())
            );

            assert_eq!(
                IndexedDB.initialize_session().await.unwrap(),
                USER_2.clone()
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(404)
                        .body::<Option<&str>>(None),
                ))
                .sync_session()
                .await
                .unwrap(),
                None
            );

            assert!(matches!(
                IndexedDB.initialize_session().await,
                Err(domain::ReadError::Storage(domain::StorageError::NoSession))
            ));
        }

        #[wasm_bindgen_test]
        async fn test_delete_session() {
            reset_cache().await;

            IndexedDB.write_session(&USER).await.unwrap();

            assert!(matches!(
                cached_rest_with_response(None).delete_session().await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(IndexedDB.initialize_session().await.unwrap(), USER.clone());

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_session()
                .await
                .unwrap(),
                domain::SignOut::Complete
            );

            assert!(matches!(
                IndexedDB.initialize_session().await,
                Err(domain::ReadError::Storage(domain::StorageError::NoSession))
            ));
        }

        #[wasm_bindgen_test]
        async fn test_delete_session_non_existing() {
            reset_cache().await;

            assert!(matches!(
                cached_rest_with_response(None).delete_session().await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_session()
                .await
                .unwrap(),
                domain::SignOut::Complete
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_version() {
            assert!(matches!(
                cached_rest_with_response(None).read_version().await,
                Err(domain::ReadError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
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
            reset_cache().await;

            assert!(matches!(
                cached_rest_with_response(None).read_users().await,
                Err(domain::ReadError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder().status(200).json(
                        &USERS
                            .iter()
                            .cloned()
                            .map(rest::User::from)
                            .collect::<Vec<_>>()
                    )
                ))
                .read_users()
                .await
                .unwrap(),
                USERS.to_vec()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_user() {
            reset_cache().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .create_user(USER.name.clone(), USER.sex, USER.height, USER.role)
                    .await,
                Err(domain::CreateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::User::from(USER.clone()))
                ))
                .create_user(USER.name.clone(), USER.sex, USER.height, USER.role)
                .await
                .unwrap(),
                USER.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_replace_user() {
            reset_cache().await;
            init_session().await;

            let mut user = USER.clone();
            user.name = domain::Name::new("C").unwrap();
            user.height = Some(170);

            assert!(matches!(
                cached_rest_with_response(None)
                    .replace_user(user.clone())
                    .await,
                Err(domain::UpdateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(IndexedDB.initialize_session().await.unwrap(), USER.clone());

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::User::from(user.clone()))
                ))
                .replace_user(user.clone())
                .await
                .unwrap(),
                user.clone()
            );

            assert_eq!(IndexedDB.initialize_session().await.unwrap(), user);
        }

        #[wasm_bindgen_test]
        async fn test_update_user() {
            reset_cache().await;
            init_session().await;

            let mut user = USER.clone();
            user.name = domain::Name::new("C").unwrap();
            user.height = Some(170);

            assert!(matches!(
                cached_rest_with_response(None)
                    .update_user(user.id, user.name.clone(), user.sex, user.height)
                    .await,
                Err(domain::UpdateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(IndexedDB.initialize_session().await.unwrap(), USER.clone());

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::User::from(user.clone()))
                ))
                .update_user(user.id, user.name.clone(), user.sex, user.height)
                .await
                .unwrap(),
                user.clone()
            );

            assert_eq!(IndexedDB.initialize_session().await.unwrap(), user);
        }

        #[wasm_bindgen_test]
        async fn test_update_user_keeps_session_of_other_user() {
            reset_cache().await;
            init_session().await;

            let mut user = USER_2.clone();
            user.height = Some(190);

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::User::from(user.clone()))
                ))
                .update_user(user.id, user.name.clone(), user.sex, user.height)
                .await
                .unwrap()
                .id,
                USER_2.id
            );

            assert_eq!(IndexedDB.initialize_session().await.unwrap(), USER.clone());
        }

        #[wasm_bindgen_test]
        async fn test_replace_user_keeps_session_of_other_user() {
            reset_cache().await;
            init_session().await;

            let mut user = USER_2.clone();
            user.height = Some(190);

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::User::from(user.clone()))
                ))
                .replace_user(user)
                .await
                .unwrap()
                .id,
                USER_2.id
            );

            assert_eq!(IndexedDB.initialize_session().await.unwrap(), USER.clone());
        }

        #[wasm_bindgen_test]
        async fn test_delete_user() {
            reset_cache().await;

            assert!(matches!(
                cached_rest_with_response(None).delete_user(USER.id).await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_user(USER.id)
                .await
                .unwrap(),
                ()
            );
        }

        #[wasm_bindgen_test]
        async fn test_sync_body_weight() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None).sync_body_weight().await,
                Err(domain::SyncError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder().status(200).json(
                        &BODY_WEIGHTS
                            .iter()
                            .cloned()
                            .map(rest::BodyWeight::from)
                            .collect::<Vec<_>>()
                    )
                ))
                .sync_body_weight()
                .await
                .unwrap(),
                BODY_WEIGHTS.to_vec()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_weight()
                    .await
                    .unwrap(),
                BODY_WEIGHTS.to_vec()
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&[rest::BodyWeight::from(BODY_WEIGHT)])
                ))
                .sync_body_weight()
                .await
                .unwrap(),
                vec![BODY_WEIGHT]
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_weight()
                    .await
                    .unwrap(),
                vec![BODY_WEIGHT]
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_body_weight() {
            reset_cache().await;
            init_session().await;

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_weight()
                    .await
                    .unwrap(),
                vec![]
            );

            IndexedDB.write_body_weight(BODY_WEIGHTS).await.unwrap();

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_weight()
                    .await
                    .unwrap(),
                BODY_WEIGHTS.to_vec()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_body_weight() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .create_body_weight(BODY_WEIGHT)
                    .await,
                Err(domain::CreateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::BodyWeight::from(BODY_WEIGHT)),
                ))
                .create_body_weight(BODY_WEIGHT)
                .await
                .unwrap(),
                BODY_WEIGHT
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_weight()
                    .await
                    .unwrap(),
                vec![BODY_WEIGHT]
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_body_weight_conflict() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .create_body_weight(BODY_WEIGHT)
                    .await,
                Err(domain::CreateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            IndexedDB.write_body_weight(&[BODY_WEIGHT]).await.unwrap();

            let mut body_weight = BODY_WEIGHT;
            body_weight.weight += 1.0;

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::BodyWeight::from(body_weight.clone())),
                ))
                .create_body_weight(body_weight.clone())
                .await
                .unwrap(),
                body_weight
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_weight()
                    .await
                    .unwrap(),
                vec![body_weight]
            );
        }

        #[wasm_bindgen_test]
        async fn test_replace_body_weight() {
            reset_cache().await;
            init_session().await;

            IndexedDB.write_body_weight(&[BODY_WEIGHT]).await.unwrap();

            let mut body_weight = BODY_WEIGHT;
            body_weight.weight += 1.0;

            assert!(matches!(
                cached_rest_with_response(None)
                    .replace_body_weight(body_weight.clone())
                    .await,
                Err(domain::UpdateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_weight()
                    .await
                    .unwrap(),
                vec![BODY_WEIGHT]
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::BodyWeight::from(body_weight.clone())),
                ))
                .replace_body_weight(body_weight.clone())
                .await
                .unwrap(),
                body_weight.clone()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_weight()
                    .await
                    .unwrap(),
                vec![body_weight]
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_body_weight() {
            reset_cache().await;
            init_session().await;

            IndexedDB.write_body_weight(&[BODY_WEIGHT]).await.unwrap();

            assert!(matches!(
                cached_rest_with_response(None)
                    .delete_body_weight(BODY_WEIGHT.date)
                    .await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_weight()
                    .await
                    .unwrap(),
                vec![BODY_WEIGHT]
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_body_weight(BODY_WEIGHT.date)
                .await
                .unwrap(),
                ()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_weight()
                    .await
                    .unwrap(),
                vec![]
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_body_weight_non_existing() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .delete_body_weight(BODY_WEIGHT.date)
                    .await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_body_weight(BODY_WEIGHT.date)
                .await
                .unwrap(),
                ()
            );
        }

        #[wasm_bindgen_test]
        async fn test_sync_body_fat() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None).sync_body_fat().await,
                Err(domain::SyncError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder().status(200).json(
                        &BODY_FATS
                            .iter()
                            .cloned()
                            .map(rest::BodyFat::from)
                            .collect::<Vec<_>>()
                    )
                ))
                .sync_body_fat()
                .await
                .unwrap(),
                BODY_FATS.to_vec()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_fat()
                    .await
                    .unwrap(),
                BODY_FATS.to_vec()
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&[rest::BodyFat::from(BODY_FAT)])
                ))
                .sync_body_fat()
                .await
                .unwrap(),
                vec![BODY_FAT]
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_fat()
                    .await
                    .unwrap(),
                vec![BODY_FAT]
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_body_fat() {
            reset_cache().await;
            init_session().await;

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_fat()
                    .await
                    .unwrap(),
                vec![]
            );

            IndexedDB.write_body_fat(BODY_FATS).await.unwrap();

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_fat()
                    .await
                    .unwrap(),
                BODY_FATS.to_vec()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_body_fat() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .create_body_fat(BODY_FAT)
                    .await,
                Err(domain::CreateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::BodyFat::from(BODY_FAT)),
                ))
                .create_body_fat(BODY_FAT)
                .await
                .unwrap(),
                BODY_FAT
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_fat()
                    .await
                    .unwrap(),
                vec![BODY_FAT]
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_body_fat_conflict() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .create_body_fat(BODY_FAT)
                    .await,
                Err(domain::CreateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            IndexedDB.write_body_fat(&[BODY_FAT]).await.unwrap();

            let mut body_fat = BODY_FAT;
            body_fat.chest = body_fat.chest.map(|v| v + 1);

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::BodyFat::from(body_fat.clone())),
                ))
                .create_body_fat(body_fat.clone())
                .await
                .unwrap(),
                body_fat
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_fat()
                    .await
                    .unwrap(),
                vec![body_fat]
            );
        }

        #[wasm_bindgen_test]
        async fn test_replace_body_fat() {
            reset_cache().await;
            init_session().await;

            IndexedDB.write_body_fat(&[BODY_FAT]).await.unwrap();

            let mut body_fat = BODY_FAT;
            body_fat.chest = body_fat.chest.map(|v| v + 1);

            assert!(matches!(
                cached_rest_with_response(None)
                    .replace_body_fat(body_fat.clone())
                    .await,
                Err(domain::UpdateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_fat()
                    .await
                    .unwrap(),
                vec![BODY_FAT]
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::BodyFat::from(body_fat.clone())),
                ))
                .replace_body_fat(body_fat.clone())
                .await
                .unwrap(),
                body_fat.clone()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_fat()
                    .await
                    .unwrap(),
                vec![body_fat]
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_body_fat() {
            reset_cache().await;
            init_session().await;

            IndexedDB.write_body_fat(&[BODY_FAT]).await.unwrap();

            assert!(matches!(
                cached_rest_with_response(None)
                    .delete_body_fat(BODY_FAT.date)
                    .await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_fat()
                    .await
                    .unwrap(),
                vec![BODY_FAT]
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_body_fat(BODY_FAT.date)
                .await
                .unwrap(),
                ()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_body_fat()
                    .await
                    .unwrap(),
                vec![]
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_body_fat_non_existing() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .delete_body_fat(BODY_FAT.date)
                    .await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_body_fat(BODY_FAT.date)
                .await
                .unwrap(),
                ()
            );
        }

        #[wasm_bindgen_test]
        async fn test_sync_period() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None).sync_period().await,
                Err(domain::SyncError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder().status(200).json(
                        &PERIODS
                            .iter()
                            .cloned()
                            .map(rest::Period::from)
                            .collect::<Vec<_>>()
                    )
                ))
                .sync_period()
                .await
                .unwrap(),
                PERIODS.to_vec()
            );

            assert_eq!(
                cached_rest_with_response(None).read_period().await.unwrap(),
                PERIODS.to_vec()
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&[rest::Period::from(PERIOD)])
                ))
                .sync_period()
                .await
                .unwrap(),
                vec![PERIOD]
            );

            assert_eq!(
                cached_rest_with_response(None).read_period().await.unwrap(),
                vec![PERIOD]
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_period() {
            reset_cache().await;
            init_session().await;

            assert_eq!(
                cached_rest_with_response(None).read_period().await.unwrap(),
                vec![]
            );

            IndexedDB.write_period(PERIODS).await.unwrap();

            assert_eq!(
                cached_rest_with_response(None).read_period().await.unwrap(),
                PERIODS.to_vec()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_period() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None).create_period(PERIOD).await,
                Err(domain::CreateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::Period::from(PERIOD)),
                ))
                .create_period(PERIOD)
                .await
                .unwrap(),
                PERIOD
            );

            assert_eq!(
                cached_rest_with_response(None).read_period().await.unwrap(),
                vec![PERIOD]
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_period_conflict() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None).create_period(PERIOD).await,
                Err(domain::CreateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            IndexedDB.write_period(&[PERIOD]).await.unwrap();

            let mut period = PERIOD;
            period.intensity = domain::Intensity::Heavy;

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::Period::from(period.clone())),
                ))
                .create_period(period.clone())
                .await
                .unwrap(),
                period
            );

            assert_eq!(
                cached_rest_with_response(None).read_period().await.unwrap(),
                vec![period]
            );
        }

        #[wasm_bindgen_test]
        async fn test_replace_period() {
            reset_cache().await;
            init_session().await;

            IndexedDB.write_period(&[PERIOD]).await.unwrap();

            let mut period = PERIOD;
            period.intensity = domain::Intensity::Heavy;

            assert!(matches!(
                cached_rest_with_response(None)
                    .replace_period(period.clone())
                    .await,
                Err(domain::UpdateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(None).read_period().await.unwrap(),
                vec![PERIOD]
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::Period::from(period.clone())),
                ))
                .replace_period(period.clone())
                .await
                .unwrap(),
                period.clone()
            );

            assert_eq!(
                cached_rest_with_response(None).read_period().await.unwrap(),
                vec![period]
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_period() {
            reset_cache().await;
            init_session().await;

            IndexedDB.write_period(&[PERIOD]).await.unwrap();

            assert!(matches!(
                cached_rest_with_response(None)
                    .delete_period(PERIOD.date)
                    .await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(None).read_period().await.unwrap(),
                vec![PERIOD]
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_period(PERIOD.date)
                .await
                .unwrap(),
                ()
            );

            assert_eq!(
                cached_rest_with_response(None).read_period().await.unwrap(),
                vec![]
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_period_non_existing() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .delete_period(PERIOD.date)
                    .await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_period(PERIOD.date)
                .await
                .unwrap(),
                ()
            );
        }

        #[wasm_bindgen_test]
        async fn test_sync_exercises() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None).sync_exercises().await,
                Err(domain::SyncError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder().status(200).json(
                        &EXERCISES
                            .iter()
                            .cloned()
                            .map(rest::Exercise::from)
                            .collect::<Vec<_>>()
                    )
                ))
                .sync_exercises()
                .await
                .unwrap(),
                EXERCISES.to_vec()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_exercises()
                    .await
                    .unwrap(),
                EXERCISES.to_vec()
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&[rest::Exercise::from(EXERCISE.clone())])
                ))
                .sync_exercises()
                .await
                .unwrap(),
                vec![EXERCISE.clone()]
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_exercises()
                    .await
                    .unwrap(),
                vec![EXERCISE.clone()]
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_exercises() {
            reset_cache().await;
            init_session().await;

            assert_eq!(
                cached_rest_with_response(None)
                    .read_exercises()
                    .await
                    .unwrap(),
                vec![]
            );

            IndexedDB.write_exercises(&EXERCISES).await.unwrap();

            assert_eq!(
                cached_rest_with_response(None)
                    .read_exercises()
                    .await
                    .unwrap(),
                EXERCISES.to_vec()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_exercise() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .create_exercise(
                        EXERCISE.name.clone(),
                        EXERCISE.muscles.clone(),
                        EXERCISE.force,
                        EXERCISE.mechanic,
                        EXERCISE.laterality,
                        EXERCISE.assistance,
                        EXERCISE.equipment.clone(),
                        EXERCISE.category,
                    )
                    .await,
                Err(domain::CreateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::Exercise::from(EXERCISE.clone())),
                ))
                .create_exercise(
                    EXERCISE.name.clone(),
                    EXERCISE.muscles.clone(),
                    EXERCISE.force,
                    EXERCISE.mechanic,
                    EXERCISE.laterality,
                    EXERCISE.assistance,
                    EXERCISE.equipment.clone(),
                    EXERCISE.category,
                )
                .await
                .unwrap(),
                EXERCISE.clone()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_exercises()
                    .await
                    .unwrap(),
                vec![EXERCISE.clone()]
            );
        }

        #[wasm_bindgen_test]
        async fn test_replace_exercise() {
            reset_cache().await;
            init_session().await;

            IndexedDB
                .write_exercises(std::slice::from_ref(&EXERCISE))
                .await
                .unwrap();

            let mut exercise = EXERCISE.clone();
            exercise.name = domain::Name::new("C").unwrap();

            assert!(matches!(
                cached_rest_with_response(None)
                    .replace_exercise(exercise.clone())
                    .await,
                Err(domain::UpdateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(None)
                    .read_exercises()
                    .await
                    .unwrap(),
                vec![EXERCISE.clone()]
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::Exercise::from(exercise.clone())),
                ))
                .replace_exercise(exercise.clone())
                .await
                .unwrap(),
                exercise.clone()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_exercises()
                    .await
                    .unwrap(),
                vec![exercise]
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_exercise() {
            reset_cache().await;
            init_session().await;

            IndexedDB
                .write_exercises(std::slice::from_ref(&EXERCISE))
                .await
                .unwrap();

            assert!(matches!(
                cached_rest_with_response(None)
                    .delete_exercise(EXERCISE.id)
                    .await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(None)
                    .read_exercises()
                    .await
                    .unwrap(),
                vec![EXERCISE.clone()]
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_exercise(EXERCISE.id)
                .await
                .unwrap(),
                ()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_exercises()
                    .await
                    .unwrap(),
                vec![]
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_exercise_non_existing() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .delete_exercise(EXERCISE.id)
                    .await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_exercise(EXERCISE.id)
                .await
                .unwrap(),
                ()
            );
        }

        #[wasm_bindgen_test]
        async fn test_sync_routines() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None).sync_routines().await,
                Err(domain::SyncError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder().status(200).json(
                        &ROUTINES
                            .iter()
                            .cloned()
                            .map(rest::Routine::from)
                            .collect::<Vec<_>>()
                    )
                ))
                .sync_routines()
                .await
                .unwrap(),
                ROUTINES.to_vec()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_routines()
                    .await
                    .unwrap(),
                ROUTINES.to_vec()
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&[rest::Routine::from(ROUTINE.clone())])
                ))
                .sync_routines()
                .await
                .unwrap(),
                vec![ROUTINE.clone()]
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_routines()
                    .await
                    .unwrap(),
                vec![ROUTINE.clone()]
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_routines() {
            reset_cache().await;
            init_session().await;

            assert_eq!(
                cached_rest_with_response(None)
                    .read_routines()
                    .await
                    .unwrap(),
                vec![]
            );

            IndexedDB.write_routines(&ROUTINES).await.unwrap();

            assert_eq!(
                cached_rest_with_response(None)
                    .read_routines()
                    .await
                    .unwrap(),
                ROUTINES.to_vec()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_routine() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .create_routine(ROUTINE.name.clone(), ROUTINE.sections.clone())
                    .await,
                Err(domain::CreateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::Routine::from(ROUTINE.clone())),
                ))
                .create_routine(ROUTINE.name.clone(), ROUTINE.sections.clone())
                .await
                .unwrap(),
                ROUTINE.clone()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_routines()
                    .await
                    .unwrap(),
                vec![ROUTINE.clone()]
            );
        }

        #[wasm_bindgen_test]
        async fn test_modify_routine() {
            reset_cache().await;
            init_session().await;

            IndexedDB
                .write_routines(std::slice::from_ref(&ROUTINE))
                .await
                .unwrap();

            let mut routine = ROUTINE.clone();
            routine.name = domain::Name::new("C").unwrap();
            routine.archived = true;
            routine.sections = vec![];

            assert!(matches!(
                cached_rest_with_response(None)
                    .modify_routine(
                        routine.id,
                        Some(routine.name.clone()),
                        Some(routine.archived),
                        Some(routine.sections.clone())
                    )
                    .await,
                Err(domain::UpdateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(None)
                    .read_routines()
                    .await
                    .unwrap(),
                vec![ROUTINE.clone()]
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::Routine::from(routine.clone())),
                ))
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

            assert_eq!(
                cached_rest_with_response(None)
                    .read_routines()
                    .await
                    .unwrap(),
                vec![routine]
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_routine() {
            reset_cache().await;
            init_session().await;

            IndexedDB
                .write_routines(std::slice::from_ref(&ROUTINE))
                .await
                .unwrap();

            assert!(matches!(
                cached_rest_with_response(None)
                    .delete_routine(ROUTINE.id)
                    .await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(None)
                    .read_routines()
                    .await
                    .unwrap(),
                vec![ROUTINE.clone()]
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_routine(ROUTINE.id)
                .await
                .unwrap(),
                ()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_routines()
                    .await
                    .unwrap(),
                vec![]
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_routine_non_existing() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .delete_routine(ROUTINE.id)
                    .await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_routine(ROUTINE.id)
                .await
                .unwrap(),
                ()
            );
        }

        #[wasm_bindgen_test]
        async fn test_sync_schedule() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None).sync_schedule().await,
                Err(domain::SyncError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::Schedule::from(SCHEDULE.clone()))
                ))
                .sync_schedule()
                .await
                .unwrap(),
                SCHEDULE.clone()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_schedule()
                    .await
                    .unwrap(),
                SCHEDULE.clone()
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::Schedule::from(domain::Schedule::default()))
                ))
                .sync_schedule()
                .await
                .unwrap(),
                domain::Schedule::default()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_schedule()
                    .await
                    .unwrap(),
                domain::Schedule::default()
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_schedule() {
            reset_cache().await;
            init_session().await;

            assert_eq!(
                cached_rest_with_response(None)
                    .read_schedule()
                    .await
                    .unwrap(),
                domain::Schedule::default()
            );

            IndexedDB.write_schedule(&SCHEDULE).await.unwrap();

            assert_eq!(
                cached_rest_with_response(None)
                    .read_schedule()
                    .await
                    .unwrap(),
                SCHEDULE.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_replace_schedule() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .replace_schedule(SCHEDULE.clone())
                    .await,
                Err(domain::UpdateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(None)
                    .read_schedule()
                    .await
                    .unwrap(),
                domain::Schedule::default()
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::Schedule::from(SCHEDULE.clone())),
                ))
                .replace_schedule(SCHEDULE.clone())
                .await
                .unwrap(),
                SCHEDULE.clone()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_schedule()
                    .await
                    .unwrap(),
                SCHEDULE.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_sync_training_sessions() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .sync_training_sessions()
                    .await,
                Err(domain::SyncError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder().status(200).json(
                        &TRAINING_SESSIONS
                            .iter()
                            .cloned()
                            .map(rest::TrainingSession::from)
                            .collect::<Vec<_>>()
                    )
                ))
                .sync_training_sessions()
                .await
                .unwrap(),
                TRAINING_SESSIONS.to_vec()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_training_sessions()
                    .await
                    .unwrap(),
                TRAINING_SESSIONS.to_vec()
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&[rest::TrainingSession::from(TRAINING_SESSION.clone())])
                ))
                .sync_training_sessions()
                .await
                .unwrap(),
                vec![TRAINING_SESSION.clone()]
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_training_sessions()
                    .await
                    .unwrap(),
                vec![TRAINING_SESSION.clone()]
            );
        }

        #[wasm_bindgen_test]
        async fn test_read_training_sessions() {
            reset_cache().await;
            init_session().await;

            assert_eq!(
                cached_rest_with_response(None)
                    .read_training_sessions()
                    .await
                    .unwrap(),
                vec![]
            );

            IndexedDB
                .write_training_sessions(&TRAINING_SESSIONS)
                .await
                .unwrap();

            assert_eq!(
                cached_rest_with_response(None)
                    .read_training_sessions()
                    .await
                    .unwrap(),
                TRAINING_SESSIONS.to_vec()
            );
        }

        #[wasm_bindgen_test]
        async fn test_create_training_session() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .create_training_session(
                        TRAINING_SESSION.routine_id,
                        TRAINING_SESSION.date,
                        TRAINING_SESSION.notes.clone(),
                        TRAINING_SESSION.elements.clone()
                    )
                    .await,
                Err(domain::CreateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::TrainingSession::from(TRAINING_SESSION.clone())),
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

            assert_eq!(
                cached_rest_with_response(None)
                    .read_training_sessions()
                    .await
                    .unwrap(),
                vec![TRAINING_SESSION.clone()]
            );
        }

        #[wasm_bindgen_test]
        async fn test_modify_training_session() {
            reset_cache().await;
            init_session().await;

            IndexedDB
                .write_training_sessions(std::slice::from_ref(&TRAINING_SESSION))
                .await
                .unwrap();

            let mut training_session = TRAINING_SESSION.clone();
            training_session.notes = "C".to_string();
            training_session.elements = vec![];

            assert!(matches!(
                cached_rest_with_response(None)
                    .modify_training_session(
                        training_session.id,
                        Some(training_session.notes.clone()),
                        Some(training_session.elements.clone()),
                        None
                    )
                    .await,
                Err(domain::UpdateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(None)
                    .read_training_sessions()
                    .await
                    .unwrap(),
                vec![TRAINING_SESSION.clone()]
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::TrainingSession::from(training_session.clone())),
                ))
                .modify_training_session(
                    training_session.id,
                    Some(training_session.notes.clone()),
                    Some(training_session.elements.clone()),
                    None
                )
                .await
                .unwrap(),
                training_session.clone()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_training_sessions()
                    .await
                    .unwrap(),
                vec![training_session]
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_training_session() {
            reset_cache().await;
            init_session().await;

            IndexedDB
                .write_training_sessions(std::slice::from_ref(&TRAINING_SESSION))
                .await
                .unwrap();

            assert!(matches!(
                cached_rest_with_response(None)
                    .delete_training_session(TRAINING_SESSION.id)
                    .await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(None)
                    .read_training_sessions()
                    .await
                    .unwrap(),
                vec![TRAINING_SESSION.clone()]
            );

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_training_session(TRAINING_SESSION.id)
                .await
                .unwrap(),
                ()
            );

            assert_eq!(
                cached_rest_with_response(None)
                    .read_training_sessions()
                    .await
                    .unwrap(),
                vec![]
            );
        }

        #[wasm_bindgen_test]
        async fn test_delete_training_session_non_existing() {
            reset_cache().await;
            init_session().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .delete_training_session(TRAINING_SESSION.id)
                    .await,
                Err(domain::DeleteError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .body::<Option<&str>>(None),
                ))
                .delete_training_session(TRAINING_SESSION.id)
                .await
                .unwrap(),
                ()
            );
        }

        #[wasm_bindgen_test]
        async fn test_sync_stores_etag_and_sends_if_none_match() {
            reset_cache().await;
            init_session().await;

            // A response carrying an ETag caches the data and persists the ETag
            assert_eq!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .header("etag", "W/\"body_weight-1\"")
                        .json(&[rest::BodyWeight::from(BODY_WEIGHT)])
                ))
                .sync_body_weight()
                .await
                .unwrap(),
                vec![BODY_WEIGHT]
            );

            assert_eq!(
                IndexedDB.read_etag("body weight").await.unwrap(),
                Some("W/\"body_weight-1\"".to_string())
            );

            // The next synchronization sends the stored ETag; a 304 keeps the cached data without
            // rewriting it
            let (cached_rest, request) = cached_rest_capturing(Some(
                gloo_net::http::Response::builder()
                    .status(304)
                    .body::<Option<&str>>(None),
            ));

            assert_eq!(
                cached_rest.sync_body_weight().await.unwrap(),
                vec![BODY_WEIGHT]
            );

            assert_eq!(
                request
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap()
                    .headers()
                    .get("If-None-Match"),
                Some("W/\"body_weight-1\"".to_string())
            );
        }

        #[wasm_bindgen_test]
        async fn test_sync_schedule_not_modified_keeps_cache() {
            reset_cache().await;
            init_session().await;

            IndexedDB.write_schedule(&SCHEDULE).await.unwrap();
            IndexedDB
                .write_etag("schedule", "W/\"schedule-1\"")
                .await
                .unwrap();

            let (cached_rest, request) = cached_rest_capturing(Some(
                gloo_net::http::Response::builder()
                    .status(304)
                    .body::<Option<&str>>(None),
            ));

            assert_eq!(cached_rest.sync_schedule().await.unwrap(), SCHEDULE.clone());

            assert_eq!(
                request
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap()
                    .headers()
                    .get("If-None-Match"),
                Some("W/\"schedule-1\"".to_string())
            );
        }

        async fn init_session() {
            IndexedDB.write_session(&USER).await.unwrap();
        }

        async fn reset_cache() {
            IndexedDB.clear_app_data().await.unwrap();
            IndexedDB.clear_session_dependent_data().await.unwrap();
        }

        fn cached_rest_with_response(
            response: Option<Result<gloo_net::http::Response, gloo_net::Error>>,
        ) -> CachedREST<MockSendRequest> {
            cached_rest_capturing(response).0
        }

        #[allow(clippy::type_complexity)]
        fn cached_rest_capturing(
            response: Option<Result<gloo_net::http::Response, gloo_net::Error>>,
        ) -> (
            CachedREST<MockSendRequest>,
            Arc<Mutex<Option<gloo_net::http::Request>>>,
        ) {
            #[allow(clippy::arc_with_non_send_sync)]
            let request = Arc::new(Mutex::new(None));
            let sender = MockSendRequest {
                request: Arc::clone(&request),
                #[allow(clippy::arc_with_non_send_sync)]
                response: Arc::new(Mutex::new(response)),
            };
            (
                CachedREST {
                    rest: REST { sender },
                },
                request,
            )
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
                    .unwrap_or(Err(gloo_net::Error::GlooError("no response".to_string())))
            }
        }
    }
}
