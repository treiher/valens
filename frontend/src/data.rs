use std::collections::{BTreeMap, BTreeSet};
use std::iter::zip;

use chrono::{prelude::*, Duration};
use gloo_storage::Storage;
use seed::{prelude::*, *};
use serde_json::{json, Map};

use crate::common;

const STORAGE_KEY_SETTINGS: &str = "settings";
const STORAGE_KEY_ONGOING_TRAINING_SESSION: &str = "ongoing training session";

// ------ ------
//     Init
// ------ ------

#[allow(clippy::needless_pass_by_value)]
pub fn init(url: Url, _orders: &mut impl Orders<Msg>) -> Model {
    let settings = gloo_storage::LocalStorage::get(STORAGE_KEY_SETTINGS).unwrap_or(Settings {
        beep_volume: 80,
        automatic_metronome: false,
        notifications: false,
    });
    let ongoing_training_session =
        gloo_storage::LocalStorage::get(STORAGE_KEY_ONGOING_TRAINING_SESSION).unwrap_or(None);
    Model {
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
            avg_rpe_per_week: Vec::new(),
            total_set_volume_per_week: Vec::new(),
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
    pub base_url: Url,
    errors: Vec<String>,
    app_update_available: bool,

    // ------ Data -----
    pub session: Option<Session>,
    pub version: String,
    pub users: BTreeMap<u32, User>,
    pub loading_users: bool,

    // ------ Session-dependent data ------
    pub body_weight: BTreeMap<NaiveDate, BodyWeight>,
    pub loading_body_weight: bool,
    pub body_fat: BTreeMap<NaiveDate, BodyFat>,
    pub loading_body_fat: bool,
    pub period: BTreeMap<NaiveDate, Period>,
    pub loading_period: bool,
    pub exercises: BTreeMap<u32, Exercise>,
    pub loading_exercises: bool,
    pub routines: BTreeMap<u32, Routine>,
    pub loading_routines: bool,
    pub training_sessions: BTreeMap<u32, TrainingSession>,
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
    pub fn routines_sorted_by_last_use(&self) -> Vec<Routine> {
        sort_routines_by_last_use(&self.routines, &self.training_sessions)
    }
}

fn sort_routines_by_last_use(
    routines: &BTreeMap<u32, Routine>,
    training_sessions: &BTreeMap<u32, TrainingSession>,
) -> Vec<Routine> {
    let mut map: BTreeMap<u32, NaiveDate> = BTreeMap::new();
    for routine_id in routines.keys() {
        map.insert(
            *routine_id,
            NaiveDate::MIN + Duration::days(i64::from(*routine_id)),
        );
    }
    for training_session in training_sessions.values() {
        if let Some(routine_id) = training_session.routine_id {
            if routines.contains_key(&routine_id) && training_session.date > map[&routine_id] {
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

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Session {
    pub id: u32,
    pub name: String,
    pub sex: u8,
}

#[derive(serde::Deserialize, Debug, Clone, PartialEq, Eq)]
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct BodyWeight {
    pub date: NaiveDate,
    pub weight: f32,
}

#[derive(Clone)]
pub struct BodyWeightStats {
    pub date: NaiveDate,
    pub avg_weight: Option<f32>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
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

pub struct TrainingStats {
    pub short_term_load: Vec<(NaiveDate, f32)>,
    pub long_term_load: Vec<(NaiveDate, f32)>,
    pub avg_rpe_per_week: Vec<(NaiveDate, f32)>,
    pub total_set_volume_per_week: Vec<(NaiveDate, f32)>,
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
        self.avg_rpe_per_week.clear();
        self.total_set_volume_per_week.clear();
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Exercise {
    pub id: u32,
    pub name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Routine {
    pub id: u32,
    pub name: String,
    pub notes: Option<String>,
    pub sections: Vec<RoutinePart>,
}

impl Routine {
    pub fn duration(&self) -> Duration {
        self.sections.iter().map(RoutinePart::duration).sum()
    }

    pub fn num_sets(&self) -> u32 {
        self.sections.iter().map(RoutinePart::num_sets).sum()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum RoutinePart {
    RoutineSection {
        rounds: u32,
        parts: Vec<RoutinePart>,
    },
    RoutineActivity {
        exercise_id: Option<u32>,
        reps: u32,
        time: u32,
        weight: f32,
        rpe: f32,
        automatic: bool,
    },
}

impl RoutinePart {
    pub fn duration(&self) -> Duration {
        match self {
            RoutinePart::RoutineSection { rounds, parts } => {
                parts.iter().map(RoutinePart::duration).sum::<Duration>()
                    * (*rounds).try_into().unwrap_or(1)
            }
            RoutinePart::RoutineActivity { reps, time, .. } => {
                let r = if *reps > 0 { *reps } else { 1 };
                let t = if *time > 0 { *time } else { 4 };
                Duration::seconds(i64::from(r * t))
            }
        }
    }

    pub fn num_sets(&self) -> u32 {
        match self {
            RoutinePart::RoutineSection { rounds, parts } => {
                parts.iter().map(RoutinePart::num_sets).sum::<u32>() * *rounds
            }
            RoutinePart::RoutineActivity { exercise_id, .. } => exercise_id.is_some().into(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct TrainingSession {
    pub id: u32,
    pub routine_id: Option<u32>,
    pub date: NaiveDate,
    pub notes: Option<String>,
    pub elements: Vec<TrainingSessionElement>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum TrainingSessionElement {
    Set {
        exercise_id: u32,
        reps: Option<u32>,
        time: Option<u32>,
        weight: Option<f32>,
        rpe: Option<f32>,
        target_reps: Option<u32>,
        target_time: Option<u32>,
        target_weight: Option<f32>,
        target_rpe: Option<f32>,
        automatic: bool,
    },
    Rest {
        target_time: Option<u32>,
        automatic: bool,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub beep_volume: u8,
    pub automatic_metronome: bool,
    pub notifications: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct OngoingTrainingSession {
    pub training_session_id: u32,
    pub start_time: DateTime<Utc>,
    pub section_idx: usize,
    pub section_start_time: DateTime<Utc>,
    pub timer_state: TimerState,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
pub enum TimerState {
    Unset,
    Active { target_time: DateTime<Utc> },
    Paused { time: i64 },
}

impl BodyFat {
    pub fn jp3(&self, sex: u8) -> Option<f32> {
        if sex == 0 {
            Some(Self::jackson_pollock(
                f32::from(self.tricep?) + f32::from(self.suprailiac?) + f32::from(self.tigh?),
                1.099_492_1,
                0.000_992_9,
                0.000_002_3,
                0.000_139_2,
            ))
        } else if sex == 1 {
            Some(Self::jackson_pollock(
                f32::from(self.chest?) + f32::from(self.abdominal?) + f32::from(self.tigh?),
                1.109_38,
                0.000_826_7,
                0.000_001_6,
                0.000_257_4,
            ))
        } else {
            None
        }
    }

    pub fn jp7(&self, sex: u8) -> Option<f32> {
        if sex == 0 {
            Some(Self::jackson_pollock(
                f32::from(self.chest?)
                    + f32::from(self.abdominal?)
                    + f32::from(self.tigh?)
                    + f32::from(self.tricep?)
                    + f32::from(self.subscapular?)
                    + f32::from(self.suprailiac?)
                    + f32::from(self.midaxillary?),
                1.097,
                0.000_469_71,
                0.000_000_56,
                0.000_128_28,
            ))
        } else if sex == 1 {
            Some(Self::jackson_pollock(
                f32::from(self.chest?)
                    + f32::from(self.abdominal?)
                    + f32::from(self.tigh?)
                    + f32::from(self.tricep?)
                    + f32::from(self.subscapular?)
                    + f32::from(self.suprailiac?)
                    + f32::from(self.midaxillary?),
                1.112,
                0.000_434_99,
                0.000_000_55,
                0.000_288_26,
            ))
        } else {
            None
        }
    }

    fn jackson_pollock(sum: f32, k0: f32, k1: f32, k2: f32, ka: f32) -> f32 {
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

impl TrainingSession {
    pub fn exercises(&self) -> BTreeSet<u32> {
        self.elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { exercise_id, .. } => Some(*exercise_id),
                _ => None,
            })
            .collect::<BTreeSet<_>>()
    }

    pub fn avg_reps(&self) -> Option<f32> {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { reps, .. } => *reps,
                _ => None,
            })
            .collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            Some(sets.iter().sum::<u32>() as f32 / sets.len() as f32)
        }
    }

    pub fn avg_time(&self) -> Option<f32> {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { time, .. } => *time,
                _ => None,
            })
            .collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            Some(sets.iter().sum::<u32>() as f32 / sets.len() as f32)
        }
    }

    pub fn avg_weight(&self) -> Option<f32> {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { weight, .. } => *weight,
                _ => None,
            })
            .collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            Some(sets.iter().sum::<f32>() / sets.len() as f32)
        }
    }

    pub fn avg_rpe(&self) -> Option<f32> {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { rpe, .. } => *rpe,
                _ => None,
            })
            .collect::<Vec<_>>();
        if sets.is_empty() {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            Some(sets.iter().sum::<f32>() / sets.len() as f32)
        }
    }

    pub fn load(&self) -> u32 {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set {
                    reps, time, rpe, ..
                } => Some(if let Some(rpe) = *rpe {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    if rpe > 5.0 {
                        (2.0_f32).powf(rpe - 5.0).round() as u32
                    } else {
                        1
                    }
                } else {
                    u32::from(reps.is_some() || time.is_some())
                }),
                _ => None,
            })
            .collect::<Vec<_>>();
        sets.iter().sum::<u32>()
    }

    pub fn set_volume(&self) -> u32 {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { rpe, .. } => {
                    if rpe.unwrap_or(0.0) >= 7.0 {
                        Some(1)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        sets.iter().sum::<u32>()
    }

    pub fn volume_load(&self) -> u32 {
        let sets = &self
            .elements
            .iter()
            .filter_map(|e| match e {
                TrainingSessionElement::Set { reps, weight, .. } => {
                    if let Some(reps) = reps {
                        #[allow(
                            clippy::cast_possible_truncation,
                            clippy::cast_precision_loss,
                            clippy::cast_sign_loss
                        )]
                        if let Some(weight) = weight {
                            Some((*reps as f32 * weight).round() as u32)
                        } else {
                            Some(*reps)
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        sets.iter().sum::<u32>()
    }

    pub fn tut(&self) -> u32 {
        let sets = &self
            .elements
            .iter()
            .map(|e| match e {
                TrainingSessionElement::Set { reps, time, .. } => {
                    reps.unwrap_or(1) * time.unwrap_or(0)
                }
                _ => 0,
            })
            .collect::<Vec<_>>();
        sets.iter().sum::<u32>()
    }
}

impl OngoingTrainingSession {
    pub fn new(training_session_id: u32) -> OngoingTrainingSession {
        OngoingTrainingSession {
            training_session_id,
            start_time: Utc::now(),
            section_idx: 0,
            section_start_time: Utc::now(),
            timer_state: TimerState::Unset,
        }
    }
}

fn calculate_body_weight_stats(
    body_weight: &BTreeMap<NaiveDate, BodyWeight>,
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

fn determine_cycles(period: &BTreeMap<NaiveDate, Period>) -> Vec<Cycle> {
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

fn calculate_training_stats(training_sessions: &[&TrainingSession]) -> TrainingStats {
    let short_term_load = calculate_weighted_sum_of_load(training_sessions, 7);
    let long_term_load = calculate_average_weighted_sum_of_load(&short_term_load, 28);
    TrainingStats {
        short_term_load,
        long_term_load,
        total_set_volume_per_week: calculate_total_set_volume_per_week(training_sessions),
        avg_rpe_per_week: calculate_avg_rpe_per_week(training_sessions),
    }
}

fn calculate_weighted_sum_of_load(
    training_sessions: &[&TrainingSession],
    window_size: usize,
) -> Vec<(NaiveDate, f32)> {
    let mut result: BTreeMap<NaiveDate, f32> = BTreeMap::new();

    let today = Local::now().date_naive();
    let mut day = training_sessions.get(0).map_or(today, |t| t.date);
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

fn calculate_total_set_volume_per_week(
    training_sessions: &[&TrainingSession],
) -> Vec<(NaiveDate, f32)> {
    let mut result: BTreeMap<NaiveDate, f32> = BTreeMap::new();

    let today = Local::now().date_naive();
    let mut day = training_sessions.get(0).map_or(today, |t| t.date);
    while day <= today.week(Weekday::Mon).last_day() {
        result.insert(day.week(Weekday::Mon).last_day(), 0.0);
        day += Duration::days(7);
    }

    #[allow(clippy::cast_precision_loss)]
    for t in training_sessions {
        result
            .entry(t.date.week(Weekday::Mon).last_day())
            .and_modify(|e| *e += t.set_volume() as f32);
    }

    result.into_iter().collect()
}

fn calculate_avg_rpe_per_week(training_sessions: &[&TrainingSession]) -> Vec<(NaiveDate, f32)> {
    let mut result: BTreeMap<NaiveDate, Vec<f32>> = BTreeMap::new();

    let today = Local::now().date_naive();
    let mut day = training_sessions.get(0).map_or(today, |t| t.date);
    while day <= today.week(Weekday::Mon).last_day() {
        result.insert(day.week(Weekday::Mon).last_day(), vec![]);
        day += Duration::days(7);
    }

    for t in training_sessions {
        if let Some(avg_rpe) = t.avg_rpe() {
            result
                .entry(t.date.week(Weekday::Mon).last_day())
                .and_modify(|e| e.push(avg_rpe));
        }
    }

    #[allow(clippy::cast_precision_loss)]
    result
        .into_iter()
        .map(|(date, values)| {
            (
                date,
                if not(values.is_empty()) {
                    values.iter().sum::<f32>() / values.len() as f32
                } else {
                    0.0
                },
            )
        })
        .collect()
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
    UserDeleted(Result<u32, String>),

    ReadBodyWeight,
    BodyWeightRead(Result<Vec<BodyWeight>, String>),
    CreateBodyWeight(BodyWeight),
    BodyWeightCreated(Result<BodyWeight, String>),
    ReplaceBodyWeight(BodyWeight),
    BodyWeightReplaced(Result<BodyWeight, String>),
    DeleteBodyWeight(NaiveDate),
    BodyWeightDeleted(Result<NaiveDate, String>),

    ReadBodyFat,
    BodyFatRead(Result<Vec<BodyFat>, String>),
    CreateBodyFat(BodyFat),
    BodyFatCreated(Result<BodyFat, String>),
    ReplaceBodyFat(BodyFat),
    BodyFatReplaced(Result<BodyFat, String>),
    DeleteBodyFat(NaiveDate),
    BodyFatDeleted(Result<NaiveDate, String>),

    ReadPeriod,
    PeriodRead(Result<Vec<Period>, String>),
    CreatePeriod(Period),
    PeriodCreated(Result<Period, String>),
    ReplacePeriod(Period),
    PeriodReplaced(Result<Period, String>),
    DeletePeriod(NaiveDate),
    PeriodDeleted(Result<NaiveDate, String>),

    ReadExercises,
    ExercisesRead(Result<Vec<Exercise>, String>),
    CreateExercise(String),
    ExerciseCreated(Result<Exercise, String>),
    ReplaceExercise(Exercise),
    ExerciseReplaced(Result<Exercise, String>),
    DeleteExercise(u32),
    ExerciseDeleted(Result<u32, String>),

    ReadRoutines,
    RoutinesRead(Result<Vec<Routine>, String>),
    CreateRoutine(String, u32),
    RoutineCreated(Result<Routine, String>),
    ModifyRoutine(u32, Option<String>, Option<Vec<RoutinePart>>),
    RoutineModified(Result<Routine, String>),
    DeleteRoutine(u32),
    RoutineDeleted(Result<u32, String>),

    ReadTrainingSessions,
    TrainingSessionsRead(Result<Vec<TrainingSession>, String>),
    CreateTrainingSession(u32, NaiveDate, String, Vec<TrainingSessionElement>),
    TrainingSessionCreated(Result<TrainingSession, String>),
    ModifyTrainingSession(u32, Option<String>, Option<Vec<TrainingSessionElement>>),
    TrainingSessionModified(Result<TrainingSession, String>),
    DeleteTrainingSession(u32),
    TrainingSessionDeleted(Result<u32, String>),

    SetBeepVolume(u8),
    SetAutomaticMetronome(bool),
    SetNotifications(bool),

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
                Ok(_) => Url::reload(),
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
                crate::Urls::new(&model.base_url.clone().set_hash_path([""; 0])).home(),
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
                        (),
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
            orders.perform_cmd(async { fetch("api/users", Msg::UsersRead).await });
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
            orders.perform_cmd(async move {
                fetch_no_content(
                    Request::new(format!("api/users/{id}")).method(Method::Delete),
                    Msg::UserDeleted,
                    id,
                )
                .await
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
                .push("Failed to delete user: ".to_owned() + &message);
        }

        Msg::ReadBodyWeight => {
            model.loading_body_weight = true;
            orders.skip().perform_cmd(async {
                fetch("api/body_weight?format=statistics", Msg::BodyWeightRead).await
            });
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
            orders.perform_cmd(async move {
                fetch_no_content(
                    Request::new(format!("api/body_weight/{date}")).method(Method::Delete),
                    Msg::BodyWeightDeleted,
                    date,
                )
                .await
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
            orders.skip().perform_cmd(async {
                fetch("api/body_fat?format=statistics", Msg::BodyFatRead).await
            });
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
            orders.perform_cmd(async move {
                fetch_no_content(
                    Request::new(format!("api/body_fat/{date}")).method(Method::Delete),
                    Msg::BodyFatDeleted,
                    date,
                )
                .await
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
                .push("Failed to delete body fat: ".to_owned() + &message);
        }

        Msg::ReadPeriod => {
            model.loading_period = true;
            orders
                .skip()
                .perform_cmd(async { fetch("api/period", Msg::PeriodRead).await });
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
            orders.perform_cmd(async move {
                fetch_no_content(
                    Request::new(format!("api/period/{date}")).method(Method::Delete),
                    Msg::PeriodDeleted,
                    date,
                )
                .await
            });
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
            orders
                .skip()
                .perform_cmd(async { fetch("api/exercises", Msg::ExercisesRead).await });
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
        Msg::ExerciseReplaced(Ok(exercise)) => {
            model.exercises.insert(exercise.id, exercise);
            orders.notify(Event::ExerciseReplacedOk);
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
                    Request::new(format!("api/exercises/{id}")).method(Method::Delete),
                    Msg::ExerciseDeleted,
                    id,
                )
                .await
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
                .push("Failed to delete exercise: ".to_owned() + &message);
        }

        Msg::ReadRoutines => {
            model.loading_routines = true;
            orders
                .skip()
                .perform_cmd(async { fetch("api/routines", Msg::RoutinesRead).await });
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
        Msg::CreateRoutine(routine_name, template_routine_id) => {
            let sections = if model.routines.contains_key(&template_routine_id) {
                json!(model.routines[&template_routine_id].sections)
            } else {
                json!([])
            };
            orders.perform_cmd(async move {
                fetch(
                    Request::new("api/routines")
                        .method(Method::Post)
                        .json(&json!({
                            "name": routine_name,
                            "notes": "",
                            "sections": sections
                        }))
                        .expect("serialization failed"),
                    Msg::RoutineCreated,
                )
                .await
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
                    Request::new(format!("api/routines/{id}"))
                        .method(Method::Patch)
                        .json(&content)
                        .expect("serialization failed"),
                    Msg::RoutineModified,
                )
                .await
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
            orders.perform_cmd(async move {
                fetch_no_content(
                    Request::new(format!("api/routines/{id}")).method(Method::Delete),
                    Msg::RoutineDeleted,
                    id,
                )
                .await
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
                .push("Failed to delete routine: ".to_owned() + &message);
        }

        Msg::ReadTrainingSessions => {
            model.loading_training_sessions = true;
            orders
                .skip()
                .perform_cmd(async { fetch("api/workouts", Msg::TrainingSessionsRead).await });
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
            orders.perform_cmd(async move {
                fetch(
                    Request::new("api/workouts")
                        .method(Method::Post)
                        .json(&json!({
                            "routine_id": routine_id,
                            "date": date,
                            "notes": notes,
                            "elements": elements
                        }))
                        .expect("serialization failed"),
                    Msg::TrainingSessionCreated,
                )
                .await
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
            let mut content = Map::new();
            if let Some(notes) = notes {
                content.insert("notes".into(), json!(notes));
            }
            if let Some(elements) = elements {
                content.insert("elements".into(), json!(elements));
            }
            orders.perform_cmd(async move {
                fetch(
                    Request::new(format!("api/workouts/{id}"))
                        .method(Method::Patch)
                        .json(&content)
                        .expect("serialization failed"),
                    Msg::TrainingSessionModified,
                )
                .await
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
            orders.perform_cmd(async move {
                fetch_no_content(
                    Request::new(format!("api/workouts/{id}")).method(Method::Delete),
                    Msg::TrainingSessionDeleted,
                    id,
                )
                .await
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
        Msg::SetAutomaticMetronome(value) => {
            model.settings.automatic_metronome = value;
            local_storage_set(STORAGE_KEY_SETTINGS, &model.settings, &mut model.errors);
        }
        Msg::SetNotifications(value) => {
            model.settings.notifications = value;
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
                ongoing_training_session.section_idx = section_idx;
                ongoing_training_session.section_start_time = Utc::now();
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
                Err(error) => message(Err(format!("deserialization failed: {error:?}"))),
            },
            Err(error) => message(Err(format!("unexpected response: {error:?}"))),
        },
        Err(_) => message(Err("no connection".into())),
    }
}

async fn fetch_no_content<'a, Ms, T>(
    request: impl Into<Request<'a>>,
    message: fn(Result<T, String>) -> Ms,
    id: T,
) -> Ms {
    match seed::browser::fetch::fetch(request).await {
        Ok(response) => match response.check_status() {
            Ok(_) => message(Ok(id)),
            Err(error) => message(Err(format!("unexpected response: {error:?}"))),
        },
        Err(_) => message(Err("no connection".into())),
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
            sort_routines_by_last_use(&routines, &training_sessions),
            vec![routine(2), routine(3), routine(4), routine(1)]
        );
    }

    #[test]
    fn test_sort_routines_by_last_use_empty() {
        let routines = BTreeMap::new();
        let training_sessions = BTreeMap::new();
        assert_eq!(
            sort_routines_by_last_use(&routines, &training_sessions),
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
            sort_routines_by_last_use(&routines, &training_sessions),
            vec![routine(2), routine(1)]
        );
    }

    fn routine(id: u32) -> Routine {
        Routine {
            id,
            name: id.to_string(),
            notes: None,
            sections: vec![],
        }
    }

    fn training_session(id: u32, routine_id: Option<u32>, date: NaiveDate) -> TrainingSession {
        TrainingSession {
            id,
            routine_id,
            date,
            notes: None,
            elements: vec![],
        }
    }
}
