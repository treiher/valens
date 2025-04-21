//! Cached REST
//!
//! The `REST` server acts as the authoritative data source, while `IndexedDB` serves as a local
//! cache for user-specific data. Data modifications are only possible if an active connection to
//! the server is available.

use chrono::NaiveDate;
use log::error;
use valens_domain as domain;

use super::indexed_db::{IndexedDB, Store};
use super::rest::{GlooNetSendRequest, REST, SendRequest};

macro_rules! sync {
    ($self: ident, $read: ident, $write: ident, $name: literal) => {{
        let rest_result = $self.rest.$read().await;
        if let Ok(ref result) = rest_result {
            if let Err(err) = IndexedDB.$write(result).await {
                error!("failed to write {} into IDB: {err}", $name);
            }
        }

        Ok(rest_result?)
    }};
}

macro_rules! create {
    ($self: ident, $create: ident, $replace: ident, $($arg:expr),*) => {{
        let result = $self.rest.$create($($arg),*).await?;
        Ok(IndexedDB.$replace(result).await?)
    }};
}

macro_rules! execute {
    ($self: ident, $method: ident, $($arg:expr),*) => {{
        $self.rest.$method($($arg.clone()),*).await?;
        IndexedDB.$method($($arg),*).await
    }};
}

#[derive(Clone)]
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
    #[must_use]
    fn default() -> Self {
        Self::new()
    }
}

impl<S: SendRequest> domain::SessionRepository for CachedREST<S> {
    async fn request_session(
        &self,
        user_id: domain::UserID,
    ) -> Result<domain::User, domain::ReadError> {
        let rest_result = self.rest.request_session(user_id).await;
        if let Ok(ref user) = rest_result {
            if let Err(err) = IndexedDB.write_session(user).await {
                error!("failed to write session into IDB: {err}");
            }
        }

        rest_result
    }

    async fn initialize_session(&self) -> Result<domain::User, domain::ReadError> {
        IndexedDB.initialize_session().await
    }

    async fn delete_session(&self) -> Result<(), domain::DeleteError> {
        execute!(self, delete_session,)?;
        Ok(IndexedDB.clear_session_dependent_data().await?)
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
    ) -> Result<domain::User, domain::CreateError> {
        self.rest.create_user(name, sex).await
    }

    async fn replace_user(&self, user: domain::User) -> Result<domain::User, domain::UpdateError> {
        self.rest.replace_user(user).await
    }

    async fn delete_user(&self, id: domain::UserID) -> Result<domain::UserID, domain::DeleteError> {
        self.rest.delete_user(id).await
    }
}

impl<S: SendRequest> domain::BodyWeightRepository for CachedREST<S> {
    async fn sync_body_weight(&self) -> Result<Vec<domain::BodyWeight>, domain::SyncError> {
        sync!(self, read_body_weight, write_body_weight, "body weight")
    }

    async fn read_body_weight(&self) -> Result<Vec<domain::BodyWeight>, domain::ReadError> {
        IndexedDB.read_body_weight().await
    }

    async fn create_body_weight(
        &self,
        body_weight: domain::BodyWeight,
    ) -> Result<domain::BodyWeight, domain::CreateError> {
        execute!(self, create_body_weight, body_weight)
    }

    async fn replace_body_weight(
        &self,
        body_weight: domain::BodyWeight,
    ) -> Result<domain::BodyWeight, domain::UpdateError> {
        execute!(self, replace_body_weight, body_weight)
    }

    async fn delete_body_weight(&self, date: NaiveDate) -> Result<NaiveDate, domain::DeleteError> {
        execute!(self, delete_body_weight, date)
    }
}

impl<S: SendRequest> domain::BodyFatRepository for CachedREST<S> {
    async fn sync_body_fat(&self) -> Result<Vec<domain::BodyFat>, domain::SyncError> {
        sync!(self, read_body_fat, write_body_fat, "body fat")
    }

    async fn read_body_fat(&self) -> Result<Vec<domain::BodyFat>, domain::ReadError> {
        IndexedDB.read_body_fat().await
    }

    async fn create_body_fat(
        &self,
        body_fat: domain::BodyFat,
    ) -> Result<domain::BodyFat, domain::CreateError> {
        execute!(self, create_body_fat, body_fat)
    }

    async fn replace_body_fat(
        &self,
        body_fat: domain::BodyFat,
    ) -> Result<domain::BodyFat, domain::UpdateError> {
        execute!(self, replace_body_fat, body_fat)
    }

    async fn delete_body_fat(&self, date: NaiveDate) -> Result<NaiveDate, domain::DeleteError> {
        execute!(self, delete_body_fat, date)
    }
}

impl<S: SendRequest> domain::PeriodRepository for CachedREST<S> {
    async fn sync_period(&self) -> Result<Vec<domain::Period>, domain::SyncError> {
        sync!(self, read_period, write_period, "period")
    }

    async fn read_period(&self) -> Result<Vec<domain::Period>, domain::ReadError> {
        IndexedDB.read_period().await
    }

    async fn create_period(
        &self,
        period: domain::Period,
    ) -> Result<domain::Period, domain::CreateError> {
        execute!(self, create_period, period)
    }

    async fn replace_period(
        &self,
        period: domain::Period,
    ) -> Result<domain::Period, domain::UpdateError> {
        execute!(self, replace_period, period)
    }

    async fn delete_period(&self, date: NaiveDate) -> Result<NaiveDate, domain::DeleteError> {
        execute!(self, delete_period, date)
    }
}

impl<S: SendRequest> domain::ExerciseRepository for CachedREST<S> {
    async fn sync_exercises(&self) -> Result<Vec<domain::Exercise>, domain::SyncError> {
        sync!(self, read_exercises, write_exercises, "exercises")
    }

    async fn read_exercises(&self) -> Result<Vec<domain::Exercise>, domain::ReadError> {
        IndexedDB.read_exercises().await
    }

    async fn create_exercise(
        &self,
        name: domain::Name,
        muscles: Vec<domain::ExerciseMuscle>,
    ) -> Result<domain::Exercise, domain::CreateError> {
        create!(self, create_exercise, replace_exercise, name, muscles)
    }

    async fn replace_exercise(
        &self,
        exercise: domain::Exercise,
    ) -> Result<domain::Exercise, domain::UpdateError> {
        execute!(self, replace_exercise, exercise)
    }

    async fn delete_exercise(
        &self,
        id: domain::ExerciseID,
    ) -> Result<domain::ExerciseID, domain::DeleteError> {
        execute!(self, delete_exercise, id)
    }
}

impl<S: SendRequest> domain::RoutineRepository for CachedREST<S> {
    async fn sync_routines(&self) -> Result<Vec<domain::Routine>, domain::SyncError> {
        sync!(self, read_routines, write_routines, "routines")
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
        Ok(IndexedDB
            .put(
                Store::Routines,
                super::indexed_db::Routine::from(&routine),
                routine,
            )
            .await?)
    }

    async fn modify_routine(
        &self,
        id: domain::RoutineID,
        name: Option<domain::Name>,
        archived: Option<bool>,
        sections: Option<Vec<domain::RoutinePart>>,
    ) -> Result<domain::Routine, domain::UpdateError> {
        execute!(self, modify_routine, id, name, archived, sections)
    }

    async fn delete_routine(
        &self,
        id: domain::RoutineID,
    ) -> Result<domain::RoutineID, domain::DeleteError> {
        execute!(self, delete_routine, id)
    }
}

impl<S: SendRequest> domain::TrainingSessionRepository for CachedREST<S> {
    async fn sync_training_sessions(
        &self,
    ) -> Result<Vec<domain::TrainingSession>, domain::SyncError> {
        sync!(
            self,
            read_training_sessions,
            write_training_sessions,
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
        Ok(IndexedDB
            .put(
                Store::TrainingSessions,
                super::indexed_db::TrainingSession::from(&training_session),
                training_session,
            )
            .await?)
    }

    async fn modify_training_session(
        &self,
        id: domain::TrainingSessionID,
        notes: Option<String>,
        elements: Option<Vec<domain::TrainingSessionElement>>,
    ) -> Result<domain::TrainingSession, domain::UpdateError> {
        execute!(self, modify_training_session, id, notes, elements)
    }

    async fn delete_training_session(
        &self,
        id: domain::TrainingSessionID,
    ) -> Result<domain::TrainingSessionID, domain::DeleteError> {
        execute!(self, delete_training_session, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    mod wasm {
        use std::cell::RefCell;

        use pretty_assertions::assert_eq;
        use serde_json::json;
        use valens_domain::{
            BodyFatRepository, BodyWeightRepository, ExerciseRepository, PeriodRepository,
            RoutineRepository, SessionRepository, TrainingSessionRepository, UserRepository,
            VersionRepository,
        };
        use wasm_bindgen_test::wasm_bindgen_test;

        use crate::rest;
        use crate::tests::data::{
            BODY_FAT, BODY_FATS, BODY_WEIGHT, BODY_WEIGHTS, EXERCISE, EXERCISES, PERIOD, PERIODS,
            ROUTINE, ROUTINES, TRAINING_SESSION, TRAINING_SESSIONS, USER, USER_2, USERS,
        };

        use super::*;

        #[wasm_bindgen_test]
        async fn test_request_session() {
            reset_cache().await;

            assert!(matches!(
                cached_rest_with_response(None)
                    .request_session(USER.id)
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
                .request_session(USER.id)
                .await
                .unwrap(),
                USER.clone()
            );

            assert_eq!(IndexedDB.initialize_session().await.unwrap(), USER.clone());

            assert!(matches!(
                cached_rest_with_response(None)
                    .request_session(USER_2.id)
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
                ()
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
                ()
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
                    .create_user(USER.name.clone(), USER.sex)
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
                .create_user(USER.name.clone(), USER.sex)
                .await
                .unwrap(),
                USER.clone()
            );
        }

        #[wasm_bindgen_test]
        async fn test_replace_user() {
            reset_cache().await;

            let mut user = USER.clone();
            user.name = domain::Name::new("C").unwrap();

            assert!(matches!(
                cached_rest_with_response(None)
                    .replace_user(user.clone())
                    .await,
                Err(domain::UpdateError::Storage(
                    domain::StorageError::NoConnection
                ))
            ));

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
                USER.id
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

            assert!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::BodyWeight::from(body_weight.clone())),
                ))
                .create_body_weight(body_weight.clone())
                .await
                .unwrap_err()
                .to_string()
                .starts_with("ConstraintError: ")
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
                BODY_WEIGHT.date
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
                BODY_WEIGHT.date
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

            assert!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::BodyFat::from(body_fat.clone())),
                ))
                .create_body_fat(body_fat.clone())
                .await
                .unwrap_err()
                .to_string()
                .starts_with("ConstraintError: ")
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
                BODY_FAT.date
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
                BODY_FAT.date
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

            assert!(
                cached_rest_with_response(Some(
                    gloo_net::http::Response::builder()
                        .status(200)
                        .json(&rest::Period::from(period.clone())),
                ))
                .create_period(period.clone())
                .await
                .unwrap_err()
                .to_string()
                .starts_with("ConstraintError: ")
            );

            assert_eq!(
                cached_rest_with_response(None).read_period().await.unwrap(),
                vec![PERIOD]
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
                PERIOD.date
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
                PERIOD.date
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
                    .create_exercise(EXERCISE.name.clone(), EXERCISE.muscles.clone())
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
                .create_exercise(EXERCISE.name.clone(), EXERCISE.muscles.clone())
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
                .write_exercises(&[EXERCISE.clone()])
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
                .write_exercises(&[EXERCISE.clone()])
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
                EXERCISE.id
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
                EXERCISE.id
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

            IndexedDB.write_routines(&[ROUTINE.clone()]).await.unwrap();

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

            IndexedDB.write_routines(&[ROUTINE.clone()]).await.unwrap();

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
                ROUTINE.id
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
                ROUTINE.id
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
                .write_training_sessions(&[TRAINING_SESSION.clone()])
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
                        Some(training_session.elements.clone())
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
                    Some(training_session.elements.clone())
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
                .write_training_sessions(&[TRAINING_SESSION.clone()])
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
                TRAINING_SESSION.id
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
                TRAINING_SESSION.id
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
            let sender = MockSendRequest {
                request: RefCell::new(None),
                response: RefCell::new(response),
            };
            CachedREST {
                rest: REST { sender },
            }
        }

        struct MockSendRequest {
            request: RefCell<Option<gloo_net::http::Request>>,
            response: RefCell<Option<Result<gloo_net::http::Response, gloo_net::Error>>>,
        }

        impl SendRequest for MockSendRequest {
            async fn send_request(
                &self,
                request: gloo_net::http::Request,
            ) -> Result<gloo_net::http::Response, gloo_net::Error> {
                *self.request.borrow_mut() = Some(request);
                (*self.response.borrow_mut())
                    .take()
                    .unwrap_or(Err(gloo_net::Error::GlooError("no response".to_string())))
            }
        }
    }
}
