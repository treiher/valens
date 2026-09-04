use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;

use crate::{
    Assistance, AuthMethod, AuthRepository, AuthService, BodyFat, BodyFatRepository,
    BodyFatService, BodyWeight, BodyWeightRepository, Category, CreateError, CurrentCycle, Cycle,
    DeleteError, Equipment, Exercise, ExerciseID, ExerciseMuscle, ExerciseRepository,
    ExerciseService, Force, Laterality, Mechanic, Name, Passkey, PasskeyID, Period,
    PeriodRepository, PeriodService, ReadError, Role, Routine, RoutineID, RoutinePart,
    RoutineRepository, RoutineService, Schedule, ScheduleRepository, ScheduleService,
    SessionRepository, SessionService, Sex, SignOut, SyncError, TrainingSession,
    TrainingSessionElement, TrainingSessionID, TrainingSessionRepository, TrainingSessionService,
    UpdateError, User, UserID, UserRepository, UserService, VersionRepository, VersionService,
    body_weight::BodyWeightService, current_cycle, cycles,
};

#[derive(Clone, Copy)]
pub struct Service<R> {
    repository: R,
}

impl<R> Service<R>
where
    R: ExerciseRepository
        + RoutineRepository
        + ScheduleRepository
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
        self.repository.sync_schedule().await?;
        self.repository.sync_training_sessions().await?;
        self.repository.sync_body_weight().await?;
        self.repository.sync_body_fat().await?;
        self.repository.sync_period().await?;
        Ok(())
    }

    pub async fn sync_exercises(&self) -> Result<Vec<Exercise>, SyncError> {
        self.repository.sync_exercises().await
    }

    pub async fn sync_routines(&self) -> Result<Vec<Routine>, SyncError> {
        self.repository.sync_routines().await
    }

    pub async fn sync_schedule(&self) -> Result<Schedule, SyncError> {
        self.repository.sync_schedule().await
    }

    pub async fn sync_training_sessions(&self) -> Result<Vec<TrainingSession>, SyncError> {
        self.repository.sync_training_sessions().await
    }

    pub async fn sync_body_weight(&self) -> Result<Vec<BodyWeight>, SyncError> {
        self.repository.sync_body_weight().await
    }

    pub async fn sync_body_fat(&self) -> Result<Vec<BodyFat>, SyncError> {
        self.repository.sync_body_fat().await
    }

    pub async fn sync_period(&self) -> Result<Vec<Period>, SyncError> {
        self.repository.sync_period().await
    }
}

impl<R: VersionRepository> VersionService for Service<R> {
    async fn get_version(&self) -> Result<String, ReadError> {
        self.repository.read_version().await
    }
}

impl<R: SessionRepository> SessionService for Service<R> {
    async fn request_session(&self, name: Name) -> Result<User, ReadError> {
        self.repository.request_session(name).await
    }

    async fn get_session(&self) -> Result<User, ReadError> {
        self.repository.initialize_session().await
    }

    async fn sync_session(&self) -> Result<Option<User>, SyncError> {
        self.repository.sync_session().await
    }

    async fn delete_session(&self) -> Result<SignOut, DeleteError> {
        self.repository.delete_session().await
    }
}

impl<R: AuthRepository> AuthService for Service<R> {
    async fn get_auth_methods(&self) -> Result<Vec<AuthMethod>, ReadError> {
        self.repository.read_auth_methods().await
    }

    async fn login_with_passkey(&self) -> Result<User, ReadError> {
        self.repository.login_with_passkey().await
    }

    async fn register_passkey(&self) -> Result<Passkey, CreateError> {
        self.repository.register_passkey().await
    }

    async fn get_passkeys(&self, user_id: UserID) -> Result<Vec<Passkey>, ReadError> {
        self.repository.read_passkeys(user_id).await
    }

    async fn rename_passkey(
        &self,
        user_id: UserID,
        id: PasskeyID,
        label: Name,
    ) -> Result<Passkey, UpdateError> {
        self.repository.rename_passkey(user_id, id, label).await
    }

    async fn delete_passkey(&self, user_id: UserID, id: PasskeyID) -> Result<(), DeleteError> {
        self.repository.delete_passkey(user_id, id).await
    }

    async fn create_login_link(&self, user_id: UserID) -> Result<String, CreateError> {
        self.repository.create_login_link(user_id).await
    }

    async fn redeem_login_link(&self, token: String) -> Result<User, ReadError> {
        self.repository.redeem_login_link(token).await
    }
}

impl<R: UserRepository> UserService for Service<R> {
    async fn get_users(&self) -> Result<Vec<User>, ReadError> {
        self.repository.read_users().await
    }

    async fn create_user(
        &self,
        name: Name,
        sex: Sex,
        height: Option<u8>,
        role: Role,
    ) -> Result<User, CreateError> {
        self.repository.create_user(name, sex, height, role).await
    }

    async fn replace_user(&self, user: User) -> Result<User, UpdateError> {
        self.repository.replace_user(user).await
    }

    async fn update_user(
        &self,
        id: UserID,
        name: Name,
        sex: Sex,
        height: Option<u8>,
    ) -> Result<User, UpdateError> {
        self.repository.update_user(id, name, sex, height).await
    }

    async fn delete_user(&self, id: UserID) -> Result<(), DeleteError> {
        self.repository.delete_user(id).await
    }
}

impl<R: ExerciseRepository> ExerciseService for Service<R> {
    async fn get_exercises(&self) -> Result<Vec<Exercise>, ReadError> {
        self.repository.read_exercises().await
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
        self.repository
            .create_exercise(
                name, notes, muscles, force, mechanic, laterality, assistance, equipment, category,
            )
            .await
    }

    async fn replace_exercise(&self, exercise: Exercise) -> Result<Exercise, UpdateError> {
        self.repository.replace_exercise(exercise).await
    }

    async fn delete_exercise(&self, id: ExerciseID) -> Result<(), DeleteError> {
        self.repository.delete_exercise(id).await
    }
}

impl<R: RoutineRepository + ScheduleRepository> RoutineService for Service<R> {
    async fn get_routines(&self) -> Result<Vec<Routine>, ReadError> {
        self.repository.read_routines().await
    }

    async fn create_routine(
        &self,
        name: Name,
        notes: String,
        sections: Vec<RoutinePart>,
    ) -> Result<Routine, CreateError> {
        self.repository.create_routine(name, notes, sections).await
    }

    async fn modify_routine(
        &self,
        id: RoutineID,
        name: Option<Name>,
        notes: Option<String>,
        archived: Option<bool>,
        sections: Option<Vec<RoutinePart>>,
    ) -> Result<Routine, UpdateError> {
        self.repository
            .modify_routine(id, name, notes, archived, sections)
            .await
    }

    async fn delete_routine(&self, id: RoutineID) -> Result<(), DeleteError> {
        if self
            .repository
            .read_schedule()
            .await?
            .routines()
            .contains(&id)
        {
            return Err(DeleteError::Conflict(
                "routine is used in the schedule".to_string(),
            ));
        }
        self.repository.delete_routine(id).await
    }
}

impl<R: ScheduleRepository + RoutineRepository> ScheduleService for Service<R> {
    async fn get_schedule(&self) -> Result<Schedule, ReadError> {
        self.repository.read_schedule().await
    }

    async fn modify_schedule(&self, schedule: Schedule) -> Result<Schedule, UpdateError> {
        let existing = self
            .repository
            .read_routines()
            .await?
            .iter()
            .map(|r| r.id)
            .collect::<BTreeSet<_>>();
        if !schedule.routines().is_subset(&existing) {
            return Err(UpdateError::Conflict(
                "schedule references an unknown routine".to_string(),
            ));
        }
        self.repository.replace_schedule(schedule).await
    }
}

impl<R: TrainingSessionRepository> TrainingSessionService for Service<R> {
    async fn get_training_sessions(&self) -> Result<Vec<TrainingSession>, ReadError> {
        let mut training_sessions = self.repository.read_training_sessions().await?;
        training_sessions.sort_by_key(|t| (t.date, t.id));
        Ok(training_sessions)
    }

    async fn create_training_session(
        &self,
        routine_id: RoutineID,
        date: NaiveDate,
        notes: String,
        elements: Vec<TrainingSessionElement>,
    ) -> Result<TrainingSession, CreateError> {
        self.repository
            .create_training_session(routine_id, date, notes, elements)
            .await
    }

    async fn modify_training_session(
        &self,
        id: TrainingSessionID,
        notes: Option<String>,
        elements: Option<Vec<TrainingSessionElement>>,
        exercise_notes: Option<BTreeMap<ExerciseID, String>>,
    ) -> Result<TrainingSession, UpdateError> {
        self.repository
            .modify_training_session(id, notes, elements, exercise_notes)
            .await
    }

    async fn delete_training_session(&self, id: TrainingSessionID) -> Result<(), DeleteError> {
        self.repository.delete_training_session(id).await
    }
}

impl<R: BodyWeightRepository> BodyWeightService for Service<R> {
    async fn get_body_weight(&self) -> Result<Vec<BodyWeight>, ReadError> {
        self.repository.read_body_weight().await
    }

    async fn create_body_weight(&self, body_weight: BodyWeight) -> Result<BodyWeight, CreateError> {
        self.repository.create_body_weight(body_weight).await
    }

    async fn replace_body_weight(
        &self,
        body_weight: BodyWeight,
    ) -> Result<BodyWeight, UpdateError> {
        self.repository.replace_body_weight(body_weight).await
    }

    async fn delete_body_weight(&self, date: NaiveDate) -> Result<(), DeleteError> {
        self.repository.delete_body_weight(date).await
    }
}

impl<R: BodyFatRepository> BodyFatService for Service<R> {
    async fn get_body_fat(&self) -> Result<Vec<BodyFat>, ReadError> {
        self.repository.read_body_fat().await
    }

    async fn create_body_fat(&self, body_fat: BodyFat) -> Result<BodyFat, CreateError> {
        self.repository.create_body_fat(body_fat).await
    }

    async fn replace_body_fat(&self, body_fat: BodyFat) -> Result<BodyFat, UpdateError> {
        self.repository.replace_body_fat(body_fat).await
    }

    async fn delete_body_fat(&self, date: NaiveDate) -> Result<(), DeleteError> {
        self.repository.delete_body_fat(date).await
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
        let mut period = self.repository.read_period().await?;
        period.sort_by_key(|p| p.date);
        Ok(period)
    }

    async fn create_period(&self, period: Period) -> Result<Period, CreateError> {
        self.repository.create_period(period).await
    }

    async fn replace_period(&self, period: Period) -> Result<Period, UpdateError> {
        self.repository.replace_period(period).await
    }

    async fn delete_period(&self, date: NaiveDate) -> Result<(), DeleteError> {
        self.repository.delete_period(date).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::Duration;
    use pretty_assertions::assert_eq;

    use crate::{
        Intensity, Rotation, RotationID, ScheduleSlot, Weekday,
        tests::{Call, FakeRepository},
    };

    use super::*;

    #[test]
    fn test_sync_syncs_all_collections() {
        let service = Service::new(FakeRepository::default());

        assert!(pollster::block_on(service.sync()).is_ok());
        assert_eq!(
            service.repository.calls(),
            [
                Call::SyncExercises,
                Call::SyncRoutines,
                Call::SyncSchedule,
                Call::SyncTrainingSessions,
                Call::SyncBodyWeight,
                Call::SyncBodyFat,
                Call::SyncPeriod,
            ]
        );
    }

    #[test]
    fn test_sync_stops_at_first_error() {
        let service = Service::new(FakeRepository::default().failing(Call::SyncRoutines));

        assert!(pollster::block_on(service.sync()).is_err());
        assert_eq!(
            service.repository.calls(),
            [Call::SyncExercises, Call::SyncRoutines]
        );
    }

    #[test]
    fn test_delete_routine() {
        let service = Service::new(FakeRepository::default());

        assert!(pollster::block_on(service.delete_routine(1.into())).is_ok());
        assert!(service.repository.calls().contains(&Call::DeleteRoutine));
    }

    #[test]
    fn test_delete_routine_scheduled() {
        let service =
            Service::new(FakeRepository::default().with_schedule(schedule_with_routine(1.into())));

        assert!(matches!(
            pollster::block_on(service.delete_routine(1.into())),
            Err(DeleteError::Conflict(_))
        ));
        assert!(!service.repository.calls().contains(&Call::DeleteRoutine));
    }

    #[test]
    fn test_delete_routine_unreadable_schedule() {
        let service = Service::new(FakeRepository::default().failing(Call::ReadSchedule));

        assert!(matches!(
            pollster::block_on(service.delete_routine(1.into())),
            Err(DeleteError::Other(_))
        ));
        assert!(!service.repository.calls().contains(&Call::DeleteRoutine));
    }

    #[test]
    fn test_modify_schedule_without_routines() {
        let service = Service::new(FakeRepository::default());

        assert_eq!(
            pollster::block_on(service.modify_schedule(Schedule::default())).ok(),
            Some(Schedule::default())
        );
    }

    #[test]
    fn test_modify_schedule_with_known_routines() {
        let service = Service::new(FakeRepository::default().with_routines(vec![routine(1)]));
        let schedule = schedule_with_routine(1.into());

        assert_eq!(
            pollster::block_on(service.modify_schedule(schedule.clone())).ok(),
            Some(schedule)
        );
    }

    #[test]
    fn test_modify_schedule_with_unknown_routine() {
        let service = Service::new(FakeRepository::default().with_routines(vec![routine(1)]));

        assert!(matches!(
            pollster::block_on(service.modify_schedule(schedule_with_routine(2.into()))),
            Err(UpdateError::Conflict(_))
        ));
        assert!(!service.repository.calls().contains(&Call::ReplaceSchedule));
    }

    #[test]
    fn test_get_training_sessions_sorts_by_date_and_id() {
        let service = Service::new(FakeRepository::default().with_training_sessions(vec![
            training_session(3, 2),
            training_session(1, 3),
            training_session(2, 2),
        ]));

        assert_eq!(
            pollster::block_on(service.get_training_sessions())
                .unwrap()
                .iter()
                .map(|t| t.id)
                .collect::<Vec<_>>(),
            [TrainingSessionID::from(2u128), 3.into(), 1.into()]
        );
    }

    #[test]
    fn test_get_period_sorts_by_date() {
        let service = Service::new(FakeRepository::default().with_period(vec![
            period(3),
            period(1),
            period(2),
        ]));

        assert_eq!(
            pollster::block_on(service.get_period())
                .unwrap()
                .iter()
                .map(|p| p.date)
                .collect::<Vec<_>>(),
            [day(1), day(2), day(3)]
        );
    }

    #[test]
    fn test_get_current_cycle() {
        let service = Service::new(FakeRepository::default().with_period(vec![
            period(-40),
            period(-10),
            period(-9),
        ]));

        assert_eq!(
            pollster::block_on(service.get_current_cycle())
                .unwrap()
                .begin,
            day(-10)
        );
    }

    #[test]
    fn test_get_current_cycle_without_cycles() {
        let service = Service::new(FakeRepository::default());

        assert!(matches!(
            pollster::block_on(service.get_current_cycle()),
            Err(ReadError::NotFound)
        ));
    }

    fn schedule_with_routine(routine_id: RoutineID) -> Schedule {
        Schedule::new(
            BTreeMap::from([(
                RotationID::from(1u128),
                Rotation::new(Name::new("R").unwrap(), vec![routine_id]).unwrap(),
            )]),
            BTreeMap::from([(Weekday::Monday, vec![ScheduleSlot::Routine(routine_id)])]),
        )
        .unwrap()
    }

    fn routine(id: u128) -> Routine {
        Routine {
            id: id.into(),
            name: Name::new("R").unwrap(),
            notes: String::new(),
            archived: false,
            sections: vec![],
        }
    }

    fn training_session(id: u128, day_offset: i64) -> TrainingSession {
        TrainingSession {
            id: id.into(),
            routine_id: 1.into(),
            date: day(day_offset),
            notes: String::new(),
            elements: vec![],
            exercise_notes: BTreeMap::new(),
        }
    }

    fn period(day_offset: i64) -> Period {
        Period {
            date: day(day_offset),
            intensity: Intensity::Medium,
        }
    }

    fn day(offset: i64) -> NaiveDate {
        chrono::Local::now().date_naive() + Duration::days(offset)
    }
}
