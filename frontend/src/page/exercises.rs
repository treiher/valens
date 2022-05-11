use seed::{prelude::*, *};
use serde_json::json;

use crate::common;

// ------ ------
//     Init
// ------ ------

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>) -> Model {
    let base_url = url.to_hash_base_url();

    orders.send_msg(Msg::FetchExercises);

    if url.next_hash_path_part() == Some("add") {
        orders.send_msg(Msg::ShowAddExerciseDialog);
    }

    Model {
        base_url,
        exercises: Vec::new(),
        dialog: Dialog::Hidden,
        loading: false,
        errors: Vec::new(),
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    base_url: Url,
    exercises: Vec<Exercise>,
    dialog: Dialog,
    loading: bool,
    errors: Vec<String>,
}

enum Dialog {
    Hidden,
    AddExercise(Form),
    EditExercise(Form),
    DeleteExercise(usize),
}

struct Form {
    id: u32,
    name: (String, Option<String>),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Exercise {
    pub id: u32,
    pub name: String,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    CloseErrorDialog,

    ShowAddExerciseDialog,
    ShowEditExerciseDialog(usize),
    ShowDeleteExerciseDialog(usize),
    CloseExercisesDialog,

    NameChanged(String),

    FetchExercises,
    ExercisesFetched(Result<Vec<Exercise>, String>),

    SaveExercises,
    ExercisesSaved(Result<Exercise, String>),

    DeleteExercise(u32),
    ExercisesDeleted(Result<(), String>),
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::CloseErrorDialog => {
            model.errors.remove(0);
        }

        Msg::ShowAddExerciseDialog => {
            model.dialog = Dialog::AddExercise(Form {
                id: 0,
                name: (String::new(), None),
            });
        }
        Msg::ShowEditExerciseDialog(index) => {
            let id = model.exercises[index].id;
            let name = model.exercises[index].name.clone();
            model.dialog = Dialog::EditExercise(Form {
                id,
                name: (name.clone(), Some(name)),
            });
        }
        Msg::ShowDeleteExerciseDialog(index) => {
            model.dialog = Dialog::DeleteExercise(index);
        }
        Msg::CloseExercisesDialog => {
            model.dialog = Dialog::Hidden;
            Url::go_and_replace(&crate::Urls::new(&model.base_url).exercises());
        }

        Msg::NameChanged(name) => match model.dialog {
            Dialog::AddExercise(ref mut form) | Dialog::EditExercise(ref mut form) => {
                if model.exercises.iter().all(|e| e.name != name) {
                    form.name = (name.clone(), Some(name));
                } else {
                    form.name = (name, None);
                }
            }
            Dialog::Hidden | Dialog::DeleteExercise(_) => {
                panic!();
            }
        },

        Msg::FetchExercises => {
            orders
                .skip()
                .perform_cmd(async { common::fetch("api/exercises", Msg::ExercisesFetched).await });
        }
        Msg::ExercisesFetched(Ok(exercises)) => {
            model.exercises = exercises;
        }
        Msg::ExercisesFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch exercises: ".to_owned() + &message);
        }

        Msg::SaveExercises => {
            model.loading = true;
            let request = match model.dialog {
                Dialog::AddExercise(ref mut form) => Request::new("api/exercises")
                    .method(Method::Post)
                    .json(&json!({ "name": form.name.1.clone().unwrap() }))
                    .expect("serialization failed"),
                Dialog::EditExercise(ref mut form) => {
                    Request::new(format!("api/exercises/{}", form.id))
                        .method(Method::Put)
                        .json(&Exercise {
                            id: form.id,
                            name: form.name.1.clone().unwrap(),
                        })
                        .expect("serialization failed")
                }
                Dialog::Hidden | Dialog::DeleteExercise(_) => {
                    panic!();
                }
            };
            orders.perform_cmd(async move { common::fetch(request, Msg::ExercisesSaved).await });
        }
        Msg::ExercisesSaved(Ok(_)) => {
            model.loading = false;
            orders
                .skip()
                .send_msg(Msg::FetchExercises)
                .send_msg(Msg::CloseExercisesDialog);
        }
        Msg::ExercisesSaved(Err(message)) => {
            model.loading = false;
            model
                .errors
                .push("Failed to save exercises: ".to_owned() + &message);
        }

        Msg::DeleteExercise(date) => {
            model.loading = true;
            let request = Request::new(format!("api/exercises/{}", date)).method(Method::Delete);
            orders.perform_cmd(async move {
                common::fetch_no_content(request, Msg::ExercisesDeleted).await
            });
        }
        Msg::ExercisesDeleted(Ok(_)) => {
            model.loading = false;
            orders
                .skip()
                .send_msg(Msg::FetchExercises)
                .send_msg(Msg::CloseExercisesDialog);
        }
        Msg::ExercisesDeleted(Err(message)) => {
            model.loading = false;
            model
                .errors
                .push("Failed to delete exercises: ".to_owned() + &message);
        }
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model) -> Node<Msg> {
    div![
        view_exercise_dialog(&model.dialog, &model.exercises, model.loading),
        common::view_error_dialog(&model.errors, &ev(Ev::Click, |_| Msg::CloseErrorDialog)),
        common::view_fab(|_| Msg::ShowAddExerciseDialog),
        view_table(model),
    ]
}

fn view_exercise_dialog(dialog: &Dialog, exercises: &[Exercise], loading: bool) -> Node<Msg> {
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
        Dialog::DeleteExercise(index) => {
            let exercise = &exercises[*index];
            let id = exercise.id;
            return common::view_delete_confirmation_dialog(
                "exercise",
                &ev(Ev::Click, move |_| Msg::DeleteExercise(id)),
                &ev(Ev::Click, |_| Msg::CloseExercisesDialog),
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
                        ev(Ev::Click, |_| Msg::CloseExercisesDialog),
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
                        ev(Ev::Click, |_| Msg::SaveExercises),
                        "Save",
                    ]
                ],
            ],
        ],
        &ev(Ev::Click, |_| Msg::CloseExercisesDialog),
    )
}

fn view_table(model: &Model) -> Node<Msg> {
    div![
        C!["table-container"],
        C!["mt-4"],
        table![
            C!["table"],
            C!["is-fullwidth"],
            C!["is-hoverable"],
            thead![tr![th!["Name"], th![]]],
            tbody![&model
                .exercises
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let id = e.id;
                    tr![
                        td![a![
                            attrs! {
                                At::Href => {
                                    crate::Urls::new(&model.base_url)
                                        .exercise()
                                        .add_hash_path_part(id.to_string())
                                }
                            },
                            e.name.to_string(),
                        ]],
                        td![p![
                            C!["is-flex is-flex-wrap-nowrap"],
                            a![
                                C!["icon"],
                                C!["mr-1"],
                                ev(Ev::Click, move |_| Msg::ShowEditExerciseDialog(i)),
                                i![C!["fas fa-edit"]]
                            ],
                            a![
                                C!["icon"],
                                C!["ml-1"],
                                ev(Ev::Click, move |_| Msg::ShowDeleteExerciseDialog(i)),
                                i![C!["fas fa-times"]]
                            ]
                        ]]
                    ]
                })
                .collect::<Vec<_>>()],
        ]
    ]
}
