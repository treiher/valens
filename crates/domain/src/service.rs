use chrono::NaiveDate;
use log::{debug, error};

use crate::{
    BodyFat, BodyFatRepository, BodyFatService, BodyWeight, BodyWeightRepository, CreateError,
    CurrentCycle, Cycle, DeleteError, Exercise, ExerciseID, ExerciseMuscle, ExerciseRepository,
    ExerciseService, Name, Period, PeriodRepository, PeriodService, ReadError, Routine, RoutineID,
    RoutinePart, RoutineRepository, RoutineService, SessionRepository, SessionService, Sex,
    SyncError, TrainingSession, TrainingSessionElement, TrainingSessionID,
    TrainingSessionRepository, TrainingSessionService, UpdateError, User, UserID, UserRepository,
    UserService, VersionRepository, VersionService, body_weight::BodyWeightService, current_cycle,
    cycles,
};

pub struct Service<R> {
    repository: R,
}

impl<R> Service<R>
where
    R: ExerciseRepository
        + RoutineRepository
        + TrainingSessionRepository
        + BodyWeightRepository
        + BodyFatRepository
        + PeriodRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn sync(&self) -> Result<(), SyncError> {
        self.repository.sync_exercises().await?;
        self.repository.sync_routines().await?;
        self.repository.sync_training_sessions().await?;
        self.repository.sync_body_weight().await?;
        self.repository.sync_body_fat().await?;
        self.repository.sync_period().await?;
        Ok(())
    }
}

macro_rules! log_on_error {
    ($func: expr, $error: ident, $action: literal, $entity: literal) => {{
        let result = $func.await;
        match result {
            Ok(_) => {}
            Err(ref err) => match err {
                $error::Storage(crate::StorageError::NoConnection) => {
                    debug!("failed to {} {}: {err}", $action, $entity);
                }
                _ => {
                    error!("failed to {} {}: {err}", $action, $entity);
                }
            },
        }
        result
    }};
}

impl<R: VersionRepository> VersionService for Service<R> {
    async fn get_version(&self) -> Result<String, ReadError> {
        log_on_error!(self.repository.read_version(), ReadError, "get", "version")
    }
}

impl<R: SessionRepository> SessionService for Service<R> {
    async fn request_session(&self, user_id: UserID) -> Result<User, ReadError> {
        log_on_error!(
            self.repository.request_session(user_id),
            ReadError,
            "request",
            "session"
        )
    }

    async fn get_session(&self) -> Result<User, ReadError> {
        log_on_error!(
            self.repository.initialize_session(),
            ReadError,
            "get",
            "session"
        )
    }

    async fn delete_session(&self) -> Result<(), DeleteError> {
        log_on_error!(
            self.repository.delete_session(),
            DeleteError,
            "delete",
            "session"
        )
    }
}

impl<R: UserRepository> UserService for Service<R> {
    async fn get_users(&self) -> Result<Vec<User>, ReadError> {
        log_on_error!(self.repository.read_users(), ReadError, "get", "users")
    }

    async fn create_user(&self, name: Name, sex: Sex) -> Result<User, CreateError> {
        log_on_error!(
            self.repository.create_user(name, sex),
            CreateError,
            "create",
            "user"
        )
    }

    async fn replace_user(&self, user: User) -> Result<User, UpdateError> {
        log_on_error!(
            self.repository.replace_user(user),
            UpdateError,
            "replace",
            "user"
        )
    }

    async fn delete_user(&self, id: UserID) -> Result<UserID, DeleteError> {
        log_on_error!(
            self.repository.delete_user(id),
            DeleteError,
            "delete",
            "user"
        )
    }
}

impl<R: ExerciseRepository> ExerciseService for Service<R> {
    async fn get_exercises(&self) -> Result<Vec<Exercise>, ReadError> {
        log_on_error!(
            self.repository.read_exercises(),
            ReadError,
            "get",
            "exercises"
        )
    }

    async fn create_exercise(
        &self,
        name: Name,
        muscles: Vec<ExerciseMuscle>,
    ) -> Result<Exercise, CreateError> {
        log_on_error!(
            self.repository.create_exercise(name, muscles),
            CreateError,
            "create",
            "exercise"
        )
    }

    async fn replace_exercise(&self, exercise: Exercise) -> Result<Exercise, UpdateError> {
        log_on_error!(
            self.repository.replace_exercise(exercise),
            UpdateError,
            "replace",
            "exercise"
        )
    }

    async fn delete_exercise(&self, id: ExerciseID) -> Result<ExerciseID, DeleteError> {
        log_on_error!(
            self.repository.delete_exercise(id),
            DeleteError,
            "delete",
            "exercise"
        )
    }
}

impl<R: RoutineRepository> RoutineService for Service<R> {
    async fn get_routines(&self) -> Result<Vec<Routine>, ReadError> {
        log_on_error!(
            self.repository.read_routines(),
            ReadError,
            "get",
            "routines"
        )
    }

    async fn create_routine(
        &self,
        name: Name,
        sections: Vec<RoutinePart>,
    ) -> Result<Routine, CreateError> {
        log_on_error!(
            self.repository.create_routine(name, sections),
            CreateError,
            "create",
            "routine"
        )
    }

    async fn modify_routine(
        &self,
        id: RoutineID,
        name: Option<Name>,
        archived: Option<bool>,
        sections: Option<Vec<RoutinePart>>,
    ) -> Result<Routine, UpdateError> {
        log_on_error!(
            self.repository.modify_routine(id, name, archived, sections),
            UpdateError,
            "modify",
            "routine"
        )
    }

    async fn delete_routine(&self, id: RoutineID) -> Result<RoutineID, DeleteError> {
        log_on_error!(
            self.repository.delete_routine(id),
            DeleteError,
            "delete",
            "routine"
        )
    }
}

impl<R: TrainingSessionRepository> TrainingSessionService for Service<R> {
    async fn get_training_sessions(&self) -> Result<Vec<TrainingSession>, ReadError> {
        log_on_error!(
            self.repository.read_training_sessions(),
            ReadError,
            "get",
            "training sessions"
        )
    }

    async fn create_training_session(
        &self,
        routine_id: RoutineID,
        date: NaiveDate,
        notes: String,
        elements: Vec<TrainingSessionElement>,
    ) -> Result<TrainingSession, CreateError> {
        log_on_error!(
            self.repository
                .create_training_session(routine_id, date, notes, elements),
            CreateError,
            "create",
            "training session"
        )
    }

    async fn modify_training_session(
        &self,
        id: TrainingSessionID,
        notes: Option<String>,
        elements: Option<Vec<TrainingSessionElement>>,
    ) -> Result<TrainingSession, UpdateError> {
        log_on_error!(
            self.repository.modify_training_session(id, notes, elements),
            UpdateError,
            "modify",
            "training session"
        )
    }

    async fn delete_training_session(
        &self,
        id: TrainingSessionID,
    ) -> Result<TrainingSessionID, DeleteError> {
        log_on_error!(
            self.repository.delete_training_session(id),
            DeleteError,
            "delete",
            "training session"
        )
    }
}

impl<R: BodyWeightRepository> BodyWeightService for Service<R> {
    async fn get_body_weight(&self) -> Result<Vec<BodyWeight>, ReadError> {
        log_on_error!(
            self.repository.read_body_weight(),
            ReadError,
            "get",
            "body weight"
        )
    }

    async fn get_body_weight_on(&self, date: NaiveDate) -> Result<BodyWeight, ReadError> {
        let body_weight = self.get_body_weight().await?;
        body_weight
            .into_iter()
            .filter(|bw| bw.date <= date)
            .max_by(|a, b| a.date.cmp(&b.date))
            .ok_or(ReadError::NotFound)
    }

    async fn create_body_weight(&self, body_weight: BodyWeight) -> Result<BodyWeight, CreateError> {
        log_on_error!(
            self.repository.create_body_weight(body_weight),
            CreateError,
            "create",
            "body weight"
        )
    }

    async fn replace_body_weight(
        &self,
        body_weight: BodyWeight,
    ) -> Result<BodyWeight, UpdateError> {
        log_on_error!(
            self.repository.replace_body_weight(body_weight),
            UpdateError,
            "replace",
            "body weight"
        )
    }

    async fn delete_body_weight(&self, date: NaiveDate) -> Result<NaiveDate, DeleteError> {
        log_on_error!(
            self.repository.delete_body_weight(date),
            DeleteError,
            "delete",
            "body weight"
        )
    }
}

impl<R: BodyFatRepository> BodyFatService for Service<R> {
    async fn get_body_fat(&self) -> Result<Vec<BodyFat>, ReadError> {
        log_on_error!(
            self.repository.read_body_fat(),
            ReadError,
            "get",
            "body fat"
        )
    }

    async fn get_body_fat_on(&self, date: NaiveDate) -> Result<BodyFat, ReadError> {
        let body_fat = self.get_body_fat().await?;
        body_fat
            .into_iter()
            .filter(|bf| bf.date <= date)
            .max_by(|a, b| a.date.cmp(&b.date))
            .ok_or(ReadError::NotFound)
    }

    async fn create_body_fat(&self, body_fat: BodyFat) -> Result<BodyFat, CreateError> {
        log_on_error!(
            self.repository.create_body_fat(body_fat),
            CreateError,
            "create",
            "body fat"
        )
    }

    async fn replace_body_fat(&self, body_fat: BodyFat) -> Result<BodyFat, UpdateError> {
        log_on_error!(
            self.repository.replace_body_fat(body_fat),
            UpdateError,
            "replace",
            "body fat"
        )
    }

    async fn delete_body_fat(&self, date: NaiveDate) -> Result<NaiveDate, DeleteError> {
        log_on_error!(
            self.repository.delete_body_fat(date),
            DeleteError,
            "delete",
            "body fat"
        )
    }
}

impl<R: PeriodRepository> PeriodService for Service<R> {
    async fn get_cycles(&self) -> Result<Vec<Cycle>, ReadError> {
        Ok(cycles(&self.get_period().await?))
    }

    async fn get_current_cycle(&self) -> Result<CurrentCycle, ReadError> {
        current_cycle(&self.get_cycles().await?).ok_or(ReadError::NotFound)
    }

    async fn get_period(&self) -> Result<Vec<Period>, ReadError> {
        log_on_error!(self.repository.read_period(), ReadError, "get", "period")
    }

    async fn create_period(&self, period: Period) -> Result<Period, CreateError> {
        log_on_error!(
            self.repository.create_period(period),
            CreateError,
            "create",
            "period"
        )
    }

    async fn replace_period(&self, period: Period) -> Result<Period, UpdateError> {
        log_on_error!(
            self.repository.replace_period(period),
            UpdateError,
            "replace",
            "period"
        )
    }

    async fn delete_period(&self, date: NaiveDate) -> Result<NaiveDate, DeleteError> {
        log_on_error!(
            self.repository.delete_period(date),
            DeleteError,
            "delete",
            "period"
        )
    }
}
