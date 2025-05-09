use std::collections::{BTreeMap, HashMap};

use chrono::{Duration, prelude::*};
use log::{error, warn};
use seed::{
    C, IF, Url,
    app::{Orders, subs},
    button, div, nodes, p,
    prelude::{El, Ev, Node, ev},
    span,
    virtual_dom::{ToClasses, UpdateEl},
};
use valens_domain as domain;
use valens_domain::{
    BodyFatRepository, BodyWeightRepository, ExerciseRepository, PeriodRepository,
    RoutineRepository, SessionRepository, TrainingSessionRepository, UserRepository,
    VersionRepository,
};
use valens_storage as storage;
use valens_web_app as web_app;
use valens_web_app::{OngoingTrainingSessionRepository, SettingsRepository};

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
        cached_rest: storage::cached_rest::CachedREST::new(),
        local_storage: storage::local_storage::LocalStorage,
        base_url: url.to_hash_base_url(),
        errors: Vec::new(),
        app_update_available: false,
        no_connection: false,
        session: None,
        version: String::new(),
        users: BTreeMap::new(),
        loading_users: 0,
        body_weight: BTreeMap::new(),
        loading_body_weight: 0,
        body_fat: BTreeMap::new(),
        loading_body_fat: 0,
        period: BTreeMap::new(),
        loading_period: 0,
        exercises: BTreeMap::new(),
        loading_exercises: 0,
        routines: BTreeMap::new(),
        loading_routines: 0,
        training_sessions: BTreeMap::new(),
        loading_training_sessions: 0,
        last_refresh: HashMap::new(),
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

pub struct Model {
    cached_rest: storage::cached_rest::CachedREST<storage::rest::GlooNetSendRequest>,
    local_storage: storage::local_storage::LocalStorage,
    pub base_url: Url,
    errors: Vec<String>,
    app_update_available: bool,
    pub no_connection: bool,

    // ------ Data -----
    pub session: Option<domain::User>,
    pub version: String,
    pub users: BTreeMap<domain::UserID, domain::User>,
    pub loading_users: u8,

    // ------ Session-dependent data ------
    pub body_weight: BTreeMap<NaiveDate, domain::BodyWeight>,
    pub loading_body_weight: u8,
    pub body_fat: BTreeMap<NaiveDate, domain::BodyFat>,
    pub loading_body_fat: u8,
    pub period: BTreeMap<NaiveDate, domain::Period>,
    pub loading_period: u8,
    pub exercises: BTreeMap<domain::ExerciseID, domain::Exercise>,
    pub loading_exercises: u8,
    pub routines: BTreeMap<domain::RoutineID, domain::Routine>,
    pub loading_routines: u8,
    pub training_sessions: BTreeMap<domain::TrainingSessionID, domain::TrainingSession>,
    pub loading_training_sessions: u8,
    pub last_refresh: HashMap<DataSet, DateTime<Utc>>,

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
        filter.exercises(self.exercises.values())
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

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum DataSet {
    BodyWeight,
    BodyFat,
    Period,
    Exercises,
    Routines,
    TrainingSessions,
}

fn sort_routines_by_last_use(
    routines: &BTreeMap<domain::RoutineID, domain::Routine>,
    training_sessions: &BTreeMap<domain::TrainingSessionID, domain::TrainingSession>,
    filter: impl Fn(&domain::Routine) -> bool,
) -> Vec<domain::Routine> {
    let mut map: BTreeMap<domain::RoutineID, NaiveDate> = BTreeMap::new();
    for (routine_id, _) in routines.iter().filter(|(_, r)| filter(r)) {
        #[allow(clippy::cast_possible_truncation)]
        map.insert(
            *routine_id,
            NaiveDate::MIN + Duration::days(routine_id.as_u128() as i64),
        );
    }
    for training_session in training_sessions.values() {
        let routine_id = training_session.routine_id;
        if routines.contains_key(&routine_id)
            && filter(&routines[&routine_id])
            && training_session.date > map[&routine_id]
        {
            map.insert(routine_id, training_session.date);
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

    Initialize,
    Refresh,
    ClearSessionDependentData,

    RequestSession(domain::UserID),
    SessionReceived(Result<domain::User, String>),
    InitializeSession,
    SessionInitialized(Result<domain::User, String>),

    DeleteSession,
    SessionDeleted(Result<(), String>),

    ReadVersion,
    VersionRead(Result<String, String>),

    ReadUsers,
    UsersRead(Result<Vec<domain::User>, String>),
    CreateUser(domain::Name, domain::Sex),
    UserCreated(Result<domain::User, String>),
    ReplaceUser(domain::User),
    UserReplaced(Result<domain::User, String>),
    DeleteUser(domain::UserID),
    UserDeleted(Result<domain::UserID, String>),

    SyncBodyWeight,
    BodyWeightSynced(Result<Vec<domain::BodyWeight>, String>),
    ReadBodyWeight,
    BodyWeightRead(Result<Vec<domain::BodyWeight>, String>),
    CreateBodyWeight(domain::BodyWeight),
    BodyWeightCreated(Result<domain::BodyWeight, String>),
    ReplaceBodyWeight(domain::BodyWeight),
    BodyWeightReplaced(Result<domain::BodyWeight, String>),
    DeleteBodyWeight(NaiveDate),
    BodyWeightDeleted(Result<NaiveDate, String>),

    SyncBodyFat,
    BodyFatSynced(Result<Vec<domain::BodyFat>, String>),
    ReadBodyFat,
    BodyFatRead(Result<Vec<domain::BodyFat>, String>),
    CreateBodyFat(domain::BodyFat),
    BodyFatCreated(Result<domain::BodyFat, String>),
    ReplaceBodyFat(domain::BodyFat),
    BodyFatReplaced(Result<domain::BodyFat, String>),
    DeleteBodyFat(NaiveDate),
    BodyFatDeleted(Result<NaiveDate, String>),

    SyncPeriod,
    PeriodSynced(Result<Vec<domain::Period>, String>),
    ReadPeriod,
    PeriodRead(Result<Vec<domain::Period>, String>),
    CreatePeriod(domain::Period),
    PeriodCreated(Result<domain::Period, String>),
    ReplacePeriod(domain::Period),
    PeriodReplaced(Result<domain::Period, String>),
    DeletePeriod(NaiveDate),
    PeriodDeleted(Result<NaiveDate, String>),

    SyncExercises,
    ExercisesSynced(Result<Vec<domain::Exercise>, String>),
    ReadExercises,
    ExercisesRead(Result<Vec<domain::Exercise>, String>),
    CreateExercise(domain::Name, Vec<domain::ExerciseMuscle>),
    ExerciseCreated(Result<domain::Exercise, String>),
    ReplaceExercise(domain::Exercise),
    ExerciseReplaced(Result<domain::Exercise, String>),
    DeleteExercise(domain::ExerciseID),
    ExerciseDeleted(Result<domain::ExerciseID, String>),

    SyncRoutines,
    RoutinesSynced(Result<Vec<domain::Routine>, String>),
    ReadRoutines,
    RoutinesRead(Result<Vec<domain::Routine>, String>),
    CreateRoutine(domain::Name, domain::RoutineID),
    RoutineCreated(Result<domain::Routine, String>),
    ModifyRoutine(
        domain::RoutineID,
        Option<domain::Name>,
        Option<bool>,
        Option<Vec<domain::RoutinePart>>,
    ),
    RoutineModified(Result<domain::Routine, String>),
    DeleteRoutine(domain::RoutineID),
    RoutineDeleted(Result<domain::RoutineID, String>),

    SyncTrainingSessions,
    TrainingSessionsSynced(Result<Vec<domain::TrainingSession>, String>),
    ReadTrainingSessions,
    TrainingSessionsRead(Result<Vec<domain::TrainingSession>, String>),
    CreateTrainingSession(
        domain::RoutineID,
        NaiveDate,
        String,
        Vec<domain::TrainingSessionElement>,
    ),
    TrainingSessionCreated(Result<domain::TrainingSession, String>),
    ModifyTrainingSession(
        domain::TrainingSessionID,
        Option<String>,
        Option<Vec<domain::TrainingSessionElement>>,
    ),
    TrainingSessionModified(Result<domain::TrainingSession, String>),
    DeleteTrainingSession(domain::TrainingSessionID),
    TrainingSessionDeleted(Result<domain::TrainingSessionID, String>),

    SetBeepVolume(u8),
    SetTheme(web_app::Theme),
    SetAutomaticMetronome(bool),
    SetNotifications(bool),
    SetShowRPE(bool),
    SetShowTUT(bool),

    StartTrainingSession(domain::TrainingSessionID),
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

        Msg::Initialize => {
            orders
                .send_msg(Msg::ReadBodyWeight)
                .send_msg(Msg::ReadBodyFat)
                .send_msg(Msg::ReadPeriod)
                .send_msg(Msg::ReadExercises)
                .send_msg(Msg::ReadRoutines)
                .send_msg(Msg::ReadTrainingSessions);
        }
        Msg::Refresh => {
            orders
                .send_msg(Msg::ReadVersion)
                .send_msg(Msg::ReadUsers)
                .send_msg(Msg::SyncBodyWeight)
                .send_msg(Msg::SyncBodyFat)
                .send_msg(Msg::SyncPeriod)
                .send_msg(Msg::SyncExercises)
                .send_msg(Msg::SyncRoutines)
                .send_msg(Msg::SyncTrainingSessions);
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
            let storage = model.cached_rest.clone();
            orders.skip().perform_cmd(async move {
                Msg::SessionReceived(
                    storage
                        .request_session(user_id)
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
        }
        Msg::SessionReceived(Ok(new_session)) => {
            model.session = Some(new_session);
            orders
                .send_msg(Msg::Initialize)
                .send_msg(Msg::Refresh)
                .request_url(
                    crate::Urls::new(model.base_url.clone().set_hash_path([""; 0])).home(),
                );
        }
        Msg::SessionReceived(Err(message)) => {
            model.session = None;
            model
                .errors
                .push(format!("Failed to request session: {message}"));
        }
        Msg::InitializeSession => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::SessionInitialized(
                    storage
                        .initialize_session()
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
        }
        Msg::SessionInitialized(Ok(session)) => {
            model.session = Some(session);
            orders
                .notify(subs::UrlChanged(Url::current()))
                .send_msg(Msg::Initialize)
                .send_msg(Msg::Refresh);
        }
        Msg::SessionInitialized(Err(_)) => {
            model.session = None;
            orders.notify(subs::UrlChanged(Url::current()));
        }
        Msg::DeleteSession => {
            let storage = model.cached_rest.clone();
            orders.skip().perform_cmd(async move {
                Msg::SessionDeleted(
                    storage
                        .delete_session()
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
        }
        Msg::SessionDeleted(Ok(())) => {
            model.session = None;
            orders
                .send_msg(Msg::ClearSessionDependentData)
                .request_url(crate::Urls::new(&model.base_url).login());
        }
        Msg::SessionDeleted(Err(message)) => {
            model
                .errors
                .push(format!("Failed to switch users: {message}"));
        }

        Msg::ReadVersion => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::VersionRead(storage.read_version().await.map_err(|err| err.to_string()))
            });
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
            model.no_connection = false;
        }
        Msg::VersionRead(Err(message)) => {
            model.no_connection = true;
            warn!("failed to read version: {message}");
        }

        Msg::ReadUsers => {
            model.loading_users += 1;
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::UsersRead(storage.read_users().await.map_err(|err| err.to_string()))
            });
        }
        Msg::UsersRead(Ok(users)) => {
            let users = users.into_iter().map(|e| (e.id, e)).collect();
            if model.users != users {
                model.users = users;
                orders.notify(Event::DataChanged);
            }
            model.loading_users -= 1;
            model.no_connection = false;
        }
        Msg::UsersRead(Err(message)) => {
            model.loading_users -= 1;
            model.no_connection = true;
            warn!("failed to read users: {message}");
        }
        Msg::CreateUser(name, sex) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::UserCreated(
                    storage
                        .create_user(name, sex)
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
        }
        Msg::UserCreated(Ok(user)) => {
            model.users.insert(user.id, user);
            orders.notify(Event::UserCreatedOk);
        }
        Msg::UserCreated(Err(message)) => {
            orders.notify(Event::UserCreatedErr);
            model
                .errors
                .push(format!("Failed to create user: {message}"));
        }
        Msg::ReplaceUser(user) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::UserReplaced(
                    storage
                        .replace_user(user)
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
        }
        Msg::UserReplaced(Ok(user)) => {
            model.users.insert(user.id, user);
            orders.notify(Event::UserReplacedOk);
        }
        Msg::UserReplaced(Err(message)) => {
            orders.notify(Event::UserReplacedErr);
            model
                .errors
                .push(format!("Failed to replace user: {message}"));
        }
        Msg::DeleteUser(id) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::UserDeleted(storage.delete_user(id).await.map_err(|err| err.to_string()))
            });
        }
        Msg::UserDeleted(Ok(id)) => {
            model.users.remove(&id);
            orders.notify(Event::UserDeletedOk);
        }
        Msg::UserDeleted(Err(message)) => {
            orders.notify(Event::UserDeletedErr);
            model
                .errors
                .push(format!("Failed to delete user: {message}"));
        }

        Msg::SyncBodyWeight => {
            model.loading_body_weight += 1;
            let storage = model.cached_rest.clone();
            orders.skip().perform_cmd(async move {
                Msg::BodyWeightSynced(
                    storage
                        .sync_body_weight()
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
        }
        Msg::BodyWeightSynced(Ok(body_weight)) => {
            let body_weight = body_weight.into_iter().map(|e| (e.date, e)).collect();
            if model.body_weight != body_weight {
                model.body_weight = body_weight;
                model.avg_body_weight = domain::avg_body_weight(&model.body_weight);
                orders.notify(Event::DataChanged);
            }
            model.loading_body_weight -= 1;
            model.last_refresh.insert(DataSet::BodyWeight, Utc::now());
        }
        Msg::BodyWeightSynced(Err(message)) => {
            model.loading_body_weight -= 1;
            warn!("failed to sync body weight: {message}");
        }
        Msg::ReadBodyWeight => {
            model.loading_body_weight += 1;
            let storage = model.cached_rest.clone();
            orders.skip().perform_cmd(async move {
                Msg::BodyWeightRead(
                    storage
                        .read_body_weight()
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
        }
        Msg::BodyWeightRead(Ok(body_weight)) => {
            let body_weight = body_weight.into_iter().map(|e| (e.date, e)).collect();
            if model.body_weight != body_weight {
                model.body_weight = body_weight;
                model.avg_body_weight = domain::avg_body_weight(&model.body_weight);
                orders.notify(Event::DataChanged);
            }
            model.loading_body_weight -= 1;
        }
        Msg::BodyWeightRead(Err(message)) => {
            model
                .errors
                .push(format!("Failed to read body weight: {message}"));
            model.loading_body_weight -= 1;
        }
        Msg::CreateBodyWeight(body_weight) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::BodyWeightCreated(
                    storage
                        .create_body_weight(body_weight)
                        .await
                        .map_err(|err| err.to_string()),
                )
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
                .push(format!("Failed to create body weight: {message}"));
        }
        Msg::ReplaceBodyWeight(body_weight) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::BodyWeightReplaced(
                    storage
                        .replace_body_weight(body_weight)
                        .await
                        .map_err(|err| err.to_string()),
                )
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
                .push(format!("Failed to replace body weight: {message}"));
        }
        Msg::DeleteBodyWeight(date) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::BodyWeightDeleted(
                    storage
                        .delete_body_weight(date)
                        .await
                        .map_err(|err| err.to_string()),
                )
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
                .push(format!("Failed to delete body weight: {message}"));
        }

        Msg::SyncBodyFat => {
            model.loading_body_fat += 1;
            let storage = model.cached_rest.clone();
            orders.skip().perform_cmd(async move {
                Msg::BodyFatSynced(storage.sync_body_fat().await.map_err(|err| err.to_string()))
            });
        }
        Msg::BodyFatSynced(Ok(body_fat)) => {
            let body_fat = body_fat.into_iter().map(|e| (e.date, e)).collect();
            if model.body_fat != body_fat {
                model.body_fat = body_fat;
                orders.notify(Event::DataChanged);
            }
            model.loading_body_fat -= 1;
            model.last_refresh.insert(DataSet::BodyFat, Utc::now());
        }
        Msg::BodyFatSynced(Err(message)) => {
            model.loading_body_fat -= 1;
            warn!("failed to sync body fat: {message}");
        }
        Msg::ReadBodyFat => {
            model.loading_body_fat += 1;
            let storage = model.cached_rest.clone();
            orders.skip().perform_cmd(async move {
                Msg::BodyFatRead(storage.read_body_fat().await.map_err(|err| err.to_string()))
            });
        }
        Msg::BodyFatRead(Ok(body_fat)) => {
            let body_fat = body_fat.into_iter().map(|e| (e.date, e)).collect();
            if model.body_fat != body_fat {
                model.body_fat = body_fat;
                orders.notify(Event::DataChanged);
            }
            model.loading_body_fat -= 1;
        }
        Msg::BodyFatRead(Err(message)) => {
            model
                .errors
                .push(format!("Failed to read body fat: {message}"));
            model.loading_body_fat -= 1;
        }
        Msg::CreateBodyFat(body_fat) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::BodyFatCreated(
                    storage
                        .create_body_fat(body_fat)
                        .await
                        .map_err(|err| err.to_string()),
                )
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
                .push(format!("Failed to create body fat: {message}"));
        }
        Msg::ReplaceBodyFat(body_fat) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::BodyFatReplaced(
                    storage
                        .replace_body_fat(body_fat)
                        .await
                        .map_err(|err| err.to_string()),
                )
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
                .push(format!("Failed to replace body fat: {message}"));
        }
        Msg::DeleteBodyFat(date) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::BodyFatDeleted(
                    storage
                        .delete_body_fat(date)
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
        }
        Msg::BodyFatDeleted(Ok(date)) => {
            model.body_fat.remove(&date);
            orders.notify(Event::BodyFatDeletedOk);
        }
        Msg::BodyFatDeleted(Err(message)) => {
            orders.notify(Event::BodyFatDeletedErr);
            model
                .errors
                .push(format!("Failed to delete body fat: {message}"));
        }

        Msg::SyncPeriod => {
            model.loading_period += 1;
            let storage = model.cached_rest.clone();
            orders.skip().perform_cmd(async move {
                Msg::PeriodSynced(storage.sync_period().await.map_err(|err| err.to_string()))
            });
        }
        Msg::PeriodSynced(Ok(period)) => {
            let period = period.into_iter().map(|e| (e.date, e)).collect();
            if model.period != period {
                model.period = period;
                model.cycles = domain::cycles(&model.period);
                model.current_cycle = domain::current_cycle(&model.cycles);
                orders.notify(Event::DataChanged);
            }
            model.loading_period -= 1;
            model.last_refresh.insert(DataSet::Period, Utc::now());
        }
        Msg::PeriodSynced(Err(message)) => {
            model.loading_period -= 1;
            warn!("failed to sync period: {message}");
        }
        Msg::ReadPeriod => {
            model.loading_period += 1;
            let storage = model.cached_rest.clone();
            orders.skip().perform_cmd(async move {
                Msg::PeriodRead(storage.read_period().await.map_err(|err| err.to_string()))
            });
        }
        Msg::PeriodRead(Ok(period)) => {
            let period = period.into_iter().map(|e| (e.date, e)).collect();
            if model.period != period {
                model.period = period;
                model.cycles = domain::cycles(&model.period);
                model.current_cycle = domain::current_cycle(&model.cycles);
                orders.notify(Event::DataChanged);
            }
            model.loading_period -= 1;
        }
        Msg::PeriodRead(Err(message)) => {
            model
                .errors
                .push(format!("Failed to read period: {message}"));
            model.loading_period -= 1;
        }
        Msg::CreatePeriod(period) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::PeriodCreated(
                    storage
                        .create_period(period)
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
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
                .push(format!("Failed to create period: {message}"));
        }
        Msg::ReplacePeriod(period) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::PeriodReplaced(
                    storage
                        .replace_period(period)
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
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
                .push(format!("Failed to replace period: {message}"));
        }
        Msg::DeletePeriod(date) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::PeriodDeleted(
                    storage
                        .delete_period(date)
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
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
                .push(format!("Failed to delete period: {message}"));
        }

        Msg::SyncExercises => {
            model.loading_exercises += 1;
            let storage = model.cached_rest.clone();
            orders.skip().perform_cmd(async move {
                Msg::ExercisesSynced(
                    storage
                        .sync_exercises()
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
        }
        Msg::ExercisesSynced(Ok(exercises)) => {
            let exercises = exercises.into_iter().map(|e| (e.id, e)).collect();
            if model.exercises != exercises {
                model.exercises = exercises;
                orders.notify(Event::DataChanged);
            }
            model.loading_exercises -= 1;
            model.last_refresh.insert(DataSet::Exercises, Utc::now());
        }
        Msg::ExercisesSynced(Err(message)) => {
            model.loading_exercises -= 1;
            warn!("failed to sync exercises: {message}");
        }
        Msg::ReadExercises => {
            model.loading_exercises += 1;
            let storage = model.cached_rest.clone();
            orders.skip().perform_cmd(async move {
                Msg::ExercisesRead(
                    storage
                        .read_exercises()
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
        }
        Msg::ExercisesRead(Ok(exercises)) => {
            let exercises = exercises.into_iter().map(|e| (e.id, e)).collect();
            if model.exercises != exercises {
                model.exercises = exercises;
                orders.notify(Event::DataChanged);
            }
            model.loading_exercises -= 1;
        }
        Msg::ExercisesRead(Err(message)) => {
            model
                .errors
                .push(format!("Failed to read exercises: {message}"));
            model.loading_exercises -= 1;
        }
        Msg::CreateExercise(name, muscles) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::ExerciseCreated(
                    storage
                        .create_exercise(name, muscles)
                        .await
                        .map_err(|err| err.to_string()),
                )
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
                .push(format!("Failed to create exercise: {message}"));
        }
        Msg::ReplaceExercise(exercise) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::ExerciseReplaced(
                    storage
                        .replace_exercise(exercise)
                        .await
                        .map_err(|err| err.to_string()),
                )
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
                .push(format!("Failed to replace exercise: {message}"));
        }
        Msg::DeleteExercise(id) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::ExerciseDeleted(
                    storage
                        .delete_exercise(id)
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
        }
        Msg::ExerciseDeleted(Ok(id)) => {
            model.exercises.remove(&id);
            orders.notify(Event::ExerciseDeletedOk);
        }
        Msg::ExerciseDeleted(Err(message)) => {
            orders.notify(Event::ExerciseDeletedErr);
            model
                .errors
                .push(format!("Failed to delete exercise: {message}"));
        }

        Msg::SyncRoutines => {
            model.loading_routines += 1;
            let storage = model.cached_rest.clone();
            orders.skip().perform_cmd(async move {
                Msg::RoutinesSynced(storage.sync_routines().await.map_err(|err| err.to_string()))
            });
        }
        Msg::RoutinesSynced(Ok(routines)) => {
            let routines = routines.into_iter().map(|r| (r.id, r)).collect();
            if model.routines != routines {
                model.routines = routines;
                orders.notify(Event::DataChanged);
            }
            model.loading_routines -= 1;
            model.last_refresh.insert(DataSet::Routines, Utc::now());
        }
        Msg::RoutinesSynced(Err(message)) => {
            model.loading_routines -= 1;
            warn!("failed to sync routines: {message}");
        }
        Msg::ReadRoutines => {
            model.loading_routines += 1;
            let storage = model.cached_rest.clone();
            orders.skip().perform_cmd(async move {
                Msg::RoutinesRead(storage.read_routines().await.map_err(|err| err.to_string()))
            });
        }
        Msg::RoutinesRead(Ok(routines)) => {
            let routines = routines.into_iter().map(|r| (r.id, r)).collect();
            if model.routines != routines {
                model.routines = routines;
                orders.notify(Event::DataChanged);
            }
            model.loading_routines -= 1;
        }
        Msg::RoutinesRead(Err(message)) => {
            model
                .errors
                .push(format!("Failed to read routines: {message}"));
            model.loading_routines -= 1;
        }
        Msg::CreateRoutine(name, template_routine_id) => {
            let sections = if model.routines.contains_key(&template_routine_id) {
                model.routines[&template_routine_id].sections.clone()
            } else {
                vec![]
            };
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::RoutineCreated(
                    storage
                        .create_routine(name, sections)
                        .await
                        .map_err(|err| err.to_string()),
                )
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
                .push(format!("Failed to create routine: {message}"));
        }
        Msg::ModifyRoutine(id, name, archived, sections) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::RoutineModified(
                    storage
                        .modify_routine(id, name, archived, sections)
                        .await
                        .map_err(|err| err.to_string()),
                )
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
                .push(format!("Failed to modify routine: {message}"));
        }
        Msg::DeleteRoutine(id) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::RoutineDeleted(
                    storage
                        .delete_routine(id)
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
        }
        Msg::RoutineDeleted(Ok(id)) => {
            model.routines.remove(&id);
            orders.notify(Event::RoutineDeletedOk);
        }
        Msg::RoutineDeleted(Err(message)) => {
            orders.notify(Event::RoutineDeletedErr);
            model
                .errors
                .push(format!("Failed to delete routine: {message}"));
        }

        Msg::SyncTrainingSessions => {
            model.loading_training_sessions += 1;
            let storage = model.cached_rest.clone();
            orders.skip().perform_cmd(async move {
                Msg::TrainingSessionsSynced(
                    storage
                        .sync_training_sessions()
                        .await
                        .map_err(|err| err.to_string()),
                )
            });
        }
        Msg::TrainingSessionsSynced(Ok(training_sessions)) => {
            let training_sessions = training_sessions.into_iter().map(|t| (t.id, t)).collect();
            if model.training_sessions != training_sessions {
                model.training_sessions = training_sessions;
                model.training_stats =
                    domain::training_stats(&model.training_sessions.values().collect::<Vec<_>>());
                orders.notify(Event::DataChanged);
            }
            model.loading_training_sessions -= 1;
            model
                .last_refresh
                .insert(DataSet::TrainingSessions, Utc::now());
        }
        Msg::TrainingSessionsSynced(Err(message)) => {
            model.loading_training_sessions -= 1;
            warn!("failed to sync training sessions: {message}");
        }
        Msg::ReadTrainingSessions => {
            model.loading_training_sessions += 1;
            let storage = model.cached_rest.clone();
            orders.skip().perform_cmd(async move {
                Msg::TrainingSessionsRead(
                    storage
                        .read_training_sessions()
                        .await
                        .map_err(|err| err.to_string()),
                )
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
            model.loading_training_sessions -= 1;
        }
        Msg::TrainingSessionsRead(Err(message)) => {
            model
                .errors
                .push(format!("Failed to read training sessions: {message}"));
            model.loading_training_sessions -= 1;
        }
        Msg::CreateTrainingSession(routine_id, date, notes, elements) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::TrainingSessionCreated(
                    storage
                        .create_training_session(routine_id, date, notes, elements)
                        .await
                        .map_err(|err| err.to_string()),
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
                .push(format!("Failed to create training session: {message}"));
        }
        Msg::ModifyTrainingSession(id, notes, elements) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::TrainingSessionModified(
                    storage
                        .modify_training_session(id, notes, elements)
                        .await
                        .map_err(|err| err.to_string()),
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
                .push(format!("Failed to modify training session: {message}"));
        }
        Msg::DeleteTrainingSession(id) => {
            let storage = model.cached_rest.clone();
            orders.perform_cmd(async move {
                Msg::TrainingSessionDeleted(
                    storage
                        .delete_training_session(id)
                        .await
                        .map_err(|err| err.to_string()),
                )
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
                .push(format!("Failed to delete training session: {message}"));
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
            model.ongoing_training_session = Some(web_app::OngoingTrainingSession::new(
                training_session_id.as_u128(),
            ));
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
            let storage = model.local_storage.clone();
            orders
                .skip()
                .perform_cmd(async move { Msg::SettingsRead(storage.read_settings().await) });
        }
        Msg::SettingsRead(Ok(settings)) => {
            apply_theme(&settings.theme);
            model.settings = settings;
        }
        Msg::SettingsRead(Err(message)) => {
            error!("failed to read settings: {message}");
        }
        Msg::WriteSettings => {
            let settings = model.settings.clone();
            let storage = model.local_storage.clone();
            orders.skip().perform_cmd(async move {
                Msg::SettingsWritten(storage.write_settings(settings).await)
            });
        }
        Msg::SettingsWritten(result) => {
            if let Err(message) = result {
                error!("failed to write settings: {message}");
            }
        }

        Msg::ReadOngoingTrainingSession => {
            let storage = model.local_storage.clone();
            orders.skip().perform_cmd(async move {
                Msg::OngoingTrainingSessionRead(storage.read_ongoing_training_session().await)
            });
        }
        Msg::OngoingTrainingSessionRead(Ok(ongoing_training_session)) => {
            model.ongoing_training_session = ongoing_training_session;
        }
        Msg::OngoingTrainingSessionRead(Err(message)) => {
            error!("failed to read ongoing training session: {message}");
        }
        Msg::WriteOngoingTrainingSession => {
            let ongoing_training_session = model.ongoing_training_session.clone();
            let storage = model.local_storage.clone();
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
                error!("failed to write ongoing training session: {message}");
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
            (1.into(), routine(1)),
            (2.into(), routine(2)),
            (3.into(), routine(3)),
            (4.into(), routine(4)),
        ]);
        let training_sessions = BTreeMap::from([
            (
                1.into(),
                training_session(1, 3, NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            ),
            (
                2.into(),
                training_session(2, 2, NaiveDate::from_ymd_opt(2020, 3, 3).unwrap()),
            ),
            (
                3.into(),
                training_session(3, 3, NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
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
        let routines = BTreeMap::from([(1.into(), routine(1)), (2.into(), routine(2))]);
        let training_sessions = BTreeMap::from([
            (
                1.into(),
                training_session(1, 3, NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            ),
            (
                2.into(),
                training_session(2, 2, NaiveDate::from_ymd_opt(2020, 3, 3).unwrap()),
            ),
            (
                3.into(),
                training_session(3, 3, NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
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
            (1.into(), routine(1)),
            (2.into(), routine(2)),
            (3.into(), routine(3)),
            (4.into(), routine(4)),
        ]);
        let training_sessions = BTreeMap::from([
            (
                1.into(),
                training_session(1, 3, NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            ),
            (
                2.into(),
                training_session(2, 2, NaiveDate::from_ymd_opt(2020, 3, 3).unwrap()),
            ),
            (
                3.into(),
                training_session(3, 3, NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
            ),
        ]);
        assert_eq!(
            sort_routines_by_last_use(&routines, &training_sessions, |r| r.id > 2.into()),
            vec![routine(3), routine(4)]
        );
    }

    fn routine(id: u128) -> domain::Routine {
        domain::Routine {
            id: id.into(),
            name: domain::Name::new(&id.to_string()).unwrap(),
            notes: String::new(),
            archived: false,
            sections: vec![],
        }
    }

    fn training_session(id: u128, routine_id: u128, date: NaiveDate) -> domain::TrainingSession {
        domain::TrainingSession {
            id: id.into(),
            routine_id: domain::RoutineID::from(routine_id),
            date,
            notes: String::new(),
            elements: vec![],
        }
    }
}
