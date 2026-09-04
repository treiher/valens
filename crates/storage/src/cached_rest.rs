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
use valens_domain::{self as domain};

use super::{
    indexed_db::IndexedDB,
    rest::{Conditional, GlooNetSendRequest, REST},
};

pub type DefaultCachedREST = CachedREST<REST<GlooNetSendRequest>, IndexedDB>;

/// The authoritative data source behind [`CachedREST`].
#[allow(async_fn_in_trait)]
pub trait Remote:
    domain::SessionRepository
    + domain::AuthRepository
    + domain::VersionRepository
    + domain::UserRepository
    + domain::BodyWeightRepository
    + domain::BodyFatRepository
    + domain::PeriodRepository
    + domain::ExerciseRepository
    + domain::RoutineRepository
    + domain::ScheduleRepository
    + domain::TrainingSessionRepository
{
    async fn read_body_weight_conditional(
        &self,
        etag: Option<&str>,
    ) -> Result<Conditional<Vec<domain::BodyWeight>>, domain::ReadError>;
    async fn read_body_fat_conditional(
        &self,
        etag: Option<&str>,
    ) -> Result<Conditional<Vec<domain::BodyFat>>, domain::ReadError>;
    async fn read_period_conditional(
        &self,
        etag: Option<&str>,
    ) -> Result<Conditional<Vec<domain::Period>>, domain::ReadError>;
    async fn read_exercises_conditional(
        &self,
        etag: Option<&str>,
    ) -> Result<Conditional<Vec<domain::Exercise>>, domain::ReadError>;
    async fn read_routines_conditional(
        &self,
        etag: Option<&str>,
    ) -> Result<Conditional<Vec<domain::Routine>>, domain::ReadError>;
    async fn read_schedule_conditional(
        &self,
        etag: Option<&str>,
    ) -> Result<Conditional<domain::Schedule>, domain::ReadError>;
    async fn read_training_sessions_conditional(
        &self,
        etag: Option<&str>,
    ) -> Result<Conditional<Vec<domain::TrainingSession>>, domain::ReadError>;
}

/// The local cache behind [`CachedREST`].
#[allow(async_fn_in_trait)]
pub trait Cache:
    domain::SessionRepository
    + domain::BodyWeightRepository
    + domain::BodyFatRepository
    + domain::PeriodRepository
    + domain::ExerciseRepository
    + domain::RoutineRepository
    + domain::ScheduleRepository
    + domain::TrainingSessionRepository
    + Send
    + Sync
    + 'static
{
    async fn read_etag(&self, collection: &str) -> Result<Option<String>, String>;
    async fn write_etag(&self, collection: &str, etag: &str) -> Result<(), String>;
    async fn write_session(&self, user: &domain::User) -> Result<(), String>;
    async fn clear_session_dependent_data(&self) -> Result<(), Box<dyn std::error::Error>>;
    async fn write_body_weight(&self, body_weight: &[domain::BodyWeight]) -> Result<(), String>;
    async fn write_body_fat(&self, body_fat: &[domain::BodyFat]) -> Result<(), String>;
    async fn write_period(&self, period: &[domain::Period]) -> Result<(), String>;
    async fn write_exercises(&self, exercises: &[domain::Exercise]) -> Result<(), String>;
    async fn write_routines(&self, routines: &[domain::Routine]) -> Result<(), String>;
    async fn write_schedule(&self, schedule: &domain::Schedule) -> Result<(), String>;
    async fn write_training_sessions(
        &self,
        training_sessions: &[domain::TrainingSession],
    ) -> Result<(), String>;
    async fn write_routine(&self, routine: &domain::Routine) -> Result<(), String>;
    async fn write_training_session(
        &self,
        training_session: &domain::TrainingSession,
    ) -> Result<(), String>;
}

macro_rules! sync {
    ($self:ident, $read:ident, $write:ident, $read_back:ident, $name:literal) => {{
        let etag = $self.cache.read_etag($name).await.ok().flatten();
        match $self.remote.$read(etag.as_deref()).await {
            Ok(Conditional::Modified { data, etag }) => {
                // Persist the ETag only after the data it describes is cached, so a later 304
                // never serves stale data behind a current ETag.
                if let Err(err) = $self.cache.$write(&data).await {
                    error!("failed to write {} into IDB: {err}", $name);
                } else if let Some(etag) = etag
                    && let Err(err) = $self.cache.write_etag($name, &etag).await
                {
                    error!("failed to write {} etag into IDB: {err}", $name);
                }
                Ok(data)
            }
            // Reuse the cached data the server confirmed is still current.
            Ok(Conditional::NotModified) => Ok($self.cache.$read_back().await?),
            Err(err) => Err(err.into()),
        }
    }};
}

macro_rules! create {
    ($self: ident, $create: ident, $replace: ident, $name: literal, $($arg:expr),*) => {{
        let result = $self.remote.$create($($arg),*).await?;
        if let Err(err) = $self.cache.$replace(result.clone()).await {
            error!("failed to update {} in IDB: {err}", $name);
        }
        Ok(result)
    }};
}

macro_rules! execute {
    ($self: ident, $method: ident, $name: literal $(, $arg:expr)*) => {{
        let result = $self.remote.$method($($arg.clone()),*).await?;
        if let Err(err) = $self.cache.$method($($arg),*).await {
            error!("failed to update {} in IDB: {err}", $name);
        }
        Ok(result)
    }};
}

#[derive(Clone, Copy)]
pub struct CachedREST<R: Remote, C: Cache> {
    pub remote: R,
    pub cache: C,
}

impl DefaultCachedREST {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            remote: REST::new(),
            cache: IndexedDB,
        }
    }
}

impl Default for DefaultCachedREST {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: Remote, C: Cache> domain::SessionRepository for CachedREST<R, C> {
    async fn request_session(&self, name: domain::Name) -> Result<domain::User, domain::ReadError> {
        let rest_result = self.remote.request_session(name).await;
        if let Ok(ref user) = rest_result
            && let Err(err) = self.cache.write_session(user).await
        {
            error!("failed to write session into IDB: {err}");
        }

        rest_result
    }

    async fn initialize_session(&self) -> Result<domain::User, domain::ReadError> {
        self.cache.initialize_session().await
    }

    async fn sync_session(&self) -> Result<Option<domain::User>, domain::SyncError> {
        if let Some(user) = self.remote.sync_session().await? {
            if let Err(err) = self.cache.write_session(&user).await {
                error!("failed to write session into IDB: {err}");
            }
            Ok(Some(user))
        } else {
            // A missing session on the server means the user is signed out
            if let Err(err) = self.cache.delete_session().await {
                error!("failed to update session in IDB: {err}");
            }
            if let Err(err) = self.cache.clear_session_dependent_data().await {
                error!("failed to update session-dependent data in IDB: {err}");
            }
            Ok(None)
        }
    }

    async fn delete_session(&self) -> Result<domain::SignOut, domain::DeleteError> {
        self.remote.delete_session().await?;
        let mut sign_out = domain::SignOut::Complete;
        if let Err(err) = self.cache.delete_session().await {
            error!("failed to update session in IDB: {err}");
            sign_out = domain::SignOut::DataRetained;
        }
        if let Err(err) = self.cache.clear_session_dependent_data().await {
            error!("failed to update session-dependent data in IDB: {err}");
            sign_out = domain::SignOut::DataRetained;
        }
        Ok(sign_out)
    }
}

impl<R: Remote, C: Cache> domain::AuthRepository for CachedREST<R, C> {
    async fn read_auth_methods(&self) -> Result<Vec<domain::AuthMethod>, domain::ReadError> {
        self.remote.read_auth_methods().await
    }

    async fn login_with_passkey(&self) -> Result<domain::User, domain::ReadError> {
        let rest_result = self.remote.login_with_passkey().await;
        if let Ok(ref user) = rest_result
            && let Err(err) = self.cache.write_session(user).await
        {
            error!("failed to write session into IDB: {err}");
        }

        rest_result
    }

    async fn register_passkey(&self) -> Result<domain::Passkey, domain::CreateError> {
        self.remote.register_passkey().await
    }

    async fn read_passkeys(
        &self,
        user_id: domain::UserID,
    ) -> Result<Vec<domain::Passkey>, domain::ReadError> {
        self.remote.read_passkeys(user_id).await
    }

    async fn rename_passkey(
        &self,
        user_id: domain::UserID,
        id: domain::PasskeyID,
        label: domain::Name,
    ) -> Result<domain::Passkey, domain::UpdateError> {
        self.remote.rename_passkey(user_id, id, label).await
    }

    async fn delete_passkey(
        &self,
        user_id: domain::UserID,
        id: domain::PasskeyID,
    ) -> Result<(), domain::DeleteError> {
        self.remote.delete_passkey(user_id, id).await
    }

    async fn create_login_link(
        &self,
        user_id: domain::UserID,
    ) -> Result<String, domain::CreateError> {
        self.remote.create_login_link(user_id).await
    }

    async fn redeem_login_link(&self, token: String) -> Result<domain::User, domain::ReadError> {
        let rest_result = self.remote.redeem_login_link(token).await;
        if let Ok(ref user) = rest_result
            && let Err(err) = self.cache.write_session(user).await
        {
            error!("failed to write session into IDB: {err}");
        }

        rest_result
    }
}

impl<R: Remote, C: Cache> domain::VersionRepository for CachedREST<R, C> {
    async fn read_version(&self) -> Result<String, domain::ReadError> {
        self.remote.read_version().await
    }
}

impl<R: Remote, C: Cache> domain::UserRepository for CachedREST<R, C> {
    async fn read_users(&self) -> Result<Vec<domain::User>, domain::ReadError> {
        self.remote.read_users().await
    }

    async fn create_user(
        &self,
        name: domain::Name,
        sex: domain::Sex,
        height: Option<u8>,
        role: domain::Role,
    ) -> Result<domain::User, domain::CreateError> {
        self.remote.create_user(name, sex, height, role).await
    }

    async fn replace_user(&self, user: domain::User) -> Result<domain::User, domain::UpdateError> {
        let user = self.remote.replace_user(user).await?;
        if let Ok(session_user) = self.cache.initialize_session().await
            && session_user.id == user.id
            && let Err(err) = self.cache.write_session(&user).await
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
        let user = self.remote.update_user(id, name, sex, height).await?;
        if let Ok(session_user) = self.cache.initialize_session().await
            && session_user.id == user.id
            && let Err(err) = self.cache.write_session(&user).await
        {
            error!("failed to write session into IDB: {err}");
        }
        Ok(user)
    }

    async fn delete_user(&self, id: domain::UserID) -> Result<(), domain::DeleteError> {
        self.remote.delete_user(id).await
    }
}

impl<R: Remote, C: Cache> domain::BodyWeightRepository for CachedREST<R, C> {
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
        self.cache.read_body_weight().await
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

impl<R: Remote, C: Cache> domain::BodyFatRepository for CachedREST<R, C> {
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
        self.cache.read_body_fat().await
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

impl<R: Remote, C: Cache> domain::PeriodRepository for CachedREST<R, C> {
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
        self.cache.read_period().await
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

impl<R: Remote, C: Cache> domain::ExerciseRepository for CachedREST<R, C> {
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
        self.cache.read_exercises().await
    }

    async fn create_exercise(
        &self,
        name: domain::Name,
        notes: String,
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
            notes,
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

impl<R: Remote, C: Cache> domain::RoutineRepository for CachedREST<R, C> {
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
        self.cache.read_routines().await
    }

    async fn create_routine(
        &self,
        name: domain::Name,
        notes: String,
        sections: Vec<domain::RoutinePart>,
    ) -> Result<domain::Routine, domain::CreateError> {
        let routine = self.remote.create_routine(name, notes, sections).await?;
        if let Err(err) = self.cache.write_routine(&routine).await {
            error!("failed to update routine in IDB: {err}");
        }
        Ok(routine)
    }

    async fn modify_routine(
        &self,
        id: domain::RoutineID,
        name: Option<domain::Name>,
        notes: Option<String>,
        archived: Option<bool>,
        sections: Option<Vec<domain::RoutinePart>>,
    ) -> Result<domain::Routine, domain::UpdateError> {
        execute!(
            self,
            modify_routine,
            "routine",
            id,
            name,
            notes,
            archived,
            sections
        )
    }

    async fn delete_routine(&self, id: domain::RoutineID) -> Result<(), domain::DeleteError> {
        execute!(self, delete_routine, "routine", id)
    }
}

impl<R: Remote, C: Cache> domain::ScheduleRepository for CachedREST<R, C> {
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
        self.cache.read_schedule().await
    }

    async fn replace_schedule(
        &self,
        schedule: domain::Schedule,
    ) -> Result<domain::Schedule, domain::UpdateError> {
        execute!(self, replace_schedule, "schedule", schedule)
    }
}

impl<R: Remote, C: Cache> domain::TrainingSessionRepository for CachedREST<R, C> {
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
        self.cache.read_training_sessions().await
    }

    async fn create_training_session(
        &self,
        routine_id: domain::RoutineID,
        date: NaiveDate,
        notes: String,
        elements: Vec<domain::TrainingSessionElement>,
    ) -> Result<domain::TrainingSession, domain::CreateError> {
        let training_session = self
            .remote
            .create_training_session(routine_id, date, notes, elements)
            .await?;
        if let Err(err) = self.cache.write_training_session(&training_session).await {
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
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::{Arc, Mutex},
    };

    use pretty_assertions::assert_eq;
    use valens_domain::{
        AuthRepository, BodyFatRepository, BodyWeightRepository, ExerciseRepository,
        PeriodRepository, RoutineRepository, ScheduleRepository, SessionRepository,
        TrainingSessionRepository, UserRepository, VersionRepository,
    };

    use crate::tests::data::{
        BODY_FATS, BODY_WEIGHTS, EXERCISES, PERIODS, ROUTINE, ROUTINES, SCHEDULE, TRAINING_SESSION,
        TRAINING_SESSIONS, USER, USER_2,
    };

    use super::*;

    /// Forwards the repository traits only the remote serves to the wrapped
    /// [`domain::tests::FakeRepository`].
    macro_rules! forward_remote_repositories {
        ($type:ty) => {
            impl domain::AuthRepository for $type {
                async fn read_auth_methods(
                    &self,
                ) -> Result<Vec<domain::AuthMethod>, domain::ReadError> {
                    self.repository.read_auth_methods().await
                }

                async fn login_with_passkey(&self) -> Result<domain::User, domain::ReadError> {
                    self.repository.login_with_passkey().await
                }

                async fn register_passkey(&self) -> Result<domain::Passkey, domain::CreateError> {
                    self.repository.register_passkey().await
                }

                async fn read_passkeys(
                    &self,
                    user_id: domain::UserID,
                ) -> Result<Vec<domain::Passkey>, domain::ReadError> {
                    self.repository.read_passkeys(user_id).await
                }

                async fn rename_passkey(
                    &self,
                    user_id: domain::UserID,
                    id: domain::PasskeyID,
                    label: domain::Name,
                ) -> Result<domain::Passkey, domain::UpdateError> {
                    self.repository.rename_passkey(user_id, id, label).await
                }

                async fn delete_passkey(
                    &self,
                    user_id: domain::UserID,
                    id: domain::PasskeyID,
                ) -> Result<(), domain::DeleteError> {
                    self.repository.delete_passkey(user_id, id).await
                }

                async fn create_login_link(
                    &self,
                    user_id: domain::UserID,
                ) -> Result<String, domain::CreateError> {
                    self.repository.create_login_link(user_id).await
                }

                async fn redeem_login_link(
                    &self,
                    token: String,
                ) -> Result<domain::User, domain::ReadError> {
                    self.repository.redeem_login_link(token).await
                }
            }

            impl domain::VersionRepository for $type {
                async fn read_version(&self) -> Result<String, domain::ReadError> {
                    self.repository.read_version().await
                }
            }

            impl domain::UserRepository for $type {
                async fn read_users(&self) -> Result<Vec<domain::User>, domain::ReadError> {
                    self.repository.read_users().await
                }

                async fn create_user(
                    &self,
                    name: domain::Name,
                    sex: domain::Sex,
                    height: Option<u8>,
                    role: domain::Role,
                ) -> Result<domain::User, domain::CreateError> {
                    self.repository.create_user(name, sex, height, role).await
                }

                async fn replace_user(
                    &self,
                    user: domain::User,
                ) -> Result<domain::User, domain::UpdateError> {
                    self.repository.replace_user(user).await
                }

                async fn update_user(
                    &self,
                    id: domain::UserID,
                    name: domain::Name,
                    sex: domain::Sex,
                    height: Option<u8>,
                ) -> Result<domain::User, domain::UpdateError> {
                    self.repository.update_user(id, name, sex, height).await
                }

                async fn delete_user(&self, id: domain::UserID) -> Result<(), domain::DeleteError> {
                    self.repository.delete_user(id).await
                }
            }
        };
    }

    /// Forwards the repository traits of the collections to the wrapped
    /// [`domain::tests::FakeRepository`].
    macro_rules! forward_collection_repositories {
        ($type:ty) => {
            impl domain::BodyWeightRepository for $type {
                async fn sync_body_weight(
                    &self,
                ) -> Result<Vec<domain::BodyWeight>, domain::SyncError> {
                    self.repository.sync_body_weight().await
                }

                async fn read_body_weight(
                    &self,
                ) -> Result<Vec<domain::BodyWeight>, domain::ReadError> {
                    self.repository.read_body_weight().await
                }

                async fn create_body_weight(
                    &self,
                    body_weight: domain::BodyWeight,
                ) -> Result<domain::BodyWeight, domain::CreateError> {
                    self.repository.create_body_weight(body_weight).await
                }

                async fn replace_body_weight(
                    &self,
                    body_weight: domain::BodyWeight,
                ) -> Result<domain::BodyWeight, domain::UpdateError> {
                    self.repository.replace_body_weight(body_weight).await
                }

                async fn delete_body_weight(
                    &self,
                    date: NaiveDate,
                ) -> Result<(), domain::DeleteError> {
                    self.repository.delete_body_weight(date).await
                }
            }

            impl domain::BodyFatRepository for $type {
                async fn sync_body_fat(&self) -> Result<Vec<domain::BodyFat>, domain::SyncError> {
                    self.repository.sync_body_fat().await
                }

                async fn read_body_fat(&self) -> Result<Vec<domain::BodyFat>, domain::ReadError> {
                    self.repository.read_body_fat().await
                }

                async fn create_body_fat(
                    &self,
                    body_fat: domain::BodyFat,
                ) -> Result<domain::BodyFat, domain::CreateError> {
                    self.repository.create_body_fat(body_fat).await
                }

                async fn replace_body_fat(
                    &self,
                    body_fat: domain::BodyFat,
                ) -> Result<domain::BodyFat, domain::UpdateError> {
                    self.repository.replace_body_fat(body_fat).await
                }

                async fn delete_body_fat(
                    &self,
                    date: NaiveDate,
                ) -> Result<(), domain::DeleteError> {
                    self.repository.delete_body_fat(date).await
                }
            }

            impl domain::PeriodRepository for $type {
                async fn sync_period(&self) -> Result<Vec<domain::Period>, domain::SyncError> {
                    self.repository.sync_period().await
                }

                async fn read_period(&self) -> Result<Vec<domain::Period>, domain::ReadError> {
                    self.repository.read_period().await
                }

                async fn create_period(
                    &self,
                    period: domain::Period,
                ) -> Result<domain::Period, domain::CreateError> {
                    self.repository.create_period(period).await
                }

                async fn replace_period(
                    &self,
                    period: domain::Period,
                ) -> Result<domain::Period, domain::UpdateError> {
                    self.repository.replace_period(period).await
                }

                async fn delete_period(&self, date: NaiveDate) -> Result<(), domain::DeleteError> {
                    self.repository.delete_period(date).await
                }
            }

            impl domain::ExerciseRepository for $type {
                async fn sync_exercises(&self) -> Result<Vec<domain::Exercise>, domain::SyncError> {
                    self.repository.sync_exercises().await
                }

                async fn read_exercises(&self) -> Result<Vec<domain::Exercise>, domain::ReadError> {
                    self.repository.read_exercises().await
                }

                #[allow(clippy::too_many_arguments)]
                async fn create_exercise(
                    &self,
                    name: domain::Name,
                    notes: String,
                    muscles: Vec<domain::ExerciseMuscle>,
                    force: Option<domain::Force>,
                    mechanic: Option<domain::Mechanic>,
                    laterality: Option<domain::Laterality>,
                    assistance: Option<domain::Assistance>,
                    equipment: Vec<domain::Equipment>,
                    category: Option<domain::Category>,
                ) -> Result<domain::Exercise, domain::CreateError> {
                    self.repository
                        .create_exercise(
                            name, notes, muscles, force, mechanic, laterality, assistance,
                            equipment, category,
                        )
                        .await
                }

                async fn replace_exercise(
                    &self,
                    exercise: domain::Exercise,
                ) -> Result<domain::Exercise, domain::UpdateError> {
                    self.repository.replace_exercise(exercise).await
                }

                async fn delete_exercise(
                    &self,
                    id: domain::ExerciseID,
                ) -> Result<(), domain::DeleteError> {
                    self.repository.delete_exercise(id).await
                }
            }

            impl domain::RoutineRepository for $type {
                async fn sync_routines(&self) -> Result<Vec<domain::Routine>, domain::SyncError> {
                    self.repository.sync_routines().await
                }

                async fn read_routines(&self) -> Result<Vec<domain::Routine>, domain::ReadError> {
                    self.repository.read_routines().await
                }

                async fn create_routine(
                    &self,
                    name: domain::Name,
                    notes: String,
                    sections: Vec<domain::RoutinePart>,
                ) -> Result<domain::Routine, domain::CreateError> {
                    self.repository.create_routine(name, notes, sections).await
                }

                async fn modify_routine(
                    &self,
                    id: domain::RoutineID,
                    name: Option<domain::Name>,
                    notes: Option<String>,
                    archived: Option<bool>,
                    sections: Option<Vec<domain::RoutinePart>>,
                ) -> Result<domain::Routine, domain::UpdateError> {
                    self.repository
                        .modify_routine(id, name, notes, archived, sections)
                        .await
                }

                async fn delete_routine(
                    &self,
                    id: domain::RoutineID,
                ) -> Result<(), domain::DeleteError> {
                    self.repository.delete_routine(id).await
                }
            }

            impl domain::ScheduleRepository for $type {
                async fn sync_schedule(&self) -> Result<domain::Schedule, domain::SyncError> {
                    self.repository.sync_schedule().await
                }

                async fn read_schedule(&self) -> Result<domain::Schedule, domain::ReadError> {
                    self.repository.read_schedule().await
                }

                async fn replace_schedule(
                    &self,
                    schedule: domain::Schedule,
                ) -> Result<domain::Schedule, domain::UpdateError> {
                    self.repository.replace_schedule(schedule).await
                }
            }

            impl domain::TrainingSessionRepository for $type {
                async fn sync_training_sessions(
                    &self,
                ) -> Result<Vec<domain::TrainingSession>, domain::SyncError> {
                    self.repository.sync_training_sessions().await
                }

                async fn read_training_sessions(
                    &self,
                ) -> Result<Vec<domain::TrainingSession>, domain::ReadError> {
                    self.repository.read_training_sessions().await
                }

                async fn create_training_session(
                    &self,
                    routine_id: domain::RoutineID,
                    date: NaiveDate,
                    notes: String,
                    elements: Vec<domain::TrainingSessionElement>,
                ) -> Result<domain::TrainingSession, domain::CreateError> {
                    self.repository
                        .create_training_session(routine_id, date, notes, elements)
                        .await
                }

                async fn modify_training_session(
                    &self,
                    id: domain::TrainingSessionID,
                    notes: Option<String>,
                    elements: Option<Vec<domain::TrainingSessionElement>>,
                    exercise_notes: Option<std::collections::BTreeMap<domain::ExerciseID, String>>,
                ) -> Result<domain::TrainingSession, domain::UpdateError> {
                    self.repository
                        .modify_training_session(id, notes, elements, exercise_notes)
                        .await
                }

                async fn delete_training_session(
                    &self,
                    id: domain::TrainingSessionID,
                ) -> Result<(), domain::DeleteError> {
                    self.repository.delete_training_session(id).await
                }
            }
        };
    }

    /// Operation of [`FakeCache`] that a test lets fail.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub(super) enum CacheCall {
        DeleteSession,
        WriteEtag,
        WriteSession,
        ClearSessionDependentData,
        WriteCollection,
        WriteRoutine,
        WriteTrainingSession,
    }

    /// The local cache of [`CachedREST`], holding its collections in memory.
    #[derive(Clone, Default)]
    pub(super) struct FakeCache {
        repository: domain::tests::FakeRepository,
        session: Arc<Mutex<Option<domain::User>>>,
        etags: Arc<Mutex<BTreeMap<String, String>>>,
        failing: Arc<Mutex<BTreeSet<CacheCall>>>,
        session_dependent_data_cleared: Arc<Mutex<bool>>,
    }

    impl FakeCache {
        fn with_session(self, user: &domain::User) -> Self {
            *self.session.lock().unwrap() = Some(user.clone());
            self
        }

        fn failing(self, call: CacheCall) -> Self {
            self.failing.lock().unwrap().insert(call);
            self
        }

        fn fails(&self, call: CacheCall) -> Option<Result<(), String>> {
            self.failing
                .lock()
                .unwrap()
                .contains(&call)
                .then(|| Err("write failed".to_string()))
        }

        fn etag(&self, collection: &str) -> Option<String> {
            self.etags.lock().unwrap().get(collection).cloned()
        }

        fn session_dependent_data_cleared(&self) -> bool {
            *self.session_dependent_data_cleared.lock().unwrap()
        }
    }

    forward_collection_repositories!(FakeCache);

    impl domain::SessionRepository for FakeCache {
        async fn request_session(
            &self,
            _: domain::Name,
        ) -> Result<domain::User, domain::ReadError> {
            panic!("unsupported")
        }

        async fn initialize_session(&self) -> Result<domain::User, domain::ReadError> {
            self.session
                .lock()
                .unwrap()
                .clone()
                .ok_or(domain::ReadError::Storage(domain::StorageError::NoSession))
        }

        async fn sync_session(&self) -> Result<Option<domain::User>, domain::SyncError> {
            panic!("unsupported")
        }

        async fn delete_session(&self) -> Result<domain::SignOut, domain::DeleteError> {
            if self.fails(CacheCall::DeleteSession).is_some() {
                return Err(domain::DeleteError::Other("delete failed".into()));
            }
            *self.session.lock().unwrap() = None;
            Ok(domain::SignOut::Complete)
        }
    }

    impl Cache for FakeCache {
        async fn read_etag(&self, collection: &str) -> Result<Option<String>, String> {
            Ok(self.etag(collection))
        }

        async fn write_etag(&self, collection: &str, etag: &str) -> Result<(), String> {
            self.fails(CacheCall::WriteEtag).unwrap_or_else(|| {
                self.etags
                    .lock()
                    .unwrap()
                    .insert(collection.to_string(), etag.to_string());
                Ok(())
            })
        }

        async fn write_session(&self, user: &domain::User) -> Result<(), String> {
            self.fails(CacheCall::WriteSession).unwrap_or_else(|| {
                *self.session.lock().unwrap() = Some(user.clone());
                Ok(())
            })
        }

        async fn clear_session_dependent_data(&self) -> Result<(), Box<dyn std::error::Error>> {
            if self.fails(CacheCall::ClearSessionDependentData).is_some() {
                return Err("clear failed".into());
            }
            *self.session_dependent_data_cleared.lock().unwrap() = true;
            Ok(())
        }

        async fn write_body_weight(
            &self,
            body_weight: &[domain::BodyWeight],
        ) -> Result<(), String> {
            self.fails(CacheCall::WriteCollection).unwrap_or_else(|| {
                let _ = self
                    .repository
                    .clone()
                    .with_body_weight(body_weight.to_vec());
                Ok(())
            })
        }

        async fn write_body_fat(&self, body_fat: &[domain::BodyFat]) -> Result<(), String> {
            self.fails(CacheCall::WriteCollection).unwrap_or_else(|| {
                let _ = self.repository.clone().with_body_fat(body_fat.to_vec());
                Ok(())
            })
        }

        async fn write_period(&self, period: &[domain::Period]) -> Result<(), String> {
            self.fails(CacheCall::WriteCollection).unwrap_or_else(|| {
                let _ = self.repository.clone().with_period(period.to_vec());
                Ok(())
            })
        }

        async fn write_exercises(&self, exercises: &[domain::Exercise]) -> Result<(), String> {
            self.fails(CacheCall::WriteCollection).unwrap_or_else(|| {
                let _ = self.repository.clone().with_exercises(exercises.to_vec());
                Ok(())
            })
        }

        async fn write_routines(&self, routines: &[domain::Routine]) -> Result<(), String> {
            self.fails(CacheCall::WriteCollection).unwrap_or_else(|| {
                let _ = self.repository.clone().with_routines(routines.to_vec());
                Ok(())
            })
        }

        async fn write_schedule(&self, schedule: &domain::Schedule) -> Result<(), String> {
            self.fails(CacheCall::WriteCollection).unwrap_or_else(|| {
                let _ = self.repository.clone().with_schedule(schedule.clone());
                Ok(())
            })
        }

        async fn write_training_sessions(
            &self,
            training_sessions: &[domain::TrainingSession],
        ) -> Result<(), String> {
            self.fails(CacheCall::WriteCollection).unwrap_or_else(|| {
                let _ = self
                    .repository
                    .clone()
                    .with_training_sessions(training_sessions.to_vec());
                Ok(())
            })
        }

        async fn write_routine(&self, routine: &domain::Routine) -> Result<(), String> {
            self.fails(CacheCall::WriteRoutine).unwrap_or_else(|| {
                let _ = self.repository.clone().with_routines(vec![routine.clone()]);
                Ok(())
            })
        }

        async fn write_training_session(
            &self,
            training_session: &domain::TrainingSession,
        ) -> Result<(), String> {
            self.fails(CacheCall::WriteTrainingSession)
                .unwrap_or_else(|| {
                    let _ = self
                        .repository
                        .clone()
                        .with_training_sessions(vec![training_session.clone()]);
                    Ok(())
                })
        }
    }

    /// What [`FakeRemote`] answers a conditional read with.
    #[derive(Clone)]
    pub(super) enum Response {
        /// The stored collection, together with the `ETag` describing it.
        Modified(Option<String>),
        NotModified,
    }

    /// The authoritative data source of [`CachedREST`], answering from what it was seeded with.
    #[derive(Clone)]
    pub(super) struct FakeRemote {
        repository: domain::tests::FakeRepository,
        session_present: Arc<Mutex<bool>>,
        responses: Arc<Mutex<BTreeMap<String, Response>>>,
        received_etags: Arc<Mutex<BTreeMap<String, Option<String>>>>,
    }

    impl Default for FakeRemote {
        fn default() -> Self {
            Self {
                repository: domain::tests::FakeRepository::default(),
                session_present: Arc::new(Mutex::new(true)),
                responses: Arc::new(Mutex::new(BTreeMap::new())),
                received_etags: Arc::new(Mutex::new(BTreeMap::new())),
            }
        }
    }

    impl domain::SessionRepository for FakeRemote {
        async fn request_session(
            &self,
            name: domain::Name,
        ) -> Result<domain::User, domain::ReadError> {
            self.repository.request_session(name).await
        }

        async fn initialize_session(&self) -> Result<domain::User, domain::ReadError> {
            self.repository.initialize_session().await
        }

        async fn sync_session(&self) -> Result<Option<domain::User>, domain::SyncError> {
            if !*self.session_present.lock().unwrap() {
                return Ok(None);
            }
            self.repository.sync_session().await
        }

        async fn delete_session(&self) -> Result<domain::SignOut, domain::DeleteError> {
            self.repository.delete_session().await
        }
    }

    impl FakeRemote {
        fn signed_out(self) -> Self {
            *self.session_present.lock().unwrap() = false;
            self
        }

        fn answering(self, collection: &str, response: Response) -> Self {
            self.responses
                .lock()
                .unwrap()
                .insert(collection.to_string(), response);
            self
        }

        fn received_etag(&self, collection: &str) -> Option<String> {
            self.received_etags
                .lock()
                .unwrap()
                .get(collection)
                .cloned()
                .flatten()
        }

        fn conditional<T>(
            &self,
            collection: &str,
            etag: Option<&str>,
            data: Result<T, domain::ReadError>,
        ) -> Result<Conditional<T>, domain::ReadError> {
            self.received_etags
                .lock()
                .unwrap()
                .insert(collection.to_string(), etag.map(str::to_string));
            match self
                .responses
                .lock()
                .unwrap()
                .get(collection)
                .cloned()
                .unwrap_or(Response::Modified(None))
            {
                Response::Modified(etag) => Ok(Conditional::Modified { data: data?, etag }),
                Response::NotModified => Ok(Conditional::NotModified),
            }
        }
    }

    forward_remote_repositories!(FakeRemote);
    forward_collection_repositories!(FakeRemote);

    impl Remote for FakeRemote {
        async fn read_body_weight_conditional(
            &self,
            etag: Option<&str>,
        ) -> Result<Conditional<Vec<domain::BodyWeight>>, domain::ReadError> {
            let data = self.repository.read_body_weight().await;
            self.conditional("body weight", etag, data)
        }

        async fn read_body_fat_conditional(
            &self,
            etag: Option<&str>,
        ) -> Result<Conditional<Vec<domain::BodyFat>>, domain::ReadError> {
            let data = self.repository.read_body_fat().await;
            self.conditional("body fat", etag, data)
        }

        async fn read_period_conditional(
            &self,
            etag: Option<&str>,
        ) -> Result<Conditional<Vec<domain::Period>>, domain::ReadError> {
            let data = self.repository.read_period().await;
            self.conditional("period", etag, data)
        }

        async fn read_exercises_conditional(
            &self,
            etag: Option<&str>,
        ) -> Result<Conditional<Vec<domain::Exercise>>, domain::ReadError> {
            let data = self.repository.read_exercises().await;
            self.conditional("exercises", etag, data)
        }

        async fn read_routines_conditional(
            &self,
            etag: Option<&str>,
        ) -> Result<Conditional<Vec<domain::Routine>>, domain::ReadError> {
            let data = self.repository.read_routines().await;
            self.conditional("routines", etag, data)
        }

        async fn read_schedule_conditional(
            &self,
            etag: Option<&str>,
        ) -> Result<Conditional<domain::Schedule>, domain::ReadError> {
            let data = self.repository.read_schedule().await;
            self.conditional("schedule", etag, data)
        }

        async fn read_training_sessions_conditional(
            &self,
            etag: Option<&str>,
        ) -> Result<Conditional<Vec<domain::TrainingSession>>, domain::ReadError> {
            let data = self.repository.read_training_sessions().await;
            self.conditional("training sessions", etag, data)
        }
    }

    fn run<T>(future: impl std::future::Future<Output = T>) -> T {
        pollster::block_on(future)
    }

    fn cached_rest(remote: FakeRemote, cache: FakeCache) -> CachedREST<FakeRemote, FakeCache> {
        CachedREST { remote, cache }
    }

    /// Asserts the glue of every collection: a modified response is cached under its `ETag`, a
    /// `304` is answered from the cache, and a failed cache write leaves the `ETag` unset so
    /// that the next synchronization downloads in full.
    macro_rules! sync_tests {
        ($module:ident, $collection:literal, $sync:ident, $seed:ident, $read:ident, $data:expr) => {
            mod $module {
                use pretty_assertions::assert_eq;

                use super::*;

                #[test]
                fn caches_a_modified_response_under_its_etag() {
                    let remote = FakeRemote::default()
                        .answering($collection, Response::Modified(Some("etag-1".to_string())));
                    let remote = remote.repository.clone().$seed($data);
                    let remote = FakeRemote {
                        repository: remote,
                        ..FakeRemote::default()
                            .answering($collection, Response::Modified(Some("etag-1".to_string())))
                    };
                    let cache = FakeCache::default();
                    let cached_rest = cached_rest(remote, cache.clone());

                    assert_eq!(run(cached_rest.$sync()).unwrap(), $data);
                    assert_eq!(run(cached_rest.$read()).unwrap(), $data);
                    assert_eq!(cache.etag($collection), Some("etag-1".to_string()));
                }

                #[test]
                fn answers_a_not_modified_response_from_the_cache() {
                    let remote =
                        FakeRemote::default().answering($collection, Response::NotModified);
                    let cache = FakeCache::default();
                    let _ = cache.repository.clone().$seed($data);
                    let cached_rest = cached_rest(remote.clone(), cache.clone());
                    let _ = run(cache.write_etag($collection, "etag-1"));

                    assert_eq!(run(cached_rest.$sync()).unwrap(), $data);
                    assert_eq!(
                        remote.received_etag($collection),
                        Some("etag-1".to_string())
                    );
                }

                #[test]
                fn a_failed_cache_write_leaves_the_etag_unset() {
                    let remote = FakeRemote {
                        repository: domain::tests::FakeRepository::default().$seed($data),
                        ..FakeRemote::default()
                            .answering($collection, Response::Modified(Some("etag-1".to_string())))
                    };
                    let cache = FakeCache::default().failing(CacheCall::WriteCollection);
                    let cached_rest = cached_rest(remote, cache.clone());

                    assert_eq!(run(cached_rest.$sync()).unwrap(), $data);
                    assert_eq!(cache.etag($collection), None);
                }

                #[test]
                fn a_failed_etag_write_leaves_the_etag_unset() {
                    let remote = FakeRemote {
                        repository: domain::tests::FakeRepository::default().$seed($data),
                        ..FakeRemote::default()
                            .answering($collection, Response::Modified(Some("etag-1".to_string())))
                    };
                    let cache = FakeCache::default().failing(CacheCall::WriteEtag);
                    let cached_rest = cached_rest(remote, cache.clone());

                    assert_eq!(run(cached_rest.$sync()).unwrap(), $data);
                    assert_eq!(cache.etag($collection), None);
                }
            }
        };
    }

    sync_tests!(
        body_weight,
        "body weight",
        sync_body_weight,
        with_body_weight,
        read_body_weight,
        BODY_WEIGHTS.to_vec()
    );
    sync_tests!(
        body_fat,
        "body fat",
        sync_body_fat,
        with_body_fat,
        read_body_fat,
        BODY_FATS.to_vec()
    );
    sync_tests!(
        period,
        "period",
        sync_period,
        with_period,
        read_period,
        PERIODS.to_vec()
    );
    sync_tests!(
        exercises,
        "exercises",
        sync_exercises,
        with_exercises,
        read_exercises,
        EXERCISES.clone()
    );
    sync_tests!(
        routines,
        "routines",
        sync_routines,
        with_routines,
        read_routines,
        ROUTINES.clone()
    );
    sync_tests!(
        schedule,
        "schedule",
        sync_schedule,
        with_schedule,
        read_schedule,
        SCHEDULE.clone()
    );
    sync_tests!(
        training_sessions,
        "training sessions",
        sync_training_sessions,
        with_training_sessions,
        read_training_sessions,
        TRAINING_SESSIONS.clone()
    );

    #[test]
    fn test_a_response_without_an_etag_leaves_the_etag_unset() {
        let remote = FakeRemote {
            repository: domain::tests::FakeRepository::default()
                .with_body_weight(BODY_WEIGHTS.to_vec()),
            ..FakeRemote::default()
        };
        let cache = FakeCache::default();
        let cached_rest = cached_rest(remote, cache.clone());

        assert_eq!(
            run(cached_rest.sync_body_weight()).unwrap(),
            BODY_WEIGHTS.to_vec()
        );
        assert_eq!(cache.etag("body weight"), None);
    }

    #[test]
    fn test_an_unreachable_remote_leaves_the_cache_untouched() {
        let remote = FakeRemote {
            repository: domain::tests::FakeRepository::default()
                .failing(domain::tests::Call::ReadBodyWeight),
            ..FakeRemote::default()
        };
        let cache = FakeCache::default();
        let cached_rest = cached_rest(remote, cache.clone());

        assert!(run(cached_rest.sync_body_weight()).is_err());
        assert_eq!(cache.etag("body weight"), None);
        assert_eq!(run(cached_rest.read_body_weight()).unwrap(), vec![]);
    }

    #[test]
    fn test_a_requested_session_is_stored_in_the_cache() {
        let cache = FakeCache::default();
        let cached_rest = cached_rest(FakeRemote::default(), cache.clone());

        let user = run(cached_rest.request_session(USER.name.clone())).unwrap();

        assert_eq!(run(cached_rest.initialize_session()).unwrap(), user);
    }

    #[test]
    fn test_a_passkey_login_stores_the_session_in_the_cache() {
        let cache = FakeCache::default();
        let cached_rest = cached_rest(FakeRemote::default(), cache.clone());

        let user = run(cached_rest.login_with_passkey()).unwrap();

        assert_eq!(run(cached_rest.initialize_session()).unwrap(), user);
    }

    #[test]
    fn test_a_redeemed_login_link_stores_the_session_in_the_cache() {
        let cache = FakeCache::default();
        let cached_rest = cached_rest(FakeRemote::default(), cache.clone());

        let user = run(cached_rest.redeem_login_link("token".to_string())).unwrap();

        assert_eq!(run(cached_rest.initialize_session()).unwrap(), user);
    }

    #[test]
    fn test_a_session_missing_on_the_server_clears_the_cached_data() {
        let cache = FakeCache::default().with_session(&USER);
        let cached_rest = cached_rest(FakeRemote::default().signed_out(), cache.clone());

        assert_eq!(run(cached_rest.sync_session()).unwrap(), None);
        assert!(run(cached_rest.initialize_session()).is_err());
        assert!(cache.session_dependent_data_cleared());
    }

    #[test]
    fn test_a_session_present_on_the_server_is_stored_in_the_cache() {
        let cache = FakeCache::default();
        let cached_rest = cached_rest(FakeRemote::default(), cache.clone());

        let user = run(cached_rest.sync_session()).unwrap().unwrap();

        assert_eq!(run(cached_rest.initialize_session()).unwrap(), user);
        assert!(!cache.session_dependent_data_cleared());
    }

    #[test]
    fn test_signing_out_reports_retained_data_when_the_session_cannot_be_removed() {
        let cache = FakeCache::default().failing(CacheCall::DeleteSession);
        let cached_rest = cached_rest(FakeRemote::default(), cache);

        assert_eq!(
            run(cached_rest.delete_session()).unwrap(),
            domain::SignOut::DataRetained
        );
    }

    #[test]
    fn test_signing_out_reports_retained_data_when_the_data_cannot_be_removed() {
        let cache = FakeCache::default().failing(CacheCall::ClearSessionDependentData);
        let cached_rest = cached_rest(FakeRemote::default(), cache);

        assert_eq!(
            run(cached_rest.delete_session()).unwrap(),
            domain::SignOut::DataRetained
        );
    }

    #[test]
    fn test_signing_out_removes_the_session_dependent_data() {
        let cache = FakeCache::default().with_session(&USER);
        let cached_rest = cached_rest(FakeRemote::default(), cache.clone());

        assert_eq!(
            run(cached_rest.delete_session()).unwrap(),
            domain::SignOut::Complete
        );
        assert!(cache.session_dependent_data_cleared());
    }

    #[test]
    fn test_a_created_routine_is_written_into_the_cache() {
        let cache = FakeCache::default();
        let cached_rest = cached_rest(FakeRemote::default(), cache.clone());

        let routine = run(cached_rest.create_routine(
            ROUTINE.name.clone(),
            ROUTINE.notes.clone(),
            ROUTINE.sections.clone(),
        ))
        .unwrap();

        assert_eq!(run(cached_rest.read_routines()).unwrap(), vec![routine]);
    }

    #[test]
    fn test_a_created_training_session_is_written_into_the_cache() {
        let cache = FakeCache::default();
        let cached_rest = cached_rest(FakeRemote::default(), cache.clone());

        let training_session = run(cached_rest.create_training_session(
            TRAINING_SESSION.routine_id,
            TRAINING_SESSION.date,
            TRAINING_SESSION.notes.clone(),
            TRAINING_SESSION.elements.clone(),
        ))
        .unwrap();

        assert_eq!(
            run(cached_rest.read_training_sessions()).unwrap(),
            vec![training_session]
        );
    }

    #[test]
    fn test_a_failed_cache_write_still_reports_the_created_routine() {
        let cache = FakeCache::default().failing(CacheCall::WriteRoutine);
        let cached_rest = cached_rest(FakeRemote::default(), cache);

        assert!(
            run(cached_rest.create_routine(
                ROUTINE.name.clone(),
                ROUTINE.notes.clone(),
                ROUTINE.sections.clone(),
            ))
            .is_ok()
        );
    }

    #[test]
    fn test_a_failed_cache_write_still_reports_the_created_training_session() {
        let cache = FakeCache::default().failing(CacheCall::WriteTrainingSession);
        let cached_rest = cached_rest(FakeRemote::default(), cache);

        assert!(
            run(cached_rest.create_training_session(
                TRAINING_SESSION.routine_id,
                TRAINING_SESSION.date,
                TRAINING_SESSION.notes.clone(),
                TRAINING_SESSION.elements.clone(),
            ))
            .is_ok()
        );
    }

    #[test]
    fn test_a_rejected_modification_leaves_the_cache_untouched() {
        let remote = FakeRemote {
            repository: domain::tests::FakeRepository::default()
                .failing(domain::tests::Call::DeleteRoutine),
            ..FakeRemote::default()
        };
        let cache = FakeCache::default();
        let _ = cache.repository.clone().with_routines(ROUTINES.clone());
        let cached_rest = cached_rest(remote, cache.clone());

        assert!(run(cached_rest.delete_routine(ROUTINE.id)).is_err());
        assert_eq!(run(cached_rest.read_routines()).unwrap(), ROUTINES.clone());
    }

    #[test]
    fn test_a_replaced_session_user_updates_the_cached_session() {
        let cache = FakeCache::default().with_session(&USER);
        let cached_rest = cached_rest(FakeRemote::default(), cache);
        let updated = domain::User {
            name: domain::Name::new("Alicia").unwrap(),
            ..USER.clone()
        };

        assert_eq!(
            run(cached_rest.replace_user(updated.clone())).unwrap(),
            updated
        );
        assert_eq!(run(cached_rest.initialize_session()).unwrap(), updated);
    }

    #[test]
    fn test_replacing_another_user_keeps_the_cached_session() {
        let cache = FakeCache::default().with_session(&USER);
        let cached_rest = cached_rest(FakeRemote::default(), cache);

        let _ = run(cached_rest.replace_user(USER_2.clone())).unwrap();

        assert_eq!(run(cached_rest.initialize_session()).unwrap(), USER.clone());
    }

    #[test]
    fn test_an_updated_session_user_updates_the_cached_session() {
        let cache = FakeCache::default().with_session(&USER);
        let cached_rest = cached_rest(FakeRemote::default(), cache);

        let updated = run(cached_rest.update_user(
            USER.id,
            domain::Name::new("Alicia").unwrap(),
            USER.sex,
            USER.height,
        ))
        .unwrap();

        assert_eq!(run(cached_rest.initialize_session()).unwrap(), updated);
    }

    #[test]
    fn test_updating_another_user_keeps_the_cached_session() {
        let cache = FakeCache::default().with_session(&USER);
        let cached_rest = cached_rest(FakeRemote::default(), cache);

        let _ =
            run(cached_rest.update_user(USER_2.id, USER_2.name.clone(), USER_2.sex, USER_2.height))
                .unwrap();

        assert_eq!(run(cached_rest.initialize_session()).unwrap(), USER.clone());
    }

    #[test]
    fn test_reads_that_reach_the_server_are_not_cached() {
        let cached_rest = cached_rest(FakeRemote::default(), FakeCache::default());

        assert_eq!(run(cached_rest.read_users()).unwrap(), vec![]);
        assert_eq!(run(cached_rest.read_version()).unwrap(), "1.0.0");
        assert_eq!(
            run(cached_rest.read_auth_methods()).unwrap(),
            vec![domain::AuthMethod::Passkey, domain::AuthMethod::Username]
        );
    }

    /// The tests that prove the production adapters are wired to the traits above, one per
    /// distinct interaction shape rather than one per branch.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    mod wasm {
        use std::sync::{Arc, Mutex};

        use pretty_assertions::assert_eq;
        use valens_domain::{
            BodyWeightRepository, RoutineRepository, ScheduleRepository, SessionRepository,
        };
        use wasm_bindgen_test::wasm_bindgen_test;

        use crate::{
            rest::{self, SendRequest},
            tests::data::{BODY_WEIGHT, ROUTINE, SCHEDULE, USER},
        };

        use super::*;

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
                domain::SignOut::Complete
            );

            assert!(matches!(
                IndexedDB.initialize_session().await,
                Err(domain::ReadError::Storage(domain::StorageError::NoSession))
            ));
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
        async fn test_modify_routine() {
            reset_cache().await;
            init_session().await;

            IndexedDB
                .write_routines(std::slice::from_ref(&ROUTINE))
                .await
                .unwrap();

            let mut routine = ROUTINE.clone();
            routine.name = domain::Name::new("C").unwrap();
            routine.notes = String::from("D");
            routine.archived = true;
            routine.sections = vec![];

            assert!(matches!(
                cached_rest_with_response(None)
                    .modify_routine(
                        routine.id,
                        Some(routine.name.clone()),
                        Some(routine.notes.clone()),
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
                    Some(routine.notes.clone()),
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
        ) -> CachedREST<REST<MockSendRequest>, IndexedDB> {
            cached_rest_capturing(response).0
        }

        #[allow(clippy::type_complexity)]
        fn cached_rest_capturing(
            response: Option<Result<gloo_net::http::Response, gloo_net::Error>>,
        ) -> (
            CachedREST<REST<MockSendRequest>, IndexedDB>,
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
                    remote: REST { sender },
                    cache: IndexedDB,
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
