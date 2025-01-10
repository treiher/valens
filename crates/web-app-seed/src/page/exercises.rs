use std::collections::BTreeMap;

use seed::{prelude::*, *};
use valens_domain as domain;

use crate::{common, component, data};

// ------ ------
//     Init
// ------ ------

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>, navbar: &mut crate::Navbar) -> Model {
    if url.next_hash_path_part() == Some("add") {
        orders.send_msg(Msg::ShowAddExerciseDialog);
    }

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Exercises");

    let mut exercise_list = component::exercise_list::Model::new(true, true, true, true);
    exercise_list.search_term = url.hash_path().get(1).cloned().unwrap_or_default();

    Model {
        exercise_list,
        dialog: Dialog::Hidden,
        loading: false,
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    exercise_list: component::exercise_list::Model,
    dialog: Dialog,
    loading: bool,
}

enum Dialog {
    Hidden,
    AddExercise(Form),
    EditExercise(Form),
    DeleteExercise(u32),
}

struct Form {
    id: u32,
    name: common::InputField<String>,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowAddExerciseDialog,
    ShowEditExerciseDialog(u32),
    ShowDeleteExerciseDialog(u32),
    CloseExerciseDialog,

    ExerciseList(component::exercise_list::Msg),
    NameChanged(String),

    GoToExercise(u32),
    SaveExercise,
    DeleteExercise(u32),
    DataEvent(data::Event),
}

pub fn update(
    msg: Msg,
    model: &mut Model,
    data_model: &data::Model,
    orders: &mut impl Orders<Msg>,
) {
    match msg {
        Msg::ShowAddExerciseDialog => {
            model.dialog = Dialog::AddExercise(Form {
                id: 0,
                name: common::InputField::default(),
            });
        }
        Msg::ShowEditExerciseDialog(id) => {
            let id = data_model.exercises[&id].id;
            let name = data_model.exercises[&id].name.clone();
            model.dialog = Dialog::EditExercise(Form {
                id,
                name: common::InputField {
                    input: name.clone(),
                    parsed: Some(name.clone()),
                    orig: name,
                },
            });
        }
        Msg::ShowDeleteExerciseDialog(id) => {
            model.dialog = Dialog::DeleteExercise(id);
        }
        Msg::CloseExerciseDialog => {
            model.dialog = Dialog::Hidden;
            Url::go_and_replace(&crate::Urls::new(&data_model.base_url).exercises());
        }

        Msg::ExerciseList(msg) => {
            match component::exercise_list::update(
                msg,
                &mut model.exercise_list,
                &mut orders.proxy(Msg::ExerciseList),
            ) {
                component::exercise_list::OutMsg::None => {}
                component::exercise_list::OutMsg::CreateClicked(name) => {
                    orders.notify(data::Msg::CreateExercise(name, vec![]));
                }
                component::exercise_list::OutMsg::Selected(exercise_id) => {
                    orders.send_msg(Msg::GoToExercise(exercise_id));
                }
                component::exercise_list::OutMsg::EditClicked(exercise_id) => {
                    orders.send_msg(Msg::ShowEditExerciseDialog(exercise_id));
                }
                component::exercise_list::OutMsg::DeleteClicked(exercise_id) => {
                    orders.send_msg(Msg::ShowDeleteExerciseDialog(exercise_id));
                }
            };
            crate::Urls::new(&data_model.base_url)
                .exercises()
                .add_hash_path_part(model.exercise_list.search_term.clone())
                .go_and_replace();
        }
        Msg::NameChanged(name) => match model.dialog {
            Dialog::AddExercise(ref mut form) | Dialog::EditExercise(ref mut form) => {
                let trimmed_name = name.trim();
                if not(trimmed_name.is_empty())
                    && (trimmed_name == form.name.orig
                        || data_model
                            .exercises
                            .values()
                            .all(|e| e.name != trimmed_name))
                {
                    form.name = common::InputField {
                        input: name.clone(),
                        parsed: Some(trimmed_name.to_string()),
                        orig: form.name.orig.clone(),
                    };
                } else {
                    form.name = common::InputField {
                        input: name.clone(),
                        parsed: None,
                        orig: form.name.orig.clone(),
                    };
                }
            }
            Dialog::Hidden | Dialog::DeleteExercise(_) => {
                panic!();
            }
        },

        Msg::GoToExercise(id) => {
            let url = crate::Urls::new(&data_model.base_url)
                .exercise()
                .add_hash_path_part(id.to_string());
            url.go_and_push();
            orders.notify(subs::UrlChanged(url));
        }
        Msg::SaveExercise => {
            model.loading = true;
            match model.dialog {
                Dialog::AddExercise(ref mut form) => {
                    orders.notify(data::Msg::CreateExercise(
                        form.name.parsed.clone().unwrap(),
                        vec![],
                    ));
                }
                Dialog::EditExercise(ref mut form) => {
                    orders.notify(data::Msg::ReplaceExercise(domain::Exercise {
                        id: form.id,
                        name: form.name.parsed.clone().unwrap(),
                        muscles: vec![],
                    }));
                }
                Dialog::Hidden | Dialog::DeleteExercise(_) => {
                    panic!();
                }
            };
        }
        Msg::DeleteExercise(id) => {
            model.loading = true;
            orders.notify(data::Msg::DeleteExercise(id));
        }
        Msg::DataEvent(event) => {
            model.loading = false;
            match event {
                data::Event::ExerciseCreatedOk
                | data::Event::ExerciseReplacedOk
                | data::Event::ExerciseDeletedOk => {
                    orders.skip().send_msg(Msg::CloseExerciseDialog);
                }
                _ => {}
            };
        }
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model, data_model: &data::Model) -> Node<Msg> {
    if data_model.exercises.is_empty() && data_model.loading_exercises {
        common::view_page_loading()
    } else {
        div![
            view_exercise_dialog(&model.dialog, &data_model.exercises, model.loading),
            component::exercise_list::view(&model.exercise_list, model.loading, data_model)
                .map_msg(Msg::ExerciseList),
            common::view_fab("plus", |_| Msg::ShowAddExerciseDialog),
        ]
    }
}

fn view_exercise_dialog(
    dialog: &Dialog,
    exercises: &BTreeMap<u32, domain::Exercise>,
    loading: bool,
) -> Node<Msg> {
    let title;
    let form;
    match dialog {
        Dialog::AddExercise(ref f) => {
            title = "Add exercise";
            form = f;
        }
        Dialog::EditExercise(ref f) => {
            title = "Edit exercise";
            form = f;
        }
        Dialog::DeleteExercise(id) => {
            let exercise = &exercises[id];
            let id = exercise.id;
            return common::view_delete_confirmation_dialog(
                "exercise",
                &ev(Ev::Click, move |_| Msg::DeleteExercise(id)),
                &ev(Ev::Click, |_| Msg::CloseExerciseDialog),
                loading,
            );
        }
        Dialog::Hidden => {
            return empty![];
        }
    }
    let save_disabled = loading || not(form.name.valid());
    common::view_dialog(
        "primary",
        title,
        nodes![
            div![
                C!["field"],
                label![C!["label"], "Name"],
                div![
                    C!["control"],
                    input_ev(Ev::Input, Msg::NameChanged),
                    keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                        IF!(
                            not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                Msg::SaveExercise
                            }
                        )
                    }),
                    input![
                        C!["input"],
                        C![IF![not(form.name.valid()) => "is-danger"]],
                        C![IF![form.name.changed() => "is-info"]],
                        attrs! {
                            At::Type => "text",
                            At::Value => form.name.input,
                        }
                    ],
                ]
            ],
            div![
                C!["field"],
                C!["is-grouped"],
                C!["is-grouped-centered"],
                C!["mt-5"],
                div![
                    C!["control"],
                    button![
                        C!["button"],
                        C!["is-light"],
                        C!["is-soft"],
                        ev(Ev::Click, |_| Msg::CloseExerciseDialog),
                        "Cancel",
                    ]
                ],
                div![
                    C!["control"],
                    button![
                        C!["button"],
                        C!["is-primary"],
                        C![IF![loading => "is-loading"]],
                        attrs! {
                            At::Disabled => save_disabled.as_at_value(),
                        },
                        ev(Ev::Click, |_| Msg::SaveExercise),
                        "Save",
                    ]
                ],
            ],
        ],
        &ev(Ev::Click, |_| Msg::CloseExerciseDialog),
    )
}
