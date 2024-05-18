use std::cmp::Ordering;
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
    }
}

fn init_form(training_session: Option<&data::TrainingSession>, data_model: &data::Model) -> Form {
    let previous_sets = previous_sets(training_session, data_model);
    if let Some(training_session) = training_session {
        let mut elements = vec![];
        let mut exercises = vec![];
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
                            elements.push(FormElement::Set { exercises });
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
                    if target_time.is_some() && target_reps.is_none() {
                        if not(exercises.is_empty()) {
                            elements.push(FormElement::Set { exercises });
                        }
                        exercises = vec![];
                    }
                }
                data::TrainingSessionElement::Rest {
                    target_time,
                    automatic,
                } => {
                    if not(exercises.is_empty()) {
                        elements.push(FormElement::Set { exercises });
                    }
                    exercises = vec![];
                    elements.push(FormElement::Rest {
                        target_time: target_time.unwrap_or(0),
                        automatic: *automatic,
                    });
                }
            }
        }

        if not(exercises.is_empty()) {
            elements.push(FormElement::Set { exercises });
        }

        Form {
            notes: training_session.notes.clone().unwrap_or_default(),
            notes_changed: false,
            elements,
        }
    } else {
        Form {
            notes: String::new(),
            notes_changed: false,
            elements: vec![],
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
}

impl Model {
    pub fn has_unsaved_changes(&self) -> bool {
        self.form.changed()
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        close_notifications();
    }
}

struct Form {
    notes: String,
    notes_changed: bool,
    elements: Vec<FormElement>,
}

impl Form {
    fn changed(&self) -> bool {
        self.notes_changed
            || self
                .elements
                .iter()
                .filter_map(|e| match e {
                    FormElement::Set { exercises } => Some(exercises),
                    _ => None,
                })
                .flatten()
                .any(|e| {
                    e.reps.changed() || e.time.changed() || e.weight.changed() || e.rpe.changed()
                })
    }

    fn valid(&self) -> bool {
        self.elements
            .iter()
            .filter_map(|e| match e {
                FormElement::Set { exercises } => Some(exercises),
                _ => None,
            })
            .flatten()
            .all(|s| s.reps.valid() && s.time.valid() && s.weight.valid() && s.rpe.valid())
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
enum FormElement {
    Set { exercises: Vec<ExerciseForm> },
    Rest { target_time: u32, automatic: bool },
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
struct ExerciseForm {
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
    element_idx: usize,
    element_start_time: DateTime<Utc>,
    timer: Timer,
    stream: Option<StreamHandle>,
    element: ElRef<web_sys::Element>,
}

impl Guide {
    fn new(beep_volume: u8) -> Guide {
        Guide {
            element_idx: 0,
            element_start_time: Utc::now(),
            timer: Timer::new(beep_volume),
            stream: None,
            element: ElRef::new(),
        }
    }

    fn from_ongoing_training_session(
        element_idx: usize,
        element_start_time: DateTime<Utc>,
        beep_volume: u8,
    ) -> Guide {
        Guide {
            element_idx,
            element_start_time,
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
    AddExercise(usize, usize, String),
    AppendExercise(String),
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
            reset_time: i64::MAX,
            target_time: None,
            beep_time: 0.,
            beep_volume,
        }
    }

    fn is_set(&self) -> bool {
        self.reset_time != i64::MAX
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
        self.time = (String::new(), None);
        self.reset_time = i64::MAX;
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
            if let Some(audio_context) = audio_context {
                if time == 10 && Some(time) != self.time.1 {
                    if let Err(err) = play_beep(
                        audio_context,
                        2000.,
                        {
                            self.beep_time = audio_context.current_time() + 0.01;
                            self.beep_time
                        },
                        0.1,
                        self.beep_volume,
                    ) {
                        error!("failed to play beep:", err);
                    }
                    if let Err(err) = play_beep(
                        audio_context,
                        2000.,
                        {
                            self.beep_time = audio_context.current_time() + 0.18;
                            self.beep_time
                        },
                        0.1,
                        self.beep_volume,
                    ) {
                        error!("failed to play beep:", err);
                    }
                }
                if (0..=2).contains(&time) && Some(time) != self.time.1 {
                    if let Err(err) = play_beep(
                        audio_context,
                        2000.,
                        if time == 2 {
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
    ShowAddExerciseDialog(usize, usize),
    ShowAppendExerciseDialog,
    SearchTermChanged(String),
    CreateExercise,
    ReplaceExercise(usize, usize, u32),
    DeferExercise(usize),
    AddSet(usize),
    AddExercise(usize, usize, u32),
    RemoveSet(usize),
    RemoveExercise(usize, usize),
    AppendExercise(u32),
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
        Msg::RepsChanged(element_idx, exercise_idx, input) => {
            if let FormElement::Set { exercises } = &mut model.form.elements[element_idx] {
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
        Msg::TimeChanged(element_idx, exercise_idx, input) => {
            if let FormElement::Set { exercises } = &mut model.form.elements[element_idx] {
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
        Msg::WeightChanged(element_idx, exercise_idx, input) => {
            if let FormElement::Set { exercises } = &mut model.form.elements[element_idx] {
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
        Msg::RPEChanged(element_idx, exercise_idx, input) => {
            if let FormElement::Set { exercises } = &mut model.form.elements[element_idx] {
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

        Msg::EnterTargetValues(element_idx, exercise_idx) => {
            if let FormElement::Set { exercises } = &mut model.form.elements[element_idx] {
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
        Msg::EnterPreviousValues(element_idx, exercise_idx) => {
            if let FormElement::Set { exercises } = &mut model.form.elements[element_idx] {
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
            update_guide(model);
            store_guide_state(model, orders);
            update_streams(model, orders);
            orders.notify(data::Msg::StartTrainingSession(model.training_session_id));
            update_metronome(model, orders, data_model.settings.automatic_metronome);
            show_element_notification(model, data_model.settings.notifications);
            Url::go_and_push(
                &crate::Urls::new(&data_model.base_url)
                    .training_session()
                    .add_hash_path_part(model.training_session_id.to_string())
                    .add_hash_path_part("guide"),
            );
        }
        Msg::ContinueGuidedTrainingSession(ongoing_training_session) => {
            model.guide = Some(Guide::from_ongoing_training_session(
                ongoing_training_session.element_idx,
                ongoing_training_session.element_start_time,
                data_model.settings.beep_volume,
            ));
            model
                .guide
                .as_mut()
                .unwrap()
                .timer
                .restore(ongoing_training_session.timer_state);
            update_metronome(model, orders, data_model.settings.automatic_metronome);
            update_streams(model, orders);
            show_element_notification(model, data_model.settings.notifications);
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
                match &model.form.elements.get(guide.element_idx) {
                    Some(FormElement::Set { exercises }) => {
                        let exercise = &exercises[0];
                        if not(show_guide_timer(exercise)) {
                            guide.timer.reset();
                        } else if let Some(target_time) = exercise.target_time {
                            if let Some(time) = guide.timer.time.1 {
                                if time <= 0 {
                                    if let Some(target_reps) = exercise.target_reps {
                                        orders.send_msg(Msg::RepsChanged(
                                            guide.element_idx,
                                            0,
                                            target_reps.to_string(),
                                        ));
                                    }
                                    orders.send_msg(Msg::TimeChanged(
                                        guide.element_idx,
                                        0,
                                        target_time.to_string(),
                                    ));
                                    orders.send_msg(Msg::GoToNextSection);
                                }
                            }
                        }
                    }
                    Some(FormElement::Rest { automatic, .. }) => {
                        if let Some(time) = guide.timer.time.1 {
                            if time <= 0 && *automatic {
                                orders.send_msg(Msg::GoToNextSection);
                            }
                        } else if *automatic {
                            orders.send_msg(Msg::GoToNextSection);
                        }
                    }
                    None => {}
                }
                guide.timer.update(&model.audio_context);
            }
        }
        Msg::StartPauseGuideTimer => {
            if let Some(guide) = &mut model.guide {
                guide.timer.start_pause();
                orders.notify(data::Msg::UpdateTrainingSession(
                    guide.element_idx,
                    guide.timer.to_timer_state(),
                ));
            }
            update_streams(model, orders);
        }
        Msg::GoToPreviousSection => {
            if let Some(guide) = &mut model.guide {
                let mut element_idx = guide.element_idx - 1;
                while if let Some(FormElement::Rest {
                    target_time,
                    automatic,
                }) = model.form.elements.get(element_idx)
                {
                    element_idx > 0 && *target_time == 0 && *automatic
                } else {
                    false
                } {
                    element_idx -= 1;
                }
                guide.element_idx = element_idx;
                guide.element_start_time = Utc::now();
            }
            update_guide(model);
            store_guide_state(model, orders);
            update_metronome(model, orders, data_model.settings.automatic_metronome);
            update_streams(model, orders);
            close_notifications();
            orders.force_render_now().send_msg(Msg::ScrollToSection);
        }
        Msg::GoToNextSection => {
            if let Some(guide) = &mut model.guide {
                let element_idx = guide.element_idx + 1;
                if element_idx == model.form.elements.len() {
                    model.guide = None;
                    close_notifications();
                    orders
                        .send_msg(Msg::PauseMetronome)
                        .notify(data::Msg::EndTrainingSession);
                } else {
                    guide.element_idx = element_idx;
                    guide.element_start_time = Utc::now();
                    update_metronome(model, orders, data_model.settings.automatic_metronome);
                    show_element_notification(model, data_model.settings.notifications);
                }
            }
            update_guide(model);
            store_guide_state(model, orders);
            update_streams(model, orders);
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
                        .elements
                        .iter()
                        .flat_map(|e| match e {
                            FormElement::Set { exercises } => exercises
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
                            FormElement::Rest {
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
                    update_guide(model);
                    update_streams(model, orders);
                }
                data::Event::TrainingSessionModifiedErr => {
                    model.loading = false;
                    update_guide(model);
                    update_streams(model, orders);
                }
                data::Event::ExerciseCreatedOk | data::Event::ExerciseCreatedErr => {
                    model.loading = false;
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
        Msg::ShowOptionsDialog(element_idx, exercise_idx) => {
            model.dialog = Dialog::Options(element_idx, exercise_idx);
        }
        Msg::ShowReplaceExerciseDialog(element_idx, exercise_idx) => {
            model.dialog = Dialog::ReplaceExercise(element_idx, exercise_idx, String::new());
        }
        Msg::ShowAddExerciseDialog(element_idx, exercise_idx) => {
            model.dialog = Dialog::AddExercise(element_idx, exercise_idx, String::new());
        }
        Msg::ShowAppendExerciseDialog => {
            model.dialog = Dialog::AppendExercise(String::new());
        }
        Msg::SearchTermChanged(search_term) => {
            if let Dialog::ReplaceExercise(_, _, st)
            | Dialog::AddExercise(_, _, st)
            | Dialog::AppendExercise(st) = &mut model.dialog
            {
                *st = search_term;
            }
        }
        Msg::CreateExercise => {
            model.loading = true;
            if let Dialog::ReplaceExercise(_, _, search_term)
            | Dialog::AddExercise(_, _, search_term)
            | Dialog::AppendExercise(search_term) = &model.dialog
            {
                orders.notify(data::Msg::CreateExercise(search_term.trim().to_string()));
            };
        }
        Msg::ReplaceExercise(element_idx, exercise_idx, new_exercise_id) => {
            replace_exercise(
                &mut model.form.elements,
                element_idx,
                exercise_idx,
                new_exercise_id,
                &data_model.exercises,
            );
            orders
                .send_msg(Msg::SaveTrainingSession)
                .send_msg(Msg::CloseDialog);
        }
        Msg::DeferExercise(element_idx) => {
            defer_exercise(&mut model.form.elements, element_idx);
            update_metronome(model, orders, data_model.settings.automatic_metronome);
            orders
                .send_msg(Msg::SaveTrainingSession)
                .send_msg(Msg::CloseDialog);
        }
        Msg::AddSet(element_idx) => {
            add_set(&mut model.form.elements, element_idx);
            orders
                .send_msg(Msg::SaveTrainingSession)
                .send_msg(Msg::CloseDialog);
        }
        Msg::AddExercise(element_idx, exercise_idx, new_exercise_id) => {
            add_exercise(
                &mut model.form.elements,
                element_idx,
                exercise_idx,
                new_exercise_id,
                &data_model.exercises,
            );
            orders
                .send_msg(Msg::SaveTrainingSession)
                .send_msg(Msg::CloseDialog);
        }
        Msg::RemoveSet(element_idx) => {
            remove_set(&mut model.form.elements, element_idx);
            orders
                .send_msg(Msg::SaveTrainingSession)
                .send_msg(Msg::CloseDialog);
        }
        Msg::RemoveExercise(element_idx, exercise_idx) => {
            remove_exercise(&mut model.form.elements, element_idx, exercise_idx);
            orders
                .send_msg(Msg::SaveTrainingSession)
                .send_msg(Msg::CloseDialog);
        }
        Msg::AppendExercise(exercise_id) => {
            append_exercise(&mut model.form.elements, exercise_id, &data_model.exercises);
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

fn update_guide(model: &mut Model) {
    if model.form.elements.is_empty() {
        return;
    }

    if let Some(guide) = &mut model.guide {
        guide.timer.unset();
        let elapsed_time = (Utc::now() - guide.element_start_time).num_seconds();
        match &model.form.elements.get(guide.element_idx) {
            Some(FormElement::Set { exercises }) => {
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
            Some(FormElement::Rest { target_time, .. }) => {
                if *target_time > 0 {
                    guide.timer.set(i64::from(*target_time) - elapsed_time);
                    guide.timer.start();
                }
            }
            None => {
                model.guide = None;
            }
        }
    }
}

fn store_guide_state(model: &mut Model, orders: &mut impl Orders<Msg>) {
    if let Some(guide) = &mut model.guide {
        orders.notify(data::Msg::UpdateTrainingSession(
            guide.element_idx,
            guide.timer.to_timer_state(),
        ));
    }
}

fn update_metronome(model: &Model, orders: &mut impl Orders<Msg>, automatic_metronome: bool) {
    if model.form.elements.is_empty() || not(automatic_metronome) {
        return;
    }

    if let Some(guide) = &model.guide {
        if guide.element_idx >= model.form.elements.len() {
            return;
        }
        match &model.form.elements[guide.element_idx] {
            FormElement::Set { exercises } => {
                let exercise = &exercises[0];
                if exercise.target_reps.is_some() {
                    if let Some(target_time) = exercise.target_time {
                        orders.send_msg(Msg::StartMetronome(target_time));
                    }
                }
            }
            FormElement::Rest { .. } => {
                orders.send_msg(Msg::PauseMetronome);
            }
        }
    }
}

fn show_notification(title: &str, body: Option<String>) {
    close_notifications();
    let mut options = HashMap::new();
    if let Some(body) = body {
        options.insert(String::from("body"), body);
    }
    if let Err(err) =
        common::post_message_to_service_worker(&common::ServiceWorkerMessage::ShowNotification {
            title: title.to_string(),
            options,
        })
    {
        error!("failed to show notification:", err);
    }
}

fn close_notifications() {
    if let Err(err) =
        common::post_message_to_service_worker(&common::ServiceWorkerMessage::CloseNotifications)
    {
        error!("failed to close notification:", err);
    }
}

fn show_element_notification(model: &mut Model, notifications_enabled: bool) {
    if not(notifications_enabled) {
        close_notifications();
        return;
    }

    if let Some(guide) = &mut model.guide {
        if guide.element_idx < model.form.elements.len() {
            let title;
            let body;
            match &model.form.elements[guide.element_idx] {
                FormElement::Set { exercises } => {
                    let exercise = &exercises[0];
                    title = exercise.exercise_name.clone();
                    let mut previously = common::format_set(
                        exercise.prev_reps,
                        exercise.prev_time,
                        exercise.prev_weight,
                        exercise.prev_rpe,
                    );
                    if not(previously.is_empty()) {
                        previously = format!("Previously:\n{previously}\n");
                    }
                    let mut target = common::format_set(
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
                FormElement::Rest { target_time, .. } => {
                    title = String::from("Rest");
                    body = if *target_time > 0 {
                        Some(format!("{target_time} s"))
                    } else {
                        None
                    };
                }
            }

            show_notification(&title, body);
        }
    }
}

fn replace_exercise(
    elements: &mut [FormElement],
    element_idx: usize,
    exercise_idx: usize,
    new_exercise_id: u32,
    data_exercises: &BTreeMap<u32, data::Exercise>,
) {
    let mut current_exercise_id = None;
    let mut current_exercise_ids = vec![];
    for mut element in elements.iter_mut().skip(element_idx) {
        if let FormElement::Set { exercises } = &mut element {
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

fn defer_exercise(elements: &mut Vec<FormElement>, element_idx: usize) {
    let mut deferred_ids = vec![];
    let mut deferred_elements = 0;
    let mut preferred_ids = vec![];
    let mut preferred_elements = 0;
    for (i, element) in elements.iter().enumerate().skip(element_idx) {
        if let FormElement::Set { exercises } = &element {
            let exercise_ids = exercises.iter().map(|e| e.exercise_id).collect::<Vec<_>>();
            if deferred_ids.is_empty() {
                deferred_ids = exercise_ids;
            } else if deferred_elements == 0 && deferred_ids != exercise_ids {
                deferred_elements = i - element_idx;
                preferred_ids = exercise_ids;
            } else if deferred_elements > 0 && preferred_ids != exercise_ids {
                preferred_elements = i - element_idx - deferred_elements;
                break;
            }
        }
        if i == elements.len() - 1 && deferred_elements > 0 {
            preferred_elements = elements.len() - element_idx - deferred_elements;
        }
    }
    if element_idx + deferred_elements + preferred_elements == elements.len() {
        if let Some(FormElement::Set { .. }) = elements.last() {
            elements.push(FormElement::Rest {
                target_time: 0,
                automatic: true,
            });
            preferred_elements += 1;
        }
    }
    elements[element_idx..element_idx + deferred_elements + preferred_elements]
        .rotate_right(preferred_elements);
}

fn add_set(elements: &mut Vec<FormElement>, element_idx: usize) {
    if not(is_set(elements, element_idx)) {
        return;
    }

    let rest_idx = determine_rest_between_sets(elements, element_idx);
    let rest = if let FormElement::Rest {
        target_time,
        automatic,
    } = &elements[rest_idx]
    {
        FormElement::Rest {
            target_time: *target_time,
            automatic: *automatic,
        }
    } else {
        FormElement::Rest {
            target_time: 0,
            automatic: true,
        }
    };

    elements.insert(element_idx + 1, rest);

    if let FormElement::Set { exercises } = &elements[element_idx] {
        elements.insert(
            element_idx + 2,
            FormElement::Set {
                exercises: exercises
                    .iter()
                    .map(|e| ExerciseForm {
                        exercise_id: e.exercise_id,
                        exercise_name: e.exercise_name.to_string(),
                        reps: common::InputField::default(),
                        time: common::InputField::default(),
                        weight: common::InputField::default(),
                        rpe: common::InputField::default(),
                        target_reps: e.target_reps,
                        target_time: e.target_time,
                        target_weight: e.target_weight,
                        target_rpe: e.target_rpe,
                        prev_reps: None,
                        prev_time: None,
                        prev_weight: None,
                        prev_rpe: None,
                        automatic: e.automatic,
                    })
                    .collect::<Vec<_>>(),
            },
        );
    }
}

fn add_exercise(
    elements: &mut [FormElement],
    element_idx: usize,
    exercise_idx: usize,
    new_exercise_id: u32,
    data_exercises: &BTreeMap<u32, data::Exercise>,
) {
    let mut current_exercise_ids = vec![];
    for mut element in elements.iter_mut().skip(element_idx) {
        if let FormElement::Set { exercises } = &mut element {
            if exercise_idx >= exercises.len() {
                return;
            }
            let ids = exercises.iter().map(|e| e.exercise_id).collect::<Vec<_>>();
            if current_exercise_ids.is_empty() {
                current_exercise_ids = ids;
            } else if current_exercise_ids != ids {
                break;
            }
            exercises.insert(
                exercise_idx + 1,
                ExerciseForm {
                    exercise_id: new_exercise_id,
                    exercise_name: data_exercises[&new_exercise_id].name.clone(),
                    reps: common::InputField::default(),
                    time: common::InputField::default(),
                    weight: common::InputField::default(),
                    rpe: common::InputField::default(),
                    target_reps: None,
                    target_time: None,
                    target_weight: None,
                    target_rpe: None,
                    prev_reps: None,
                    prev_time: None,
                    prev_weight: None,
                    prev_rpe: None,
                    automatic: false,
                },
            );
        }
    }
}

fn remove_set(elements: &mut Vec<FormElement>, element_idx: usize) {
    if not(is_set(elements, element_idx)) {
        return;
    }

    let rest_idx = determine_rest_between_sets(elements, element_idx);

    elements.remove(element_idx);

    match rest_idx.cmp(&element_idx) {
        Ordering::Less => {
            elements.remove(rest_idx);
        }
        Ordering::Greater => {
            elements.remove(rest_idx - 1);
        }
        Ordering::Equal => {}
    }
}

fn remove_exercise(elements: &mut Vec<FormElement>, element_idx: usize, exercise_idx: usize) {
    let ids = determine_exercise_ids(elements, element_idx);
    if ids.is_empty() || exercise_idx >= ids.len() {
    } else if ids.len() > 1 {
        for mut element in elements.iter_mut().skip(element_idx) {
            if let FormElement::Set { exercises } = &mut element {
                if ids != exercises.iter().map(|e| e.exercise_id).collect::<Vec<_>>() {
                    break;
                }
                exercises.remove(exercise_idx);
            }
        }
    } else {
        let last = determine_last_element_of_section(elements, element_idx);
        elements.drain(element_idx..=last);
    }
}

fn append_exercise(
    elements: &mut Vec<FormElement>,
    exercise_id: u32,
    data_exercises: &BTreeMap<u32, data::Exercise>,
) {
    if let Some(FormElement::Set { exercises: _ }) = elements.last() {
        elements.push(FormElement::Rest {
            target_time: 0,
            automatic: true,
        });
    }
    elements.push(FormElement::Set {
        exercises: vec![ExerciseForm {
            exercise_id,
            exercise_name: data_exercises[&exercise_id].name.clone(),
            reps: common::InputField::default(),
            time: common::InputField::default(),
            weight: common::InputField::default(),
            rpe: common::InputField::default(),
            target_reps: None,
            target_time: None,
            target_weight: None,
            target_rpe: None,
            prev_reps: None,
            prev_time: None,
            prev_weight: None,
            prev_rpe: None,
            automatic: false,
        }],
    });
}

fn is_set(elements: &mut Vec<FormElement>, element_idx: usize) -> bool {
    if element_idx >= elements.len() {
        return false;
    }

    match &elements[element_idx] {
        FormElement::Set { .. } => true,
        FormElement::Rest { .. } => false,
    }
}

fn determine_exercise_ids(elements: &[FormElement], element_idx: usize) -> Vec<u32> {
    if let Some(FormElement::Set { exercises }) = &elements.get(element_idx) {
        exercises.iter().map(|e| e.exercise_id).collect::<Vec<_>>()
    } else {
        Vec::new()
    }
}

fn determine_first_set_with_same_exercises(elements: &[FormElement], element_idx: usize) -> usize {
    let ids = determine_exercise_ids(elements, element_idx);
    let mut first_idx = element_idx;

    for (i, element) in elements
        .iter()
        .rev()
        .enumerate()
        .skip(elements.len() - element_idx + 1)
    {
        if let FormElement::Set { exercises } = &element {
            let exercise_ids = exercises.iter().map(|e| e.exercise_id).collect::<Vec<_>>();
            if ids == exercise_ids {
                first_idx = element_idx - (elements.len() - i + 1);
            } else {
                break;
            }
        }
    }

    first_idx
}

fn determine_last_set_with_same_exercises(elements: &[FormElement], element_idx: usize) -> usize {
    let ids = determine_exercise_ids(elements, element_idx);
    let mut last_idx = element_idx;

    for (i, element) in elements.iter().enumerate().skip(element_idx + 1) {
        if let FormElement::Set { exercises } = &element {
            let exercise_ids = exercises.iter().map(|e| e.exercise_id).collect::<Vec<_>>();
            if ids == exercise_ids {
                last_idx = i;
            } else {
                break;
            }
        }
    }

    last_idx
}

fn determine_rest_between_sets(elements: &[FormElement], element_idx: usize) -> usize {
    let first_idx = determine_first_set_with_same_exercises(elements, element_idx);
    let last_idx = determine_last_set_with_same_exercises(elements, element_idx);

    assert!(first_idx <= last_idx);
    assert!(last_idx < elements.len());

    let rest_idx = if element_idx < last_idx {
        element_idx + 1
    } else if first_idx < element_idx {
        element_idx - 1
    } else if element_idx + 1 < elements.len() {
        element_idx + 1
    } else if element_idx > 0 && element_idx + 1 == elements.len() {
        element_idx - 1
    } else {
        element_idx
    };

    if let FormElement::Rest { .. } = &elements[rest_idx] {
        rest_idx
    } else {
        element_idx
    }
}

fn determine_sections(elements: &[FormElement]) -> Vec<(usize, usize)> {
    let mut sections = vec![];
    let mut idx = 0;

    while idx < elements.len() {
        let last = determine_last_element_of_section(elements, idx);
        sections.push((idx, last));
        idx = last + 1;
    }

    sections
}

fn determine_last_element_of_section(elements: &[FormElement], element_idx: usize) -> usize {
    let mut last = determine_last_set_with_same_exercises(elements, element_idx);
    assert!(element_idx <= last);
    if last + 1 < elements.len() {
        if let FormElement::Rest { .. } = &elements[last + 1] {
            last += 1;
        }
    }
    last
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
                        view_list(model, data_model),
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
    div![
        common::view_title(&span![training_session.date.to_string()], 3),
        if let Some(routine) = data_model
            .routines
            .get(&training_session.routine_id.unwrap_or(0))
        {
            common::view_title(
                &a![
                    attrs! {
                        At::Href => crate::Urls::new(&data_model.base_url).routine().add_hash_path_part(routine.id.to_string()),
                    },
                    &routine.name
                ],
                3,
            )
        } else {
            empty![]
        }
    ]
}

fn view_list(model: &Model, data_model: &data::Model) -> Vec<Node<Msg>> {
    let sections = determine_sections(&model.form.elements);
    sections
        .iter()
        .flat_map(|(first, last)| {
            nodes![
                div![
                    C!["block"],
                    C!["mb-2"],
                    if let FormElement::Set { exercises } = &model.form.elements[*first] {
                        let mut last = exercises.len() - 1;
                        if last > 0 && exercises.iter().all(|e| e.exercise_id == exercises[0].exercise_id) {
                            last = 0;
                        }
                        exercises[0..=last]
                            .iter()
                            .map(|e| {
                                div![
                                    C!["has-text-centered"],
                                    C!["has-text-weight-bold"],
                                    a![
                                        attrs! {
                                            At::Href => crate::Urls::new(&data_model.base_url).exercise().add_hash_path_part(e.exercise_id.to_string()),
                                        },
                                        span![style! {St::WhiteSpace => "nowrap" }, &e.exercise_name]
                                    ]
                                ]
                            })
                            .collect::<Vec<_>>()
                    } else {
                        vec![]
                    }
                ],
                div![
                    C!["block"],
                    model.form.elements[*first..=*last].iter().map(|element| {
                        nodes![
                            if let FormElement::Set { exercises } = element {
                                exercises
                                    .iter()
                                    .map(|e| {
                                        div![
                                            C!["has-text-centered"],
                                            span![
                                                style! {St::WhiteSpace => "nowrap" },
                                                common::format_set(
                                                    e.reps.parsed,
                                                    e.time.parsed,
                                                    e.weight.parsed,
                                                    e.rpe.parsed
                                                )
                                            ]
                                        ]
                                    })
                                    .collect::<Vec<_>>()
                            } else {
                                vec![]
                            },
                            div![C!["my-2"]]
                        ]
                    }),
                ]
            ]
        })
        .collect::<Vec<_>>()
}

fn view_notes(training_session: &data::TrainingSession) -> Node<Msg> {
    if let Some(notes) = &training_session.notes {
        if notes.is_empty() {
            empty![]
        } else {
            div![
                C!["has-text-centered"],
                C!["m-3"],
                C!["mt-6"],
                common::view_title(&span!["Notes"], 3),
                p![notes]
            ]
        }
    } else {
        empty![]
    }
}

fn view_training_session_form(model: &Model, data_model: &data::Model) -> Node<Msg> {
    let sections = determine_sections(&model.form.elements);
    let valid = model.form.valid();
    let save_disabled = not(model.form.changed()) || not(valid);

    let form = sections.iter().map(|(first, last)| {
        let mut section_form: std::vec::Vec<seed::virtual_dom::Node<Msg>> = nodes![];
        for (idx, element) in model.form.elements[*first..=*last].iter().enumerate() {
            let element_idx = first + idx;
            if let Some(guide) = &model.guide {
                if guide.element_idx == element_idx && element_idx != 0 {
                    section_form.push(div![
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

            match element {
                FormElement::Set {
                    exercises: exercise_forms,
                } => {
                    section_form.push(
                        div![
                            if let Some(guide) = &model.guide {
                                if guide.element_idx == element_idx {
                                    el_ref(&guide.element)
                                } else {
                                    el_ref(&ElRef::new())
                                }
                            } else {
                                el_ref(&ElRef::new())
                            },
                            C!["message"],
                            C!["is-info"],
                            C!["has-background-white"],
                            IF![model.guide.as_ref().map_or(false, |guide| guide.element_idx != element_idx) => C!["is-semitransparent"]],
                            IF![idx > 0 => C!["mt-3"]],
                            C!["mb-0"],
                            div![
                                C!["message-body"],
                                C!["p-3"],
                                exercise_forms.iter().enumerate().map(|(position, s)| {
                                    let input_fields = div![
                                            C!["field"],
                                            C!["has-addons"],
                                            div![
                                                C!["control"],
                                                C!["has-icons-right"],
                                                C!["has-text-right"],
                                                input_ev(Ev::Input, move |v| Msg::RepsChanged(element_idx, position, v)),
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
                                                input_ev(Ev::Input, move |v| Msg::TimeChanged(element_idx, position, v)),
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
                                                input_ev(Ev::Input, move |v| Msg::WeightChanged(element_idx, position, v)),
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
                                                input_ev(Ev::Input, move |v| Msg::RPEChanged(element_idx, position, v)),
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
                                                    ev(Ev::Click, move |_| Msg::ShowOptionsDialog(element_idx, position)),
                                                    span![C!["icon"], i![C!["fas fa-ellipsis-vertical"]]]
                                                ]],
                                            ],
                                        ],
                                        if let Some(guide) = &model.guide {
                                            if guide.timer.is_set() && guide.element_idx == element_idx {
                                                view_guide_timer(guide)
                                            } else {
                                                input_fields
                                            }
                                        } else {
                                            input_fields
                                        },
                                        {
                                            let target = common::format_set(s.target_reps, s.target_time, s.target_weight, s.target_rpe);
                                            let previous = common::format_set(s.prev_reps, s.prev_time, s.prev_weight, s.prev_rpe);
                                            p![
                                                IF![not(target.is_empty()) =>
                                                    span![
                                                        C!["icon-text"],
                                                        C!["mr-4"],
                                                        span![C!["icon"], i![C!["fas fa-bullseye"]]],
                                                        a![
                                                            ev(Ev::Click, move |_| Msg::EnterTargetValues(element_idx, position)),
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
                                                            ev(Ev::Click, move |_| Msg::EnterPreviousValues(element_idx, position)),
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
                FormElement::Rest {
                    target_time,
                    automatic,
                } => {
                    section_form.push(div![
                        if let Some(guide) = &model.guide {
                            if guide.element_idx == element_idx {
                                el_ref(&guide.element)
                            } else {
                                el_ref(&ElRef::new())
                            }
                        } else {
                            el_ref(&ElRef::new())
                        },
                        C!["message"],
                        C!["is-success"],
                        C!["has-background-white"],
                        IF![model.guide.as_ref().map_or(false, |guide| guide.element_idx != element_idx) => C!["is-semitransparent"]],
                        IF![idx > 0 => C!["mt-3"]],
                        C!["mb-0"],
                        div![
                            C!["message-body"],
                            C!["p-3"],
                            if let Some(guide) = &model.guide {
                                if guide.timer.is_set() && guide.element_idx == element_idx {
                                    view_guide_timer(guide)
                                } else {
                                    common::view_rest(*target_time, *automatic)
                                }
                            } else {
                                common::view_rest(*target_time, *automatic)
                            },
                        ]
                    ]);
                }
            }

            if let Some(guide) = &model.guide {
                if guide.element_idx == element_idx {
                    section_form.push(div![
                        C!["has-text-centered"],
                        C!["m-5"],
                        button![
                            C!["button"],
                            C!["is-link"],
                            ev(Ev::Click, |_| Msg::GoToNextSection),
                            if element_idx < model.form.elements.len() - 1 {
                                span![C!["icon"], i![C!["fas fa-angles-down"]]]
                            } else {
                                span![C!["icon"], i![C!["fas fa-check"]]]
                            },
                        ]
                    ]);
                }
            }
        };
        div![
            C!["message"],
            C!["has-background-white-bis"],
            C!["p-3"],
            C!["mb-3"],
            section_form
        ]
    }).collect::<Vec<_>>();

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
                C!["has-text-centered"],
                C!["m-5"],
                button![
                    C!["button"],
                    C!["is-light"],
                    ev(Ev::Click, move |_| Msg::ShowAppendExerciseDialog),
                    span![C!["icon"], i![C!["fas fa-plus"]]]
                ]
            ],
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
        ],
    ]
}

fn view_guide_timer(guide: &Guide) -> Node<Msg> {
    div![
        C!["is-size-1"],
        C![IF![not(guide.timer.is_active()) => "is-blinking"]],
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
        Dialog::Options(element_idx, exercise_idx) => {
            view_options_dialog(*element_idx, *exercise_idx)
        }
        Dialog::ReplaceExercise(element_idx, exercise_idx, search_term) => {
            view_replace_exercise_dialog(
                *element_idx,
                *exercise_idx,
                search_term,
                loading,
                exercises,
            )
        }
        Dialog::AddExercise(element_idx, exercise_idx, search_term) => {
            view_add_exercise_dialog(*element_idx, *exercise_idx, search_term, loading, exercises)
        }
        Dialog::AppendExercise(search_term) => {
            view_append_exercise_dialog(search_term, loading, exercises)
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

fn view_options_dialog(element_idx: usize, exercise_idx: usize) -> Vec<Node<Msg>> {
    nodes![
        p![a![
            C!["has-text-weight-bold"],
            ev(Ev::Click, move |_| Msg::ShowReplaceExerciseDialog(
                element_idx,
                exercise_idx
            )),
            span![
                C!["icon-text"],
                span![C!["icon"], i![C!["fas fa-arrow-right-arrow-left"]]],
                span!["Replace exercise"],
            ]
        ]],
        IF![exercise_idx == 0 =>
            p![
                C!["mt-3"],
                a![
                    C!["has-text-weight-bold"],
                    ev(Ev::Click, move |_| Msg::DeferExercise(
                        element_idx
                    )),
                    span![
                        C!["icon-text"],
                        span![C!["icon"], i![C!["fas fa-arrow-turn-down"]]],
                        span!["Defer exercise"],
                    ]
                ]
            ]
        ],
        IF![exercise_idx == 0 =>
            p![
                C!["mt-3"],
                a![
                    C!["has-text-weight-bold"],
                    ev(Ev::Click, move |_| Msg::AddSet(
                        element_idx
                    )),
                    span![
                        C!["icon-text"],
                        span![C!["icon"], i![C!["fas fa-plus"]]],
                        span!["Add set"],
                    ]
                ]
            ]
        ],
        IF![exercise_idx == 0 =>
            p![
                C!["mt-3"],
                a![
                    C!["has-text-weight-bold"],
                    ev(Ev::Click, move |_| Msg::ShowAddExerciseDialog(
                        element_idx,
                        exercise_idx
                    )),
                    span![
                        C!["icon-text"],
                        span![C!["icon"], i![C!["fas fa-plus"]]],
                        span!["Add exercise to sets"],
                    ]
                ]
            ]
        ],
        p![C!["mt-5"]],
        IF![exercise_idx == 0 =>
            p![
                C!["mt-3"],
                a![
                    C!["has-text-danger"],
                    C!["has-text-weight-bold"],
                    ev(Ev::Click, move |_| Msg::RemoveSet(
                        element_idx
                    )),
                    span![
                        C!["icon-text"],
                        span![C!["icon"], i![C!["fas fa-times"]]],
                        span!["Remove set"],
                    ]
                ]
            ]
        ],
        p![
            C!["mt-3"],
            a![
                C!["has-text-danger"],
                C!["has-text-weight-bold"],
                ev(Ev::Click, move |_| Msg::RemoveExercise(
                    element_idx,
                    exercise_idx
                )),
                span![
                    C!["icon-text"],
                    span![C!["icon"], i![C!["fas fa-times"]]],
                    span!["Remove exercise"],
                ]
            ]
        ]
    ]
}

fn view_replace_exercise_dialog(
    element_idx: usize,
    exercise_idx: usize,
    search_term: &str,
    loading: bool,
    exercises: &BTreeMap<u32, data::Exercise>,
) -> Vec<Node<Msg>> {
    let element_idx = element_idx;
    let exercise_idx = exercise_idx;
    common::view_exercises_with_search(
        exercises,
        search_term,
        Msg::SearchTermChanged,
        |_| Msg::CreateExercise,
        loading,
        move |exercise_id| Msg::ReplaceExercise(element_idx, exercise_idx, exercise_id),
    )
}

fn view_add_exercise_dialog(
    element_idx: usize,
    exercise_idx: usize,
    search_term: &str,
    loading: bool,
    exercises: &BTreeMap<u32, data::Exercise>,
) -> Vec<Node<Msg>> {
    let element_idx = element_idx;
    let exercise_idx = exercise_idx;
    common::view_exercises_with_search(
        exercises,
        search_term,
        Msg::SearchTermChanged,
        |_| Msg::CreateExercise,
        loading,
        move |exercise_id| Msg::AddExercise(element_idx, exercise_idx, exercise_id),
    )
}

fn view_append_exercise_dialog(
    search_term: &str,
    loading: bool,
    exercises: &BTreeMap<u32, data::Exercise>,
) -> Vec<Node<Msg>> {
    common::view_exercises_with_search(
        exercises,
        search_term,
        Msg::SearchTermChanged,
        |_| Msg::CreateExercise,
        loading,
        Msg::AppendExercise,
    )
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
    use super::*;
    use crate::common::InputField;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_replace_exercise_first_set() {
        let mut elements = vec![
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
        replace_exercise(&mut elements, 0, 0, 2, &exercises(2));
        assert_eq!(
            elements,
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
        let mut elements = vec![
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
        replace_exercise(&mut elements, 2, 0, 2, &exercises(2));
        assert_eq!(
            elements,
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
        let mut elements = vec![
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
        replace_exercise(&mut elements, 8, 0, 2, &exercises(2));
        assert_eq!(
            elements,
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
        let mut elements = vec![
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
        replace_exercise(&mut elements, 10, 0, 2, &exercises(2));
        assert_eq!(
            elements,
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
        let mut elements = vec![
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
        replace_exercise(&mut elements, 0, 0, 3, &exercises(3));
        assert_eq!(
            elements,
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
        let mut elements = vec![
            set(vec![exercise(0, 0), exercise(1, 0)]),
            rest(0),
            set(vec![exercise(2, 0), exercise(3, 0)]),
            rest(1),
            set(vec![exercise(4, 0), exercise(5, 2)]),
            rest(2),
            set(vec![exercise(6, 0), exercise(7, 2)]),
            rest(3),
        ];
        replace_exercise(&mut elements, 0, 0, 3, &exercises(3));
        assert_eq!(
            elements,
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
        let mut elements = vec![
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
        replace_exercise(&mut elements, 4, 1, 3, &exercises(3));
        assert_eq!(
            elements,
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

    #[test]
    fn test_defer_exercise_first_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 1)]),
            rest(1),
            set(vec![exercise(2, 2)]),
            rest(2),
        ];
        defer_exercise(&mut elements, 0);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(1, 1)]),
                rest(1),
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(2, 2)]),
                rest(2),
            ]
        );
    }

    #[test]
    fn test_defer_exercise_penultimate_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 1)]),
            rest(1),
            set(vec![exercise(2, 2)]),
            rest(2),
        ];
        defer_exercise(&mut elements, 2);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(2, 2)]),
                rest(2),
                set(vec![exercise(1, 1)]),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_defer_exercise_penultimate_set_without_trailing_rest() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 1)]),
            rest(1),
            set(vec![exercise(2, 2)]),
        ];
        defer_exercise(&mut elements, 2);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(2, 2)]),
                rest(0),
                set(vec![exercise(1, 1)]),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_defer_exercise_last_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 1)]),
            rest(1),
            set(vec![exercise(2, 2)]),
            rest(2),
        ];
        defer_exercise(&mut elements, 4);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 1)]),
                rest(1),
                set(vec![exercise(2, 2)]),
                rest(2),
            ]
        );
    }

    #[test]
    fn test_defer_exercise_multiple_sets() {
        let mut elements = vec![
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
        ];
        defer_exercise(&mut elements, 0);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(3, 1)]),
                rest(3),
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
                set(vec![exercise(4, 2)]),
                rest(4),
                set(vec![exercise(5, 2)]),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_defer_exercise_supersets() {
        let mut elements = vec![
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
        defer_exercise(&mut elements, 4);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0), exercise(1, 1)]),
                rest(0),
                set(vec![exercise(2, 0), exercise(3, 1)]),
                rest(1),
                set(vec![exercise(8, 1), exercise(9, 2)]),
                rest(4),
                set(vec![exercise(10, 1), exercise(11, 2)]),
                rest(5),
                set(vec![exercise(4, 0), exercise(5, 2)]),
                rest(2),
                set(vec![exercise(6, 0), exercise(7, 2)]),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_add_set_first_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
            set(vec![exercise(2, 1)]),
            rest(2),
            set(vec![exercise(3, 1)]),
            rest(3),
        ];
        add_set(&mut elements, 0);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(3, 1)]),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_add_set_second_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
            set(vec![exercise(2, 1)]),
            rest(2),
            set(vec![exercise(3, 1)]),
            rest(3),
        ];
        add_set(&mut elements, 2);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(3, 1)]),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_add_set_penultimate_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
            set(vec![exercise(2, 1)]),
            rest(2),
            set(vec![exercise(3, 1)]),
            rest(3),
        ];
        add_set(&mut elements, 4);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(3, 1)]),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_add_set_last_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
            set(vec![exercise(2, 1)]),
            rest(2),
            set(vec![exercise(3, 1)]),
            rest(3),
        ];
        add_set(&mut elements, 6);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(3, 1)]),
                rest(2),
                set(vec![exercise(3, 1)]),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_add_set_superset() {
        let mut elements = vec![
            set(vec![exercise(0, 0), exercise(4, 2)]),
            rest(0),
            set(vec![exercise(1, 0), exercise(5, 2)]),
            rest(1),
        ];
        add_set(&mut elements, 0);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0), exercise(4, 2)]),
                rest(0),
                set(vec![exercise(0, 0), exercise(4, 2)]),
                rest(0),
                set(vec![exercise(1, 0), exercise(5, 2)]),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_add_set_no_rest_first_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            set(vec![exercise(1, 0)]),
            set(vec![exercise(2, 1)]),
            set(vec![exercise(3, 1)]),
        ];
        add_set(&mut elements, 0);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(0, 0)]),
                set(vec![exercise(1, 0)]),
                set(vec![exercise(2, 1)]),
                set(vec![exercise(3, 1)]),
            ]
        );
    }

    #[test]
    fn test_add_set_no_rest_second_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            set(vec![exercise(1, 0)]),
            set(vec![exercise(2, 1)]),
            set(vec![exercise(3, 1)]),
        ];
        add_set(&mut elements, 1);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                set(vec![exercise(1, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                set(vec![exercise(2, 1)]),
                set(vec![exercise(3, 1)]),
            ]
        );
    }

    #[test]
    fn test_add_set_no_rest_penultimate_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            set(vec![exercise(1, 0)]),
            set(vec![exercise(2, 1)]),
            set(vec![exercise(3, 1)]),
        ];
        add_set(&mut elements, 2);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                set(vec![exercise(1, 0)]),
                set(vec![exercise(2, 1)]),
                rest(0),
                set(vec![exercise(2, 1)]),
                set(vec![exercise(3, 1)]),
            ]
        );
    }

    #[test]
    fn test_add_set_no_rest_last_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            set(vec![exercise(1, 0)]),
            set(vec![exercise(2, 1)]),
            set(vec![exercise(3, 1)]),
        ];
        add_set(&mut elements, 3);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                set(vec![exercise(1, 0)]),
                set(vec![exercise(2, 1)]),
                set(vec![exercise(3, 1)]),
                rest(0),
                set(vec![exercise(3, 1)]),
            ]
        );
    }

    #[test]
    fn test_add_set_first_single_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 1)]),
        ];
        add_set(&mut elements, 0);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 1)]),
            ]
        );
    }

    #[test]
    fn test_add_set_last_single_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 1)]),
        ];
        add_set(&mut elements, 2);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 1)]),
                rest(0),
                set(vec![exercise(1, 1)]),
            ]
        );
    }

    #[test]
    fn test_add_set_invalid_element_idx_rest() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
        ];
        add_set(&mut elements, 1);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_add_set_invalid_element_idx_out_of_range() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
        ];
        add_set(&mut elements, 4);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_add_exercise_first_set() {
        let mut elements = vec![
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
        add_exercise(&mut elements, 0, 0, 2, &exercises(2));
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0), exercise(0, 2)]),
                rest(0),
                set(vec![exercise(1, 0), exercise(0, 2)]),
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
    fn test_add_exercise_second_set() {
        let mut elements = vec![
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
        add_exercise(&mut elements, 2, 0, 2, &exercises(2));
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0), exercise(0, 2)]),
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
    fn test_add_exercise_penultimate_set() {
        let mut elements = vec![
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
        add_exercise(&mut elements, 8, 0, 2, &exercises(2));
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(3, 1)]),
                rest(3),
                set(vec![exercise(4, 0), exercise(0, 2)]),
                rest(4),
                set(vec![exercise(5, 0), exercise(0, 2)]),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_add_exercise_last_set() {
        let mut elements = vec![
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
        add_exercise(&mut elements, 10, 0, 2, &exercises(2));
        assert_eq!(
            elements,
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
                set(vec![exercise(5, 0), exercise(0, 2)]),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_add_exercise_superset_first_exercise() {
        let mut elements = vec![
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
        add_exercise(&mut elements, 0, 0, 3, &exercises(3));
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0), exercise(0, 3), exercise(1, 1)]),
                rest(0),
                set(vec![exercise(2, 0), exercise(0, 3), exercise(3, 1)]),
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
    fn test_add_exercise_dropsets() {
        let mut elements = vec![
            set(vec![exercise(0, 0), exercise(1, 0)]),
            rest(0),
            set(vec![exercise(2, 0), exercise(3, 0)]),
            rest(1),
            set(vec![exercise(4, 0), exercise(5, 2)]),
            rest(2),
            set(vec![exercise(6, 0), exercise(7, 2)]),
            rest(3),
        ];
        add_exercise(&mut elements, 0, 0, 3, &exercises(3));
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0), exercise(0, 3), exercise(1, 0)]),
                rest(0),
                set(vec![exercise(2, 0), exercise(0, 3), exercise(3, 0)]),
                rest(1),
                set(vec![exercise(4, 0), exercise(5, 2)]),
                rest(2),
                set(vec![exercise(6, 0), exercise(7, 2)]),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_add_exercise_superset_second_exercise() {
        let mut elements = vec![
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
        add_exercise(&mut elements, 4, 1, 3, &exercises(3));
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0), exercise(1, 1)]),
                rest(0),
                set(vec![exercise(2, 0), exercise(3, 1)]),
                rest(1),
                set(vec![exercise(4, 0), exercise(5, 2), exercise(0, 3)]),
                rest(2),
                set(vec![exercise(6, 0), exercise(7, 2), exercise(0, 3)]),
                rest(3),
                set(vec![exercise(8, 1), exercise(9, 2)]),
                rest(4),
                set(vec![exercise(10, 1), exercise(11, 2)]),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_add_exercise_element_idx_rest() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
        ];
        add_exercise(&mut elements, 1, 0, 2, &exercises(2));
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0), exercise(0, 2)]),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_add_exercise_invalid_element_idx_out_of_range() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
        ];
        add_exercise(&mut elements, 4, 0, 2, &exercises(2));
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_add_exercise_invalid_exercise_idx_out_of_range() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
        ];
        add_exercise(&mut elements, 0, 1, 2, &exercises(2));
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_remove_set_first_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
            set(vec![exercise(2, 1)]),
            rest(2),
            set(vec![exercise(3, 1)]),
            rest(3),
        ];
        remove_set(&mut elements, 0);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(1, 0)]),
                rest(1),
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(3, 1)]),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_remove_set_second_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
            set(vec![exercise(2, 1)]),
            rest(2),
            set(vec![exercise(3, 1)]),
            rest(3),
        ];
        remove_set(&mut elements, 2);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(1),
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(3, 1)]),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_remove_set_penultimate_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
            set(vec![exercise(2, 1)]),
            rest(2),
            set(vec![exercise(3, 1)]),
            rest(3),
        ];
        remove_set(&mut elements, 4);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
                set(vec![exercise(3, 1)]),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_remove_set_last_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
            set(vec![exercise(2, 1)]),
            rest(2),
            set(vec![exercise(3, 1)]),
            rest(3),
        ];
        remove_set(&mut elements, 6);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
                set(vec![exercise(2, 1)]),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_remove_set_superset() {
        let mut elements = vec![
            set(vec![exercise(0, 0), exercise(4, 2)]),
            rest(0),
            set(vec![exercise(1, 0), exercise(5, 2)]),
            rest(1),
        ];
        remove_set(&mut elements, 0);
        assert_eq!(
            elements,
            vec![set(vec![exercise(1, 0), exercise(5, 2)]), rest(1),]
        );
    }

    #[test]
    fn test_remove_set_no_rest_first_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            set(vec![exercise(1, 0)]),
            set(vec![exercise(2, 1)]),
            set(vec![exercise(3, 1)]),
        ];
        remove_set(&mut elements, 0);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(1, 0)]),
                set(vec![exercise(2, 1)]),
                set(vec![exercise(3, 1)]),
            ]
        );
    }

    #[test]
    fn test_remove_set_no_rest_second_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            set(vec![exercise(1, 0)]),
            set(vec![exercise(2, 1)]),
            set(vec![exercise(3, 1)]),
        ];
        remove_set(&mut elements, 1);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                set(vec![exercise(2, 1)]),
                set(vec![exercise(3, 1)]),
            ]
        );
    }

    #[test]
    fn test_remove_set_no_rest_penultimate_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            set(vec![exercise(1, 0)]),
            set(vec![exercise(2, 1)]),
            set(vec![exercise(3, 1)]),
        ];
        remove_set(&mut elements, 2);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                set(vec![exercise(1, 0)]),
                set(vec![exercise(3, 1)]),
            ]
        );
    }

    #[test]
    fn test_remove_set_no_rest_last_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            set(vec![exercise(1, 0)]),
            set(vec![exercise(2, 1)]),
            set(vec![exercise(3, 1)]),
        ];
        remove_set(&mut elements, 3);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                set(vec![exercise(1, 0)]),
                set(vec![exercise(2, 1)]),
            ]
        );
    }

    #[test]
    fn test_remove_set_first_single_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 1)]),
        ];
        remove_set(&mut elements, 0);
        assert_eq!(elements, vec![set(vec![exercise(1, 1)]),]);
    }

    #[test]
    fn test_remove_set_last_single_set() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 1)]),
        ];
        remove_set(&mut elements, 2);
        assert_eq!(elements, vec![set(vec![exercise(0, 0)])]);
    }

    #[test]
    fn test_remove_set_invalid_element_idx_rest() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
        ];
        remove_set(&mut elements, 1);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_remove_set_invalid_element_idx_out_of_range() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
        ];
        remove_set(&mut elements, 4);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_remove_exercise_first_set() {
        let mut elements = vec![
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
        remove_exercise(&mut elements, 0, 0);
        assert_eq!(
            elements,
            vec![
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
    fn test_remove_exercise_second_set() {
        let mut elements = vec![
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
        remove_exercise(&mut elements, 2, 0);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
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
    fn test_remove_exercise_penultimate_set() {
        let mut elements = vec![
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
        remove_exercise(&mut elements, 8, 0);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(3, 1)]),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_remove_exercise_last_set() {
        let mut elements = vec![
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
        remove_exercise(&mut elements, 10, 0);
        assert_eq!(
            elements,
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
            ]
        );
    }

    #[test]
    fn test_remove_exercise_superset_first_exercise() {
        let mut elements = vec![
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
        remove_exercise(&mut elements, 0, 0);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(1, 1)]),
                rest(0),
                set(vec![exercise(3, 1)]),
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
    fn test_remove_exercise_dropsets() {
        let mut elements = vec![
            set(vec![exercise(0, 0), exercise(1, 0)]),
            rest(0),
            set(vec![exercise(2, 0), exercise(3, 0)]),
            rest(1),
            set(vec![exercise(4, 0), exercise(5, 2)]),
            rest(2),
            set(vec![exercise(6, 0), exercise(7, 2)]),
            rest(3),
        ];
        remove_exercise(&mut elements, 0, 0);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(1, 0)]),
                rest(0),
                set(vec![exercise(3, 0)]),
                rest(1),
                set(vec![exercise(4, 0), exercise(5, 2)]),
                rest(2),
                set(vec![exercise(6, 0), exercise(7, 2)]),
                rest(3),
            ]
        );
    }

    #[test]
    fn test_remove_exercise_superset_second_exercise() {
        let mut elements = vec![
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
        remove_exercise(&mut elements, 4, 1);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0), exercise(1, 1)]),
                rest(0),
                set(vec![exercise(2, 0), exercise(3, 1)]),
                rest(1),
                set(vec![exercise(4, 0)]),
                rest(2),
                set(vec![exercise(6, 0)]),
                rest(3),
                set(vec![exercise(8, 1), exercise(9, 2)]),
                rest(4),
                set(vec![exercise(10, 1), exercise(11, 2)]),
                rest(5),
            ]
        );
    }

    #[test]
    fn test_remove_exercise_invalid_element_idx_rest() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
        ];
        remove_exercise(&mut elements, 1, 0);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_remove_exercise_invalid_element_idx_out_of_range() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
        ];
        remove_exercise(&mut elements, 4, 0);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_remove_exercise_invalid_exercise_idx_out_of_range() {
        let mut elements = vec![
            set(vec![exercise(0, 0)]),
            rest(0),
            set(vec![exercise(1, 0)]),
            rest(1),
        ];
        remove_exercise(&mut elements, 0, 2);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_remove_exercise_superset_invalid_exercise_idx_out_of_range() {
        let mut elements = vec![
            set(vec![exercise(0, 0), exercise(1, 1)]),
            rest(0),
            set(vec![exercise(2, 0), exercise(3, 1)]),
            rest(1),
        ];
        remove_exercise(&mut elements, 0, 2);
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 0), exercise(1, 1)]),
                rest(0),
                set(vec![exercise(2, 0), exercise(3, 1)]),
                rest(1),
            ]
        );
    }

    #[test]
    fn test_append_exercise_empty() {
        let mut elements = vec![];
        append_exercise(&mut elements, 1, &exercises(1));
        assert_eq!(elements, vec![set(vec![exercise(0, 1)])]);
    }

    #[test]
    fn test_append_exercise_same() {
        let mut elements = vec![set(vec![exercise(0, 1)])];
        append_exercise(&mut elements, 1, &exercises(1));
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 1)]),
                rest(0),
                set(vec![exercise(0, 1)])
            ]
        );
    }

    #[test]
    fn test_append_exercise_different() {
        let mut elements = vec![set(vec![exercise(0, 1)])];
        append_exercise(&mut elements, 2, &exercises(2));
        assert_eq!(
            elements,
            vec![
                set(vec![exercise(0, 1)]),
                rest(0),
                set(vec![exercise(0, 2)])
            ]
        );
    }

    #[test]
    fn test_determine_sections() {
        assert_eq!(
            determine_sections(&[
                set(vec![exercise(0, 0)]),
                rest(0),
                set(vec![exercise(1, 0)]),
                rest(1),
                set(vec![exercise(2, 1)]),
                rest(2),
                set(vec![exercise(4, 0), exercise(5, 2)]),
                rest(4),
                set(vec![exercise(6, 0), exercise(7, 2)]),
                rest(5),
                set(vec![exercise(8, 0), exercise(9, 2)]),
                rest(6),
            ]),
            vec![(0, 3), (4, 5), (6, 11)]
        );
        assert_eq!(
            determine_sections(&[
                set(vec![exercise(0, 0)]),
                set(vec![exercise(1, 0)]),
                set(vec![exercise(2, 1)]),
                set(vec![exercise(4, 0), exercise(5, 2)]),
                set(vec![exercise(6, 0), exercise(7, 2)]),
                set(vec![exercise(8, 0), exercise(9, 2)]),
            ]),
            vec![(0, 1), (2, 2), (3, 5)]
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
            exercise_id,
            exercise_name: exercise_id.to_string(),
            reps: InputField::default(),
            time: InputField::default(),
            weight: InputField::default(),
            rpe: InputField::default(),
            target_reps: if entry_id > 0 { Some(entry_id) } else { None },
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

    fn set(exercises: Vec<ExerciseForm>) -> FormElement {
        FormElement::Set { exercises }
    }

    fn rest(entry_id: u32) -> FormElement {
        FormElement::Rest {
            target_time: entry_id,
            automatic: true,
        }
    }
}
