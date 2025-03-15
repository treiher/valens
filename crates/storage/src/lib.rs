#![warn(clippy::pedantic)]

use async_trait::async_trait;
use chrono::NaiveDate;
use valens_domain::{
    BodyFat, BodyWeight, Exercise, ExerciseMuscle, Period, Routine, RoutinePart, TrainingSession,
    TrainingSessionElement, User,
};
use valens_web_app::{OngoingTrainingSession, Settings};

#[allow(clippy::module_name_repetitions)]
pub mod local_storage;
pub mod rest;

#[async_trait(?Send)]
pub trait Storage {
    async fn request_session(&self, user_id: u32) -> Result<User, String>;
    async fn initialize_session(&self) -> Result<User, String>;
    async fn delete_session(&self) -> Result<(), String>;

    async fn read_version(&self) -> Result<String, String>;

    async fn read_users(&self) -> Result<Vec<User>, String>;
    async fn create_user(&self, name: String, sex: u8) -> Result<User, String>;
    async fn replace_user(&self, user: User) -> Result<User, String>;
    async fn delete_user(&self, id: u32) -> Result<u32, String>;

    async fn read_body_weight(&self) -> Result<Vec<BodyWeight>, String>;
    async fn create_body_weight(&self, body_weight: BodyWeight) -> Result<BodyWeight, String>;
    async fn replace_body_weight(&self, body_weight: BodyWeight) -> Result<BodyWeight, String>;
    async fn delete_body_weight(&self, date: NaiveDate) -> Result<NaiveDate, String>;

    async fn read_body_fat(&self) -> Result<Vec<BodyFat>, String>;
    async fn create_body_fat(&self, body_fat: BodyFat) -> Result<BodyFat, String>;
    async fn replace_body_fat(&self, body_fat: BodyFat) -> Result<BodyFat, String>;
    async fn delete_body_fat(&self, date: NaiveDate) -> Result<NaiveDate, String>;

    async fn read_period(&self) -> Result<Vec<Period>, String>;
    async fn create_period(&self, period: Period) -> Result<Period, String>;
    async fn replace_period(&self, period: Period) -> Result<Period, String>;
    async fn delete_period(&self, date: NaiveDate) -> Result<NaiveDate, String>;

    async fn read_exercises(&self) -> Result<Vec<Exercise>, String>;
    async fn create_exercise(
        &self,
        name: String,
        muscles: Vec<ExerciseMuscle>,
    ) -> Result<Exercise, String>;
    async fn replace_exercise(&self, exercise: Exercise) -> Result<Exercise, String>;
    async fn delete_exercise(&self, id: u32) -> Result<u32, String>;

    async fn read_routines(&self) -> Result<Vec<Routine>, String>;
    async fn create_routine(
        &self,
        name: String,
        sections: Vec<RoutinePart>,
    ) -> Result<Routine, String>;
    async fn modify_routine(
        &self,
        id: u32,
        name: Option<String>,
        archived: Option<bool>,
        sections: Option<Vec<RoutinePart>>,
    ) -> Result<Routine, String>;
    async fn delete_routine(&self, id: u32) -> Result<u32, String>;

    async fn read_training_sessions(&self) -> Result<Vec<TrainingSession>, String>;
    async fn create_training_session(
        &self,
        routine_id: Option<u32>,
        date: NaiveDate,
        notes: String,
        elements: Vec<TrainingSessionElement>,
    ) -> Result<TrainingSession, String>;
    async fn modify_training_session(
        &self,
        id: u32,
        notes: Option<String>,
        elements: Option<Vec<TrainingSessionElement>>,
    ) -> Result<TrainingSession, String>;
    async fn delete_training_session(&self, id: u32) -> Result<u32, String>;
}

#[async_trait(?Send)]
pub trait UI {
    async fn read_settings(&self) -> Result<Settings, String>;
    async fn write_settings(&self, settings: Settings) -> Result<(), String>;

    async fn read_ongoing_training_session(&self)
    -> Result<Option<OngoingTrainingSession>, String>;
    async fn write_ongoing_training_session(
        &self,
        ongoing_training_session: Option<OngoingTrainingSession>,
    ) -> Result<(), String>;
}
