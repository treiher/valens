use seed::{prelude::*, *};
use serde_json::json;

use crate::common;

// ------ ------
//     Init
// ------ ------

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>) -> Model {
    let base_url = url.to_hash_base_url();

    orders.skip().send_msg(Msg::FetchRoutines);

    if url.next_hash_path_part() == Some("add") {
        orders.send_msg(Msg::ShowAddRoutineDialog);
    }

    Model {
        base_url,
        routines: Vec::new(),
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
    routines: Vec<Routine>,
    dialog: Dialog,
    loading: bool,
    errors: Vec<String>,
}

enum Dialog {
    Hidden,
    AddRoutine(Form),
    EditRoutine(Form),
    DeleteRoutine(usize),
}

struct Form {
    id: u32,
    name: (String, Option<String>),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Routine {
    pub id: u32,
    pub name: String,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    CloseErrorDialog,

    ShowAddRoutineDialog,
    ShowEditRoutineDialog(usize),
    ShowDeleteRoutineDialog(usize),
    CloseRoutineDialog,

    NameChanged(String),

    FetchRoutines,
    RoutinesFetched(Result<Vec<Routine>, String>),

    SaveRoutine,
    RoutineSaved(Result<Routine, String>),

    DeleteRoutine(u32),
    RoutineDeleted(Result<(), String>),
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::CloseErrorDialog => {
            model.errors.remove(0);
        }

        Msg::ShowAddRoutineDialog => {
            model.dialog = Dialog::AddRoutine(Form {
                id: 0,
                name: (String::new(), None),
            });
        }
        Msg::ShowEditRoutineDialog(index) => {
            let id = model.routines[index].id;
            let name = model.routines[index].name.clone();
            model.dialog = Dialog::EditRoutine(Form {
                id,
                name: (name.clone(), Some(name)),
            });
        }
        Msg::ShowDeleteRoutineDialog(index) => {
            model.dialog = Dialog::DeleteRoutine(index);
        }
        Msg::CloseRoutineDialog => {
            model.dialog = Dialog::Hidden;
            Url::go_and_replace(&crate::Urls::new(&model.base_url).routines());
        }

        Msg::NameChanged(name) => match model.dialog {
            Dialog::AddRoutine(ref mut form) | Dialog::EditRoutine(ref mut form) => {
                if model.routines.iter().all(|e| e.name != name) {
                    form.name = (name.clone(), Some(name));
                } else {
                    form.name = (name, None);
                }
            }
            Dialog::Hidden | Dialog::DeleteRoutine(_) => {
                panic!();
            }
        },

        Msg::FetchRoutines => {
            orders
                .skip()
                .perform_cmd(async { common::fetch("api/routines", Msg::RoutinesFetched).await });
        }
        Msg::RoutinesFetched(Ok(routines)) => {
            model.routines = routines;
            model.routines.sort_by(|a, b| b.id.cmp(&a.id));
        }
        Msg::RoutinesFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch routines: ".to_owned() + &message);
        }

        Msg::SaveRoutine => {
            model.loading = true;
            let request = match model.dialog {
                Dialog::AddRoutine(ref mut form) => Request::new("api/routines")
                    .method(Method::Post)
                    .json(&json!({ "name": form.name.1.clone().unwrap() }))
                    .expect("serialization failed"),
                Dialog::EditRoutine(ref mut form) => {
                    Request::new(format!("api/routines/{}", form.id))
                        .method(Method::Put)
                        .json(&Routine {
                            id: form.id,
                            name: form.name.1.clone().unwrap(),
                        })
                        .expect("serialization failed")
                }
                Dialog::Hidden | Dialog::DeleteRoutine(_) => {
                    panic!();
                }
            };
            orders.perform_cmd(async move { common::fetch(request, Msg::RoutineSaved).await });
        }
        Msg::RoutineSaved(Ok(_)) => {
            model.loading = false;
            orders
                .skip()
                .send_msg(Msg::FetchRoutines)
                .send_msg(Msg::CloseRoutineDialog);
        }
        Msg::RoutineSaved(Err(message)) => {
            model.loading = false;
            model
                .errors
                .push("Failed to save routine: ".to_owned() + &message);
        }

        Msg::DeleteRoutine(date) => {
            model.loading = true;
            let request = Request::new(format!("api/routines/{}", date)).method(Method::Delete);
            orders.perform_cmd(async move {
                common::fetch_no_content(request, Msg::RoutineDeleted).await
            });
        }
        Msg::RoutineDeleted(Ok(_)) => {
            model.loading = false;
            orders
                .skip()
                .send_msg(Msg::FetchRoutines)
                .send_msg(Msg::CloseRoutineDialog);
        }
        Msg::RoutineDeleted(Err(message)) => {
            model.loading = false;
            model
                .errors
                .push("Failed to delete routine: ".to_owned() + &message);
        }
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model) -> Node<Msg> {
    div![
        view_routine_dialog(&model.dialog, &model.routines, model.loading),
        common::view_error_dialog(&model.errors, &ev(Ev::Click, |_| Msg::CloseErrorDialog)),
        common::view_fab(|_| Msg::ShowAddRoutineDialog),
        view_table(model),
    ]
}

fn view_routine_dialog(dialog: &Dialog, routines: &[Routine], loading: bool) -> Node<Msg> {
    let title;
    let form;
    match dialog {
        Dialog::AddRoutine(ref f) => {
            title = "Add routine";
            form = f;
        }
        Dialog::EditRoutine(ref f) => {
            title = "Edit routine";
            form = f;
        }
        Dialog::DeleteRoutine(index) => {
            let routine = &routines[*index];
            let id = routine.id;
            return common::view_delete_confirmation_dialog(
                "routine",
                &ev(Ev::Click, move |_| Msg::DeleteRoutine(id)),
                &ev(Ev::Click, |_| Msg::CloseRoutineDialog),
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
                        ev(Ev::Click, |_| Msg::CloseRoutineDialog),
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
                        ev(Ev::Click, |_| Msg::SaveRoutine),
                        "Save",
                    ]
                ],
            ],
        ],
        &ev(Ev::Click, |_| Msg::CloseRoutineDialog),
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
                .routines
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let id = e.id;
                    tr![
                        td![a![
                            attrs! {
                                At::Href => {
                                    crate::Urls::new(&model.base_url)
                                        .routine()
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
                                ev(Ev::Click, move |_| Msg::ShowEditRoutineDialog(i)),
                                i![C!["fas fa-edit"]]
                            ],
                            a![
                                C!["icon"],
                                C!["ml-1"],
                                ev(Ev::Click, move |_| Msg::ShowDeleteRoutineDialog(i)),
                                i![C!["fas fa-times"]]
                            ]
                        ]]
                    ]
                })
                .collect::<Vec<_>>()],
        ]
    ]
}
