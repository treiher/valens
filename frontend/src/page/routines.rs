use seed::{prelude::*, *};

use crate::common;
use crate::data;

// ------ ------
//     Init
// ------ ------

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>) -> Model {
    if url.next_hash_path_part() == Some("add") {
        orders.send_msg(Msg::ShowAddRoutineDialog);
    }

    orders.subscribe(Msg::DataEvent);

    Model {
        dialog: Dialog::Hidden,
        loading: false,
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    dialog: Dialog,
    loading: bool,
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

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowAddRoutineDialog,
    ShowEditRoutineDialog(usize),
    ShowDeleteRoutineDialog(usize),
    CloseRoutineDialog,

    NameChanged(String),

    SaveRoutine,
    DeleteRoutine(u32),
    DataEvent(data::Event),
}

pub fn update(
    msg: Msg,
    model: &mut Model,
    data_model: &data::Model,
    orders: &mut impl Orders<Msg>,
) {
    match msg {
        Msg::ShowAddRoutineDialog => {
            model.dialog = Dialog::AddRoutine(Form {
                id: 0,
                name: (String::new(), None),
            });
        }
        Msg::ShowEditRoutineDialog(index) => {
            let id = data_model.routines[index].id;
            let name = data_model.routines[index].name.clone();
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
            Url::go_and_replace(&crate::Urls::new(&data_model.base_url).routines());
        }

        Msg::NameChanged(name) => match model.dialog {
            Dialog::AddRoutine(ref mut form) | Dialog::EditRoutine(ref mut form) => {
                if data_model.routines.iter().all(|e| e.name != name) {
                    form.name = (name.clone(), Some(name));
                } else {
                    form.name = (name, None);
                }
            }
            Dialog::Hidden | Dialog::DeleteRoutine(_) => {
                panic!();
            }
        },

        Msg::SaveRoutine => {
            model.loading = true;
            match model.dialog {
                Dialog::AddRoutine(ref mut form) => {
                    orders.notify(data::Msg::CreateRoutine(form.name.1.clone().unwrap()));
                }
                Dialog::EditRoutine(ref mut form) => {
                    orders.notify(data::Msg::ModifyRoutine(form.id, form.name.1.clone(), None));
                }
                Dialog::Hidden | Dialog::DeleteRoutine(_) => {
                    panic!();
                }
            };
        }
        Msg::DeleteRoutine(id) => {
            model.loading = true;
            orders.notify(data::Msg::DeleteRoutine(id));
        }
        Msg::DataEvent(event) => {
            model.loading = false;
            match event {
                data::Event::RoutineCreatedOk
                | data::Event::RoutineModifiedOk
                | data::Event::RoutineDeletedOk => {
                    orders.skip().send_msg(Msg::CloseRoutineDialog);
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
    div![
        view_routine_dialog(&model.dialog, &data_model.routines, model.loading),
        common::view_fab(|_| Msg::ShowAddRoutineDialog),
        view_table(data_model),
    ]
}

fn view_routine_dialog(dialog: &Dialog, routines: &[data::Routine], loading: bool) -> Node<Msg> {
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

fn view_table(data_model: &data::Model) -> Node<Msg> {
    div![
        C!["table-container"],
        C!["mt-4"],
        table![
            C!["table"],
            C!["is-fullwidth"],
            C!["is-hoverable"],
            thead![tr![th!["Name"], th![]]],
            tbody![&data_model
                .routines
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let id = e.id;
                    tr![
                        td![a![
                            attrs! {
                                At::Href => {
                                    crate::Urls::new(&data_model.base_url)
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
