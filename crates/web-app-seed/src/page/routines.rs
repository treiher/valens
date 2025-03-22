use seed::{prelude::*, *};
use valens_domain as domain;

use crate::{common, data};

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
        search_term: url.hash_path().get(1).cloned().unwrap_or_default(),
        dialog: Dialog::Hidden,
        archive_visible: false,
        loading: false,
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    search_term: String,
    dialog: Dialog,
    archive_visible: bool,
    loading: bool,
}

enum Dialog {
    Hidden,
    AddRoutine(Form),
    EditRoutine(Form),
    DeleteRoutine(domain::RoutineID),
}

struct Form {
    id: domain::RoutineID,
    name: common::InputField<domain::Name>,
    template_routine_id: domain::RoutineID,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowAddRoutineDialog,
    ShowEditRoutineDialog(domain::RoutineID),
    ShowDeleteRoutineDialog(domain::RoutineID),
    CloseRoutineDialog,

    SearchTermChanged(String),
    NameChanged(String),
    TemplateRoutineChanged(String),

    ShowArchive,

    SaveRoutine,
    ChangeArchived(domain::RoutineID, bool),
    DeleteRoutine(domain::RoutineID),
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
                id: 0.into(),
                name: common::InputField::default(),
                template_routine_id: 0.into(),
            });
        }
        Msg::ShowEditRoutineDialog(id) => {
            let id = data_model.routines[&id].id;
            let name = data_model.routines[&id].name.clone();
            model.dialog = Dialog::EditRoutine(Form {
                id,
                name: common::InputField {
                    input: name.to_string(),
                    parsed: Some(name.clone()),
                    orig: name.to_string(),
                },
                template_routine_id: 0.into(),
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
            model.search_term.clone_from(&search_term);
            crate::Urls::new(&data_model.base_url)
                .routines()
                .add_hash_path_part(search_term)
                .go_and_replace();
        }
        Msg::NameChanged(name) => match model.dialog {
            Dialog::AddRoutine(ref mut form) | Dialog::EditRoutine(ref mut form) => {
                let parsed = domain::Name::new(&name).ok().and_then(|name| {
                    if name.as_ref() == &form.name.orig
                        || data_model.routines.values().all(|r| r.name != name)
                    {
                        Some(name)
                    } else {
                        None
                    }
                });
                form.name = common::InputField {
                    input: name,
                    parsed,
                    orig: form.name.orig.clone(),
                };
            }
            Dialog::Hidden | Dialog::DeleteRoutine(_) => {
                panic!();
            }
        },
        Msg::TemplateRoutineChanged(routine_id) => match model.dialog {
            Dialog::AddRoutine(ref mut form) => match routine_id.parse::<u128>() {
                Ok(parsed_routine_id) => {
                    form.template_routine_id = parsed_routine_id.into();
                }
                Err(_) => form.template_routine_id = 0.into(),
            },
            Dialog::Hidden | Dialog::EditRoutine(_) | Dialog::DeleteRoutine(_) => {
                panic!();
            }
        },

        Msg::ShowArchive => {
            model.archive_visible = true;
        }

        Msg::SaveRoutine => {
            model.loading = true;
            match model.dialog {
                Dialog::AddRoutine(ref mut form) => {
                    orders.notify(data::Msg::CreateRoutine(
                        form.name.parsed.clone().unwrap(),
                        form.template_routine_id,
                    ));
                }
                Dialog::EditRoutine(ref mut form) => {
                    orders.notify(data::Msg::ModifyRoutine(
                        form.id,
                        form.name.parsed.clone(),
                        None,
                        None,
                    ));
                }
                Dialog::Hidden | Dialog::DeleteRoutine(_) => {
                    panic!();
                }
            };
        }
        Msg::ChangeArchived(id, archived) => {
            model.loading = true;
            orders.notify(data::Msg::ModifyRoutine(id, None, Some(archived), None));
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
            view_routine_dialog(
                &model.dialog,
                &data_model.routines_sorted_by_last_use(|r: &domain::Routine| !r.archived),
                model.loading
            ),
            div![
                C!["px-4"],
                common::view_search_box(&model.search_term, Msg::SearchTermChanged)
            ],
            view_table(&model.search_term, model.archive_visible, data_model),
            common::view_fab("plus", |_| Msg::ShowAddRoutineDialog),
        ]
    }
}

fn view_routine_dialog(dialog: &Dialog, routines: &[domain::Routine], loading: bool) -> Node<Msg> {
    let title;
    let form;
    let mut template_selection = false;
    match dialog {
        Dialog::AddRoutine(f) => {
            title = "Add routine";
            form = f;
            template_selection = true;
        }
        Dialog::EditRoutine(f) => {
            title = "Edit routine";
            form = f;
        }
        Dialog::DeleteRoutine(id) => {
            let id = *id;
            let name = routines
                .iter()
                .find(|r| r.id == id)
                .map(|r| r.name.to_string())
                .unwrap_or_default();
            return common::view_delete_confirmation_dialog(
                "routine",
                &span![&name],
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
        span![title],
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
            IF![template_selection => div![
                C!["field"],
                label![C!["label"], "Template"],
                div![
                    C!["control"],
                    input_ev(Ev::Change, Msg::TemplateRoutineChanged),
                    div![
                        C!["select"],
                        select![
                            option!["",
                                attrs![
                                    At::Value => 0,
                                ]
                            ],
                            routines.iter()
                            .map(|r| {
                                option![
                                    &r.name.as_ref(),
                                    attrs![
                                        At::Value => r.id.as_u128(),
                                    ]
                                ]
                            })
                            .collect::<Vec<_>>()],
                    ],
                ],
            ]],
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

fn view_table(search_term: &str, archive_visible: bool, data_model: &data::Model) -> Node<Msg> {
    let routines = data_model.routines_sorted_by_last_use(|r: &domain::Routine| {
        !r.archived
            && r.name
                .as_ref()
                .to_lowercase()
                .contains(&search_term.to_lowercase())
    });
    let archived_routines = data_model.routines_sorted_by_last_use(|r: &domain::Routine| {
        r.archived
            && r.name
                .as_ref()
                .to_lowercase()
                .contains(&search_term.to_lowercase())
    });
    div![
        C!["table-container"],
        C!["mt-4"],
        table![
            C!["table"],
            C!["is-fullwidth"],
            C!["is-hoverable"],
            tbody![routines.iter().map(|r| view_table_row(
                r.id,
                r.name.as_ref(),
                r.archived,
                &data_model.base_url
            ))],
        ],
        IF![!archived_routines.is_empty() =>
            if archive_visible {
                nodes![
                    common::view_title(&span!["Archive"], 3),
                    table![
                        C!["table"],
                        C!["is-fullwidth"],
                        C!["is-hoverable"],
                        tbody![archived_routines
                            .iter()
                            .map(|r|
                                view_table_row(r.id, r.name.as_ref(), r.archived, &data_model.base_url)
                            )
                        ],
                    ]
                ]
            } else {
                nodes![
                    div![
                        C!["has-text-centered"],
                        button![
                            C!["button"],
                            C!["is-small"],
                            ev(Ev::Click, move |_| Msg::ShowArchive),
                            span![
                                C!["icon"],
                                C!["is-small"],
                                i![C!["fas fa-box-archive"]]
                            ],
                            span!["Show archive"]
                        ]
                    ]
                ]
            }
        ]
    ]
}

fn view_table_row(id: domain::RoutineID, name: &str, archived: bool, base_url: &Url) -> Node<Msg> {
    tr![td![
        C!["is-flex"],
        C!["is-justify-content-space-between"],
        a![
            attrs! {
                At::Href => {
                    crate::Urls::new(base_url)
                        .routine()
                        .add_hash_path_part(id.as_u128().to_string())
                }
            },
            name,
        ],
        p![
            C!["is-flex is-flex-wrap-nowrap"],
            if archived {
                a![
                    C!["icon"],
                    C!["mr-1"],
                    ev(Ev::Click, move |_| Msg::ChangeArchived(id, false)),
                    i![C!["fas fa-box-open"]]
                ]
            } else {
                a![
                    C!["icon"],
                    C!["mr-1"],
                    ev(Ev::Click, move |_| Msg::ChangeArchived(id, true)),
                    i![C!["fas fa-box-archive"]]
                ]
            },
            a![
                C!["icon"],
                C!["mx-1"],
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
}
