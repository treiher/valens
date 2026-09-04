use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};

use chrono::NaiveDate;

use crate::{
    Assistance, AuthMethod, AuthRepository, BodyFat, BodyFatRepository, BodyWeight,
    BodyWeightRepository, Category, CreateError, DeleteError, Equipment, Exercise, ExerciseID,
    ExerciseMuscle, ExerciseRepository, Force, Laterality, Mechanic, Name, Passkey, PasskeyID,
    Period, PeriodRepository, ReadError, Role, Routine, RoutineID, RoutinePart, RoutineRepository,
    Schedule, ScheduleRepository, SessionRepository, Sex, SignOut, SyncError, TrainingSession,
    TrainingSessionElement, TrainingSessionID, TrainingSessionRepository, UpdateError, User,
    UserID, UserRepository, VersionRepository,
};

/// Repository operation that tests observe in the call log or let fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Call {
    SyncExercises,
    SyncRoutines,
    SyncSchedule,
    SyncTrainingSessions,
    SyncBodyWeight,
    SyncBodyFat,
    SyncPeriod,
    ReadExercises,
    ReadRoutines,
    ReadSchedule,
    ReadTrainingSessions,
    ReadBodyWeight,
    ReadBodyFat,
    ReadPeriod,
    ReadUsers,
    DeleteRoutine,
    ReplaceSchedule,
}

/// Repository holding its collections in memory.
///
/// Reads return the stored collection, writes return a value built from their arguments without
/// changing the collections. Every operation of [`Call`] is appended to the call log and returns an
/// error instead of its result while it is marked as failing.
#[derive(Default)]
pub(crate) struct FakeRepository {
    exercises: Mutex<Vec<Exercise>>,
    routines: Mutex<Vec<Routine>>,
    schedule: Mutex<Schedule>,
    training_sessions: Mutex<Vec<TrainingSession>>,
    body_weight: Mutex<Vec<BodyWeight>>,
    body_fat: Mutex<Vec<BodyFat>>,
    period: Mutex<Vec<Period>>,
    users: Mutex<Vec<User>>,
    calls: Mutex<Vec<Call>>,
    failing: Mutex<BTreeSet<Call>>,
}

impl FakeRepository {
    pub(crate) fn with_exercises(self, exercises: Vec<Exercise>) -> Self {
        *self.exercises.lock().unwrap() = exercises;
        self
    }

    pub(crate) fn with_routines(self, routines: Vec<Routine>) -> Self {
        *self.routines.lock().unwrap() = routines;
        self
    }

    pub(crate) fn with_schedule(self, schedule: Schedule) -> Self {
        *self.schedule.lock().unwrap() = schedule;
        self
    }

    pub(crate) fn with_training_sessions(self, training_sessions: Vec<TrainingSession>) -> Self {
        *self.training_sessions.lock().unwrap() = training_sessions;
        self
    }

    pub(crate) fn with_body_weight(self, body_weight: Vec<BodyWeight>) -> Self {
        *self.body_weight.lock().unwrap() = body_weight;
        self
    }

    pub(crate) fn with_body_fat(self, body_fat: Vec<BodyFat>) -> Self {
        *self.body_fat.lock().unwrap() = body_fat;
        self
    }

    pub(crate) fn with_period(self, period: Vec<Period>) -> Self {
        *self.period.lock().unwrap() = period;
        self
    }

    pub(crate) fn with_users(self, users: Vec<User>) -> Self {
        *self.users.lock().unwrap() = users;
        self
    }

    pub(crate) fn failing(self, call: Call) -> Self {
        self.failing.lock().unwrap().insert(call);
        self
    }

    pub(crate) fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    fn called(&self, call: Call) -> bool {
        self.calls.lock().unwrap().push(call);
        self.failing.lock().unwrap().contains(&call)
    }

    fn read<T: Clone>(&self, call: Call, values: &Mutex<Vec<T>>) -> Result<Vec<T>, ReadError> {
        if self.called(call) {
            return Err(ReadError::NotFound);
        }
        Ok(values.lock().unwrap().clone())
    }

    fn sync<T: Clone>(&self, call: Call, values: &Mutex<Vec<T>>) -> Result<Vec<T>, SyncError> {
        if self.called(call) {
            return Err(SyncError::Other("sync failed".into()));
        }
        Ok(values.lock().unwrap().clone())
    }
}

impl ExerciseRepository for FakeRepository {
    async fn sync_exercises(&self) -> Result<Vec<Exercise>, SyncError> {
        self.sync(Call::SyncExercises, &self.exercises)
    }

    async fn read_exercises(&self) -> Result<Vec<Exercise>, ReadError> {
        self.read(Call::ReadExercises, &self.exercises)
    }

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
    ) -> Result<Exercise, CreateError> {
        Ok(Exercise {
            id: ExerciseID::nil(),
            name,
            notes,
            muscles,
            force,
            mechanic,
            laterality,
            assistance,
            equipment,
            category,
        })
    }

    async fn replace_exercise(&self, exercise: Exercise) -> Result<Exercise, UpdateError> {
        Ok(exercise)
    }

    async fn delete_exercise(&self, _: ExerciseID) -> Result<(), DeleteError> {
        Ok(())
    }
}

impl RoutineRepository for FakeRepository {
    async fn sync_routines(&self) -> Result<Vec<Routine>, SyncError> {
        self.sync(Call::SyncRoutines, &self.routines)
    }

    async fn read_routines(&self) -> Result<Vec<Routine>, ReadError> {
        self.read(Call::ReadRoutines, &self.routines)
    }

    async fn create_routine(
        &self,
        name: Name,
        notes: String,
        sections: Vec<RoutinePart>,
    ) -> Result<Routine, CreateError> {
        Ok(Routine {
            id: RoutineID::nil(),
            name,
            notes,
            archived: false,
            sections,
        })
    }

    async fn modify_routine(
        &self,
        id: RoutineID,
        name: Option<Name>,
        notes: Option<String>,
        archived: Option<bool>,
        sections: Option<Vec<RoutinePart>>,
    ) -> Result<Routine, UpdateError> {
        Ok(Routine {
            id,
            name: name.unwrap_or_else(|| Name::new("routine").unwrap()),
            notes: notes.unwrap_or_default(),
            archived: archived.unwrap_or_default(),
            sections: sections.unwrap_or_default(),
        })
    }

    async fn delete_routine(&self, _: RoutineID) -> Result<(), DeleteError> {
        if self.called(Call::DeleteRoutine) {
            return Err(DeleteError::Other("delete failed".into()));
        }
        Ok(())
    }
}

impl ScheduleRepository for FakeRepository {
    async fn sync_schedule(&self) -> Result<Schedule, SyncError> {
        if self.called(Call::SyncSchedule) {
            return Err(SyncError::Other("sync failed".into()));
        }
        Ok(self.schedule.lock().unwrap().clone())
    }

    async fn read_schedule(&self) -> Result<Schedule, ReadError> {
        if self.called(Call::ReadSchedule) {
            return Err(ReadError::NotFound);
        }
        Ok(self.schedule.lock().unwrap().clone())
    }

    async fn replace_schedule(&self, schedule: Schedule) -> Result<Schedule, UpdateError> {
        if self.called(Call::ReplaceSchedule) {
            return Err(UpdateError::Other("replace failed".into()));
        }
        Ok(schedule)
    }
}

impl TrainingSessionRepository for FakeRepository {
    async fn sync_training_sessions(&self) -> Result<Vec<TrainingSession>, SyncError> {
        self.sync(Call::SyncTrainingSessions, &self.training_sessions)
    }

    async fn read_training_sessions(&self) -> Result<Vec<TrainingSession>, ReadError> {
        self.read(Call::ReadTrainingSessions, &self.training_sessions)
    }

    async fn create_training_session(
        &self,
        routine_id: RoutineID,
        date: NaiveDate,
        notes: String,
        elements: Vec<TrainingSessionElement>,
    ) -> Result<TrainingSession, CreateError> {
        Ok(TrainingSession {
            id: TrainingSessionID::nil(),
            routine_id,
            date,
            notes,
            elements,
            exercise_notes: BTreeMap::new(),
        })
    }

    async fn modify_training_session(
        &self,
        id: TrainingSessionID,
        notes: Option<String>,
        elements: Option<Vec<TrainingSessionElement>>,
        exercise_notes: Option<BTreeMap<ExerciseID, String>>,
    ) -> Result<TrainingSession, UpdateError> {
        Ok(TrainingSession {
            id,
            routine_id: RoutineID::nil(),
            date: NaiveDate::default(),
            notes: notes.unwrap_or_default(),
            elements: elements.unwrap_or_default(),
            exercise_notes: exercise_notes.unwrap_or_default(),
        })
    }

    async fn delete_training_session(&self, _: TrainingSessionID) -> Result<(), DeleteError> {
        Ok(())
    }
}

impl BodyWeightRepository for FakeRepository {
    async fn sync_body_weight(&self) -> Result<Vec<BodyWeight>, SyncError> {
        self.sync(Call::SyncBodyWeight, &self.body_weight)
    }

    async fn read_body_weight(&self) -> Result<Vec<BodyWeight>, ReadError> {
        self.read(Call::ReadBodyWeight, &self.body_weight)
    }

    async fn create_body_weight(&self, body_weight: BodyWeight) -> Result<BodyWeight, CreateError> {
        Ok(body_weight)
    }

    async fn replace_body_weight(
        &self,
        body_weight: BodyWeight,
    ) -> Result<BodyWeight, UpdateError> {
        Ok(body_weight)
    }

    async fn delete_body_weight(&self, _: NaiveDate) -> Result<(), DeleteError> {
        Ok(())
    }
}

impl BodyFatRepository for FakeRepository {
    async fn sync_body_fat(&self) -> Result<Vec<BodyFat>, SyncError> {
        self.sync(Call::SyncBodyFat, &self.body_fat)
    }

    async fn read_body_fat(&self) -> Result<Vec<BodyFat>, ReadError> {
        self.read(Call::ReadBodyFat, &self.body_fat)
    }

    async fn create_body_fat(&self, body_fat: BodyFat) -> Result<BodyFat, CreateError> {
        Ok(body_fat)
    }

    async fn replace_body_fat(&self, body_fat: BodyFat) -> Result<BodyFat, UpdateError> {
        Ok(body_fat)
    }

    async fn delete_body_fat(&self, _: NaiveDate) -> Result<(), DeleteError> {
        Ok(())
    }
}

impl PeriodRepository for FakeRepository {
    async fn sync_period(&self) -> Result<Vec<Period>, SyncError> {
        self.sync(Call::SyncPeriod, &self.period)
    }

    async fn read_period(&self) -> Result<Vec<Period>, ReadError> {
        self.read(Call::ReadPeriod, &self.period)
    }

    async fn create_period(&self, period: Period) -> Result<Period, CreateError> {
        Ok(period)
    }

    async fn replace_period(&self, period: Period) -> Result<Period, UpdateError> {
        Ok(period)
    }

    async fn delete_period(&self, _: NaiveDate) -> Result<(), DeleteError> {
        Ok(())
    }
}

impl UserRepository for FakeRepository {
    async fn read_users(&self) -> Result<Vec<User>, ReadError> {
        self.read(Call::ReadUsers, &self.users)
    }

    async fn create_user(
        &self,
        name: Name,
        sex: Sex,
        height: Option<u8>,
        role: Role,
    ) -> Result<User, CreateError> {
        Ok(User {
            id: UserID::nil(),
            name,
            sex,
            height,
            role,
        })
    }

    async fn replace_user(&self, user: User) -> Result<User, UpdateError> {
        Ok(user)
    }

    async fn update_user(
        &self,
        id: UserID,
        name: Name,
        sex: Sex,
        height: Option<u8>,
    ) -> Result<User, UpdateError> {
        Ok(User {
            id,
            name,
            sex,
            height,
            role: Role::USER,
        })
    }

    async fn delete_user(&self, _: UserID) -> Result<(), DeleteError> {
        Ok(())
    }
}

impl SessionRepository for FakeRepository {
    async fn request_session(&self, name: Name) -> Result<User, ReadError> {
        Ok(user(name))
    }

    async fn initialize_session(&self) -> Result<User, ReadError> {
        Ok(user(Name::new("Alice").unwrap()))
    }

    async fn sync_session(&self) -> Result<Option<User>, SyncError> {
        Ok(Some(user(Name::new("Alice").unwrap())))
    }

    async fn delete_session(&self) -> Result<SignOut, DeleteError> {
        Ok(SignOut::Complete)
    }
}

impl AuthRepository for FakeRepository {
    async fn read_auth_methods(&self) -> Result<Vec<AuthMethod>, ReadError> {
        Ok(vec![AuthMethod::Passkey, AuthMethod::Username])
    }

    async fn login_with_passkey(&self) -> Result<User, ReadError> {
        Ok(user(Name::new("Alice").unwrap()))
    }

    async fn register_passkey(&self) -> Result<Passkey, CreateError> {
        Ok(passkey(Name::new("Passkey").unwrap()))
    }

    async fn read_passkeys(&self, _: UserID) -> Result<Vec<Passkey>, ReadError> {
        Ok(vec![passkey(Name::new("Passkey").unwrap())])
    }

    async fn rename_passkey(
        &self,
        _: UserID,
        id: PasskeyID,
        label: Name,
    ) -> Result<Passkey, UpdateError> {
        Ok(Passkey {
            id,
            ..passkey(label)
        })
    }

    async fn delete_passkey(&self, _: UserID, _: PasskeyID) -> Result<(), DeleteError> {
        Ok(())
    }

    async fn create_login_link(&self, _: UserID) -> Result<String, CreateError> {
        Ok("https://example.com/login/token".to_string())
    }

    async fn redeem_login_link(&self, _: String) -> Result<User, ReadError> {
        Ok(user(Name::new("Alice").unwrap()))
    }
}

impl VersionRepository for FakeRepository {
    async fn read_version(&self) -> Result<String, ReadError> {
        Ok("1.0.0".to_string())
    }
}

fn user(name: Name) -> User {
    User {
        id: 1.into(),
        name,
        sex: Sex::FEMALE,
        height: None,
        role: Role::USER,
    }
}

fn passkey(label: Name) -> Passkey {
    Passkey {
        id: 1.into(),
        label,
        created: NaiveDate::default(),
        last_used: None,
    }
}
