use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    iter::zip,
    slice::Iter,
};

use chrono::{Days, Duration, Local, NaiveDate};

#[derive(serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub sex: u8,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Exercise {
    pub id: u32,
    pub name: String,
    pub muscles: Vec<ExerciseMuscle>,
}

impl Exercise {
    pub fn muscle_stimulus(&self) -> BTreeMap<u8, u8> {
        self.muscles
            .iter()
            .map(|m| (m.muscle_id, m.stimulus))
            .collect()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ExerciseMuscle {
    pub muscle_id: u8,
    pub stimulus: u8,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Routine {
    pub id: u32,
    pub name: String,
    pub notes: Option<String>,
    pub archived: bool,
    pub sections: Vec<RoutinePart>,
}

impl Routine {
    pub fn duration(&self) -> Duration {
        self.sections.iter().map(RoutinePart::duration).sum()
    }

    pub fn num_sets(&self) -> u32 {
        self.sections.iter().map(RoutinePart::num_sets).sum()
    }

    pub fn stimulus_per_muscle(&self, exercises: &BTreeMap<u32, Exercise>) -> BTreeMap<u8, u32> {
        let mut result: BTreeMap<u8, u32> = Muscle::iter().map(|m| (m.id(), 0)).collect();
        for section in &self.sections {
            for (id, stimulus) in section.stimulus_per_muscle(exercises) {
                if result.contains_key(&id) {
                    *result.entry(id).or_insert(0) += stimulus;
                }
            }
        }
        result
    }

    pub fn exercises(&self) -> BTreeSet<u32> {
        self.sections
            .iter()
            .flat_map(RoutinePart::exercises)
            .collect::<BTreeSet<_>>()
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

    pub fn stimulus_per_muscle(&self, exercises: &BTreeMap<u32, Exercise>) -> BTreeMap<u8, u32> {
        match self {
            RoutinePart::RoutineSection { rounds, parts } => {
                let mut result: BTreeMap<u8, u32> = BTreeMap::new();
                for part in parts {
                    for (id, stimulus) in part.stimulus_per_muscle(exercises) {
                        *result.entry(id).or_insert(0) += stimulus * rounds;
                    }
                }
                result
            }
            RoutinePart::RoutineActivity { exercise_id, .. } => exercises
                .get(&exercise_id.unwrap_or_default())
                .map(|e| {
                    e.muscle_stimulus()
                        .iter()
                        .map(|(id, stimulus)| (*id, u32::from(*stimulus)))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct TrainingSession {
    pub id: u32,
    pub routine_id: Option<u32>,
    pub date: NaiveDate,
    pub notes: Option<String>,
    pub elements: Vec<TrainingSessionElement>,
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
                TrainingSessionElement::Set {
                    reps, time, rpe, ..
                } => {
                    if rpe.unwrap_or(10.0) >= 7.0 {
                        Some(u32::from(reps.is_some() || time.is_some()))
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

    pub fn tut(&self) -> Option<u32> {
        let sets = &self
            .elements
            .iter()
            .map(|e| match e {
                TrainingSessionElement::Set { reps, time, .. } => {
                    time.as_ref().map(|v| reps.unwrap_or(1) * v)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if sets.iter().all(Option::is_none) {
            return None;
        }
        Some(sets.iter().filter_map(|e| *e).sum::<u32>())
    }

    pub fn stimulus_per_muscle(&self, exercises: &BTreeMap<u32, Exercise>) -> BTreeMap<u8, u32> {
        let mut result: BTreeMap<u8, u32> = BTreeMap::new();
        for element in &self.elements {
            if let TrainingSessionElement::Set {
                exercise_id,
                reps,
                time,
                rpe,
                ..
            } = element
            {
                if reps.is_none() && time.is_none() {
                    continue;
                }
                if let Some(rpe) = rpe {
                    if *rpe < 7.0 {
                        continue;
                    }
                }
                if let Some(exercise) = exercises.get(exercise_id) {
                    for (id, stimulus) in &exercise.muscle_stimulus() {
                        *result.entry(*id).or_insert(0) += u32::from(*stimulus);
                    }
                }
            }
        }
        result
    }
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

#[cfg_attr(test, derive(Debug, PartialEq))]
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

pub fn training_stats(training_sessions: &[&TrainingSession]) -> TrainingStats {
    let short_term_load = weighted_sum_of_load(training_sessions, 7);
    let long_term_load = average_weighted_sum_of_load(&short_term_load, 28);
    TrainingStats {
        short_term_load,
        long_term_load,
    }
}

fn weighted_sum_of_load(
    training_sessions: &[&TrainingSession],
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

fn average_weighted_sum_of_load(
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

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub enum Muscle {
    // Neck
    Neck = 1,
    // Chest
    Pecs = 11,
    // Back
    Traps = 21,
    Lats = 22,
    // Shoulders
    FrontDelts = 31,
    SideDelts = 32,
    RearDelts = 33,
    // Upper arms
    Biceps = 41,
    Triceps = 42,
    // Forearms
    Forearms = 51,
    // Waist
    Abs = 61,
    ErectorSpinae = 62,
    // Hips
    Glutes = 71,
    Abductors = 72,
    // Thighs
    Quads = 81,
    Hamstrings = 82,
    Adductors = 83,
    // Calves
    Calves = 91,
}

impl Muscle {
    pub fn iter() -> Iter<'static, Muscle> {
        static MUSCLES: [Muscle; 18] = [
            Muscle::Neck,
            Muscle::Pecs,
            Muscle::Traps,
            Muscle::Lats,
            Muscle::FrontDelts,
            Muscle::SideDelts,
            Muscle::RearDelts,
            Muscle::Biceps,
            Muscle::Triceps,
            Muscle::Forearms,
            Muscle::Abs,
            Muscle::ErectorSpinae,
            Muscle::Glutes,
            Muscle::Abductors,
            Muscle::Quads,
            Muscle::Hamstrings,
            Muscle::Adductors,
            Muscle::Calves,
        ];
        MUSCLES.iter()
    }

    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn from_repr(repr: u8) -> Option<Muscle> {
        match repr {
            x if x == Muscle::Neck as u8 => Some(Muscle::Neck),
            x if x == Muscle::Pecs as u8 => Some(Muscle::Pecs),
            x if x == Muscle::Traps as u8 => Some(Muscle::Traps),
            x if x == Muscle::Lats as u8 => Some(Muscle::Lats),
            x if x == Muscle::FrontDelts as u8 => Some(Muscle::FrontDelts),
            x if x == Muscle::SideDelts as u8 => Some(Muscle::SideDelts),
            x if x == Muscle::RearDelts as u8 => Some(Muscle::RearDelts),
            x if x == Muscle::Biceps as u8 => Some(Muscle::Biceps),
            x if x == Muscle::Triceps as u8 => Some(Muscle::Triceps),
            x if x == Muscle::Forearms as u8 => Some(Muscle::Forearms),
            x if x == Muscle::Abs as u8 => Some(Muscle::Abs),
            x if x == Muscle::ErectorSpinae as u8 => Some(Muscle::ErectorSpinae),
            x if x == Muscle::Glutes as u8 => Some(Muscle::Glutes),
            x if x == Muscle::Abductors as u8 => Some(Muscle::Abductors),
            x if x == Muscle::Quads as u8 => Some(Muscle::Quads),
            x if x == Muscle::Hamstrings as u8 => Some(Muscle::Hamstrings),
            x if x == Muscle::Adductors as u8 => Some(Muscle::Adductors),
            x if x == Muscle::Calves as u8 => Some(Muscle::Calves),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Muscle::Neck => "Neck",
            Muscle::Pecs => "Pecs",
            Muscle::Traps => "Traps",
            Muscle::Lats => "Lats",
            Muscle::FrontDelts => "Front Delts",
            Muscle::SideDelts => "Side Delts",
            Muscle::RearDelts => "Rear Delts",
            Muscle::Biceps => "Biceps",
            Muscle::Triceps => "Triceps",
            Muscle::Forearms => "Forearms",
            Muscle::Abs => "Abs",
            Muscle::ErectorSpinae => "Erector Spinae",
            Muscle::Glutes => "Glutes",
            Muscle::Abductors => "Abductors",
            Muscle::Quads => "Quads",
            Muscle::Hamstrings => "Hamstrings",
            Muscle::Adductors => "Adductors",
            Muscle::Calves => "Calves",
        }
    }

    pub fn description(self) -> &'static str {
        #[allow(clippy::match_same_arms)]
        match self {
            Muscle::Neck => "",
            Muscle::Pecs => "Chest",
            Muscle::Traps => "Upper back",
            Muscle::Lats => "Sides of back",
            Muscle::FrontDelts => "Anterior shoulders",
            Muscle::SideDelts => "Mid shoulders",
            Muscle::RearDelts => "Posterior shoulders",
            Muscle::Biceps => "Front of upper arms",
            Muscle::Triceps => "Back of upper arms",
            Muscle::Forearms => "",
            Muscle::Abs => "Belly",
            Muscle::ErectorSpinae => "Lower back and spine",
            Muscle::Glutes => "Buttocks",
            Muscle::Abductors => "Outside of hips",
            Muscle::Quads => "Front of thighs",
            Muscle::Hamstrings => "Back of thighs",
            Muscle::Adductors => "Inner thighs",
            Muscle::Calves => "Back of lower legs",
        }
    }
}

#[derive(Default, PartialEq)]
pub struct ExerciseFilter {
    pub muscles: HashSet<Muscle>,
}

impl ExerciseFilter {
    pub fn is_empty(&self) -> bool {
        self.muscles.is_empty()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct BodyWeight {
    pub date: NaiveDate,
    pub weight: f32,
}

pub fn avg_body_weight(
    body_weight: &BTreeMap<NaiveDate, BodyWeight>,
) -> BTreeMap<NaiveDate, BodyWeight> {
    let data = body_weight
        .values()
        .map(|bw| (bw.date, bw.weight))
        .collect::<Vec<_>>();
    value_based_centered_moving_average(&data, 4)
        .into_iter()
        .map(|(date, weight)| (date, BodyWeight { date, weight }))
        .collect()
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BodyFat {
    pub date: NaiveDate,
    pub chest: Option<u8>,
    pub abdominal: Option<u8>,
    pub thigh: Option<u8>,
    pub tricep: Option<u8>,
    pub subscapular: Option<u8>,
    pub suprailiac: Option<u8>,
    pub midaxillary: Option<u8>,
}

impl BodyFat {
    pub fn jp3(&self, sex: u8) -> Option<f32> {
        if sex == 0 {
            Some(Self::jackson_pollock(
                f32::from(self.tricep?) + f32::from(self.suprailiac?) + f32::from(self.thigh?),
                1.099_492_1,
                0.000_992_9,
                0.000_002_3,
                0.000_139_2,
            ))
        } else if sex == 1 {
            Some(Self::jackson_pollock(
                f32::from(self.chest?) + f32::from(self.abdominal?) + f32::from(self.thigh?),
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
                    + f32::from(self.thigh?)
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
                    + f32::from(self.thigh?)
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Period {
    pub date: NaiveDate,
    pub intensity: u8,
}

pub fn cycles(period: &BTreeMap<NaiveDate, Period>) -> Vec<Cycle> {
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

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct Cycle {
    pub begin: NaiveDate,
    pub length: Duration,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct CurrentCycle {
    pub begin: NaiveDate,
    pub time_left: Duration,
    pub time_left_variation: Duration,
}

pub fn current_cycle(cycles: &[Cycle]) -> Option<CurrentCycle> {
    if cycles.is_empty() {
        return None;
    }

    let today = Local::now().date_naive();
    let cycles = cycles
        .iter()
        .filter(|c| (c.begin >= today - Duration::days(182) && c.begin <= today))
        .collect::<Vec<_>>();
    let stats = cycle_stats(&cycles);

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

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct CycleStats {
    pub length_median: Duration,
    pub length_variation: Duration,
}

pub fn cycle_stats(cycles: &[&Cycle]) -> CycleStats {
    let mut cycle_lengths = cycles.iter().map(|c| c.length).collect::<Vec<_>>();
    cycle_lengths.sort();
    CycleStats {
        length_median: quartile(&cycle_lengths, Quartile::Q2),
        length_variation: (quartile(&cycle_lengths, Quartile::Q3)
            - quartile(&cycle_lengths, Quartile::Q1))
            / 2,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Quartile {
    Q1 = 1,
    Q2 = 2,
    Q3 = 3,
}

pub fn quartile(durations: &[Duration], quartile_num: Quartile) -> Duration {
    if durations.is_empty() {
        return Duration::days(0);
    }
    let idx = durations.len() / 2;
    match quartile_num {
        Quartile::Q1 => quartile(&durations[..idx], Quartile::Q2),
        Quartile::Q2 => {
            if durations.len() % 2 == 0 {
                (durations[idx - 1] + durations[idx]) / 2
            } else {
                durations[idx]
            }
        }
        Quartile::Q3 => {
            if durations.len() % 2 == 0 {
                quartile(&durations[idx..], Quartile::Q2)
            } else {
                quartile(&durations[idx + 1..], Quartile::Q2)
            }
        }
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Interval {
    pub first: NaiveDate,
    pub last: NaiveDate,
}

impl From<std::ops::RangeInclusive<NaiveDate>> for Interval {
    fn from(value: std::ops::RangeInclusive<NaiveDate>) -> Self {
        Interval {
            first: *value.start(),
            last: *value.end(),
        }
    }
}

impl Interval {
    pub fn since(days: i64) -> Self {
        let today = Local::now().date_naive();
        Interval {
            first: today - Duration::days(days),
            last: today,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum DefaultInterval {
    All,
    _1Y = 365,
    _6M = 182,
    _3M = 91,
    _1M = 30,
}

pub fn init_interval(dates: &[NaiveDate], default_interval: DefaultInterval) -> Interval {
    let today = Local::now().date_naive();
    let mut first = dates.iter().copied().min().unwrap_or(today);
    let mut last = dates.iter().copied().max().unwrap_or(today);

    if default_interval != DefaultInterval::All
        && last >= today - Duration::days(default_interval as i64)
    {
        first = today - Duration::days(default_interval as i64);
    };

    last = today;

    Interval { first, last }
}

// Calculate the one-rep max for a given exercise and interval.
//
// The formula by Epley is used for 1RM calculation. The function returns a tuple
// of minimum and maximum 1RM (min, max) for each date.
pub fn one_rep_max_values(
    training_sessions: &[&TrainingSession],
    exercise_id: u32,
    interval: &Interval,
) -> BTreeMap<NaiveDate, (f32, f32)> {
    group_days(
        &training_sessions
            .iter()
            .flat_map(|s| {
                s.elements
                    .iter()
                    .filter_map(|e| match e {
                        TrainingSessionElement::Set {
                            exercise_id: id,
                            reps,
                            weight,
                            rpe,
                            ..
                        } if *id == exercise_id
                            && interval.first <= s.date
                            && s.date <= interval.last =>
                        {
                            reps.map_or(None, |reps| {
                                weight.map_or(None, |weight| {
                                    #[allow(clippy::cast_precision_loss)]
                                    Some((
                                        s.date,
                                        // Epley, B. (1985). “Poundage Chart”. Boyd Epley Workout.
                                        // Lincoln, NE: Body Enterprises.
                                        weight * (1. + (reps as f32 + (10. - rpe.unwrap_or(10.))) / 30.),
                                    ))
                                })
                            })
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
        |x| {
            let min = x
                .iter()
                .reduce(|acc, val| if *val < *acc { val } else { acc });
            let max = x
                .iter()
                .reduce(|acc, val| if *val < *acc { acc } else { val });
            min.and_then(|min| max.map(|max| (*min, *max)))
        },
    )
}

// Group a series of (date, value) pairs by day.
//
/// The `group_day` is called to combine values of the *same* day.
pub fn group_days<E: Copy, F: Copy>(
    data: &Vec<(NaiveDate, E)>,
    group_day: impl Fn(Vec<E>) -> Option<F>,
) -> BTreeMap<NaiveDate, F> {
    let mut date_map: BTreeMap<NaiveDate, Vec<E>> = BTreeMap::new();

    for (date, value) in data {
        date_map.entry(*date).or_default().push(*value);
    }

    let mut grouped: BTreeMap<NaiveDate, F> = BTreeMap::new();

    for (date, values) in date_map {
        if let Some(result) = group_day(values) {
            grouped.insert(date, result);
        }
    }

    grouped
}

/// Group a series of (date, value) pairs.
///
/// The `radius` parameter determines the number of days before and after the
/// center value to include in the calculation.
///
/// Only values which have a date within `interval` are used as a center value
/// for the calculation. Values outside the interval are included in the
/// calculation if they fall within the radius of a center value.
///
/// Two user-provided functions determine how values are combined:
///
///  - `group_day` is called to combine values of the *same* day.
///  - `group_range` is called to combine values of multiple days after all
///     values for the same day have been combined by `group_day`.
///
/// Return `None` in those functions to indicate the absence of a value.
///
pub fn centered_moving_grouping<E: Copy, F: Copy>(
    data: &Vec<(NaiveDate, E)>,
    interval: &Interval,
    radius: u64,
    group_day: impl Fn(Vec<E>) -> Option<F>,
    group_range: impl Fn(Vec<F>) -> Option<f32>,
) -> Vec<Vec<(NaiveDate, f32)>> {
    let grouped = group_days(data, group_day);
    interval
        .first
        .iter_days()
        .take_while(|d| *d <= interval.last)
        .fold(
            vec![vec![]],
            |mut result: Vec<Vec<(NaiveDate, f32)>>, center| {
                let value = group_range(
                    center
                        .checked_sub_days(Days::new(radius))
                        .unwrap_or(center)
                        .iter_days()
                        .take_while(|d| {
                            *d <= interval.last
                                && *d
                                    <= center.checked_add_days(Days::new(radius)).unwrap_or(center)
                        })
                        .filter_map(|d| grouped.get(&d))
                        .copied()
                        .collect::<Vec<_>>(),
                );
                if let Some(last) = result.last_mut() {
                    match value {
                        Some(v) => {
                            last.push((center, v));
                        }
                        None => {
                            if !last.is_empty() {
                                result.push(vec![]);
                            }
                        }
                    }
                }
                result
            },
        )
        .into_iter()
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
}

/// Calculate a series of moving totals from a given series of (date, value) pairs.
///
/// The radius argument determines the number of days to include into the calculated
/// total before and after each value within the interval.
///
/// Multiple values for the same date will be summed up.
///
/// An empty result vector may be returned if there is no data within the interval.
pub fn centered_moving_total(
    data: &Vec<(NaiveDate, f32)>,
    interval: &Interval,
    radius: u64,
) -> Vec<(NaiveDate, f32)> {
    centered_moving_grouping(
        data,
        interval,
        radius,
        |d| Some(d.iter().sum()),
        |d| Some(d.iter().sum()),
    )[0]
    .clone()
}

/// Calculate a series of moving averages from a given series of (date, value) pairs.
///
/// The radius argument determines the number of days to include into the calculated
/// average before and after each value within the interval.
///
/// Multiple values for the same date will be averaged.
///
/// An empty result vector may be returned if there is no data within the interval.
/// Multiple result vectors may be returned in cases where there are gaps of more than
/// 2*radius+1 days in the input data within the interval.
pub fn centered_moving_average(
    data: &Vec<(NaiveDate, f32)>,
    interval: &Interval,
    radius: u64,
) -> Vec<Vec<(NaiveDate, f32)>> {
    #[allow(clippy::cast_precision_loss)]
    centered_moving_grouping(
        data,
        interval,
        radius,
        |d| Some(d.iter().sum::<f32>() / d.len() as f32),
        |d| {
            if d.is_empty() {
                None
            } else {
                Some(d.iter().sum::<f32>() / d.len() as f32)
            }
        },
    )
}

/// Calculate a series of moving averages from a given series of (date, value) pairs.
///
/// The data argument must have only one value per day.
///
/// The radius argument determines the number of values to include into the calculated
/// average before and after each value.
pub fn value_based_centered_moving_average(
    data: &[(NaiveDate, f32)],
    radius: usize,
) -> Vec<(NaiveDate, f32)> {
    let window = 2 * radius + 1;
    let length = data.len();
    data.iter()
        .enumerate()
        .map(|(i, (date, _))| {
            #[allow(clippy::cast_precision_loss)]
            let avg = data[i.saturating_sub(window / 2)..=(i + window / 2).min(length - 1)]
                .iter()
                .map(|(_, value)| value)
                .sum::<f32>()
                / window
                    .min(length - (i.saturating_sub(window / 2)))
                    .min(i + window / 2 + 1) as f32;
            (*date, avg)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use serde_json::json;

    static TODAY: std::sync::LazyLock<NaiveDate> =
        std::sync::LazyLock::new(|| Local::now().date_naive());

    static EXERCISES: std::sync::LazyLock<BTreeMap<u32, Exercise>> =
        std::sync::LazyLock::new(|| {
            BTreeMap::from([(
                1,
                Exercise {
                    id: 1,
                    name: String::from("A"),
                    muscles: vec![
                        ExerciseMuscle {
                            muscle_id: 11,
                            stimulus: 100,
                        },
                        ExerciseMuscle {
                            muscle_id: 31,
                            stimulus: 50,
                        },
                    ],
                },
            )])
        });

    static ROUTINE: std::sync::LazyLock<Routine> = std::sync::LazyLock::new(|| Routine {
        id: 1,
        name: String::from("A"),
        notes: Some(String::from("B")),
        archived: false,
        sections: vec![
            RoutinePart::RoutineSection {
                rounds: 2,
                parts: vec![
                    RoutinePart::RoutineActivity {
                        exercise_id: Some(1),
                        reps: 10,
                        time: 2,
                        weight: 30.0,
                        rpe: 10.0,
                        automatic: false,
                    },
                    RoutinePart::RoutineActivity {
                        exercise_id: None,
                        reps: 0,
                        time: 60,
                        weight: 0.0,
                        rpe: 0.0,
                        automatic: true,
                    },
                ],
            },
            RoutinePart::RoutineSection {
                rounds: 2,
                parts: vec![
                    RoutinePart::RoutineActivity {
                        exercise_id: Some(2),
                        reps: 10,
                        time: 0,
                        weight: 0.0,
                        rpe: 0.0,
                        automatic: false,
                    },
                    RoutinePart::RoutineActivity {
                        exercise_id: None,
                        reps: 0,
                        time: 30,
                        weight: 0.0,
                        rpe: 0.0,
                        automatic: true,
                    },
                ],
            },
        ],
    });

    static TRAINING_SESSION: std::sync::LazyLock<TrainingSession> =
        std::sync::LazyLock::new(|| TrainingSession {
            id: 1,
            routine_id: Some(2),
            date: *TODAY - Duration::days(10),
            notes: Some(String::from("A")),
            elements: vec![
                TrainingSessionElement::Set {
                    exercise_id: 1,
                    reps: Some(10),
                    time: Some(3),
                    weight: Some(30.0),
                    rpe: Some(8.0),
                    target_reps: Some(8),
                    target_time: Some(4),
                    target_weight: Some(40.0),
                    target_rpe: Some(9.0),
                    automatic: false,
                },
                TrainingSessionElement::Rest {
                    target_time: Some(60),
                    automatic: true,
                },
                TrainingSessionElement::Set {
                    exercise_id: 2,
                    reps: Some(5),
                    time: Some(4),
                    weight: None,
                    rpe: Some(4.0),
                    target_reps: None,
                    target_time: None,
                    target_weight: None,
                    target_rpe: None,
                    automatic: false,
                },
                TrainingSessionElement::Rest {
                    target_time: Some(60),
                    automatic: true,
                },
                TrainingSessionElement::Set {
                    exercise_id: 2,
                    reps: None,
                    time: Some(60),
                    weight: None,
                    rpe: None,
                    target_reps: None,
                    target_time: None,
                    target_weight: None,
                    target_rpe: None,
                    automatic: false,
                },
                TrainingSessionElement::Rest {
                    target_time: Some(60),
                    automatic: true,
                },
            ],
        });

    static EMPTY_TRAINING_SESSION: std::sync::LazyLock<TrainingSession> =
        std::sync::LazyLock::new(|| {
            let mut training_session = TRAINING_SESSION.clone();
            training_session.elements = TRAINING_SESSION
                .elements
                .iter()
                .map(|e| match e {
                    TrainingSessionElement::Set {
                        exercise_id,
                        target_reps,
                        target_time,
                        target_weight,
                        target_rpe,
                        automatic,
                        ..
                    } => TrainingSessionElement::Set {
                        exercise_id: *exercise_id,
                        reps: None,
                        time: None,
                        weight: None,
                        rpe: None,
                        target_reps: *target_reps,
                        target_time: *target_time,
                        target_weight: *target_weight,
                        target_rpe: *target_rpe,
                        automatic: *automatic,
                    },
                    TrainingSessionElement::Rest { .. } => e.clone(),
                })
                .collect::<Vec<_>>();
            training_session
        });

    #[test]
    fn test_user_serde() {
        let obj = User {
            id: 1,
            name: String::from("A"),
            sex: 0,
        };
        let serialized = json!({
            "id": 1,
            "name": "A",
            "sex": 0
        });
        let deserialized: User = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, obj);
    }

    #[test]
    fn test_exercise_serde() {
        let obj = Exercise {
            id: 1,
            name: String::from("A"),
            muscles: vec![ExerciseMuscle {
                muscle_id: 2,
                stimulus: 100,
            }],
        };
        let serialized = json!(obj);
        let deserialized: Exercise = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, obj);
    }

    #[test]
    fn test_exercise_muscle_stimulus() {
        assert_eq!(
            Exercise {
                id: 1,
                name: String::from("A"),
                muscles: vec![
                    ExerciseMuscle {
                        muscle_id: 2,
                        stimulus: 100,
                    },
                    ExerciseMuscle {
                        muscle_id: 8,
                        stimulus: 50,
                    }
                ],
            }
            .muscle_stimulus(),
            BTreeMap::from([(2, 100), (8, 50)])
        );
    }

    #[test]
    fn test_routine_serde() {
        let obj = &*ROUTINE;
        let serialized = json!(obj);
        let deserialized: Routine = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, *obj);
    }

    #[test]
    fn test_routine_duration() {
        assert_eq!(ROUTINE.duration(), Duration::seconds(300));
    }

    #[test]
    fn test_routine_num_sets() {
        assert_eq!(ROUTINE.num_sets(), 4);
    }

    #[test]
    fn test_routine_stimulus_per_muscle() {
        assert_eq!(
            ROUTINE.stimulus_per_muscle(&EXERCISES),
            BTreeMap::from([
                (1, 0),
                (11, 200),
                (21, 0),
                (22, 0),
                (31, 100),
                (32, 0),
                (33, 0),
                (41, 0),
                (42, 0),
                (51, 0),
                (61, 0),
                (62, 0),
                (71, 0),
                (72, 0),
                (81, 0),
                (82, 0),
                (83, 0),
                (91, 0),
            ])
        );
    }

    #[test]
    fn test_routine_exercises() {
        assert_eq!(ROUTINE.exercises(), BTreeSet::from([1, 2]));
    }

    #[test]
    fn test_training_session_serde() {
        let obj = &*TRAINING_SESSION;
        let serialized = json!(obj);
        let deserialized: TrainingSession = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, *obj);
    }

    #[test]
    fn test_training_session_exercises() {
        assert_eq!(TRAINING_SESSION.exercises(), BTreeSet::from([1, 2]));
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, Some(7.5))]
    #[case(&*EMPTY_TRAINING_SESSION, None)]
    fn test_training_session_avg_reps(
        #[case] training_session: &TrainingSession,
        #[case] expected: Option<f32>,
    ) {
        assert_eq!(training_session.avg_reps(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, Some(22.333_334))]
    #[case(&*EMPTY_TRAINING_SESSION, None)]
    fn test_training_session_avg_time(
        #[case] training_session: &TrainingSession,
        #[case] expected: Option<f32>,
    ) {
        assert_eq!(training_session.avg_time(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, Some(30.0))]
    #[case(&*EMPTY_TRAINING_SESSION, None)]
    fn test_training_session_avg_weight(
        #[case] training_session: &TrainingSession,
        #[case] expected: Option<f32>,
    ) {
        assert_eq!(training_session.avg_weight(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, Some(6.0))]
    #[case(&*EMPTY_TRAINING_SESSION, None)]
    fn test_training_session_avg_rpe(
        #[case] training_session: &TrainingSession,
        #[case] expected: Option<f32>,
    ) {
        assert_eq!(training_session.avg_rpe(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, 10)]
    #[case(&*EMPTY_TRAINING_SESSION, 0)]
    fn test_training_session_load(
        #[case] training_session: &TrainingSession,
        #[case] expected: u32,
    ) {
        assert_eq!(training_session.load(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, 2)]
    #[case(&*EMPTY_TRAINING_SESSION, 0)]
    fn test_training_session_set_volume(
        #[case] training_session: &TrainingSession,
        #[case] expected: u32,
    ) {
        assert_eq!(training_session.set_volume(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, 305)]
    #[case(&*EMPTY_TRAINING_SESSION, 0)]
    fn test_training_session_volume_load(
        #[case] training_session: &TrainingSession,
        #[case] expected: u32,
    ) {
        assert_eq!(training_session.volume_load(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, Some(110))]
    #[case(&*EMPTY_TRAINING_SESSION, None)]
    fn test_training_session_tut(
        #[case] training_session: &TrainingSession,
        #[case] expected: Option<u32>,
    ) {
        assert_eq!(training_session.tut(), expected);
    }

    #[rstest]
    #[case(&*TRAINING_SESSION, BTreeMap::from([(11, 100), (31, 50)]))]
    #[case(&*EMPTY_TRAINING_SESSION, BTreeMap::new())]
    fn test_training_session_stimulus_per_muscle(
        #[case] training_session: &TrainingSession,
        #[case] expected: BTreeMap<u8, u32>,
    ) {
        let exercises = BTreeMap::from([(
            1,
            Exercise {
                id: 1,
                name: String::from("A"),
                muscles: vec![
                    ExerciseMuscle {
                        muscle_id: 11,
                        stimulus: 100,
                    },
                    ExerciseMuscle {
                        muscle_id: 31,
                        stimulus: 50,
                    },
                ],
            },
        )]);
        assert_eq!(training_session.stimulus_per_muscle(&exercises), expected);
    }

    #[rstest]
    #[case::no_load_ratio(vec![], vec![], None)]
    #[case::load_ratio(
        vec![(from_num_days(0), 12.0), (from_num_days(1), 10.0)],
        vec![(from_num_days(0), 10.0), (from_num_days(1), 8.0)],
        Some(1.25)
    )]
    fn test_training_stats_load_ratio(
        #[case] short_term_load: Vec<(NaiveDate, f32)>,
        #[case] long_term_load: Vec<(NaiveDate, f32)>,
        #[case] expected: Option<f32>,
    ) {
        assert_eq!(
            TrainingStats {
                short_term_load,
                long_term_load,
            }
            .load_ratio(),
            expected
        );
    }

    #[test]
    fn test_training_stats_clear() {
        let mut training_stats = TrainingStats {
            short_term_load: vec![(from_num_days(0), 10.0)],
            long_term_load: vec![(from_num_days(0), 8.0)],
        };

        assert!(!training_stats.short_term_load.is_empty());
        assert!(!training_stats.long_term_load.is_empty());

        training_stats.clear();

        assert!(training_stats.short_term_load.is_empty());
        assert!(training_stats.long_term_load.is_empty());
    }

    #[rstest]
    #[case::no_sessions(&[], vec![(*TODAY, 0.0)], vec![])]
    #[case::one_session(
        &[&*TRAINING_SESSION],
        vec![
            (*TODAY - Duration::days(10), 10.0),
            (*TODAY - Duration::days(9), 8.571_428),
            (*TODAY - Duration::days(8), 7.142_857_6),
            (*TODAY - Duration::days(7), 5.714_285_4),
            (*TODAY - Duration::days(6), 4.285_714),
            (*TODAY - Duration::days(5), 2.857_142_7),
            (*TODAY - Duration::days(4), 1.428_570_7),
            (*TODAY - Duration::days(3), 0.0),
            (*TODAY - Duration::days(2), 0.0),
            (*TODAY - Duration::days(1), 0.0),
            (*TODAY, 0.0),
        ],
        vec![]
    )]
    fn test_training_stats(
        #[case] training_sessions: &[&TrainingSession],
        #[case] short_term_load: Vec<(NaiveDate, f32)>,
        #[case] long_term_load: Vec<(NaiveDate, f32)>,
    ) {
        assert_eq!(
            training_stats(training_sessions),
            TrainingStats {
                short_term_load,
                long_term_load
            }
        );
    }

    #[rstest]
    #[case::no_load(&[], 2, vec![])]
    #[case::load(
        &[
            (from_num_days(0), 10.0),
            (from_num_days(1), 8.0),
            (from_num_days(2), 6.0),
            (from_num_days(3), 4.0),
            (from_num_days(4), 2.0),
            (from_num_days(5), 0.0),
            (from_num_days(6), 0.0),
            (from_num_days(7), 0.0),
        ],
        3,
        vec![
            (from_num_days(2), 8.0),
            (from_num_days(3), 6.0),
            (from_num_days(4), 4.0),
            (from_num_days(5), 2.0),
            (from_num_days(6), 0.666_666_7),
            (from_num_days(7), 0.0),
        ]
    )]
    fn test_average_weighted_sum_of_load(
        #[case] weighted_sum_of_load: &[(NaiveDate, f32)],
        #[case] window_size: usize,
        #[case] expected: Vec<(NaiveDate, f32)>,
    ) {
        assert_eq!(
            average_weighted_sum_of_load(weighted_sum_of_load, window_size),
            expected
        );
    }

    #[test]
    fn test_muscle_id() {
        for muscle in Muscle::iter() {
            assert_eq!(Muscle::from_repr(muscle.id()).unwrap(), *muscle);
        }

        assert_eq!(Muscle::from_repr(u8::MAX), None);
    }

    #[test]
    fn test_muscle_name() {
        let mut names = HashSet::new();

        for muscle in Muscle::iter() {
            let name = muscle.name();

            assert!(!name.is_empty());
            assert!(!names.contains(name));

            names.insert(name);
        }
    }

    #[test]
    fn test_muscle_description() {
        let mut descriptions = HashSet::new();

        for muscle in Muscle::iter() {
            let description = muscle.description();

            assert!(description.is_empty() || !descriptions.contains(description));

            descriptions.insert(description);
        }
    }

    #[test]
    fn test_exercise_filter_is_empty() {
        assert!(ExerciseFilter {
            muscles: HashSet::new()
        }
        .is_empty());
    }

    #[test]
    fn test_body_weight_serde() {
        let obj = BodyWeight {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            weight: 80.0,
        };
        let serialized = json!(obj);
        let deserialized: BodyWeight = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, obj);
    }

    #[rstest]
    #[case::no_value(vec![], vec![])]
    #[case::one_value(
        vec![BodyWeight { date: from_num_days(0), weight: 80.0 }],
        vec![BodyWeight { date: from_num_days(0), weight: 80.0 }],
    )]
    #[case::less_values_than_radius(
        vec![
            BodyWeight { date: from_num_days(0), weight: 80.0 },
            BodyWeight { date: from_num_days(2), weight: 82.0 },
            BodyWeight { date: from_num_days(3), weight: 79.0 },
            BodyWeight { date: from_num_days(5), weight: 79.0 },
        ],
        vec![
            BodyWeight { date: from_num_days(0), weight: 80.0 },
            BodyWeight { date: from_num_days(2), weight: 80.0 },
            BodyWeight { date: from_num_days(3), weight: 80.0 },
            BodyWeight { date: from_num_days(5), weight: 80.0 },
        ],
    )]
    #[case::more_values_than_radius(
        vec![
            BodyWeight { date: from_num_days(0), weight: 81.0 },
            BodyWeight { date: from_num_days(2), weight: 82.0 },
            BodyWeight { date: from_num_days(3), weight: 83.0 },
            BodyWeight { date: from_num_days(5), weight: 84.0 },
            BodyWeight { date: from_num_days(6), weight: 85.0 },
            BodyWeight { date: from_num_days(8), weight: 86.0 },
            BodyWeight { date: from_num_days(9), weight: 87.0 },
            BodyWeight { date: from_num_days(10), weight: 88.0 },
            BodyWeight { date: from_num_days(12), weight: 89.0 },
        ],
        vec![
            BodyWeight { date: from_num_days(0), weight: 83.0 },
            BodyWeight { date: from_num_days(2), weight: 83.5 },
            BodyWeight { date: from_num_days(3), weight: 84.0 },
            BodyWeight { date: from_num_days(5), weight: 84.5 },
            BodyWeight { date: from_num_days(6), weight: 85.0 },
            BodyWeight { date: from_num_days(8), weight: 85.5 },
            BodyWeight { date: from_num_days(9), weight: 86.0 },
            BodyWeight { date: from_num_days(10), weight: 86.5 },
            BodyWeight { date: from_num_days(12), weight: 87.0 },
        ],
    )]
    fn test_avg_body_weight(
        #[case] body_weight: Vec<BodyWeight>,
        #[case] expected: Vec<BodyWeight>,
    ) {
        assert_eq!(
            avg_body_weight(&body_weight.into_iter().map(|bw| (bw.date, bw)).collect()),
            expected.into_iter().map(|bw| (bw.date, bw)).collect()
        );
    }

    #[test]
    fn test_body_fat_serde() {
        let obj = BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: Some(1),
            abdominal: Some(2),
            thigh: Some(3),
            tricep: Some(4),
            subscapular: Some(5),
            suprailiac: Some(6),
            midaxillary: Some(7),
        };
        let serialized = json!(obj);
        let deserialized: BodyFat = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, obj);
    }

    #[rstest]
    #[case::female_none(
        BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: None,
            abdominal: None,
            thigh: None,
            tricep: None,
            subscapular: None,
            suprailiac: None,
            midaxillary: None,
        },
        0,
        None,
        None
    )]
    #[case::female_jp3(
        BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: None,
            abdominal: None,
            thigh: Some(20),
            tricep: Some(15),
            subscapular: None,
            suprailiac: Some(5),
            midaxillary: None,
        },
        0,
        Some(17.298_523),
        None
    )]
    #[case::female_jp7(
        BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: Some(5),
            abdominal: Some(10),
            thigh: Some(20),
            tricep: Some(15),
            subscapular: Some(5),
            suprailiac: Some(5),
            midaxillary: Some(5),
        },
        0,
        Some(17.298_523),
        Some(14.794_678)
    )]
    #[case::male_none(
        BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: None,
            abdominal: None,
            thigh: None,
            tricep: None,
            subscapular: None,
            suprailiac: None,
            midaxillary: None,
        },
        1,
        None,
        None
    )]
    #[case::male_jp3(
        BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: Some(5),
            abdominal: Some(15),
            thigh: Some(15),
            tricep: None,
            subscapular: None,
            suprailiac: None,
            midaxillary: None,
        },
        1,
        Some(10.600_708),
        None
    )]
    #[case::male_jp7(
        BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: Some(5),
            abdominal: Some(15),
            thigh: Some(15),
            tricep: Some(15),
            subscapular: Some(10),
            suprailiac: Some(10),
            midaxillary: Some(10),
        },
        1,
        Some(10.600_708),
        Some(11.722_29)
    )]
    #[case::invalid(
        BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: Some(5),
            abdominal: Some(15),
            thigh: Some(15),
            tricep: Some(15),
            subscapular: Some(10),
            suprailiac: Some(10),
            midaxillary: Some(10),
        },
        2,
        None,
        None
    )]
    fn test_body_fat_jp(
        #[case] body_fat: BodyFat,
        #[case] sex: u8,
        #[case] expected_jp3: Option<f32>,
        #[case] expected_jp7: Option<f32>,
    ) {
        assert_eq!(body_fat.jp3(sex), expected_jp3);
        assert_eq!(body_fat.jp7(sex), expected_jp7);
    }

    #[test]
    fn test_period_serde() {
        let obj = Period {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            intensity: 2,
        };
        let serialized = json!(obj);
        let deserialized: Period = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, obj);
    }

    #[test]
    fn test_cycles() {
        assert_eq!(cycles(&BTreeMap::new()), vec![]);
        assert_eq!(
            cycles(&BTreeMap::from(
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

    #[rstest]
    #[case::no_cycle(&[], None)]
    #[case::no_recent_cycles(
        &[
            Cycle {
                begin: *TODAY - Duration::days(228),
                length: Duration::days(26),
            },
            Cycle {
                begin: *TODAY - Duration::days(202),
                length: Duration::days(28),
            }
        ],
        None
    )]
    #[case::one_cycle(
        &[
            Cycle {
                begin: *TODAY - Duration::days(42),
                length: Duration::days(28),
            }
        ],
        Some(
            CurrentCycle {
                begin: *TODAY - Duration::days(14),
                time_left: Duration::days(13),
                time_left_variation: Duration::days(0)
            }
        )
    )]
    #[case::multiple_cycles(
        &[
            Cycle {
                begin: *TODAY - Duration::days(68),
                length: Duration::days(26),
            },
            Cycle {
                begin: *TODAY - Duration::days(42),
                length: Duration::days(28),
            }
        ],
        Some(
            CurrentCycle {
                begin: *TODAY - Duration::days(14),
                time_left: Duration::days(12),
                time_left_variation: Duration::days(1)
            }
        )
    )]
    fn test_current_cycle(#[case] cycles: &[Cycle], #[case] expected: Option<CurrentCycle>) {
        assert_eq!(current_cycle(cycles), expected);
    }

    #[test]
    fn test_quartile_one() {
        assert_eq!(quartile(&[], Quartile::Q1), Duration::days(0));
        assert_eq!(
            quartile(&[Duration::days(2)], Quartile::Q1),
            Duration::days(0)
        );
        assert_eq!(
            quartile(&[Duration::days(4), Duration::days(12)], Quartile::Q1),
            Duration::days(4)
        );
        assert_eq!(
            quartile(
                &[Duration::days(2), Duration::days(4), Duration::days(6)],
                Quartile::Q1
            ),
            Duration::days(2)
        );
        assert_eq!(
            quartile(
                &[
                    Duration::days(2),
                    Duration::days(4),
                    Duration::days(6),
                    Duration::days(8)
                ],
                Quartile::Q1
            ),
            Duration::days(3)
        );
        assert_eq!(
            quartile(
                &[
                    Duration::days(2),
                    Duration::days(4),
                    Duration::days(5),
                    Duration::days(6),
                    Duration::days(8)
                ],
                Quartile::Q1
            ),
            Duration::days(3)
        );
        assert_eq!(
            quartile(
                &[
                    Duration::days(2),
                    Duration::days(4),
                    Duration::days(5),
                    Duration::days(6),
                    Duration::days(7),
                    Duration::days(8)
                ],
                Quartile::Q1
            ),
            Duration::days(4)
        );
    }

    #[test]
    fn test_quartile_two() {
        assert_eq!(quartile(&[], Quartile::Q2), Duration::days(0));
        assert_eq!(
            quartile(&[Duration::days(2)], Quartile::Q2),
            Duration::days(2)
        );
        assert_eq!(
            quartile(&[Duration::days(4), Duration::days(12)], Quartile::Q2),
            Duration::days(8)
        );
        assert_eq!(
            quartile(
                &[Duration::days(2), Duration::days(4), Duration::days(6)],
                Quartile::Q2
            ),
            Duration::days(4)
        );
    }

    #[test]
    fn test_quartile_three() {
        assert_eq!(quartile(&[], Quartile::Q3), Duration::days(0));
        assert_eq!(
            quartile(&[Duration::days(2)], Quartile::Q3),
            Duration::days(0)
        );
        assert_eq!(
            quartile(
                &[Duration::days(2), Duration::days(4), Duration::days(6)],
                Quartile::Q3
            ),
            Duration::days(6)
        );
        assert_eq!(
            quartile(
                &[
                    Duration::days(2),
                    Duration::days(4),
                    Duration::days(6),
                    Duration::days(8)
                ],
                Quartile::Q3
            ),
            Duration::days(7)
        );
        assert_eq!(
            quartile(
                &[
                    Duration::days(2),
                    Duration::days(4),
                    Duration::days(5),
                    Duration::days(6),
                    Duration::days(8)
                ],
                Quartile::Q3
            ),
            Duration::days(7)
        );
        assert_eq!(
            quartile(
                &[
                    Duration::days(2),
                    Duration::days(3),
                    Duration::days(4),
                    Duration::days(5),
                    Duration::days(6),
                    Duration::days(8)
                ],
                Quartile::Q3
            ),
            Duration::days(6)
        );
    }

    #[rstest]
    #[case(*TODAY - Duration::days(21), *TODAY - Duration::days(42))]
    fn test_interval_from_range_inclusive(#[case] first: NaiveDate, #[case] last: NaiveDate) {
        let interval: Interval = (first..=last).into();
        assert_eq!(interval, Interval { first, last });
    }

    #[rstest]
    #[case::no_dates(
        &[],
        DefaultInterval::_1M,
        *TODAY - Duration::days(DefaultInterval::_1M as i64),
        *TODAY
    )]
    #[case::last_date_inside_default_interval(
        &[*TODAY - Duration::days(DefaultInterval::_1M as i64 - 2)],
        DefaultInterval::_1M,
        *TODAY - Duration::days(DefaultInterval::_1M as i64),
        *TODAY
    )]
    #[case::last_date_outside_default_interval(
        &[*TODAY - Duration::days(DefaultInterval::_1M as i64 + 42)],
        DefaultInterval::_1M,
        *TODAY - Duration::days(DefaultInterval::_1M as i64 + 42),
        *TODAY
    )]
    #[case::default_interval_all(
        &[*TODAY - Duration::days(21), *TODAY - Duration::days(42)],
        DefaultInterval::All,
        *TODAY - Duration::days(42),
        *TODAY,
    )]
    fn test_init_interval(
        #[case] dates: &[NaiveDate],
        #[case] default_interval: DefaultInterval,
        #[case] first: NaiveDate,
        #[case] last: NaiveDate,
    ) {
        assert_eq!(
            init_interval(dates, default_interval),
            Interval { first, last }
        );
    }

    #[rstest]
    #[case::empty_series(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[],
        vec![]
    )]
    #[case::value_outside_interval(
        (2020, 3, 3),
        (2020, 3, 5),
        0,
        &[(2020, 2, 3, 1.0)],
        vec![]
    )]
    #[case::zero_radius_single_value(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[(2020, 2, 3, 1.0)],
        vec![vec![(2020, 2, 3, 1.0)]]
    )]
    #[case::zero_radius_multiple_days(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 1.0), (2020, 2, 5, 1.0)],
        vec![vec![(2020, 2, 3, 1.0), (2020, 2, 4, 1.0), (2020, 2, 5, 1.0)]]
    )]
    #[case::zero_radius_multiple_values_per_day(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 1.0), (2020, 2, 5, 1.0), (2020, 2, 3, 3.0)],
        vec![vec![(2020, 2, 3, 2.0), (2020, 2, 4, 1.0), (2020, 2, 5, 1.0)]]
    )]
    #[case::nonzero_radius_multiple_days(
        (2020, 2, 3),
        (2020, 2, 5),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0)],
        vec![vec![(2020, 2, 3, 1.5), (2020, 2, 4, 2.0), (2020, 2, 5, 2.5)]]
    )]
    #[case::nonzero_radius_missing_day(
        (2020, 2, 2),
        (2020, 2, 6),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0)],
        vec![vec![(2020, 2, 2, 1.0), (2020, 2, 3, 1.5), (2020, 2, 4, 2.0), (2020, 2, 5, 2.5), (2020, 2, 6, 3.0)]]
    )]
    #[case::nonzero_radius_with_gap_1(
        (2020, 2, 3),
        (2020, 2, 7),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 7, 1.0)],
        vec![vec![(2020, 2, 3, 1.0), (2020, 2, 4, 1.0)], vec![(2020, 2, 6, 1.0), (2020, 2, 7, 1.0)]]
    )]
    #[case::nonzero_radius_with_gap_2(
        (2020, 2, 3),
        (2020, 2, 9),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 9, 1.0)],
        vec![vec![(2020, 2, 3, 1.0), (2020, 2, 4, 1.0)], vec![(2020, 2, 8, 1.0), (2020, 2, 9, 1.0)]]
    )]
    fn test_centered_moving_average(
        #[case] start: (i32, u32, u32),
        #[case] end: (i32, u32, u32),
        #[case] radius: u64,
        #[case] input: &[(i32, u32, u32, f32)],
        #[case] expected: Vec<Vec<(i32, u32, u32, f32)>>,
    ) {
        assert_eq!(
            centered_moving_average(
                &input
                    .iter()
                    .map(|(y, m, d, v)| (NaiveDate::from_ymd_opt(*y, *m, *d).unwrap(), *v))
                    .collect::<Vec<_>>(),
                &Interval {
                    first: NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap(),
                    last: NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap(),
                },
                radius,
            ),
            expected
                .iter()
                .map(|v| v
                    .iter()
                    .map(|(y, m, d, v)| (NaiveDate::from_ymd_opt(*y, *m, *d).unwrap(), *v))
                    .collect::<Vec<_>>())
                .collect::<Vec<_>>(),
        );
    }

    #[rstest]
    #[case::empty_series(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[],
        &[(2020, 2, 3, 0.0), (2020, 2, 4, 0.0), (2020, 2, 5, 0.0)],
    )]
    #[case::value_outside_interval(
        (2020, 3, 3),
        (2020, 3, 5),
        0,
        &[(2020, 2, 3, 1.0)],
        &[(2020, 3, 3, 0.0), (2020, 3, 4, 0.0), (2020, 3, 5, 0.0)],
    )]
    #[case::zero_radius_single_day(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[(2020, 2, 3, 1.0)],
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 0.0), (2020, 2, 5, 0.0)],
    )]
    #[case::zero_radius_multiple_days(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0)],
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0)],
    )]
    #[case::zero_radius_multiple_values_per_day(
        (2020, 2, 3),
        (2020, 2, 5),
        0,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0), (2020, 2, 3, 1.0)],
        &[(2020, 2, 3, 2.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0)],
    )]
    #[case::nonzero_radius_multiple_days(
        (2020, 2, 3),
        (2020, 2, 5),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0)],
        &[(2020, 2, 3, 3.0), (2020, 2, 4, 6.0), (2020, 2, 5, 5.0)],
    )]
    #[case::nonzero_radius_missing_day(
        (2020, 2, 2),
        (2020, 2, 6),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0)],
        &[(2020, 2, 2, 1.0), (2020, 2, 3, 3.0), (2020, 2, 4, 6.0), (2020, 2, 5, 5.0), (2020, 2, 6, 3.0)],
    )]
    #[case::nonzero_radius_multiple_missing_days_1(
        (2020, 2, 3),
        (2020, 2, 7),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 7, 1.0)],
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 1.0), (2020, 2, 5, 0.0), (2020, 2, 6, 1.0), (2020, 2, 7, 1.0)],
    )]
    #[case::nonzero_radius_multiple_missing_days_2(
        (2020, 2, 3),
        (2020, 2, 9),
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 9, 1.0)],
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 1.0), (2020, 2, 5, 0.0), (2020, 2, 6, 0.0), (2020, 2, 7, 0.0), (2020, 2, 8, 1.0), (2020, 2, 9, 1.0)]
    )]
    fn test_centered_moving_total(
        #[case] start: (i32, u32, u32),
        #[case] end: (i32, u32, u32),
        #[case] radius: u64,
        #[case] input: &[(i32, u32, u32, f32)],
        #[case] expected: &[(i32, u32, u32, f32)],
    ) {
        assert_eq!(
            centered_moving_total(
                &input
                    .iter()
                    .map(|(y, m, d, v)| (NaiveDate::from_ymd_opt(*y, *m, *d).unwrap(), *v))
                    .collect::<Vec<_>>(),
                &Interval {
                    first: NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap(),
                    last: NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap(),
                },
                radius,
            ),
            expected
                .iter()
                .map(|(y, m, d, v)| (NaiveDate::from_ymd_opt(*y, *m, *d).unwrap(), *v))
                .collect::<Vec<_>>(),
        );
    }

    #[rstest]
    #[case::empty_series(
        0,
        &[],
        vec![]
    )]
    #[case::zero_radius_single_value(
        0,
        &[(2020, 2, 3, 1.0)],
        vec![(2020, 2, 3, 1.0)]
    )]
    #[case::zero_radius_multiple_days(
        0,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 1.0), (2020, 2, 5, 1.0)],
        vec![(2020, 2, 3, 1.0), (2020, 2, 4, 1.0), (2020, 2, 5, 1.0)]
    )]
    #[case::nonzero_radius_multiple_days(
        1,
        &[(2020, 2, 3, 1.0), (2020, 2, 5, 2.0), (2020, 2, 7, 3.0)],
        vec![(2020, 2, 3, 1.5), (2020, 2, 5, 2.0), (2020, 2, 7, 2.5)]
    )]
    #[case::nonzero_radius_multiple_days(
        2,
        &[(2020, 2, 3, 1.0), (2020, 2, 4, 2.0), (2020, 2, 5, 3.0), (2020, 2, 6, 4.0), (2020, 2, 6, 5.0)],
        vec![(2020, 2, 3, 2.0), (2020, 2, 4, 2.5), (2020, 2, 5, 3.0), (2020, 2, 6, 3.5), (2020, 2, 6, 4.0)]
    )]
    fn test_value_based_centered_moving_average(
        #[case] radius: usize,
        #[case] input: &[(i32, u32, u32, f32)],
        #[case] expected: Vec<(i32, u32, u32, f32)>,
    ) {
        assert_eq!(
            value_based_centered_moving_average(
                &input
                    .iter()
                    .map(|(y, m, d, v)| (NaiveDate::from_ymd_opt(*y, *m, *d).unwrap(), *v))
                    .collect::<Vec<_>>(),
                radius,
            ),
            expected
                .iter()
                .map(|(y, m, d, v)| (NaiveDate::from_ymd_opt(*y, *m, *d).unwrap(), *v))
                .collect::<Vec<_>>()
        );
    }

    fn from_num_days(days: i32) -> NaiveDate {
        NaiveDate::from_num_days_from_ce_opt(days).unwrap()
    }
}
