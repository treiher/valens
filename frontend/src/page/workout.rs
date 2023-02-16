use std::collections::HashMap;

use chrono::{prelude::*, Duration};
use seed::{prelude::*, *};
use slice_group_by::GroupBy;

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
    let workout_id = url
        .next_hash_path_part()
        .unwrap_or("")
        .parse::<u32>()
        .unwrap_or(0);

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Workout");
    navbar.items = vec![div![
        span![C!["icon"], C!["px-5"], i![C!["fas fa-stopwatch"]]],
        ev(Ev::Click, |_| crate::Msg::Workout(Msg::ShowSMTDialog)),
        "Stopwatch / Metronome / Timer"
    ]];

    let workout = &data_model.workouts.iter().find(|w| w.id == workout_id);

    Model {
        workout_id,
        form: init_form(workout),
        previous_sets: init_previous_sets(workout, data_model),
        timer_dialog: SMTDialog {
            visible: false,
            stopwatch: Stopwatch {
                time: 0,
                start_time: None,
            },
            metronome: Metronome {
                audio_context: None,
                interval: 1,
                stressed_beat: 1,
                beat_number: 0,
                next_beat_time: 0.,
            },
            timer: Timer {
                time: (String::from("60"), Some(60)),
                reset_time: 60,
                target_time: None,
            },
        },
        timer_handle: None,
        loading: false,
    }
}

fn init_form(workout: &Option<&data::Workout>) -> Form {
    if let Some(workout) = workout {
        Form {
            notes: workout.notes.clone().unwrap_or_default(),
            notes_changed: false,
            sets: workout
                .sets
                .iter()
                .map(|s| SetForm {
                    exercise_id: s.exercise_id,
                    reps: (
                        s.reps.map(|v| v.to_string()).unwrap_or_default(),
                        true,
                        s.reps,
                        false,
                    ),
                    time: (
                        s.time.map(|v| v.to_string()).unwrap_or_default(),
                        true,
                        s.time,
                        false,
                    ),
                    weight: (
                        s.weight.map(|v| v.to_string()).unwrap_or_default(),
                        true,
                        s.weight,
                        false,
                    ),
                    rpe: (
                        s.rpe.map(|v| v.to_string()).unwrap_or_default(),
                        true,
                        s.rpe,
                        false,
                    ),
                })
                .collect::<Vec<_>>(),
        }
    } else {
        Form {
            notes: String::new(),
            notes_changed: false,
            sets: vec![],
        }
    }
}

fn init_previous_sets(
    workout: &Option<&data::Workout>,
    data_model: &data::Model,
) -> HashMap<u32, Vec<data::WorkoutSet>> {
    let mut sets: HashMap<u32, Vec<data::WorkoutSet>> = HashMap::new();
    if let Some(workout) = workout {
        if let Some(previous_workout) = &data_model
            .workouts
            .iter()
            .filter(|w| {
                w.id != workout.id
                    && w.date <= workout.date
                    && (not(workout.routine_id.is_some()) || w.routine_id == workout.routine_id)
            })
            .last()
        {
            for s in &previous_workout.sets {
                sets.entry(s.exercise_id).or_default().push(s.clone());
            }
        }
    }
    sets
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    workout_id: u32,
    form: Form,
    previous_sets: HashMap<u32, Vec<data::WorkoutSet>>,
    timer_dialog: SMTDialog,
    timer_handle: Option<StreamHandle>,
    loading: bool,
}

struct Form {
    notes: String,
    notes_changed: bool,
    sets: Vec<SetForm>,
}

#[derive(Clone)]
struct SetForm {
    exercise_id: u32,
    reps: (String, bool, Option<u32>, bool),
    time: (String, bool, Option<u32>, bool),
    weight: (String, bool, Option<f32>, bool),
    rpe: (String, bool, Option<f32>, bool),
}

struct SMTDialog {
    visible: bool,
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
    audio_context: Option<web_sys::AudioContext>,
    interval: u32,
    stressed_beat: u32,
    beat_number: u32,
    next_beat_time: f64,
}

impl Metronome {
    fn is_active(&self) -> bool {
        self.audio_context.is_some()
    }

    fn start_pause(&mut self) {
        if self.is_active() {
            self.audio_context = None;
            self.beat_number = 0;
            self.next_beat_time = 0.5;
        } else {
            self.audio_context = web_sys::AudioContext::new().ok();
        }
    }

    fn update(&mut self) {
        if self.is_active() {
            if let Some(audio_context) = &self.audio_context {
                if let Ok(oscillator) = audio_context.create_oscillator() {
                    if let Err(err) =
                        oscillator.connect_with_audio_node(&audio_context.destination())
                    {
                        error!("failed to connect oscillator:", err);
                    }
                    while self.next_beat_time < audio_context.current_time() + 0.5 {
                        if self.beat_number % self.stressed_beat == 0 {
                            oscillator.frequency().set_value(1000.);
                        } else {
                            oscillator.frequency().set_value(500.);
                        }
                        if let Err(err) = oscillator.start_with_when(self.next_beat_time) {
                            error!("failed to start oscillator:", err);
                        }
                        if let Err(err) = oscillator.stop_with_when(self.next_beat_time + 0.05) {
                            error!("failed to stop oscillator:", err);
                        }
                        self.next_beat_time += self.interval as f64;
                        self.beat_number += 1;
                    }
                }
            }
        }
    }
}

struct Timer {
    time: (String, Option<i64>),
    reset_time: i64,
    target_time: Option<DateTime<Utc>>,
}

impl Timer {
    fn is_active(&self) -> bool {
        self.target_time.is_some()
    }

    fn start_pause(&mut self) {
        self.target_time = match self.target_time {
            Some(_) => None,
            None => Some(Utc::now() + Duration::seconds(self.time.1.unwrap())),
        };
    }

    fn reset(&mut self) {
        self.time = (self.reset_time.to_string(), Some(self.reset_time));
        if self.target_time.is_some() {
            self.target_time = Some(Utc::now() + Duration::seconds(self.reset_time));
        }
    }

    fn update(&mut self) {
        if let Some(target_time) = self.target_time {
            let time = (target_time
                .signed_duration_since(Utc::now())
                .num_milliseconds() as f64
                / 1000.)
                .round() as i64;
            self.time = (time.to_string(), Some(time));
        }
    }
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    RepsChanged(usize, String),
    TimeChanged(usize, String),
    WeightChanged(usize, String),
    RPEChanged(usize, String),
    NotesChanged(String),

    SaveWorkout,
    DataEvent(data::Event),

    ShowSMTDialog,
    CloseSMTDialog,
    UpdateSMTDialog,

    StartPauseStopwatch,
    ResetStopwatch,
    ToggleStopwatch,

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
        Msg::RepsChanged(index, reps) => match reps.parse::<u32>() {
            Ok(parsed_reps) => {
                let valid = parsed_reps > 0 && parsed_reps < 1000;
                model.form.sets[index].reps = (
                    reps,
                    valid,
                    if valid { Some(parsed_reps) } else { None },
                    true,
                )
            }
            Err(_) => model.form.sets[index].reps = (reps.clone(), reps.is_empty(), None, true),
        },
        Msg::TimeChanged(index, time) => match time.parse::<u32>() {
            Ok(parsed_time) => {
                let valid = parsed_time > 0 && parsed_time < 1000;
                model.form.sets[index].time = (
                    time,
                    valid,
                    if valid { Some(parsed_time) } else { None },
                    true,
                )
            }
            Err(_) => model.form.sets[index].time = (time.clone(), time.is_empty(), None, true),
        },
        Msg::WeightChanged(index, weight) => match weight.parse::<f32>() {
            Ok(parsed_weight) => {
                let valid = parsed_weight > 0.0
                    && parsed_weight < 1000.0
                    && (parsed_weight * 10.0 % 1.0).abs() < f32::EPSILON;
                model.form.sets[index].weight = (
                    weight,
                    valid,
                    if valid { Some(parsed_weight) } else { None },
                    true,
                )
            }
            Err(_) => {
                model.form.sets[index].weight = (weight.clone(), weight.is_empty(), None, true)
            }
        },
        Msg::RPEChanged(index, rpe) => match rpe.parse::<f32>() {
            Ok(parsed_rpe) => {
                let valid =
                    (0.0..=10.0).contains(&parsed_rpe) && (parsed_rpe % 0.5).abs() < f32::EPSILON;
                model.form.sets[index].rpe = (
                    rpe,
                    valid,
                    if valid { Some(parsed_rpe) } else { None },
                    true,
                )
            }
            Err(_) => model.form.sets[index].rpe = (rpe.clone(), rpe.is_empty(), None, true),
        },
        Msg::NotesChanged(notes) => {
            model.form.notes = notes;
            model.form.notes_changed = true;
        }

        Msg::SaveWorkout => {
            model.loading = true;
            orders.notify(data::Msg::ModifyWorkout(
                model.workout_id,
                Some(model.form.notes.clone()),
                Some(
                    model
                        .form
                        .sets
                        .iter()
                        .map(|s| data::WorkoutSet {
                            exercise_id: s.exercise_id,
                            reps: s.reps.2,
                            time: s.time.2,
                            weight: s.weight.2,
                            rpe: s.rpe.2,
                        })
                        .collect::<Vec<_>>(),
                ),
            ));
        }
        Msg::DataEvent(event) => {
            match event {
                data::Event::DataChanged => {
                    let workout = &data_model
                        .workouts
                        .iter()
                        .find(|w| w.id == model.workout_id);
                    model.form = init_form(workout);
                    model.previous_sets = init_previous_sets(workout, data_model);
                }
                data::Event::WorkoutModifiedOk => {
                    model.loading = false;
                }
                _ => {}
            };
        }

        Msg::ShowSMTDialog => {
            model.timer_dialog.visible = true;
        }
        Msg::CloseSMTDialog => {
            model.timer_dialog.visible = false;
        }
        Msg::UpdateSMTDialog => {
            model.timer_dialog.stopwatch.update();
            model.timer_dialog.metronome.update();
            model.timer_dialog.timer.update();
        }

        Msg::StartPauseStopwatch => {
            model.timer_dialog.stopwatch.start_pause();
            update_timer_handle(model, orders);
        }
        Msg::ResetStopwatch => {
            model.timer_dialog.stopwatch.reset();
        }
        Msg::ToggleStopwatch => {
            model.timer_dialog.stopwatch.toggle();
            update_timer_handle(model, orders);
        }

        Msg::StartPauseMetronome => {
            model.timer_dialog.metronome.start_pause();
            update_timer_handle(model, orders);
        }
        Msg::MetronomeIntervalChanged(interval) => {
            model.timer_dialog.metronome.interval = interval.parse::<u32>().unwrap_or(1)
        }
        Msg::MetronomeStressChanged(stressed_beat) => {
            model.timer_dialog.metronome.stressed_beat = stressed_beat.parse::<u32>().unwrap_or(1)
        }

        Msg::StartPauseTimer => {
            model.timer_dialog.timer.start_pause();
            update_timer_handle(model, orders);
        }
        Msg::ResetTimer => {
            model.timer_dialog.timer.reset();
        }
        Msg::TimerTimeChanged(time) => match time.parse::<i64>() {
            Ok(parsed_time) => {
                model.timer_dialog.timer.time = (time, Some(parsed_time));
                model.timer_dialog.timer.reset_time = parsed_time;
            }
            Err(_) => model.timer_dialog.timer.time = (time, None),
        },
    }
}

fn update_timer_handle(model: &mut Model, orders: &mut impl Orders<Msg>) {
    model.timer_handle = if model.timer_dialog.stopwatch.is_active()
        || model.timer_dialog.metronome.is_active()
        || model.timer_dialog.timer.is_active()
    {
        Some(orders.stream_with_handle(streams::interval(100, || Msg::UpdateSMTDialog)))
    } else {
        None
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model, data_model: &data::Model) -> Node<Msg> {
    if data_model.workouts.is_empty() && data_model.loading_workouts {
        common::view_loading()
    } else if let Some(workout) = data_model
        .workouts
        .iter()
        .find(|w| w.id == model.workout_id)
    {
        if model.timer_dialog.visible {
            div![
                Node::NoChange,
                Node::NoChange,
                view_timer_dialog(&model.timer_dialog),
            ]
        } else {
            div![
                view_title(workout, data_model),
                view_workout_form(model, data_model)
            ]
        }
    } else {
        common::view_error_not_found("Workout")
    }
}

fn view_title(workout: &data::Workout, data_model: &data::Model) -> Node<Msg> {
    let title = if let Some(routine) = data_model
        .routines
        .iter()
        .find(|r| Some(r.id) == workout.routine_id)
    {
        span![
            workout.date.to_string(),
            " (",
            a![
                attrs! {
                    At::Href => crate::Urls::new(&data_model.base_url).routine().add_hash_path_part(routine.id.to_string()),
                },
                &routine.name
            ],
            ")"
        ]
    } else {
        span![workout.date.to_string()]
    };
    common::view_title(&title, 3)
}

fn view_workout_form(model: &Model, data_model: &data::Model) -> Node<Msg> {
    let changed = model.form.notes_changed
        || model
            .form
            .sets
            .iter()
            .any(|s| s.reps.3 || s.time.3 || s.weight.3 || s.rpe.3);
    let valid = model
        .form
        .sets
        .iter()
        .all(|s| s.reps.1 && s.time.1 && s.weight.1 && s.rpe.1);
    let save_disabled = not(changed) || not(valid);
    let mut form: std::vec::Vec<seed::virtual_dom::Node<Msg>> = nodes![];
    let mut position = 0;
    for sets in (model.form.sets[..]).linear_group_by(|a, b| a.exercise_id == b.exercise_id) {
        form.push(div![
                C!["field"],
                label![
                    C!["label"],
                    a![
                        attrs! {
                            At::Href => {
                                crate::Urls::new(&data_model.base_url)
                                    .exercise()
                                    .add_hash_path_part(sets.first().unwrap().exercise_id.to_string())
                            },
                            At::from("tabindex") => -1
                        },
                        &data_model
                            .exercises
                            .iter()
                            .find(|e| e.id == sets.first().unwrap().exercise_id)
                            .map(|e| e.name.clone())
                            .unwrap_or_default()
                    ],
                ],
                sets.iter().enumerate().map(|(j, s)| {
                    position += 1;
                    let (prev_reps, prev_time, prev_weight, prev_rpe) =
                        if let Some(prev_sets) = model.previous_sets.get(&s.exercise_id) {
                            if let Some(prev_set) = prev_sets.get(j) {
                                (prev_set.reps.map(|v| v.to_string()).unwrap_or_default(),
                                prev_set.time.map(|v| v.to_string()).unwrap_or_default(),
                                prev_set.weight.map(|v| v.to_string()).unwrap_or_default(),
                                prev_set.rpe.map(|v| v.to_string()).unwrap_or_default())
                            } else {
                                (String::new(),String::new(),String::new(),String::new())
                            }
                        } else {
                            (String::new(),String::new(),String::new(),String::new())
                        };
                    div![
                        C!["field"],
                        C!["has-addons"],
                        div![
                            C!["control"],
                            C!["has-icons-right"],
                            C!["has-text-right"],
                            input_ev(Ev::Input, move |v| Msg::RepsChanged(position - 1, v)),
                            keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                                IF!(
                                    not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                        Msg::SaveWorkout
                                    }
                                )
                            }),
                            input![
                                C!["input"],
                                C!["has-text-right"],
                                C![IF![not(s.reps.1) => "is-danger"]],
                                C![IF![s.reps.3 => "is-info"]],
                                attrs! {
                                    At::Type => "number",
                                    At::Min => 0,
                                    At::Max => 999,
                                    At::Step => 1,
                                    At::Size => 2,
                                    At::Value => s.reps.0,
                                    At::Placeholder => prev_reps,
                                }
                            ],
                            span![C!["icon"], C!["is-small"], C!["is-right"], "✕"],
                        ],
                        div![
                            C!["control"],
                            C!["has-icons-right"],
                            C!["has-text-right"],
                            input_ev(Ev::Input, move |v| Msg::TimeChanged(position - 1, v)),
                            keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                                IF!(
                                    not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                        Msg::SaveWorkout
                                    }
                                )
                            }),
                            input![
                                C!["input"],
                                C!["has-text-right"],
                                C![IF![not(s.time.1) => "is-danger"]],
                                C![IF![s.time.3 => "is-info"]],
                                attrs! {
                                    At::Type => "number",
                                    At::Min => 0,
                                    At::Max => 999,
                                    At::Step => 1,
                                    At::Size => 2,
                                    At::Value => s.time.0,
                                    At::Placeholder => prev_time,
                                },
                            ],
                            span![C!["icon"], C!["is-small"], C!["is-right"], "s"],
                        ],
                        div![
                            C!["control"],
                            C!["has-icons-right"],
                            C!["has-text-right"],
                            input_ev(Ev::Input, move |v| Msg::WeightChanged(position - 1, v)),
                            keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                                IF!(
                                    not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                        Msg::SaveWorkout
                                    }
                                )
                            }),
                            input![
                                C!["input"],
                                C!["has-text-right"],
                                C![IF![not(s.weight.1) => "is-danger"]],
                                C![IF![s.weight.3 => "is-info"]],
                                attrs! {
                                    At::from("inputmode") => "numeric",
                                    At::Size => 3,
                                    At::Value => s.weight.0,
                                    At::Placeholder => prev_weight,
                                },
                            ],
                            span![C!["icon"], C!["is-small"], C!["is-right"], "kg"],
                        ],
                        div![
                            C!["control"],
                            C!["has-icons-left"],
                            C!["has-text-right"],
                            input_ev(Ev::Input, move |v| Msg::RPEChanged(position - 1, v)),
                            keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                                IF!(
                                    not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                        Msg::SaveWorkout
                                    }
                                )
                            }),
                            input![
                                C!["input"],
                                C!["has-text-right"],
                                C![IF![not(s.rpe.1) => "is-danger"]],
                                C![IF![s.rpe.3 => "is-info"]],
                                attrs! {
                                    At::from("inputmode") => "numeric",
                                    At::Size => 2,
                                    At::Value => s.rpe.0,
                                    At::Placeholder => prev_rpe,
                                },
                            ],
                            span![C!["icon"], C!["is-small"], C!["is-left"], "@"],
                        ],
                    ]
                })
            ]);
    }
    div![
        C!["container"],
        C!["mx-2"],
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
            ev(Ev::Click, |_| Msg::SaveWorkout),
            span![C!["icon"], i![C!["fas fa-save"]]]
        ]
    ]
}

fn view_timer_dialog(dialog: &SMTDialog) -> Node<Msg> {
    div![
        C!["modal"],
        IF![dialog.visible => C!["is-active"]],
        div![
            C!["modal-background"],
            ev(Ev::Click, |_| Msg::CloseSMTDialog),
        ],
        div![
            C!["modal-content"],
            div![
                C!["box"],
                C!["mx-2"],
                div![
                    C!["block"],
                    label![C!["subtitle"], "Stopwatch"],
                    div![
                        C!["container"],
                        C!["has-text-centered"],
                        C!["p-5"],
                        p![C!["title"], C!["is-size-1"],
                        ev(Ev::Click, |_| Msg::ToggleStopwatch),
                        format!("{:.1}", dialog.stopwatch.time as f64 / 1000.)],
                        button![
                            C!["button"],
                            C!["mt-1"],
                            C!["mx-3"],
                            attrs! {At::Type => "button"},
                            ev(Ev::Click, |_| Msg::StartPauseStopwatch),
                            if dialog.stopwatch.is_active() {
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
                                                        At::Selected => (i == dialog.metronome.interval).as_at_value()
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
                                                        At::Selected => (i == dialog.metronome.stressed_beat).as_at_value()
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
                                        if dialog.metronome.is_active() {
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
                                C![IF![not(&dialog.timer.time.1.is_some()) => "is-danger"]],
                                style! {
                                    St::Height => "auto",
                                    St::Width => "auto",
                                    St::Padding => 0,
                                },
                                attrs! {
                                    At::Type => "number",
                                    At::Value => &dialog.timer.time.0,
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
                            if dialog.timer.is_active() {
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
            ],
            button![
                C!["modal-close"],
                C!["is-large"],
                ev(Ev::Click, |_| Msg::CloseSMTDialog),
            ]
        ]
    ]
}
