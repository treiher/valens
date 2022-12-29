use std::collections::BTreeSet;

use chrono::{prelude::*, Duration};
use seed::prelude::*;
use serde_json::{json, Map};

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
        cycles: Vec::new(),
        current_cycle: None,
        exercises: Vec::new(),
        routines: Vec::new(),
        workouts: Vec::new(),
        last_refresh: DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
            Utc,
        ),
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    pub base_url: Url,
    errors: Vec<String>,

    // ------ Data -----
    pub session: Option<Session>,
    pub version: String,
    pub users: Vec<User>,

    // ------ Session-dependent data ------
    pub body_weight: Vec<BodyWeight>,
    pub body_fat: Vec<BodyFat>,
    pub period: Vec<Period>,
    pub cycles: Vec<Cycle>,
    pub current_cycle: Option<CurrentCycle>,
    pub exercises: Vec<Exercise>,
    pub routines: Vec<Routine>,
    pub workouts: Vec<Workout>,
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
    #[serde(skip)]
    pub avg_weight: Option<f32>,
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
pub struct Period {
    pub date: NaiveDate,
    pub intensity: u8,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct Cycle {
    pub begin: NaiveDate,
    pub length: Duration,
}

pub struct CurrentCycle {
    pub begin: NaiveDate,
    pub time_left: Duration,
    pub time_left_variation: Duration,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct CycleStats {
    pub length_median: Duration,
    pub length_variation: Duration,
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
    pub notes: Option<String>,
    pub sections: Vec<RoutinePart>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum RoutinePart {
    RoutineSection {
        position: u32,
        rounds: u32,
        parts: Vec<RoutinePart>,
    },
    RoutineActivity {
        position: u32,
        exercise_id: Option<u32>,
        duration: u32,
        tempo: u32,
        automatic: bool,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Workout {
    pub id: u32,
    pub routine_id: Option<u32>,
    pub date: NaiveDate,
    pub notes: Option<String>,
    pub sets: Vec<WorkoutSet>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct WorkoutSet {
    pub position: u32,
    pub exercise_id: u32,
    pub reps: Option<u32>,
    pub time: Option<u32>,
    pub weight: Option<f32>,
    pub rpe: Option<f32>,
}

impl BodyFat {
    pub fn jp3(&self, sex: u8) -> Option<f32> {
        if sex == 0 {
            Some(self.jackson_pollock(
                self.tricep? as f32 + self.suprailiac? as f32 + self.tigh? as f32,
                1.0994921,
                0.0009929,
                0.0000023,
                0.0001392,
            ))
        } else if sex == 1 {
            Some(self.jackson_pollock(
                self.chest? as f32 + self.abdominal? as f32 + self.tigh? as f32,
                1.10938,
                0.0008267,
                0.0000016,
                0.0002574,
            ))
        } else {
            None
        }
    }

    pub fn jp7(&self, sex: u8) -> Option<f32> {
        if sex == 0 {
            Some(self.jackson_pollock(
                self.chest? as f32
                    + self.abdominal? as f32
                    + self.tigh? as f32
                    + self.tricep? as f32
                    + self.subscapular? as f32
                    + self.suprailiac? as f32
                    + self.midaxillary? as f32,
                1.097,
                0.00046971,
                0.00000056,
                0.00012828,
            ))
        } else if sex == 1 {
            Some(self.jackson_pollock(
                self.chest? as f32
                    + self.abdominal? as f32
                    + self.tigh? as f32
                    + self.tricep? as f32
                    + self.subscapular? as f32
                    + self.suprailiac? as f32
                    + self.midaxillary? as f32,
                1.112,
                0.00043499,
                0.00000055,
                0.00028826,
            ))
        } else {
            None
        }
    }

    fn jackson_pollock(&self, sum: f32, k0: f32, k1: f32, k2: f32, ka: f32) -> f32 {
        let age = 30.; // assume an age of 30
        (495. / (k0 - (k1 * sum) + (k2 * sum * sum) - (ka * age))) - 450.
    }
}

impl Routine {
    pub fn exercises(&self) -> BTreeSet<u32> {
        self.sections
            .iter()
            .flat_map(RoutinePart::exercises)
            .collect::<BTreeSet<_>>()
    }
}

impl RoutinePart {
    fn exercises(&self) -> BTreeSet<u32> {
        let mut result: BTreeSet<u32> = BTreeSet::new();
        match self {
            RoutinePart::RoutineSection { parts, .. } => {
                for p in parts {
                    result.extend(Self::exercises(p));
                }
            }
            RoutinePart::RoutineActivity { exercise_id, .. } => {
                if let Some(id) = exercise_id {
                    result.insert(*id);
                }
            }
        }
        result
    }
}

impl Workout {
    pub fn avg_reps(&self) -> Option<f32> {
        let sets = &self.sets.iter().filter_map(|s| s.reps).collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            Some(sets.iter().sum::<u32>() as f32 / sets.len() as f32)
        }
    }

    pub fn avg_time(&self) -> Option<f32> {
        let sets = &self.sets.iter().filter_map(|s| s.time).collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            Some(sets.iter().sum::<u32>() as f32 / sets.len() as f32)
        }
    }

    pub fn avg_weight(&self) -> Option<f32> {
        let sets = &self
            .sets
            .iter()
            .filter_map(|s| s.weight)
            .collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            Some(sets.iter().sum::<f32>() / sets.len() as f32)
        }
    }

    pub fn avg_rpe(&self) -> Option<f32> {
        let sets = &self.sets.iter().filter_map(|s| s.rpe).collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            Some(sets.iter().sum::<f32>() / sets.len() as f32)
        }
    }

    pub fn volume(&self) -> u32 {
        let sets = &self.sets.iter().filter_map(|s| s.reps).collect::<Vec<_>>();
        sets.iter().sum::<u32>()
    }

    pub fn tut(&self) -> u32 {
        let sets = &self
            .sets
            .iter()
            .map(|s| s.reps.unwrap_or(1) * s.time.unwrap_or(0))
            .collect::<Vec<_>>();
        sets.iter().sum::<u32>()
    }
}

fn calculate_body_weight_stats(mut body_weight: Vec<BodyWeight>) -> Vec<BodyWeight> {
    // centered rolling mean
    let window = 9;
    let length = body_weight.len();
    for i in 0..length {
        if i >= window / 2 && i < length - window / 2 {
            let avg_weight = body_weight[i - window / 2..=i + window / 2]
                .iter()
                .map(|bw| bw.weight)
                .sum::<f32>()
                / window as f32;
            body_weight[i].avg_weight = Some(avg_weight);
        }
    }

    body_weight
}

fn determine_cycles(period: &[Period]) -> Vec<Cycle> {
    if period.is_empty() {
        return vec![];
    }

    let mut result = vec![];
    let mut begin = period[0].date;
    let mut last = begin;

    for p in &period[1..] {
        if p.date - last > Duration::days(3) {
            result.push(Cycle {
                begin,
                length: p.date - begin,
            });
            begin = p.date;
        }
        last = p.date;
    }

    result
}

fn determine_current_cycle(cycles: &[Cycle]) -> Option<CurrentCycle> {
    if cycles.is_empty() {
        return None;
    }

    let today = Local::now().date_naive();
    let cycles = cycles
        .iter()
        .filter(|c| (c.begin >= today - Duration::days(182) && c.begin <= today))
        .collect::<Vec<_>>();
    let stats = calculate_cycle_stats(&cycles);

    if let Some(last_cycle) = cycles.last() {
        let begin = last_cycle.begin + last_cycle.length;
        Some(CurrentCycle {
            begin,
            time_left: stats.length_median - (today - begin),
            time_left_variation: stats.length_variation,
        })
    } else {
        None
    }
}

pub fn calculate_cycle_stats(cycles: &[&Cycle]) -> CycleStats {
    let mut cycle_lengths = cycles.iter().map(|c| c.length).collect::<Vec<_>>();
    cycle_lengths.sort();
    CycleStats {
        length_median: common::quartile(&cycle_lengths, common::Quartile::Q2),
        length_variation: (common::quartile(&cycle_lengths, common::Quartile::Q3)
            - common::quartile(&cycle_lengths, common::Quartile::Q1))
            / 2,
    }
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

    ReadVersion,
    VersionRead(Result<String, String>),

    ReadUsers,
    UsersRead(Result<Vec<User>, String>),
    CreateUser(NewUser),
    UserCreated(Result<User, String>),
    ReplaceUser(User),
    UserReplaced(Result<User, String>),
    DeleteUser(u32),
    UserDeleted(Result<(), String>),

    ReadBodyWeight,
    BodyWeightRead(Result<Vec<BodyWeight>, String>),
    CreateBodyWeight(BodyWeight),
    BodyWeightCreated(Result<BodyWeight, String>),
    ReplaceBodyWeight(BodyWeight),
    BodyWeightReplaced(Result<BodyWeight, String>),
    DeleteBodyWeight(NaiveDate),
    BodyWeightDeleted(Result<(), String>),

    ReadBodyFat,
    BodyFatRead(Result<Vec<BodyFat>, String>),
    CreateBodyFat(BodyFat),
    BodyFatCreated(Result<BodyFat, String>),
    ReplaceBodyFat(BodyFat),
    BodyFatReplaced(Result<BodyFat, String>),
    DeleteBodyFat(NaiveDate),
    BodyFatDeleted(Result<(), String>),

    ReadPeriod,
    PeriodRead(Result<Vec<Period>, String>),
    CreatePeriod(Period),
    PeriodCreated(Result<Period, String>),
    ReplacePeriod(Period),
    PeriodReplaced(Result<Period, String>),
    DeletePeriod(NaiveDate),
    PeriodDeleted(Result<(), String>),

    ReadExercises,
    ExercisesRead(Result<Vec<Exercise>, String>),
    CreateExercise(String),
    ExerciseCreated(Result<Exercise, String>),
    ReplaceExercise(Exercise),
    ExerciseReplaced(Result<Exercise, String>),
    DeleteExercise(u32),
    ExerciseDeleted(Result<(), String>),

    ReadRoutines,
    RoutinesRead(Result<Vec<Routine>, String>),
    CreateRoutine(String),
    RoutineCreated(Result<Routine, String>),
    ModifyRoutine(u32, Option<String>, Option<Vec<RoutinePart>>),
    RoutineModified(Result<Routine, String>),
    DeleteRoutine(u32),
    RoutineDeleted(Result<(), String>),

    ReadWorkouts,
    WorkoutsRead(Result<Vec<Workout>, String>),
    CreateWorkout(u32, NaiveDate, String, Vec<WorkoutSet>),
    WorkoutCreated(Result<Workout, String>),
    ModifyWorkout(u32, Option<String>, Option<Vec<WorkoutSet>>),
    WorkoutModified(Result<Workout, String>),
    DeleteWorkout(u32),
    WorkoutDeleted(Result<(), String>),
}

#[derive(Clone)]
pub enum Event {
    UserCreatedOk,
    UserCreatedErr,
    UserReplacedOk,
    UserReplacedErr,
    UserDeletedOk,
    UserDeletedErr,
    BodyWeightCreatedOk,
    BodyWeightCreatedErr,
    BodyWeightReplacedOk,
    BodyWeightReplacedErr,
    BodyWeightDeletedOk,
    BodyWeightDeletedErr,
    BodyFatCreatedOk,
    BodyFatCreatedErr,
    BodyFatReplacedOk,
    BodyFatReplacedErr,
    BodyFatDeletedOk,
    BodyFatDeletedErr,
    PeriodCreatedOk,
    PeriodCreatedErr,
    PeriodReplacedOk,
    PeriodReplacedErr,
    PeriodDeletedOk,
    PeriodDeletedErr,
    ExerciseCreatedOk,
    ExerciseCreatedErr,
    ExerciseReplacedOk,
    ExerciseReplacedErr,
    ExerciseDeletedOk,
    ExerciseDeletedErr,
    RoutineCreatedOk,
    RoutineCreatedErr,
    RoutineModifiedOk,
    RoutineModifiedErr,
    RoutineDeletedOk,
    RoutineDeletedErr,
    WorkoutsReadOk,
    WorkoutsReadErr,
    WorkoutCreatedOk,
    WorkoutCreatedErr,
    WorkoutModifiedOk,
    WorkoutModifiedErr,
    WorkoutDeletedOk,
    WorkoutDeletedErr,
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
                .send_msg(Msg::ReadVersion)
                .send_msg(Msg::ReadUsers)
                .send_msg(Msg::ReadBodyWeight)
                .send_msg(Msg::ReadBodyFat)
                .send_msg(Msg::ReadPeriod)
                .send_msg(Msg::ReadExercises)
                .send_msg(Msg::ReadRoutines)
                .send_msg(Msg::ReadWorkouts);
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

        Msg::ReadVersion => {
            orders.perform_cmd(async { fetch("api/version", Msg::VersionRead).await });
        }
        Msg::VersionRead(Ok(version)) => {
            model.version = version;
            let frontend_version: Vec<&str> = env!("VALENS_VERSION").split('.').collect();
            let backend_version: Vec<&str> = model.version.split('.').collect();
            if frontend_version[0] != backend_version[0]
                || frontend_version[1] != backend_version[1]
            {
                model
                .errors
                .push(format!("Mismatch between frontend and backend version ({}, {}). This may lead to unexpected errors. Please close and restart the app.", env!("VALENS_VERSION"), model.version));
            }
        }
        Msg::VersionRead(Err(message)) => {
            model
                .errors
                .push("Failed to read version: ".to_owned() + &message);
        }

        Msg::ReadUsers => {
            orders.perform_cmd(async { fetch("api/users", Msg::UsersRead).await });
        }
        Msg::UsersRead(Ok(users)) => {
            model.users = users;
        }
        Msg::UsersRead(Err(message)) => {
            model
                .errors
                .push("Failed to read users: ".to_owned() + &message);
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
            orders.notify(Event::UserCreatedOk).send_msg(Msg::ReadUsers);
        }
        Msg::UserCreated(Err(message)) => {
            orders.notify(Event::UserCreatedErr);
            model
                .errors
                .push("Failed to create user: ".to_owned() + &message);
        }
        Msg::ReplaceUser(user) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new(format!("api/users/{}", user.id))
                        .method(Method::Put)
                        .json(&NewUser {
                            name: user.name,
                            sex: user.sex,
                        })
                        .expect("serialization failed"),
                    Msg::UserReplaced,
                )
                .await
            });
        }
        Msg::UserReplaced(Ok(_)) => {
            orders
                .notify(Event::UserReplacedOk)
                .send_msg(Msg::ReadUsers);
        }
        Msg::UserReplaced(Err(message)) => {
            orders.notify(Event::UserReplacedErr);
            model
                .errors
                .push("Failed to replace user: ".to_owned() + &message);
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
            orders.notify(Event::UserDeletedOk).send_msg(Msg::ReadUsers);
        }
        Msg::UserDeleted(Err(message)) => {
            orders.notify(Event::UserDeletedErr);
            model
                .errors
                .push("Failed to delete user: ".to_owned() + &message);
        }

        Msg::ReadBodyWeight => {
            orders.skip().perform_cmd(async {
                fetch("api/body_weight?format=statistics", Msg::BodyWeightRead).await
            });
        }
        Msg::BodyWeightRead(Ok(body_weight)) => {
            model.body_weight = calculate_body_weight_stats(body_weight);
            model.body_weight.sort_by(|a, b| a.date.cmp(&b.date));
        }
        Msg::BodyWeightRead(Err(message)) => {
            model
                .errors
                .push("Failed to read body weight: ".to_owned() + &message);
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
                .notify(Event::BodyWeightCreatedOk)
                .send_msg(Msg::ReadBodyWeight);
        }
        Msg::BodyWeightCreated(Err(message)) => {
            orders.notify(Event::BodyWeightCreatedErr);
            model
                .errors
                .push("Failed to create body weight: ".to_owned() + &message);
        }
        Msg::ReplaceBodyWeight(body_weight) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new(format!("api/body_weight/{}", body_weight.date))
                        .method(Method::Put)
                        .json(&json!({ "weight": body_weight.weight }))
                        .expect("serialization failed"),
                    Msg::BodyWeightReplaced,
                )
                .await
            });
        }
        Msg::BodyWeightReplaced(Ok(_)) => {
            orders
                .notify(Event::BodyWeightReplacedOk)
                .send_msg(Msg::ReadBodyWeight);
        }
        Msg::BodyWeightReplaced(Err(message)) => {
            orders.notify(Event::BodyWeightReplacedErr);
            model
                .errors
                .push("Failed to replace body weight: ".to_owned() + &message);
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
                .notify(Event::BodyWeightDeletedOk)
                .send_msg(Msg::ReadBodyWeight);
        }
        Msg::BodyWeightDeleted(Err(message)) => {
            orders.notify(Event::BodyWeightDeletedErr);
            model
                .errors
                .push("Failed to delete body weight: ".to_owned() + &message);
        }

        Msg::ReadBodyFat => {
            orders.skip().perform_cmd(async {
                fetch("api/body_fat?format=statistics", Msg::BodyFatRead).await
            });
        }
        Msg::BodyFatRead(Ok(body_fat)) => {
            model.body_fat = body_fat;
            model.body_fat.sort_by(|a, b| a.date.cmp(&b.date));
        }
        Msg::BodyFatRead(Err(message)) => {
            model
                .errors
                .push("Failed to read body fat: ".to_owned() + &message);
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
                .notify(Event::BodyFatCreatedOk)
                .send_msg(Msg::ReadBodyFat);
        }
        Msg::BodyFatCreated(Err(message)) => {
            orders.notify(Event::BodyFatCreatedErr);
            model
                .errors
                .push("Failed to create body fat: ".to_owned() + &message);
        }
        Msg::ReplaceBodyFat(body_fat) => {
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
                    Msg::BodyFatReplaced,
                )
                .await
            });
        }
        Msg::BodyFatReplaced(Ok(_)) => {
            orders
                .notify(Event::BodyFatReplacedOk)
                .send_msg(Msg::ReadBodyFat);
        }
        Msg::BodyFatReplaced(Err(message)) => {
            orders.notify(Event::BodyFatReplacedErr);
            model
                .errors
                .push("Failed to replace body fat: ".to_owned() + &message);
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
                .notify(Event::BodyFatDeletedOk)
                .send_msg(Msg::ReadBodyFat);
        }
        Msg::BodyFatDeleted(Err(message)) => {
            orders.notify(Event::BodyFatDeletedErr);
            model
                .errors
                .push("Failed to delete body fat: ".to_owned() + &message);
        }

        Msg::ReadPeriod => {
            orders
                .skip()
                .perform_cmd(async { fetch("api/period", Msg::PeriodRead).await });
        }
        Msg::PeriodRead(Ok(period)) => {
            model.period = period;
            model.period.sort_by(|a, b| a.date.cmp(&b.date));
            model.cycles = determine_cycles(&model.period);
            model.current_cycle = determine_current_cycle(&model.cycles);
        }
        Msg::PeriodRead(Err(message)) => {
            model
                .errors
                .push("Failed to read period: ".to_owned() + &message);
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
                .notify(Event::PeriodCreatedOk)
                .send_msg(Msg::ReadPeriod);
        }
        Msg::PeriodCreated(Err(message)) => {
            orders.notify(Event::PeriodCreatedErr);
            model
                .errors
                .push("Failed to create period: ".to_owned() + &message);
        }
        Msg::ReplacePeriod(period) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new(format!("api/period/{}", period.date))
                        .method(Method::Put)
                        .json(&json!({ "intensity": period.intensity }))
                        .expect("serialization failed"),
                    Msg::PeriodReplaced,
                )
                .await
            });
        }
        Msg::PeriodReplaced(Ok(_)) => {
            orders
                .notify(Event::PeriodReplacedOk)
                .send_msg(Msg::ReadPeriod);
        }
        Msg::PeriodReplaced(Err(message)) => {
            orders.notify(Event::PeriodReplacedErr);
            model
                .errors
                .push("Failed to replace period: ".to_owned() + &message);
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
                .notify(Event::PeriodDeletedOk)
                .send_msg(Msg::ReadPeriod);
        }
        Msg::PeriodDeleted(Err(message)) => {
            orders.notify(Event::PeriodDeletedErr);
            model
                .errors
                .push("Failed to delete period: ".to_owned() + &message);
        }

        Msg::ReadExercises => {
            orders
                .skip()
                .perform_cmd(async { fetch("api/exercises", Msg::ExercisesRead).await });
        }
        Msg::ExercisesRead(Ok(exercises)) => {
            model.exercises = exercises;
            model.exercises.sort_by(|a, b| a.name.cmp(&b.name));
        }
        Msg::ExercisesRead(Err(message)) => {
            model
                .errors
                .push("Failed to read exercises: ".to_owned() + &message);
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
                .notify(Event::ExerciseCreatedOk)
                .send_msg(Msg::ReadExercises);
        }
        Msg::ExerciseCreated(Err(message)) => {
            orders.notify(Event::ExerciseCreatedErr);
            model
                .errors
                .push("Failed to create exercise: ".to_owned() + &message);
        }
        Msg::ReplaceExercise(exercise) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new(format!("api/exercises/{}", exercise.id))
                        .method(Method::Put)
                        .json(&exercise)
                        .expect("serialization failed"),
                    Msg::ExerciseReplaced,
                )
                .await
            });
        }
        Msg::ExerciseReplaced(Ok(_)) => {
            orders
                .notify(Event::ExerciseReplacedOk)
                .send_msg(Msg::ReadExercises);
        }
        Msg::ExerciseReplaced(Err(message)) => {
            orders.notify(Event::ExerciseReplacedErr);
            model
                .errors
                .push("Failed to replace exercise: ".to_owned() + &message);
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
                .notify(Event::ExerciseDeletedOk)
                .send_msg(Msg::ReadExercises);
        }
        Msg::ExerciseDeleted(Err(message)) => {
            orders.notify(Event::ExerciseDeletedErr);
            model
                .errors
                .push("Failed to delete exercise: ".to_owned() + &message);
        }

        Msg::ReadRoutines => {
            orders
                .skip()
                .perform_cmd(async { fetch("api/routines", Msg::RoutinesRead).await });
        }
        Msg::RoutinesRead(Ok(routines)) => {
            model.routines = routines;
            model.routines.sort_by(|a, b| b.id.cmp(&a.id));
        }
        Msg::RoutinesRead(Err(message)) => {
            model
                .errors
                .push("Failed to read routines: ".to_owned() + &message);
        }
        Msg::CreateRoutine(routine_name) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new("api/routines")
                        .method(Method::Post)
                        .json(&json!({
                            "name": routine_name,
                            "notes": "",
                            "sections": []
                        }))
                        .expect("serialization failed"),
                    Msg::RoutineCreated,
                )
                .await
            });
        }
        Msg::RoutineCreated(Ok(_)) => {
            orders
                .notify(Event::RoutineCreatedOk)
                .send_msg(Msg::ReadRoutines);
        }
        Msg::RoutineCreated(Err(message)) => {
            orders.notify(Event::RoutineCreatedErr);
            model
                .errors
                .push("Failed to create routine: ".to_owned() + &message);
        }
        Msg::ModifyRoutine(id, name, sections) => {
            let mut content = Map::new();
            if let Some(name) = name {
                content.insert("name".into(), json!(name));
            }
            if let Some(sections) = sections {
                content.insert("sections".into(), json!(sections));
            }
            orders.perform_cmd(async move {
                fetch(
                    Request::new(format!("api/routines/{}", id))
                        .method(Method::Patch)
                        .json(&content)
                        .expect("serialization failed"),
                    Msg::RoutineModified,
                )
                .await
            });
        }
        Msg::RoutineModified(Ok(_)) => {
            orders
                .notify(Event::RoutineModifiedOk)
                .send_msg(Msg::ReadRoutines);
        }
        Msg::RoutineModified(Err(message)) => {
            orders.notify(Event::RoutineModifiedErr);
            model
                .errors
                .push("Failed to modify routine: ".to_owned() + &message);
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
                .notify(Event::RoutineDeletedOk)
                .send_msg(Msg::ReadRoutines);
        }
        Msg::RoutineDeleted(Err(message)) => {
            orders.notify(Event::RoutineDeletedErr);
            model
                .errors
                .push("Failed to delete routine: ".to_owned() + &message);
        }

        Msg::ReadWorkouts => {
            orders
                .skip()
                .perform_cmd(async { fetch("api/workouts", Msg::WorkoutsRead).await });
        }
        Msg::WorkoutsRead(Ok(workouts)) => {
            orders.notify(Event::WorkoutsReadOk);
            model.workouts = workouts;
            model.workouts.sort_by(|a, b| a.date.cmp(&b.date));
        }
        Msg::WorkoutsRead(Err(message)) => {
            orders.notify(Event::WorkoutsReadErr);
            model
                .errors
                .push("Failed to read workouts: ".to_owned() + &message);
        }
        Msg::CreateWorkout(routine_id, date, notes, sets) => {
            orders.perform_cmd(async move {
                fetch(
                    Request::new("api/workouts")
                        .method(Method::Post)
                        .json(&json!({
                            "routine_id": routine_id,
                            "date": date,
                            "notes": notes,
                            "sets": sets
                        }))
                        .expect("serialization failed"),
                    Msg::WorkoutCreated,
                )
                .await
            });
        }
        Msg::WorkoutCreated(Ok(_)) => {
            orders
                .notify(Event::WorkoutCreatedOk)
                .send_msg(Msg::ReadWorkouts);
        }
        Msg::WorkoutCreated(Err(message)) => {
            orders.notify(Event::WorkoutCreatedErr);
            model
                .errors
                .push("Failed to create workout: ".to_owned() + &message);
        }
        Msg::ModifyWorkout(id, notes, sets) => {
            let mut content = Map::new();
            if let Some(notes) = notes {
                content.insert("notes".into(), json!(notes));
            }
            if let Some(sets) = sets {
                content.insert("sets".into(), json!(sets));
            }
            orders.perform_cmd(async move {
                fetch(
                    Request::new(format!("api/workouts/{}", id))
                        .method(Method::Patch)
                        .json(&content)
                        .expect("serialization failed"),
                    Msg::WorkoutModified,
                )
                .await
            });
        }
        Msg::WorkoutModified(Ok(_)) => {
            orders
                .notify(Event::WorkoutModifiedOk)
                .send_msg(Msg::ReadWorkouts);
        }
        Msg::WorkoutModified(Err(message)) => {
            orders.notify(Event::WorkoutModifiedErr);
            model
                .errors
                .push("Failed to modify workout: ".to_owned() + &message);
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
                .notify(Event::WorkoutDeletedOk)
                .send_msg(Msg::ReadWorkouts);
        }
        Msg::WorkoutDeleted(Err(message)) => {
            orders.notify(Event::WorkoutDeletedErr);
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

// ------ ------
//     Tests
// ------ ------

#[cfg(test)]
mod tests {
    use super::*;

    fn from_num_days(days: i32) -> NaiveDate {
        NaiveDate::from_num_days_from_ce_opt(days).unwrap()
    }

    #[test]
    fn test_determine_cycles() {
        assert_eq!(determine_cycles(&[]), vec![]);
        assert_eq!(
            determine_cycles(&[
                Period {
                    date: from_num_days(1),
                    intensity: 3,
                },
                Period {
                    date: from_num_days(5),
                    intensity: 4,
                },
                Period {
                    date: from_num_days(8),
                    intensity: 2,
                },
                Period {
                    date: from_num_days(33),
                    intensity: 1,
                }
            ]),
            vec![
                Cycle {
                    begin: from_num_days(1),
                    length: Duration::days(4),
                },
                Cycle {
                    begin: from_num_days(5),
                    length: Duration::days(28),
                }
            ]
        )
    }
}
