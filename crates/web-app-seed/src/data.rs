use std::{collections::BTreeMap, sync::Arc};

use chrono::{prelude::*, Duration};
use gloo_console::{debug, error};
use seed::{
    app::{subs, Orders},
    button, div, nodes, p,
    prelude::{ev, El, Ev, Node},
    span,
    virtual_dom::{ToClasses, UpdateEl},
    Url, C, IF,
};
use valens_domain as domain;
use valens_storage as storage;
use valens_web_app as web_app;

use crate::common;

// ------ ------
//     Init
// ------ ------

#[allow(clippy::needless_pass_by_value)]
pub fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders
        .send_msg(Msg::ReadSettings)
        .send_msg(Msg::ReadOngoingTrainingSession);
    Model {
        storage: Arc::new(storage::rest::Storage),
        ui_storage: Arc::new(storage::local_storage::UI),
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
        last_refresh: DateTime::default(),
        avg_body_weight: BTreeMap::new(),
        cycles: Vec::new(),
        current_cycle: None,
        training_stats: domain::TrainingStats {
            short_term_load: Vec::new(),
            long_term_load: Vec::new(),
        },
        settings: web_app::Settings::default(),
        ongoing_training_session: None,
    }
}

// ------ ------
//     Model
// ------ ------

#[allow(clippy::struct_excessive_bools)]
pub struct Model {
    storage: Arc<dyn storage::Storage>,
    ui_storage: Arc<dyn storage::UI>,
    pub base_url: Url,
    errors: Vec<String>,
    app_update_available: bool,

    // ------ Data -----
    pub session: Option<domain::User>,
    pub version: String,
    pub users: BTreeMap<u32, domain::User>,
    pub loading_users: bool,

    // ------ Session-dependent data ------
    pub body_weight: BTreeMap<NaiveDate, domain::BodyWeight>,
    pub loading_body_weight: bool,
    pub body_fat: BTreeMap<NaiveDate, domain::BodyFat>,
    pub loading_body_fat: bool,
    pub period: BTreeMap<NaiveDate, domain::Period>,
    pub loading_period: bool,
    pub exercises: BTreeMap<u32, domain::Exercise>,
    pub loading_exercises: bool,
    pub routines: BTreeMap<u32, domain::Routine>,
    pub loading_routines: bool,
    pub training_sessions: BTreeMap<u32, domain::TrainingSession>,
    pub loading_training_sessions: bool,
    pub last_refresh: DateTime<Utc>,

    // ------ Derived data ------
    pub avg_body_weight: BTreeMap<NaiveDate, domain::BodyWeight>,
    pub cycles: Vec<domain::Cycle>,
    pub current_cycle: Option<domain::CurrentCycle>,
    pub training_stats: domain::TrainingStats,

    // ------ Client-side data ------
    pub settings: web_app::Settings,
    pub ongoing_training_session: Option<web_app::OngoingTrainingSession>,
}

impl Model {
    pub fn exercises(&self, filter: &domain::ExerciseFilter) -> Vec<&domain::Exercise> {
        self.exercises
            .values()
            .filter(|e| {
                filter.muscles.is_empty()
                    || filter
                        .muscles
                        .iter()
                        .all(|m| e.muscle_stimulus().contains_key(&m.id()))
            })
            .collect()
    }

    pub fn routines_sorted_by_last_use(
        &self,
        filter: impl Fn(&domain::Routine) -> bool,
    ) -> Vec<domain::Routine> {
        sort_routines_by_last_use(&self.routines, &self.training_sessions, filter)
    }

    pub fn training_sessions_date_range(&self) -> std::ops::RangeInclusive<NaiveDate> {
        let dates = self.training_sessions.values().map(|t| t.date);
        dates.clone().min().unwrap_or_default()..=dates.max().unwrap_or_default()
    }

    pub fn theme(&self) -> &web_app::Theme {
        match self.settings.theme {
            web_app::Theme::System => {
                if let Some(window) = web_sys::window() {
                    if let Ok(prefers_dark_scheme) =
                        window.match_media("(prefers-color-scheme: dark)")
                    {
                        if let Some(media_query_list) = prefers_dark_scheme {
                            if media_query_list.matches() {
                                &web_app::Theme::Dark
                            } else {
                                &web_app::Theme::Light
                            }
                        } else {
                            error!("failed to determine preferred color scheme");
                            &web_app::Theme::Light
                        }
                    } else {
                        error!("failed to match media to determine preferred color scheme");
                        &web_app::Theme::Light
                    }
                } else {
                    error!("failed to access window to determine preferred color scheme");
                    &web_app::Theme::Light
                }
            }
            web_app::Theme::Light | web_app::Theme::Dark => &self.settings.theme,
        }
    }
}

fn sort_routines_by_last_use(
    routines: &BTreeMap<u32, domain::Routine>,
    training_sessions: &BTreeMap<u32, domain::TrainingSession>,
    filter: impl Fn(&domain::Routine) -> bool,
) -> Vec<domain::Routine> {
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

// ------ ------
//    Update
// ------ ------

#[derive(Clone)]
pub enum Msg {
    RemoveError,

    UpdateApp,
    CancelAppUpdate,

    Refresh,
    ClearSessionDependentData,

    RequestSession(u32),
    SessionReceived(Result<domain::User, String>),
    InitializeSession,
    SessionInitialized(Result<domain::User, String>),

    DeleteSession,
    SessionDeleted(Result<(), String>),

    ReadVersion,
    VersionRead(Result<String, String>),

    ReadUsers,
    UsersRead(Result<Vec<domain::User>, String>),
    CreateUser(String, u8),
    UserCreated(Result<domain::User, String>),
    ReplaceUser(domain::User),
    UserReplaced(Result<domain::User, String>),
    DeleteUser(u32),
    UserDeleted(Result<u32, String>),

    ReadBodyWeight,
    BodyWeightRead(Result<Vec<domain::BodyWeight>, String>),
    CreateBodyWeight(domain::BodyWeight),
    BodyWeightCreated(Result<domain::BodyWeight, String>),
    ReplaceBodyWeight(domain::BodyWeight),
    BodyWeightReplaced(Result<domain::BodyWeight, String>),
    DeleteBodyWeight(NaiveDate),
    BodyWeightDeleted(Result<NaiveDate, String>),

    ReadBodyFat,
    BodyFatRead(Result<Vec<domain::BodyFat>, String>),
    CreateBodyFat(domain::BodyFat),
    BodyFatCreated(Result<domain::BodyFat, String>),
    ReplaceBodyFat(domain::BodyFat),
    BodyFatReplaced(Result<domain::BodyFat, String>),
    DeleteBodyFat(NaiveDate),
    BodyFatDeleted(Result<NaiveDate, String>),

    ReadPeriod,
    PeriodRead(Result<Vec<domain::Period>, String>),
    CreatePeriod(domain::Period),
    PeriodCreated(Result<domain::Period, String>),
    ReplacePeriod(domain::Period),
    PeriodReplaced(Result<domain::Period, String>),
    DeletePeriod(NaiveDate),
    PeriodDeleted(Result<NaiveDate, String>),

    ReadExercises,
    ExercisesRead(Result<Vec<domain::Exercise>, String>),
    CreateExercise(String, Vec<domain::ExerciseMuscle>),
    ExerciseCreated(Result<domain::Exercise, String>),
    ReplaceExercise(domain::Exercise),
    ExerciseReplaced(Result<domain::Exercise, String>),
    DeleteExercise(u32),
    ExerciseDeleted(Result<u32, String>),

    ReadRoutines,
    RoutinesRead(Result<Vec<domain::Routine>, String>),
    CreateRoutine(String, u32),
    RoutineCreated(Result<domain::Routine, String>),
    ModifyRoutine(
        u32,
        Option<String>,
        Option<bool>,
        Option<Vec<domain::RoutinePart>>,
    ),
    RoutineModified(Result<domain::Routine, String>),
    DeleteRoutine(u32),
    RoutineDeleted(Result<u32, String>),

    ReadTrainingSessions,
    TrainingSessionsRead(Result<Vec<domain::TrainingSession>, String>),
    CreateTrainingSession(
        Option<u32>,
        NaiveDate,
        String,
        Vec<domain::TrainingSessionElement>,
    ),
    TrainingSessionCreated(Result<domain::TrainingSession, String>),
    ModifyTrainingSession(
        u32,
        Option<String>,
        Option<Vec<domain::TrainingSessionElement>>,
    ),
    TrainingSessionModified(Result<domain::TrainingSession, String>),
    DeleteTrainingSession(u32),
    TrainingSessionDeleted(Result<u32, String>),

    SetBeepVolume(u8),
    SetTheme(web_app::Theme),
    SetAutomaticMetronome(bool),
    SetNotifications(bool),
    SetShowRPE(bool),
    SetShowTUT(bool),

    StartTrainingSession(u32),
    UpdateTrainingSession(usize, web_app::TimerState),
    EndTrainingSession,

    ReadSettings,
    SettingsRead(Result<web_app::Settings, String>),
    WriteSettings,
    SettingsWritten(Result<(), String>),

    ReadOngoingTrainingSession,
    OngoingTrainingSessionRead(Result<Option<web_app::OngoingTrainingSession>, String>),
    WriteOngoingTrainingSession,
    OngoingTrainingSessionWritten(Result<(), String>),
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

        Msg::UpdateApp => {
            match web_app::service_worker::post(&web_app::service_worker::Message::UpdateCache) {
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
            model.avg_body_weight.clear();
            model.cycles.clear();
            model.current_cycle = None;
            model.training_stats.clear();
        }

        Msg::RequestSession(user_id) => {
            let storage = model.storage.clone();
            orders.skip().perform_cmd(async move {
                Msg::SessionReceived(storage.request_session(user_id).await)
            });
        }
        Msg::SessionReceived(Ok(new_session)) => {
            model.session = Some(new_session);
            orders.send_msg(Msg::Refresh).request_url(
                crate::Urls::new(model.base_url.clone().set_hash_path([""; 0])).home(),
            );
        }
        Msg::SessionReceived(Err(message)) => {
            model.session = None;
            model
                .errors
                .push("Failed to request session: ".to_owned() + &message);
        }
        Msg::InitializeSession => {
            let storage = model.storage.clone();
            orders.perform_cmd(async move {
                Msg::SessionInitialized(storage.initialize_session().await)
            });
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
            let storage = model.storage.clone();
            orders
                .skip()
                .send_msg(Msg::ClearSessionDependentData)
                .perform_cmd(async move { Msg::SessionDeleted(storage.delete_session().await) });
        }
        Msg::SessionDeleted(Ok(())) => {
            model.session = None;
            orders.request_url(crate::Urls::new(&model.base_url).login());
        }
        Msg::SessionDeleted(Err(message)) => {
            model
                .errors
                .push("Failed to switch users: ".to_owned() + &message);
        }

        Msg::ReadVersion => {
            let storage = model.storage.clone();
            orders.perform_cmd(async move { Msg::VersionRead(storage.read_version().await) });
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
            let storage = model.storage.clone();
            orders.perform_cmd(async move { Msg::UsersRead(storage.read_users().await) });
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
        Msg::CreateUser(name, sex) => {
            let storage = model.storage.clone();
            orders
                .perform_cmd(async move { Msg::UserCreated(storage.create_user(name, sex).await) });
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
            let storage = model.storage.clone();
            orders.perform_cmd(async move { Msg::UserReplaced(storage.replace_user(user).await) });
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
            let storage = model.storage.clone();
            orders.perform_cmd(async move { Msg::UserDeleted(storage.delete_user(id).await) });
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
            let storage = model.storage.clone();
            orders
                .skip()
                .perform_cmd(async move { Msg::BodyWeightRead(storage.read_body_weight().await) });
        }
        Msg::BodyWeightRead(Ok(body_weight)) => {
            let body_weight = body_weight.into_iter().map(|e| (e.date, e)).collect();
            if model.body_weight != body_weight {
                model.body_weight = body_weight;
                model.avg_body_weight = domain::avg_body_weight(&model.body_weight);
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
            let storage = model.storage.clone();
            orders.perform_cmd(async move {
                Msg::BodyWeightCreated(storage.create_body_weight(body_weight).await)
            });
        }
        Msg::BodyWeightCreated(Ok(body_weight)) => {
            model.body_weight.insert(body_weight.date, body_weight);
            model.avg_body_weight = domain::avg_body_weight(&model.body_weight);
            orders.notify(Event::BodyWeightCreatedOk);
        }
        Msg::BodyWeightCreated(Err(message)) => {
            orders.notify(Event::BodyWeightCreatedErr);
            model
                .errors
                .push("Failed to create body weight: ".to_owned() + &message);
        }
        Msg::ReplaceBodyWeight(body_weight) => {
            let storage = model.storage.clone();
            orders.perform_cmd(async move {
                Msg::BodyWeightReplaced(storage.replace_body_weight(body_weight).await)
            });
        }
        Msg::BodyWeightReplaced(Ok(body_weight)) => {
            model.body_weight.insert(body_weight.date, body_weight);
            model.avg_body_weight = domain::avg_body_weight(&model.body_weight);
            orders.notify(Event::BodyWeightReplacedOk);
        }
        Msg::BodyWeightReplaced(Err(message)) => {
            orders.notify(Event::BodyWeightReplacedErr);
            model
                .errors
                .push("Failed to replace body weight: ".to_owned() + &message);
        }
        Msg::DeleteBodyWeight(date) => {
            let storage = model.storage.clone();
            orders.perform_cmd(async move {
                Msg::BodyWeightDeleted(storage.delete_body_weight(date).await)
            });
        }
        Msg::BodyWeightDeleted(Ok(date)) => {
            model.body_weight.remove(&date);
            model.avg_body_weight = domain::avg_body_weight(&model.body_weight);
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
            let storage = model.storage.clone();
            orders
                .skip()
                .perform_cmd(async move { Msg::BodyFatRead(storage.read_body_fat().await) });
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
            let storage = model.storage.clone();
            orders.perform_cmd(async move {
                Msg::BodyFatCreated(storage.create_body_fat(body_fat).await)
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
            let storage = model.storage.clone();
            orders.perform_cmd(async move {
                Msg::BodyFatReplaced(storage.replace_body_fat(body_fat).await)
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
            let storage = model.storage.clone();
            orders.perform_cmd(
                async move { Msg::BodyFatDeleted(storage.delete_body_fat(date).await) },
            );
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
            let storage = model.storage.clone();
            orders
                .skip()
                .perform_cmd(async move { Msg::PeriodRead(storage.read_period().await) });
        }
        Msg::PeriodRead(Ok(period)) => {
            let period = period.into_iter().map(|e| (e.date, e)).collect();
            if model.period != period {
                model.period = period;
                model.cycles = domain::cycles(&model.period);
                model.current_cycle = domain::current_cycle(&model.cycles);
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
            let storage = model.storage.clone();
            orders.perform_cmd(
                async move { Msg::PeriodCreated(storage.create_period(period).await) },
            );
        }
        Msg::PeriodCreated(Ok(period)) => {
            model.period.insert(period.date, period);
            model.cycles = domain::cycles(&model.period);
            model.current_cycle = domain::current_cycle(&model.cycles);
            orders.notify(Event::PeriodCreatedOk);
        }
        Msg::PeriodCreated(Err(message)) => {
            orders.notify(Event::PeriodCreatedErr);
            model
                .errors
                .push("Failed to create period: ".to_owned() + &message);
        }
        Msg::ReplacePeriod(period) => {
            let storage = model.storage.clone();
            orders.perform_cmd(
                async move { Msg::PeriodReplaced(storage.replace_period(period).await) },
            );
        }
        Msg::PeriodReplaced(Ok(period)) => {
            model.period.insert(period.date, period);
            model.cycles = domain::cycles(&model.period);
            model.current_cycle = domain::current_cycle(&model.cycles);
            orders.notify(Event::PeriodReplacedOk);
        }
        Msg::PeriodReplaced(Err(message)) => {
            orders.notify(Event::PeriodReplacedErr);
            model
                .errors
                .push("Failed to replace period: ".to_owned() + &message);
        }
        Msg::DeletePeriod(date) => {
            let storage = model.storage.clone();
            orders
                .perform_cmd(async move { Msg::PeriodDeleted(storage.delete_period(date).await) });
        }
        Msg::PeriodDeleted(Ok(date)) => {
            model.period.remove(&date);
            model.cycles = domain::cycles(&model.period);
            model.current_cycle = domain::current_cycle(&model.cycles);
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
            let storage = model.storage.clone();
            orders
                .skip()
                .perform_cmd(async move { Msg::ExercisesRead(storage.read_exercises().await) });
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
            let storage = model.storage.clone();
            orders.perform_cmd(async move {
                Msg::ExerciseCreated(storage.create_exercise(name, muscles).await)
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
            let storage = model.storage.clone();
            orders.perform_cmd(async move {
                Msg::ExerciseReplaced(storage.replace_exercise(exercise).await)
            });
        }
        Msg::ExerciseReplaced(Ok(exercise)) => {
            model.exercises.insert(exercise.id, exercise);
            model.training_stats =
                domain::training_stats(&model.training_sessions.values().collect::<Vec<_>>());
            orders.notify(Event::ExerciseReplacedOk);
        }
        Msg::ExerciseReplaced(Err(message)) => {
            orders.notify(Event::ExerciseReplacedErr);
            model
                .errors
                .push("Failed to replace exercise: ".to_owned() + &message);
        }
        Msg::DeleteExercise(id) => {
            let storage = model.storage.clone();
            orders.perform_cmd(
                async move { Msg::ExerciseDeleted(storage.delete_exercise(id).await) },
            );
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
            let storage = model.storage.clone();
            orders
                .skip()
                .perform_cmd(async move { Msg::RoutinesRead(storage.read_routines().await) });
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
            let storage = model.storage.clone();
            orders.perform_cmd(async move {
                Msg::RoutineCreated(storage.create_routine(name, sections).await)
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
            let storage = model.storage.clone();
            orders.perform_cmd(async move {
                Msg::RoutineModified(storage.modify_routine(id, name, archived, sections).await)
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
            let storage = model.storage.clone();
            orders
                .perform_cmd(async move { Msg::RoutineDeleted(storage.delete_routine(id).await) });
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
            let storage = model.storage.clone();
            orders.skip().perform_cmd(async move {
                Msg::TrainingSessionsRead(storage.read_training_sessions().await)
            });
        }
        Msg::TrainingSessionsRead(Ok(training_sessions)) => {
            let training_sessions = training_sessions.into_iter().map(|t| (t.id, t)).collect();
            if model.training_sessions != training_sessions {
                model.training_sessions = training_sessions;
                model.training_stats =
                    domain::training_stats(&model.training_sessions.values().collect::<Vec<_>>());
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
            let storage = model.storage.clone();
            orders.perform_cmd(async move {
                Msg::TrainingSessionCreated(
                    storage
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
                domain::training_stats(&model.training_sessions.values().collect::<Vec<_>>());
            orders.notify(Event::TrainingSessionCreatedOk);
        }
        Msg::TrainingSessionCreated(Err(message)) => {
            orders.notify(Event::TrainingSessionCreatedErr);
            model
                .errors
                .push("Failed to create training session: ".to_owned() + &message);
        }
        Msg::ModifyTrainingSession(id, notes, elements) => {
            let storage = model.storage.clone();
            orders.perform_cmd(async move {
                Msg::TrainingSessionModified(
                    storage.modify_training_session(id, notes, elements).await,
                )
            });
        }
        Msg::TrainingSessionModified(Ok(training_session)) => {
            model
                .training_sessions
                .insert(training_session.id, training_session);
            model.training_stats =
                domain::training_stats(&model.training_sessions.values().collect::<Vec<_>>());
            orders.notify(Event::TrainingSessionModifiedOk);
        }
        Msg::TrainingSessionModified(Err(message)) => {
            orders.notify(Event::TrainingSessionModifiedErr);
            model
                .errors
                .push("Failed to modify training session: ".to_owned() + &message);
        }
        Msg::DeleteTrainingSession(id) => {
            let storage = model.storage.clone();
            orders.perform_cmd(async move {
                Msg::TrainingSessionDeleted(storage.delete_training_session(id).await)
            });
        }
        Msg::TrainingSessionDeleted(Ok(id)) => {
            model.training_sessions.remove(&id);
            model.training_stats =
                domain::training_stats(&model.training_sessions.values().collect::<Vec<_>>());
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
            orders
                .send_msg(Msg::WriteSettings)
                .notify(Event::BeepVolumeChanged);
        }
        Msg::SetTheme(theme) => {
            apply_theme(&theme);
            model.settings.theme = theme;
            orders.send_msg(Msg::WriteSettings);
        }
        Msg::SetAutomaticMetronome(value) => {
            model.settings.automatic_metronome = value;
            orders.send_msg(Msg::WriteSettings);
        }
        Msg::SetNotifications(value) => {
            model.settings.notifications = value;
            orders.send_msg(Msg::WriteSettings);
        }
        Msg::SetShowRPE(value) => {
            model.settings.show_rpe = value;
            orders.send_msg(Msg::WriteSettings);
        }
        Msg::SetShowTUT(value) => {
            model.settings.show_tut = value;
            orders.send_msg(Msg::WriteSettings);
        }

        Msg::StartTrainingSession(training_session_id) => {
            model.ongoing_training_session =
                Some(web_app::OngoingTrainingSession::new(training_session_id));
            orders.send_msg(Msg::WriteOngoingTrainingSession);
        }
        Msg::UpdateTrainingSession(section_idx, timer_state) => {
            if let Some(ongoing_training_session) = &mut model.ongoing_training_session {
                ongoing_training_session.element_idx = section_idx;
                ongoing_training_session.element_start_time = Utc::now();
                ongoing_training_session.timer_state = timer_state;
            }
            orders.send_msg(Msg::WriteOngoingTrainingSession);
        }
        Msg::EndTrainingSession => {
            model.ongoing_training_session = None;
            orders.send_msg(Msg::WriteOngoingTrainingSession);
        }

        Msg::ReadSettings => {
            let storage = model.ui_storage.clone();
            orders
                .skip()
                .perform_cmd(async move { Msg::SettingsRead(storage.read_settings().await) });
        }
        Msg::SettingsRead(Ok(settings)) => {
            apply_theme(&settings.theme);
            model.settings = settings;
        }
        Msg::SettingsRead(Err(message)) => {
            debug!("Failed to read settings: ".to_owned() + &message);
        }
        Msg::WriteSettings => {
            let settings = model.settings.clone();
            let storage = model.ui_storage.clone();
            orders.skip().perform_cmd(async move {
                Msg::SettingsWritten(storage.write_settings(settings).await)
            });
        }
        Msg::SettingsWritten(result) => {
            if let Err(message) = result {
                error!("Failed to write settings: ".to_owned() + &message);
            }
        }

        Msg::ReadOngoingTrainingSession => {
            let storage = model.ui_storage.clone();
            orders.skip().perform_cmd(async move {
                Msg::OngoingTrainingSessionRead(storage.read_ongoing_training_session().await)
            });
        }
        Msg::OngoingTrainingSessionRead(Ok(ongoing_training_session)) => {
            model.ongoing_training_session = ongoing_training_session;
        }
        Msg::OngoingTrainingSessionRead(Err(message)) => {
            debug!("Failed to read ongoing training session: ".to_owned() + &message);
        }
        Msg::WriteOngoingTrainingSession => {
            let ongoing_training_session = model.ongoing_training_session.clone();
            let storage = model.ui_storage.clone();
            orders.skip().perform_cmd(async move {
                Msg::OngoingTrainingSessionWritten(
                    storage
                        .write_ongoing_training_session(ongoing_training_session)
                        .await,
                )
            });
        }
        Msg::OngoingTrainingSessionWritten(result) => {
            if let Err(message) = result {
                error!("Failed to write ongoing training session: ".to_owned() + &message);
            }
        }
    }
}

fn apply_theme(theme: &web_app::Theme) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(html_element) = document.document_element() {
                let _ = match theme {
                    web_app::Theme::System => html_element.remove_attribute("data-theme"),
                    web_app::Theme::Light => html_element.set_attribute("data-theme", "light"),
                    web_app::Theme::Dark => html_element.set_attribute("data-theme", "dark"),
                };
            }
        }
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
        span!["Update"],
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

    fn routine(id: u32) -> domain::Routine {
        domain::Routine {
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
    ) -> domain::TrainingSession {
        domain::TrainingSession {
            id,
            routine_id,
            date,
            notes: None,
            elements: vec![],
        }
    }
}
