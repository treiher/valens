use std::collections::BTreeMap;

use seed::{prelude::*, *};

use crate::common;
use crate::data;

// ------ ------
//     Init
// ------ ------

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>, navbar: &mut crate::Navbar) -> Model {
    if url.next_hash_path_part() == Some("add") {
        orders.send_msg(Msg::ShowAddRoutineDialog);
    }

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Routines");

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
    AddRoutine(Form),
    EditRoutine(Form),
    DeleteRoutine(u32),
}

struct Form {
    id: u32,
    name: common::InputField<String>,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowAddRoutineDialog,
    ShowEditRoutineDialog(u32),
    ShowDeleteRoutineDialog(u32),
    CloseRoutineDialog,

    SearchTermChanged(String),
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
                name: common::InputField::default(),
            });
        }
        Msg::ShowEditRoutineDialog(id) => {
            let id = data_model.routines[&id].id;
            let name = data_model.routines[&id].name.clone();
            model.dialog = Dialog::EditRoutine(Form {
                id,
                name: common::InputField {
                    input: name.clone(),
                    parsed: Some(name.clone()),
                    orig: name,
                },
            });
        }
        Msg::ShowDeleteRoutineDialog(id) => {
            model.dialog = Dialog::DeleteRoutine(id);
        }
        Msg::CloseRoutineDialog => {
            model.dialog = Dialog::Hidden;
            Url::go_and_replace(&crate::Urls::new(&data_model.base_url).routines());
        }

        Msg::SearchTermChanged(search_term) => {
            model.search_term = search_term;
        }
        Msg::NameChanged(name) => match model.dialog {
            Dialog::AddRoutine(ref mut form) | Dialog::EditRoutine(ref mut form) => {
                let trimmed_name = name.trim();
                if not(trimmed_name.is_empty())
                    && (trimmed_name == form.name.orig
                        || data_model.routines.values().all(|e| e.name != trimmed_name))
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
            Dialog::Hidden | Dialog::DeleteRoutine(_) => {
                panic!();
            }
        },

        Msg::SaveRoutine => {
            model.loading = true;
            match model.dialog {
                Dialog::AddRoutine(ref mut form) => {
                    orders.notify(data::Msg::CreateRoutine(form.name.parsed.clone().unwrap()));
                }
                Dialog::EditRoutine(ref mut form) => {
                    orders.notify(data::Msg::ModifyRoutine(
                        form.id,
                        form.name.parsed.clone(),
                        None,
                    ));
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
    if data_model.routines.is_empty() && data_model.loading_routines {
        common::view_page_loading()
    } else {
        div![
            view_routine_dialog(&model.dialog, &data_model.routines, model.loading),
            div![
                C!["px-4"],
                common::view_search_box(&model.search_term, Msg::SearchTermChanged)
            ],
            view_table(&model.search_term, data_model),
            common::view_fab("plus", |_| Msg::ShowAddRoutineDialog),
        ]
    }
}

fn view_routine_dialog(
    dialog: &Dialog,
    routines: &BTreeMap<u32, data::Routine>,
    loading: bool,
) -> Node<Msg> {
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
        Dialog::DeleteRoutine(id) => {
            let routine = &routines[id];
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
                                Msg::SaveRoutine
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

fn view_table(search_term: &str, data_model: &data::Model) -> Node<Msg> {
    div![
        C!["table-container"],
        C!["mt-4"],
        table![
            C!["table"],
            C!["is-fullwidth"],
            C!["is-hoverable"],
            tbody![&data_model
                .routines
                .values()
                .rev()
                .filter(|e| e.name.to_lowercase().contains(&search_term.to_lowercase()))
                .map(|e| {
                    let id = e.id;
                    tr![td![
                        C!["is-flex"],
                        C!["is-justify-content-space-between"],
                        a![
                            attrs! {
                                At::Href => {
                                    crate::Urls::new(&data_model.base_url)
                                        .routine()
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
                                ev(Ev::Click, move |_| Msg::ShowEditRoutineDialog(id)),
                                i![C!["fas fa-edit"]]
                            ],
                            a![
                                C!["icon"],
                                C!["ml-1"],
                                ev(Ev::Click, move |_| Msg::ShowDeleteRoutineDialog(id)),
                                i![C!["fas fa-times"]]
                            ]
                        ]
                    ]]
                })
                .collect::<Vec<_>>()],
        ]
    ]
}
