use std::{collections::BTreeMap, iter::zip, sync::Arc};

use chrono::{prelude::*, Duration};
use gloo_console::error;
use gloo_storage::Storage as GlooStorage;
use seed::{
    app::{subs, Orders},
    button, div, nodes, p,
    prelude::{ev, El, Ev, Node},
    virtual_dom::{ToClasses, UpdateEl},
    Url, C, IF,
};

use crate::{
    domain, storage,
    ui::{self, common},
};

const STORAGE_KEY_SETTINGS: &str = "settings";
const STORAGE_KEY_ONGOING_TRAINING_SESSION: &str = "ongoing training session";

// ------ ------
//     Init
// ------ ------

#[allow(clippy::needless_pass_by_value)]
pub fn init(url: Url, _orders: &mut impl Orders<Msg>) -> Model {
    let settings = gloo_storage::LocalStorage::get(STORAGE_KEY_SETTINGS).unwrap_or(Settings {
        beep_volume: 80,
        theme: Theme::Light,
        automatic_metronome: false,
        notifications: false,
        show_rpe: true,
        show_tut: true,
    });
    let ongoing_training_session =
        gloo_storage::LocalStorage::get(STORAGE_KEY_ONGOING_TRAINING_SESSION).unwrap_or(None);
    Model {
        store: Arc::new(storage::rest::Storage),
        base_url: url.to_hash_base_url(),
        errors: Vec::new(),
        app_update_available: false,
        session: None,
        version: String::new(),
        users: BTreeMap::new(),
        loading_users: false,
        body_weight: BTreeMap::new(),
        loading_body_weight: false,
        body_fat: BTreeMap::new(),
        loading_body_fat: false,
        period: BTreeMap::new(),
        loading_period: false,
        exercises: BTreeMap::new(),
        loading_exercises: false,
        routines: BTreeMap::new(),
        loading_routines: false,
        training_sessions: BTreeMap::new(),
        loading_training_sessions: false,
        last_refresh: DateTime::from_naive_utc_and_offset(
            NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
            Utc,
        ),
        body_weight_stats: BTreeMap::new(),
        cycles: Vec::new(),
        current_cycle: None,
        training_stats: TrainingStats {
            short_term_load: Vec::new(),
            long_term_load: Vec::new(),
        },
        settings,
        ongoing_training_session,
    }
}

// ------ ------
//     Model
// ------ ------

#[allow(clippy::struct_excessive_bools)]
pub struct Model {
    store: Arc<dyn storage::Storage>,
    pub base_url: Url,
    errors: Vec<String>,
    app_update_available: bool,

    // ------ Data -----
    pub session: Option<storage::Session>,
    pub version: String,
    pub users: BTreeMap<u32, storage::User>,
    pub loading_users: bool,

    // ------ Session-dependent data ------
    pub body_weight: BTreeMap<NaiveDate, storage::BodyWeight>,
    pub loading_body_weight: bool,
    pub body_fat: BTreeMap<NaiveDate, storage::BodyFat>,
    pub loading_body_fat: bool,
    pub period: BTreeMap<NaiveDate, storage::Period>,
    pub loading_period: bool,
    pub exercises: BTreeMap<u32, storage::Exercise>,
    pub loading_exercises: bool,
    pub routines: BTreeMap<u32, storage::Routine>,
    pub loading_routines: bool,
    pub training_sessions: BTreeMap<u32, storage::TrainingSession>,
    pub loading_training_sessions: bool,
    pub last_refresh: DateTime<Utc>,

    // ------ Derived data ------
    pub body_weight_stats: BTreeMap<NaiveDate, BodyWeightStats>,
    pub cycles: Vec<Cycle>,
    pub current_cycle: Option<CurrentCycle>,
    pub training_stats: TrainingStats,

    // ------ Client-side data ------
    pub settings: Settings,
    pub ongoing_training_session: Option<OngoingTrainingSession>,
}

impl Model {
    pub fn exercises(&self, filter: &domain::ExerciseFilter) -> Vec<&storage::Exercise> {
        self.exercises
            .values()
            .filter(|e| {
                filter.muscles.is_empty()
                    || filter
                        .muscles
                        .iter()
                        .all(|m| e.muscle_stimulus().contains_key(&domain::Muscle::id(*m)))
            })
            .collect()
    }

    pub fn routines_sorted_by_last_use(
        &self,
        filter: impl Fn(&storage::Routine) -> bool,
    ) -> Vec<storage::Routine> {
        sort_routines_by_last_use(&self.routines, &self.training_sessions, filter)
    }

    pub fn training_sessions_date_range(&self) -> std::ops::RangeInclusive<NaiveDate> {
        let dates = self.training_sessions.values().map(|t| t.date);
        dates.clone().min().unwrap_or_default()..=dates.max().unwrap_or_default()
    }

    pub fn theme(&self) -> &Theme {
        match self.settings.theme {
            Theme::System => {
                if let Some(window) = web_sys::window() {
                    if let Ok(prefers_dark_scheme) =
                        window.match_media("(prefers-color-scheme: dark)")
                    {
                        if let Some(media_query_list) = prefers_dark_scheme {
                            if media_query_list.matches() {
                                &Theme::Dark
                            } else {
                                &Theme::Light
                            }
                        } else {
                            error!("failed to determine preferred color scheme");
                            &Theme::Light
                        }
                    } else {
                        error!("failed to match media to determine preferred color scheme");
                        &Theme::Light
                    }
                } else {
                    error!("failed to access window to determine preferred color scheme");
                    &Theme::Light
                }
            }
            Theme::Light | Theme::Dark => &self.settings.theme,
        }
    }
}

fn sort_routines_by_last_use(
    routines: &BTreeMap<u32, storage::Routine>,
    training_sessions: &BTreeMap<u32, storage::TrainingSession>,
    filter: impl Fn(&storage::Routine) -> bool,
) -> Vec<storage::Routine> {
    let mut map: BTreeMap<u32, NaiveDate> = BTreeMap::new();
    for (routine_id, _) in routines.iter().filter(|(_, r)| filter(r)) {
        map.insert(
            *routine_id,
            NaiveDate::MIN + Duration::days(i64::from(*routine_id)),
        );
    }
    for training_session in training_sessions.values() {
        if let Some(routine_id) = training_session.routine_id {
            if routines.contains_key(&routine_id)
                && filter(&routines[&routine_id])
                && training_session.date > map[&routine_id]
            {
                map.insert(routine_id, training_session.date);
            }
        }
    }
    let mut list: Vec<_> = map.iter().collect();
    list.sort_by(|a, b| a.1.cmp(b.1).reverse());
    list.iter()
        .map(|(routine_id, _)| routines[routine_id].clone())
        .collect()
}

#[derive(Clone)]
pub struct BodyWeightStats {
    pub date: NaiveDate,
    pub avg_weight: Option<f32>,
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

pub struct TrainingStats {
    pub short_term_load: Vec<(NaiveDate, f32)>,
    pub long_term_load: Vec<(NaiveDate, f32)>,
}

impl TrainingStats {
    pub const LOAD_RATIO_LOW: f32 = 0.8;
    pub const LOAD_RATIO_HIGH: f32 = 1.5;

    pub fn load_ratio(&self) -> Option<f32> {
        let long_term_load = self.long_term_load.last().map_or(0., |(_, l)| *l);
        if long_term_load > 0. {
            let short_term_load = self.short_term_load.last().map_or(0., |(_, l)| *l);
            Some(short_term_load / long_term_load)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.short_term_load.clear();
        self.long_term_load.clear();
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub beep_volume: u8,
    pub theme: Theme,
    pub automatic_metronome: bool,
    pub notifications: bool,
    pub show_rpe: bool,
    pub show_tut: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct OngoingTrainingSession {
    pub training_session_id: u32,
    pub start_time: DateTime<Utc>,
    pub element_idx: usize,
    pub element_start_time: DateTime<Utc>,
    pub timer_state: TimerState,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
pub enum TimerState {
    Unset,
    Active { target_time: DateTime<Utc> },
    Paused { time: i64 },
}

impl OngoingTrainingSession {
    pub fn new(training_session_id: u32) -> OngoingTrainingSession {
        OngoingTrainingSession {
            training_session_id,
            start_time: Utc::now(),
            element_idx: 0,
            element_start_time: Utc::now(),
            timer_state: TimerState::Unset,
        }
    }
}

fn calculate_body_weight_stats(
    body_weight: &BTreeMap<NaiveDate, storage::BodyWeight>,
) -> BTreeMap<NaiveDate, BodyWeightStats> {
    let body_weight = body_weight.values().collect::<Vec<_>>();

    // centered rolling mean
    let window = 9;
    let length = body_weight.len();
    body_weight
        .iter()
        .enumerate()
        .map(|(i, bw)| {
            (
                bw.date,
                BodyWeightStats {
                    date: bw.date,
                    avg_weight: if i >= window / 2 && i < length - window / 2 {
                        #[allow(clippy::cast_precision_loss)]
                        let avg_weight = body_weight[i - window / 2..=i + window / 2]
                            .iter()
                            .map(|bw| bw.weight)
                            .sum::<f32>()
                            / window as f32;
                        Some(avg_weight)
                    } else {
                        None
                    },
                },
            )
        })
        .collect()
}

fn determine_cycles(period: &BTreeMap<NaiveDate, storage::Period>) -> Vec<Cycle> {
    if period.is_empty() {
        return vec![];
    }

    let mut result = vec![];
    let mut begin = period.keys().min().copied().unwrap();
    let mut last = begin;

    let period = period.values().collect::<Vec<_>>();

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
            time_left: stats.length_median - (today - begin + Duration::days(1)),
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

fn calculate_training_stats(training_sessions: &[&storage::TrainingSession]) -> TrainingStats {
    let short_term_load = calculate_weighted_sum_of_load(training_sessions, 7);
    let long_term_load = calculate_average_weighted_sum_of_load(&short_term_load, 28);
    TrainingStats {
        short_term_load,
        long_term_load,
    }
}

fn calculate_weighted_sum_of_load(
    training_sessions: &[&storage::TrainingSession],
    window_size: usize,
) -> Vec<(NaiveDate, f32)> {
    let mut result: BTreeMap<NaiveDate, f32> = BTreeMap::new();

    let today = Local::now().date_naive();
    let mut day = training_sessions.first().map_or(today, |t| t.date);
    while day <= today {
        result.insert(day, 0.0);
        day += Duration::days(1);
    }

    for t in training_sessions {
        #[allow(clippy::cast_precision_loss)]
        result
            .entry(t.date)
            .and_modify(|e| *e += t.load() as f32)
            .or_insert(t.load() as f32);
    }

    #[allow(clippy::cast_precision_loss)]
    let weighting: Vec<f32> = (0..window_size)
        .map(|i| 1. - 1. / window_size as f32 * i as f32)
        .collect();
    let mut window: Vec<f32> = (0..window_size).map(|_| 0.).collect();

    result
        .into_iter()
        .map(|(date, load)| {
            window.rotate_right(1);
            window[0] = load;
            (
                date,
                zip(&window, &weighting)
                    .map(|(load, weight)| load * weight)
                    .sum(),
            )
        })
        .collect()
}

fn calculate_average_weighted_sum_of_load(
    weighted_sum_of_load: &[(NaiveDate, f32)],
    window_size: usize,
) -> Vec<(NaiveDate, f32)> {
    #[allow(clippy::cast_precision_loss)]
    weighted_sum_of_load
        .windows(window_size)
        .map(|window| {
            (
                window.last().unwrap().0,
                window.iter().map(|(_, l)| l).sum::<f32>() / window_size as f32,
            )
        })
        .collect::<Vec<_>>()
}

// ------ ------
//    Update
// ------ ------

#[derive(Clone)]
pub enum Msg {
    RemoveError,
    ClearErrors,

    UpdateApp,
    CancelAppUpdate,

    Refresh,
    ClearSessionDependentData,

    RequestSession(u32),
    SessionReceived(Result<storage::Session, String>),
    InitializeSession,
    SessionInitialized(Result<storage::Session, String>),

    DeleteSession,
    SessionDeleted(Result<(), String>),

    ReadVersion,
    VersionRead(Result<String, String>),

    ReadUsers,
    UsersRead(Result<Vec<storage::User>, String>),
    CreateUser(storage::NewUser),
    UserCreated(Result<storage::User, String>),
    ReplaceUser(storage::User),
    UserReplaced(Result<storage::User, String>),
    DeleteUser(u32),
    UserDeleted(Result<u32, String>),

    ReadBodyWeight,
    BodyWeightRead(Result<Vec<storage::BodyWeight>, String>),
    CreateBodyWeight(storage::BodyWeight),
    BodyWeightCreated(Result<storage::BodyWeight, String>),
    ReplaceBodyWeight(storage::BodyWeight),
    BodyWeightReplaced(Result<storage::BodyWeight, String>),
    DeleteBodyWeight(NaiveDate),
    BodyWeightDeleted(Result<NaiveDate, String>),

    ReadBodyFat,
    BodyFatRead(Result<Vec<storage::BodyFat>, String>),
    CreateBodyFat(storage::BodyFat),
    BodyFatCreated(Result<storage::BodyFat, String>),
    ReplaceBodyFat(storage::BodyFat),
    BodyFatReplaced(Result<storage::BodyFat, String>),
    DeleteBodyFat(NaiveDate),
    BodyFatDeleted(Result<NaiveDate, String>),

    ReadPeriod,
    PeriodRead(Result<Vec<storage::Period>, String>),
    CreatePeriod(storage::Period),
    PeriodCreated(Result<storage::Period, String>),
    ReplacePeriod(storage::Period),
    PeriodReplaced(Result<storage::Period, String>),
    DeletePeriod(NaiveDate),
    PeriodDeleted(Result<NaiveDate, String>),

    ReadExercises,
    ExercisesRead(Result<Vec<storage::Exercise>, String>),
    CreateExercise(String, Vec<storage::ExerciseMuscle>),
    ExerciseCreated(Result<storage::Exercise, String>),
    ReplaceExercise(storage::Exercise),
    ExerciseReplaced(Result<storage::Exercise, String>),
    DeleteExercise(u32),
    ExerciseDeleted(Result<u32, String>),

    ReadRoutines,
    RoutinesRead(Result<Vec<storage::Routine>, String>),
    CreateRoutine(String, u32),
    RoutineCreated(Result<storage::Routine, String>),
    ModifyRoutine(
        u32,
        Option<String>,
        Option<bool>,
        Option<Vec<storage::RoutinePart>>,
    ),
    RoutineModified(Result<storage::Routine, String>),
    DeleteRoutine(u32),
    RoutineDeleted(Result<u32, String>),

    ReadTrainingSessions,
    TrainingSessionsRead(Result<Vec<storage::TrainingSession>, String>),
    CreateTrainingSession(
        Option<u32>,
        NaiveDate,
        String,
        Vec<storage::TrainingSessionElement>,
    ),
    TrainingSessionCreated(Result<storage::TrainingSession, String>),
    ModifyTrainingSession(
        u32,
        Option<String>,
        Option<Vec<storage::TrainingSessionElement>>,
    ),
    TrainingSessionModified(Result<storage::TrainingSession, String>),
    DeleteTrainingSession(u32),
    TrainingSessionDeleted(Result<u32, String>),

    SetBeepVolume(u8),
    SetTheme(Theme),
    SetAutomaticMetronome(bool),
    SetNotifications(bool),
    SetShowRPE(bool),
    SetShowTUT(bool),

    StartTrainingSession(u32),
    UpdateTrainingSession(usize, TimerState),
    EndTrainingSession,
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
    TrainingSessionCreatedOk,
    TrainingSessionCreatedErr,
    TrainingSessionModifiedOk,
    TrainingSessionModifiedErr,
    TrainingSessionDeletedOk,
    TrainingSessionDeletedErr,
    DataChanged,
    BeepVolumeChanged,
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::RemoveError => {
            model.errors.pop();
        }
        Msg::ClearErrors => {
            model.errors.clear();
        }

        Msg::UpdateApp => {
            match common::post_message_to_service_worker(&common::ServiceWorkerMessage::UpdateCache)
            {
                Ok(()) => Url::reload(),
                Err(err) => {
                    model.errors.push(format!("Update failed: {err}"));
                }
            }
        }
        Msg::CancelAppUpdate => {
            model.app_update_available = false;
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
                .send_msg(Msg::ReadTrainingSessions);
            model.last_refresh = Utc::now();
        }
        Msg::ClearSessionDependentData => {
            model.body_weight.clear();
            model.body_fat.clear();
            model.period.clear();
            model.exercises.clear();
            model.routines.clear();
            model.training_sessions.clear();
            model.body_weight_stats.clear();
            model.cycles.clear();
            model.current_cycle = None;
            model.training_stats.clear();
        }

        Msg::RequestSession(user_id) => {
            let store = model.store.clone();
            orders.skip().perform_cmd(async move {
                Msg::SessionReceived(store.request_session(user_id).await)
            });
        }
        Msg::SessionReceived(Ok(new_session)) => {
            model.session = Some(new_session);
            orders
                .send_msg(Msg::Refresh)
                .request_url(ui::Urls::new(model.base_url.clone().set_hash_path([""; 0])).home());
        }
        Msg::SessionReceived(Err(message)) => {
            model.session = None;
            model
                .errors
                .push("Failed to request session: ".to_owned() + &message);
        }
        Msg::InitializeSession => {
            let store = model.store.clone();
            orders.perform_cmd(
                async move { Msg::SessionInitialized(store.initialize_session().await) },
            );
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
            let store = model.store.clone();
            orders
                .skip()
                .send_msg(Msg::ClearSessionDependentData)
                .perform_cmd(async move { Msg::SessionDeleted(store.delete_session().await) });
        }
        Msg::SessionDeleted(Ok(())) => {
            model.session = None;
            orders.request_url(ui::Urls::new(&model.base_url).login());
        }
        Msg::SessionDeleted(Err(message)) => {
            model
                .errors
                .push("Failed to switch users: ".to_owned() + &message);
        }

        Msg::ReadVersion => {
            let store = model.store.clone();
            orders.perform_cmd(async move { Msg::VersionRead(store.read_version().await) });
        }
        Msg::VersionRead(Ok(version)) => {
            model.version = version;
            let frontend_version: Vec<&str> = env!("VALENS_VERSION").split('.').collect();
            let backend_version: Vec<&str> = model.version.split('.').collect();
            if frontend_version[0] != backend_version[0]
                || frontend_version[1] != backend_version[1]
                || frontend_version[2] != backend_version[2]
            {
                model.app_update_available = true;
            }
        }
        Msg::VersionRead(Err(message)) => {
            model
                .errors
                .push("Failed to read version: ".to_owned() + &message);
        }

        Msg::ReadUsers => {
            model.loading_users = true;
            let store = model.store.clone();
            orders.perform_cmd(async move { Msg::UsersRead(store.read_users().await) });
        }
        Msg::UsersRead(Ok(users)) => {
            let users = users.into_iter().map(|e| (e.id, e)).collect();
            if model.users != users {
                model.users = users;
                orders.notify(Event::DataChanged);
            }
            model.loading_users = false;
        }
        Msg::UsersRead(Err(message)) => {
            model
                .errors
                .push("Failed to read users: ".to_owned() + &message);
            model.loading_users = false;
        }
        Msg::CreateUser(user) => {
            let store = model.store.clone();
            orders.perform_cmd(async move { Msg::UserCreated(store.create_user(user).await) });
        }
        Msg::UserCreated(Ok(user)) => {
            model.users.insert(user.id, user);
            orders.notify(Event::UserCreatedOk);
        }
        Msg::UserCreated(Err(message)) => {
            orders.notify(Event::UserCreatedErr);
            model
                .errors
                .push("Failed to create user: ".to_owned() + &message);
        }
        Msg::ReplaceUser(user) => {
            let store = model.store.clone();
            orders.perform_cmd(async move { Msg::UserReplaced(store.replace_user(user).await) });
        }
        Msg::UserReplaced(Ok(user)) => {
            model.users.insert(user.id, user);
            orders.notify(Event::UserReplacedOk);
        }
        Msg::UserReplaced(Err(message)) => {
            orders.notify(Event::UserReplacedErr);
            model
                .errors
                .push("Failed to replace user: ".to_owned() + &message);
        }
        Msg::DeleteUser(id) => {
            let store = model.store.clone();
            orders.perform_cmd(async move { Msg::UserDeleted(store.delete_user(id).await) });
        }
        Msg::UserDeleted(Ok(id)) => {
            model.users.remove(&id);
            orders.notify(Event::UserDeletedOk);
        }
        Msg::UserDeleted(Err(message)) => {
            orders.notify(Event::UserDeletedErr);
            model
                .errors
                .push("Failed to delete user: ".to_owned() + &message);
        }

        Msg::ReadBodyWeight => {
            model.loading_body_weight = true;
            let store = model.store.clone();
            orders
                .skip()
                .perform_cmd(async move { Msg::BodyWeightRead(store.read_body_weight().await) });
        }
        Msg::BodyWeightRead(Ok(body_weight)) => {
            let body_weight = body_weight.into_iter().map(|e| (e.date, e)).collect();
            if model.body_weight != body_weight {
                model.body_weight = body_weight;
                model.body_weight_stats = calculate_body_weight_stats(&model.body_weight);
                orders.notify(Event::DataChanged);
            }
            model.loading_body_weight = false;
        }
        Msg::BodyWeightRead(Err(message)) => {
            model
                .errors
                .push("Failed to read body weight: ".to_owned() + &message);
            model.loading_body_weight = false;
        }
        Msg::CreateBodyWeight(body_weight) => {
            let store = model.store.clone();
            orders.perform_cmd(async move {
                Msg::BodyWeightCreated(store.create_body_weight(body_weight).await)
            });
        }
        Msg::BodyWeightCreated(Ok(body_weight)) => {
            model.body_weight.insert(body_weight.date, body_weight);
            model.body_weight_stats = calculate_body_weight_stats(&model.body_weight);
            orders.notify(Event::BodyWeightCreatedOk);
        }
        Msg::BodyWeightCreated(Err(message)) => {
            orders.notify(Event::BodyWeightCreatedErr);
            model
                .errors
                .push("Failed to create body weight: ".to_owned() + &message);
        }
        Msg::ReplaceBodyWeight(body_weight) => {
            let store = model.store.clone();
            orders.perform_cmd(async move {
                Msg::BodyWeightReplaced(store.replace_body_weight(body_weight).await)
            });
        }
        Msg::BodyWeightReplaced(Ok(body_weight)) => {
            model.body_weight.insert(body_weight.date, body_weight);
            model.body_weight_stats = calculate_body_weight_stats(&model.body_weight);
            orders.notify(Event::BodyWeightReplacedOk);
        }
        Msg::BodyWeightReplaced(Err(message)) => {
            orders.notify(Event::BodyWeightReplacedErr);
            model
                .errors
                .push("Failed to replace body weight: ".to_owned() + &message);
        }
        Msg::DeleteBodyWeight(date) => {
            let store = model.store.clone();
            orders.perform_cmd(async move {
                Msg::BodyWeightDeleted(store.delete_body_weight(date).await)
            });
        }
        Msg::BodyWeightDeleted(Ok(date)) => {
            model.body_weight.remove(&date);
            model.body_weight_stats = calculate_body_weight_stats(&model.body_weight);
            orders.notify(Event::BodyWeightDeletedOk);
        }
        Msg::BodyWeightDeleted(Err(message)) => {
            orders.notify(Event::BodyWeightDeletedErr);
            model
                .errors
                .push("Failed to delete body weight: ".to_owned() + &message);
        }

        Msg::ReadBodyFat => {
            model.loading_body_fat = true;
            let store = model.store.clone();
            orders
                .skip()
                .perform_cmd(async move { Msg::BodyFatRead(store.read_body_fat().await) });
        }
        Msg::BodyFatRead(Ok(body_fat)) => {
            let body_fat = body_fat.into_iter().map(|e| (e.date, e)).collect();
            if model.body_fat != body_fat {
                model.body_fat = body_fat;
                orders.notify(Event::DataChanged);
            }
            model.loading_body_fat = false;
        }
        Msg::BodyFatRead(Err(message)) => {
            model
                .errors
                .push("Failed to read body fat: ".to_owned() + &message);
            model.loading_body_fat = false;
        }
        Msg::CreateBodyFat(body_fat) => {
            let store = model.store.clone();
            orders.perform_cmd(async move {
                Msg::BodyFatCreated(store.create_body_fat(body_fat).await)
            });
        }
        Msg::BodyFatCreated(Ok(body_fat)) => {
            model.body_fat.insert(body_fat.date, body_fat);
            orders.notify(Event::BodyFatCreatedOk);
        }
        Msg::BodyFatCreated(Err(message)) => {
            orders.notify(Event::BodyFatCreatedErr);
            model
                .errors
                .push("Failed to create body fat: ".to_owned() + &message);
        }
        Msg::ReplaceBodyFat(body_fat) => {
            let store = model.store.clone();
            orders.perform_cmd(async move {
                Msg::BodyFatReplaced(store.replace_body_fat(body_fat).await)
            });
        }
        Msg::BodyFatReplaced(Ok(body_fat)) => {
            model.body_fat.insert(body_fat.date, body_fat);
            orders.notify(Event::BodyFatReplacedOk);
        }
        Msg::BodyFatReplaced(Err(message)) => {
            orders.notify(Event::BodyFatReplacedErr);
            model
                .errors
                .push("Failed to replace body fat: ".to_owned() + &message);
        }
        Msg::DeleteBodyFat(date) => {
            let store = model.store.clone();
            orders
                .perform_cmd(async move { Msg::BodyFatDeleted(store.delete_body_fat(date).await) });
        }
        Msg::BodyFatDeleted(Ok(date)) => {
            model.body_fat.remove(&date);
            orders.notify(Event::BodyFatDeletedOk);
        }
        Msg::BodyFatDeleted(Err(message)) => {
            orders.notify(Event::BodyFatDeletedErr);
            model
                .errors
                .push("Failed to delete body fat: ".to_owned() + &message);
        }

        Msg::ReadPeriod => {
            model.loading_period = true;
            let store = model.store.clone();
            orders
                .skip()
                .perform_cmd(async move { Msg::PeriodRead(store.read_period().await) });
        }
        Msg::PeriodRead(Ok(period)) => {
            let period = period.into_iter().map(|e| (e.date, e)).collect();
            if model.period != period {
                model.period = period;
                model.cycles = determine_cycles(&model.period);
                model.current_cycle = determine_current_cycle(&model.cycles);
                orders.notify(Event::DataChanged);
            }
            model.loading_period = false;
        }
        Msg::PeriodRead(Err(message)) => {
            model
                .errors
                .push("Failed to read period: ".to_owned() + &message);
            model.loading_period = false;
        }
        Msg::CreatePeriod(period) => {
            let store = model.store.clone();
            orders
                .perform_cmd(async move { Msg::PeriodCreated(store.create_period(period).await) });
        }
        Msg::PeriodCreated(Ok(period)) => {
            model.period.insert(period.date, period);
            model.cycles = determine_cycles(&model.period);
            model.current_cycle = determine_current_cycle(&model.cycles);
            orders.notify(Event::PeriodCreatedOk);
        }
        Msg::PeriodCreated(Err(message)) => {
            orders.notify(Event::PeriodCreatedErr);
            model
                .errors
                .push("Failed to create period: ".to_owned() + &message);
        }
        Msg::ReplacePeriod(period) => {
            let store = model.store.clone();
            orders.perform_cmd(
                async move { Msg::PeriodReplaced(store.replace_period(period).await) },
            );
        }
        Msg::PeriodReplaced(Ok(period)) => {
            model.period.insert(period.date, period);
            model.cycles = determine_cycles(&model.period);
            model.current_cycle = determine_current_cycle(&model.cycles);
            orders.notify(Event::PeriodReplacedOk);
        }
        Msg::PeriodReplaced(Err(message)) => {
            orders.notify(Event::PeriodReplacedErr);
            model
                .errors
                .push("Failed to replace period: ".to_owned() + &message);
        }
        Msg::DeletePeriod(date) => {
            let store = model.store.clone();
            orders.perform_cmd(async move { Msg::PeriodDeleted(store.delete_period(date).await) });
        }
        Msg::PeriodDeleted(Ok(date)) => {
            model.period.remove(&date);
            model.cycles = determine_cycles(&model.period);
            model.current_cycle = determine_current_cycle(&model.cycles);
            orders.notify(Event::PeriodDeletedOk);
        }
        Msg::PeriodDeleted(Err(message)) => {
            orders.notify(Event::PeriodDeletedErr);
            model
                .errors
                .push("Failed to delete period: ".to_owned() + &message);
        }

        Msg::ReadExercises => {
            model.loading_exercises = true;
            let store = model.store.clone();
            orders
                .skip()
                .perform_cmd(async move { Msg::ExercisesRead(store.read_exercises().await) });
        }
        Msg::ExercisesRead(Ok(exercises)) => {
            let exercises = exercises.into_iter().map(|e| (e.id, e)).collect();
            if model.exercises != exercises {
                model.exercises = exercises;
                orders.notify(Event::DataChanged);
            }
            model.loading_exercises = false;
        }
        Msg::ExercisesRead(Err(message)) => {
            model
                .errors
                .push("Failed to read exercises: ".to_owned() + &message);
            model.loading_exercises = false;
        }
        Msg::CreateExercise(name, muscles) => {
            let store = model.store.clone();
            orders.perform_cmd(async move {
                Msg::ExerciseCreated(store.create_exercise(name, muscles).await)
            });
        }
        Msg::ExerciseCreated(Ok(exercise)) => {
            model.exercises.insert(exercise.id, exercise);
            orders.notify(Event::ExerciseCreatedOk);
        }
        Msg::ExerciseCreated(Err(message)) => {
            orders.notify(Event::ExerciseCreatedErr);
            model
                .errors
                .push("Failed to create exercise: ".to_owned() + &message);
        }
        Msg::ReplaceExercise(exercise) => {
            let store = model.store.clone();
            orders.perform_cmd(async move {
                Msg::ExerciseReplaced(store.replace_exercise(exercise).await)
            });
        }
        Msg::ExerciseReplaced(Ok(exercise)) => {
            model.exercises.insert(exercise.id, exercise);
            model.training_stats =
                calculate_training_stats(&model.training_sessions.values().collect::<Vec<_>>());
            orders.notify(Event::ExerciseReplacedOk);
        }
        Msg::ExerciseReplaced(Err(message)) => {
            orders.notify(Event::ExerciseReplacedErr);
            model
                .errors
                .push("Failed to replace exercise: ".to_owned() + &message);
        }
        Msg::DeleteExercise(id) => {
            let store = model.store.clone();
            orders
                .perform_cmd(async move { Msg::ExerciseDeleted(store.delete_exercise(id).await) });
        }
        Msg::ExerciseDeleted(Ok(id)) => {
            model.exercises.remove(&id);
            orders.notify(Event::ExerciseDeletedOk);
        }
        Msg::ExerciseDeleted(Err(message)) => {
            orders.notify(Event::ExerciseDeletedErr);
            model
                .errors
                .push("Failed to delete exercise: ".to_owned() + &message);
        }

        Msg::ReadRoutines => {
            model.loading_routines = true;
            let store = model.store.clone();
            orders
                .skip()
                .perform_cmd(async move { Msg::RoutinesRead(store.read_routines().await) });
        }
        Msg::RoutinesRead(Ok(routines)) => {
            let routines = routines.into_iter().map(|r| (r.id, r)).collect();
            if model.routines != routines {
                model.routines = routines;
                orders.notify(Event::DataChanged);
            }
            model.loading_routines = false;
        }
        Msg::RoutinesRead(Err(message)) => {
            model
                .errors
                .push("Failed to read routines: ".to_owned() + &message);
            model.loading_routines = false;
        }
        Msg::CreateRoutine(name, template_routine_id) => {
            let sections = if model.routines.contains_key(&template_routine_id) {
                model.routines[&template_routine_id].sections.clone()
            } else {
                vec![]
            };
            let store = model.store.clone();
            orders.perform_cmd(async move {
                Msg::RoutineCreated(store.create_routine(name, sections).await)
            });
        }
        Msg::RoutineCreated(Ok(routine)) => {
            model.routines.insert(routine.id, routine);
            orders.notify(Event::RoutineCreatedOk);
        }
        Msg::RoutineCreated(Err(message)) => {
            orders.notify(Event::RoutineCreatedErr);
            model
                .errors
                .push("Failed to create routine: ".to_owned() + &message);
        }
        Msg::ModifyRoutine(id, name, archived, sections) => {
            let store = model.store.clone();
            orders.perform_cmd(async move {
                Msg::RoutineModified(store.modify_routine(id, name, archived, sections).await)
            });
        }
        Msg::RoutineModified(Ok(routine)) => {
            model.routines.insert(routine.id, routine);
            orders.notify(Event::RoutineModifiedOk);
        }
        Msg::RoutineModified(Err(message)) => {
            orders.notify(Event::RoutineModifiedErr);
            model
                .errors
                .push("Failed to modify routine: ".to_owned() + &message);
        }
        Msg::DeleteRoutine(id) => {
            let store = model.store.clone();
            orders.perform_cmd(async move { Msg::RoutineDeleted(store.delete_routine(id).await) });
        }
        Msg::RoutineDeleted(Ok(id)) => {
            model.routines.remove(&id);
            orders.notify(Event::RoutineDeletedOk);
        }
        Msg::RoutineDeleted(Err(message)) => {
            orders.notify(Event::RoutineDeletedErr);
            model
                .errors
                .push("Failed to delete routine: ".to_owned() + &message);
        }

        Msg::ReadTrainingSessions => {
            model.loading_training_sessions = true;
            let store = model.store.clone();
            orders.skip().perform_cmd(async move {
                Msg::TrainingSessionsRead(store.read_training_sessions().await)
            });
        }
        Msg::TrainingSessionsRead(Ok(training_sessions)) => {
            let training_sessions = training_sessions.into_iter().map(|t| (t.id, t)).collect();
            if model.training_sessions != training_sessions {
                model.training_sessions = training_sessions;
                model.training_stats =
                    calculate_training_stats(&model.training_sessions.values().collect::<Vec<_>>());
                orders.notify(Event::DataChanged);
            }
            model.loading_training_sessions = false;
        }
        Msg::TrainingSessionsRead(Err(message)) => {
            model
                .errors
                .push("Failed to read training sessions: ".to_owned() + &message);
            model.loading_training_sessions = false;
        }
        Msg::CreateTrainingSession(routine_id, date, notes, elements) => {
            let store = model.store.clone();
            orders.perform_cmd(async move {
                Msg::TrainingSessionCreated(
                    store
                        .create_training_session(routine_id, date, notes, elements)
                        .await,
                )
            });
        }
        Msg::TrainingSessionCreated(Ok(training_session)) => {
            model
                .training_sessions
                .insert(training_session.id, training_session);
            model.training_stats =
                calculate_training_stats(&model.training_sessions.values().collect::<Vec<_>>());
            orders.notify(Event::TrainingSessionCreatedOk);
        }
        Msg::TrainingSessionCreated(Err(message)) => {
            orders.notify(Event::TrainingSessionCreatedErr);
            model
                .errors
                .push("Failed to create training session: ".to_owned() + &message);
        }
        Msg::ModifyTrainingSession(id, notes, elements) => {
            let store = model.store.clone();
            orders.perform_cmd(async move {
                Msg::TrainingSessionModified(
                    store.modify_training_session(id, notes, elements).await,
                )
            });
        }
        Msg::TrainingSessionModified(Ok(training_session)) => {
            model
                .training_sessions
                .insert(training_session.id, training_session);
            model.training_stats =
                calculate_training_stats(&model.training_sessions.values().collect::<Vec<_>>());
            orders.notify(Event::TrainingSessionModifiedOk);
        }
        Msg::TrainingSessionModified(Err(message)) => {
            orders.notify(Event::TrainingSessionModifiedErr);
            model
                .errors
                .push("Failed to modify training session: ".to_owned() + &message);
        }
        Msg::DeleteTrainingSession(id) => {
            let store = model.store.clone();
            orders.perform_cmd(async move {
                Msg::TrainingSessionDeleted(store.delete_training_session(id).await)
            });
        }
        Msg::TrainingSessionDeleted(Ok(id)) => {
            model.training_sessions.remove(&id);
            model.training_stats =
                calculate_training_stats(&model.training_sessions.values().collect::<Vec<_>>());
            orders.notify(Event::TrainingSessionDeletedOk);
        }
        Msg::TrainingSessionDeleted(Err(message)) => {
            orders.notify(Event::TrainingSessionDeletedErr);
            model
                .errors
                .push("Failed to delete training session: ".to_owned() + &message);
        }

        Msg::SetBeepVolume(value) => {
            model.settings.beep_volume = value;
            local_storage_set(STORAGE_KEY_SETTINGS, &model.settings, &mut model.errors);
            orders.notify(Event::BeepVolumeChanged);
        }
        Msg::SetTheme(theme) => {
            model.settings.theme = theme;
            local_storage_set(STORAGE_KEY_SETTINGS, &model.settings, &mut model.errors);
        }
        Msg::SetAutomaticMetronome(value) => {
            model.settings.automatic_metronome = value;
            local_storage_set(STORAGE_KEY_SETTINGS, &model.settings, &mut model.errors);
        }
        Msg::SetNotifications(value) => {
            model.settings.notifications = value;
            local_storage_set(STORAGE_KEY_SETTINGS, &model.settings, &mut model.errors);
        }
        Msg::SetShowRPE(value) => {
            model.settings.show_rpe = value;
            local_storage_set(STORAGE_KEY_SETTINGS, &model.settings, &mut model.errors);
        }
        Msg::SetShowTUT(value) => {
            model.settings.show_tut = value;
            local_storage_set(STORAGE_KEY_SETTINGS, &model.settings, &mut model.errors);
        }

        Msg::StartTrainingSession(training_session_id) => {
            model.ongoing_training_session = Some(OngoingTrainingSession::new(training_session_id));
            local_storage_set(
                STORAGE_KEY_ONGOING_TRAINING_SESSION,
                &model.ongoing_training_session,
                &mut model.errors,
            );
        }
        Msg::UpdateTrainingSession(section_idx, timer_state) => {
            if let Some(ongoing_training_session) = &mut model.ongoing_training_session {
                ongoing_training_session.element_idx = section_idx;
                ongoing_training_session.element_start_time = Utc::now();
                ongoing_training_session.timer_state = timer_state;
            }
            local_storage_set(
                STORAGE_KEY_ONGOING_TRAINING_SESSION,
                &model.ongoing_training_session,
                &mut model.errors,
            );
        }
        Msg::EndTrainingSession => {
            model.ongoing_training_session = None;
            local_storage_set(
                STORAGE_KEY_ONGOING_TRAINING_SESSION,
                &model.ongoing_training_session,
                &mut model.errors,
            );
        }
    }
}

fn local_storage_set<T: serde::Serialize>(key: &str, value: &T, errors: &mut Vec<String>) {
    if let Err(message) = gloo_storage::LocalStorage::set(key, value) {
        errors.push(format!("Failed to store {key} in local storage: {message}"));
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model) -> Vec<Node<Msg>> {
    nodes![
        common::view_error_dialog(&model.errors, &ev(Ev::Click, |_| Msg::RemoveError)),
        view_app_update_dialog(model),
    ]
}

fn view_app_update_dialog(model: &Model) -> Option<Node<Msg>> {
    IF![model.app_update_available => common::view_dialog(
        "info",
        "Update",
        nodes![
            div![
                C!["block"],
                p!["An app update is available."],
                p![C!["my-3"], common::view_versions(&model.version)],
                p!["Update now to prevent unexpected errors due to incompatibilities with the server."]
            ],
            div![
                C!["field"],
                C!["is-grouped"],
                C!["is-grouped-centered"],
                div![
                    C!["control"],
                    button![C!["button"], C!["is-info"], &ev(Ev::Click, |_| Msg::UpdateApp), "Update"]
                ],
            ],
        ],
        &ev(Ev::Click, |_| Msg::CancelAppUpdate),
    )]
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
        assert_eq!(determine_cycles(&BTreeMap::new()), vec![]);
        assert_eq!(
            determine_cycles(&BTreeMap::from(
                [
                    storage::Period {
                        date: from_num_days(1),
                        intensity: 3,
                    },
                    storage::Period {
                        date: from_num_days(5),
                        intensity: 4,
                    },
                    storage::Period {
                        date: from_num_days(8),
                        intensity: 2,
                    },
                    storage::Period {
                        date: from_num_days(33),
                        intensity: 1,
                    }
                ]
                .map(|p| (p.date, p))
            )),
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
        );
    }

    #[test]
    fn test_sort_routines_by_last_use() {
        let routines = BTreeMap::from([
            (1, routine(1)),
            (2, routine(2)),
            (3, routine(3)),
            (4, routine(4)),
        ]);
        let training_sessions = BTreeMap::from([
            (
                1,
                training_session(1, Some(3), NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            ),
            (
                2,
                training_session(2, Some(2), NaiveDate::from_ymd_opt(2020, 3, 3).unwrap()),
            ),
            (
                3,
                training_session(3, Some(3), NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
            ),
        ]);
        assert_eq!(
            sort_routines_by_last_use(&routines, &training_sessions, |_| true),
            vec![routine(2), routine(3), routine(4), routine(1)]
        );
    }

    #[test]
    fn test_sort_routines_by_last_use_empty() {
        let routines = BTreeMap::new();
        let training_sessions = BTreeMap::new();
        assert_eq!(
            sort_routines_by_last_use(&routines, &training_sessions, |_| true),
            vec![]
        );
    }

    #[test]
    fn test_sort_routines_by_last_use_missing_routines() {
        let routines = BTreeMap::from([(1, routine(1)), (2, routine(2))]);
        let training_sessions = BTreeMap::from([
            (
                1,
                training_session(1, Some(3), NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            ),
            (
                2,
                training_session(2, Some(2), NaiveDate::from_ymd_opt(2020, 3, 3).unwrap()),
            ),
            (
                3,
                training_session(3, Some(3), NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
            ),
        ]);
        assert_eq!(
            sort_routines_by_last_use(&routines, &training_sessions, |_| true),
            vec![routine(2), routine(1)]
        );
    }

    #[test]
    fn test_sort_routines_by_last_use_filter() {
        let routines = BTreeMap::from([
            (1, routine(1)),
            (2, routine(2)),
            (3, routine(3)),
            (4, routine(4)),
        ]);
        let training_sessions = BTreeMap::from([
            (
                1,
                training_session(1, Some(3), NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            ),
            (
                2,
                training_session(2, Some(2), NaiveDate::from_ymd_opt(2020, 3, 3).unwrap()),
            ),
            (
                3,
                training_session(3, Some(3), NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
            ),
        ]);
        assert_eq!(
            sort_routines_by_last_use(&routines, &training_sessions, |r| r.id > 2),
            vec![routine(3), routine(4)]
        );
    }

    fn routine(id: u32) -> storage::Routine {
        storage::Routine {
            id,
            name: id.to_string(),
            notes: None,
            archived: false,
            sections: vec![],
        }
    }

    fn training_session(
        id: u32,
        routine_id: Option<u32>,
        date: NaiveDate,
    ) -> storage::TrainingSession {
        storage::TrainingSession {
            id,
            routine_id,
            date,
            notes: None,
            elements: vec![],
        }
    }
}
