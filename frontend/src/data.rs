use chrono::prelude::*;
use seed::prelude::*;
use serde_json::json;

use crate::common;

// ------ ------
//     Init
// ------ ------

pub fn init(url: Url, _orders: &mut impl Orders<Msg>) -> Model {
    Model {
        base_url: url.to_hash_base_url(),
        errors: Vec::new(),
        session: None,
        version: String::new(),
        users: Vec::new(),
        body_weight: Vec::new(),
        body_fat: Vec::new(),
        period: Vec::new(),
        exercises: Vec::new(),
        routines: Vec::new(),
        workouts: Vec::new(),
        last_refresh: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    base_url: Url,
    errors: Vec<String>,

    // ------ Data -----
    pub session: Option<Session>,
    pub version: String,
    pub users: Vec<User>,

    // ------ Session-dependent data ------
    pub body_weight: Vec<BodyWeightStats>,
    pub body_fat: Vec<BodyFatStats>,
    pub period: Vec<Period>,
    pub exercises: Vec<Exercise>,
    pub routines: Vec<Routine>,
    pub workouts: Vec<WorkoutStats>,
    pub last_refresh: DateTime<Utc>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Session {
    pub id: u32,
    pub name: String,
    pub sex: u8,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub sex: i8,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct NewUser {
    pub name: String,
    pub sex: i8,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct BodyWeight {
    pub date: NaiveDate,
    pub weight: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct BodyWeightStats {
    pub date: NaiveDate,
    pub weight: f32,
    pub avg_weight: Option<f32>,
    pub avg_weight_change: Option<f32>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct BodyFat {
    pub date: NaiveDate,
    pub chest: Option<u8>,
    pub abdominal: Option<u8>,
    pub tigh: Option<u8>,
    pub tricep: Option<u8>,
    pub subscapular: Option<u8>,
    pub suprailiac: Option<u8>,
    pub midaxillary: Option<u8>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct BodyFatStats {
    pub date: NaiveDate,
    pub chest: Option<u8>,
    pub abdominal: Option<u8>,
    pub tigh: Option<u8>,
    pub tricep: Option<u8>,
    pub subscapular: Option<u8>,
    pub suprailiac: Option<u8>,
    pub midaxillary: Option<u8>,
    pub jp3: Option<f32>,
    pub jp7: Option<f32>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Period {
    pub date: NaiveDate,
    pub intensity: u8,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Exercise {
    pub id: u32,
    pub name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Routine {
    pub id: u32,
    pub name: String,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Workout {
    pub id: u32,
    pub routine_id: Option<u32>,
    pub date: NaiveDate,
    pub notes: String,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct WorkoutStats {
    pub id: u32,
    pub routine_id: Option<u32>,
    pub routine: String,
    pub date: NaiveDate,
    pub avg_reps: Option<f32>,
    pub avg_time: Option<f32>,
    pub avg_weight: Option<f32>,
    pub avg_rpe: Option<f32>,
    pub volume: u32,
    pub tut: u32,
}

// ------ ------
//    Update
// ------ ------

#[derive(Clone)]
pub enum Msg {
    RemoveError,
    ClearErrors,

    Refresh,
    ClearSessionDependentData,

    RequestSession(u32),
    SessionReceived(Result<Session, String>),
    InitializeSession,
    SessionInitialized(Result<Session, String>),

    DeleteSession,
    SessionDeleted(Result<(), String>),

    FetchVersion,
    VersionFetched(Result<String, String>),

    FetchUsers,
    UsersFetched(Result<Vec<User>, String>),
    CreateUser(NewUser),
    UserCreated(Result<User, String>),
    UpdateUser(User),
    UserUpdated(Result<User, String>),
    DeleteUser(u32),
    UserDeleted(Result<(), String>),

    FetchBodyWeight,
    BodyWeightFetched(Result<Vec<BodyWeightStats>, String>),
    CreateBodyWeight(BodyWeight),
    BodyWeightCreated(Result<BodyWeight, String>),
    UpdateBodyWeight(BodyWeight),
    BodyWeightUpdated(Result<BodyWeight, String>),
    DeleteBodyWeight(NaiveDate),
    BodyWeightDeleted(Result<(), String>),

    FetchBodyFat,
    BodyFatFetched(Result<Vec<BodyFatStats>, String>),
    CreateBodyFat(BodyFat),
    BodyFatCreated(Result<BodyFat, String>),
    UpdateBodyFat(BodyFat),
    BodyFatUpdated(Result<BodyFat, String>),
    DeleteBodyFat(NaiveDate),
    BodyFatDeleted(Result<(), String>),

    FetchPeriod,
    PeriodFetched(Result<Vec<Period>, String>),
    CreatePeriod(Period),
    PeriodCreated(Result<Period, String>),
    UpdatePeriod(Period),
    PeriodUpdated(Result<Period, String>),
    DeletePeriod(NaiveDate),
    PeriodDeleted(Result<(), String>),

    FetchExercises,
    ExercisesFetched(Result<Vec<Exercise>, String>),
    CreateExercise(String),
    ExerciseCreated(Result<Exercise, String>),
    UpdateExercise(Exercise),
    ExerciseUpdated(Result<Exercise, String>),
    DeleteExercise(u32),
    ExerciseDeleted(Result<(), String>),

    FetchRoutines,
    RoutinesFetched(Result<Vec<Routine>, String>),
    CreateRoutine(String),
    RoutineCreated(Result<Routine, String>),
    UpdateRoutine(Routine),
    RoutineUpdated(Result<Routine, String>),
    DeleteRoutine(u32),
    RoutineDeleted(Result<(), String>),

    FetchWorkouts,
    WorkoutsFetched(Result<Vec<WorkoutStats>, String>),
    CreateWorkout(NaiveDate, u32),
    WorkoutCreated(Result<Workout, String>),
    DeleteWorkout(u32),
    WorkoutDeleted(Result<(), String>),
}

#[derive(Clone)]
pub enum Event {
    UserCreationSuccessful,
    UserCreationFailed,
    UserUpdateSuccessful,
    UserUpdateFailed,
    UserDeleteSuccessful,
    UserDeleteFailed,
    BodyWeightCreationSuccessful,
    BodyWeightCreationFailed,
    BodyWeightUpdateSuccessful,
    BodyWeightUpdateFailed,
    BodyWeightDeleteSuccessful,
    BodyWeightDeleteFailed,
    BodyFatCreationSuccessful,
    BodyFatCreationFailed,
    BodyFatUpdateSuccessful,
    BodyFatUpdateFailed,
    BodyFatDeleteSuccessful,
    BodyFatDeleteFailed,
    PeriodCreationSuccessful,
    PeriodCreationFailed,
    PeriodUpdateSuccessful,
    PeriodUpdateFailed,
    PeriodDeleteSuccessful,
    PeriodDeleteFailed,
    ExerciseCreationSuccessful,
    ExerciseCreationFailed,
    ExerciseUpdateSuccessful,
    ExerciseUpdateFailed,
    ExerciseDeleteSuccessful,
    ExerciseDeleteFailed,
    RoutineCreationSuccessful,
    RoutineCreationFailed,
    RoutineUpdateSuccessful,
    RoutineUpdateFailed,
    RoutineDeleteSuccessful,
    RoutineDeleteFailed,
    WorkoutCreationSuccessful,
    WorkoutCreationFailed,
    WorkoutDeleteSuccessful,
    WorkoutDeleteFailed,
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::RemoveError => {
            model.errors.pop();
        }
        Msg::ClearErrors => {
            model.errors.clear();
        }

        Msg::Refresh => {
            orders
                .send_msg(Msg::FetchVersion)
                .send_msg(Msg::FetchUsers)
                .send_msg(Msg::FetchBodyWeight)
                .send_msg(Msg::FetchBodyFat)
                .send_msg(Msg::FetchPeriod)
                .send_msg(Msg::FetchExercises)
                .send_msg(Msg::FetchRoutines)
                .send_msg(Msg::FetchWorkouts);
            model.last_refresh = Utc::now();
        }
        Msg::ClearSessionDependentData => {
            model.body_weight.clear();
            model.body_fat.clear();
            model.period.clear();
            model.exercises.clear();
            model.routines.clear();
            model.workouts.clear();
        }

        Msg::RequestSession(user_id) => {
            orders.skip().perform_cmd(async move {
                fetch(
                    Request::new("api/session")
                        .method(Method::Post)
                        .json(&json!({ "id": user_id }))
                        .expect("serialization failed"),
                    Msg::SessionReceived,
                )
                .await
            });
        }
        Msg::SessionReceived(Ok(new_session)) => {
            model.session = Some(new_session);
            orders.send_msg(Msg::Refresh).request_url(
                crate::Urls::new(&model.base_url.clone().set_hash_path(&[""; 0])).home(),
            );
        }
        Msg::SessionReceived(Err(message)) => {
            model.session = None;
            model
                .errors
                .push("Failed to request session: ".to_owned() + &message);
        }
        Msg::InitializeSession => {
            orders.perform_cmd(async { fetch("api/session", Msg::SessionInitialized).await });
        }
        Msg::SessionInitialized(Ok(session)) => {
            model.session = Some(session);
            orders
                .notify(subs::UrlChanged(Url::current()))
                .send_msg(Msg::Refresh);
        }
        Msg::SessionInitialized(Err(_)) => {
            model.session = None;
            orders.notify(subs::UrlChanged(Url::current()));
        }
        Msg::DeleteSession => {
            orders
                .skip()
                .send_msg(Msg::ClearSessionDependentData)
                .perform_cmd(async {
                    fetch_no_content(
                        Request::new("api/session").method(Method::Delete),
                        Msg::SessionDeleted,
                    )
                    .await
                });
        }
        Msg::SessionDeleted(Ok(_)) => {
            model.session = None;
            orders.request_url(crate::Urls::new(&model.base_url).login());
        }
        Msg::SessionDeleted(Err(message)) => {
            model
                .errors
                .push("Failed to switch users: ".to_owned() + &message);
        }

        Msg::FetchVersion => {
            orders.perform_cmd(async { fetch("api/version", Msg::VersionFetched).await });
        }
        Msg::VersionFetched(Ok(version)) => {
            model.version = version;
        }
        Msg::VersionFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch version: ".to_owned() + &message);
        }

        Msg::FetchUsers => {
            orders.perform_cmd(async { fetch("api/users", Msg::UsersFetched).await });
        }
        Msg::UsersFetched(Ok(users)) => {
            model.users = users;
        }
        Msg::UsersFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch users: ".to_owned() + &message);
        }
        Msg::CreateUser(user) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new("api/users")
                        .method(Method::Post)
                        .json(&user)
                        .expect("serialization failed"),
                    Msg::UserCreated,
                )
                .await
            });
        }
        Msg::UserCreated(Ok(_)) => {
            orders
                .notify(Event::UserCreationSuccessful)
                .send_msg(Msg::FetchUsers);
        }
        Msg::UserCreated(Err(message)) => {
            orders.notify(Event::UserCreationFailed);
            model
                .errors
                .push("Failed to add user: ".to_owned() + &message);
        }
        Msg::UpdateUser(user) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new(format!("api/users/{}", user.id))
                        .method(Method::Put)
                        .json(&NewUser {
                            name: user.name,
                            sex: user.sex,
                        })
                        .expect("serialization failed"),
                    Msg::UserUpdated,
                )
                .await
            });
        }
        Msg::UserUpdated(Ok(_)) => {
            orders
                .notify(Event::UserUpdateSuccessful)
                .send_msg(Msg::FetchUsers);
        }
        Msg::UserUpdated(Err(message)) => {
            orders.notify(Event::UserUpdateFailed);
            model
                .errors
                .push("Failed to update user: ".to_owned() + &message);
        }
        Msg::DeleteUser(id) => {
            orders.perform_cmd(async move {
                fetch_no_content(
                    Request::new(format!("api/users/{}", id)).method(Method::Delete),
                    Msg::UserDeleted,
                )
                .await
            });
        }
        Msg::UserDeleted(Ok(_)) => {
            orders
                .notify(Event::UserDeleteSuccessful)
                .send_msg(Msg::FetchUsers);
        }
        Msg::UserDeleted(Err(message)) => {
            orders.notify(Event::UserDeleteFailed);
            model
                .errors
                .push("Failed to delete user: ".to_owned() + &message);
        }

        Msg::FetchBodyWeight => {
            orders.skip().perform_cmd(async {
                fetch("api/body_weight?format=statistics", Msg::BodyWeightFetched).await
            });
        }
        Msg::BodyWeightFetched(Ok(body_weight)) => {
            model.body_weight = body_weight;
        }
        Msg::BodyWeightFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch body weight: ".to_owned() + &message);
        }
        Msg::CreateBodyWeight(body_weight) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new("api/body_weight")
                        .method(Method::Post)
                        .json(&body_weight)
                        .expect("serialization failed"),
                    Msg::BodyWeightCreated,
                )
                .await
            });
        }
        Msg::BodyWeightCreated(Ok(_)) => {
            orders
                .notify(Event::BodyWeightCreationSuccessful)
                .send_msg(Msg::FetchBodyWeight);
        }
        Msg::BodyWeightCreated(Err(message)) => {
            orders.notify(Event::BodyWeightCreationFailed);
            model
                .errors
                .push("Failed to add body weight: ".to_owned() + &message);
        }
        Msg::UpdateBodyWeight(body_weight) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new(format!("api/body_weight/{}", body_weight.date))
                        .method(Method::Put)
                        .json(&json!({ "weight": body_weight.weight }))
                        .expect("serialization failed"),
                    Msg::BodyWeightUpdated,
                )
                .await
            });
        }
        Msg::BodyWeightUpdated(Ok(_)) => {
            orders
                .notify(Event::BodyWeightUpdateSuccessful)
                .send_msg(Msg::FetchBodyWeight);
        }
        Msg::BodyWeightUpdated(Err(message)) => {
            orders.notify(Event::BodyWeightUpdateFailed);
            model
                .errors
                .push("Failed to update body weight: ".to_owned() + &message);
        }
        Msg::DeleteBodyWeight(date) => {
            orders.perform_cmd(async move {
                fetch_no_content(
                    Request::new(format!("api/body_weight/{}", date)).method(Method::Delete),
                    Msg::BodyWeightDeleted,
                )
                .await
            });
        }
        Msg::BodyWeightDeleted(Ok(_)) => {
            orders
                .notify(Event::BodyWeightDeleteSuccessful)
                .send_msg(Msg::FetchBodyWeight);
        }
        Msg::BodyWeightDeleted(Err(message)) => {
            orders.notify(Event::BodyWeightDeleteFailed);
            model
                .errors
                .push("Failed to delete body weight: ".to_owned() + &message);
        }

        Msg::FetchBodyFat => {
            orders.skip().perform_cmd(async {
                fetch("api/body_fat?format=statistics", Msg::BodyFatFetched).await
            });
        }
        Msg::BodyFatFetched(Ok(body_fat)) => {
            model.body_fat = body_fat;
        }
        Msg::BodyFatFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch body fat: ".to_owned() + &message);
        }
        Msg::CreateBodyFat(body_fat) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new("api/body_fat")
                        .method(Method::Post)
                        .json(&body_fat)
                        .expect("serialization failed"),
                    Msg::BodyFatCreated,
                )
                .await
            });
        }
        Msg::BodyFatCreated(Ok(_)) => {
            orders
                .notify(Event::BodyFatCreationSuccessful)
                .send_msg(Msg::FetchBodyFat);
        }
        Msg::BodyFatCreated(Err(message)) => {
            orders.notify(Event::BodyFatCreationFailed);
            model
                .errors
                .push("Failed to add body fat: ".to_owned() + &message);
        }
        Msg::UpdateBodyFat(body_fat) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new(format!("api/body_fat/{}", body_fat.date))
                        .method(Method::Put)
                        .json(&json!({
                            "chest": body_fat.chest,
                            "abdominal": body_fat.abdominal,
                            "tigh": body_fat.tigh,
                            "tricep": body_fat.tricep,
                            "subscapular": body_fat.subscapular,
                            "suprailiac": body_fat.suprailiac,
                            "midaxillary": body_fat.midaxillary,
                        }))
                        .expect("serialization failed"),
                    Msg::BodyFatUpdated,
                )
                .await
            });
        }
        Msg::BodyFatUpdated(Ok(_)) => {
            orders
                .notify(Event::BodyFatUpdateSuccessful)
                .send_msg(Msg::FetchBodyFat);
        }
        Msg::BodyFatUpdated(Err(message)) => {
            orders.notify(Event::BodyFatUpdateFailed);
            model
                .errors
                .push("Failed to update body fat: ".to_owned() + &message);
        }
        Msg::DeleteBodyFat(date) => {
            orders.perform_cmd(async move {
                fetch_no_content(
                    Request::new(format!("api/body_fat/{}", date)).method(Method::Delete),
                    Msg::BodyFatDeleted,
                )
                .await
            });
        }
        Msg::BodyFatDeleted(Ok(_)) => {
            orders
                .notify(Event::BodyFatDeleteSuccessful)
                .send_msg(Msg::FetchBodyFat);
        }
        Msg::BodyFatDeleted(Err(message)) => {
            orders.notify(Event::BodyFatDeleteFailed);
            model
                .errors
                .push("Failed to delete body fat: ".to_owned() + &message);
        }

        Msg::FetchPeriod => {
            orders
                .skip()
                .perform_cmd(async { fetch("api/period", Msg::PeriodFetched).await });
        }
        Msg::PeriodFetched(Ok(period)) => {
            model.period = period;
        }
        Msg::PeriodFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch period: ".to_owned() + &message);
        }
        Msg::CreatePeriod(period) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new("api/period")
                        .method(Method::Post)
                        .json(&period)
                        .expect("serialization failed"),
                    Msg::PeriodCreated,
                )
                .await
            });
        }
        Msg::PeriodCreated(Ok(_)) => {
            orders
                .notify(Event::PeriodCreationSuccessful)
                .send_msg(Msg::FetchPeriod);
        }
        Msg::PeriodCreated(Err(message)) => {
            orders.notify(Event::PeriodCreationFailed);
            model
                .errors
                .push("Failed to add period: ".to_owned() + &message);
        }
        Msg::UpdatePeriod(period) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new(format!("api/period/{}", period.date))
                        .method(Method::Put)
                        .json(&json!({ "intensity": period.intensity }))
                        .expect("serialization failed"),
                    Msg::PeriodUpdated,
                )
                .await
            });
        }
        Msg::PeriodUpdated(Ok(_)) => {
            orders
                .notify(Event::PeriodUpdateSuccessful)
                .send_msg(Msg::FetchPeriod);
        }
        Msg::PeriodUpdated(Err(message)) => {
            orders.notify(Event::PeriodUpdateFailed);
            model
                .errors
                .push("Failed to update period: ".to_owned() + &message);
        }
        Msg::DeletePeriod(date) => {
            orders.perform_cmd(async move {
                fetch_no_content(
                    Request::new(format!("api/period/{}", date)).method(Method::Delete),
                    Msg::PeriodDeleted,
                )
                .await
            });
        }
        Msg::PeriodDeleted(Ok(_)) => {
            orders
                .notify(Event::PeriodDeleteSuccessful)
                .send_msg(Msg::FetchPeriod);
        }
        Msg::PeriodDeleted(Err(message)) => {
            orders.notify(Event::PeriodDeleteFailed);
            model
                .errors
                .push("Failed to delete period: ".to_owned() + &message);
        }

        Msg::FetchExercises => {
            orders
                .skip()
                .perform_cmd(async { fetch("api/exercises", Msg::ExercisesFetched).await });
        }
        Msg::ExercisesFetched(Ok(exercises)) => {
            model.exercises = exercises;
        }
        Msg::ExercisesFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch exercises: ".to_owned() + &message);
        }
        Msg::CreateExercise(exercise_name) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new("api/exercises")
                        .method(Method::Post)
                        .json(&json!({ "name": exercise_name }))
                        .expect("serialization failed"),
                    Msg::ExerciseCreated,
                )
                .await
            });
        }
        Msg::ExerciseCreated(Ok(_)) => {
            orders
                .notify(Event::ExerciseCreationSuccessful)
                .send_msg(Msg::FetchExercises);
        }
        Msg::ExerciseCreated(Err(message)) => {
            orders.notify(Event::ExerciseCreationFailed);
            model
                .errors
                .push("Failed to add exercise: ".to_owned() + &message);
        }
        Msg::UpdateExercise(exercise) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new(format!("api/exercises/{}", exercise.id))
                        .method(Method::Put)
                        .json(&exercise)
                        .expect("serialization failed"),
                    Msg::ExerciseUpdated,
                )
                .await
            });
        }
        Msg::ExerciseUpdated(Ok(_)) => {
            orders
                .notify(Event::ExerciseUpdateSuccessful)
                .send_msg(Msg::FetchExercises);
        }
        Msg::ExerciseUpdated(Err(message)) => {
            orders.notify(Event::ExerciseUpdateFailed);
            model
                .errors
                .push("Failed to update exercise: ".to_owned() + &message);
        }
        Msg::DeleteExercise(id) => {
            orders.perform_cmd(async move {
                fetch_no_content(
                    Request::new(format!("api/exercises/{}", id)).method(Method::Delete),
                    Msg::ExerciseDeleted,
                )
                .await
            });
        }
        Msg::ExerciseDeleted(Ok(_)) => {
            orders
                .notify(Event::ExerciseDeleteSuccessful)
                .send_msg(Msg::FetchExercises);
        }
        Msg::ExerciseDeleted(Err(message)) => {
            orders.notify(Event::ExerciseDeleteFailed);
            model
                .errors
                .push("Failed to delete exercise: ".to_owned() + &message);
        }

        Msg::FetchRoutines => {
            orders
                .skip()
                .perform_cmd(async { fetch("api/routines", Msg::RoutinesFetched).await });
        }
        Msg::RoutinesFetched(Ok(routines)) => {
            model.routines = routines;
            model.routines.sort_by(|a, b| b.id.cmp(&a.id));
        }
        Msg::RoutinesFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch routines: ".to_owned() + &message);
        }
        Msg::CreateRoutine(routine_name) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new("api/routines")
                        .method(Method::Post)
                        .json(&json!({ "name": routine_name }))
                        .expect("serialization failed"),
                    Msg::RoutineCreated,
                )
                .await
            });
        }
        Msg::RoutineCreated(Ok(_)) => {
            orders
                .notify(Event::RoutineCreationSuccessful)
                .send_msg(Msg::FetchRoutines);
        }
        Msg::RoutineCreated(Err(message)) => {
            orders.notify(Event::RoutineCreationFailed);
            model
                .errors
                .push("Failed to add routine: ".to_owned() + &message);
        }
        Msg::UpdateRoutine(routine) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new(format!("api/routines/{}", routine.id))
                        .method(Method::Put)
                        .json(&routine)
                        .expect("serialization failed"),
                    Msg::RoutineUpdated,
                )
                .await
            });
        }
        Msg::RoutineUpdated(Ok(_)) => {
            orders
                .notify(Event::RoutineUpdateSuccessful)
                .send_msg(Msg::FetchRoutines);
        }
        Msg::RoutineUpdated(Err(message)) => {
            orders.notify(Event::RoutineUpdateFailed);
            model
                .errors
                .push("Failed to update routine: ".to_owned() + &message);
        }
        Msg::DeleteRoutine(id) => {
            orders.perform_cmd(async move {
                fetch_no_content(
                    Request::new(format!("api/routines/{}", id)).method(Method::Delete),
                    Msg::RoutineDeleted,
                )
                .await
            });
        }
        Msg::RoutineDeleted(Ok(_)) => {
            orders
                .notify(Event::RoutineDeleteSuccessful)
                .send_msg(Msg::FetchRoutines);
        }
        Msg::RoutineDeleted(Err(message)) => {
            orders.notify(Event::RoutineDeleteFailed);
            model
                .errors
                .push("Failed to delete routine: ".to_owned() + &message);
        }

        Msg::FetchWorkouts => {
            orders.skip().perform_cmd(async {
                fetch("api/workouts?format=statistics", Msg::WorkoutsFetched).await
            });
        }
        Msg::WorkoutsFetched(Ok(workouts)) => {
            model.workouts = workouts;
        }
        Msg::WorkoutsFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch workouts: ".to_owned() + &message);
        }
        Msg::CreateWorkout(date, routine_id) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new("api/workouts")
                        .method(Method::Post)
                        .json(&json!({ "date": date, "routine_id": routine_id }))
                        .expect("serialization failed"),
                    Msg::WorkoutCreated,
                )
                .await
            });
        }
        Msg::WorkoutCreated(Ok(_)) => {
            orders
                .notify(Event::WorkoutCreationSuccessful)
                .send_msg(Msg::FetchWorkouts);
        }
        Msg::WorkoutCreated(Err(message)) => {
            orders.notify(Event::WorkoutCreationFailed);
            model
                .errors
                .push("Failed to add workout: ".to_owned() + &message);
        }
        Msg::DeleteWorkout(id) => {
            orders.perform_cmd(async move {
                fetch_no_content(
                    Request::new(format!("api/workouts/{}", id)).method(Method::Delete),
                    Msg::WorkoutDeleted,
                )
                .await
            });
        }
        Msg::WorkoutDeleted(Ok(_)) => {
            orders
                .notify(Event::WorkoutDeleteSuccessful)
                .send_msg(Msg::FetchWorkouts);
        }
        Msg::WorkoutDeleted(Err(message)) => {
            orders.notify(Event::WorkoutDeleteFailed);
            model
                .errors
                .push("Failed to delete workout: ".to_owned() + &message);
        }
    }
}

async fn fetch<'a, Ms, T>(
    request: impl Into<Request<'a>>,
    message: fn(Result<T, String>) -> Ms,
) -> Ms
where
    T: 'static + for<'de> serde::Deserialize<'de>,
{
    match seed::browser::fetch::fetch(request).await {
        Ok(response) => match response.check_status() {
            Ok(response) => match response.json::<T>().await {
                Ok(data) => message(Ok(data)),
                Err(error) => message(Err(format!("deserialization failed: {:?}", error))),
            },
            Err(error) => message(Err(format!("unexpected response: {:?}", error))),
        },
        Err(_) => message(Err("no connection".into())),
    }
}

async fn fetch_no_content<'a, Ms>(
    request: impl Into<Request<'a>>,
    message: fn(Result<(), String>) -> Ms,
) -> Ms {
    match seed::browser::fetch::fetch(request).await {
        Ok(response) => match response.check_status() {
            Ok(_) => message(Ok(())),
            Err(error) => message(Err(format!("unexpected response: {:?}", error))),
        },
        Err(_) => message(Err("no connection".into())),
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model) -> Node<Msg> {
    common::view_error_dialog(&model.errors, &ev(Ev::Click, |_| Msg::RemoveError))
}
