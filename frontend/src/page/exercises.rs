use std::collections::BTreeMap;

use seed::{prelude::*, *};

use crate::common;
use crate::data;

// ------ ------
//     Init
// ------ ------

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>, navbar: &mut crate::Navbar) -> Model {
    if url.next_hash_path_part() == Some("add") {
        orders.send_msg(Msg::ShowAddExerciseDialog);
    }

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Exercises");

    Model {
        search_term: String::new(),
        dialog: Dialog::Hidden,
        loading: false,
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    search_term: String,
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
    name: (String, Option<String>),
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowAddExerciseDialog,
    ShowEditExerciseDialog(u32),
    ShowDeleteExerciseDialog(u32),
    CloseExerciseDialog,

    SearchTermChanged(String),
    NameChanged(String),

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
                name: (String::new(), None),
            });
        }
        Msg::ShowEditExerciseDialog(id) => {
            let id = data_model.exercises[&id].id;
            let name = data_model.exercises[&id].name.clone();
            model.dialog = Dialog::EditExercise(Form {
                id,
                name: (name.clone(), Some(name)),
            });
        }
        Msg::ShowDeleteExerciseDialog(id) => {
            model.dialog = Dialog::DeleteExercise(id);
        }
        Msg::CloseExerciseDialog => {
            model.dialog = Dialog::Hidden;
            Url::go_and_replace(&crate::Urls::new(&data_model.base_url).exercises());
        }

        Msg::SearchTermChanged(search_term) => {
            model.search_term = search_term;
        }
        Msg::NameChanged(name) => match model.dialog {
            Dialog::AddExercise(ref mut form) | Dialog::EditExercise(ref mut form) => {
                if data_model.exercises.values().all(|e| e.name != name) {
                    form.name = (name.clone(), Some(name));
                } else {
                    form.name = (name, None);
                }
            }
            Dialog::Hidden | Dialog::DeleteExercise(_) => {
                panic!();
            }
        },

        Msg::SaveExercise => {
            model.loading = true;
            match model.dialog {
                Dialog::AddExercise(ref mut form) => {
                    orders.notify(data::Msg::CreateExercise(form.name.1.clone().unwrap()));
                }
                Dialog::EditExercise(ref mut form) => {
                    orders.notify(data::Msg::ReplaceExercise(data::Exercise {
                        id: form.id,
                        name: form.name.1.clone().unwrap(),
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
        common::view_loading()
    } else {
        div![
            view_exercise_dialog(&model.dialog, &data_model.exercises, model.loading),
            div![
                C!["px-4"],
                common::view_search_box(&model.search_term, Msg::SearchTermChanged)
            ],
            view_table(&model.search_term, data_model),
            common::view_fab("plus", |_| Msg::ShowAddExerciseDialog),
        ]
    }
}

fn view_exercise_dialog(
    dialog: &Dialog,
    exercises: &BTreeMap<u32, data::Exercise>,
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
    let save_disabled = loading || form.name.1.is_none();
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
                        C![IF![form.name.1.is_none() => "is-danger"]],
                        attrs! {
                            At::Type => "text",
                            At::Value => form.name.0,
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

fn view_table(search_term: &str, data_model: &data::Model) -> Node<Msg> {
    let mut exercises = data_model
        .exercises
        .values()
        .filter(|e| e.name.to_lowercase().contains(&search_term.to_lowercase()))
        .collect::<Vec<_>>();
    exercises.sort_by(|a, b| a.name.cmp(&b.name));

    div![
        C!["table-container"],
        C!["mt-4"],
        table![
            C!["table"],
            C!["is-fullwidth"],
            C!["is-hoverable"],
            tbody![exercises
                .iter()
                .map(|e| {
                    let id = e.id;
                    tr![td![
                        C!["is-flex"],
                        C!["is-justify-content-space-between"],
                        a![
                            attrs! {
                                At::Href => {
                                    crate::Urls::new(&data_model.base_url)
                                        .exercise()
                                        .add_hash_path_part(id.to_string())
                                }
                            },
                            e.name.to_string(),
                        ],
                        p![
                            C!["is-flex is-flex-wrap-nowrap"],
                            a![
                                C!["icon"],
                                C!["mr-1"],
                                ev(Ev::Click, move |_| Msg::ShowEditExerciseDialog(id)),
                                i![C!["fas fa-edit"]]
                            ],
                            a![
                                C!["icon"],
                                C!["ml-1"],
                                ev(Ev::Click, move |_| Msg::ShowDeleteExerciseDialog(id)),
                                i![C!["fas fa-times"]]
                            ]
                        ]
                    ]]
                })
                .collect::<Vec<_>>()],
        ]
    ]
}
