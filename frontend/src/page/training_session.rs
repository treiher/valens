use std::collections::BTreeMap;
use std::collections::HashMap;

use chrono::{prelude::*, Duration};
use seed::{prelude::*, *};

use crate::common;
use crate::data;

// ------ ------
//     Init
// ------ ------

pub fn init(
    mut url: Url,
    orders: &mut impl Orders<Msg>,
    data_model: &data::Model,
    navbar: &mut crate::Navbar,
) -> Model {
    let training_session_id = url
        .next_hash_path_part()
        .unwrap_or("")
        .parse::<u32>()
        .unwrap_or(0);
    let action = url.next_hash_path_part();
    let editing = action == Some("edit");
    let guide = if action == Some("guide") {
        Some(Guide::new(data_model.settings.beep_volume))
    } else {
        None
    };

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Training session");
    navbar.items = vec![(
        ev(Ev::Click, |_| {
            crate::Msg::TrainingSession(Msg::ShowSMTDialog)
        }),
        String::from("stopwatch"),
    )];

    let training_session = data_model.training_sessions.get(&training_session_id);
    let audio_context = match web_sys::AudioContext::new() {
        Ok(ctx) => Some(ctx),
        Err(err) => {
            error!("failed to create audio context:", err);
            None
        }
    };

    if let Some(ongoing_training_session) = &data_model.ongoing_training_session {
        if ongoing_training_session.training_session_id == training_session_id {
            orders.send_msg(Msg::ContinueGuidedTrainingSession(
                ongoing_training_session.clone(),
            ));
        }
    }

    Model {
        training_session_id,
        form: init_form(training_session, data_model),
        guide,
        dialog: Dialog::Hidden,
        smt: StopwatchMetronomTimer {
            stopwatch: Stopwatch {
                time: 0,
                start_time: None,
            },
            metronome: Metronome {
                interval: 1,
                stressed_beat: 1,
                beat_number: 0,
                next_beat_time: 0.,
                is_active: false,
                beep_volume: data_model.settings.beep_volume,
            },
            timer: Timer {
                time: (String::from("60"), Some(60)),
                reset_time: 60,
                target_time: None,
                beep_time: 0.,
                beep_volume: data_model.settings.beep_volume,
            },
        },
        timer_stream: None,
        audio_context,
        editing,
        loading: false,
        notification: None,
    }
}

fn init_form(training_session: Option<&data::TrainingSession>, data_model: &data::Model) -> Form {
    let previous_sets = previous_sets(training_session, data_model);
    if let Some(training_session) = training_session {
        let mut sections = vec![];
        let mut exercises = vec![];
        let mut position = 0;
        let mut prev_set_positions: HashMap<u32, usize> = HashMap::new();

        for e in &training_session.elements {
            match e {
                data::TrainingSessionElement::Set {
                    exercise_id,
                    reps,
                    time,
                    weight,
                    rpe,
                    target_reps,
                    target_time,
                    target_weight,
                    target_rpe,
                    automatic,
                } => {
                    if target_time.is_some() && target_reps.is_none() {
                        if not(exercises.is_empty()) {
                            sections.push(FormSection::Set { exercises });
                            position = 0;
                        }
                        exercises = vec![];
                    }
                    let prev_set_position = prev_set_positions
                        .entry(*exercise_id)
                        .and_modify(|position| *position += 1)
                        .or_insert(0);
                    let (prev_reps, prev_time, prev_weight, prev_rpe) =
                        if let Some(prev_sets) = previous_sets.get(exercise_id) {
                            if let Some(data::TrainingSessionElement::Set {
                                reps,
                                time,
                                weight,
                                rpe,
                                ..
                            }) = prev_sets.get(*prev_set_position)
                            {
                                (*reps, *time, *weight, *rpe)
                            } else {
                                (None, None, None, None)
                            }
                        } else {
                            (None, None, None, None)
                        };
                    exercises.push(ExerciseForm {
                        position,
                        exercise_id: *exercise_id,
                        exercise_name: data_model
                            .exercises
                            .get(exercise_id)
                            .map_or_else(|| format!("Exercise#{exercise_id}"), |e| e.name.clone()),
                        reps: common::InputField {
                            input: reps.map(|v| v.to_string()).unwrap_or_default(),
                            parsed: some_or_default(*reps),
                            orig: reps.map(|v| v.to_string()).unwrap_or_default(),
                        },
                        time: common::InputField {
                            input: time.map(|v| v.to_string()).unwrap_or_default(),
                            parsed: some_or_default(*time),
                            orig: time.map(|v| v.to_string()).unwrap_or_default(),
                        },
                        weight: common::InputField {
                            input: weight.map(|v| v.to_string()).unwrap_or_default(),
                            parsed: some_or_default(*weight),
                            orig: weight.map(|v| v.to_string()).unwrap_or_default(),
                        },
                        rpe: common::InputField {
                            input: rpe.map(|v| v.to_string()).unwrap_or_default(),
                            parsed: some_or_default(*rpe),
                            orig: rpe.map(|v| v.to_string()).unwrap_or_default(),
                        },
                        target_reps: *target_reps,
                        target_time: *target_time,
                        target_weight: *target_weight,
                        target_rpe: *target_rpe,
                        prev_reps,
                        prev_time,
                        prev_weight,
                        prev_rpe,
                        automatic: *automatic,
                    });
                    position += 1;
                    if target_time.is_some() && target_reps.is_none() {
                        if not(exercises.is_empty()) {
                            sections.push(FormSection::Set { exercises });
                            position = 0;
                        }
                        exercises = vec![];
                    }
                }
                data::TrainingSessionElement::Rest {
                    target_time,
                    automatic,
                } => {
                    if not(exercises.is_empty()) {
                        sections.push(FormSection::Set { exercises });
                        position = 0;
                    }
                    exercises = vec![];
                    sections.push(FormSection::Rest {
                        target_time: target_time.unwrap_or(0),
                        automatic: *automatic,
                    });
                }
            }
        }

        if not(exercises.is_empty()) {
            sections.push(FormSection::Set { exercises });
        }

        Form {
            notes: training_session.notes.clone().unwrap_or_default(),
            notes_changed: false,
            sections,
        }
    } else {
        Form {
            notes: String::new(),
            notes_changed: false,
            sections: vec![],
        }
    }
}

fn previous_sets(
    training_session: Option<&data::TrainingSession>,
    data_model: &data::Model,
) -> HashMap<u32, Vec<data::TrainingSessionElement>> {
    let mut sets: HashMap<u32, Vec<data::TrainingSessionElement>> = HashMap::new();
    if let Some(training_session) = training_session {
        if let Some(previous_training_session) = &data_model
            .training_sessions
            .values()
            .filter(|t| {
                t.id != training_session.id
                    && t.date <= training_session.date
                    && (not(training_session.routine_id.is_some())
                        || t.routine_id == training_session.routine_id)
            })
            .last()
        {
            for e in &previous_training_session.elements {
                if let data::TrainingSessionElement::Set { exercise_id, .. } = e {
                    sets.entry(*exercise_id).or_default().push(e.clone());
                }
            }
        }
    }
    sets
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    training_session_id: u32,
    form: Form,
    guide: Option<Guide>,
    dialog: Dialog,
    smt: StopwatchMetronomTimer,
    timer_stream: Option<StreamHandle>,
    audio_context: Option<web_sys::AudioContext>,
    editing: bool,
    loading: bool,
    notification: Option<web_sys::Notification>,
}

impl Model {
    pub fn has_unsaved_changes(&self) -> bool {
        self.form.changed()
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        close_notification(self);
    }
}

struct Form {
    notes: String,
    notes_changed: bool,
    sections: Vec<FormSection>,
}

impl Form {
    fn changed(&self) -> bool {
        self.notes_changed
            || self
                .sections
                .iter()
                .filter_map(|s| match s {
                    FormSection::Set { exercises } => Some(exercises),
                    _ => None,
                })
                .flatten()
                .any(|e| {
                    e.reps.changed() || e.time.changed() || e.weight.changed() || e.rpe.changed()
                })
    }

    fn valid(&self) -> bool {
        self.sections
            .iter()
            .filter_map(|s| match s {
                FormSection::Set { exercises } => Some(exercises),
                _ => None,
            })
            .flatten()
            .all(|s| s.reps.valid() && s.time.valid() && s.weight.valid() && s.rpe.valid())
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
enum FormSection {
    Set { exercises: Vec<ExerciseForm> },
    Rest { target_time: u32, automatic: bool },
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
struct ExerciseForm {
    position: usize,
    exercise_id: u32,
    exercise_name: String,
    reps: common::InputField<u32>,
    time: common::InputField<u32>,
    weight: common::InputField<f32>,
    rpe: common::InputField<f32>,
    target_reps: Option<u32>,
    target_time: Option<u32>,
    target_weight: Option<f32>,
    target_rpe: Option<f32>,
    prev_reps: Option<u32>,
    prev_time: Option<u32>,
    prev_weight: Option<f32>,
    prev_rpe: Option<f32>,
    automatic: bool,
}

struct Guide {
    section_idx: usize,
    section_start_time: DateTime<Utc>,
    timer: Timer,
    stream: Option<StreamHandle>,
    element: ElRef<web_sys::Element>,
}

impl Guide {
    fn new(beep_volume: u8) -> Guide {
        Guide {
            section_idx: 0,
            section_start_time: Utc::now(),
            timer: Timer::new(beep_volume),
            stream: None,
            element: ElRef::new(),
        }
    }

    fn from_ongoing_training_session(
        section_idx: usize,
        section_start_time: DateTime<Utc>,
        beep_volume: u8,
    ) -> Guide {
        Guide {
            section_idx,
            section_start_time,
            timer: Timer::new(beep_volume),
            stream: None,
            element: ElRef::new(),
        }
    }
}

#[derive(PartialEq)]
enum Dialog {
    Hidden,
    StopwatchMetronomTimer,
    Options(usize, usize),
    ReplaceExercise(usize, usize, String),
}

struct StopwatchMetronomTimer {
    stopwatch: Stopwatch,
    metronome: Metronome,
    timer: Timer,
}

struct Stopwatch {
    time: i64,
    start_time: Option<DateTime<Utc>>,
}

impl Stopwatch {
    fn is_active(&self) -> bool {
        self.start_time.is_some()
    }

    fn toggle(&mut self) {
        if not(self.is_active()) && self.time > 0 {
            self.reset();
        } else {
            self.start_pause();
        }
    }

    fn start_pause(&mut self) {
        self.start_time = match self.start_time {
            Some(_) => None,
            None => Some(Utc::now() - Duration::milliseconds(self.time)),
        };
    }

    fn reset(&mut self) {
        self.time = 0;
        if self.start_time.is_some() {
            self.start_time = Some(Utc::now());
        }
    }

    fn update(&mut self) {
        if let Some(start_time) = self.start_time {
            self.time = Utc::now()
                .signed_duration_since(start_time)
                .num_milliseconds();
        }
    }
}

struct Metronome {
    interval: u32,
    stressed_beat: u32,
    beat_number: u32,
    next_beat_time: f64,
    is_active: bool,
    beep_volume: u8,
}

impl Metronome {
    fn is_active(&self) -> bool {
        self.is_active
    }

    fn start(&mut self, audio_context: &Option<web_sys::AudioContext>) {
        self.is_active = true;
        if let Some(audio_context) = audio_context {
            self.beat_number = 0;
            self.next_beat_time = audio_context.current_time() + 0.5;
        }
    }

    fn pause(&mut self) {
        self.is_active = false;
    }

    fn start_pause(&mut self, audio_context: &Option<web_sys::AudioContext>) {
        if self.is_active() {
            self.pause();
        } else {
            self.start(audio_context);
        }
    }

    fn update(&mut self, audio_context: &Option<web_sys::AudioContext>) {
        if self.is_active() {
            if let Some(audio_context) = audio_context {
                while self.next_beat_time < audio_context.current_time() + 0.5 {
                    if let Err(err) = play_beep(
                        audio_context,
                        if self.beat_number % self.stressed_beat == 0 {
                            1000.
                        } else {
                            500.
                        },
                        self.next_beat_time,
                        0.05,
                        self.beep_volume,
                    ) {
                        error!("failed to play beep:", err);
                    }
                    self.next_beat_time += f64::from(self.interval);
                    self.beat_number += 1;
                }
            }
        }
    }
}

struct Timer {
    time: (String, Option<i64>),
    reset_time: i64,
    target_time: Option<DateTime<Utc>>,
    beep_time: f64,
    beep_volume: u8,
}

impl Timer {
    fn new(beep_volume: u8) -> Timer {
        Timer {
            time: (String::new(), None),
            reset_time: 0,
            target_time: None,
            beep_time: 0.,
            beep_volume,
        }
    }

    fn is_set(&self) -> bool {
        self.reset_time > 0
    }

    fn is_active(&self) -> bool {
        self.target_time.is_some()
    }

    fn start(&mut self) {
        self.target_time = Some(Utc::now() + Duration::seconds(self.time.1.unwrap()));
    }

    fn pause(&mut self) {
        self.target_time = None;
    }

    fn start_pause(&mut self) {
        if self.target_time.is_some() {
            self.pause();
        } else {
            self.start();
        }
    }

    fn set(&mut self, time: i64) {
        self.time = (time.to_string(), Some(time));
        self.reset_time = time;
        if self.target_time.is_some() {
            self.target_time = Some(Utc::now() + Duration::seconds(time));
        }
    }

    fn unset(&mut self) {
        self.reset_time = 0;
        self.target_time = None;
        self.beep_time = 0.;
    }

    fn reset(&mut self) {
        self.set(self.reset_time);
    }

    fn update(&mut self, audio_context: &Option<web_sys::AudioContext>) {
        if let Some(target_time) = self.target_time {
            #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
            let time = (target_time
                .signed_duration_since(Utc::now())
                .num_milliseconds() as f64
                / 1000.)
                .round() as i64;
            if (0..=3).contains(&time) && Some(time) != self.time.1 {
                if let Some(audio_context) = audio_context {
                    if let Err(err) = play_beep(
                        audio_context,
                        2000.,
                        if time == 3 {
                            self.beep_time = audio_context.current_time() + 0.01;
                            self.beep_time
                        } else {
                            self.beep_time += 1.;
                            self.beep_time
                        },
                        if time == 0 { 0.5 } else { 0.15 },
                        self.beep_volume,
                    ) {
                        error!("failed to play beep:", err);
                    }
                }
            }
            self.time = (time.to_string(), Some(time));
        }
    }

    fn to_timer_state(&self) -> data::TimerState {
        if self.is_active() {
            data::TimerState::Active {
                target_time: self.target_time.unwrap_or(Utc::now()),
            }
        } else if self.is_set() {
            data::TimerState::Paused {
                time: self.time.1.unwrap_or(0),
            }
        } else {
            data::TimerState::Unset
        }
    }

    fn restore(&mut self, timer_state: data::TimerState) {
        match timer_state {
            data::TimerState::Unset => {
                self.unset();
            }
            data::TimerState::Active { target_time } => {
                self.set((target_time - Utc::now()).num_seconds());
                self.start();
            }
            data::TimerState::Paused { time } => {
                self.set(time);
                self.pause();
            }
        }
    }
}

fn play_beep(
    audio_context: &web_sys::AudioContext,
    frequency: f32,
    start: f64,
    length: f64,
    volume: u8,
) -> Result<(), JsValue> {
    let oscillator = audio_context.create_oscillator()?;
    let gain = audio_context.create_gain()?;
    gain.gain().set_value(f32::from(volume) / 100.);
    gain.connect_with_audio_node(&audio_context.destination())?;
    oscillator.connect_with_audio_node(&gain)?;
    oscillator.frequency().set_value(frequency);
    oscillator.start_with_when(start)?;
    oscillator.stop_with_when(start + length)?;
    Ok(())
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    RepsChanged(usize, usize, String),
    TimeChanged(usize, usize, String),
    WeightChanged(usize, usize, String),
    RPEChanged(usize, usize, String),
    NotesChanged(String),

    EnterTargetValues(usize, usize),
    EnterPreviousValues(usize, usize),

    StartGuidedTrainingSession,
    ContinueGuidedTrainingSession(data::OngoingTrainingSession),
    UpdateGuidedTrainingSession,
    StartPauseGuideTimer,
    GoToPreviousSection,
    GoToNextSection,
    ScrollToSection,

    EditTrainingSession,
    SaveTrainingSession,
    DataEvent(data::Event),

    ShowSMTDialog,
    ShowOptionsDialog(usize, usize),
    ShowReplaceExerciseDialog(usize, usize),
    SearchTermChanged(String),
    CreateExercise,
    ReplaceExercise(usize, usize, u32),
    CloseDialog,

    UpdateStopwatchMetronomTimer,

    StartPauseStopwatch,
    ResetStopwatch,
    ToggleStopwatch,

    StartMetronome(u32),
    PauseMetronome,
    StartPauseMetronome,
    MetronomeIntervalChanged(String),
    MetronomeStressChanged(String),

    StartPauseTimer,
    ResetTimer,
    TimerTimeChanged(String),
}

pub fn update(
    msg: Msg,
    model: &mut Model,
    data_model: &data::Model,
    orders: &mut impl Orders<Msg>,
) {
    match msg {
        Msg::RepsChanged(section_idx, exercise_idx, input) => {
            if let FormSection::Set { exercises } = &mut model.form.sections[section_idx] {
                let ExerciseForm { reps, .. } = &mut exercises[exercise_idx];
                match input.parse::<u32>() {
                    Ok(parsed_reps) => {
                        let valid = common::valid_reps(parsed_reps);
                        let parsed = if valid { Some(parsed_reps) } else { None };
                        *reps = common::InputField {
                            input,
                            parsed,
                            orig: reps.orig.clone(),
                        }
                    }
                    Err(_) => {
                        *reps = common::InputField {
                            input: input.clone(),
                            parsed: if input.is_empty() { Some(0) } else { None },
                            orig: reps.orig.clone(),
                        }
                    }
                }
            }
        }
        Msg::TimeChanged(section_idx, exercise_idx, input) => {
            if let FormSection::Set { exercises } = &mut model.form.sections[section_idx] {
                let ExerciseForm { time, .. } = &mut exercises[exercise_idx];
                match input.parse::<u32>() {
                    Ok(parsed_time) => {
                        let valid = common::valid_time(parsed_time);
                        let parsed = if valid { Some(parsed_time) } else { None };
                        *time = common::InputField {
                            input,
                            parsed,
                            orig: time.orig.clone(),
                        }
                    }
                    Err(_) => {
                        *time = common::InputField {
                            input: input.clone(),
                            parsed: if input.is_empty() { Some(0) } else { None },
                            orig: time.orig.clone(),
                        }
                    }
                }
            }
        }
        Msg::WeightChanged(section_idx, exercise_idx, input) => {
            if let FormSection::Set { exercises } = &mut model.form.sections[section_idx] {
                let ExerciseForm { weight, .. } = &mut exercises[exercise_idx];
                match input.parse::<f32>() {
                    Ok(parsed_weight) => {
                        let valid = common::valid_weight(parsed_weight);
                        let parsed = if valid { Some(parsed_weight) } else { None };
                        *weight = common::InputField {
                            input,
                            parsed,
                            orig: weight.orig.clone(),
                        }
                    }
                    Err(_) => {
                        *weight = common::InputField {
                            input: input.clone(),
                            parsed: if input.is_empty() { Some(0.0) } else { None },
                            orig: weight.orig.clone(),
                        }
                    }
                }
            }
        }
        Msg::RPEChanged(section_idx, exercise_idx, input) => {
            if let FormSection::Set { exercises } = &mut model.form.sections[section_idx] {
                let ExerciseForm { rpe, .. } = &mut exercises[exercise_idx];
                match input.parse::<f32>() {
                    Ok(parsed_rpe) => {
                        let valid = common::valid_rpe(parsed_rpe);
                        let parsed = if valid { Some(parsed_rpe) } else { None };
                        *rpe = common::InputField {
                            input,
                            parsed,
                            orig: rpe.orig.clone(),
                        }
                    }
                    Err(_) => {
                        *rpe = common::InputField {
                            input: input.clone(),
                            parsed: if input.is_empty() { Some(0.0) } else { None },
                            orig: rpe.orig.clone(),
                        }
                    }
                }
            }
        }
        Msg::NotesChanged(notes) => {
            model.form.notes = notes;
            model.form.notes_changed = true;
        }

        Msg::EnterTargetValues(section_idx, exercise_idx) => {
            if let FormSection::Set { exercises } = &mut model.form.sections[section_idx] {
                let ExerciseForm {
                    reps,
                    time,
                    weight,
                    rpe,
                    target_reps,
                    target_time,
                    target_weight,
                    target_rpe,
                    ..
                } = &mut exercises[exercise_idx];
                *reps = common::InputField {
                    input: target_reps.map(|v| v.to_string()).unwrap_or_default(),
                    parsed: some_or_default(*target_reps),
                    orig: reps.orig.clone(),
                };
                *time = common::InputField {
                    input: target_time.map(|v| v.to_string()).unwrap_or_default(),
                    parsed: some_or_default(*target_time),
                    orig: time.orig.clone(),
                };
                *weight = common::InputField {
                    input: target_weight.map(|v| v.to_string()).unwrap_or_default(),
                    parsed: some_or_default(*target_weight),
                    orig: weight.orig.clone(),
                };
                *rpe = common::InputField {
                    input: target_rpe.map(|v| v.to_string()).unwrap_or_default(),
                    parsed: some_or_default(*target_rpe),
                    orig: rpe.orig.clone(),
                };
            }
        }
        Msg::EnterPreviousValues(section_idx, exercise_idx) => {
            if let FormSection::Set { exercises } = &mut model.form.sections[section_idx] {
                let ExerciseForm {
                    reps,
                    time,
                    weight,
                    rpe,
                    prev_reps,
                    prev_time,
                    prev_weight,
                    prev_rpe,
                    ..
                } = &mut exercises[exercise_idx];
                *reps = common::InputField {
                    input: prev_reps.map(|v| v.to_string()).unwrap_or_default(),
                    parsed: some_or_default(*prev_reps),
                    orig: reps.orig.clone(),
                };
                *time = common::InputField {
                    input: prev_time.map(|v| v.to_string()).unwrap_or_default(),
                    parsed: some_or_default(*prev_time),
                    orig: time.orig.clone(),
                };
                *weight = common::InputField {
                    input: prev_weight.map(|v| v.to_string()).unwrap_or_default(),
                    parsed: some_or_default(*prev_weight),
                    orig: weight.orig.clone(),
                };
                *rpe = common::InputField {
                    input: prev_rpe.map(|v| v.to_string()).unwrap_or_default(),
                    parsed: some_or_default(*prev_rpe),
                    orig: rpe.orig.clone(),
                };
            }
        }

        Msg::StartGuidedTrainingSession => {
            model.guide = Some(Guide::new(data_model.settings.beep_volume));
            update_guide_timer(model);
            update_streams(model, orders);
            orders.notify(data::Msg::StartTrainingSession(model.training_session_id));
            if let Some(guide) = &model.guide {
                if data_model.settings.automatic_metronome {
                    update_metronome(&model.form.sections[guide.section_idx], orders);
                }
                orders.notify(data::Msg::UpdateTrainingSession(
                    guide.section_idx,
                    guide.timer.to_timer_state(),
                ));
            }
            show_section_notification(model);
            Url::go_and_push(
                &crate::Urls::new(&data_model.base_url)
                    .training_session()
                    .add_hash_path_part(model.training_session_id.to_string())
                    .add_hash_path_part("guide"),
            );
        }
        Msg::ContinueGuidedTrainingSession(ongoing_training_session) => {
            model.guide = Some(Guide::from_ongoing_training_session(
                ongoing_training_session.section_idx,
                ongoing_training_session.section_start_time,
                data_model.settings.beep_volume,
            ));
            model
                .guide
                .as_mut()
                .unwrap()
                .timer
                .restore(ongoing_training_session.timer_state);
            if let Some(guide) = &model.guide {
                if data_model.settings.automatic_metronome
                    && guide.section_idx < model.form.sections.len()
                {
                    update_metronome(&model.form.sections[guide.section_idx], orders);
                }
            }
            update_streams(model, orders);
            show_section_notification(model);
            orders.force_render_now().send_msg(Msg::ScrollToSection);
            Url::go_and_push(
                &crate::Urls::new(&data_model.base_url)
                    .training_session()
                    .add_hash_path_part(model.training_session_id.to_string())
                    .add_hash_path_part("guide"),
            );
        }
        Msg::UpdateGuidedTrainingSession => {
            if let Some(guide) = &mut model.guide {
                match &model.form.sections[guide.section_idx] {
                    FormSection::Set { exercises } => {
                        let exercise = &exercises[0];
                        if not(show_guide_timer(exercise)) {
                            guide.timer.reset_time = 0;
                        } else if let Some(target_time) = exercise.target_time {
                            if let Some(time) = guide.timer.time.1 {
                                if time <= 0 {
                                    if let Some(target_reps) = exercise.target_reps {
                                        orders.send_msg(Msg::RepsChanged(
                                            guide.section_idx,
                                            0,
                                            target_reps.to_string(),
                                        ));
                                    }
                                    orders.send_msg(Msg::TimeChanged(
                                        guide.section_idx,
                                        0,
                                        target_time.to_string(),
                                    ));
                                    orders.send_msg(Msg::GoToNextSection);
                                }
                            }
                        }
                    }
                    FormSection::Rest { automatic, .. } => {
                        if let Some(time) = guide.timer.time.1 {
                            if time <= 0 && *automatic {
                                orders.send_msg(Msg::GoToNextSection);
                            }
                        }
                    }
                }
                guide.timer.update(&model.audio_context);
            }
        }
        Msg::StartPauseGuideTimer => {
            if let Some(guide) = &mut model.guide {
                guide.timer.start_pause();
                orders.notify(data::Msg::UpdateTrainingSession(
                    guide.section_idx,
                    guide.timer.to_timer_state(),
                ));
            }
            update_streams(model, orders);
        }
        Msg::GoToPreviousSection => {
            if let Some(guide) = &mut model.guide {
                guide.section_idx -= 1;
                guide.section_start_time = Utc::now();
            }
            update_guide_timer(model);
            update_streams(model, orders);
            if let Some(guide) = &mut model.guide {
                if data_model.settings.automatic_metronome {
                    update_metronome(&model.form.sections[guide.section_idx], orders);
                }
                if let Some(ongoing_training_session) = &data_model.ongoing_training_session {
                    orders.notify(data::Msg::UpdateTrainingSession(
                        ongoing_training_session.section_idx - 1,
                        guide.timer.to_timer_state(),
                    ));
                }
            }
            close_notification(model);
            orders.force_render_now().send_msg(Msg::ScrollToSection);
        }
        Msg::GoToNextSection => {
            if let Some(guide) = &mut model.guide {
                guide.section_idx += 1;
                if guide.section_idx == model.form.sections.len() {
                    model.guide = None;
                    close_notification(model);
                    orders
                        .send_msg(Msg::PauseMetronome)
                        .notify(data::Msg::EndTrainingSession);
                } else {
                    guide.section_start_time = Utc::now();

                    if data_model.settings.automatic_metronome {
                        update_metronome(&model.form.sections[guide.section_idx], orders);
                    }

                    show_section_notification(model);
                }
            }
            update_guide_timer(model);
            update_streams(model, orders);
            if let Some(guide) = &mut model.guide {
                if let Some(ongoing_training_session) = &data_model.ongoing_training_session {
                    orders.notify(data::Msg::UpdateTrainingSession(
                        ongoing_training_session.section_idx + 1,
                        guide.timer.to_timer_state(),
                    ));
                }
            }
            orders
                .force_render_now()
                .send_msg(Msg::UpdateGuidedTrainingSession)
                .send_msg(Msg::ScrollToSection);
            if model.form.changed() {
                orders.send_msg(Msg::SaveTrainingSession);
            }
        }
        Msg::ScrollToSection => {
            if let Some(guide) = &mut model.guide {
                let mut options = web_sys::ScrollIntoViewOptions::new();
                options.behavior(web_sys::ScrollBehavior::Smooth);
                options.block(web_sys::ScrollLogicalPosition::Center);
                if let Some(element) = guide.element.get() {
                    element.scroll_into_view_with_scroll_into_view_options(&options);
                }
            }
        }

        Msg::EditTrainingSession => {
            model.editing = true;
            Url::go_and_push(
                &crate::Urls::new(&data_model.base_url)
                    .training_session()
                    .add_hash_path_part(model.training_session_id.to_string())
                    .add_hash_path_part("edit"),
            );
        }
        Msg::SaveTrainingSession => {
            model.loading = true;
            orders.notify(data::Msg::ModifyTrainingSession(
                model.training_session_id,
                Some(model.form.notes.clone()),
                Some(
                    model
                        .form
                        .sections
                        .iter()
                        .flat_map(|s| match s {
                            FormSection::Set { exercises } => exercises
                                .iter()
                                .map(|e| data::TrainingSessionElement::Set {
                                    exercise_id: e.exercise_id,
                                    reps: e.reps.parsed.filter(|reps| *reps > 0),
                                    time: e.time.parsed.filter(|time| *time > 0),
                                    weight: e.weight.parsed.filter(|weight| *weight > 0.0),
                                    rpe: e.rpe.parsed.filter(|rpe| *rpe > 0.0),
                                    target_reps: e.target_reps,
                                    target_time: e.target_time,
                                    target_weight: e.target_weight,
                                    target_rpe: e.target_rpe,
                                    automatic: e.automatic,
                                })
                                .collect(),
                            FormSection::Rest {
                                target_time,
                                automatic,
                            } => vec![data::TrainingSessionElement::Rest {
                                target_time: if *target_time > 0 {
                                    Some(*target_time)
                                } else {
                                    None
                                },
                                automatic: *automatic,
                            }],
                        })
                        .collect::<Vec<_>>(),
                ),
            ));
        }
        Msg::DataEvent(event) => {
            match event {
                data::Event::DataChanged | data::Event::TrainingSessionModifiedOk => {
                    model.form = init_form(
                        data_model.training_sessions.get(&model.training_session_id),
                        data_model,
                    );
                    model.loading = false;
                    update_guide_timer(model);
                    update_streams(model, orders);
                }
                data::Event::TrainingSessionModifiedErr => {
                    model.loading = false;
                    update_guide_timer(model);
                    update_streams(model, orders);
                }
                data::Event::BeepVolumeChanged => {
                    model.smt.metronome.beep_volume = data_model.settings.beep_volume;
                    model.smt.timer.beep_volume = data_model.settings.beep_volume;
                    if let Some(guide) = &mut model.guide {
                        guide.timer.beep_volume = data_model.settings.beep_volume;
                    }
                }
                _ => {}
            };
        }

        Msg::ShowSMTDialog => {
            model.dialog = Dialog::StopwatchMetronomTimer;
        }
        Msg::ShowOptionsDialog(section_idx, exercise_idx) => {
            model.dialog = Dialog::Options(section_idx, exercise_idx);
        }
        Msg::ShowReplaceExerciseDialog(section_idx, exercise_idx) => {
            model.dialog = Dialog::ReplaceExercise(section_idx, exercise_idx, String::new());
        }
        Msg::SearchTermChanged(search_term) => {
            if let Dialog::ReplaceExercise(_, _, st) = &mut model.dialog {
                *st = search_term;
            }
        }
        Msg::CreateExercise => {
            model.loading = true;
            if let Dialog::ReplaceExercise(_, _, search_term) = &model.dialog {
                orders.notify(data::Msg::CreateExercise(search_term.trim().to_string()));
            };
        }
        Msg::ReplaceExercise(section_idx, exercise_idx, new_exercise_id) => {
            replace_exercise(
                &mut model.form.sections,
                section_idx,
                exercise_idx,
                new_exercise_id,
                &data_model.exercises,
            );
            orders
                .send_msg(Msg::SaveTrainingSession)
                .send_msg(Msg::CloseDialog);
        }
        Msg::CloseDialog => {
            model.dialog = Dialog::Hidden;
        }

        Msg::UpdateStopwatchMetronomTimer => {
            model.smt.stopwatch.update();
            model.smt.metronome.update(&model.audio_context);
            model.smt.timer.update(&model.audio_context);
        }

        Msg::StartPauseStopwatch => {
            model.smt.stopwatch.start_pause();
            update_streams(model, orders);
        }
        Msg::ResetStopwatch => {
            model.smt.stopwatch.reset();
        }
        Msg::ToggleStopwatch => {
            model.smt.stopwatch.toggle();
            update_streams(model, orders);
        }

        Msg::StartMetronome(interval) => {
            model.smt.metronome.interval = interval;
            model.smt.metronome.stressed_beat = 1;
            model.smt.metronome.start(&model.audio_context);
            update_streams(model, orders);
        }
        Msg::PauseMetronome => {
            model.smt.metronome.pause();
            update_streams(model, orders);
        }
        Msg::StartPauseMetronome => {
            model.smt.metronome.start_pause(&model.audio_context);
            update_streams(model, orders);
        }
        Msg::MetronomeIntervalChanged(interval) => {
            model.smt.metronome.interval = interval.parse::<u32>().unwrap_or(1);
        }
        Msg::MetronomeStressChanged(stressed_beat) => {
            model.smt.metronome.stressed_beat = stressed_beat.parse::<u32>().unwrap_or(1);
        }

        Msg::StartPauseTimer => {
            model.smt.timer.start_pause();
            update_streams(model, orders);
        }
        Msg::ResetTimer => {
            model.smt.timer.reset();
        }
        Msg::TimerTimeChanged(time) => match time.parse::<i64>() {
            Ok(parsed_time) => {
                model.smt.timer.time = (time, Some(parsed_time));
                model.smt.timer.reset_time = parsed_time;
            }
            Err(_) => model.smt.timer.time = (time, None),
        },
    }
}

fn update_streams(model: &mut Model, orders: &mut impl Orders<Msg>) {
    if let Some(guide) = &mut model.guide {
        guide.stream =
            if guide.timer.is_active() {
                Some(orders.stream_with_handle(streams::interval(1000, || {
                    Msg::UpdateGuidedTrainingSession
                })))
            } else {
                None
            }
    };
    model.timer_stream = if model.smt.stopwatch.is_active()
        || model.smt.metronome.is_active()
        || model.smt.timer.is_active()
    {
        Some(
            orders.stream_with_handle(streams::interval(100, || Msg::UpdateStopwatchMetronomTimer)),
        )
    } else {
        None
    };
}

fn update_guide_timer(model: &mut Model) {
    if model.form.sections.is_empty() {
        return;
    }

    if let Some(guide) = &mut model.guide {
        guide.timer.unset();
        let elapsed_time = (Utc::now() - guide.section_start_time).num_seconds();
        match &model.form.sections[guide.section_idx] {
            FormSection::Set { exercises } => {
                let exercise = &exercises[0];
                if not(show_guide_timer(exercise)) {
                    return;
                }
                if let Some(target_time) = exercise.target_time {
                    let target_time = if let Some(target_reps) = exercise.target_reps {
                        target_time * target_reps
                    } else {
                        target_time
                    };
                    guide.timer.set(i64::from(target_time) - elapsed_time);
                    if exercise.automatic {
                        guide.timer.start();
                    }
                }
            }
            FormSection::Rest { target_time, .. } => {
                if *target_time > 0 {
                    guide.timer.set(i64::from(*target_time) - elapsed_time);
                    guide.timer.start();
                }
            }
        }
    }
}

fn update_metronome(form_section: &FormSection, orders: &mut impl Orders<Msg>) {
    match form_section {
        FormSection::Set { exercises } => {
            let exercise = &exercises[0];
            if exercise.target_reps.is_some() {
                if let Some(target_time) = exercise.target_time {
                    orders.send_msg(Msg::StartMetronome(target_time));
                }
            }
        }
        FormSection::Rest { .. } => {
            orders.send_msg(Msg::PauseMetronome);
        }
    }
}

fn show_notification(model: &mut Model, title: &str, body: Option<String>) {
    if let Some(notification) = &model.notification {
        notification.close();
    }
    let mut options = web_sys::NotificationOptions::new();
    if let Some(body) = body {
        options.body(&body);
    }
    model.notification = web_sys::Notification::new_with_options(title, &options).ok();
}

fn close_notification(model: &mut Model) {
    if let Some(notification) = &model.notification {
        notification.close();
    }
}

fn show_section_notification(model: &mut Model) {
    if let Some(guide) = &mut model.guide {
        if guide.section_idx < model.form.sections.len() {
            let title;
            let body;
            match &model.form.sections[guide.section_idx] {
                FormSection::Set { exercises } => {
                    let exercise = &exercises[0];
                    title = exercise.exercise_name.clone();
                    let mut previously = format_set(
                        exercise.prev_reps,
                        exercise.prev_time,
                        exercise.prev_weight,
                        exercise.prev_rpe,
                    );
                    if not(previously.is_empty()) {
                        previously = format!("Previously:\n{previously}\n");
                    }
                    let mut target = format_set(
                        exercise.target_reps,
                        exercise.target_time,
                        exercise.target_weight,
                        exercise.target_rpe,
                    );
                    if not(target.is_empty()) {
                        target = format!("Target:\n{target}\n");
                    }
                    body = Some(format!("{previously}{target}"));
                }
                FormSection::Rest { target_time, .. } => {
                    title = String::from("Rest");
                    body = if *target_time > 0 {
                        Some(format!("{target_time} s"))
                    } else {
                        None
                    };
                }
            }

            show_notification(model, &title, body);
        }
    }
}

fn replace_exercise(
    sections: &mut [FormSection],
    section_idx: usize,
    exercise_idx: usize,
    new_exercise_id: u32,
    data_exercises: &BTreeMap<u32, data::Exercise>,
) {
    let mut current_exercise_id = None;
    let mut current_exercise_ids = vec![];
    for mut section in sections.iter_mut().skip(section_idx) {
        if let FormSection::Set { exercises } = &mut section {
            let ids = exercises.iter().map(|e| e.exercise_id).collect::<Vec<_>>();
            if current_exercise_ids.is_empty() {
                current_exercise_ids = ids;
            } else if current_exercise_ids != ids {
                break;
            }
            for exercise in exercises.iter_mut().skip(exercise_idx) {
                let ExerciseForm {
                    exercise_id,
                    exercise_name,
                    ..
                } = exercise;
                match current_exercise_id {
                    None => current_exercise_id = Some(*exercise_id),
                    Some(id) => {
                        if *exercise_id != id {
                            break;
                        }
                    }
                }
                *exercise_id = new_exercise_id;
                *exercise_name = data_exercises[&new_exercise_id].name.clone();
            }
        }
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model, data_model: &data::Model) -> Node<Msg> {
    if data_model.training_sessions.is_empty() && data_model.loading_training_sessions {
        common::view_page_loading()
    } else if let Some(training_session) =
        data_model.training_sessions.get(&model.training_session_id)
    {
        if model.dialog == Dialog::Hidden {
            div![
                view_title(training_session, data_model),
                if model.editing || model.guide.is_some() {
                    nodes![view_training_session_form(model, data_model)]
                } else {
                    nodes![
                        view_table(training_session, data_model),
                        view_notes(training_session),
                        common::view_fab("edit", |_| Msg::EditTrainingSession)
                    ]
                }
            ]
        } else {
            div![
                Node::NoChange,
                Node::NoChange,
                view_dialog(
                    &model.dialog,
                    &model.smt,
                    model.loading,
                    &data_model.exercises
                ),
            ]
        }
    } else {
        common::view_error_not_found("Training session")
    }
}

fn view_title(training_session: &data::TrainingSession, data_model: &data::Model) -> Node<Msg> {
    let title = if let Some(routine) = data_model
        .routines
        .get(&training_session.routine_id.unwrap_or(0))
    {
        div![
            p![C!["mb-3"], training_session.date.to_string()],
            p![a![
                attrs! {
                    At::Href => crate::Urls::new(&data_model.base_url).routine().add_hash_path_part(routine.id.to_string()),
                },
                &routine.name
            ]]
        ]
    } else {
        span![training_session.date.to_string()]
    };
    common::view_title(&title, 3)
}

fn view_table(training_session: &data::TrainingSession, data_model: &data::Model) -> Node<Msg> {
    div![
        C!["table-container"],
        C!["mt-4"],
        table![
            C!["table"],
            C!["is-fullwidth"],
            C!["is-hoverable"],
            tbody![
                tr![td![], td![]],
                training_session
                    .elements
                    .iter()
                    .map(|e| match e {
                        data::TrainingSessionElement::Set {
                            exercise_id,
                            reps,
                            time,
                            weight,
                            rpe,
                            ..
                        } => {
                            tr![
                                td![a![
                                    attrs! {
                                        At::Href => {
                                            crate::Urls::new(&data_model.base_url)
                                                .exercise()
                                                .add_hash_path_part(exercise_id.to_string())
                                        }
                                    },
                                    data_model.exercises.get(exercise_id).map_or_else(
                                        || format!("Exercise#{exercise_id}"),
                                        |e| e.name.clone()
                                    )
                                ]],
                                td![
                                    C!["is-vcentered"],
                                    C!["has-text-centered"],
                                    span![
                                        style! {St::WhiteSpace => "nowrap" },
                                        format_set(*reps, *time, *weight, *rpe)
                                    ]
                                ],
                            ]
                        }
                        _ => {
                            tr![td![C!["p-1"]], td![C!["p-1"]]]
                        }
                    })
                    .collect::<Vec<_>>()
            ],
        ]
    ]
}

fn view_notes(training_session: &data::TrainingSession) -> Node<Msg> {
    if let Some(notes) = &training_session.notes {
        if notes.is_empty() {
            empty![]
        } else {
            div![C!["m-3"], h1![C!["title"], C!["is-5"], "Notes"], p![notes]]
        }
    } else {
        empty![]
    }
}

fn view_training_session_form(model: &Model, data_model: &data::Model) -> Node<Msg> {
    let valid = model.form.valid();
    let save_disabled = not(model.form.changed()) || not(valid);
    let mut form: std::vec::Vec<seed::virtual_dom::Node<Msg>> = nodes![];

    for (section_idx, section) in model.form.sections.iter().enumerate() {
        if let Some(guide) = &model.guide {
            if guide.section_idx == section_idx && section_idx != 0 {
                form.push(div![
                    C!["has-text-centered"],
                    C!["m-5"],
                    button![
                        C!["button"],
                        C!["is-link"],
                        ev(Ev::Click, |_| Msg::GoToPreviousSection),
                        span![C!["icon"], i![C!["fas fa-angles-up"]]]
                    ]
                ]);
            }
        }

        match section {
            FormSection::Set {
                exercises: exercise_forms,
            } => {
                form.push(
                    div![
                        if let Some(guide) = &model.guide {
                            if guide.section_idx == section_idx {
                                el_ref(&guide.element)
                            } else {
                                el_ref(&ElRef::new())
                            }
                        } else {
                            el_ref(&ElRef::new())
                        },
                        C!["message"],
                        C!["is-info"],
                        C!["has-background-white-bis"],
                        IF![model.guide.as_ref().map_or(false, |guide| guide.section_idx != section_idx) => C!["is-semitransparent"]],
                        div![
                            C!["message-body"],
                            C!["p-3"],
                            exercise_forms.iter().map(|s| {
                                let position = s.position;
                                let input_fields = div![
                                        C!["field"],
                                        C!["has-addons"],
                                        div![
                                            C!["control"],
                                            C!["has-icons-right"],
                                            C!["has-text-right"],
                                            input_ev(Ev::Input, move |v| Msg::RepsChanged(section_idx, position, v)),
                                            keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                                                IF!(
                                                    not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                                        Msg::SaveTrainingSession
                                                    }
                                                )
                                            }),
                                            input![
                                                C!["input"],
                                                C!["has-text-right"],
                                                C![IF![not(s.reps.valid()) => "is-danger"]],
                                                C![IF![s.reps.changed() => "is-info"]],
                                                attrs! {
                                                    At::Type => "number",
                                                    At::Min => 0,
                                                    At::Max => 999,
                                                    At::Step => 1,
                                                    At::Size => 2,
                                                    At::Value => s.reps.input,
                                                }
                                            ],
                                            span![C!["icon"], C!["is-small"], C!["is-right"], "✕"],
                                        ],
                                        div![
                                            C!["control"],
                                            C!["has-icons-right"],
                                            C!["has-text-right"],
                                            input_ev(Ev::Input, move |v| Msg::TimeChanged(section_idx, position, v)),
                                            keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                                                IF!(
                                                    not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                                        Msg::SaveTrainingSession
                                                    }
                                                )
                                            }),
                                            input![
                                                C!["input"],
                                                C!["has-text-right"],
                                                C![IF![not(s.time.valid()) => "is-danger"]],
                                                C![IF![s.time.changed() => "is-info"]],
                                                attrs! {
                                                    At::Type => "number",
                                                    At::Min => 0,
                                                    At::Max => 999,
                                                    At::Step => 1,
                                                    At::Size => 2,
                                                    At::Value => s.time.input,
                                                },
                                            ],
                                            span![C!["icon"], C!["is-small"], C!["is-right"], "s"],
                                        ],
                                        div![
                                            C!["control"],
                                            C!["has-icons-right"],
                                            C!["has-text-right"],
                                            input_ev(Ev::Input, move |v| Msg::WeightChanged(section_idx, position, v)),
                                            keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                                                IF!(
                                                    not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                                        Msg::SaveTrainingSession
                                                    }
                                                )
                                            }),
                                            input![
                                                C!["input"],
                                                C!["has-text-right"],
                                                C![IF![not(s.weight.valid()) => "is-danger"]],
                                                C![IF![s.weight.changed() => "is-info"]],
                                                attrs! {
                                                    At::from("inputmode") => "numeric",
                                                    At::Size => 3,
                                                    At::Value => s.weight.input,
                                                },
                                            ],
                                            span![C!["icon"], C!["is-small"], C!["is-right"], "kg"],
                                        ],
                                        div![
                                            C!["control"],
                                            C!["has-icons-left"],
                                            C!["has-text-right"],
                                            input_ev(Ev::Input, move |v| Msg::RPEChanged(section_idx, position, v)),
                                            keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                                                IF!(
                                                    not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                                        Msg::SaveTrainingSession
                                                    }
                                                )
                                            }),
                                            input![
                                                C!["input"],
                                                C!["has-text-right"],
                                                C![IF![not(s.rpe.valid()) => "is-danger"]],
                                                C![IF![s.rpe.changed() => "is-info"]],
                                                attrs! {
                                                    At::from("inputmode") => "numeric",
                                                    At::Size => 2,
                                                    At::Value => s.rpe.input,
                                                },
                                            ],
                                            span![C!["icon"], C!["is-small"], C!["is-left"], "@"],
                                        ],
                                    ];
                                div![
                                    C!["field"],
                                    div![
                                        C!["has-text-weight-bold"],
                                        C!["mb-2"],
                                        div![
                                            C!["is-flex"],
                                            C!["is-justify-content-space-between"],
                                            a![
                                                attrs! {
                                                    At::Href => {
                                                        crate::Urls::new(&data_model.base_url)
                                                            .exercise()
                                                            .add_hash_path_part(s.exercise_id.to_string())
                                                    },
                                                    At::from("tabindex") => -1
                                                },
                                                &s.exercise_name
                                            ],
                                            div![a![
                                                ev(Ev::Click, move |_| Msg::ShowOptionsDialog(section_idx, position)),
                                                span![C!["icon"], i![C!["fas fa-ellipsis-vertical"]]]
                                            ]],
                                        ],
                                    ],
                                    if let Some(guide) = &model.guide {
                                        if guide.timer.is_set() && guide.section_idx == section_idx {
                                            view_guide_timer(guide)
                                        } else {
                                            input_fields
                                        }
                                    } else {
                                        input_fields
                                    },
                                    {
                                        let target = format_set(s.target_reps, s.target_time, s.target_weight, s.target_rpe);
                                        let previous = format_set(s.prev_reps, s.prev_time, s.prev_weight, s.prev_rpe);
                                        p![
                                            IF![not(target.is_empty()) =>
                                                span![
                                                    C!["icon-text"],
                                                    C!["mr-4"],
                                                    span![C!["icon"], i![C!["fas fa-bullseye"]]],
                                                    a![
                                                        ev(Ev::Click, move |_| Msg::EnterTargetValues(section_idx, position)),
                                                        target
                                                    ]
                                                ]
                                            ],
                                            IF![not(previous.is_empty()) =>
                                                span![
                                                    C!["icon-text"],
                                                    C!["mr-4"],
                                                    span![C!["icon"], i![C!["fas fa-clipboard-list"]]],
                                                    a![
                                                        ev(Ev::Click, move |_| Msg::EnterPreviousValues(section_idx, position)),
                                                        previous
                                                    ]
                                                ]
                                            ],
                                            IF![
                                                s.automatic =>
                                                span![
                                                    C!["icon"],
                                                    common::automatic_icon()
                                                ]
                                            ]
                                        ]
                                    }
                                ]
                            })
                        ]
                    ]
                );
            }
            FormSection::Rest {
                target_time,
                automatic,
            } => {
                form.push(div![
                    if let Some(guide) = &model.guide {
                        if guide.section_idx == section_idx {
                            el_ref(&guide.element)
                        } else {
                            el_ref(&ElRef::new())
                        }
                    } else {
                        el_ref(&ElRef::new())
                    },
                    C!["message"],
                    C!["is-success"],
                    C!["has-background-white-bis"],
                    IF![model.guide.as_ref().map_or(false, |guide| guide.section_idx != section_idx) => C!["is-semitransparent"]],
                    div![
                        C!["message-body"],
                        C!["p-3"],
                        div![C!["field"], C!["has-text-weight-bold"], plain!["Rest"]],
                        if let Some(guide) = &model.guide {
                            if guide.timer.is_set() && guide.section_idx == section_idx {
                                view_guide_timer(guide)
                            } else {
                                empty![]
                            }
                        } else {
                            empty![]
                        },
                        div![
                            IF![
                                *target_time > 0 =>
                                span![
                                    C!["icon-text"],
                                    C!["mr-4"],
                                    span![C!["mr-2"], i![C!["fas fa-clock-rotate-left"]]],
                                    span![target_time, " s"]
                                ]
                            ],
                            IF![
                                *automatic =>
                                span![
                                    C!["icon"],
                                    common::automatic_icon()
                                ]
                            ]
                        ],
                    ]
                ]);
            }
        }

        if let Some(guide) = &model.guide {
            if guide.section_idx == section_idx {
                form.push(div![
                    C!["has-text-centered"],
                    C!["m-5"],
                    button![
                        C!["button"],
                        C!["is-link"],
                        ev(Ev::Click, |_| Msg::GoToNextSection),
                        if section_idx < model.form.sections.len() - 1 {
                            span![C!["icon"], i![C!["fas fa-angles-down"]]]
                        } else {
                            span![C!["icon"], i![C!["fas fa-check"]]]
                        },
                    ]
                ]);
            }
        }
    }

    div![
        C!["container"],
        C!["mx-2"],
        IF![
            model.guide.is_none() =>
            div![
                C!["has-text-centered"],
                C!["m-5"],
                button![
                    C!["button"],
                    C!["is-link"],
                    ev(Ev::Click, |_| Msg::StartGuidedTrainingSession),
                    span![C!["icon"], i![C!["fas fa-play"]]]
                ]
            ]
        ],
        form![
            attrs! {
                At::Action => "javascript:void(0);",
                At::OnKeyPress => "if (event.which == 13) return false;"
            },
            &form,
            div![
                C!["field"],
                label![C!["label"], "Notes"],
                input_ev(Ev::Input, Msg::NotesChanged),
                textarea![
                    C!["textarea"],
                    C![IF![model.form.notes_changed => "is-info"]],
                    &model.form.notes,
                ]
            ],
        ],
        IF![
            model.guide.is_none() =>
            button![
                C!["button"],
                C!["is-fab"],
                C!["is-medium"],
                C!["is-link"],
                C![IF![not(valid) => "is-danger"]],
                C![IF![model.loading => "is-loading"]],
                attrs![
                    At::Disabled => save_disabled.as_at_value(),
                ],
                ev(Ev::Click, |_| Msg::SaveTrainingSession),
                span![C!["icon"], i![C!["fas fa-save"]]]
            ]
        ]
    ]
}

fn view_guide_timer(guide: &Guide) -> Node<Msg> {
    div![
        C!["is-size-1"],
        C!["has-text-centered"],
        ev(Ev::Click, |_| Msg::StartPauseGuideTimer),
        &guide.timer.time.0,
        " s"
    ]
}

fn view_dialog(
    dialog: &Dialog,
    smt: &StopwatchMetronomTimer,
    loading: bool,
    exercises: &BTreeMap<u32, data::Exercise>,
) -> Node<Msg> {
    let content = match dialog {
        Dialog::Hidden => nodes![],
        Dialog::StopwatchMetronomTimer => view_smt_dialog(smt),
        Dialog::Options(section_idx, exercise_idx) => {
            view_options_dialog(*section_idx, *exercise_idx)
        }
        Dialog::ReplaceExercise(section_idx, exercise_idx, search_term) => {
            view_replace_exercise_dialog(
                *section_idx,
                *exercise_idx,
                search_term,
                loading,
                exercises,
            )
        }
    };

    div![
        C!["modal"],
        C!["is-active"],
        div![C!["modal-background"], ev(Ev::Click, |_| Msg::CloseDialog)],
        div![
            C!["modal-content"],
            div![
                C!["box"],
                C!["mx-2"],
                content,
                button![
                    C!["modal-close"],
                    C!["is-large"],
                    ev(Ev::Click, |_| Msg::CloseDialog),
                ]
            ]
        ]
    ]
}

fn view_smt_dialog(smt: &StopwatchMetronomTimer) -> Vec<Node<Msg>> {
    nodes![
        div![
            C!["block"],
            label![C!["subtitle"], "Stopwatch"],
            div![
                C!["container"],
                C!["has-text-centered"],
                C!["p-5"],
                p![C!["title"], C!["is-size-1"],
                ev(Ev::Click, |_| Msg::ToggleStopwatch),
                {
                    #[allow(clippy::cast_precision_loss)]
                    let time = smt.stopwatch.time as f64 / 1000.;
                    format!("{time:.1}")
                }],
                button![
                    C!["button"],
                    C!["mt-1"],
                    C!["mx-3"],
                    attrs! {At::Type => "button"},
                    ev(Ev::Click, |_| Msg::StartPauseStopwatch),
                    if smt.stopwatch.is_active() {
                        span![C!["icon"], i![C!["fas fa-pause"]]]
                    } else {
                        span![C!["icon"], i![C!["fas fa-play"]]]
                    }
                ],
                button![
                    C!["button"],
                    C!["mt-1"],
                    C!["mx-3"],
                    attrs! {At::Type => "button"},
                    ev(Ev::Click, |_| Msg::ResetStopwatch),
                    span![C!["icon"], i![C!["fas fa-rotate-left"]]]
                ],
            ],
        ],
        div![
            C!["block"],
            label![C!["subtitle"], "Metronome"],
            div![
                C!["container"],
                C!["p-5"],
                div![
                    C!["field"],
                    C!["is-grouped"],
                    C!["is-grouped-centered"],
                    div![
                        C!["field"],
                        C!["mx-4"],
                        label![C!["label"], "Interval"],
                        div![
                            C!["control"],
                            input_ev(Ev::Change, Msg::MetronomeIntervalChanged),
                            div![
                                C!["select"],
                                select![
                                    (1..61).map(|i| {
                                        option![
                                            &i,
                                            attrs! {
                                                At::Value => i,
                                                At::Selected => (i == smt.metronome.interval).as_at_value()
                                            }
                                        ]
                                    }).collect::<Vec<_>>()
                                ]
                            ]
                        ]
                    ],
                    div![
                        C!["field"],
                        C!["mx-4"],
                        label![C!["label"], "Stress"],
                        div![
                            C!["control"],
                            input_ev(Ev::Change, Msg::MetronomeStressChanged),
                            div![
                                C!["select"],
                                select![
                                    (1..13).map(|i| {
                                        option![
                                            &i,
                                            attrs! {
                                                At::Value => i,
                                                At::Selected => (i == smt.metronome.stressed_beat).as_at_value()
                                            }
                                        ]
                                    }).collect::<Vec<_>>()
                                ]
                            ]
                        ]
                    ],
                    div![
                        C!["field"],
                        C!["has-text-centered"],
                        C!["mx-4"],
                        label![C!["label"], raw!["&nbsp;"]],
                        div![
                            C!["control"],
                            button![
                                C!["button"],
                                attrs! {At::Type => "button"},
                                ev(Ev::Click, |_| Msg::StartPauseMetronome),
                                if smt.metronome.is_active() {
                                    span![C!["icon"], i![C!["fas fa-pause"]]]
                                } else {
                                    span![C!["icon"], i![C!["fas fa-play"]]]
                                }
                            ],
                        ]
                    ]
                ]
            ],
        ],
        div![
            C!["block"],
            label![C!["subtitle"], "Timer"],
            div![
                C!["container"],
                C!["has-text-centered"],
                C!["p-5"],
                div![C!["field"],
                div![
                    C!["control"],
                    input_ev(Ev::Input, Msg::TimerTimeChanged),
                    input![
                        C!["input"],
                        C!["title"],
                        C!["is-size-1"],
                        C!["has-text-centered"],
                        C![IF![not(&smt.timer.time.1.is_some()) => "is-danger"]],
                        style! {
                            St::Height => "auto",
                            St::Width => "auto",
                            St::Padding => 0,
                        },
                        attrs! {
                            At::Type => "number",
                            At::Value => &smt.timer.time.0,
                            At::Min => 0,
                            At::Max => 9999,
                            At::Step => 1,
                            At::Size => 4
                        },
                    ]
                ]],
                button![
                    C!["button"],
                    C!["mt-5"],
                    C!["mx-3"],
                    attrs! {At::Type => "button"},
                    ev(Ev::Click, |_| Msg::StartPauseTimer),
                    if smt.timer.is_active() {
                        span![C!["icon"], i![C!["fas fa-pause"]]]
                    } else {
                        span![C!["icon"], i![C!["fas fa-play"]]]
                    }
                ],
                button![
                    C!["button"],
                    C!["mt-5"],
                    C!["mx-3"],
                    attrs! {At::Type => "button"},
                    ev(Ev::Click, |_| Msg::ResetTimer),
                    span![C!["icon"], i![C!["fas fa-rotate-left"]]]
                ],
            ],
        ],
    ]
}

fn view_options_dialog(section_idx: usize, exercise_idx: usize) -> Vec<Node<Msg>> {
    nodes![p![a![
        C!["has-text-weight-bold"],
        ev(Ev::Click, move |_| Msg::ShowReplaceExerciseDialog(
            section_idx,
            exercise_idx
        )),
        span![
            C!["icon-text"],
            span![C!["icon"], i![C!["fas fa-arrow-right-arrow-left"]]],
            span!["Replace exercise"],
        ]
    ]]]
}

fn view_replace_exercise_dialog(
    section_idx: usize,
    exercise_idx: usize,
    search_term: &str,
    loading: bool,
    exercises: &BTreeMap<u32, data::Exercise>,
) -> Vec<Node<Msg>> {
    let section_idx = section_idx;
    let exercise_idx = exercise_idx;
    common::view_exercises_with_search(
        exercises,
        search_term,
        Msg::SearchTermChanged,
        |_| Msg::CreateExercise,
        loading,
        move |exercise_id| Msg::ReplaceExercise(section_idx, exercise_idx, exercise_id),
    )
}

fn format_set(
    reps: Option<u32>,
    time: Option<u32>,
    weight: Option<f32>,
    rpe: Option<f32>,
) -> String {
    let mut parts = vec![];

    if let Some(reps) = reps {
        parts.push(reps.to_string());
    }

    if let Some(time) = time {
        parts.push(format!("{time} s"));
    }

    if let Some(weight) = weight {
        parts.push(format!("{weight} kg"));
    }

    let mut result = parts.join(" × ");

    if let Some(rpe) = rpe {
        result.push_str(&format!(" @ {rpe}"));
    }

    result
}

fn some_or_default<T: Default>(value: Option<T>) -> Option<T> {
    if value.is_some() {
        value
    } else {
        Some(T::default())
    }
}

fn show_guide_timer(exercise: &ExerciseForm) -> bool {
    exercise.target_time.is_some() && (exercise.target_reps.is_none() || exercise.automatic)
}

#[cfg(test)]
mod tests {
    use crate::common::InputField;

    use super::*;

    #[test]
    fn test_replace_exercise_first_set() {
        let mut sections = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
            set(vec![exercise(2, 1)]),
            rest(2),
            set(vec![exercise(3, 1)]),
            rest(3),
            set(vec![exercise(4, 0)]),
            rest(4),
            set(vec![exercise(5, 0)]),
            rest(5),
        ];
        replace_exercise(&mut sections, 0, 0, 2, &exercises(2));
        assert_eq!(
            sections,
            vec![
                set(vec![exercise(0, 2)]),
                rest(0),
                set(vec![exercise(1, 2)]),
                rest(1),
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(3, 1)]),
                rest(3),
                set(vec![exercise(4, 0)]),
                rest(4),
                set(vec![exercise(5, 0)]),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_replace_exercise_second_set() {
        let mut sections = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
            set(vec![exercise(2, 1)]),
            rest(2),
            set(vec![exercise(3, 1)]),
            rest(3),
            set(vec![exercise(4, 0)]),
            rest(4),
            set(vec![exercise(5, 0)]),
            rest(5),
        ];
        replace_exercise(&mut sections, 2, 0, 2, &exercises(2));
        assert_eq!(
            sections,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 2)]),
                rest(1),
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(3, 1)]),
                rest(3),
                set(vec![exercise(4, 0)]),
                rest(4),
                set(vec![exercise(5, 0)]),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_replace_exercise_penultimate_set() {
        let mut sections = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
            set(vec![exercise(2, 1)]),
            rest(2),
            set(vec![exercise(3, 1)]),
            rest(3),
            set(vec![exercise(4, 0)]),
            rest(4),
            set(vec![exercise(5, 0)]),
            rest(5),
        ];
        replace_exercise(&mut sections, 8, 0, 2, &exercises(2));
        assert_eq!(
            sections,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(3, 1)]),
                rest(3),
                set(vec![exercise(4, 2)]),
                rest(4),
                set(vec![exercise(5, 2)]),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_replace_exercise_last_set() {
        let mut sections = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
            set(vec![exercise(2, 1)]),
            rest(2),
            set(vec![exercise(3, 1)]),
            rest(3),
            set(vec![exercise(4, 0)]),
            rest(4),
            set(vec![exercise(5, 0)]),
            rest(5),
        ];
        replace_exercise(&mut sections, 10, 0, 2, &exercises(2));
        assert_eq!(
            sections,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(3, 1)]),
                rest(3),
                set(vec![exercise(4, 0)]),
                rest(4),
                set(vec![exercise(5, 2)]),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_replace_exercise_superset_first_exercise() {
        let mut sections = vec![
            set(vec![exercise(0, 0), exercise(1, 1)]),
            rest(0),
            set(vec![exercise(2, 0), exercise(3, 1)]),
            rest(1),
            set(vec![exercise(4, 0), exercise(5, 2)]),
            rest(2),
            set(vec![exercise(6, 0), exercise(7, 2)]),
            rest(3),
            set(vec![exercise(8, 1), exercise(9, 2)]),
            rest(4),
            set(vec![exercise(10, 1), exercise(11, 2)]),
            rest(5),
        ];
        replace_exercise(&mut sections, 0, 0, 3, &exercises(3));
        assert_eq!(
            sections,
            vec![
                set(vec![exercise(0, 3), exercise(1, 1)]),
                rest(0),
                set(vec![exercise(2, 3), exercise(3, 1)]),
                rest(1),
                set(vec![exercise(4, 0), exercise(5, 2)]),
                rest(2),
                set(vec![exercise(6, 0), exercise(7, 2)]),
                rest(3),
                set(vec![exercise(8, 1), exercise(9, 2)]),
                rest(4),
                set(vec![exercise(10, 1), exercise(11, 2)]),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_replace_exercise_dropsets() {
        let mut sections = vec![
            set(vec![exercise(0, 0), exercise(1, 0)]),
            rest(0),
            set(vec![exercise(2, 0), exercise(3, 0)]),
            rest(1),
            set(vec![exercise(4, 0), exercise(5, 2)]),
            rest(2),
            set(vec![exercise(6, 0), exercise(7, 2)]),
            rest(3),
        ];
        replace_exercise(&mut sections, 0, 0, 3, &exercises(3));
        assert_eq!(
            sections,
            vec![
                set(vec![exercise(0, 3), exercise(1, 3)]),
                rest(0),
                set(vec![exercise(2, 3), exercise(3, 3)]),
                rest(1),
                set(vec![exercise(4, 0), exercise(5, 2)]),
                rest(2),
                set(vec![exercise(6, 0), exercise(7, 2)]),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_replace_exercise_superset_second_exercise() {
        let mut sections = vec![
            set(vec![exercise(0, 0), exercise(1, 1)]),
            rest(0),
            set(vec![exercise(2, 0), exercise(3, 1)]),
            rest(1),
            set(vec![exercise(4, 0), exercise(5, 2)]),
            rest(2),
            set(vec![exercise(6, 0), exercise(7, 2)]),
            rest(3),
            set(vec![exercise(8, 1), exercise(9, 2)]),
            rest(4),
            set(vec![exercise(10, 1), exercise(11, 2)]),
            rest(5),
        ];
        replace_exercise(&mut sections, 4, 1, 3, &exercises(3));
        assert_eq!(
            sections,
            vec![
                set(vec![exercise(0, 0), exercise(1, 1)]),
                rest(0),
                set(vec![exercise(2, 0), exercise(3, 1)]),
                rest(1),
                set(vec![exercise(4, 0), exercise(5, 3)]),
                rest(2),
                set(vec![exercise(6, 0), exercise(7, 3)]),
                rest(3),
                set(vec![exercise(8, 1), exercise(9, 2)]),
                rest(4),
                set(vec![exercise(10, 1), exercise(11, 2)]),
                rest(5),
            ]
        );
    }

    fn exercises(id: u32) -> BTreeMap<u32, data::Exercise> {
        BTreeMap::from([(
            id,
            data::Exercise {
                id,
                name: id.to_string(),
            },
        )])
    }

    fn exercise(entry_id: u32, exercise_id: u32) -> ExerciseForm {
        ExerciseForm {
            position: 0,
            exercise_id,
            exercise_name: exercise_id.to_string(),
            reps: InputField::default(),
            time: InputField::default(),
            weight: InputField::default(),
            rpe: InputField::default(),
            target_reps: Some(entry_id),
            target_time: None,
            target_weight: None,
            target_rpe: None,
            prev_reps: None,
            prev_time: None,
            prev_weight: None,
            prev_rpe: None,
            automatic: false,
        }
    }

    fn set(exercises: Vec<ExerciseForm>) -> FormSection {
        FormSection::Set { exercises }
    }

    fn rest(entry_id: u32) -> FormSection {
        FormSection::Rest {
            target_time: entry_id,
            automatic: true,
        }
    }
}
