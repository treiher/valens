use std::collections::BTreeMap;

use chrono::prelude::*;
use seed::{prelude::*, *};
use valens_domain::{self as domain, Property};
use valens_web_app as web_app;

use crate::{common, data, page::training};

// ------ ------
//     Init
// ------ ------

pub fn init(
    mut url: Url,
    orders: &mut impl Orders<Msg>,
    data_model: &data::Model,
    navbar: &mut crate::Navbar,
) -> Model {
    let exercise_id = url
        .next_hash_path_part()
        .unwrap_or("")
        .parse::<u128>()
        .unwrap_or_default()
        .into();
    let editing = url.next_hash_path_part() == Some("edit");

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Exercise");

    let mut model = Model {
        interval: domain::init_interval(&[], domain::DefaultInterval::_3M),
        exercise_id,
        name: common::InputField::default(),
        muscle_stimulus: BTreeMap::new(),
        dialog: Dialog::Hidden,
        editing,
        loading: false,
    };

    update_model(&mut model, data_model);

    model
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    interval: domain::Interval,
    exercise_id: domain::ExerciseID,
    name: common::InputField<domain::Name>,
    muscle_stimulus: BTreeMap<domain::MuscleID, domain::Stimulus>,
    dialog: Dialog,
    editing: bool,
    loading: bool,
}

impl Model {
    pub fn has_unsaved_changes(&self) -> bool {
        self.name.changed()
    }

    pub fn mark_as_unchanged(&mut self) {
        self.name.input = self.name.parsed.clone().unwrap().to_string();
        self.name.orig = self.name.parsed.clone().unwrap().to_string();
    }

    fn saving_disabled(&self) -> bool {
        self.loading || not(self.name.valid())
    }
}

enum Dialog {
    Hidden,
    DeleteTrainingSession(domain::TrainingSessionID),
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    EditExercise,
    SaveExercise,

    ShowDeleteTrainingSessionDialog(domain::TrainingSessionID),
    CloseDialog,

    NameChanged(String),
    SetMuscleStimulus(domain::MuscleID, domain::Stimulus),

    DeleteTrainingSession(domain::TrainingSessionID),
    DataEvent(data::Event),

    ChangeInterval(NaiveDate, NaiveDate),
}

pub fn update(
    msg: Msg,
    model: &mut Model,
    data_model: &data::Model,
    orders: &mut impl Orders<Msg>,
) {
    match msg {
        Msg::EditExercise => {
            model.editing = true;
            Url::go_and_push(
                &crate::Urls::new(&data_model.base_url)
                    .exercise()
                    .add_hash_path_part(model.exercise_id.as_u128().to_string())
                    .add_hash_path_part("edit"),
            );
        }
        Msg::SaveExercise => {
            model.loading = true;
            orders.notify(data::Msg::ReplaceExercise(domain::Exercise {
                id: model.exercise_id,
                name: model.name.parsed.clone().unwrap(),
                muscles: model
                    .muscle_stimulus
                    .iter()
                    .map(|(muscle_id, stimulus)| domain::ExerciseMuscle {
                        muscle_id: *muscle_id,
                        stimulus: *stimulus,
                    })
                    .collect(),
            }));
        }

        Msg::ShowDeleteTrainingSessionDialog(id) => {
            model.dialog = Dialog::DeleteTrainingSession(id);
        }
        Msg::CloseDialog => {
            model.dialog = Dialog::Hidden;
            model.loading = false;
            Url::go_and_replace(
                &crate::Urls::new(&data_model.base_url)
                    .routine()
                    .add_hash_path_part(model.exercise_id.as_u128().to_string()),
            );
        }

        Msg::NameChanged(name) => {
            let parsed = domain::Name::new(&name).ok().and_then(|name| {
                if name.as_ref() == &model.name.orig
                    || data_model.exercises.values().all(|e| e.name != name)
                {
                    Some(name)
                } else {
                    None
                }
            });
            model.name = common::InputField {
                input: name,
                parsed,
                orig: model.name.orig.clone(),
            };
        }
        Msg::SetMuscleStimulus(muscle_id, stimulus) => match stimulus {
            domain::Stimulus::NONE => {
                model.muscle_stimulus.remove(&muscle_id);
            }
            _ => {
                model.muscle_stimulus.insert(muscle_id, stimulus);
            }
        },

        Msg::DeleteTrainingSession(id) => {
            model.loading = true;
            orders.notify(data::Msg::DeleteTrainingSession(id));
        }
        Msg::DataEvent(event) => {
            model.loading = false;
            match event {
                data::Event::DataChanged => {
                    update_model(model, data_model);
                }
                data::Event::ExerciseReplacedOk => {
                    model.editing = false;
                    model.mark_as_unchanged();
                    Url::go_and_push(
                        &crate::Urls::new(&data_model.base_url)
                            .exercise()
                            .add_hash_path_part(model.exercise_id.as_u128().to_string()),
                    );
                }
                data::Event::TrainingSessionDeletedOk => {
                    orders.skip().send_msg(Msg::CloseDialog);
                }
                _ => {}
            };
        }

        Msg::ChangeInterval(first, last) => {
            model.interval.first = first;
            model.interval.last = last;
        }
    }
}

fn update_model(model: &mut Model, data_model: &data::Model) {
    model.interval = domain::init_interval(
        &data_model
            .training_sessions
            .values()
            .filter(|t| t.exercises().contains(&model.exercise_id))
            .map(|t| t.date)
            .collect::<Vec<NaiveDate>>(),
        domain::DefaultInterval::_3M,
    );

    let exercise = &data_model.exercises.get(&model.exercise_id);

    if let Some(exercise) = exercise {
        model.name = common::InputField {
            input: exercise.name.to_string(),
            parsed: Some(exercise.name.clone()),
            orig: exercise.name.to_string(),
        };
        model.muscle_stimulus = exercise.muscle_stimulus();
    };
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model, data_model: &data::Model) -> Node<Msg> {
    if data_model.exercises.is_empty() && data_model.loading_exercises > 0 {
        common::view_page_loading()
    } else if data_model.exercises.contains_key(&model.exercise_id) {
        let exercise_training_sessions = exercise_training_sessions(model, data_model);
        let dates = exercise_training_sessions.iter().map(|t| t.date);
        let exercise_interval = domain::Interval {
            first: dates.clone().min().unwrap_or_default(),
            last: dates.max().unwrap_or_default(),
        };
        let training_sessions = exercise_training_sessions
            .iter()
            .filter(|t| t.date >= model.interval.first && t.date <= model.interval.last)
            .collect::<Vec<_>>();
        div![
            view_title(model),
            view_muscles(model),
            if model.editing {
                nodes![button![
                    C!["button"],
                    C!["is-fab"],
                    C!["is-medium"],
                    C!["is-link"],
                    C![IF![model.loading => "is-loading"]],
                    attrs![
                        At::Disabled => model.saving_disabled().as_at_value(),
                    ],
                    ev(Ev::Click, |_| Msg::SaveExercise),
                    span![C!["icon"], i![C!["fas fa-save"]]]
                ]]
            } else {
                nodes![
                    if training_sessions.is_empty() {
                        nodes![if data_model.loading_training_sessions > 0 {
                            common::view_loading()
                        } else {
                            common::view_no_data()
                        }]
                    } else {
                        nodes![
                            common::view_interval_buttons(
                                &model.interval,
                                &exercise_interval,
                                Msg::ChangeInterval
                            ),
                            view_charts(
                                &training_sessions,
                                &model.interval,
                                data_model.theme(),
                                data_model.settings.show_rpe,
                                data_model.settings.show_tut,
                            ),
                            view_calendar(&training_sessions, &model.interval),
                            training::view_table(
                                &training_sessions,
                                &data_model.routines,
                                &data_model.base_url,
                                Msg::ShowDeleteTrainingSessionDialog,
                                data_model.settings.show_rpe,
                                data_model.settings.show_tut,
                            ),
                            view_sets(
                                &training_sessions,
                                &data_model.routines,
                                &data_model.base_url,
                                data_model.settings.show_rpe,
                                data_model.settings.show_tut,
                            ),
                        ]
                    },
                    view_dialog(&model.dialog, model.loading, data_model),
                    common::view_fab("edit", |_| Msg::EditExercise),
                ]
            },
        ]
    } else {
        common::view_error_not_found("Exercise")
    }
}

fn view_title(model: &Model) -> Node<Msg> {
    div![
        C!["mx-2"],
        C!["mb-5"],
        if model.editing {
            let saving_disabled = model.saving_disabled();
            div![
                C!["field"],
                div![
                    C!["control"],
                    input_ev(Ev::Input, Msg::NameChanged),
                    keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                        IF!(
                            not(saving_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                Msg::SaveExercise
                            }
                        )
                    }),
                    input![
                        C!["input"],
                        C![IF![not(model.name.valid()) => "is-danger"]],
                        C![IF![model.name.changed() => "is-info"]],
                        attrs! {
                            At::Type => "text",
                            At::Value => model.name.input,
                        }
                    ]
                ],
            ]
        } else {
            common::view_title(&span![&model.name.input], 0)
        }
    ]
}

fn view_muscles(model: &Model) -> Node<Msg> {
    let muscles = domain::MuscleID::iter()
        .map(|m| {
            let stimulus = model
                .muscle_stimulus
                .get(m)
                .copied()
                .unwrap_or(domain::Stimulus::NONE);
            (m, stimulus)
        })
        .collect::<Vec<_>>();
    if model.editing {
        div![
            C!["mx-2"],
            C!["mb-5"],
            muscles.iter().map(|(m, stimulus)| {
                let m = **m;
                div![
                    C!["columns"],
                    C!["is-mobile"],
                    div![
                        C!["column"],
                        p![m.name()],
                        p![C!["is-size-7"], m.description()]
                    ],
                    div![
                        C!["column"],
                        C!["is-flex"],
                        div![
                            C!["field"],
                            C!["has-addons"],
                            C!["has-addons-centered"],
                            C!["my-auto"],
                            p![
                                C!["control"],
                                a![
                                    C!["button"],
                                    C!["is-small"],
                                    C![IF![*stimulus == domain::Stimulus::PRIMARY => "is-link"]],
                                    &ev(Ev::Click, move |_| Msg::SetMuscleStimulus(
                                        m,
                                        domain::Stimulus::PRIMARY
                                    )),
                                    "primary",
                                ]
                            ],
                            p![
                                C!["control"],
                                a![
                                    C!["button"],
                                    C!["is-small"],
                                    C![IF![**stimulus > *domain::Stimulus::NONE && **stimulus < *domain::Stimulus::PRIMARY => "is-link"]],
                                    &ev(Ev::Click, move |_| Msg::SetMuscleStimulus(
                                        m,
                                        domain::Stimulus::SECONDARY
                                    )),
                                    "secondary",
                                ]
                            ],
                            p![
                                C!["control"],
                                a![
                                    C!["button"],
                                    C!["is-small"],
                                    C![IF![*stimulus == domain::Stimulus::NONE => "is-link"]],
                                    &ev(Ev::Click, move |_| Msg::SetMuscleStimulus(m, domain::Stimulus::NONE)),
                                    "none",
                                ]
                            ],
                        ],
                    ]
                ]
            })
        ]
    } else {
        let mut muscles = muscles
            .iter()
            .filter(|(_, stimulus)| **stimulus > *domain::Stimulus::NONE)
            .collect::<Vec<_>>();
        muscles.sort_by(|a, b| b.1.cmp(&a.1));
        if muscles.is_empty() {
            empty![]
        } else {
            div![
                C!["tags"],
                C!["is-centered"],
                C!["mx-2"],
                C!["mb-5"],
                muscles.iter().map(|(m, stimulus)| {
                    common::view_element_with_description(
                        span![
                            C!["tag"],
                            C!["is-link"],
                            C![IF![**stimulus < *domain::Stimulus::PRIMARY => "is-light"]],
                            m.name()
                        ],
                        m.description(),
                    )
                })
            ]
        }
    }
}

pub fn view_charts<Ms>(
    training_sessions: &[&domain::TrainingSession],
    interval: &domain::Interval,
    theme: web_app::Theme,
    show_rpe: bool,
    show_tut: bool,
) -> Vec<Node<Ms>> {
    let mut set_volume: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut volume_load: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut tut: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut reps_rpe: BTreeMap<NaiveDate, (Vec<f32>, Vec<domain::RPE>)> = BTreeMap::new();
    for training_session in training_sessions {
        #[allow(clippy::cast_precision_loss)]
        set_volume
            .entry(training_session.date)
            .and_modify(|e| *e += training_session.set_volume() as f32)
            .or_insert(training_session.set_volume() as f32);
        #[allow(clippy::cast_precision_loss)]
        volume_load
            .entry(training_session.date)
            .and_modify(|e| *e += training_session.volume_load() as f32)
            .or_insert(training_session.volume_load() as f32);
        #[allow(clippy::cast_precision_loss)]
        tut.entry(training_session.date)
            .and_modify(|e| *e += training_session.tut().unwrap_or_default() as f32)
            .or_insert(training_session.tut().unwrap_or_default() as f32);
        if let Some(avg_reps) = training_session.avg_reps() {
            reps_rpe
                .entry(training_session.date)
                .and_modify(|e| e.0.push(avg_reps))
                .or_insert((vec![avg_reps], vec![]));
        }
        if let Some(avg_rpe) = training_session.avg_rpe() {
            reps_rpe
                .entry(training_session.date)
                .and_modify(|e| e.1.push(avg_rpe));
        }
    }

    let mut labels = vec![(
        "Repetitions",
        web_app::chart::COLOR_REPS,
        web_app::chart::OPACITY_LINE,
    )];
    let reps_rpe_values = reps_rpe
        .iter()
        .map(|(date, (avg_reps, _))| {
            #[allow(clippy::cast_precision_loss)]
            (*date, avg_reps.iter().sum::<f32>() / avg_reps.len() as f32)
        })
        .collect::<Vec<_>>();

    let mut data = vec![];

    if show_rpe {
        let rir_values = reps_rpe
            .into_iter()
            .filter_map(|(date, (avg_reps_values, avg_rpe_values))| {
                #[allow(clippy::cast_precision_loss)]
                let avg_reps = avg_reps_values.iter().sum::<f32>() / avg_reps_values.len() as f32;
                domain::RPE::avg(&avg_rpe_values)
                    .map(|avg_rpe| (date, avg_reps + f32::from(domain::RIR::from(avg_rpe))))
            })
            .collect::<Vec<_>>();
        if !rir_values.is_empty() {
            labels.push((
                "+ Repetitions in reserve",
                web_app::chart::COLOR_REPS_RIR,
                web_app::chart::OPACITY_AREA,
            ));
            data.push(web_app::chart::PlotData {
                values_high: rir_values,
                values_low: Some(reps_rpe_values.clone()),
                plots: web_app::chart::plot_area(web_app::chart::COLOR_REPS_RIR),
                params: web_app::chart::PlotParams::primary_range(0., 10.),
            });
        }
    }

    data.push(web_app::chart::PlotData {
        values_high: reps_rpe_values,
        values_low: None,
        plots: web_app::chart::plot_line(web_app::chart::COLOR_REPS),
        params: web_app::chart::PlotParams::primary_range(0., 10.),
    });

    nodes![
        common::view_chart(
            &[(
                "Set volume",
                web_app::chart::COLOR_SET_VOLUME,
                web_app::chart::OPACITY_LINE
            )],
            web_app::chart::plot(
                &[web_app::chart::PlotData {
                    values_high: set_volume.into_iter().collect::<Vec<_>>(),
                    values_low: None,
                    plots: web_app::chart::plot_area_with_border(
                        web_app::chart::COLOR_SET_VOLUME,
                        web_app::chart::COLOR_SET_VOLUME,
                    ),
                    params: web_app::chart::PlotParams::primary_range(0., 10.),
                }],
                interval,
                theme,
            ),
            false,
        ),
        common::view_chart(
            &[(
                "Volume load",
                web_app::chart::COLOR_VOLUME_LOAD,
                web_app::chart::OPACITY_LINE
            )],
            web_app::chart::plot(
                &[web_app::chart::PlotData {
                    values_high: volume_load.into_iter().collect::<Vec<_>>(),
                    values_low: None,
                    plots: web_app::chart::plot_area_with_border(
                        web_app::chart::COLOR_VOLUME_LOAD,
                        web_app::chart::COLOR_VOLUME_LOAD,
                    ),
                    params: web_app::chart::PlotParams::primary_range(0., 10.),
                }],
                interval,
                theme,
            ),
            false,
        ),
        IF![show_tut =>
            common::view_chart(
                &[("Time under tension (s)", web_app::chart::COLOR_TUT, web_app::chart::OPACITY_LINE)],
                web_app::chart::plot(
                    &[web_app::chart::PlotData {
                        values_high: tut.into_iter().collect::<Vec<_>>(),
                        values_low: None,
                        plots: web_app::chart::plot_area_with_border(web_app::chart::COLOR_TUT, web_app::chart::COLOR_TUT),
                        params: web_app::chart::PlotParams::primary_range(0., 10.),
                    }],
                    interval,
                    theme,
                ),
                false,
            )
        ],
        common::view_chart(&labels, web_app::chart::plot(&data, interval, theme), false,),
        common::view_chart(
            &[
                (
                    "Weight (kg)",
                    web_app::chart::COLOR_WEIGHT,
                    web_app::chart::OPACITY_AREA
                ),
                (
                    "Avg. weight (kg)",
                    web_app::chart::COLOR_WEIGHT,
                    web_app::chart::OPACITY_LINE
                )
            ],
            web_app::chart::plot_min_avg_max(
                &training_sessions
                    .iter()
                    .flat_map(|s| s
                        .elements
                        .iter()
                        .filter_map(|e| match e {
                            domain::TrainingSessionElement::Set { weight, .. } =>
                                weight.map(|w| (s.date, w)),
                            _ => None,
                        })
                        .collect::<Vec<_>>())
                    .collect::<Vec<_>>(),
                interval,
                web_app::chart::PlotParams::primary_range(0., 10.),
                web_app::chart::COLOR_WEIGHT,
                theme,
            ),
            false,
        ),
        IF![show_tut =>
            common::view_chart(
                &[
                    ("Time (s)", web_app::chart::COLOR_TIME, web_app::chart::OPACITY_AREA),
                    ("Avg. time (s)", web_app::chart::COLOR_TIME, web_app::chart::OPACITY_LINE)
                ],
                web_app::chart::plot_min_avg_max(
                    &training_sessions
                        .iter()
                        .flat_map(|s| s
                            .elements
                            .iter()
                            .filter_map(|e| match e {
                                #[allow(clippy::cast_precision_loss)]
                                domain::TrainingSessionElement::Set { time, .. } =>
                                    time.map(|v| (s.date, u32::from(v) as f32)),
                                _ => None,
                            })
                            .collect::<Vec<_>>())
                        .collect::<Vec<_>>(),
                    interval,
                    web_app::chart::PlotParams::primary_range(0., 10.),
                    web_app::chart::COLOR_TIME,
                    theme,
                ),
                false,
            )
        ],
    ]
}

fn view_calendar(
    training_sessions: &[&domain::TrainingSession],
    interval: &domain::Interval,
) -> Node<Msg> {
    let mut volume_load: BTreeMap<NaiveDate, u32> = BTreeMap::new();
    for training_session in training_sessions {
        if (interval.first..=interval.last).contains(&training_session.date) {
            volume_load
                .entry(training_session.date)
                .and_modify(|e| *e += training_session.volume_load())
                .or_insert(training_session.volume_load());
        }
    }
    let min = volume_load
        .values()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0);
    let max = volume_load
        .values()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0);

    common::view_calendar(
        volume_load
            .iter()
            .map(|(date, volume_load)| {
                (
                    *date,
                    web_app::chart::COLOR_VOLUME_LOAD,
                    if max > min {
                        (f64::from(volume_load - min) / f64::from(max - min)) * 0.8 + 0.2
                    } else {
                        1.0
                    },
                )
            })
            .collect(),
        interval,
    )
}

fn view_sets(
    training_sessions: &[&domain::TrainingSession],
    routines: &BTreeMap<domain::RoutineID, domain::Routine>,
    base_url: &Url,
    show_rpe: bool,
    show_tut: bool,
) -> Vec<Node<Msg>> {
    training_sessions
            .iter()
            .rev()
            .flat_map(|t| {
                nodes![
                    div![
                        C!["block"],
                        C!["has-text-centered"],
                        C!["has-text-weight-bold"],
                        C!["mb-2"],
                        a![
                            attrs! {
                                At::Href => crate::Urls::new(base_url).training_session().add_hash_path_part(t.id.as_u128().to_string()),
                            },
                            common::no_wrap(&t.date.to_string())
                        ],
                        raw!["&emsp;"],
                        if t.routine_id.is_nil() {
                            plain!["-"]
                        } else {
                            a![
                                attrs! {
                                    At::Href => crate::Urls::new(base_url).routine().add_hash_path_part(t.routine_id.as_u128().to_string()),
                                },
                                match &routines.get(&t.routine_id) {
                                    Some(routine) => raw![&routine.name.as_ref()],
                                    None => vec![common::view_loading()]
                                }
                            ]
                        }
                    ],
                    div![
                        C!["block"],
                        C!["has-text-centered"],
                        t.elements.iter().map(|e| {
                            if let domain::TrainingSessionElement::Set {
                                reps,
                                time,
                                weight,
                                rpe,
                                ..
                            } = e {
                                div![common::no_wrap(&common::format_set(*reps, *time, show_tut, *weight, *rpe, show_rpe))]
                            } else {
                                empty![]
                            }
                        })
                    ]
                ]
            })
            .collect::<Vec<_>>()
}

fn view_dialog(dialog: &Dialog, loading: bool, data_model: &data::Model) -> Node<Msg> {
    match dialog {
        Dialog::DeleteTrainingSession(id) => {
            #[allow(clippy::clone_on_copy)]
            let id = id.clone();
            let date = data_model
                .training_sessions
                .get(&id)
                .map(|t| t.date)
                .unwrap_or_default();
            common::view_delete_confirmation_dialog(
                "training session",
                &span!["of ", common::no_wrap(&date.to_string())],
                &ev(Ev::Click, move |_| Msg::DeleteTrainingSession(id)),
                &ev(Ev::Click, |_| Msg::CloseDialog),
                loading,
            )
        }
        Dialog::Hidden => {
            empty![]
        }
    }
}

fn exercise_training_sessions(
    model: &Model,
    data_model: &data::Model,
) -> Vec<domain::TrainingSession> {
    data_model
        .training_sessions
        .values()
        .filter(|t| t.exercises().contains(&model.exercise_id))
        .map(|t| domain::TrainingSession {
            id: t.id,
            routine_id: t.routine_id,
            date: t.date,
            notes: t.notes.clone(),
            elements: t
                .elements
                .iter()
                .filter(|e| match e {
                    domain::TrainingSessionElement::Set { exercise_id, .. } => {
                        *exercise_id == model.exercise_id
                    }
                    _ => false,
                })
                .cloned()
                .collect::<Vec<_>>(),
        })
        .collect::<Vec<_>>()
}
